#!/bin/bash
# Installs the language servers record.py asks, into one directory (default <cache>/servers),
# never into merl. Once, on a recording machine; the prototype of 2026-09-28 used these.
# clangd and sourcekit-lsp come from the Xcode command-line tools (`xcrun`), not from here.
set -euo pipefail
D=${1:-${D_BENCH_SERVERS:-${D_BENCH_CACHE:-$HOME/.cache/merl-d-bench}/servers}}
mkdir -p "$D/bin"
cd "$D"
# typescript@7 on npm has no tsserver.js; typescript-language-server needs it.
npm install --no-save --prefix "$D" pyright typescript-language-server typescript@6 intelephense \
  @nomicfoundation/solidity-language-server
GOBIN="$D/bin" go install golang.org/x/tools/gopls@latest
rustup component add rust-analyzer
ln -sf "$(rustup which rust-analyzer)" "$D/bin-ra"
arch=$(uname -m | sed 's/x86_64/amd64/')
curl -fsSL "https://github.com/withered-magic/starpls/releases/download/v0.1.22/starpls-$(uname -s | tr A-Z a-z)-$arch" -o "$D/bin/starpls"
chmod +x "$D/bin/starpls"
