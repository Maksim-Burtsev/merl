#!/usr/bin/env bash
# reset.sh NOTES: round 1 reviewed (every file ticked, six notes), then the agent's round 2
# leaving NOTES (notes-round2-A.txt, notes-round2-B.txt, or none: the file deleted, as today).
set -euo pipefail
D=$(cd "$(dirname "$0")" && pwd)
"$D/setup.sh" >/dev/null
cd /tmp/1307-httpx
mkdir -p .git/merl/notes/agent
cp "$D/notes-round1.txt" .git/merl/notes/agent/unquote
python3 - <<'PY'
import subprocess, datetime
files = subprocess.run(["git", "diff", "--name-only", "origin/master"], capture_output=True, text=True).stdout.split()
def fnv(b):
    h = 0xcbf29ce484222325
    for x in b:
        h = ((h ^ x) * 0x100000001b3) & 0xFFFFFFFFFFFFFFFF
    return "%016x" % h
day = datetime.datetime.now(datetime.timezone.utc).strftime("%Y-%m-%d")
with open(".git/merl/viewed", "w") as out:
    for f in files:
        out.write(f"{day}\tagent/unquote\torigin/master\t{fnv(open(f, 'rb').read())}\t{f}\n")
PY
[ "$1" = round1 ] && exit 0
"$D/round2.sh" "$1" >/dev/null
