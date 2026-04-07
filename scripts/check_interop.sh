#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."

echo "=== Running edgerun v0 conformance corpus ==="

(cd rust && cargo test -p edgerun-core -- conformance --nocapture 2>&1)

echo "conformance check complete"
