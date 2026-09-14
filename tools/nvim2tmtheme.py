#!/usr/bin/env python3
"""Convert a Neovim highlight dump into a TextMate .tmTheme for syntect.

    python3 tools/nvim2tmtheme.py hl.json out.tmTheme "Theme Name"

`hl.json` is `vim.api.nvim_get_hl(0, {})` as JSON, which `tools/port-theme.sh`
writes: computed colours, so Lua themes, vimscript themes and variants all look
the same here. Every theme goes through one fixed group -> scope table.
"""

import json
import plistlib
import sys

# tmTheme global key <- (Neovim group, attribute).
GLOBALS = [
    ("background", "Normal", "bg"),
    ("foreground", "Normal", "fg"),
    ("lineHighlight", "CursorLine", "bg"),
    ("selection", "Visual", "bg"),
    ("findHighlight", "Search", "bg"),
    ("findHighlightForeground", "Search", "fg"),
    ("gutterForeground", "LineNr", "fg"),
]

# TextMate scopes <- Neovim groups, most specific first: the first group that sets a
# foreground wins. bat's grammars hand out scopes differently from tree-sitter captures,
# so this is where a group that lands on the wrong tokens gets fixed, for every theme.
SCOPES = [
    ("comment", ["@comment", "Comment"]),
    ("keyword, storage", ["@keyword", "Keyword", "Statement"]),
    ("keyword.operator", ["@keyword.operator", "Operator"]),
    ("entity.name.function, support.function", ["@function", "Function"]),
    ("entity.name.type, entity.name.class, support.type", ["@type", "Type"]),
    ("string", ["@string", "String"]),
    ("constant.numeric", ["@number", "Number", "@constant", "Constant"]),
    ("constant.language", ["@boolean", "Boolean", "@constant", "Constant"]),
    ("constant", ["@constant", "Constant"]),
    ("variable", ["@variable", "Identifier"]),
    ("variable.parameter", ["@variable.parameter", "@parameter"]),
    (
        "variable.other.member, support.type.property-name",
        ["@property", "@variable.member", "@field"],
    ),
    ("entity.name.tag", ["@tag", "Tag"]),
    ("punctuation", ["@punctuation", "@punctuation.delimiter", "Delimiter"]),
    ("markup.heading", ["@markup.heading", "Title"]),
    ("invalid", ["DiagnosticError", "@error", "Error"]),
]

# The names infrastructure grammars emit (#16), which no code group reaches. Here a group painted
# in the Normal foreground does not count, so the next candidate gets its turn: left in the text
# colour, a TOML table or a Makefile variable reads as no highlighting at all.
INFRA_SCOPES = [
    (
        "entity.name.tag.toml, variable.other.readwrite.terraform, "
        "variable.other.member.terraform, variable.other.env",
        ["@property", "@variable.member", "@field", "Identifier", "@function", "Function"],
    ),
    (
        "entity.name.table.toml, entity.section.ini, entity.name.context.location.nginx",
        ["@markup.heading", "Title", "@type", "Type"],
    ),
    (
        "variable.other.makefile, variable.parameter.makefile, "
        "variable.language.automatic.makefile, variable.other.nginx",
        ["@constant.macro", "Macro", "PreProc", "@constant", "Constant"],
    ),
    (
        "variable.stage-name, entity.name.other.anchor.yaml, variable.other.alias.yaml",
        ["@label", "Label", "@type", "Type"],
    ),
    ("entity.name.enum.tag-digest", ["@string.special", "Special", "@number", "Number"]),
    ("variable.parameter.option", ["@variable.parameter", "@parameter", "@attribute", "Special"]),
    ("entity.name.pattern.git.ignore", ["@string.regexp", "@string.special", "@string", "String"]),
]


def resolve(hl, name):
    """The group's own attributes, following `link` chains. Missing or empty: {}."""
    seen = set()
    group = hl.get(name)
    while isinstance(group, dict) and "link" in group and name not in seen:
        seen.add(name)
        name = group["link"]
        group = hl.get(name)
    # vim.json.encode writes an empty table as [].
    return group if isinstance(group, dict) else {}


def colours(hl, name):
    """(fg, bg) of a group as it is drawn: `reverse` swaps them, falling back to Normal."""
    group = resolve(hl, name)
    fg, bg = group.get("fg"), group.get("bg")
    if group.get("reverse"):
        normal = resolve(hl, "Normal")
        fg, bg = bg or normal.get("bg"), fg or normal.get("fg")
    return {"fg": fg, "bg": bg}


def hex_colour(value):
    return "#%06X" % value


def convert(hl, name):
    top = {}
    for key, group, attr in GLOBALS:
        value = colours(hl, group)[attr]
        if value is not None:
            top[key] = hex_colour(value)

    normal_fg = colours(hl, "Normal")["fg"]
    rules = [(s, g, False) for s, g in SCOPES] + [(s, g, True) for s, g in INFRA_SCOPES]
    settings = [{"settings": top}]
    for scope, groups, must_differ in rules:
        group = next(
            (
                g
                for g in groups
                if colours(hl, g)["fg"] not in (None, normal_fg if must_differ else None)
            ),
            None,
        )
        if group is None:
            continue
        attrs = resolve(hl, group)
        out = {"foreground": hex_colour(colours(hl, group)["fg"])}
        style = " ".join(s for s in ("bold", "italic", "underline") if attrs.get(s))
        if style:
            out["fontStyle"] = style
        settings.append({"name": group, "scope": scope, "settings": out})

    return {"name": name, "settings": settings}


def main():
    if len(sys.argv) != 4:
        sys.exit(__doc__)
    src, dst, name = sys.argv[1:]
    with open(src, encoding="utf-8") as f:
        hl = json.load(f)
    if "bg" not in resolve(hl, "Normal"):
        sys.exit(f"{src}: Normal has no background; did the colorscheme load?")
    with open(dst, "wb") as f:
        plistlib.dump(convert(hl, name), f, fmt=plistlib.FMT_XML)


if __name__ == "__main__":
    main()
