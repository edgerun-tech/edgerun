#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CRATE_DIR="$ROOT_DIR/crates/edgerun-runtime-web"
OUT_DIR="$ROOT_DIR/frontend/public/wasm/edgerun-runtime-web"

if ! command -v wasm-pack >/dev/null 2>&1; then
  echo "error: wasm-pack is required. Install with: cargo install wasm-pack" >&2
  exit 1
fi

mkdir -p "$OUT_DIR"

cd "$CRATE_DIR"
wasm-pack build \
  --target web \
  --release \
  --out-dir "$OUT_DIR"

echo "built wasm package at: $OUT_DIR"
