# lifegraph-virtual-disk

Utilities for two related jobs:

- creating and managing virtual disk image files
- serving block-device semantics over a simple master/slave wire protocol

## Disk image support

Supported formats:

- `Raw` (`.raw`)
- `Qcow2` (`.qcow2`)
- `Vhd` (`.vhd` via qemu-img `vpc` format)
- `Vhdx` (`.vhdx`)

`Qcow2`/`Vhd`/`Vhdx` operations require `qemu-img` to be installed.

## Remote block support

The crate now includes a first-pass remote block bridge with:

- `BlockBackend` for storage backends
- `MemoryBlockBackend` and `FileBlockBackend`
- `BlockRequest` / `BlockResponse` wire types
- `BlockClient` and `BlockServer` for framed master/slave exchange
- `UnixBlockServer` and `BlockClient::connect_unix(...)` for easy same-machine testing
- `TcpBlockServer` and `BlockClient::connect_tcp(...)` for network testing
- `TcpNbdServer` / `MultiExportTcpNbdServer` for exporting one or more backends as NBD over TCP
- `attach_nbd(...)` / `detach_nbd(...)` plus an `nbd-attach` helper for Linux `/dev/nbdX`
  implemented directly in Rust, without requiring `nbd-client`

This is designed so the same block backend can later be surfaced as NBD, USB mass storage,
NVMe, or another frontend.

## Local Unix-socket testing

Demo binaries are included for manual testing.

Unix socket:

```bash
cargo run -p lifegraph-virtual-disk --bin block-server -- unix /tmp/lifegraph-block.sock mem 512 128
```

In another terminal:

```bash
cargo run -p lifegraph-virtual-disk --bin block-client -- unix /tmp/lifegraph-block.sock info
cargo run -p lifegraph-virtual-disk --bin block-client -- unix /tmp/lifegraph-block.sock write 0 1 "$(printf 'ab%.0s' {1..512})"
cargo run -p lifegraph-virtual-disk --bin block-client -- unix /tmp/lifegraph-block.sock read 0 1
```

TCP:

```bash
cargo run -p lifegraph-virtual-disk --bin block-server -- tcp 127.0.0.1:9000 mem 512 128
cargo run -p lifegraph-virtual-disk --bin block-client -- tcp 127.0.0.1:9000 info
```

File-backed disk:

```bash
cargo run -p lifegraph-virtual-disk --bin block-server -- unix /tmp/lifegraph-block.sock file /path/to/disk.raw 512
```

Minimal NBD export over TCP:

```bash
cargo run -p lifegraph-virtual-disk --bin nbd-server -- 127.0.0.1:10809 lifegraph mem 512 128
```

Multiple named NBD exports:

```bash
cargo run -p lifegraph-virtual-disk --bin nbd-server -- 127.0.0.1:10809 multi-mem alpha 512 128 beta 512 256
```

If your client supports listing exports, the server now responds to NBD export-list requests before export selection.

Linux attach helper:

```bash
cargo run -p lifegraph-virtual-disk --bin nbd-attach -- attach 127.0.0.1 10809 lifegraph /dev/nbd0
cargo run -p lifegraph-virtual-disk --bin nbd-attach -- detach /dev/nbd0
```

One-shot serve-and-attach helper:

```bash
cargo run -p lifegraph-virtual-disk --bin nbd-quick-attach -- 127.0.0.1 10809 lifegraph /dev/nbd0 mem 512 128
```

## Example

```rust
use lifegraph_virtual_disk::{create, VirtualDiskFormat, VirtualDiskSpec};

let spec = VirtualDiskSpec {
    path: "/var/data/disk.qcow2".into(),
    size_bytes: 2 * 1024 * 1024 * 1024,
    format: VirtualDiskFormat::Qcow2,
    sparse: true,
};
create(&spec)?;
# Ok::<(), lifegraph_virtual_disk::VirtualDiskError>(())
```

```rust
use lifegraph_virtual_disk::{BlockDeviceInfo, BlockRequest, MemoryBlockBackend, handle_request};

let backend = MemoryBlockBackend::new(BlockDeviceInfo {
    block_size: 512,
    block_count: 8,
    readonly: false,
    supports_flush: true,
    supports_discard: true,
    supports_write_zeroes: true,
    model: "lifegraph-demo".into(),
    serial: "demo-001".into(),
})?;

let response = handle_request(&backend, BlockRequest::GetInfo);
match response {
    lifegraph_virtual_disk::BlockResponse::Info(info) => assert_eq!(info.block_count, 8),
    other => panic!("unexpected response: {other:?}"),
}
# Ok::<(), lifegraph_virtual_disk::BlockError>(())
```
