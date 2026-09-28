"""Renders a `tmux capture-pane -e` dump into a PNG, in the colours of the owner's terminal:
Ghostty's TokyoNight Moon, or the Ghostty theme named after the PNG.

    python3 tools/shot.py CAPTURE OUT.png ['TokyoNight Day']

The dump is read by tools/cast.py, which the screencasts use: the SGR state carries over from
one line to the next as tmux emits it, and the 16 basic colours are the terminal's.
"""

import os
import sys

from PIL import Image, ImageDraw, ImageFont

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import cast  # noqa: E402

SIZE = 26
# Menlo.ttc: 0 regular, 1 bold, 2 italic, 3 bold italic.
FACES = {(False, False): 0, (True, False): 1, (False, True): 2, (True, True): 3}


def main():
    capture, out = sys.argv[1], sys.argv[2]
    if len(sys.argv) > 3:
        cast.use_ghostty(sys.argv[3])
    text = open(capture, encoding="utf-8").read().rstrip("\n")
    width = max(len(cast.SGR.sub("", line)) for line in text.split("\n"))
    grid = cast.parse(text, width, text.count("\n") + 1)

    fonts = {k: ImageFont.truetype(cast.FONT, SIZE, index=i) for k, i in FACES.items()}
    cw = round(fonts[(False, False)].getlength("M"))
    ch = SIZE * 2
    # The background merl paints on most cells is the theme's own; use it for the padding too.
    counts = {}
    for row in grid:
        for _, _, bg, _, _ in row:
            counts[bg] = counts.get(bg, 0) + 1
    page_bg = max(counts, key=counts.get)

    pad = cw
    img = Image.new("RGB", (width * cw + pad * 2, len(grid) * ch + pad * 2), page_bg)
    draw = ImageDraw.Draw(img)
    for y, row in enumerate(grid):
        for x, (char, fg, bg, bold, italic) in enumerate(row):
            px, py = pad + x * cw, pad + y * ch
            if bg != page_bg:
                draw.rectangle([px, py, px + cw - 1, py + ch - 1], fill=bg)
            if char != " ":
                draw.text((px, py + SIZE // 4), char, font=fonts[(bold, italic)], fill=fg)
    # 256 colours keep a terminal screenshot sharp at about a third of the size.
    img.quantize(256).save(out, optimize=True)
    print(out)


main()
