#!/usr/bin/env python3
"""Record merl in a tmux pane as an animated GIF that reads like a screen recording.

    cargo build --release
    tools/cast.py --bin target/release/merl --keys steps.txt -o after.gif -- tutor/notes/store.py

One run records one binary. A before/after pair is two runs of the same steps file: one against
a build of master, one against the branch. Build master in its own worktree with its own
CARGO_TARGET_DIR -- worktrees sharing a target dir hand you a stale binary.

The steps file is one verb per line, `#` comments and blank lines ignored:

    wait store.py      poll the pane until the text shows up (10 s, then fail)
    key Down Down d    tmux key names, several per line
    type Duration      the literal characters, one keystroke at a time
    sleep 0.8          seconds

The pane is sampled on a timer and identical frames are merged, so the GIF keeps the real
timing of the run instead of one frame per keystroke. Keep a recording under ~15 s.

Record outside the repository. merl walks up to the project root, so a sample project inside
the checkout puts merl's own tree in the shot and makes `s` grep thousands of files -- the UI
blocks and the keystrokes are lost:

    cp -R tutor/notes /tmp/notes && tools/cast.py --project /tmp/notes ...

macOS only: the renderer reads Menlo out of /System/Library/Fonts/Menlo.ttc.
"""
import argparse, os, re, shlex, shutil, subprocess, sys, tempfile, threading, time
from PIL import Image, ImageDraw, ImageFont

FONT = "/System/Library/Fonts/Menlo.ttc"  # index 0 regular, 1 bold, 2 italic, 3 bold-italic
DEFAULT_FG, DEFAULT_BG = (0xcc, 0xcc, 0xcc), (0x10, 0x10, 0x10)
SELFTEST_STEPS = ["wait store.py", "key s", "sleep 0.4", "type config", "sleep 1.2"]
# Menlo's box glyphs leave gaps at a cell this tall, so they are drawn as lines through the centre.
SEGMENTS = {"│": "ud", "┃": "ud", "─": "lr", "━": "lr", "┌": "dr", "┐": "dl", "└": "ur", "┘": "ul",
            "├": "udr", "┤": "udl", "┬": "dlr", "┴": "ulr", "┼": "udlr",
            "╭": "dr", "╮": "dl", "╰": "ur", "╯": "ul"}
SGR = re.compile(r"\x1b\[([0-9;]*)m")


def xterm(n):
    if n < 16:
        return [(0, 0, 0), (205, 0, 0), (0, 205, 0), (205, 205, 0), (0, 0, 238), (205, 0, 205),
                (0, 205, 205), (229, 229, 229), (127, 127, 127), (255, 0, 0), (0, 255, 0),
                (255, 255, 0), (92, 92, 255), (255, 0, 255), (0, 255, 255), (255, 255, 255)][n]
    if n < 232:
        s, n = (0, 95, 135, 175, 215, 255), n - 16
        return (s[n // 36], s[(n // 6) % 6], s[n % 6])
    v = 8 + (n - 232) * 10
    return (v, v, v)


def apply_sgr(params, st):
    fg, bg, bold, italic, rev = st
    codes = [int(p) for p in params.split(";") if p] or [0]
    i = 0
    while i < len(codes):
        c = codes[i]
        if c == 0: fg, bg, bold, italic, rev = DEFAULT_FG, DEFAULT_BG, False, False, False
        elif c == 1: bold = True
        elif c == 3: italic = True
        elif c == 7: rev = True
        elif c == 22: bold = False
        elif c == 23: italic = False
        elif c == 27: rev = False
        elif c == 39: fg = DEFAULT_FG
        elif c == 49: bg = DEFAULT_BG
        elif 30 <= c <= 37: fg = xterm(c - 30)
        elif 90 <= c <= 97: fg = xterm(c - 82)
        elif 40 <= c <= 47: bg = xterm(c - 40)
        elif 100 <= c <= 107: bg = xterm(c - 92)
        elif c in (38, 48):
            if codes[i + 1:i + 2] == [2]:
                col, i = tuple(codes[i + 2:i + 5]), i + 4
            else:
                col, i = xterm(codes[i + 2] if i + 2 < len(codes) else 0), i + 2
            fg, bg = (col, bg) if c == 38 else (fg, col)
        i += 1
    return fg, bg, bold, italic, rev


def parse(capture, cols, rows):
    """A capture-pane -e dump into a grid of (char, fg, bg, bold, italic).

    SGR state carries across lines: capture-pane only emits the changes, so a parser that resets
    per line paints whole rows in the wrong background."""
    st = (DEFAULT_FG, DEFAULT_BG, False, False, False)
    grid = []
    for raw in capture.split("\n")[:rows]:
        cells = []
        pos = 0
        for m in SGR.finditer(raw):
            fg, bg, bold, italic, rev = st
            cells += [(ch, bg, fg, bold, italic) if rev else (ch, fg, bg, bold, italic)
                      for ch in raw[pos:m.start()]]
            st, pos = apply_sgr(m.group(1), st), m.end()
        fg, bg, bold, italic, rev = st
        cells += [(ch, bg, fg, bold, italic) if rev else (ch, fg, bg, bold, italic)
                  for ch in raw[pos:]]
        grid.append((cells + [(" ", fg, bg, False, False)] * cols)[:cols])
    blank = [(" ", DEFAULT_FG, DEFAULT_BG, False, False)] * cols
    return (grid + [blank] * rows)[:rows]


def render(grid, cursor, fonts, cell):
    cw, ch = cell
    img = Image.new("RGB", (len(grid[0]) * cw, len(grid) * ch), DEFAULT_BG)
    draw = ImageDraw.Draw(img)
    for row, cells in enumerate(grid):
        for col, (char, fg, bg, bold, italic) in enumerate(cells):
            x, y = col * cw, row * ch
            if cursor == (col, row):
                # A block of one colour, as a terminal with a cursor colour draws it. Swapping the
                # cell's own colours turns the cursor on a find match into a dark hole in it.
                fg, bg = DEFAULT_BG, DEFAULT_FG
            if bg != DEFAULT_BG or cursor == (col, row):
                draw.rectangle([x, y, x + cw - 1, y + ch - 1], fill=bg)
            if char in SEGMENTS:
                seg, mx, my = SEGMENTS[char], x + cw // 2, y + ch // 2
                if "u" in seg: draw.line([(mx, y), (mx, my)], fill=fg)
                if "d" in seg: draw.line([(mx, my), (mx, y + ch)], fill=fg)
                if "l" in seg: draw.line([(x, my), (mx, my)], fill=fg)
                if "r" in seg: draw.line([(mx, my), (x + cw, my)], fill=fg)
            elif char != " ":
                draw.text((x, y), char, font=fonts[bold + 2 * italic], fill=fg)
    return img


class Pane:
    """A merl running on a tmux socket of its own, with an empty HOME so the user's config and
    themes stay out of the recording."""

    def __init__(self, binary, merl_args, project, cols, rows):
        self.sock = "cast%d" % os.getpid()
        self.home = tempfile.mkdtemp(prefix="cast-home-")
        cmd = "env HOME=%s COLORTERM=truecolor %s" % (
            shlex.quote(self.home), shlex.join([os.path.abspath(binary)] + merl_args))
        self.tmux("new-session", "-d", "-x", str(cols), "-y", str(rows), "-c", project, cmd)

    def tmux(self, *args):
        out = subprocess.run(["tmux", "-L", self.sock, "-f", "/dev/null", *args],
                             capture_output=True, text=True)
        if out.returncode:
            sys.exit("tmux %s: %s" % (args[0], out.stderr.strip()))
        return out.stdout

    def frame(self):
        return (self.tmux("capture-pane", "-p", "-e", "-N", "-t", "0"),
                self.tmux("display", "-p", "-t", "0", "#{cursor_x},#{cursor_y},#{cursor_flag}").strip())

    def wait(self, text, timeout=10):
        deadline = time.monotonic() + timeout
        while time.monotonic() < deadline:
            if text in self.tmux("capture-pane", "-p", "-t", "0"):
                return
            time.sleep(0.05)
        sys.exit("waited %gs for %r, the pane never showed it" % (timeout, text))

    def close(self):
        subprocess.run(["tmux", "-L", self.sock, "kill-server"], capture_output=True)
        shutil.rmtree(self.home, ignore_errors=True)


def run(pane, steps, fps, key_delay, tail):
    """Feed the steps to the pane while a timer samples it, and give back (frame, seconds) pairs."""
    shots, stop = [], threading.Event()
    # Leading waits are the app starting up; sampling them would open the GIF on a blank pane.
    while steps and steps[0].split(" ")[0] in ("wait", "sleep"):
        verb, _, rest = steps.pop(0).partition(" ")
        if verb == "wait":
            pane.wait(rest.strip())
        else:
            time.sleep(float(rest))

    def sampler():
        while not stop.is_set():
            start = time.monotonic()
            shots.append((start, pane.frame()))
            stop.wait(max(0.0, 1 / fps - (time.monotonic() - start)))

    thread = threading.Thread(target=sampler, daemon=True)
    thread.start()
    for line in steps:
        verb, _, rest = line.partition(" ")
        if verb == "wait":
            pane.wait(rest.strip())
        elif verb == "sleep":
            time.sleep(float(rest))
        elif verb == "key":
            for key in rest.split():
                pane.tmux("send-keys", "-t", "0", key)
                time.sleep(key_delay)
        elif verb == "type":
            for char in rest:
                pane.tmux("send-keys", "-t", "0", "-l", char)
                time.sleep(key_delay)
        else:
            sys.exit("step %r: expected wait, key, type or sleep" % line)
    time.sleep(tail)
    stop.set()
    thread.join()

    merged = []
    for i, (when, frame) in enumerate(shots):
        end = shots[i + 1][0] if i + 1 < len(shots) else when + 1 / fps
        if merged and merged[-1][0] == frame:
            merged[-1][1] += end - when
        else:
            merged.append([frame, end - when])
    return merged


def main():
    p = argparse.ArgumentParser(description=__doc__,
                                formatter_class=argparse.RawDescriptionHelpFormatter)
    p.add_argument("--bin", required=True, help="the merl binary to record")
    p.add_argument("--keys", help="steps file; omit with --selftest")
    p.add_argument("-o", "--out", help="the GIF to write")
    p.add_argument("--project", default=".", help="directory merl runs in (default: .)")
    p.add_argument("--size", default="90x20", help="pane in cells (default: 90x20)")
    p.add_argument("--font-size", type=int, default=18)
    p.add_argument("--fps", type=float, default=10)
    p.add_argument("--key-delay", type=float, default=0.12, help="pause after each keystroke")
    p.add_argument("--tail", type=float, default=1.2, help="seconds held on the last frame")
    p.add_argument("--selftest", action="store_true", help="record tutor/notes and check the GIF")
    p.add_argument("merl_args", nargs=argparse.REMAINDER, help="after --, arguments for merl")
    args = p.parse_args()

    merl_args = args.merl_args[1:] if args.merl_args[:1] == ["--"] else args.merl_args
    if args.selftest:
        repo = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
        sample = os.path.join(tempfile.mkdtemp(prefix="cast-notes-"), "notes")
        shutil.copytree(os.path.join(repo, "tutor", "notes"), sample)
        steps, args.project = SELFTEST_STEPS, sample
        merl_args, args.out = ["store.py"], args.out or tempfile.mktemp(suffix=".gif")
    elif not args.keys or not args.out:
        p.error("--keys and --out are required without --selftest")
    else:
        with open(args.keys) as f:
            steps = [l.strip() for l in f if l.strip() and not l.startswith("#")]

    cols, rows = (int(n) for n in args.size.split("x"))
    pane = Pane(args.bin, merl_args, args.project, cols, rows)
    try:
        frames = run(pane, steps, args.fps, args.key_delay, args.tail)
    finally:
        pane.close()

    fonts = [ImageFont.truetype(FONT, args.font_size, index=i) for i in range(4)]
    ascent, descent = fonts[0].getmetrics()
    cell = (round(fonts[0].getlength("M")), ascent + descent)
    images, durations = [], []
    for (capture, cursor), seconds in frames:
        x, y, shown = (int(n) for n in cursor.split(","))
        images.append(render(parse(capture, cols, rows), (x, y) if shown else None, fonts, cell))
        durations.append(max(30, round(seconds * 1000)))
    # The palette comes from frames spread over the run: taken from the first one alone, a pane
    # that has not painted yet collapses every later frame into its two colours.
    strip = images[:: max(1, len(images) // 8)][:8]
    sheet = Image.new("RGB", (strip[0].width, strip[0].height * len(strip)))
    for i, im in enumerate(strip):
        sheet.paste(im, (0, i * strip[0].height))
    palette = sheet.quantize(colors=256)
    flat = [im.quantize(palette=palette, dither=Image.Dither.NONE) for im in images]
    flat[0].save(args.out, save_all=True, append_images=flat[1:], duration=durations, loop=0,
                 optimize=True)
    print("%s  %d frames  %.1fs  %.1f MB" % (args.out, len(flat), sum(durations) / 1000,
                                             os.path.getsize(args.out) / 1e6))
    if args.selftest:
        assert len(flat) >= 5, "%d frames: the typing never reached merl" % len(flat)
        written = Image.open(args.out)
        assert written.n_frames == len(flat)
        written.seek(written.n_frames - 1)
        colours = len(written.convert("RGB").getcolors(1 << 20))
        assert colours > 16, "last frame has %d colours: the palette collapsed" % colours
        print("selftest ok")


if __name__ == "__main__":
    main()
