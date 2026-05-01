#!/bin/bash
set -e

TARGET="wasm32-unknown-unknown"
WASM_TARGET="${WASM_TARGET:-x86_64-unknown-linux-gnu}"

echo "=== EdgeRun WASM Build ==="
echo ""

# Build example WASM
echo "Building WASM module..."
cargo build --target "$TARGET" --release -p edgerun-example 2>&1 | grep -v warning || true

WASM="target/$TARGET/release/edgerun_example.wasm"

if [ ! -f "$WASM" ]; then
    echo "ERROR: Build failed, output not found: $WASM"
    exit 1
fi

echo "Built: $WASM"

# Optimize with wasm-opt if available
if command -v wasm-opt &> /dev/null; then
    echo "Optimizing with wasm-opt..."
    wasm-opt -Oz --enable-bulk-memory "$WASM" -o "$WASM"
    echo "Optimized: $WASM"
fi

cp "$WASM" "app.wasm"
SIZE=$(wc -c < app.wasm)
echo ""
echo "Output: app.wasm ($SIZE bytes)"

# Validate
echo ""
echo "Validating..."
cargo run -p edgerun-wasm --bin edgerun-validate --target "$WASM_TARGET" -- app.wasm --verbose 2>&1

# Run
echo ""
echo "Running..."
cargo run -p edgerun-wasm --bin edgerun-runtime --target "$WASM_TARGET" -- app.wasm --verbose 2>&1
