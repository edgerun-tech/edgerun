#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TARGET="x86_64-unknown-none"
BIN="$ROOT/target/$TARGET/release/edgerun-relay-qemu-smoke"
LOG="$ROOT/target/qemu-relay-wss-smoke.log"

cd "$ROOT"

CARGO_TARGET_DIR="$ROOT/target" cargo build \
  -p edgerun-relay \
  --bin edgerun-relay-qemu-smoke \
  --no-default-features \
  --features wss \
  --target "$TARGET" \
  --release

rm -f "$LOG"

set +e
timeout 20s qemu-system-x86_64 \
  -machine accel=tcg \
  -m 64M \
  -no-reboot \
  -no-shutdown \
  -display none \
  -serial "file:$LOG" \
  -device isa-debug-exit,iobase=0x501,iosize=0x01 \
  -kernel "$BIN"
STATUS=$?
set -e

cat "$LOG"

if [[ "$STATUS" != "1" ]]; then
  echo "qemu exited with unexpected status $STATUS" >&2
  exit 1
fi

grep -q "edgerun-relay qemu smoke: wss cert in memory" "$LOG"
