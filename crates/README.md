# Crate Layout

The crate tree is converging around admitted work: signed intent, admission,
routes, capability packets, proofs, receipts, and settlement. Single-writer
streams, immutable objects, signed commands, query access, and local trust
policy are local evidence or migration surfaces unless they are bound to
admitted work.

The internal wire protocol is rkyv only. Utility crates may implement external
standards such as HTTP, DNS, TLS, QUIC, HPACK, QPACK, DHCP, NFC, JSON, or TLV,
but those are not Edgerun internal wire protocols.

## Protocol Fabric

- `edgerun-wire`: rkyv-only internal wire boundary.
- `edgerun-work`: canonical authority for work requests, admissions, routes,
  channels, capability packets, proofs, receipts, and settlement.
- `edgerun-core`: legacy native protocol records and validators kept while
  useful pieces are migrated into wire/work/storage/domain crates. Do not add
  new authority semantics here.
- `edgerun-storage`: event log, encrypted blobs, file/block/memory stores,
  indexes, snapshots, replay cache, rebuild/integrity logic. Logs are audit and
  projection data unless they record admitted work evidence.
- `edgerun-node`: the runtime/node boundary and resource adapter. Apps and
  deployment specs can request protocol bindings, routes, storage, signing, and
  hardware capabilities, but admission/work policy decides whether a request may
  cross node, relay, capability, storage, or settlement boundaries.
- `edgerun-rt`: low-level no_std executor, sync, time, and transport
  primitives used by the node. Its socket bind/listen APIs are implementation
  primitives, not app-facing authority.
- `edgerun-protocols`: transport-independent external protocol bytes and state
  machines. Protocols do not own ports, sockets, filesystems, trust roots, or
  host policy.

The remaining protocol-named crates such as `edgerun-node http`, `edgerun-email`,
and `edgerun-tls` are transitional runtime adapters around
`edgerun-protocols` and node-owned resources. New protocol parsing/state-machine
logic should go into `edgerun-protocols`; new resource realization should go
into `edgerun-node`; new cross-node authority should go into `edgerun-work`.

## Shared UI

The shared UI kit is `utility/edgerun-ui-core`. It builds Rust-owned `GpuScene`
command buffers for browser, SDL, and native hosts. Start with:

```bash
cargo test -p edgerun-ui-core
cargo test -p edgerun-ui-core --no-default-features
cargo check -p edgerun-ui-core --features std
cargo check -p edgerun-ui-core --features fontdue-text
```

Detailed feature, preview, generated-asset, and host-boundary notes live in
`utility/edgerun-ui-core/README.md` and
`utility/edgerun-ui-core/UI_ARCHITECTURE.md`.
