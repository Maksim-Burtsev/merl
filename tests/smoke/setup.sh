#!/bin/bash
# Builds the smoke test's fixture in DIR; tests/smoke/run.py runs it before every run.
#
#   DIR/orders      project/ as a git repository on `main`, everything committed, plus what its
#                   .gitignore leaves out (a .venv without pip, a .env) and the odd files, written
#                   here byte by byte: a 1 MB JSON on one line, a CSV with a BOM and CRLF.
#   DIR/origin.git  its bare `origin`, which also holds `agent/refund`, the branch an agent pushed
#                   from a clone of its own (agent-refund.patch). orders never fetched it, as a
#                   reviewer's clone would not have: `merl --review=agent/refund` fetches it.
#   DIR/agent-wt    a worktree of orders on `agent/page-size`, an agent's at work: one commit and a
#                   file not committed yet. `merl -r agent/page-size` reviews it there (#396).
#   DIR/agent-totp  a worktree on `agent/totp`: two files of a Java package eleven directories
#                   deep, the review panel's one-child chain (#399).
#   DIR/home        the HOME merl runs with: empty but for a .zshrc to open outside a repository.
#
# No network. Fixed names and dates, so the commits and their hashes are the same on every run,
# and no global git config of the user's gets in.
set -eu
here=$(cd "$(dirname "$0")" && pwd)
dir=${1:?usage: setup.sh DIR}
rm -rf "$dir"
mkdir -p "$dir/home"
printf 'export EDITOR=merl\nalias gs="git status"\n' > "$dir/home/.zshrc"
cp -R "$here/project" "$dir/orders"
cd "$dir/orders"
export GIT_CONFIG_GLOBAL=/dev/null GIT_CONFIG_NOSYSTEM=1 \
    GIT_AUTHOR_NAME=dev GIT_AUTHOR_EMAIL=dev@example.com GIT_AUTHOR_DATE=2026-09-01T10:00:00Z \
    GIT_COMMITTER_NAME=dev GIT_COMMITTER_EMAIL=dev@example.com GIT_COMMITTER_DATE=2026-09-01T10:00:00Z

mkdir -p data
python3 - <<'PY'
import json, random
random.seed(7)
orders = [{"id": i, "customer_id": random.randint(1, 3000), "total": "%.2f" % random.uniform(5, 900),
           "status": random.choice(["pending", "paid", "shipped", "cancelled", "refunded"]),
           "created_at": "2026-%02d-%02dT%02d:%02d:00Z" % (random.randint(1, 8), random.randint(1, 28),
                                                          random.randint(0, 23), random.randint(0, 59)),
           "items": [{"sku": "SKU-%05d" % random.randint(0, 99999), "qty": random.randint(1, 3)}
                     for _ in range(random.randint(1, 3))]} for i in range(1, 6200)]
open("data/orders.min.json", "w").write(json.dumps(orders, separators=(",", ":")))
PY
# Excel's CSV export: a UTF-8 BOM and CRLF line ends.
printf '\357\273\277order_id;amount;reason\r\n1042;40.00;damaged in transit\r\n1077;12.50;never arrived\r\n' \
    > data/refunds-2026-08.csv
printf 'API_TOKEN=changeme\nDATABASE_URL=postgresql://orders:orders@localhost/orders\n' > .env
python3 -m venv --without-pip .venv

git init -q -b main
# Two edits three lines apart stay two blocks in merl whatever this says (#220).
git config diff.interHunkContext 3
git add -A
git commit -qm "orders: API, worker, web page, chart"
git init -q --bare -b main ../origin.git
git remote add origin ../origin.git
git push -q -u origin main
git remote set-head origin main

# The agent's branch, pushed from its clone: a refund flow across Python, Go and the chart, a
# rewritten method, an in-memory repository and a script gone, a redrawn diagram.
git switch -q -c agent/refund
git apply --index "$here/agent-refund.patch"
GIT_AUTHOR_NAME=agent GIT_AUTHOR_DATE=2026-09-02T10:00:00Z GIT_COMMITTER_DATE=2026-09-02T10:00:00Z \
    git commit -qm "refunds: refund a paid order inside the window"
git push -q origin agent/refund
git switch -q main
git branch -q -D agent/refund
git update-ref -d refs/remotes/origin/agent/refund
git reflog expire --expire=now --all

# The agent that works in a worktree of its own (#396). Relative paths, so the copy run.py plays
# in points at its own worktree, not at this fixture's.
git worktree add -q --relative-paths -b agent/page-size ../agent-wt
sed -i.bak 's/page_size: int = 50/page_size: int = 100/' ../agent-wt/app/config.py
rm ../agent-wt/app/config.py.bak
GIT_AUTHOR_NAME=agent git -C ../agent-wt commit -qam "config: a page of 100 orders"
printf '# Paging\n\nA page holds 100 orders unless PAGE_SIZE says otherwise.\n' > ../agent-wt/docs/paging.md

d=application/src/main/java/run/halo/app/security/authentication/twofactor/totp
git worktree add -q --relative-paths -b agent/totp ../agent-totp
mkdir -p ../agent-totp/$d
printf 'class TotpAuthenticationFilter {}\n' > ../agent-totp/$d/TotpAuthenticationFilter.java
printf 'class TotpCodeAuthenticationConverter {}\n' > ../agent-totp/$d/TotpCodeAuthenticationConverter.java
git -C ../agent-totp add -A
GIT_AUTHOR_NAME=agent git -C ../agent-totp commit -qm "totp: a filter and a converter"
