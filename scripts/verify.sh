#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

echo "[1/2] Running worker/scheduler Rust tests"
cargo test -p edgerun-worker -p edgerun-scheduler --quiet

echo "[2/2] Running Anchor test gate"
(
  cd program
  ./scripts/anchor-test-verify.sh
)
