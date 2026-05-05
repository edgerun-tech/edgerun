#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

CRATE="edgerun-sign-verify-e2e"
TARGET="wasm32-unknown-unknown"
PROFILE="release"
WASM="target/${TARGET}/${PROFILE}/edgerun_sign_verify_e2e.wasm"
BENCH_ITERS="${1:-1000}"

if ! rustup target list --installed | grep -qx "$TARGET"; then
  echo "Installing Rust target: $TARGET"
  rustup target add "$TARGET"
fi

echo "== native tests =="
cargo test -p "$CRATE"

echo "== native benchmark =="
cargo run --release -p "$CRATE" --bin edgerun-sign-verify-bench -- "$BENCH_ITERS"

echo "== wasm build =="
cargo build --release -p "$CRATE" --no-default-features --target "$TARGET"

if [[ ! -f "$WASM" ]]; then
  echo "missing wasm output: $WASM" >&2
  exit 1
fi

raw_size=$(wc -c < "$WASM" | tr -d ' ')
echo "wasm_raw_bytes=$raw_size"

if command -v wasm-strip >/dev/null 2>&1; then
  STRIPPED="target/${TARGET}/${PROFILE}/edgerun_sign_verify_e2e.stripped.wasm"
  cp "$WASM" "$STRIPPED"
  wasm-strip "$STRIPPED"
  stripped_size=$(wc -c < "$STRIPPED" | tr -d ' ')
  echo "wasm_stripped_bytes=$stripped_size"
fi

if command -v wasm-opt >/dev/null 2>&1; then
  OPT="target/${TARGET}/${PROFILE}/edgerun_sign_verify_e2e.opt.wasm"
  wasm-opt -Oz "$WASM" -o "$OPT"
  opt_size=$(wc -c < "$OPT" | tr -d ' ')
  echo "wasm_opt_Oz_bytes=$opt_size"
fi

if command -v wasm2wat >/dev/null 2>&1; then
  echo "== wasm exports =="
  wasm2wat "$WASM" | grep '^(export ' || true
fi

echo "== done =="
