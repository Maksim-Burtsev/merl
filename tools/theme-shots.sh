#!/usr/bin/env bash
set -uo pipefail

root=$(cd "$(dirname "$0")/.." && pwd)
out=${OUT:-$root/assets/themes}
bin=${BIN:-$root/target/release/merl}
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
mkdir -p "$out"
cp -R "$root/tutor/notes" "$tmp/notes"
tm() { tmux -L "merl-shots-$$" -f /dev/null "$@"; }

failed=0
for name in ${*:-$(sed -n 's/^    ("\([a-z0-9-]*\)", include_bytes!.*/\1/p' "$root/src/theme.rs")}; do
  tm kill-server 2>/dev/null
  tm new-session -d -s shot -x 150 -y 38 \
    "HOME='${MERL_HOME:-$HOME}' '$bin' --theme '$name' '$tmp/notes/store.py:9'"
  ok=
  for _ in $(seq 50); do
    if tm capture-pane -p -t shot 2>/dev/null | grep -q "store.py"; then
      ok=1
      break
    fi
    sleep 0.2
  done
  if [[ -z $ok ]]; then
    echo "FAILED $name: $(tm capture-pane -p -t shot 2>/dev/null | grep -m1 . || echo 'no output')" >&2
    failed=1
    continue
  fi
  sleep 0.5
  if [[ -n ${KEYS:-} ]]; then
    tm send-keys -t shot "$KEYS"
    sleep 0.6
  fi
  tm capture-pane -p -e -t shot >"$tmp/shot.ans"
  python3 "$root/tools/shot.py" "$tmp/shot.ans" "$out/$name.png" || {
    echo "FAILED $name: render" >&2
    failed=1
  }
done
tm kill-server 2>/dev/null
exit $failed
