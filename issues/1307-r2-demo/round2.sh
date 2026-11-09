#!/usr/bin/env bash
# The agent's second round on /tmp/1307-httpx: fixes the notes, commits, and leaves notes file $1.
set -euo pipefail
D=$(cd "$(dirname "$0")" && pwd)
cd /tmp/1307-httpx
python3 - <<'PY'
p='httpx/_utils.py'; s=open(p).read()
s=s.replace('''def unquote(value: str) -> str:
    if len(value) >= 2 and value[0] == value[-1] and value[0] in "\\"'":
''','''def unquote_header(value: str) -> str:
    if len(value) >= 2 and value[0] == value[-1] and value[0] in ('"', "'"):
''')
open(p,'w').write(s)
p='httpx/_auth.py'; s=open(p).read()
s=s.replace("from ._quoting import strip_quotes\n","")
s=s.replace("from ._utils import to_bytes, to_str\n","from ._utils import to_bytes, to_str, unquote_header\n")
s=s.replace("header_dict[key] = strip_quotes(value)","header_dict[key] = unquote_header(value)")
open(p,'w').write(s)
p='tests/test_utils.py'; s=open(p).read()
s=s.replace("get_environment_proxies, unquote","get_environment_proxies, unquote_header")
s=s.replace('''        ("abc", "abc"),
    ],''','''        ("abc", "abc"),
        ('"', '"'),
    ],''')
s=s.replace("def test_unquote(value, expected):\n    assert unquote(value) == expected","def test_unquote_header(value, expected):\n    assert unquote_header(value) == expected")
open(p,'w').write(s)
PY
git rm -q httpx/_quoting.py
git add -A && git commit -q -m "Review fixes: one unquote_header, tuple, lone-quote test"
if [ "$1" = none ]; then rm -f .git/merl/notes/agent/unquote; else cp "$D/$1" .git/merl/notes/agent/unquote; fi
echo "round 2 at $(git rev-parse --short HEAD) with $1"
