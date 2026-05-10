#!/usr/bin/env sh
set -eu

if [ "$#" -lt 1 ]; then
  echo "usage: $0 /path/to/edk2 [usb-mount]" >&2
  exit 2
fi

EDK2_DIR="$1"
USB_MOUNT="${2:-}"
PKG_NAME="EdgerunEfiBtPkg"
SRC_DIR="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
PKG_DIR="$EDK2_DIR/$PKG_NAME"

if [ ! -d "$EDK2_DIR/MdePkg" ]; then
  echo "not an EDK II checkout: $EDK2_DIR" >&2
  exit 1
fi

rm -rf "$PKG_DIR"
mkdir -p "$PKG_DIR"
cp "$SRC_DIR"/*.c "$SRC_DIR"/*.h "$SRC_DIR"/*.inf "$SRC_DIR"/*.dsc "$PKG_DIR"/

(
  cd "$EDK2_DIR"
  if [ -f edksetup.sh ]; then
    . ./edksetup.sh >/dev/null
  fi
  build -a X64 -t GCC5 -b RELEASE -p "$PKG_NAME/$PKG_NAME.dsc"
)

EFI="$EDK2_DIR/Build/$PKG_NAME/RELEASE_GCC5/X64/EdgerunEfiBt.efi"
if [ ! -f "$EFI" ]; then
  echo "built EFI not found: $EFI" >&2
  exit 1
fi

echo "built: $EFI"

if [ -n "$USB_MOUNT" ]; then
  mkdir -p "$USB_MOUNT/EFI/BOOT"
  cp "$EFI" "$USB_MOUNT/EFI/BOOT/BOOTX64.EFI"
  sync
  echo "installed: $USB_MOUNT/EFI/BOOT/BOOTX64.EFI"
fi
