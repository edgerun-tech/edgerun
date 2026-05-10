#!/usr/bin/env sh
set -eu

USB_MOUNT="${1:-/mnt/usb}"
TARGET="x86_64-unknown-uefi"
NAME="edgerun-efi-bluetooth"

rustup target add "$TARGET"
cargo build --release

mkdir -p "$USB_MOUNT/EFI/BOOT"
cp "target/$TARGET/release/$NAME.efi" "$USB_MOUNT/EFI/BOOT/BOOTX64.EFI"
sync

echo "installed $USB_MOUNT/EFI/BOOT/BOOTX64.EFI"
