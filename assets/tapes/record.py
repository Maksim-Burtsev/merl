#!/usr/bin/env python3
"""Records one of the README's GIFs: a real terminal session, played back at the speed it ran.

    cargo build --release
    assets/tapes/setup.sh                              # once: the checkouts in /tmp/merl-demo
    assets/tapes/record.py assets/demo.steps           # writes assets/demo.gif
    assets/tapes/record.py --keys assets/review.steps  # writes assets/review.gif and, from the
                                                       # same take, review-keys.gif: each key drawn
                                                       # as it is pressed

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

merl starts in the gitea checkout setup.sh made, on the branch it left; a `# project: mealie` line
names another of its checkouts.
"""
import json, os, shlex, subprocess, sys, tempfile, time
from PIL import Image, ImageDraw, ImageFont

ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
DEMO = "/tmp/merl-demo"
COLS, ROWS = 132, 41          # a 16:10 laptop window at an 18 px font
FONT, LINE = 36, 1.25         # agg's font size in px, twice the 18 the window is laid out for
PAD = 28                      # px of background around the screen, so no text touches the edge
FPS = 25                      # merl draws a screen at once: a higher cap only makes 10 ms frames
TYPE, PACE = 0.12, 0.35       # seconds between typed characters, and between named keys
SOCK = "merl-readme"
BG = "222436"
# tokyonight-moon's background and foreground, then the 16 ANSI colours; merl itself only emits
# 24-bit colour, so the 16 never show.
THEME = f"{BG},c8d3f5," + ",".join(["1b1d2b", "ff757f", "c3e88d", "ffc777", "82aaff", "c099ff",
                                    "86e1fc", "828bb8"] * 2)

# --keys: a keycap in the bottom right corner for every key pressed, the way macOS keystroke
# visualisers draw them. Keys pressed less than HOLD apart share the row, a repeat counts up on its
# cap, and HOLD after the last one the row fades out over FADE.
HOLD, FADE, FADE_STEPS = 1.2, 0.3, 5
CAPS = {"M-Right": "⌥→", "M-Left": "⌥←", "Right": "→", "Left": "←", "Down": "↓", "Up": "↑",
        "Enter": "↵", "Escape": "esc", "Tab": "⇥", "BSpace": "⌫"}
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


def keycaps(presses):
    """What the corner shows from each moment on: [(time, caps, opacity)], caps a list of
    (label, count)."""
    runs = []
    for t, name in presses:
        if runs and t - runs[-1][-1][0] < HOLD:
            runs[-1].append((t, name))
        else:
            runs.append([(t, name)])
    states = []
    for run in runs:
        # A run that starts while the last one fades takes over at once.
        states = [st for st in states if st[0] < run[0][0]]
        caps = []
        for t, name in run:
            label = CAPS.get(name, name)
            if caps and caps[-1][0] == label:
                caps[-1] = (label, caps[-1][1] + 1)
            else:
                caps.append((label, 1))
            states.append((t, caps[-3:], 1.0))
        last = run[-1][0] + HOLD
        for i in range(1, FADE_STEPS + 1):
            states.append((last + FADE * (i - 1) / FADE_STEPS, caps[-3:], 1 - i / (FADE_STEPS + 1)))
        states.append((last + FADE, [], 0.0))
    return states


def draw_caps(frame, caps, opacity):
    if not caps:
        return frame
    s = FONT / 18  # the sizes below are in the pixels of the 18 px window
    big, small = ImageFont.truetype(SANS, round(28 * s)), ImageFont.truetype(SANS, round(17 * s))
    big.set_variation_by_name("Semibold")
    small.set_variation_by_name("Medium")
    layer = Image.new("RGBA", frame.size)
    d = ImageDraw.Draw(layer)
    h, gap, r = round(54 * s), round(10 * s), round(12 * s)
    # Each cap: its label, and after it the repeat count, smaller.
    texts = [(label, f"×{count}" if count > 1 else "") for label, count in caps]
    sizes = [(d.textlength(t, font=big), d.textlength(c, font=small) + 5 * s if c else 0) for t, c in texts]
    widths = [max(h, round(tw + cw + 32 * s)) for tw, cw in sizes]
    # Bottom right, over the code and clear of the status line.
    x = frame.width - PAD - round(22 * s) - sum(widths) - gap * (len(caps) - 1)
    y = frame.height - PAD - round(FONT * LINE) - round(20 * s) - h
    a = lambda v: round(v * opacity)
    for (label, count), (tw, cw), w in zip(texts, sizes, widths):
        d.rounded_rectangle((x, y, x + w, y + h), r, fill=(56, 62, 94, a(242)),
                            outline=(130, 139, 184, a(150)), width=max(1, round(s)))
        tx = x + (w - tw - cw) / 2
        d.text((tx, y + h / 2), label, font=big, fill=(230, 235, 255, a(255)), anchor="lm")
        d.text((tx + tw + 5 * s, y + h / 2 + 2 * s), count, font=small, fill=(170, 178, 220, a(255)), anchor="lm")
        x += w + gap
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
        # A key shows with merl's answer to it: the change of the screen nearest the press, which
        # the two clocks put a few tens of milliseconds either side of it.
        snap = lambda t: min((s for s in starts if abs(s - t) <= 0.15), key=lambda s: abs(s - t), default=t)
        states = [(snap(t) if o == 1 else t, c, o) for t, c, o in keycaps(presses)] if keys else []
        # Browsers hold a frame shorter than 20 ms for 100 ms, and a GIF rounds every delay to 10 ms:
        # the corner changes only 40 ms or more away from a change of the screen, as agg's frames do.
        cuts = sorted({*starts, *(s[0] for s in states if 0 <= s[0] < total
                                  and all(abs(s[0] - t) >= 0.04 or s[0] == t for t in starts))})
        bg = Image.new("RGB", (src.width + 2 * PAD, src.height + 2 * PAD), f"#{BG}")
        listing, frames, last = [], 0, None
        for n, cut in enumerate(cuts):
            base = max(i for i, s in enumerate(starts) if s <= cut)
            state = next((s for s in reversed(states) if s[0] <= cut), (0, [], 0.0))
            dur = (cuts[n + 1] if n + 1 < len(cuts) else total) - cut
            if (base, state[1:]) == last:
                listing[-1][1] += dur
                continue
            last = (base, state[1:])
            src.seek(base)
            frame = bg.copy()
            frame.paste(src.convert("RGB"), (PAD, PAD))
            path = f"{tmp}/{frames:04d}.png"
            draw_caps(frame, state[1], state[2]).save(path, compress_level=1)
            listing.append([path, dur])
            frames += 1
        with open(f"{tmp}/list.txt", "w") as f:
            for path, dur in listing:
                f.write(f"file '{path}'\nduration {dur:.3f}\n")
            f.write(f"file '{listing[-1][0]}'\n")  # the concat demuxer drops the last duration
        subprocess.run(["ffmpeg", "-loglevel", "error", "-y", "-f", "concat", "-safe", "0", "-i",
                        f"{tmp}/list.txt", "-filter_complex",
                        "split[a][b];[a]palettegen=stats_mode=full[p];"
                        "[b][p]paletteuse=dither=none:diff_mode=rectangle",
                        "-fps_mode", "passthrough", gif], check=True)
    return frames


def main():
    args = sys.argv[1:]
    keys = "--keys" in args
    steps_path = os.path.abspath(next(a for a in args if a != "--keys"))
    gif = steps_path.removesuffix(".steps") + ".gif"
    text = open(steps_path).read().splitlines()
    steps = [l.strip() for l in text if l.strip() and not l.lstrip().startswith("#")]
    assert steps[0].split()[0] == "merl", "the first step is `merl [ARGS]`"
    name = next((l.split(":", 1)[1].strip() for l in text if l.startswith("# project:")), "gitea")
    project = f"{DEMO}/{name}"
    # The take starts clean: the last take's files, and the viewed marks merl keeps from one
    # start to the next (#240).
    clean = 'git checkout -q -- . && git clean -fdq && rm -f "$(git rev-parse --git-common-dir)/merl/viewed"'
    subprocess.run(clean, shell=True, cwd=project, check=True)
    cast, presses = take(steps, project)
    for out, drawn in [(gif, False)] + [(gif.removesuffix(".gif") + "-keys.gif", True)] * keys:
        frames = render(cast, presses, out, drawn)
        print(out, f"{os.path.getsize(out) / 1e6:.2f} MB, {frames} frames")
    os.unlink(cast)


main()
