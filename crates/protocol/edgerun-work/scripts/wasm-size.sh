#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TARGET_DIR="${CARGO_TARGET_DIR:-$ROOT/target}"
TARGET="wasm32-unknown-unknown"
PROFILE="release"
PKG="edgerun-work"

cd "$ROOT"

cargo build \
  --target "$TARGET" \
  --release \
  --no-default-features \
  --lib

WASM="$TARGET_DIR/$TARGET/$PROFILE/${PKG//-/_}.wasm"

if [[ ! -f "$WASM" ]]; then
  echo "wasm artifact not found: $WASM" >&2
  echo "try: find '$TARGET_DIR' -name '*.wasm' -type f" >&2
  exit 1
fi

RAW_BYTES="$(wc -c < "$WASM" | tr -d ' ')"
printf 'raw_wasm=%s\nraw_bytes=%s\n' "$WASM" "$RAW_BYTES"

if command -v wasm-opt >/dev/null 2>&1; then
  OPT_WASM="${WASM%.wasm}.opt.wasm"
  wasm-opt -Oz "$WASM" -o "$OPT_WASM"
  OPT_BYTES="$(wc -c < "$OPT_WASM" | tr -d ' ')"
  printf 'opt_wasm=%s\nopt_bytes=%s\n' "$OPT_WASM" "$OPT_BYTES"
else
  echo 'wasm-opt not found; install binaryen to get opt_bytes'
fi
