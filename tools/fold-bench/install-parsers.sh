#!/bin/bash
# tools/fold-bench/install-parsers.sh DIR: nvim-treesitter at a pinned commit, its queries, the
# tree-sitter CLI and every parser record.py reads, built into DIR. Nothing outside DIR.
set -euo pipefail
DIR=$(mkdir -p "$1" && cd "$1" && pwd)
NVIM_TS=910fdf6f49e9dee7e7257c7d11a76b040fdfb9de
NVIM=${NVIM:-nvim}
[ -d "$DIR/nvim-treesitter/.git" ] || git init -q "$DIR/nvim-treesitter"
git -C "$DIR/nvim-treesitter" fetch -q --depth 1 https://github.com/nvim-treesitter/nvim-treesitter.git $NVIM_TS
git -C "$DIR/nvim-treesitter" checkout -q FETCH_HEAD
[ -x "$DIR/ts-cli/node_modules/.bin/tree-sitter" ] || npm --prefix "$DIR/ts-cli" install -s tree-sitter-cli
mkdir -p "$DIR/grammars" "$DIR/rt/parser"
cat > "$DIR/parsers.lua" <<'LUA'
local p = dofile(arg[1] .. '/lua/nvim-treesitter/parsers.lua')
for _, l in ipairs(vim.split(arg[2], ',')) do
  local i = p[l].install_info
  print(l, i.url, i.revision, i.location or '-', tostring(i.generate or false))
end
LUA
LANGS=$(python3 "$(dirname "$0")/record.py" parsers)
"$NVIM" --clean -l "$DIR/parsers.lua" "$DIR/nvim-treesitter" "$LANGS" 2>&1 | while read -r lang url rev loc gen; do
  repo=$DIR/grammars/$(basename "$url")
  [ -d "$repo/.git" ] || git init -q "$repo"
  if [ "$(git -C "$repo" rev-parse HEAD 2>/dev/null)" != "$rev" ]; then
    git -C "$repo" fetch -q --depth 1 "$url" "$rev" && git -C "$repo" checkout -q FETCH_HEAD
  fi
  src=$repo; [ "$loc" != - ] && src=$repo/$loc
  [ "$gen" = true ] && [ ! -f "$src/src/parser.c" ] && (cd "$src" && "$DIR/ts-cli/node_modules/.bin/tree-sitter" generate)
  files=("$src/src/parser.c"); [ -f "$src/src/scanner.c" ] && files+=("$src/src/scanner.c")
  cc -O2 -shared -fPIC -std=c11 -I"$src/src" "${files[@]}" -o "$DIR/rt/parser/$lang.so"
  echo "$lang"
done
