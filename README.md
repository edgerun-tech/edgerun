# Edgerun Core

Edgerun Core is an identity-based, append-only information fabric for edge
nodes. The internal wire protocol is consolidated on rkyv: hashing, signing,
storage, transport payloads, caches, and local bridges must archive concrete
protocol types through the rkyv boundary. Legacy schema/generated wire artifacts
have been removed so old call sites fail instead of silently using a second
protocol.

## Protocol Model

The v0 protocol has five central rules:

1. A stream has exactly one writer for its lifetime.
2. Events are immutable, strictly ordered by `seq`, hash-linked by
   `prev_event_hash`, and signed by the stream writer.
3. Commands are signed external requests; they become authoritative only when
   the target node validates them and records a committed or rejected event in
   its own stream.
4. Objects are immutable logical data. Stored representations may be encrypted,
   chunked, compressed, or replicated, but object identity is independent of the
   storage packaging.
5. Trust is explicit and local: controller sets, delegation chains, revocations,
   assurance claims, query proofs, and route hints are validated under local
   policy rather than global consensus.

## Implementation Map

| Layer | Main crates | What the code actually does |
|---|---|---|
| Protocol core | `edgerun-core`, `edgerun-wire` | Owns native protocol records, rkyv wire boundary exports, domain-separated hashes/signature inputs, command/stream/delegation/snapshot/query/network/identity/proof/trust validation, and explicit breakpoints for removed legacy byte paths. |
| Streams | `edgerun-stream` | Creates signed genesis events, appends contiguous events, verifies stream-chain invariants, and rejects missing genesis, sequence gaps, bad `prev_hash`, and tampering. |
| Storage | `edgerun-storage` | Persists event logs as authority, stores encrypted blobs, derives logical object and representation ids, tracks stream heads/replay/object/snapshot indexes, rebuilds indexes from logs, records command replay outcomes, and supports file, memory, and block-backed storage paths. |
| Node runtime | `edgerun-node` | Owns node orchestration. Remaining legacy node byte paths are explicit breakpoints until migrated to rkyv archive payloads. |
| Mesh | `edgerun-mesh`, `edgerun-mesh-link`, `edgerun-mesh-session`, `edgerun-mesh-daemon`, `edgerun-mesh-capability` | Uses P-256 public keys as `NodeID`s, signs mesh frames, routes by identity, discovers peers, performs ECDH session handshakes, and carries rkyv capability envelopes. |
| Capabilities | `edgerun-capabilities`, `edgerun-capability-policy`, `edgerun-remote-capability` | Defines provider descriptors, selectors, requests, grants, invocations, results, revocations, policy decisions, session grant binding, and rkyv capability message boundaries. |
| Hardware identity | `edgerun-hardware-signing`, `edgerun-tpm`, `edgerun-yubikey`, `edgerun-android-keystore` | Normalizes hardware-backed signing around ECDSA P-256 `NodeID`s. |
| Services | `edgerun-server`, `edgerun-http`, `edgerun-tls`, `edgerun-quic`, `edgerun-dns`, `edgerun-dhcp`, `edgerun-dhcpv6`, `edgerun-email`, `edgerun-proxy`, `edgerun-oci` | Implements service stacks on top of `edgerun-rt`. |
| Bare metal | `edgerun-rt`, `edgerun-platform`, `edgerun-unikernel`, `edgerun-ipxe`, `edgerun-tftp`, `edgerun-virtio`, `edgerun-rtl8125` | Provides runtime, platform primitives, drivers, boot support, and a freestanding unikernel binary. |
| Local support crates | `edgerun-json`, `edgerun-encoding`, `edgerun-hpack`, `edgerun-qpack`, `edgerun-crypto`, `edgerun-clap`, `edgerun-log`, `edgerun-vfs`, `edgerun-virtual-disk` | Provides local utilities. These crates are not alternate protocol wire formats. |

## Wire Protocol Rule

There is one internal wire protocol: `rkyv`.

Removed legacy paths must not be reintroduced as compatibility shims. If a
caller breaks, migrate the caller to archive and access the concrete rkyv type at
that boundary.

## Build

Hosted checks for a specific crate:

```bash
cargo check -p edgerun-core
cargo test -p edgerun-core
cargo test -p edgerun-stream
cargo test -p edgerun-storage
```

Workspace inventory:

```bash
cargo metadata --no-deps --format-version 1
```

The unikernel path requires nightly and `build-std`:

```bash
cargo +nightly build --release -p edgerun-unikernel \
  --target x86_64-unknown-none \
  -Zbuild-std=core,alloc
```

## License and Release Status

Edgerun Core is licensed under `MIT OR Apache-2.0`. See [LICENSE](LICENSE),
[LICENSE-MIT](LICENSE-MIT), and [LICENSE-APACHE](LICENSE-APACHE).
