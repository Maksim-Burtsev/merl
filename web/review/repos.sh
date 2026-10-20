#!/usr/bin/env bash
set -euo pipefail
[ $# -eq 1 ] || { echo "usage: web/review/repos.sh OUT_DIR" >&2; exit 2; }
here=$(cd "$(dirname "$0")" && pwd)
mkdir -p "$1"
out=$(cd "$1" && pwd)
commit() {
  GIT_AUTHOR_NAME=$1 GIT_AUTHOR_EMAIL=$2 GIT_COMMITTER_NAME=$1 GIT_COMMITTER_EMAIL=$2 \
    GIT_AUTHOR_DATE=$3 GIT_COMMITTER_DATE=$3 git commit -q --no-verify -m "$4"
}
for lang in python typescript go rust; do
  case $lang in
    python) old=limits.py new=plans.py ;;
    typescript) old=src/limits.ts new=src/plans.ts ;;
    go) old=limits.go new=plans.go ;;
    rust) old=src/limits.rs new=src/plans.rs ;;
  esac
  repo=$out/ratelimit-$lang/ratelimit
  rm -rf "$out/ratelimit-$lang"
  mkdir -p "$repo"
  cp -R "$here/main/$lang/." "$repo/"
  cd "$repo"
  git init -q -b main
  git add -A
  commit "Dana Reyes" dana@example.com 2026-10-12T10:00:00+00:00 "Rate limiter service"
  git checkout -q -b fix/early-429
  git mv "$old" "$new"
  cp -R "$here/branch/$lang/." "$repo/"
  git add -A
  commit agent agent@example.com 2026-10-16T15:30:00+00:00 "Size each bucket's burst from its plan"
  echo "$lang: $(git diff -M --name-status main...fix/early-429 | tr '\t\n' '  ')"
done
