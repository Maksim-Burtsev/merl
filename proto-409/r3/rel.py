#!/usr/bin/env python3
"""For every branch: type the part a person would type, press Tab once, Enter, and check merl got the branch.

    SOCK=x rel.py KIND > KIND.tsv
"""
import re, sys
import tab

names = [l for l in open("/tmp/409r3/fetched.txt").read().splitlines() if l not in ("HEAD",)]


def natural(b):
    m = re.match(r"dev-\w+/(\d+)-", b)
    if m:
        return m.group(1)
    m = re.match(r"claude/task-(\w+)$", b)
    if m:
        return m.group(1)
    return safe(b.rsplit("/", 1)[-1])


def safe(q):
    out = re.split(r"[^\w./@+,=%-]", q, flags=re.UNICODE)
    return max(out, key=len)


def safe_prefix(b):
    cut = b[: max(3, len(b) - 3)]
    m = re.search(r"[^\w./@+,=%-]", cut, flags=re.UNICODE)
    return cut[: m.start()] if m else cut


queries = []
for b in names:
    queries.append(("substring", b, natural(b)))
    queries.append(("prefix", b, safe_prefix(b)))
kind = sys.argv[1]
tab.start(kind)
print("kind\tquery_kind\tbranch\ttyped\tcandidates\tresult\treceived")
for qk, b, q in queries:
    hits = [n for n in names if n.startswith(q)] or [n for n in names if q in n]
    after, got = tab.query(f"merl -r {q}", execute=True)
    want = f"merl got: [-r] [{b}] END"
    if len(hits) == 1:
        res = "ok" if got == want else "WRONG"
    else:
        res = "ambiguous-ok" if got and got.endswith("END") and q in got else "ambiguous-?"
    print(f"{kind}\t{qk}\t{b}\t{q}\t{len(hits)}\t{res}\t{got}", flush=True)
tab.t("kill-server", check=False)
