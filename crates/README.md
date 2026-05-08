# Crate Layout

The crate tree is organized around the v0 protocol: single-writer streams,
immutable objects, signed commands, explicit capabilities/delegation, query
access, and local trust policy.

The internal wire protocol is rkyv only. Utility crates may implement external
standards such as HTTP, DNS, TLS, QUIC, HPACK, QPACK, DHCP, NFC, JSON, or TLV,
but those are not Edgerun internal wire protocols.

## Protocol Fabric

- `edgerun-wire`: rkyv-only internal wire boundary.
- `edgerun-core`: native protocol records, domain-separated hashes/signatures,
  and validators.
- `edgerun-stream`: single-writer signed event streams.
- `edgerun-storage`: event log, encrypted blobs, file/block/memory stores,
  indexes, snapshots, replay cache, rebuild/integrity logic.
- `edgerun-node`: the runtime/node boundary and resource authority. Apps and
  deployment specs can request protocol bindings, routes, storage, signing, and
  hardware capabilities, but only the node decides whether a request becomes a
  native socket, browser message route, mesh route, filesystem path, hardware
  handle, or no resource on the current host.
- `edgerun-rt`: low-level no_std executor, sync, time, and transport
  primitives used by the node. Its socket bind/listen APIs are implementation
  primitives, not app-facing authority.
- `edgerun-protocols`: transport-independent external protocol bytes and state
  machines. Protocols do not own ports, sockets, filesystems, trust roots, or
  host policy.

The remaining protocol-named crates such as `edgerun-node http`, `edgerun-email`,
and `edgerun-tls` are transitional runtime adapters around
`edgerun-protocols` and node-owned resources. New protocol logic should go into
`edgerun-protocols`; new resource realization should go into `edgerun-node`.
