#!/bin/bash
set -euo pipefail
D=${1:-${D_BENCH_SERVERS:-${D_BENCH_CACHE:-$HOME/.cache/merl-d-bench}/servers}}
mkdir -p "$D/bin"
cd "$D"
npm install --no-save --prefix "$D" pyright typescript-language-server typescript@6 intelephense \
  vscode-langservers-extracted @vue/language-server@2 svelte-language-server @astrojs/language-server \
  @nomicfoundation/solidity-language-server bash-language-server@5.8.1
GOBIN="$D/bin" go install golang.org/x/tools/gopls@latest
rustup component add rust-analyzer
ln -sf "$(rustup which rust-analyzer)" "$D/bin-ra"
arch=$(uname -m | sed 's/x86_64/amd64/')
curl -fsSL "https://github.com/withered-magic/starpls/releases/download/v0.1.22/starpls-$(uname -s | tr A-Z a-z)-$arch" -o "$D/bin/starpls"
chmod +x "$D/bin/starpls"
mkdir -p "$D/lua-language-server"
curl -fsSL "https://github.com/LuaLS/lua-language-server/releases/download/3.19.2/lua-language-server-3.19.2-$(uname -s | tr A-Z a-z)-$(uname -m | sed 's/x86_64/x64/; s/aarch64/arm64/').tar.gz" |
  tar -xz -C "$D/lua-language-server"
julia --project=@ls -e 'using Pkg; Pkg.add("LanguageServer")'
depot=$(julia -e 'print(first(DEPOT_PATH))')
test -d "$depot"
mkdir -p "$D/julia-home"
ln -sfn "$depot" "$D/julia-home/.julia"
