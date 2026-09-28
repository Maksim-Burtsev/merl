#!/usr/bin/env python3
"""Records one of the README's GIFs: a real terminal session, played back at the speed it ran.

    cargo build --release
    assets/tapes/setup.sh                              # once: the checkouts in /tmp/merl-demo
    assets/tapes/record.py assets/demo.steps           # writes assets/demo.gif
    assets/tapes/record.py --keys assets/review.steps  # writes assets/review.gif, the key just
                                                       # pressed drawn in its corner

merl runs under `asciinema rec` inside a detached tmux pane, the keys arrive through
`tmux send-keys` at a human pace, and `agg` turns the recording into frames: nothing is drawn by
hand and nothing is sped up. agg draws them at twice the pixels of the window they were sized for
(a 36 px font in a window laid out for 18), so the GIF stays sharp on a retina screen, at the
README's width and opened full size alike. Each frame then gets a margin of the background colour,
and ffmpeg writes the GIF with one palette for the whole take and no dithering: text keeps clean
edges, and a frame stores only the rectangle that changed. Needs tmux, asciinema, agg, ffmpeg and
Pillow (`brew install asciinema agg ffmpeg`, `pip install pillow`).

A steps file is one verb per line (`#` comments and blank lines are skipped), the same verbs as
tools/cast.py plus `merl`, `show` and `spawn`:

    merl --review      the arguments merl starts with; the first line
    wait block.go      poll the pane until the text shows up (15 s, then fail)
    key Down Enter     tmux key names, one keystroke each, PACE apart; `key@0.1 Right*12` repeats
    type user block    the characters, one keystroke each, TYPE apart; `type@0.25 IsOw` is slower
    sleep 1.5          seconds
    show               the GIF starts here: what came before is on screen in the first frame
    spawn CMD          run CMD in the project in the background (the agent in the next split)

merl starts in the gitea checkout setup.sh made, on the branch it left; a `project polar` line
names another of its checkouts, as in tests/smoke/run.py.

    assets/tapes/record.py --selftest                  # checks the keycaps' timing, no recording
"""
import json, os, shlex, subprocess, sys, tempfile, time
from PIL import Image, ImageDraw, ImageFont

ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
sys.path.insert(0, os.path.join(ROOT, "tools"))
sys.dont_write_bytecode = True  # no __pycache__ left in tools/
import cast  # noqa: E402
DEMO = "/tmp/merl-demo"
COLS, ROWS = 132, 41          # a 16:10 laptop window at an 18 px font
FONT, LINE = 36, 1.25         # agg's font size in px, twice the 18 the window is laid out for
PAD = 28                      # px of background around the screen, so no text touches the edge
FPS = 25                      # merl draws a screen at once: a higher cap only makes 10 ms frames
TYPE, PACE = 0.12, 0.35       # seconds between typed characters, and between named keys
SOCK = "merl-readme"
# The owner's terminal, as tools/cast.py has it: the background, the foreground, then the 16 ANSI
# colours, which only what the shell prints uses: merl paints in 24-bit colour.
THEME = ",".join("%02x%02x%02x" % c for c in (cast.DEFAULT_BG, cast.DEFAULT_FG, *cast.ANSI))
BG = THEME[:6]

# --keys: the key just pressed, drawn in the bottom right corner the way macOS keystroke
# visualisers draw it. It shows with merl's answer to the press, stays until the next key replaces
# it, and HOLD after the last one fades out over FADE: no flash, no count, no row of past keys,
# which would read as a chord.
HOLD, FADE, FADE_STEPS = 1.5, 0.25, 4
CAPS = {"M-Right": "⌥→", "M-Left": "⌥←", "Right": "→", "Left": "←", "Down": "↓", "Up": "↑",
        "Enter": "Enter", "Escape": "Esc", "Tab": "Tab", "BSpace": "⌫", "C-d": "Ctrl+D",
        "C-u": "Ctrl+U"}
SANS = "/System/Library/Fonts/SFNS.ttf"


def tmux(*args):
    return subprocess.run(["tmux", "-L", SOCK, *args], capture_output=True, text=True).stdout


def wait(text, timeout=15):
    end = time.time() + timeout
    while time.time() < end:
        if text in tmux("capture-pane", "-p", "-t", "p"):
            return
        time.sleep(0.05)
    sys.exit(f"never saw {text!r} on screen:\n" + tmux("capture-pane", "-p", "-t", "p"))


def take(steps, project):
    """Plays the steps; returns the recording, its clock set to 0 at `show`, and every key pressed
    with its time on that clock."""
    cast = tempfile.mktemp(suffix=".cast")
    merl = shlex.join([f"{ROOT}/target/release/merl", *steps[0].split()[1:]])
    tmux("kill-server")
    tmux("new-session", "-d", "-s", "p", "-x", str(COLS), "-y", str(ROWS), "-c", project,
         f"HOME={DEMO}/home asciinema rec -q --overwrite -f asciicast-v2 -c {shlex.quote(merl)} {cast}")
    tmux("set", "-t", "p", "status", "off")
    shown, presses = None, []
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
                    presses.append((time.time(), name))
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
    lag = killed - end
    shown = max(shown - lag, first) if shown else first
    with open(cast, "w") as out:
        out.write(json.dumps(header) + "\n")
        for t, kind, data in events:
            out.write(json.dumps([round(max(t - shown, 0.0), 4), kind, data]) + "\n")
    return cast, [(t - lag - shown, name) for t, name in presses if t - lag >= shown]


def draw_caps(frame, key, opacity):
    if not key:
        return frame
    s = FONT / 18  # the sizes below are in the pixels of the 18 px window
    font = ImageFont.truetype(SANS, round(44 * s))
    font.set_variation_by_name("Semibold")
    label = CAPS.get(key, key)
    layer = Image.new("RGBA", frame.size)
    d = ImageDraw.Draw(layer)
    h = round(88 * s)
    w = max(h, round(d.textlength(label, font=font) + h * 0.55))
    # Bottom right, over the code and clear of the status line.
    x = frame.width - PAD - round(20 * s) - w
    y = frame.height - PAD - round(FONT * LINE) - round(14 * s) - h
    a = lambda v: round(v * opacity)
    d.rounded_rectangle((x, y, x + w, y + h), round(h * 0.2), fill=(47, 51, 77, a(250)),
                        outline=(130, 139, 184, a(160)), width=max(2, round(1.2 * s)))
    d.text((x + w / 2, y + h / 2), label, font=font, fill=(230, 235, 255, a(255)), anchor="mm")
    return Image.alpha_composite(frame.convert("RGBA"), layer).convert("RGB")


def render(cast, presses, gif, keys):
    with tempfile.TemporaryDirectory() as tmp:
        raw = f"{tmp}/agg.gif"
        subprocess.run(["agg", "--font-family", "SF Mono,Menlo", "--font-size", str(FONT),
                        "--line-height", str(LINE), "--theme", THEME, "--fps-cap", str(FPS),
                        "--idle-time-limit", "10", "--last-frame-duration", "2", cast, raw],
                       check=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        src = Image.open(raw)
        starts, t = [], 0.0
        for i in range(src.n_frames):
            src.seek(i)
            starts.append(t)
            t += src.info["duration"] / 1000
        total = t
        bg = Image.new("RGB", (src.width + 2 * PAD, src.height + 2 * PAD), f"#{BG}")
        listing = []
        for cut, dur, base, key, opacity in timeline(starts, total, presses if keys else []):
            src.seek(base)
            frame = bg.copy()
            frame.paste(src.convert("RGB"), (PAD, PAD))
            path = f"{tmp}/{len(listing):04d}.png"
            draw_caps(frame, key, opacity).save(path, compress_level=1)
            listing.append((path, dur))
        with open(f"{tmp}/list.txt", "w") as f:
            for path, dur in listing:
                f.write(f"file '{path}'\nduration {dur:.3f}\n")
            f.write(f"file '{listing[-1][0]}'\n")  # the concat demuxer drops the last duration
        subprocess.run(["ffmpeg", "-loglevel", "error", "-y", "-f", "concat", "-safe", "0", "-i",
                        f"{tmp}/list.txt", "-filter_complex",
                        "split[a][b];[a]palettegen=stats_mode=full[p];"
                        "[b][p]paletteuse=dither=none:diff_mode=rectangle",
                        "-fps_mode", "passthrough", gif], check=True)
    return len(listing)


def timeline(starts, total, presses):
    """The GIF's frames, [(time, duration, screen, key, opacity)]: agg's screens (their start
    times, `total` the end of the last) under the corner's key, None when it is empty."""
    # A key shows with merl's answer to it, the first change of the screen after the press: a `d`
    # that searches the project answers half a second later, and its key waits for it. The two
    # clocks put the answer a few tens of milliseconds either side of the press.
    shown = []
    for i, (t, name) in enumerate(presses):
        nxt = presses[i + 1][0] if i + 1 < len(presses) else t + 1.0
        at = next((s for s in starts if t - 0.06 <= s < min(nxt - 0.06, t + 1.0)), t)
        shown.append((max(at, shown[-1][0] + 0.04) if shown else at, name))
    states = []
    for i, (at, name) in enumerate(shown):
        nxt = shown[i + 1][0] if i + 1 < len(shown) else float("inf")
        states.append((at, name, 1.0))
        for k in range(1, FADE_STEPS + 1):
            if at + HOLD + FADE * (k - 1) / FADE_STEPS < nxt:
                states.append((at + HOLD + FADE * (k - 1) / FADE_STEPS, name, 1 - k / (FADE_STEPS + 1)))
        if at + HOLD + FADE < nxt:
            states.append((at + HOLD + FADE, None, 0.0))
    cuts = sorted({*starts, *(t for t, _, _ in states if 0 <= t < total)})
    # Browsers hold a frame shorter than 20 ms for 100 ms and a GIF counts in 10 ms: a fade step
    # within 40 ms of the next change gives way to it.
    cuts = [c for i, c in enumerate(cuts) if c in starts or i + 1 == len(cuts) or cuts[i + 1] - c >= 0.04]
    frames = []
    for n, cut in enumerate(cuts):
        base = max(i for i, s in enumerate(starts) if s <= cut)
        _, key, opacity = next((st for st in reversed(states) if st[0] <= cut + 1e-9), (0, None, 0.0))
        dur = (cuts[n + 1] if n + 1 < len(cuts) else total) - cut
        if frames and frames[-1][2:] == (base, key, opacity):
            frames[-1] = (frames[-1][0], frames[-1][1] + dur, *frames[-1][2:])
        else:
            frames.append((cut, dur, base, key, opacity))
    return frames


def selftest():
    """The corner's timing, on made-up screens: no tmux, no agg."""
    shown = lambda frames, t: next(f for f in reversed(frames) if f[0] <= t + 1e-9)
    # A press lands on merl's answer 30 ms later; the corner is empty HOLD + FADE after it.
    f = timeline([0, 1.03, 5.0], 7, [(1.0, "c")])
    assert shown(f, 1.0)[3] is None and shown(f, 1.03)[3:] == ("c", 1.0), f
    assert shown(f, 1.03 + HOLD + FADE)[3] is None, f
    # A d answered half a second later shows then, not before.
    f = timeline([0, 1.5, 5.0], 7, [(1.0, "d")])
    assert shown(f, 1.3)[3] is None and shown(f, 1.5)[3] == "d", f
    # The next key replaces the last at once, at full strength; a press with no answer shows
    # when it was pressed.
    f = timeline([0, 1.02, 1.22, 5.0], 7, [(1.0, "M-Right"), (1.2, "M-Right"), (1.4, "d")])
    assert shown(f, 1.22)[3:] == ("M-Right", 1.0) and shown(f, 1.4)[3:] == ("d", 1.0), f
    # No frame is shorter than 40 ms.
    f = timeline([0, 2.8, 5.0], 7, [(1.0, "c"), (2.77, "c")])
    assert all(d >= 0.04 - 1e-9 for _, d, *_ in f), f
    # Without keys, the frames are agg's.
    assert [t for t, *_ in timeline([0, 1, 2], 3, [])] == [0, 1, 2]
    print("selftest ok")


def main():
    args = sys.argv[1:]
    if args == ["--selftest"]:
        return selftest()
    keys = "--keys" in args
    steps_path = os.path.abspath(next(a for a in args if a != "--keys"))
    gif = steps_path.removesuffix(".steps") + ".gif"
    text = open(steps_path).read().splitlines()
    steps = [l.strip() for l in text if l.strip() and not l.lstrip().startswith("#")]
    name = next((l.split()[1] for l in steps if l.split()[0] == "project"), "gitea")
    steps = [l for l in steps if l.split()[0] != "project"]
    assert steps[0].split()[0] == "merl", "the first step is `merl [ARGS]`"
    project = f"{DEMO}/{name}"
    # The take starts clean: the last take's files, and the viewed marks merl keeps from one
    # start to the next (#240).
    clean = 'git checkout -q -- . && git clean -fdq && rm -f "$(git rev-parse --git-common-dir)/merl/viewed"'
    subprocess.run(clean, shell=True, cwd=project, check=True)
    cast, presses = take(steps, project)
    frames = render(cast, presses, gif, keys)
    print(gif, f"{os.path.getsize(gif) / 1e6:.2f} MB, {frames} frames")
    os.unlink(cast)


if __name__ == "__main__":
    main()
