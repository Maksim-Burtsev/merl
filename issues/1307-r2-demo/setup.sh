#!/usr/bin/env bash
# Builds /tmp/1307-httpx: httpx@b5addb6 with a branch agent/unquote, an agent's first round.
set -euo pipefail
SRC=/tmp/1307-httpx-src
[ -d "$SRC" ] || git clone -q https://github.com/encode/httpx.git "$SRC"
P=/tmp/1307-httpx
rm -rf "$P" "$P-origin.git"
git clone -q --bare "$SRC" "$P-origin.git"
git -C "$P-origin.git" update-ref refs/heads/master b5addb6
git clone -q "$P-origin.git" "$P"
cd "$P"
git checkout -q -B master origin/master
git config user.name agent && git config user.email agent@example.com
git switch -q -c agent/unquote
python3 - <<'PY'
import re
p='httpx/_utils.py'; s=open(p).read()
s=s.replace('''def unquote(value: str) -> str:
    return value[1:-1] if value[0] == value[-1] == '"' else value
''','''def unquote(value: str) -> str:
    if len(value) >= 2 and value[0] == value[-1] and value[0] in "\\"'":
        return value[1:-1]
    return value
''')
open(p,'w').write(s)
open('httpx/_quoting.py','w').write('''"""Quote helpers for header values."""


def is_quoted(value: str) -> bool:
    return len(value) >= 2 and value[0] == value[-1] and value[0] in "\\"'"


def strip_quotes(value: str) -> str:
    return value[1:-1] if is_quoted(value) else value
''')
p='httpx/_auth.py'; s=open(p).read()
s=s.replace("from ._models import Cookies, Request, Response\n","from ._models import Cookies, Request, Response\nfrom ._quoting import strip_quotes\n")
s=s.replace("from ._utils import to_bytes, to_str, unquote\n","from ._utils import to_bytes, to_str\n")
s=s.replace("header_dict[key] = unquote(value)","header_dict[key] = strip_quotes(value)")
open(p,'w').write(s)
p='tests/test_utils.py'; s=open(p).read()
s=s.replace("from httpx._utils import URLPattern, get_environment_proxies","from httpx._utils import URLPattern, get_environment_proxies, unquote")
s+='''

@pytest.mark.parametrize(
    "value, expected",
    [
        ('"abc"', "abc"),
        ("'abc'", "abc"),
        ("abc", "abc"),
    ],
)
def test_unquote(value, expected):
    assert unquote(value) == expected
'''
open(p,'w').write(s)
p='docs/advanced/authentication.md'; s=open(p).read()
s=s.replace("It requires an additional round-trip in order to negotiate the authentication. \n","It requires an additional round-trip in order to negotiate the authentication. \n\nThe server's header values may be quoted with double or single quotes.\n",1)
open(p,'w').write(s)
p='CHANGELOG.md'; s=open(p).read()
s=s.replace("* Expose `FunctionAuth` from the public API. (#3699)\n","* Expose `FunctionAuth` from the public API. (#3699)\n* Digest auth reads single-quoted header values. (#3801)\n")
open(p,'w').write(s)
PY
git add -A && git commit -q -m "Digest auth: read single-quoted values"
git push -q origin agent/unquote
git remote set-head origin master
echo "round 1 at $(git rev-parse --short HEAD)"
