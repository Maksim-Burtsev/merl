#!/bin/sh
# Prepares what the tapes record: a checkout of gitea at a pinned commit in /tmp/merl-demo/gitea,
# on a branch that plays the agent's work (agent-branch.patch), one of polar's backend in
# /tmp/merl-demo/polar on the branch assets/review.steps walks (license-key-lookup.patch), and an
# empty HOME so that merl runs with its defaults. Run once, then:
# assets/tapes/record.py assets/demo.steps, ...
# tests/smoke/run.py passes a DIR of its own, so a smoke run and a recording never share a checkout.
set -eu
here=$(cd "$(dirname "$0")" && pwd)
dir=${1:-/tmp/merl-demo}
commit=3296046b4ab1f8cf3efcc908df2b1918b16831b7
polar=ce071cf3a82cc684befc8b447d322b31d86dfd11

mkdir -p "$dir/home"
dir=$(cd "$dir" && pwd)  # absolute: the script moves between the checkouts
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

# The README's review: polar's backend, a FastAPI app, opened on its own as its developers do
# (server/ is the repository), and an agent's branch of three files.
if [ ! -d "$dir/polar/.git" ]; then
    src=$(mktemp -d)
    git init -q "$src"
    git -C "$src" fetch -q --depth 1 https://github.com/polarsource/polar.git "$polar"
    mkdir -p "$dir/polar"
    git -C "$src" archive FETCH_HEAD server | tar -x --strip-components 1 -C "$dir/polar"
    rm -rf "$src"
    git init -q -b main "$dir/polar"
    git -C "$dir/polar" add -A
    git -C "$dir/polar" -c user.name=polar -c user.email=polar@example.com commit -qm "polar at $polar"
    git -C "$dir/polar" update-ref refs/remotes/origin/main main
    git -C "$dir/polar" symbolic-ref refs/remotes/origin/HEAD refs/remotes/origin/main
fi
cd "$dir/polar"
git checkout -q -f main
git clean -fdq
git branch -q -D agent/lookup 2>/dev/null || true
git checkout -q -b agent/lookup
git apply "$here/license-key-lookup.patch"
git -c user.name=agent -c user.email=agent@example.com commit -qam "license keys: look a key up by the key a customer holds; validate, activate and deactivate share the lookup"
echo "ready: $dir/polar on $(git branch --show-current)"
