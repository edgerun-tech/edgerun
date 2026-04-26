# Edgerun Unikernel Build Instructions

## Building

```bash
cd /home/ken/edgerun_core
cargo +nightly build --release -p edgerun-unikernel --target x86_64-unknown-none -Zbuild-std=core,alloc
/usr/bin/objcopy -O binary target/x86_64-unknown-none/release/edgerun-unikernel /tmp/edgerun.bin
```

Output: `/tmp/edgerun.bin` (~8KB)

**Note**: Requires nightly Rust with `-Zbuild-std=core,alloc` for no_std alloc support.

## Boot Flow

1. **iPXE** (pre-installed on MSI MEG X570 Unite NIC)chain loads `edgerun.bin` to 0x100000
2. **`_start`**: Sets up 16KB stack, zeros BSS
3. **`main()`**: Initialize NIC, DHCP, TFTP, boot kernel

## Hardware

- NIC: Realtek RTL8125 (10ec:8125) on PCIe
- Boot: iPXE via PXE (Intel I211 used for management)

## Crates

- `edgerun-bare-rt`: no_std async runtime
- `edgerun-unikernel`: bare-metal binary entry
- `edgerun-virtio`: virtio-net driver
- `edgerun-platform`: bare-metal platform primitives