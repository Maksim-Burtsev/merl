#!/usr/bin/env bash
# Ports a Neovim colorscheme, from its own repo, into themes/<name>.tmTheme.
#
#   tools/port-theme.sh <git-url> <colorscheme> <name> [ex command...]
#
# Ex commands run before `:colorscheme`, for themes that take their variant from options:
#   tools/port-theme.sh https://github.com/sainnhe/everforest everforest everforest-light \
#     "set background=light" "let g:everforest_background='medium'"
# DEPS="<git-url> ..." clones plugins the theme needs onto the runtime path (bluloco: lush.nvim).
set -euo pipefail

if [[ $# -lt 3 ]]; then
  sed -n '2,9p' "$0"
  exit 2
fi
url=$1 colorscheme=$2 name=$3
shift 3
root=$(cd "$(dirname "$0")/.." && pwd)
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

git clone -q --depth 1 "$url" "$tmp/theme"
args=(-c "set rtp+=$tmp/theme")
for dep in ${DEPS:-}; do
  dir="$tmp/dep-$(basename "$dep")"
  git clone -q --depth 1 "$dep" "$dir"
  args+=(-c "set rtp+=$dir")
done
args+=(-c "set termguicolors")
for cmd in "$@"; do
  args+=(-c "$cmd")
done

# Themes that compile themselves (nightfox, kanagawa) write caches: keep them out of $HOME.
XDG_CACHE_HOME=$tmp/xdg XDG_STATE_HOME=$tmp/xdg XDG_DATA_HOME=$tmp/xdg \
  nvim --headless -u NONE "${args[@]}" \
  -c "lua local ok, e = pcall(vim.cmd.colorscheme, '$colorscheme'); if not ok then io.stderr:write(e .. '\n'); vim.cmd('cquit 1') end" \
  -c "lua vim.fn.writefile({vim.json.encode(vim.api.nvim_get_hl(0, {}))}, '$tmp/hl.json')" \
  -c qa

python3 "$root/tools/nvim2tmtheme.py" "$tmp/hl.json" "$root/themes/$name.tmTheme" "$name"

# One licence per source repo: rose-pine/neovim -> rose-pine, rebelot/kanagawa.nvim -> kanagawa.
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

echo "themes/$name.tmTheme; licence themes/LICENSE-$repo: $(grep -m1 . "$licence")"
echo "Add to THEMES in src/theme.rs:"
printf '    ("%s", include_bytes!("../themes/%s.tmTheme")),\n' "$name" "$name"
echo "Add to docs/themes.md under Dark or Light (and a new licence under Licences), then run"
echo "tools/theme-shots.sh $name:"
src=${url%.git}
printf '| `%s`<br>[%s](%s) | <img src="../assets/themes/%s.png" alt="merl in %s" width="480"> |\n' \
  "$name" "${src#https://github.com/}" "$src" "$name" "$name"
