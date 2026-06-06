#!/bin/sh
# Build all pipeline modules for WASM interpreter integration
set -e

W2W="wasm-tools wat2wasm"
ROOT="${WASM_BUILD_DIR:-$(dirname "$0")/..}"
RUNTIME="${RUNTIME_DIR:-$(dirname "$0")/../../system/runtime}"

echo "=== Building pipeline modules ==="

# Runtime core
echo "  edgerun-core..."
$W2W "$RUNTIME/edgerun-core.wat" -o "$ROOT/io/edgerun-core.wasm"

# I/O primitives
echo "  pipe-core..."
$W2W "$ROOT/io/pipe-core.wat" -o "$ROOT/io/pipe-core.wasm"

echo "  frame-core..."
$W2W "$ROOT/io/frame-core.wat" -o "$ROOT/io/frame-core.wasm"

echo "  mux-core..."
$W2W "$ROOT/io/mux-core.wat" -o "$ROOT/io/mux-core.wasm"

echo "  socket-core..."
$W2W "$ROOT/io/socket-core.wat" -o "$ROOT/io/socket-core.wasm"

echo "  pipeline-core..."
$W2W "$ROOT/io/pipeline-core.wat" -o "$ROOT/io/pipeline-core.wasm"

# Encoding stages
echo "  encoding-text..."
$W2W "$ROOT/encoding/encoding-text.wat" -o "$ROOT/encoding/encoding-text.wasm"

# WS stage
echo "  ws-stage..."
$W2W "$ROOT/net/ws-stage.wat" -o "$ROOT/net/ws-stage.wasm"

# WASM interpreter (pipeline-local copy)
echo "  wasm-interpreter..."
$W2W "$(dirname "$0")/wasm-interpreter.wat" -o "$(dirname "$0")/wasm-interpreter.wasm"

# WASM exec stage
echo "  wasm-exec-stage..."
$W2W "$(dirname "$0")/wasm-exec-stage.wat" -o "$(dirname "$0")/wasm-exec-stage.wasm"

# Stage registry (wires all stages into dispatch table)
echo "  stage-registry..."
$W2W "$(dirname "$0")/stage-registry.wat" -o "$(dirname "$0")/stage-registry.wasm"

echo "=== All modules built successfully ==="
