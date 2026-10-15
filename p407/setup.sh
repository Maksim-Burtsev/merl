#!/bin/zsh
set -eu
here=${0:A:h}
sh $here/../assets/tapes/setup.sh ${1:-/tmp/merl-demo} >/dev/null 2>&1 || true
cd ${1:-/tmp/merl-demo}/polar
git checkout -q -f main && git clean -fdq
git branch -q -D agent/lookup 2>/dev/null || true
git checkout -q -b agent/lookup
git apply $here/../assets/tapes/license-key-lookup.patch
G=(git -c user.name=agent -c user.email=agent@example.com)
git add polar/license_key/service.py && $G commit -qm "license keys: one service lookup by the key a customer holds

validate, activate and deactivate each built the same repository query
and raised ResourceNotFound on a miss. get_by_key_or_raise does it once,
limited to the organisations the caller can read, so the new lookup
endpoint and the three old ones cannot drift apart."
git add polar/license_key/endpoints.py && $G commit -qm "license keys: GET /lookup; validate, activate, deactivate share the lookup

Support gets keys quoted in emails and could find one only through the
dashboard. The endpoint is read-only and takes the same auth as GET /{id}.
The three POST endpoints drop their copies of the query for the service
call, which is why the repository import goes."
git add tests && $G commit -qm "license keys: tests for the lookup endpoint

Anonymous gets 401; a key of another organisation gets 404, the same as
an unknown key, so the endpoint never tells a caller that a key exists
elsewhere; a key of the caller's organisation comes back by its id."
git log --oneline main..
