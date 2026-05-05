#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

CRATE="edgerun-sign-verify-e2e"
TARGET="wasm32-unknown-unknown"
HOST_TARGET="$(rustc -vV | awk '/^host:/ { print $2 }')"
PROFILE="release"
BENCH_ITERS="${1:-1000}"

cargo_target_dir() {
  cargo metadata --format-version 1 --no-deps \
    | python -c 'import json,sys; print(json.load(sys.stdin)["target_directory"])'
}

TARGET_DIR="$(cargo_target_dir)"
WASM="${TARGET_DIR}/${TARGET}/${PROFILE}/edgerun_sign_verify_e2e.wasm"
OUT_DIR="${TARGET_DIR}/${TARGET}/${PROFILE}"

wasm_opt_oz() {
  local input="$1"
  local output="$2"

  if wasm-opt --enable-bulk-memory -Oz "$input" -o "$output" 2>/tmp/edgerun-wasm-opt.err; then
    return 0
  fi

  if wasm-opt --enable-bulk-memory-opt -Oz "$input" -o "$output" 2>/tmp/edgerun-wasm-opt.err; then
    return 0
  fi

  echo "wasm-opt failed; stderr:" >&2
  cat /tmp/edgerun-wasm-opt.err >&2
  return 1
}

if ! rustup target list --installed | grep -qx "$TARGET"; then
  echo "Installing Rust target: $TARGET"
  rustup target add "$TARGET"
fi

if ! rustup target list --installed | grep -qx "$HOST_TARGET"; then
  echo "Installing Rust host target: $HOST_TARGET"
  rustup target add "$HOST_TARGET"
fi

echo "== native tests =="
echo "host_target=$HOST_TARGET"
cargo test -p "$CRATE" --target "$HOST_TARGET"

echo "== native benchmark =="
cargo run --release -p "$CRATE" --target "$HOST_TARGET" --bin edgerun-sign-verify-bench -- "$BENCH_ITERS"

echo "== wasm build =="
cargo build --release -p "$CRATE" --no-default-features --target "$TARGET"

if [[ ! -f "$WASM" ]]; then
  echo "missing wasm output: $WASM" >&2
  echo "searching for crate wasm outputs under ${TARGET_DIR}/${TARGET}/${PROFILE}:" >&2
  find "${TARGET_DIR}/${TARGET}/${PROFILE}" -maxdepth 3 -type f -name '*sign*verify*.wasm' -print >&2 || true
  exit 1
fi

raw_size=$(wc -c < "$WASM" | tr -d ' ')
echo "wasm_path=$WASM"
echo "wasm_raw_bytes=$raw_size"

if command -v wasm-strip >/dev/null 2>&1; then
  STRIPPED="${OUT_DIR}/edgerun_sign_verify_e2e.stripped.wasm"
  cp "$WASM" "$STRIPPED"
  wasm-strip "$STRIPPED"
  stripped_size=$(wc -c < "$STRIPPED" | tr -d ' ')
  echo "wasm_stripped_path=$STRIPPED"
  echo "wasm_stripped_bytes=$stripped_size"
fi

if command -v wasm-opt >/dev/null 2>&1; then
  OPT="${OUT_DIR}/edgerun_sign_verify_e2e.opt.wasm"
  if wasm_opt_oz "$WASM" "$OPT"; then
    opt_size=$(wc -c < "$OPT" | tr -d ' ')
    echo "wasm_opt_Oz_path=$OPT"
    echo "wasm_opt_Oz_bytes=$opt_size"
  else
    echo "wasm_opt_Oz_bytes=unavailable"
  fi
fi

if command -v wasm2wat >/dev/null 2>&1; then
  echo "== wasm exports =="
  wasm2wat --enable-bulk-memory "$WASM" | grep '^(export ' || wasm2wat "$WASM" | grep '^(export ' || true
fi

echo "== done =="
