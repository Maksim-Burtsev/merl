#!/usr/bin/env python3
"""Record merl in a tmux pane as an animated GIF that reads like a screen recording.

    cargo build --release
    tools/cast.py --bin target/release/merl --keys steps.txt -o after.gif -- tutor/notes/store.py

One run records one binary. A before/after pair is two runs of the same steps file: one against
a build of master, one against the branch. Build master in its own worktree with its own
CARGO_TARGET_DIR -- worktrees sharing a target dir hand you a stale binary.

The steps file is one verb per line, `#` comments and blank lines ignored:

    wait store.py      poll the pane until the text shows up (10 s, then fail)
    key Down Down d    tmux key names, several per line, a person's pace apart (--key-delay);
                       each shows in a keycap in the bottom right corner while it acts
    type Duration      the literal characters, at typing speed (--type-delay); no keycap
    sleep 0.8          seconds

The pane is sampled on a timer and identical frames are merged, so the GIF keeps the real
timing of the run instead of one frame per keystroke. Keep a recording under ~15 s. How to walk
the code in one, AGENTS.md `## Screencasts` says.

Record outside the repository. merl walks up to the project root, so a sample project inside
the checkout puts merl's own tree in the shot and makes `s` grep thousands of files -- the UI
blocks and the keystrokes are lost:

    cp -R tutor/notes /tmp/notes && tools/cast.py --project /tmp/notes ...

macOS only: the renderer reads Menlo out of /System/Library/Fonts/Menlo.ttc.
"""
import argparse, os, re, shlex, shutil, subprocess, sys, tempfile, threading, time
from PIL import Image, ImageDraw, ImageFont

FONT = "/System/Library/Fonts/Menlo.ttc"  # index 0 regular, 1 bold, 2 italic, 3 bold-italic
FONT_SIZE = 18  # the screencasts' and tools/shot.py's, so every picture reads in one size
# The terminal's own colours, as the owner's Ghostty draws them with its TokyoNight Moon theme:
# the default foreground and background, then the 16 ANSI colours. merl paints its theme in
# 24-bit colour, but the gutter marks and the review panel's letters are basic colours whose
# shade the terminal picks; xterm's blue, (0, 0, 238), is nearly black on a dark theme.
DEFAULT_FG, DEFAULT_BG = (0xc8, 0xd3, 0xf5), (0x22, 0x24, 0x36)
ANSI = [tuple(bytes.fromhex(h)) for h in ("1b1d2b ff757f c3e88d ffc777 82aaff c099ff 86e1fc 828bb8 "
                                          "444a73 ff757f c3e88d ffc777 82aaff c099ff 86e1fc c8d3f5"
                                          ).split()]
GHOSTTY_THEMES = "/Applications/Ghostty.app/Contents/Resources/ghostty/themes"
# xterm's colours, for telling cells apart rather than drawing them: its defaults and 16 colours
# differ from each other and from merl's themes, where Moon's bright colours repeat the normal
# ones and its defaults are merl's default theme's.
XTERM = ((0xcc, 0xcc, 0xcc), (0x10, 0x10, 0x10),
         [(0, 0, 0), (205, 0, 0), (0, 205, 0), (205, 205, 0), (0, 0, 238), (205, 0, 205),
          (0, 205, 205), (229, 229, 229), (127, 127, 127), (255, 0, 0), (0, 255, 0),
          (255, 255, 0), (92, 92, 255), (255, 0, 255), (0, 255, 255), (255, 255, 255)])
SELFTEST_STEPS = ["wait store.py", "key s", "sleep 0.4", "type config", "sleep 1.2"]
# Menlo's box glyphs leave gaps at a cell this tall, so they are drawn as lines through the centre.
SEGMENTS = {"│": "ud", "┃": "ud", "─": "lr", "━": "lr", "┌": "dr", "┐": "dl", "└": "ur", "┘": "ul",
            "├": "udr", "┤": "udl", "┬": "dlr", "┴": "ulr", "┼": "udlr",
            "╭": "dr", "╮": "dl", "╰": "ur", "╯": "ul"}
SGR = re.compile(r"\x1b\[([0-9;]*)m")
HOLD = 1.5
CAPS = {"M-Right": "⌥→", "M-Left": "⌥←", "Right": "→", "Left": "←", "Down": "↓", "Up": "↑",
        "Enter": "Enter", "Escape": "Esc", "Tab": "Tab", "BSpace": "⌫", "C-d": "Ctrl+D",
        "C-u": "Ctrl+U", "C-End": "Ctrl+End", "C-Home": "Ctrl+Home", "PageDown": "PgDn",
        "PageUp": "PgUp", "S-Up": "⇧↑", "S-Down": "⇧↓"}
SANS = "/System/Library/Fonts/SFNS.ttf"


def use_ghostty(theme):
    """The terminal's colours from a Ghostty theme, a name such as `TokyoNight Day` or a path."""
    global DEFAULT_FG, DEFAULT_BG
    path = theme if os.sep in theme else os.path.join(GHOSTTY_THEMES, theme)
    for line in open(path):
        key, _, value = (s.strip() for s in line.partition("="))
        if key == "palette":
            n, _, value = value.partition("=")
            ANSI[int(n)] = tuple(bytes.fromhex(value.strip().lstrip("#")))
        elif key == "foreground":
            DEFAULT_FG = tuple(bytes.fromhex(value.lstrip("#")))
        elif key == "background":
            DEFAULT_BG = tuple(bytes.fromhex(value.lstrip("#")))


def xterm(n, ansi):
    if n < 16:
        return ansi[n]
    if n < 232:
        s, n = (0, 95, 135, 175, 215, 255), n - 16
        return (s[n // 36], s[(n // 6) % 6], s[n % 6])
    v = 8 + (n - 232) * 10
    return (v, v, v)


def apply_sgr(params, st, palette):
    fg, bg, bold, italic, rev = st
    default_fg, default_bg, ansi = palette
    codes = [int(p) for p in params.split(";") if p] or [0]
    i = 0
    while i < len(codes):
        c = codes[i]
        if c == 0: fg, bg, bold, italic, rev = default_fg, default_bg, False, False, False
        elif c == 1: bold = True
        elif c == 3: italic = True
        elif c == 7: rev = True
        elif c == 22: bold = False
        elif c == 23: italic = False
        elif c == 27: rev = False
        elif c == 39: fg = default_fg
        elif c == 49: bg = default_bg
        elif 30 <= c <= 37: fg = xterm(c - 30, ansi)
        elif 90 <= c <= 97: fg = xterm(c - 82, ansi)
        elif 40 <= c <= 47: bg = xterm(c - 40, ansi)
        elif 100 <= c <= 107: bg = xterm(c - 92, ansi)
        elif c in (38, 48):
            if codes[i + 1:i + 2] == [2]:
                col, i = tuple(codes[i + 2:i + 5]), i + 4
            else:
                col, i = xterm(codes[i + 2] if i + 2 < len(codes) else 0, ansi), i + 2
            fg, bg = (col, bg) if c == 38 else (fg, col)
        i += 1
    return fg, bg, bold, italic, rev


def parse(capture, cols, rows, palette=None):
    """A capture-pane -e dump into a grid of (char, fg, bg, bold, italic), in the terminal's
    colours or in `palette`, a (foreground, background, 16 colours) such as XTERM.

    SGR state carries across lines: capture-pane only emits the changes, so a parser that resets
    per line paints whole rows in the wrong background."""
    palette = palette or (DEFAULT_FG, DEFAULT_BG, ANSI)
    st = (palette[0], palette[1], False, False, False)
    grid = []
    for raw in capture.split("\n")[:rows]:
        cells = []
        pos = 0
        for m in SGR.finditer(raw):
            fg, bg, bold, italic, rev = st
            cells += [(ch, bg, fg, bold, italic) if rev else (ch, fg, bg, bold, italic)
                      for ch in raw[pos:m.start()]]
            st, pos = apply_sgr(m.group(1), st, palette), m.end()
        fg, bg, bold, italic, rev = st
        cells += [(ch, bg, fg, bold, italic) if rev else (ch, fg, bg, bold, italic)
                  for ch in raw[pos:]]
        grid.append((cells + [(" ", fg, bg, False, False)] * cols)[:cols])
    blank = [(" ", palette[0], palette[1], False, False)] * cols
    return (grid + [blank] * rows)[:rows]


def block(char, x, y, cw, ch):
    """The rectangle a block element fills, as a terminal draws it: to the edges of the cell.
    Menlo's glyphs stop short of them, and merl's gutter bars would break between the rows."""
    n = ord(char) - 0x2580
    if 1 <= n <= 8:  # ▁ … █, the lower n eighths
        return [x, y + ch - ch * n // 8, x + cw - 1, y + ch - 1]
    if 9 <= n <= 15:  # ▉ … ▏, the left 16 - n eighths
        return [x, y, x + cw * (16 - n) // 8 - 1, y + ch - 1]
    return None


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
            elif rect := block(char, x, y, cw, ch):
                draw.rectangle(rect, fill=fg)
            # U+FE0F makes the emoji before it two cells wide. Read a char to a cell, the selector
            # lands in the second cell, which a terminal leaves blank; Menlo would draw a box.
            elif char not in (" ", "️"):
                draw.text((x, y), char, font=fonts[bold + 2 * italic], fill=fg)
    return img


def draw_cap(frame, key, scale, above, opacity=1.0):
    if not key:
        return frame
    font = ImageFont.truetype(SANS, round(44 * scale))
    font.set_variation_by_name("Semibold")
    label = CAPS.get(key, key)
    layer = Image.new("RGBA", frame.size)
    d = ImageDraw.Draw(layer)
    h = round(88 * scale)
    w = max(h, round(d.textlength(label, font=font) + h * 0.55))
    x = frame.width - round(20 * scale) - w
    y = frame.height - above - round(14 * scale) - h
    a = lambda v: round(v * opacity)
    d.rounded_rectangle((x, y, x + w, y + h), round(h * 0.2), fill=(47, 51, 77, a(250)),
                        outline=(130, 139, 184, a(160)), width=max(2, round(1.2 * scale)))
    d.text((x + w / 2, y + h / 2), label, font=font, fill=(230, 235, 255, a(255)), anchor="mm")
    return Image.alpha_composite(frame.convert("RGBA"), layer).convert("RGB")


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


def run(pane, steps, fps, key_delay, type_delay, tail):
    shots, presses, stop = [], [], threading.Event()
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
                presses.append((time.monotonic(), key))
                pane.tmux("send-keys", "-t", "0", key)
                time.sleep(key_delay)
        elif verb == "type":
            for char in rest:
                pane.tmux("send-keys", "-t", "0", "-l", char)
                time.sleep(type_delay)
        else:
            sys.exit("step %r: expected wait, key, type or sleep" % line)
    time.sleep(tail)
    stop.set()
    thread.join()

    def cap(when):
        last = next((p for p in reversed(presses) if p[0] <= when), None)
        return last[1] if last and when - last[0] < HOLD else None

    merged = []
    for i, (when, frame) in enumerate(shots):
        end = shots[i + 1][0] if i + 1 < len(shots) else when + 1 / fps
        if merged and merged[-1][:2] == [frame, cap(when)]:
            merged[-1][2] += end - when
        else:
            merged.append([frame, cap(when), end - when])
    return merged


def save_gif(images, durations, out):
    """The frames as a looping GIF, each shown for its duration in milliseconds."""
    # The palette comes from frames spread over the run: taken from the first one alone, a pane
    # that has not painted yet collapses every later frame into its two colours.
    strip = images[:: max(1, len(images) // 8)][:8]
    sheet = Image.new("RGB", (strip[0].width, strip[0].height * len(strip)))
    for i, im in enumerate(strip):
        sheet.paste(im, (0, i * strip[0].height))
    palette = sheet.quantize(colors=256)
    flat = [im.quantize(palette=palette, dither=Image.Dither.NONE) for im in images]
    flat[0].save(out, save_all=True, append_images=flat[1:], duration=durations, loop=0,
                 optimize=True)
    print("%s  %d frames  %.1fs  %.1f MB" % (out, len(flat), sum(durations) / 1000,
                                             os.path.getsize(out) / 1e6))


def main():
    p = argparse.ArgumentParser(description=__doc__,
                                formatter_class=argparse.RawDescriptionHelpFormatter)
    p.add_argument("--bin", required=True, help="the merl binary to record")
    p.add_argument("--keys", help="steps file; omit with --selftest")
    p.add_argument("-o", "--out", help="the GIF to write")
    p.add_argument("--project", default=".", help="directory merl runs in (default: .)")
    p.add_argument("--size", default="90x20", help="pane in cells (default: 90x20)")
    p.add_argument("--font-size", type=int, default=FONT_SIZE)
    p.add_argument("--fps", type=float, default=10)
    p.add_argument("--key-delay", type=float, default=0.35,
                   help="pause after each `key` keystroke: a person's pace, the eye follows it")
    p.add_argument("--type-delay", type=float, default=0.12, help="pause after each typed char")
    p.add_argument("--tail", type=float, default=1.2, help="seconds held on the last frame")
    p.add_argument("--ghostty", metavar="THEME",
                   help="the terminal's colours from this Ghostty theme (default: TokyoNight Moon)")
    p.add_argument("--selftest", action="store_true", help="record tutor/notes and check the GIF")
    p.add_argument("merl_args", nargs=argparse.REMAINDER, help="after --, arguments for merl")
    args = p.parse_args()

    merl_args = args.merl_args[1:] if args.merl_args[:1] == ["--"] else args.merl_args
    if args.selftest and args.ghostty:
        p.error("--selftest checks the default colours; drop --ghostty")
    if args.ghostty:
        use_ghostty(args.ghostty)
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
        frames = run(pane, steps, args.fps, args.key_delay, args.type_delay, args.tail)
    finally:
        pane.close()

    fonts = [ImageFont.truetype(FONT, args.font_size, index=i) for i in range(4)]
    ascent, descent = fonts[0].getmetrics()
    cell = (round(fonts[0].getlength("M")), ascent + descent)
    images, durations = [], []
    for (capture, cursor), key, seconds in frames:
        x, y, shown = (int(n) for n in cursor.split(","))
        image = render(parse(capture, cols, rows), (x, y) if shown else None, fonts, cell)
        images.append(draw_cap(image, key, args.font_size / 18, cell[1]))
        durations.append(max(30, round(seconds * 1000)))
    save_gif(images, durations, args.out)
    if args.selftest:
        assert len(images) >= 5, "%d frames: the typing never reached merl" % len(images)
        keys = [key for _, key, _ in frames]
        assert "s" in keys and keys[-1] is None, "the keycap: %r" % keys
        written = Image.open(args.out)
        assert written.n_frames == len(images)
        written.seek(written.n_frames - 1)
        colours = len(written.convert("RGB").getcolors(1 << 20))
        assert colours > 16, "last frame has %d colours: the palette collapsed" % colours
        red = (255, 0, 0)
        bar = render([[("\u258e", red, DEFAULT_BG, False, False)]] * 2, None, fonts, cell)
        assert all(bar.getpixel((0, y)) == red for y in range(bar.height)), "the ▎ bar has gaps"
        # A second line goes on in the first one's background; the default text and `\e[34m` are
        # TokyoNight Moon's, and a Ghostty theme replaces both.
        g = parse("\x1b[48;2;1;2;3mx\ny\n\x1b[34mM", 1, 3)
        assert g[1][0][2] == (1, 2, 3), "the second line lost the background it carries on"
        assert g[0][0][1] == (0xc8, 0xd3, 0xf5), "the default text is not Moon's"
        assert g[2][0][1] == (0x82, 0xaa, 0xff), "the blue is not Moon's"
        theme = os.path.join(tempfile.mkdtemp(prefix="cast-theme-"), "day")
        with open(theme, "w") as f:
            f.write("background = #e1e2e7\nforeground = #3760bf\npalette = 4=#2e7de9\n")
        use_ghostty(theme)
        g = parse("x\n\x1b[34mM", 1, 2)
        assert g[0][0][1:3] == ((0x37, 0x60, 0xbf), (0xe1, 0xe2, 0xe7)), "the theme's fg/bg not taken"
        assert g[1][0][1] == (0x2e, 0x7d, 0xe9), "the theme's blue not taken"
        print("selftest ok")


if __name__ == "__main__":
    main()
