# Edgerun Core

Edgerun Core is an identity-based, append-only information fabric for edge
nodes. A node is addressed by its signing identity, writes authoritative facts to
its own signed stream, and derives local indexes, service views, routes, and
capability decisions from durable event logs.

The internal Edgerun wire protocol is `rkyv`. Hashing, signing, storage,
transport payloads, caches, and local bridge payloads archive concrete protocol
types through the rkyv boundary. Legacy schema and generated wire artifacts have
been removed so stale callers fail loudly instead of silently using a second
protocol path.

## Protocol Model

The v0 protocol has five central rules that define the system end to end:

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

The authority flow is:

```text
hardware or software signer
  -> protocol signature input
  -> command or event
  -> validated stream append
  -> durable event log
  -> derived indexes, caches, snapshots, routes, and service views
```

Only the signed stream and immutable objects are authoritative. Indexes,
snapshots, replay caches, query results, route hints, dashboards, health checks,
and service projections are derived state; they must be rebuildable from logs or
rejectable under local policy.

## Wire Boundary

There is one internal wire protocol: `rkyv`.

Removed legacy paths must not be reintroduced as compatibility shims. If a
caller breaks, migrate that caller to archive and access the concrete rkyv type
at that boundary.

External standards keep their standards-defined encodings. HTTP, DNS, TLS,
QUIC, HPACK, QPACK, DHCP, NFC, OCI, TPM, and device protocols are external
protocol domains, not alternate Edgerun internal wire formats.

## Runtime Map

For a fuller architecture and consolidation map, see `docs/architecture.md`.

| Layer | Main crates | What the code actually does |
|---|---|---|
| Protocol core | `edgerun-core`, `edgerun-wire` | Owns native protocol records, rkyv wire boundary exports, domain-separated hashes/signature inputs, and command/stream/delegation/snapshot/query/network/identity/proof/trust validation. |
| Streams | `edgerun-stream` | Creates signed genesis events, appends contiguous events, verifies stream-chain invariants, and rejects missing genesis, sequence gaps, bad `prev_hash`, and tampering. |
| Storage | `edgerun-storage` | Persists event logs as authority, stores encrypted blobs, derives logical object and representation ids, tracks stream heads/replay/object/snapshot indexes, rebuilds indexes from logs, records command replay outcomes, and supports file, memory, and block-backed storage paths. |
| Node runtime | `edgerun-node` | Owns node initialization, provisioning, status inspection, command validation/dispatch components, hardware discovery, and stream append integration. |
| Mesh | `edgerun-mesh`, `edgerun-mesh-link`, `edgerun-mesh-session`, `edgerun-mesh-capability` | Uses P-256 public keys as `NodeID`s, signs mesh frames, routes by identity, discovers peers, performs ECDH session handshakes, and carries rkyv capability envelopes. |
| Capabilities | `edgerun-capabilities`, `edgerun-capability-policy`, `edgerun-remote-capability` | Defines provider descriptors, selectors, requests, grants, invocations, results, revocations, policy decisions, session grant binding, and rkyv capability message boundaries. |
| Hardware identity | `edgerun-hardware-signing`, `edgerun-tpm`, `edgerun-yubikey`, `edgerun-android-keystore` | Normalizes hardware-backed signing around ECDSA P-256 `NodeID`s. |
| Services | `edgerun-node`, `edgerun-http`, `edgerun-tls`, `edgerun-quic`, `edgerun-email`, `edgerun-oci` | Implements service stacks on top of `edgerun-rt`; transport-independent protocol pieces live in `edgerun-protocols`. |
| Bare metal | `edgerun-rt`, `edgerun-platform`, `edgerun-unikernel`, `edgerun-virtio`, `edgerun-rtl8125` | Provides runtime, platform primitives, drivers, boot support, and a freestanding unikernel binary; TFTP and PXE/iPXE ABI code lives in `edgerun-protocols`. |
| Local support crates | `edgerun-json`, `edgerun-encoding`, `edgerun-hpack`, `edgerun-qpack`, `edgerun-crypto`, `edgerun-clap`, `edgerun-log`, `edgerun-vfs`, `edgerun-virtual-disk` | Provides local utilities. These crates are not alternate protocol wire formats. |

## End-to-End Shape

An Edgerun node starts with a P-256 identity. The signer may be software-backed
for development or hardware-backed through TPM, YubiKey, Android Keystore, or
another backend in production.

A new node writes a signed genesis event. After that, every authoritative update
is either a stream event produced by the fixed stream writer or the durable
result of a command that the target node validated and committed or rejected in
its own stream.

Storage persists those logs and immutable objects. It may maintain stream heads,
object indexes, replay records, file indexes, snapshots, and service-specific
views, but those are derived from the event log. Rebuilding an index must not
change authority.

Mesh, capability, and service crates sit above that authority boundary. They can
carry commands, capability envelopes, route hints, external protocol traffic, or
service results, but delivery is not authority. Trust enters only when local
policy validates the record and the target node records the outcome.

## 10-Minute Proof

The fastest local proof is a software-key node plus a service bind check. The
software key path is development-only; production nodes should use TPM, YubiKey,
or another hardware signing backend.

```bash
cargo run -p edgerun-node --bin edged -- init \
  --config /tmp/edgerun-node-a \
  --name demo-a \
  --software

cargo run -p edgerun-node --bin edged -- status \
  --config /tmp/edgerun-node-a
```

That creates a P-256 node identity, writes a signed genesis event, and then
reads the durable event log back as node status. The status output is the first
proof point: identity is the node address, and authoritative state begins at the
signed stream boundary.

To prove that the same runtime can realize service surfaces, run the bind check
with the broad service feature set:

```bash
cargo run -p edgerun-node --bin edged \
  --features "http https dns dhcp smtp imap proxy derived-db quic acme" \
  -- bind-check
```

The command binds non-privileged loopback ports for HTTP, HTTPS, DNS, DHCP,
SMTP, IMAP, proxy, derived DB, QUIC, and ACME support, then prints a JSON report.
Use `--standard-ports` only when the process has permission to bind privileged
ports.

Two-node smoke path:

```bash
cargo run -p edgerun-node --bin edged -- init \
  --config /tmp/edgerun-node-b \
  --name demo-b \
  --software

cargo run -p edgerun-node --bin edged -- status \
  --config /tmp/edgerun-node-b
```

For a polished demo, show both status outputs side by side: two different node
identities, two signed genesis streams, and the same runtime verifying each
node's local event-log state.

## Checks

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
