#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
set -euo pipefail

ROOT_DIR="$(git rev-parse --show-toplevel 2>/dev/null || (cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd))"
CRATE_DIR="$ROOT_DIR/crates/edgerun-runtime-web"
PKG_DIR="$CRATE_DIR/pkg-web"
FRONTEND_OUT_DIR="$ROOT_DIR/frontend/public/wasm/edgerun-runtime-web"
SYNC_FRONTEND=1

usage() {
  cat <<'EOF'
Usage: build-runtime-web.sh [--no-sync] [--help]

Builds edgerun-runtime-web wasm package into:
  crates/edgerun-runtime-web/pkg-web

Options:
  --no-sync   Skip mirroring artifacts into frontend/public/wasm/...
  --help      Show this help text
EOF
}

while [ $# -gt 0 ]; do
  case "$1" in
    --no-sync)
      SYNC_FRONTEND=0
      shift
      ;;
    --help|-h)
      usage
      exit 0
      ;;
    *)
      echo "error: unknown argument: $1" >&2
      usage >&2
      exit 1
      ;;
  esac
done

if ! command -v wasm-pack >/dev/null 2>&1; then
  echo "error: wasm-pack is required. Install with: cargo install wasm-pack" >&2
  exit 1
fi

mkdir -p "$PKG_DIR"

cd "$CRATE_DIR"
wasm-pack build \
  --target web \
  --release \
  --out-dir "$PKG_DIR"

echo "built wasm package at: $PKG_DIR"

if [ "$SYNC_FRONTEND" = "1" ] && [ -d "$ROOT_DIR/frontend" ]; then
  mkdir -p "$FRONTEND_OUT_DIR"
  cp -a "$PKG_DIR"/. "$FRONTEND_OUT_DIR"/
  echo "synced wasm package to: $FRONTEND_OUT_DIR"
fi
