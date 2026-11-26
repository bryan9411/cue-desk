#!/usr/bin/env bash
# 專案整體檢查：Rust 編譯／Lint／測試、前端建置／測試／格式，以及（存在時）樣式檢查。
set -euo pipefail

cd "$(dirname "$0")/.."

MANIFEST="src-tauri/Cargo.toml"

step() {
  printf '\n==> %s\n' "$1"
}

step "cargo check"
cargo check --manifest-path "$MANIFEST"

step "cargo clippy"
cargo clippy --manifest-path "$MANIFEST" -- -D warnings

step "cargo test"
cargo test --manifest-path "$MANIFEST"

step "pnpm build"
pnpm build

step "pnpm test"
pnpm test

step "pnpm format:check"
pnpm format:check

if node -e 'process.exit(require("./package.json").scripts?.["lint:style"] ? 0 : 1)'; then
  step "pnpm lint:style"
  pnpm lint:style
fi

printf '\n全部檢查通過\n'
