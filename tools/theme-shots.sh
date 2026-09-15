#!/usr/bin/env bash
# A screenshot of merl in every shipped theme, or only the named ones, for the gallery in
# docs/themes.md. Needs vhs, Python with Pillow and a release build:
#
#   cargo build --release && tools/theme-shots.sh [NAME...]
#
# Writes assets/themes/<name>.png (or $OUT/<name>.png). vhs 0.12.0 exits 0 without writing any image
# (charmbracelet/vhs#787); use 0.11.0 until a release fixes it.
set -euo pipefail

root=$(cd "$(dirname "$0")/.." && pwd)
out=${OUT:-$root/assets/themes}
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
mkdir -p "$out"
# A copy outside the repository, so the tree shows the sample project, not merl's own files.
cp -R "$root/tutor/notes" "$tmp/notes"

# The names given, or all of them in picker order, straight from the THEMES table.
for name in ${*:-$(sed -n 's/^    ("\([a-z0-9-]*\)", include_bytes!.*/\1/p' "$root/src/theme.rs")}; do
  cat >"$tmp/shot.tape" <<EOF
Output "$tmp/shot.gif"
Set Shell bash
Set FontSize 15
Set Width 1200
Set Height 640
Set Padding 0
Hide
Type "'$root/target/release/merl' --theme $name '$tmp/notes/store.py:9'"
Enter
Sleep 1.5s
Show
Sleep 300ms
Screenshot "$out/$name.png"
Sleep 100ms
EOF
  vhs "$tmp/shot.tape" >/dev/null
  # 256 colours keep a terminal screenshot sharp at about a third of the size.
  python3 -c 'import sys; from PIL import Image
p = sys.argv[1]; Image.open(p).convert("RGB").quantize(256).save(p, optimize=True)' "$out/$name.png"
  echo "$out/$name.png"
done
