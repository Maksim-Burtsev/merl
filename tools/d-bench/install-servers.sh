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
curl -fsSL "https://storage.googleapis.com/dart-archive/channels/stable/release/3.13.5/sdk/dartsdk-$(uname -s | tr A-Z a-z | sed 's/darwin/macos/')-$(uname -m | sed 's/x86_64/x64/; s/aarch64/arm64/')-release.zip" -o "$D/dart-sdk.zip"
rm -rf "$D/dart-sdk" && unzip -q "$D/dart-sdk.zip" -d "$D" && rm "$D/dart-sdk.zip"
mkdir -p "$D/php"
curl -fsSL "https://dl.static-php.dev/static-php-cli/common/php-8.4.23-cli-$(uname -s | tr A-Z a-z | sed 's/darwin/macos/')-$(uname -m | sed 's/arm64/aarch64/').tar.gz" | tar -xz -C "$D/php"
curl -fsSL https://getcomposer.org/download/2.10.3/composer.phar -o "$D/php/composer.phar"
echo "7a2d379d5b8ffdaa028580ef26494c36d2feef4b178d3dd1473a4dbc5e17c8d6  $D/php/composer.phar" | shasum -a 256 -c -
julia --project=@ls -e 'using Pkg; Pkg.add("LanguageServer")'
depot=$(julia -e 'print(first(DEPOT_PATH))')
test -d "$depot"
mkdir -p "$D/julia-home"
ln -sfn "$depot" "$D/julia-home/.julia"
