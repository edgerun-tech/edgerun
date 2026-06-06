#!/bin/sh
# Build all pipeline modules including WASM interpreter integration
set -e

W2W="wasm-tools parse"
DIR="$(dirname "$(realpath "$0")")"
ROOT="$(dirname "$DIR")"
RUNTIME="$(realpath "$DIR/../../../system/runtime")"

echo "=== Building pipeline modules ==="
echo "  root:    $ROOT"
echo "  runtime: $RUNTIME"
echo ""

# Runtime core (shared memory + offset globals + status codes)
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
$W2W "$ROOT/net/socket-core.wat" -o "$ROOT/net/socket-core.wasm"
echo "  pipeline-core..."
$W2W "$ROOT/io/pipeline-core.wat" -o "$ROOT/io/pipeline-core.wasm"

# Encoding stages
echo "  encoding-text..."
$W2W "$ROOT/encoding/encoding-text.wat" -o "$ROOT/encoding/encoding-text.wasm"

# WS stage
echo "  ws-stage..."
$W2W "$ROOT/protocol/ws/ws-stage.wat" -o "$ROOT/protocol/ws/ws-stage.wasm"

# ── WASM interpreter integration ──
echo "  wasm-interpreter (pipeline-local copy)..."
$W2W "$DIR/wasm-interpreter.wat" -o "$DIR/wasm-interpreter.wasm"
echo "  wasm-exec-stage..."
$W2W "$DIR/wasm-exec-stage.wat" -o "$DIR/wasm-exec-stage.wasm"
echo "  wat-parse-core..."
$W2W "$ROOT/text/wat-parse-core.wat" -o "$ROOT/text/wat-parse-core.wasm"
echo "  process-wat-parse..."
$W2W "$DIR/process-wat-parse.wat" -o "$DIR/process-wat-parse.wasm"

# Stage registry (wires all stages into 64-slot dispatch table)
echo "  stage-registry..."
$W2W "$DIR/stage-registry.wat" -o "$DIR/stage-registry.wasm"

echo ""
echo "=== All modules built successfully ==="
