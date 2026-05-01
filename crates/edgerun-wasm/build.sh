#!/bin/bash
set -e

# EdgeRun WASM Build Script
# Usage: ./build.sh [example-name]

EXAMPLE="${1:-hello}"
TARGET="wasm32-unknown-unknown"

echo "Building EdgeRun WASM: $EXAMPLE"

# Build for wasm32-unknown-unknown
cargo build --target $TARGET --release -p edgerun-sdk --example $EXAMPLE

# Locate output
OUTPUT="target/$TARGET/release/examples/$EXAMPLE.wasm"

if [ ! -f "$OUTPUT" ]; then
    echo "ERROR: Build failed, output not found"
    exit 1
fi

echo "Built: $OUTPUT"

# Run wasm-opt if available
if command -v wasm-opt &> /dev/null; then
    echo "Optimizing with wasm-opt..."
    wasm-opt -Oz "$OUTPUT" -o "$OUTPUT"
    echo "Optimized: $OUTPUT"
fi

# Copy to app.wasm
cp "$OUTPUT" "app.wasm"
echo "Output: app.wasm ($(stat -f%z "$OUTPUT") bytes)"

echo ""
echo "To validate:"
echo "  cargo run -p edgerun-wasm --bin edgerun-validate app.wasm"
echo ""
echo "To run locally:"
echo "  cargo run -p edgerun-wasm --bin edgerun-runtime -- --input 'test'"