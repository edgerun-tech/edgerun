#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
TARGET="x86_64-unknown-none"
BIN_NAME="edgerun-unikernel-relay-virtio"
FEATURES="relay-virtio-runtime"
LOG_NAME="${LOG_NAME:-qemu-relay-virtio-e2e}"
HOST_PORT="${HOST_PORT:-17999}"
ITERATIONS="${1:-100}"
PAYLOAD_LEN="${2:-256}"
WINDOW="${3:-32}"
BIN="$ROOT/target/$TARGET/release/$BIN_NAME"
LOG="$ROOT/target/$LOG_NAME.log"
BOOT_OBJ="$ROOT/target/$LOG_NAME-boot.o"
BOOT_BIN="$ROOT/target/$LOG_NAME-boot.bin"
BOOT_LD="$ROOT/target/$LOG_NAME-boot.ld"
RAW_BIN="$ROOT/target/$LOG_NAME.bin"

cd "$ROOT"

CARGO_TARGET_DIR="$ROOT/target" RUSTFLAGS="${RUSTFLAGS:-} -C relocation-model=static" cargo build \
  --manifest-path "$ROOT/crates/edgerun-unikernel/Cargo.toml" \
  --bin "$BIN_NAME" \
  --no-default-features \
  --features "$FEATURES" \
  --target "$TARGET" \
  --release

rm -f "$LOG"

objcopy -O binary "$BIN" "$RAW_BIN"
cat > "$BOOT_LD" <<'EOF'
OUTPUT_FORMAT(binary)
ENTRY(_start)
SECTIONS
{
  . = 0x7c00;
  .text : { *(.text) }
  /DISCARD/ : { *(.note*) *(.eh_frame*) }
}
EOF
as --32 "$ROOT/crates/edgerun-unikernel/qemu_boot.S" -o "$BOOT_OBJ"
ld -m elf_i386 -T "$BOOT_LD" "$BOOT_OBJ" -o "$BOOT_BIN"

if [[ "$(wc -c < "$BOOT_BIN")" != "512" ]]; then
  echo "qemu boot sector is not 512 bytes" >&2
  exit 1
fi

cleanup() {
  if [[ -n "${QEMU_PID:-}" ]] && kill -0 "$QEMU_PID" 2>/dev/null; then
    kill "$QEMU_PID" 2>/dev/null || true
    wait "$QEMU_PID" 2>/dev/null || true
  fi
}
trap cleanup EXIT

timeout "${QEMU_TIMEOUT:-90s}" qemu-system-x86_64 \
  -machine accel=tcg \
  -m 64M \
  -no-reboot \
  -boot a \
  -display none \
  -serial none \
  -debugcon "file:$LOG" \
  -global isa-debugcon.iobase=0xe9 \
  -device isa-debug-exit,iobase=0xf4,iosize=0x04 \
  -drive "file=$BOOT_BIN,format=raw,if=floppy" \
  -device "loader,file=$RAW_BIN,addr=0x100000,force-raw=on" \
  -device virtio-rng-pci,disable-legacy=on \
  -netdev "user,id=relaynet,hostfwd=udp:127.0.0.1:${HOST_PORT}-:7999" \
  -device virtio-net-pci,disable-legacy=on,netdev=relaynet,mac=52:54:00:12:34:56 \
  &
QEMU_PID=$!

for _ in $(seq 1 200); do
  if [[ -f "$LOG" ]] && grep -q "edgerun-unikernel relay virtio: relay ready" "$LOG"; then
    break
  fi
  if ! kill -0 "$QEMU_PID" 2>/dev/null; then
    cat "$LOG" 2>/dev/null || true
    echo "qemu exited before relay became ready" >&2
    exit 1
  fi
  sleep 0.05
done

if ! grep -q "edgerun-unikernel relay virtio: relay ready" "$LOG"; then
  cat "$LOG" 2>/dev/null || true
  echo "relay did not become ready" >&2
  exit 1
fi

CARGO_NET_OFFLINE="${CARGO_NET_OFFLINE:-true}" cargo run \
  --manifest-path "$ROOT/crates/relay/edgerun-relay/Cargo.toml" \
  --release \
  --bin edgerun-relay-bench \
  --features std \
  --no-default-features \
  -- \
  --client-custom-quic "127.0.0.1:${HOST_PORT}" "$ITERATIONS" "$PAYLOAD_LEN" "$WINDOW"

cat "$LOG"
