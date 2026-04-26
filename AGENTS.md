# Edgerun Unikernel Build Instructions

## Building

```bash
cd /home/ken/edgerun_core
cargo +nightly build --release -p edgerun-unikernel --target x86_64-unknown-none -Zbuild-std=core,alloc
/usr/bin/objcopy -O binary target/x86_64-unknown-none/release/edgerun-unikernel /tmp/edgerun.bin
```

Output: `/tmp/edgerun.bin` (~15KB)

**Note**: Requires nightly Rust with `-Zbuild-std=core,alloc` for no_std alloc support.

## Boot Flow

1. **iPXE** (pre-installed on MSI MEG X570 Unite NIC) chain loads `edgerun.bin` to 0x100000
2. **`_start`**: Sets up 16KB stack, zeros BSS
3. **`main()`**: Initialize NIC, DHCP, TFTP, boot kernel

## Hardware

- NIC: Realtek RTL8125 (10ec:8125) on PCIe
- Boot: iPXE via PXE (Intel I211 used for management)

## no_std Crates

| Crate | Purpose |
|------|---------|
| edgerun-bare-rt | Bare-metal async runtime |
| edgerun-platform | CPU, timer, IRQ, TLS |
| edgerun-virtio | Virtio-net driver |
| edgerun-glob | Glob patterns |
| edgerun-regex | Regex |
| edgerun-error | Error derive macro |
| edgerun-ipxe | iPXE wrapper |
| edgerun-tftp | TFTP client |
| edgerun-clap-derive | CLI derive |
| edgerun-log | Logging |
| edgerun-json | JSON (alloc feature) |

## Build Multiple Crates

```bash
# Test all no_std crates for bare target
for crate in edgerun-glob edgerun-regex edgerun-error edgerun-ipxe edgerun-tftp edgerun-clap-derive edgerun-log edgerun-json edgerun-bare-rt edgerun-platform edgerun-virtio; do
  cargo +nightly build -p "$crate" --release --target x86_64-unknown-none -Zbuild-std=core,alloc
done
```

## blocked Crates

These depend on `std::collections` or external crates like `log`, `serde`, `bytes`:

- edgerun-encoding
- edgerun-hpack  
- edgerun-qpack
- edgerun-http
- edgerun-quic
- Most other crates