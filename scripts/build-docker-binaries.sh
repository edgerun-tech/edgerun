#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

export CARGO_TARGET_DIR="$ROOT_DIR/out/target"

cargo build --locked --release \
  -p edgerun-scheduler \
  -p edgerun-worker

cargo build --locked --release -p edgerun-term-server --features term
