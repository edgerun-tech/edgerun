#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TARGET="xtensa-esp32s3-none-elf"
CHIP="esp32s3"
PORT="${ESPFLASH_PORT:-/dev/ttyACM0}"
BAUD="${ESPFLASH_BAUD:-921600}"
MONITOR_BAUD="${MONITOR_BAUD:-115200}"
ESP_TOOLCHAIN_BIN="${ESP_TOOLCHAIN_BIN:-$ROOT/devices/edgerun-tcl-usb-ap-bridge/.embuild/espressif/tools/xtensa-esp-elf/esp-15.2.0_20251204/xtensa-esp-elf/bin}"
PROFILE="${PROFILE:-release}"
FEATURES="${FEATURES:-esp32s3-wifi-mmio,esp32s3-headless}"
OUT_DIR="$ROOT/target/edgerun-esp32s3"

if [[ "$PROFILE" == "release" ]]; then
  ELF="$ROOT/target/$TARGET/release/edgerun-unikernel"
  BUILD_PROFILE=(--release)
else
  ELF="$ROOT/target/$TARGET/debug/edgerun-unikernel"
  BUILD_PROFILE=()
fi

APP_BIN="$OUT_DIR/edgerun-unikernel.bin"

export PATH="$ESP_TOOLCHAIN_BIN:$PATH"
export ESPFLASH_SKIP_UPDATE_CHECK="${ESPFLASH_SKIP_UPDATE_CHECK:-true}"

build_elf() {
  local feature_args=()
  if [[ -n "$FEATURES" ]]; then
    feature_args=(--features "$FEATURES")
  fi
  cargo +esp build "${BUILD_PROFILE[@]}" -p edgerun-unikernel "${feature_args[@]}" --target "$TARGET" -Zbuild-std=core,alloc
}

save_image() {
  mkdir -p "$OUT_DIR"
  espflash save-image \
    --chip "$CHIP" \
    --flash-mode dio \
    --flash-freq 40mhz \
    --flash-size 16mb \
    "$ELF" \
    "$APP_BIN"
}

image_info() {
  esptool.py --chip "$CHIP" image-info "$APP_BIN"
}

case "${1:-image}" in
  build)
    build_elf
    ;;
  layout)
    build_elf
    "$ROOT/scripts/esp32s3-elf-layout.py" \
      --toolchain-bin "$ESP_TOOLCHAIN_BIN" \
      "$ELF"
    ;;
  layout-strict)
    build_elf
    "$ROOT/scripts/esp32s3-elf-layout.py" \
      --toolchain-bin "$ESP_TOOLCHAIN_BIN" \
      --strict \
      "$ELF"
    ;;
  image)
    build_elf
    save_image
    image_info
    ;;
  flash)
    build_elf
    espflash flash \
      --chip "$CHIP" \
      --port "$PORT" \
      --baud "$BAUD" \
      --non-interactive \
      "$ELF"
    ;;
  monitor)
    espflash monitor \
      --chip "$CHIP" \
      --port "$PORT" \
      --baud "$MONITOR_BAUD"
    ;;
  ports)
    espflash list-ports
    ;;
  *)
    echo "usage: $0 {build|layout|layout-strict|image|flash|monitor|ports}" >&2
    exit 2
    ;;
esac
