"""Changed rows a review walked only with `c` never puts on screen.

merl's `c` stands on a hunk's first changed row and centres it, so the view shows rows
[s - H/2, s + H/2), or [0, H) near the top of a file. A row of a changed line that no stop's
view covers is never drawn unless the reader scrolls. Ghost (deleted) rows sit above the added
rows of their hunk; `c` stands on the first added row of a replacement and on the first deleted
row of a pure deletion (#690). The bottom-of-file clamp is ignored, so near a file's end this
counts a few rows unseen that a real view would show.

usage: coverage.py H REPO... (first-parent commits of HEAD, newest 300 each)
"""
import re
import subprocess
import sys

SKIP = re.compile(r"(lock\.json|\.lock|lock\.yaml|go\.sum|\.min\.(js|css)|\.snap|\.svg|\.pb\.go|_pb2\.py)$|/vendor/|^vendor/")
HUNK = re.compile(r"^@@ -(\d+)(?:,(\d+))? \+(\d+)(?:,(\d+))? @@")


def files_of(repo, sha):
    out = subprocess.run(["git", "-C", repo, "diff", "-U0", "--no-color", "--no-ext-diff", "-M",
                          f"{sha}^1", sha], capture_output=True, text=True, errors="replace").stdout
    files, cur = {}, None
    for line in out.splitlines():
        if line.startswith("+++ "):
            cur = line[6:] if line.startswith("+++ b/") else None
            if cur and SKIP.search(cur):
                cur = None
            if cur:
                files[cur] = []
        elif cur and line.startswith("@@"):
            m = HUNK.match(line)
            if m:
                old_n = int(m.group(2)) if m.group(2) is not None else 1
                new_s = int(m.group(3))
                new_n = int(m.group(4)) if m.group(4) is not None else 1
                files[cur].append((new_s, old_n, new_n))
    return files


def unseen(hunks, h):
    rows, stops, shift = [], [], 0
    for new_s, old_n, new_n in hunks:
        first_new = new_s - 1 if new_n else new_s
        top = first_new + shift
        rows += list(range(top, top + old_n + new_n))
        stops.append(top + old_n if new_n else top)
        shift += old_n
    seen = set()
    for s in stops:
        lo = max(0, s - h // 2)
        hi = lo + h if s - h // 2 < 0 else s + h - h // 2
        seen.update(range(lo, hi))
    return len(rows), sum(1 for r in rows if r not in seen), [o + n for _, o, n in hunks]


def main():
    h = int(sys.argv[1])
    tot_rows = tot_unseen = n_hunks = tall = commits = hit = 0
    per_commit = []
    for repo in sys.argv[2:]:
        shas = subprocess.run(["git", "-C", repo, "rev-list", "--first-parent", "--max-count=300",
                               "HEAD"], capture_output=True, text=True).stdout.split()
        for sha in shas:
            fs = files_of(repo, sha)
            if not fs:
                continue
            c_rows = c_unseen = 0
            for hunks in fs.values():
                r, u, sizes = unseen(hunks, h)
                c_rows, c_unseen = c_rows + r, c_unseen + u
                n_hunks += len(sizes)
                tall += sum(1 for s in sizes if s > h // 2)
            commits += 1
            hit += c_unseen > 0
            tot_rows, tot_unseen = tot_rows + c_rows, tot_unseen + c_unseen
            per_commit.append((c_rows, c_unseen))
    big = [(r, u) for r, u in per_commit if r >= 300]
    print(f"H={h}: {commits} changes, {n_hunks} hunks; hunks taller than H/2: {100*tall/n_hunks:.1f}%")
    print(f"  changed rows never on screen in a c-only walk: {tot_unseen}/{tot_rows} = {100*tot_unseen/tot_rows:.1f}%")
    print(f"  changes with any such row: {hit}/{commits} = {100*hit/commits:.1f}%")
    if big:
        share = sorted(u / r for r, u in big)
        print(f"  changes of 300+ changed rows: {len(big)}, median unseen share {100*share[len(share)//2]:.0f}%")


if __name__ == "__main__":
    main()
