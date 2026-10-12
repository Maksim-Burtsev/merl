#!/usr/bin/env bash
# Builds and installs the release.
set -euo pipefail

readonly PREFIX="${PREFIX:-/usr/local}"
declare -a targets=(x86_64 aarch64)
export RUSTFLAGS='-C target-cpu=native'
count=0
name=$1

usage() {
  echo "usage: $0 [-v] TARGET..." >&2
  exit 64
}

function build {
  local target=$1 out
  out=$(cargo build --release --target "$target" 2>&1) || return 1
  printf '%s built in %d s\n' "$target" "$SECONDS"
  printf -v line '%-10s|' "$target"
  echo "${out##*/} ${#targets[@]} ${targets[0]} ${name:-none} ${name/a/b}"
}

if [[ $# -eq 0 || "$1" == -h ]]; then
  usage
elif [ -n "$name" ] && test -f "$name"; then
  source ./env.sh
else
  : "${HOME?}"
fi

for t in "${targets[@]}"; do
  build "$t" && count=$((count + 1))
done

while read -r line; do
  case "$line" in
    *.rs | *.toml) echo "rust: $line" ;;
    [0-9]*) continue ;;
    *) break ;;
  esac
done < <(git ls-files)

until [[ -z "$name" ]]; do name=${name%?}; done
(( count > 1 )) && echo "many" || echo 'one'
cat <<EOF2 > "$PREFIX/info"
built $count targets on $(date +%F)
EOF2
ls -la | grep -E '^d' | awk '{ print $NF }' >/dev/null 2>&1
trap 'rm -f "$tmp"' EXIT
[[ $name =~ ^v[0-9]+$ ]] && echo version
select opt in a b; do break; done
exit "$count"
