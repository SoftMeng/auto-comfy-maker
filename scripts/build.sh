#!/usr/bin/env bash
# 一键构建三平台发布产物，输出到 dist/。
#
# 工具链：
#   - cargo + rustup（必需）
#   - cargo-zigbuild（macOS → Windows / Linux 交叉编译）
#   - zig（cargo-zigbuild 的 C 链接器，macOS 上 brew install zig）
#   - 国内网络需代理：export https_proxy=http://127.0.0.1:7890
#
# 用法：
#   ./scripts/build.sh                       # 全平台
#   ./scripts/build.sh linux windows         # 子集
#   ./scripts/build.sh --skip-test          # 跳过 cargo test
set -euo pipefail

cd "$(dirname "$0")/.."

SKIP_TEST=0
for arg in "$@"; do
  case "$arg" in
    --skip-test) SKIP_TEST=1 ;;
    *) ;;  # platform filter handled below
  esac
done

run_test() {
  if [ "$SKIP_TEST" -eq 0 ]; then
    echo "==> cargo test"
    cargo test
  fi
}

build_macos_arm64() {
  echo "==> macOS arm64 (Apple Silicon)"
  cargo build --release --target aarch64-apple-darwin
}

build_macos_x86_64() {
  echo "==> macOS x86_64 (Intel)"
  cargo build --release --target x86_64-apple-darwin
}

build_windows() {
  echo "==> Windows x86_64 (MinGW)"
  cargo zigbuild --release --target x86_64-pc-windows-gnu
}

build_linux() {
  echo "==> Linux x86_64 (musl static)"
  cargo zigbuild --release --target x86_64-unknown-linux-musl
}

requested=("$@")
should_run() {
  local target=$1
  if [ ${#requested[@]} -eq 0 ]; then return 0; fi
  for r in "${requested[@]}"; do
    [ "$r" = "$target" ] && return 0
  done
  return 1
}

mkdir -p dist
run_test

if should_run macos-arm64; then build_macos_arm64; fi
if should_run macos-x86_64; then build_macos_x86_64; fi
if should_run windows; then build_windows; fi
if should_run linux; then build_linux; fi

# Stage
echo "==> staging to dist/"
[ -f target/aarch64-apple-darwin/release/auto-comfy-maker ]   && cp target/aarch64-apple-darwin/release/auto-comfy-maker dist/auto-comfy-maker-macos-arm64
[ -f target/x86_64-apple-darwin/release/auto-comfy-maker ]    && cp target/x86_64-apple-darwin/release/auto-comfy-maker  dist/auto-comfy-maker-macos-x86_64
[ -f target/x86_64-pc-windows-gnu/release/auto-comfy-maker.exe ] && cp target/x86_64-pc-windows-gnu/release/auto-comfy-maker.exe dist/auto-comfy-maker-windows-x86_64.exe
[ -f target/x86_64-unknown-linux-musl/release/auto-comfy-maker ] && cp target/x86_64-unknown-linux-musl/release/auto-comfy-maker dist/auto-comfy-maker-linux-x86_64

# SHA256
( cd dist && shasum -a 256 auto-comfy-maker-* > SHA256SUMS )
echo "==> done. Files in dist/:"
ls -lh dist/
echo "==> SHA256:"
cat dist/SHA256SUMS