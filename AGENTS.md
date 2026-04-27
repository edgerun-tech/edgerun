# Edgerun Core Agent Notes

Start from the protocol, not from the crate list. The repository implements the
v0 edgerun model from `edgerun_core_protocol_v0_single_file.md`: single-writer
streams, signed events, immutable objects, commands that become authoritative
only after target-node commitment, explicit capabilities/delegation, query-based
access, and identity-routed networking.

## Current Workspace Health

`cargo metadata --no-deps --format-version 1` succeeds in this checkout and
reports 110 workspace packages/members. The root manifest's textual `members`
array still contains a duplicate `crates/edgerun-tftp` entry, and a few crate
directories are reached through path/workspace resolution rather than being
listed directly in that array.

Useful inventory commands:

```bash
find crates -mindepth 2 -maxdepth 2 -name Cargo.toml | sort
awk '/^members = \[/{flag=1;next}/^\]/{if(flag){flag=0}}flag{print}' Cargo.toml
```

## What To Read First

1. `edgerun_core_protocol_v0_single_file.md`
2. `proto/edgerun/v0/{common,identity,trust,stream,object,access,network}.proto`
3. `crates/edgerun-core/src/{protocol.rs,crypto.rs,command.rs,validators/*.rs}`
4. `crates/edgerun-stream/src/lib.rs`
5. `crates/edgerun-storage/src/{lib.rs,core,store.rs,fs,event_log.rs,file_index.rs,blobs.rs}`
6. `crates/edgerun-node/src/{lib.rs,store_task.rs,command_dispatch.rs,query_engine.rs,tcp_server.rs,daemon.rs}`
7. Mesh and capability crates: `edgerun-mesh*`, `edgerun-remote-capability`,
   `edgerun-capabilities`, `edgerun-capability-policy`
8. Hardware signing and providers: `edgerun-hardware-signing`, `edgerun-tpm`,
   `edgerun-yubikey`, Linux/sysfs/ALSA/evdev/V4L2/Goodix/Bluetooth adapters

## Protocol Invariants To Preserve

- Never treat command delivery as authority. A command matters only after the
  target node validates it and records a `COMMAND_COMMITTED` or
  `COMMAND_REJECTED` event in its stream.
- Never mutate authoritative state outside the event log. Indexes, snapshots,
  route hints, query results, and caches are derived.
- Stream append must be contiguous by `seq`, hash-linked by previous event
  hash, and signed by the fixed writer identity.
- Object identity and stored representation identity are different.
- Delegation must attenuate: child delegations cannot expand parent actions,
  scope, timing, assurance, or constraints.
- Route and query artifacts can be advisory; accepting them does not install
  trust roots, controller authority, or stream authority.

## Targeted Checks

Prefer package-level checks while editing a specific area:

```bash
cargo test -p edgerun-core
cargo test -p edgerun-stream
cargo test -p edgerun-storage
cargo test -p edgerun-node
```

Bare/unikernel build:

```bash
cargo +nightly build --release -p edgerun-unikernel \
  --target x86_64-unknown-none \
  -Zbuild-std=core,alloc
```

QEMU helpers:

```bash
scripts/qemu-unikernel.sh
scripts/qemu-unikernel-net-pump.sh
scripts/qemu-unikernel-swtpm.sh
```

## Major Code Areas

- Protocol validation: `edgerun-core`
- Signed stream production/verification: `edgerun-stream`
- Durable event/object storage: `edgerun-storage`
- Node command/query/mesh orchestration: `edgerun-node`
- Identity-routed mesh transport: `edgerun-mesh`, `edgerun-mesh-link`,
  `edgerun-mesh-session`, `edgerun-mesh-daemon`
- Remote hardware capability protocol: `edgerun-remote-capability`,
  `edgerun-mesh-capability`
- Secure identity/signing: `edgerun-hardware-signing`, `edgerun-tpm`,
  `edgerun-yubikey`, `edgerun-android-keystore`
- Service protocols: `edgerun-http`, `edgerun-tls`, `edgerun-quic`,
  `edgerun-dns`, `edgerun-dhcp`, `edgerun-dhcpv6`, `edgerun-email`,
  `edgerun-server`, `edgerun-net`, `edgerun-proxy`
- Runtime and bare metal: `edgerun-rt`, `edgerun-platform`,
  `edgerun-unikernel`, `edgerun-virtio`, `edgerun-rtl8125`, `edgerun-ipxe`,
  `edgerun-tftp`

## Documentation Rule

When updating docs, state whether a feature is:

- implemented in code,
- a protocol/design requirement,
- generated type/catalog material,
- host-only,
- bare-target/stubbed,
- or currently blocked by missing implementation/runtime support.
