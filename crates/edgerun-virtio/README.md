# edgerun-virtio

`edgerun-virtio` is the bare-target VirtIO driver crate used by the EdgeRun
unikernel path. It intentionally keeps a small, explicit surface: discover a
modern VirtIO PCI/MMIO device, initialize one driver instance for each supported
device class, and perform synchronous split-virtqueue I/O.

## Support Matrix

| Device | Current support | Tested path | Missing before production use |
| --- | --- | --- | --- |
| Network | Modern PCI and MMIO discovery, feature negotiation for `VERSION_1`, optional MAC/status config, split RX/TX queues, RX refill, TX completion reap, typed send/recv errors | Unit tests, QEMU boot DHCP, QEMU socket net-pump ARP/ICMP | Interrupt-driven polling, larger/dynamic queues, checksum/GSO feature policy, multiple NIC instances |
| Block | Modern PCI and MMIO discovery, read/write sectors, multi-sector helpers, flush request when negotiated, read-only handling, typed request/completion errors | Unit tests, QEMU boot sector reads, storage adapter read | Larger request batching, non-512-byte logical block support, durable write/flush integration tests |
| RNG | Modern PCI and MMIO discovery, entropy fill over split queue, typed timeout/completion errors | Unit tests, QEMU entropy mixing marker | Interrupt-driven completion, multiple RNG instances |
| Console | Modern PCI and MMIO discovery, console write, RX queue, typed read/write errors | Unit tests, QEMU console init marker and console backing-file check | Control queues, resize/config events, interrupt-driven RX |

## API Boundary

Prefer the typed APIs for new code:

- `try_find_virtio_*()` / `try_find_initialized_virtio_*()`
- `try_from_mmio_base()` / `VirtioDeviceInfo::try_open_*()`
- `try_init()`
- `try_send()` / `try_recv()`
- `try_read_sector()` / `try_write_sector()` / `try_flush()`
- `try_fill_bytes()`
- `try_write_all()`

The older `bool`/`Option` discovery and I/O methods remain compatibility
wrappers around the typed APIs. They should not be used in new code where the
caller can surface or recover from the specific `VirtioError`.

## Ownership Model

The current bare driver owns one static queue/buffer set per device class and
guards it with a claim flag. This is acceptable for early boot and the current
unikernel smoke path, but it is not the final multi-device model. Production
work should move queue memory into explicit per-device storage so multiple
devices of the same class can be initialized safely.

## Verification

Narrow checks while editing this crate:

```bash
cargo test --manifest-path crates/edgerun-virtio/Cargo.toml --target x86_64-unknown-linux-gnu
cargo +nightly check --manifest-path crates/edgerun-virtio/Cargo.toml --target x86_64-unknown-none -Zbuild-std=core,alloc
cargo check --manifest-path crates/edgerun-virtio/Cargo.toml --target wasm32-unknown-unknown
scripts/qemu-virtio-smoke.sh
scripts/qemu-unikernel-net-pump.sh
```
