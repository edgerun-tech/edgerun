#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
MODE="${1:-all}"
export CARGO_TARGET_DIR="${ROOT_DIR}/out/target"

case "${MODE}" in
  all)
    cargo build --release \
      -p edgerun-scheduler \
      -p edgerun-worker \
      -p edgerun-term-server
    ;;
  stack)
    cargo build --release \
      -p edgerun-scheduler \
      -p edgerun-worker
    ;;
  terminal)
    cargo build --release -p edgerun-term-server
    ;;
  *)
    echo "usage: $0 [all|stack|terminal]" >&2
    exit 1
    ;;
esac

