"""Renders a `tmux capture-pane -e` dump into a PNG.

    python3 tools/shot.py CAPTURE OUT.png
"""

import re
import sys

from PIL import Image, ImageDraw, ImageFont

FONT = "/System/Library/Fonts/Menlo.ttc"
SIZE = 26
# Menlo.ttc: 0 regular, 1 bold, 2 italic, 3 bold italic.
FACES = {(False, False): 0, (True, False): 1, (False, True): 2, (True, True): 3}
SGR = re.compile(r"\x1b\[([0-9;]*)m")
# The 16 ANSI colours, as xterm renders them; merl itself always emits 24-bit colour, so these
# only turn up in whatever the shell left on screen.
BASIC = [
    (0, 0, 0), (205, 0, 0), (0, 205, 0), (205, 205, 0),
    (0, 0, 238), (205, 0, 205), (0, 205, 205), (229, 229, 229),
    (127, 127, 127), (255, 0, 0), (0, 255, 0), (255, 255, 0),
    (92, 92, 255), (255, 0, 255), (0, 255, 255), (255, 255, 255),
]


def xterm256(n):
    if n < 16:
        return BASIC[n]
    if n < 232:
        n -= 16
        steps = [0, 95, 135, 175, 215, 255]
        return steps[n // 36], steps[n // 6 % 6], steps[n % 6]
    v = 8 + (n - 232) * 10
    return v, v, v


def cells(line):
    """One screen line as (char, fg, bg, bold, italic) cells; `None` means the default colour."""
    out, fg, bg, bold, italic, i = [], None, None, False, False, 0
    for m in SGR.finditer(line):
        for ch in line[i:m.start()]:
            out.append((ch, fg, bg, bold, italic))
        i = m.end()
        codes = [int(c or 0) for c in m.group(1).split(";")]
        j = 0
        while j < len(codes):
            c = codes[j]
            if c == 0:
                fg, bg, bold, italic = None, None, False, False
            elif c == 1:
                bold = True
            elif c == 3:
                italic = True
            elif c in (22, 23):
                bold, italic = (False, italic) if c == 22 else (bold, False)
            elif c == 39:
                fg = None
            elif c == 49:
                bg = None
            elif 30 <= c <= 37:
                fg = BASIC[c - 30]
            elif 40 <= c <= 47:
                bg = BASIC[c - 40]
            elif 90 <= c <= 97:
                fg = BASIC[c - 82]
            elif 100 <= c <= 107:
                bg = BASIC[c - 92]
            elif c in (38, 48) and j + 1 < len(codes):
                colour = None
                if codes[j + 1] == 2 and j + 4 < len(codes):
                    colour = tuple(codes[j + 2:j + 5])
                    j += 4
                elif codes[j + 1] == 5 and j + 2 < len(codes):
                    colour = xterm256(codes[j + 2])
                    j += 2
                if c == 38:
                    fg = colour
                else:
                    bg = colour
            j += 1
    for ch in line[i:]:
        out.append((ch, fg, bg, bold, italic))
    return out


def main():
    capture, out = sys.argv[1], sys.argv[2]
    grid = [cells(line.rstrip("\n")) for line in open(capture, encoding="utf-8")]
    grid = [row for row in grid if row]
    width = max(len(row) for row in grid)

    fonts = {k: ImageFont.truetype(FONT, SIZE, index=i) for k, i in FACES.items()}
    cw = round(fonts[(False, False)].getlength("M"))
    ch = SIZE * 2
    # The background merl paints on most cells is the theme's own; use it for the padding too.
    counts = {}
    for row in grid:
        for _, _, bg, _, _ in row:
            if bg:
                counts[bg] = counts.get(bg, 0) + 1
    page_bg = max(counts, key=counts.get) if counts else (0, 0, 0)
    page_fg = (255, 255, 255) if sum(page_bg) < 384 else (0, 0, 0)

    pad = cw
    img = Image.new("RGB", (width * cw + pad * 2, len(grid) * ch + pad * 2), page_bg)
    draw = ImageDraw.Draw(img)
    for y, row in enumerate(grid):
        for x, (char, fg, bg, bold, italic) in enumerate(row):
            px, py = pad + x * cw, pad + y * ch
            if bg and bg != page_bg:
                draw.rectangle([px, py, px + cw - 1, py + ch - 1], fill=bg)
            if char != " ":
                draw.text((px, py + SIZE // 4), char, font=fonts[(bold, italic)], fill=fg or page_fg)
    # 256 colours keep a terminal screenshot sharp at about a third of the size.
    img.quantize(256).save(out, optimize=True)
    print(out)


main()
