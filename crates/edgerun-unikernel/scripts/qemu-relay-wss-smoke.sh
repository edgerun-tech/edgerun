#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
TARGET="x86_64-unknown-none"
BIN="$ROOT/target/$TARGET/release/edgerun-unikernel-relay-wss-smoke"
LOG="$ROOT/target/qemu-relay-wss-smoke.log"
BOOT_OBJ="$ROOT/target/qemu-relay-wss-boot.o"
BOOT_BIN="$ROOT/target/qemu-relay-wss-boot.bin"
BOOT_LD="$ROOT/target/qemu-relay-wss-boot.ld"
RAW_BIN="$ROOT/target/qemu-relay-wss-smoke.bin"

cd "$ROOT"

CARGO_TARGET_DIR="$ROOT/target" RUSTFLAGS="${RUSTFLAGS:-} -C relocation-model=static" cargo build \
  --manifest-path "$ROOT/crates/edgerun-unikernel/Cargo.toml" \
  --bin edgerun-unikernel-relay-wss-smoke \
  --no-default-features \
  --features relay-wss-smoke \
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

set +e
timeout 20s qemu-system-x86_64 \
  -machine accel=tcg \
  -m 64M \
  -no-reboot \
  -no-shutdown \
  -boot a \
  -display none \
  -serial none \
  -debugcon "file:$LOG" \
  -global isa-debugcon.iobase=0xe9 \
  -device isa-debug-exit,iobase=0xf4,iosize=0x04 \
  -drive "file=$BOOT_BIN,format=raw,if=floppy" \
  -device "loader,file=$RAW_BIN,addr=0x100000,force-raw=on"
STATUS=$?
set -e

cat "$LOG"

if [[ "$STATUS" != "1" ]]; then
  echo "qemu exited with unexpected status $STATUS" >&2
  exit 1
fi

grep -q "edgerun-unikernel relay wss smoke: wss cert in memory" "$LOG"
