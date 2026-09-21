#!/usr/bin/env python3
"""Records one of the README's GIFs: a real terminal session, played back at the speed it ran.

    cargo build --release
    assets/tapes/setup.sh                       # once: the gitea checkout in /tmp/merl-demo
    assets/tapes/record.py assets/demo.steps    # writes assets/demo.gif

merl runs under `asciinema rec` inside a detached tmux pane, the keys arrive through
`tmux send-keys` at a human pace, and `agg` turns the recording into the GIF: nothing is drawn by
hand and nothing is sped up. Needs tmux, asciinema and agg (`brew install asciinema agg`).

A steps file is one verb per line (`#` comments and blank lines are skipped), the same verbs as
tools/cast.py plus `merl`, `show` and `spawn`:

    merl --review      the arguments merl starts with; the first line
    wait block.go      poll the pane until the text shows up (15 s, then fail)
    key Down Enter     tmux key names, one keystroke each, PACE apart; `key@0.1 Right*12` repeats
    type user block    the characters, one keystroke each, TYPE apart; `type@0.25 IsOw` is slower
    sleep 1.5          seconds
    show               the GIF starts here: what came before is on screen in the first frame
    spawn CMD          run CMD in the project in the background (the agent in the next split)
"""
import json, os, shlex, subprocess, sys, tempfile, time

ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
DEMO = "/tmp/merl-demo"
COLS, ROWS = 132, 41          # a 16:10 laptop window at the font below
TYPE, PACE = 0.12, 0.35       # seconds between typed characters, and between named keys
SOCK = "merl-readme"
# tokyonight-moon's background and foreground, then the 16 ANSI colours; merl itself only emits
# 24-bit colour, so the 16 never show.
THEME = "222436,c8d3f5," + ",".join(["1b1d2b", "ff757f", "c3e88d", "ffc777", "82aaff", "c099ff",
                                     "86e1fc", "828bb8"] * 2)


def tmux(*args):
    return subprocess.run(["tmux", "-L", SOCK, *args], capture_output=True, text=True).stdout


def wait(text, timeout=15):
    end = time.time() + timeout
    while time.time() < end:
        if text in tmux("capture-pane", "-p", "-t", "p"):
            return
        time.sleep(0.05)
    sys.exit(f"never saw {text!r} on screen:\n" + tmux("capture-pane", "-p", "-t", "p"))


def main():
    steps_path = os.path.abspath(sys.argv[1])
    gif = steps_path.removesuffix(".steps") + ".gif"
    steps = [l.strip() for l in open(steps_path) if l.strip() and not l.lstrip().startswith("#")]
    assert steps[0].split()[0] == "merl", "the first step is `merl [ARGS]`"
    project = f"{DEMO}/gitea"
    subprocess.run("git checkout -q -- . && git clean -fdq", shell=True, cwd=project, check=True)
    cast = tempfile.mktemp(suffix=".cast")
    merl = shlex.join([f"{ROOT}/target/release/merl", *steps[0].split()[1:]])
    tmux("kill-server")
    tmux("new-session", "-d", "-s", "p", "-x", str(COLS), "-y", str(ROWS), "-c", project,
         f"HOME={DEMO}/home asciinema rec -q --overwrite -f asciicast-v2 -c {shlex.quote(merl)} {cast}")
    tmux("set", "-t", "p", "status", "off")
    shown = None
    time.sleep(1.5)
    for step in steps[1:]:
        verb, _, arg = step.partition(" ")
        if verb == "wait":
            wait(arg)
        elif verb == "sleep":
            time.sleep(float(arg))
        elif verb.startswith("key"):
            pace = float(verb.partition("@")[2] or PACE)
            for k in arg.split():
                name, _, count = k.partition("*")
                for _ in range(int(count or 1)):
                    tmux("send-keys", "-t", "p", name)
                    time.sleep(pace)
        elif verb.startswith("type"):
            pace = float(verb.partition("@")[2] or TYPE)
            for ch in arg:
                tmux("send-keys", "-t", "p", "-l", ch)
                time.sleep(pace)
        elif verb == "show":
            shown = time.time()
        elif verb == "spawn":
            subprocess.Popen(arg, shell=True, cwd=project, env={**os.environ, "ROOT": ROOT})
        else:
            sys.exit(f"unknown step: {step}")
    killed = time.time()
    tmux("kill-server")
    time.sleep(0.5)

    # Everything before `show` happens at 0 s, so the first frame is the screen as it stood then.
    lines = open(cast).read().splitlines()
    header, events = json.loads(lines[0]), [json.loads(l) for l in lines[1:]]
    # merl dying puts the shell's screen back: the GIF ends before that.
    leave = next((i for i, e in enumerate(events) if "\x1b[?1049l" in e[2]), len(events))
    end = events[min(leave, len(events) - 1)][0]
    events = events[:leave]
    first = next(t for t, kind, _ in events if kind == "o")
    # The recording's clock: its last event is merl dying, which happened at `killed`. A `show`
    # always follows a pause, so a few milliseconds either way change nothing.
    shown = max(shown - (killed - end), first) if shown else first
    with open(cast, "w") as out:
        out.write(json.dumps(header) + "\n")
        for t, kind, data in events:
            out.write(json.dumps([round(max(t - shown, 0.0), 4), kind, data]) + "\n")
    subprocess.run(["agg", "--font-family", "SF Mono,Menlo", "--font-size", "18", "--line-height", "1.25",
                    "--theme", THEME, "--fps-cap", "25", "--idle-time-limit", "10",
                    "--last-frame-duration", "2", cast, gif], check=True,
                   stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    os.unlink(cast)
    print(gif, f"{os.path.getsize(gif) / 1e6:.2f} MB")


main()
