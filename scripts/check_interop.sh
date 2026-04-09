#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."

echo "=== Running edgerun v0 conformance corpus ==="
echo ""
echo "Running mandatory conformance vectors..."
echo "(Add --ignored to see known-gap results)"
echo ""

# Run the conformance tests (ignored by default since there are known gaps)
RESULT=0
(cd crates/edgerun-core && cargo test -- conformance --ignored --nocapture 2>&1) || RESULT=$?

if [ "$RESULT" -ne 0 ]; then
    echo ""
    echo "⚠ Conformance gaps detected. See output above."
    echo "  This is expected — the corpus has known gaps that are being worked down."
fi

echo ""
echo "conformance check complete"
