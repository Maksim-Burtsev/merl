#!/usr/bin/env python3
"""Release step 1 (AGENTS.md `## Releases`): gather `changes/N.md` under CHANGELOG.md's
`## [Unreleased]`, one heading each in Keep a Changelog's order, entries by issue number, and
delete the fragments.

    tools/gather-changes.py          # writes CHANGELOG.md, deletes changes/*.md
    tools/gather-changes.py --print  # prints the section, touches nothing
"""
import os
import re
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
CHANGES = os.path.join(ROOT, "changes")
ORDER = ["Added", "Changed", "Deprecated", "Removed", "Fixed", "Security"]


def fragments():
    names = os.listdir(CHANGES) if os.path.isdir(CHANGES) else []
    return [os.path.join(CHANGES, n) for n in sorted(names, key=lambda n: int(n.split(".")[0]))]


def section(paths):
    entries = {h: [] for h in ORDER}
    for path in paths:
        for block in re.split(r"^### ", open(path).read(), flags=re.M)[1:]:
            heading, _, body = block.partition("\n")
            entries[heading.strip()].append(body.strip())
    return "\n\n".join(f"### {h}\n\n" + "\n".join(e) for h, e in entries.items() if e)


def main():
    paths = fragments()
    if not paths:
        sys.exit("no changes/N.md to gather")
    gathered = section(paths)
    if "--print" in sys.argv:
        print(gathered)
        return
    log = os.path.join(ROOT, "CHANGELOG.md")
    head, rest = open(log).read().split("## [Unreleased]\n", 1)
    rest = rest.lstrip("\n")
    if not rest.startswith("## ["):
        sys.exit("## [Unreleased] is not empty: move its entries into changes/N.md first")
    open(log, "w").write(f"{head}## [Unreleased]\n\n{gathered}\n\n{rest}")
    for path in paths:
        os.remove(path)


main()
