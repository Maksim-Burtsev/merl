#!/bin/sh
# Prepares what the tapes record: a checkout of gitea at a pinned commit in /tmp/merl-demo/gitea,
# on a branch that plays the agent's work (agent-branch.patch), one of mealie in
# /tmp/merl-demo/mealie on the branch assets/review.steps walks (webhook-retry.patch), and an empty
# HOME so that merl runs with its defaults. Run once, then: assets/tapes/record.py assets/demo.steps, ...
# tests/smoke/run.py passes a DIR of its own, so a smoke run and a recording never share a checkout.
set -eu
here=$(cd "$(dirname "$0")" && pwd)
dir=${1:-/tmp/merl-demo}
commit=3296046b4ab1f8cf3efcc908df2b1918b16831b7
mealie=2b3a16225314c734f9d4092e4a03ed4b6708cf7a

mkdir -p "$dir/home"
if [ ! -d "$dir/gitea/.git" ]; then
    git init -q -b main "$dir/gitea"
    git -C "$dir/gitea" remote add origin https://github.com/go-gitea/gitea.git
    git -C "$dir/gitea" fetch -q --depth 1 origin "$commit"
    git -C "$dir/gitea" reset -q --hard FETCH_HEAD
    git -C "$dir/gitea" update-ref refs/remotes/origin/main "$commit"
    git -C "$dir/gitea" symbolic-ref refs/remotes/origin/HEAD refs/remotes/origin/main
fi
cd "$dir/gitea"
git checkout -q -f main
git clean -fdq
git branch -q -D agent/unstar-paging 2>/dev/null || true
git checkout -q -b agent/unstar-paging
git apply "$here/agent-branch.patch"
git -c user.name=agent -c user.email=agent@example.com commit -qam "user: unstar and unwatch by ID when blocking, paging skipped rows"
echo "ready: $dir/gitea on $(git branch --show-current)"

# The README's review: a Python app, an agent's branch of four files.
if [ ! -d "$dir/mealie/.git" ]; then
    git init -q -b main "$dir/mealie"
    git -C "$dir/mealie" remote add origin https://github.com/mealie-recipes/mealie.git
    git -C "$dir/mealie" fetch -q --depth 1 origin "$mealie"
    git -C "$dir/mealie" reset -q --hard FETCH_HEAD
    git -C "$dir/mealie" update-ref refs/remotes/origin/main "$mealie"
    git -C "$dir/mealie" symbolic-ref refs/remotes/origin/HEAD refs/remotes/origin/main
fi
cd "$dir/mealie"
git checkout -q -f main
git clean -fdq
git branch -q -D agent/webhook-retry 2>/dev/null || true
git checkout -q -b agent/webhook-retry
git apply "$here/webhook-retry.patch"
git add -A
git -c user.name=agent -c user.email=agent@example.com commit -qm "webhooks: retry a failed delivery with backoff, honouring Retry-After"
echo "ready: $dir/mealie on $(git branch --show-current)"
