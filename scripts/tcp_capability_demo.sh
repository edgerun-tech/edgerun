#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
RUST_DIR="$ROOT/rust"
ADDR="${1:-127.0.0.1:47070}"
SERVER_LOG="${TMPDIR:-/tmp}/edgerun-capability-demo-server.log"
CLIENT_LOG="${TMPDIR:-/tmp}/edgerun-capability-demo-client.log"

cleanup() {
  if [[ -n "${server_pid:-}" ]]; then
    kill "$server_pid" 2>/dev/null || true
    wait "$server_pid" 2>/dev/null || true
  fi
}
trap cleanup EXIT

cd "$RUST_DIR"

echo "[demo] starting server on tcp:$ADDR"
cargo run -q -p edgerun-remote-capability --bin capability-demo-server -- tcp "$ADDR" \
  > /dev/null 2>"$SERVER_LOG" &
server_pid=$!

sleep 1

echo "[demo] running client"
cargo run -q -p edgerun-remote-capability --bin capability-demo-client -- tcp "$ADDR" \
  >"$CLIENT_LOG" 2>&1

cleanup
trap - EXIT

echo "--- server log ---"
cat "$SERVER_LOG"
echo "--- client log ---"
cat "$CLIENT_LOG"
