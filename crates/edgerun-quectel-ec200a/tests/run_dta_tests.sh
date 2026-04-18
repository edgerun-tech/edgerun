#!/bin/bash
set -e

echo "========================================"
echo "Quectel EC200A DTA Network End-to-End Tests"
echo "========================================"
echo ""

# Compile the test binary
echo "Compiling test binary..."
cargo test --no-run --quiet 2>/dev/null

# Run the DTA integration tests
echo "Running DTA integration tests..."
cargo test --test dta_integration 2>&1 | tail -30

echo ""
echo "========================================"
echo "Test Summary"
echo "========================================"
