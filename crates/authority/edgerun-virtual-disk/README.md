# edgerun-virtual-disk

Utilities for two related jobs:

- creating and managing virtual disk image files
- adapting block-device semantics to stream/session handlers

## Disk image support

Supported formats:

- `Raw` (`.raw`)
- `Qcow2` (`.qcow2`)
- `Vhd` (`.vhd` via qemu-img `vpc` format)
- `Vhdx` (`.vhdx`)

`Qcow2`/`Vhd`/`Vhdx` operations require `qemu-img` to be installed.

## Remote block support

The crate now includes a first-pass remote block bridge with:

- `BlockBackend` protocol traits and request/response types from `edgerun-protocols`
- `MemoryBlockBackend` and `FileBlockBackend`
- `BlockClient` and `BlockServer` for framed master/slave exchange
- `serve_nbd_connection(...)` / `serve_nbd_connection_multi(...)` for NBD sessions
- `attach_nbd(...)` / `detach_nbd(...)` for node-owned Linux `/dev/nbdX`
  attachment flows, implemented directly in Rust without requiring `nbd-client`

Native listener ownership belongs in `edgerun-node`. This crate handles
backends and already-accepted streams; node decides whether a stream came from
TCP, Unix sockets, mesh, browser IPC, email, or another route.

## Example

```rust
use edgerun_virtual_disk::{create, VirtualDiskFormat, VirtualDiskSpec};

let spec = VirtualDiskSpec {
    path: "/var/data/disk.qcow2".into(),
    size_bytes: 2 * 1024 * 1024 * 1024,
    format: VirtualDiskFormat::Qcow2,
    sparse: true,
};
create(&spec)?;
# Ok::<(), edgerun_virtual_disk::VirtualDiskError>(())
```

```rust
use edgerun_protocols::block::{BlockDeviceInfo, BlockError, BlockRequest, BlockResponse, handle_request};
use edgerun_virtual_disk::MemoryBlockBackend;

let backend = MemoryBlockBackend::new(BlockDeviceInfo {
    block_size: 512,
    block_count: 8,
    readonly: false,
    supports_flush: true,
    supports_discard: true,
    supports_write_zeroes: true,
    model: "edgerun-memory".into(),
    serial: "memory-001".into(),
})?;

let response = handle_request(&backend, BlockRequest::GetInfo);
match response {
    BlockResponse::Info(info) => assert_eq!(info.block_count, 8),
    other => panic!("unexpected response: {other:?}"),
}
# Ok::<(), BlockError>(())
```
