#!/usr/bin/env python3
"""Convert a VS Code JSON colour theme into a TextMate .tmTheme for syntect.

    python3 tools/vscode2tmtheme.py in.json out.tmTheme "Theme Name"

Only the keys merl needs are carried over: the editor chrome colours and
`tokenColors`. `semanticTokenColors` is dropped (syntect cannot use it).
`#RRGGBBAA` global colours are composited over `editor.background`; alpha on
token foregrounds is stripped.
"""

import json
import plistlib
import sys

# tmTheme global key <- VS Code colors key.
GLOBALS = [
    ("background", "editor.background"),
    ("foreground", "editor.foreground"),
    ("caret", "editorCursor.foreground"),
    ("lineHighlight", "editor.lineHighlightBackground"),
    ("selection", "editor.selectionBackground"),
    ("gutter", "editorGutter.background", "editor.background"),
    ("gutterForeground", "editorLineNumber.foreground"),
    ("findHighlight", "editor.findMatchBackground"),
]


def rgb(c):
    """#RGB / #RRGGBB / #RRGGBBAA -> (r, g, b, a)."""
    h = c.lstrip("#")
    if len(h) in (3, 4):
        h = "".join(ch * 2 for ch in h)
    v = [int(h[i : i + 2], 16) for i in range(0, len(h), 2)]
    return (v[0], v[1], v[2], v[3] if len(v) > 3 else 255)


def over(color, background):
    """Composite `color` over `background` and return an opaque #RRGGBB."""
    r, g, b, a = rgb(color)
    br, bg_, bb, _ = rgb(background)
    if a == 255:
        return "#%02X%02X%02X" % (r, g, b)
    def mix(f, b):
        return round((f * a + b * (255 - a)) / 255)

    return "#%02X%02X%02X" % (mix(r, br), mix(g, bg_), mix(b, bb))


def opaque(color):
    """Drop any alpha byte without compositing."""
    r, g, b, _ = rgb(color)
    return "#%02X%02X%02X" % (r, g, b)


def convert(theme, name):
    colors = theme.get("colors", {})
    bg = colors.get("editor.background", "#000000")

    top = {}
    for key, *sources in GLOBALS:
        value = next((colors[s] for s in sources if s in colors), None)
        if value:
            top[key] = over(value, bg)
    # A selection in the cursor line's colour makes a selected line look like the cursor line.
    # Shokunin Light does that; its terminal selection is the theme's own selection colour.
    same = "selection" in top and top["selection"] == top.get("lineHighlight")
    if same and "terminal.selectionBackground" in colors:
        top["selection"] = over(colors["terminal.selectionBackground"], bg)

    settings = [{"settings": top}]
    for token in theme.get("tokenColors", []):
        scope = token.get("scope", "")
        if isinstance(scope, list):
            scope = ", ".join(scope)
        s = token.get("settings", {})
        out = {}
        for key in ("foreground", "background"):
            if s.get(key):
                out[key] = opaque(s[key])
        style = s.get("fontStyle", "").strip()
        if style and style != "normal":
            out["fontStyle"] = style
        entry = {"scope": scope, "settings": out}
        if token.get("name"):
            entry["name"] = token["name"]
        settings.append(entry)

    return {"name": name, "settings": settings}


def main():
    if len(sys.argv) != 4:
        sys.exit(__doc__)
    src, dst, name = sys.argv[1:]
    with open(src, encoding="utf-8") as f:
        theme = json.load(f)
    with open(dst, "wb") as f:
        plistlib.dump(convert(theme, name), f, fmt=plistlib.FMT_XML)


if __name__ == "__main__":
    main()
