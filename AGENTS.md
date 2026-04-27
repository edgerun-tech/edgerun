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

Build tested with `x86_64-unknown-none` + `-Zbuild-std=core,alloc`:

| Crate | Purpose |
|------|---------|
| edgerun-bare-rt | Bare-metal async runtime |
| edgerun-platform | CPU, timer, IRQ, TLS |
| edgerun-virtio | Virtio-net driver |
| edgerun-glob | Glob patterns (no_std, zero-dep) |
| edgerun-regex | Regex (no_std, zero-dep) |
| edgerun-error | Error derive macro (proc-macro) |
| edgerun-ipxe | iPXE wrapper |
| edgerun-tftp | TFTP client |
| edgerun-log | Minimal logging (no_std, zero-dep) |
| edgerun-hpack | HPACK header compression (RFC 7541) |
| edgerun-encoding | Encoding utilities (base64, varint, buf) |
| edgerun-qpack | QPACK header compression (RFC 9204) |
| edgerun-json | JSON (alloc feature) |

## Build Test Command

```bash
# Test all no_std crates for bare target
for crate in edgerun-glob edgerun-regex edgerun-error edgerun-ipxe edgerun-tftp edgerun-log edgerun-json edgerun-bare-rt edgerun-platform edgerun-virtio edgerun-hpack edgerun-encoding edgerun-qpack; do
  cargo +nightly build -p "$crate" --release --target x86_64-unknown-none -Zbuild-std=core,alloc
done
```

## Blocked Crates

These need work to become no_std:

- **edgerun-http**: Heavy async rt dependencies (edgerun-rt), TLS, DNS
- **edgerun-quic**: Depends on http + rt + tls + crypto
- **edgerun-tls**: Depends on edgerun-rt + libc
- **edgerun-crypto**: External crates, needs getrandom with rdrand feature
- **edgerun-dns**: Depends on edgerun-rt
- **edgerun-net**: Depends on getrandom

## Bare Metal RNG

For x86_64 bare metal, use getrandom with rdrand feature:

```toml
# In your binary crate (not lib)
getrandom = { version = "0.2", features = ["rdrand"] }
```

This enables RDRAND instruction for RNG on x86/x86_64 targets.

## Crate Status Notes

### edgerun-encoding (FIXED)
- Already uses `core::error::Error` ✅
- Has no_std `Buf`/`BufMut` traits with working `Cursor<T: AsRef<[u8]>>`
- Removed optional std-only impls (`std::io::Cursor`) - available in std build via standard library
- `std::time::SystemTime::now()` stubbed for no_std (returns epoch 0)

### edgerun-hpack (FIXED)
- Replaced `std::collections::VecDeque` → `alloc::collections::VecDeque`
- Replaced `std::collections::HashMap` → linear table lookup
- Created custom `Writer` trait (no_std `io::Write`)
- Uses `edgerun-log` for logging (already no_std)
- Uses `core::error::Error` (stable in no_std)

### edgerun-http (BLOCKED)
- Heavily coupled to edgerun-rt (async networking)
- Would need splitting: protocol parsing core → no_std, server wrapper → rt
- Protocol parts (frames, HPACK) are separate from I/O

### edgerun-rt vs edgerun-bare-rt
- **edgerun-rt**: Full async runtime (epoll, sockets, threads) - requires std
- **edgerun-bare-rt**: Bare-metal async (no std) - different API
- Network crates use rt; bare-metal crates use bare-rt