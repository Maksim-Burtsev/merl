#!/usr/bin/env bash
# A screenshot of merl in every shipped theme, or only the named ones, for the gallery in
# docs/themes.md. tmux paints merl for real, `capture-pane -e` keeps the colours and tools/shot.py
# draws the cells, so it needs only tmux, Python with Pillow and a release build:
#
#   cargo build --release && tools/theme-shots.sh [NAME...]
#
# Writes assets/themes/<name>.png (or $OUT/<name>.png). MERL_HOME=<dir> runs merl with that HOME,
# which is how a theme in <dir>/.config/merl/themes is reached; KEYS=T opens an overlay before the
# capture. Not vhs: 0.12.0 exits 0 without writing an image (charmbracelet/vhs#787).
set -uo pipefail

root=$(cd "$(dirname "$0")/.." && pwd)
out=${OUT:-$root/assets/themes}
bin=${BIN:-$root/target/release/merl}
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
mkdir -p "$out"
# A copy outside the repository, so the tree shows the sample project, not merl's own files.
cp -R "$root/tutor/notes" "$tmp/notes"
# A private server: the user's tmux.conf and sessions stay out of the picture.
tm() { tmux -L "merl-shots-$$" -f /dev/null "$@"; }

failed=0
# The names given, or all of them in picker order, straight from the THEMES table.
for name in ${*:-$(sed -n 's/^    ("\([a-z0-9-]*\)", include_bytes!.*/\1/p' "$root/src/theme.rs")}; do
  tm kill-server 2>/dev/null
  tm new-session -d -s shot -x 150 -y 38 \
    "HOME='${MERL_HOME:-$HOME}' '$bin' --theme '$name' '$tmp/notes/store.py:9'"
  # Wait for the first paint: the status bar carries the file name.
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
  # Let the highlighter finish the visible screen before the capture.
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
