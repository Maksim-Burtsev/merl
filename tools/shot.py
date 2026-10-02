"""Renders a `tmux capture-pane -e` dump into a PNG, in the colours of the owner's terminal:
Ghostty's TokyoNight Moon, or the Ghostty theme named after the PNG.

    python3 tools/shot.py CAPTURE OUT.png ['TokyoNight Day']
    python3 tools/shot.py --selftest

The dump is read by tools/cast.py, which the screencasts use: the SGR state carries over from
one line to the next as tmux emits it, and the 16 basic colours are the terminal's.
"""

import os
import sys

from PIL import ImageFont

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
sys.dont_write_bytecode = True  # no __pycache__ left in tools/
import cast  # noqa: E402


def shot(text):
    """The capture drawn by cast.py's renderer: the font, size and box lines of the screencasts."""
    width = max(len(cast.SGR.sub("", line)) for line in text.split("\n"))
    grid = cast.parse(text, width, text.count("\n") + 1)
    fonts = [ImageFont.truetype(cast.FONT, cast.FONT_SIZE, index=i) for i in range(4)]
    cell = (round(fonts[0].getlength("M")), sum(fonts[0].getmetrics()))
    return cast.render(grid, None, fonts, cell), cell


def selftest():
    # The third line goes on in the second one's background, not the first one's (#441), and
    # `\e[34m` is the terminal's blue, not xterm's near-black.
    img, (cw, ch) = shot("\x1b[48;2;9;9;9mAAAA\n\x1b[48;2;1;2;3mx\ny\n\x1b[34m█\n│\n│")
    cell = lambda x, y: img.getpixel((x * cw + cw // 2, y * ch + ch // 2))
    assert cell(2, 2) == (1, 2, 3), "the third line lost the background it carries on"
    assert cell(0, 3) == cast.ANSI[4] == (0x82, 0xaa, 0xff), "the blue is not the terminal's"
    assert all(img.getpixel((cw // 2, y)) != img.getpixel((cw + cw // 2, y))
               for y in range(4 * ch, 6 * ch)), "the box line breaks between rows"
    print("selftest ok")


def main():
    if sys.argv[1:] == ["--selftest"]:
        return selftest()
    capture, out = sys.argv[1], sys.argv[2]
    if len(sys.argv) > 3:
        cast.use_ghostty(sys.argv[3])
    img, _ = shot(open(capture, encoding="utf-8").read().rstrip("\n"))
    # 256 colours keep a terminal screenshot sharp at about a third of the size.
    img.quantize(256).save(out, optimize=True)
    print(out)


main()
