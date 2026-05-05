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
  validators, and explicit breakpoints for removed legacy byte paths.
- `edgerun-stream`: single-writer signed event streams.
- `edgerun-storage`: event log, encrypted blobs, file/block/memory stores,
  indexes, snapshots, replay cache, rebuild/integrity logic.
- `edgerun-node`: node-level command/query/store task orchestration, with old
  byte paths removed until migrated to rkyv.
