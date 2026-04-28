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
qemu_netdev="${QEMU_NETDEV:-user,id=n0}"
qemu_tpm_socket="${QEMU_TPM_SOCKET:-}"
qemu_tpm_device="${QEMU_TPM_DEVICE:-tpm-crb}"
qemu_virtio_rng="${QEMU_VIRTIO_RNG:-1}"
qemu_virtio_console="${QEMU_VIRTIO_CONSOLE:-1}"
qemu_virtio_blk="${QEMU_VIRTIO_BLK:-1}"
qemu_console_log="${QEMU_CONSOLE_LOG:-/tmp/edgerun-qemu-virtio-console.log}"
qemu_disk_img="${QEMU_DISK_IMG:-/tmp/edgerun-qemu-virtio-blk.img}"
qemu_disk_size="${QEMU_DISK_SIZE:-64M}"
qemu_disk_layout="${QEMU_DISK_LAYOUT:-empty}"
qemu_http_smoke="${QEMU_HTTP_SMOKE:-0}"
qemu_oci_pull_smoke="${QEMU_OCI_PULL_SMOKE:-0}"
qemu_boot_config="${QEMU_BOOT_CONFIG:-image=registry.local/edge/test:latest
edgefs=format-data-partition
rootfs_path=/apps/test
}"
if [[ "$qemu_http_smoke" != "0" ]]; then
    qemu_boot_config="${qemu_boot_config}http_smoke=http://10.0.2.2:${QEMU_HTTP_SMOKE_PORT:-18080}/edgerun-smoke
"
fi
if [[ "$qemu_oci_pull_smoke" != "0" ]]; then
    qemu_boot_config="${qemu_boot_config}image=10.0.2.2:${QEMU_OCI_PULL_SMOKE_PORT:-18080}/edge/test:latest
oci_pull=true
registry_insecure_http=true
"
fi

create_qemu_disk_image() {
    local image="$1"
    local size="$2"
    local layout="$3"

    truncate -s "$size" "$image"
    case "$layout" in
        empty)
            ;;
        mbr-blank)
            printf '\x00\x00\x00\x00\x0c\x00\x00\x00\x00\x08\x00\x00\x00\xf8\x01\x00' |
                dd of="$image" bs=1 seek=446 conv=notrunc status=none
            printf '\x55\xaa' | dd of="$image" bs=1 seek=510 conv=notrunc status=none
            ;;
        fat-boot)
            printf '\x00\x00\x00\x00\x0c\x00\x00\x00\x00\x08\x00\x00\x00\xf8\x01\x00' |
                dd of="$image" bs=1 seek=446 conv=notrunc status=none
            printf '\x55\xaa' | dd of="$image" bs=1 seek=510 conv=notrunc status=none
            mkfs.fat --invariant --offset=2048 -F 16 -r 16 -n EDGERUN "$image" 64512
            local cfg_dir
            cfg_dir="$(mktemp -d)"
            printf '%s' "$qemu_boot_config" >"$cfg_dir/boot.cfg"
            mmd -i "$image@@1048576" ::/edgerun
            mcopy -i "$image@@1048576" "$cfg_dir/boot.cfg" ::/edgerun/
            rm -f "$cfg_dir/boot.cfg"
            rmdir "$cfg_dir"
            ;;
        fat-edgefs-boot)
            printf '\x00\x00\x00\x00\x0c\x00\x00\x00\x00\x08\x00\x00\x00\xfc\x00\x00' |
                dd of="$image" bs=1 seek=446 conv=notrunc status=none
            printf '\x00\x00\x00\x00\x83\x00\x00\x00\x00\x08\x01\x00\x00\xf0\x00\x00' |
                dd of="$image" bs=1 seek=462 conv=notrunc status=none
            printf '\x55\xaa' | dd of="$image" bs=1 seek=510 conv=notrunc status=none
            mkfs.fat --invariant --offset=2048 -F 16 -r 16 -n EDGERUN "$image" 64512
            local cfg_dir
            cfg_dir="$(mktemp -d)"
            printf '%s' "$qemu_boot_config" >"$cfg_dir/boot.cfg"
            mmd -i "$image@@1048576" ::/edgerun
            mcopy -i "$image@@1048576" "$cfg_dir/boot.cfg" ::/edgerun/
            rm -f "$cfg_dir/boot.cfg"
            rmdir "$cfg_dir"
            ;;
        *)
            echo "Unsupported QEMU_DISK_LAYOUT: $layout" >&2
            exit 2
            ;;
    esac
}

cargo +nightly build --release -p edgerun-unikernel \
    --target "$target" \
    -Zbuild-std=core,alloc

/usr/bin/objcopy -O binary "$kernel_elf" "$kernel_bin"

qemu_extra_args=()
if [[ -n "$qemu_net_dump" ]]; then
    rm -f "$qemu_net_dump"
    qemu_extra_args+=(-object "filter-dump,id=edgerun-net-dump,netdev=n0,file=$qemu_net_dump")
fi

if [[ -n "$qemu_tpm_socket" ]]; then
    qemu_extra_args+=(
        -chardev "socket,id=chrtpm,path=$qemu_tpm_socket"
        -tpmdev "emulator,id=tpm0,chardev=chrtpm"
        -device "$qemu_tpm_device,tpmdev=tpm0"
    )
fi

if [[ "$qemu_virtio_rng" != "0" ]]; then
    qemu_extra_args+=(
        -object "rng-random,id=edgerun-rng,filename=/dev/urandom"
        -device "virtio-rng-pci,disable-legacy=on,disable-modern=off,rng=edgerun-rng"
    )
fi

if [[ "$qemu_virtio_console" != "0" ]]; then
    rm -f "$qemu_console_log"
    qemu_extra_args+=(
        -device "virtio-serial-pci,disable-legacy=on,disable-modern=off"
        -chardev "file,id=edgerun-console,path=$qemu_console_log"
        -device "virtconsole,chardev=edgerun-console"
    )
fi

if [[ "$qemu_virtio_blk" != "0" ]]; then
    if [[ ! -e "$qemu_disk_img" ]]; then
        create_qemu_disk_image "$qemu_disk_img" "$qemu_disk_size" "$qemu_disk_layout"
    fi
    qemu_extra_args+=(
        -drive "if=none,id=edgerun-blk,file=$qemu_disk_img,format=raw"
        -device "virtio-blk-pci,disable-legacy=on,disable-modern=off,drive=edgerun-blk"
    )
fi

as --32 -o "$boot_obj" "$repo_root/crates/edgerun-unikernel/qemu_boot.S"
ld -m elf_i386 -Ttext 0x7c00 --oformat binary -o "$boot_bin" "$boot_obj"
dd if="$boot_bin" of="$boot_sector" bs=512 count=1 status=none

set +e
timeout "$timeout_seconds" qemu-system-x86_64 \
    -machine "${QEMU_MACHINE:-pc}" \
    -m "${QEMU_MEMORY:-128M}" \
    -nographic \
    -serial mon:stdio \
    -no-reboot \
    -no-shutdown \
    -drive "file=$boot_sector,format=raw,if=floppy" \
    -boot a \
    -device "loader,file=$kernel_bin,addr=0x100000,force-raw=on" \
    -netdev "$qemu_netdev" \
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
