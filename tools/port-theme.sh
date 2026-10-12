#!/usr/bin/env bash
set -euo pipefail

root=$(cd "$(dirname "$0")/.." && pwd)
if [[ ${1:-} == --all ]]; then
  while IFS=$'\t' read -r name source colorscheme before deps; do
    [[ $name == '#'* || $source == skip ]] && continue
    deps=${deps//,/ }
    [[ $deps == - ]] && deps=
    deps=$(for d in $deps; do printf 'https://github.com/%s ' "$d"; done)
    [[ $before == - ]] && set -- || set -- "$before"
    DEPS=$deps "$0" "https://github.com/$source" "$colorscheme" "$name" "$@" >/dev/null </dev/null
    echo "$name"
  done <"$root/tools/theme-bench/themes.tsv"
  exit
fi
if [[ $# -lt 3 ]]; then
  sed -n '2,11p' "$0"
  exit 2
fi
url=$1 colorscheme=$2 name=$3
shift 3
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

pinned='^(.*)@([0-9a-f]{40})$'
fetch() {
  if [[ $1 =~ $pinned ]]; then
    git init -q "$2"
    git -C "$2" fetch -q --depth 1 "${BASH_REMATCH[1]}" "${BASH_REMATCH[2]}"
    git -C "$2" checkout -q FETCH_HEAD
  else
    git clone -q --depth 1 "$1" "$2"
  fi
}

fetch "$url" "$tmp/theme"
[[ $url =~ $pinned ]] && url=${BASH_REMATCH[1]}
args=(-c "set rtp+=$tmp/theme")
for dep in ${DEPS:-}; do
  dir="$tmp/dep-$(basename "$dep" | cut -d@ -f1)"
  fetch "$dep" "$dir"
  args+=(-c "set rtp+=$dir")
done
args+=(-c "set termguicolors")
for cmd in "$@"; do
  args+=(-c "$cmd")
done

XDG_CACHE_HOME=$tmp/xdg XDG_STATE_HOME=$tmp/xdg XDG_DATA_HOME=$tmp/xdg \
  nvim --headless -u NONE "${args[@]}" \
  -c "lua local ok, e = pcall(vim.cmd.colorscheme, '$colorscheme'); if not ok then io.stderr:write(e .. '\n'); vim.cmd('cquit 1') end" \
  -c "lua local o = {} for g in pairs(vim.api.nvim_get_hl(0, {})) do o[g] = vim.api.nvim_get_hl(0, {name = g, link = false}) end vim.fn.writefile({vim.json.encode(o)}, '$tmp/hl.json')" \
  -c qa

python3 "$root/tools/nvim2tmtheme.py" "$tmp/hl.json" "$root/themes/$name.tmTheme" "$name"

repo=$(basename "${url%.git}")
case $repo in
  neovim | nvim) repo=$(basename "$(dirname "$url")") ;;
esac
repo=${repo%.nvim}
repo=${repo%-nvim}
repo=${repo%-neovim}
licence=$(find "$tmp/theme" -maxdepth 1 -iname 'licen[cs]e*' | head -1)
if [[ -z $licence ]]; then
  echo "$url has no LICENSE file" >&2
  exit 1
fi
[[ -e "$root/themes/LICENSE-$repo" ]] || cp "$licence" "$root/themes/LICENSE-$repo"

echo "themes/$name.tmTheme, themes/$name.groups.json; licence themes/LICENSE-$repo: $(grep -m1 . "$licence")"
echo "Add to THEMES in src/theme.rs:"
printf '    ("%s", include_bytes!("../themes/%s.tmTheme"), Some(include_str!("../themes/%s.groups.json"))),\n' \
  "$name" "$name" "$name"
echo "Add to docs/themes.md under Dark or Light (and a new licence under Licences), then run"
echo "tools/theme-shots.sh $name:"
src=${url%.git}
printf '| `%s`<br>[%s](%s) | <img src="../assets/themes/%s.png" alt="merl in %s" width="480"> |\n' \
  "$name" "${src#https://github.com/}" "$src" "$name" "$name"
