#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

export CARGO_TARGET_DIR="$ROOT_DIR/out/target"

cargo build --locked --release \
  -p edgerun-cli \
  -p edgerun-scheduler \
  -p edgerun-term-server \
  -p edgerun-worker
