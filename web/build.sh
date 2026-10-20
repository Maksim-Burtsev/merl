#!/usr/bin/env bash
set -euo pipefail
[ $# -eq 1 ] || { echo "usage: web/build.sh OUT.wasm  (WASI_SDK_PATH: an unpacked wasi-sdk 34, else it is downloaded)" >&2; exit 2; }
out=$(cd "$(dirname "$1")" && pwd)/$(basename "$1")
cd "$(dirname "$0")"

version=34
case $(uname -sm) in
  "Darwin arm64") asset=arm64-macos sum=9c59398106b417f8f14913380fdf0097a8cc0ff4af9eb3ce0065a859e88d49e9 ;;
  "Darwin x86_64") asset=x86_64-macos sum=87d27fa8adc68dee59bfbf2e22a6d34ef717c34d6bf1d8af2a56fc929d9ce0eb ;;
  "Linux x86_64") asset=x86_64-linux sum=b761e3a0721dbae9c09a0059e5fdb2bf917d1b4a8a7b430fb3b5aafb0984b2c4 ;;
  "Linux aarch64") asset=arm64-linux sum=f7e243dff54d60bcc576e94d6166b69f410f2500ae4a9ceef34315be10e77971 ;;
  *) echo "web/build.sh: no wasi-sdk $version for $(uname -sm)" >&2; exit 1 ;;
esac
target=${CARGO_TARGET_DIR:-target}
sdk=${WASI_SDK_PATH:-$target/wasi-sdk-$version}
if [ ! -x "$sdk/bin/clang" ]; then
  mkdir -p "$sdk"
  tarball=$sdk.tar.gz
  curl -fsSL -o "$tarball" "https://github.com/WebAssembly/wasi-sdk/releases/download/wasi-sdk-$version/wasi-sdk-$version.0-$asset.tar.gz"
  echo "$sum  $tarball" | shasum -a 256 -c - >/dev/null
  tar xzf "$tarball" -C "$sdk" --strip-components=1
  rm "$tarball"
fi
sdk=$(cd "$sdk" && pwd)

if command -v rustup >/dev/null; then
  PATH=$(dirname "$(rustup which cargo)"):$PATH
fi
export CC_wasm32_wasip1=$sdk/bin/clang AR_wasm32_wasip1=$sdk/bin/llvm-ar
cp ../Cargo.lock Cargo.lock
cargo build --release --target wasm32-wasip1
cp "$target/wasm32-wasip1/release/merl_web.wasm" "$out"
