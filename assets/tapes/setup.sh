#!/bin/sh
# Prepares what the tapes record: a checkout of gitea at a pinned commit in /tmp/merl-demo/gitea,
# on a branch that plays the agent's work (agent-branch.patch), and an empty HOME so that merl
# runs with its defaults. Run once, then: assets/tapes/record.py assets/demo.steps, ...
set -eu
here=$(cd "$(dirname "$0")" && pwd)
dir=/tmp/merl-demo
commit=3296046b4ab1f8cf3efcc908df2b1918b16831b7

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
