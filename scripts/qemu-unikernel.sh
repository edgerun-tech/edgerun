#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
target="${TARGET:-x86_64-unknown-none}"
profile="${PROFILE:-release}"
build_dir="$repo_root/target/$target/$profile"
kernel_elf="$build_dir/edgerun-unikernel"
kernel_bin="${KERNEL_BIN:-/tmp/edgerun.bin}"
boot_obj="${BOOT_OBJ:-/tmp/edgerun-qemu-boot.o}"
boot_bin="${BOOT_BIN:-/tmp/edgerun-qemu-boot.bin}"
boot_sector="${BOOT_SECTOR:-/tmp/edgerun-qemu-boot-512.bin}"
timeout_seconds="${QEMU_TIMEOUT:-8}"
qemu_log="${QEMU_LOG:-/tmp/edgerun-qemu.log}"
expected_marker="${QEMU_EXPECT:-VirtIO found}"
qemu_net_dump="${QEMU_NET_DUMP:-}"

cargo +nightly build --release -p edgerun-unikernel \
    --target "$target" \
    -Zbuild-std=core,alloc

/usr/bin/objcopy -O binary "$kernel_elf" "$kernel_bin"

qemu_extra_args=()
if [[ -n "$qemu_net_dump" ]]; then
    rm -f "$qemu_net_dump"
    qemu_extra_args+=(-object "filter-dump,id=edgerun-net-dump,netdev=n0,file=$qemu_net_dump")
fi

as --32 -o "$boot_obj" "$repo_root/crates/edgerun-unikernel/qemu_boot.S"
ld -m elf_i386 -Ttext 0x7c00 --oformat binary -o "$boot_bin" "$boot_obj"
dd if="$boot_bin" of="$boot_sector" bs=512 count=1 status=none

set +e
timeout "$timeout_seconds" qemu-system-x86_64 \
    -m "${QEMU_MEMORY:-128M}" \
    -nographic \
    -serial mon:stdio \
    -no-reboot \
    -no-shutdown \
    -drive "file=$boot_sector,format=raw,if=floppy" \
    -boot a \
    -device "loader,file=$kernel_bin,addr=0x100000,force-raw=on" \
    -netdev user,id=n0 \
    -device virtio-net-pci,disable-legacy=on,disable-modern=off,netdev=n0 \
    "${qemu_extra_args[@]}" \
    2>&1 | tee "$qemu_log"
qemu_status=${PIPESTATUS[0]}
set -e

if [[ "$qemu_status" -eq 124 ]]; then
    echo "QEMU timeout reached after ${timeout_seconds}s"
elif [[ "$qemu_status" -ne 0 ]]; then
    exit "$qemu_status"
fi

if grep -q "$expected_marker" "$qemu_log"; then
    echo "QEMU smoke test reached marker: $expected_marker"
else
    echo "QEMU smoke test did not reach marker: $expected_marker" >&2
    echo "Serial log: $qemu_log" >&2
    exit 1
fi
