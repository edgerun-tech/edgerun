# Edgerun Core

Edgerun Core is an identity-based, append-only information fabric for edge
nodes. The protocol is defined in
[edgerun_core_protocol_v0_single_file.md](edgerun_core_protocol_v0_single_file.md):
nodes are addressed by cryptographic identity, durable truth is a set of
single-writer signed streams, data is carried as immutable objects, and access
is mediated by commands, capabilities, delegations, queries, and local policy.

The crate tree implements that model across protocol validation, event storage,
mesh transport, node daemons, services, hardware capability providers, and
bare-metal runtime support. Most library crates are `no_std` or `alloc`-first
and use local runtime/serialization/logging primitives rather than Tokio,
serde_json, clap, or tracing.

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

Generated protobuf schemas live in `proto/edgerun/v0`. They cover the core
families from the spec (`common`, `identity`, `trust`, `stream`, `object`,
`access`, `network`) plus capability runtime messages and generated web-platform
type catalogs used by codegen experiments.

## Implementation Map

| Layer | Main crates | What the code actually does |
|---|---|---|
| Protocol core | `edgerun-core`, `edgerun-proto` | Re-exports generated protobuf types as canonical protocol records; canonicalizes with prost encoding; clears signatures for signable forms; defines domain-separated hashes/signature inputs; validates command, stream, delegation, snapshot, query, network, identity, proof, and trust cases. |
| Streams | `edgerun-stream` | Creates signed genesis events, appends contiguous events, computes canonical event hashes, verifies ECDSA P-256 signatures, and rejects missing genesis, sequence gaps, bad `prev_hash`, and tampering. |
| Storage | `edgerun-storage` | Persists event logs as authority, stores encrypted blobs, derives logical object and representation ids, tracks stream heads/replay/object/snapshot indexes, rebuilds indexes from logs, records command replay outcomes, and supports file, memory, and block-backed storage paths. |
| Node runtime | `edgerun-node` | Loads node config, owns a stream, validates incoming `CommandEnvelope`s, records commit/reject events, runs a store task, handles TCP framed sessions, executes queries over `NodeStore`, tracks workloads, and bridges mesh/TCP traffic into the same command path. |
| Mesh | `edgerun-mesh`, `edgerun-mesh-link`, `edgerun-mesh-session`, `edgerun-mesh-daemon`, `edgerun-mesh-capability` | Uses P-256 public keys as `NodeID`s, signs mesh frames, routes by identity, discovers peers over raw Ethernet/UDP/multicast/tunnel links, performs ECDH session handshakes with replay/rekey policy, and carries remote capability envelopes over mesh frames. |
| Capabilities | `edgerun-capabilities`, `edgerun-capability-policy`, `edgerun-remote-capability` | Defines provider descriptors, selectors, requests, grants, invocations, results, revocations, policy decisions, session grant binding, signed capability messages, in-memory/TCP/Unix transports, and adapters for hardware capability traits. |
| Hardware identity | `edgerun-hardware-signing`, `edgerun-tpm`, `edgerun-yubikey`, `edgerun-android-keystore` | Normalizes hardware-backed signing around ECDSA P-256 `NodeID`s; supports TPM raw commands, YubiKey PIV/APDU flows, Android Keystore adapters, assurance metadata, and provider-specific signature wrappers. |
| Services | `edgerun-server`, `edgerun-http`, `edgerun-tls`, `edgerun-quic`, `edgerun-dns`, `edgerun-dhcp`, `edgerun-dhcpv6`, `edgerun-email`, `edgerun-proxy`, `edgerun-oci` | Implements protocol/service stacks on top of `edgerun-rt`: HTTP/1/2/3, TLS 1.3, QUIC, DNS/DHCP/TFTP/PXE pieces, SMTP/IMAP/LMTP, CONNECT proxying, and OCI parsing/runtime/registry paths. |
| Bare metal | `edgerun-rt`, `edgerun-platform`, `edgerun-unikernel`, `edgerun-ipxe`, `edgerun-tftp`, `edgerun-virtio`, `edgerun-rtl8125` | Provides a no_std async runtime, timers, channels, sync primitives, async I/O traits, TCP/UDP/IP primitives, platform primitives, VirtIO/RTL8125 drivers, TFTP boot support, and a freestanding unikernel binary. Dedicated crates own service protocols such as HTTP, DNS, TLS, QUIC, DHCP, and TFTP. |
| Hardware adapters | `edgerun-linux-*`, `edgerun-alsa-*`, `edgerun-evdev-input`, `edgerun-v4l2-camera`, `edgerun-goodix-fingerprint`, `edgerun-mgmt-bluetooth`, `edgerun-amd-xdna`, `edgerun-quectel-ec200a` | Exposes Linux/sysfs/ioctl backed capability providers for network interfaces, WiFi, PCI, USB, GPU, CEC, power, ALSA capture/playback, evdev, V4L2, Goodix fingerprint, Bluetooth mgmt, AMD XDNA, and cellular modem control. |
| Local support crates | `edgerun-json`, `edgerun-encoding`, `edgerun-hpack`, `edgerun-qpack`, `edgerun-crypto`, `edgerun-clap`, `edgerun-log`, `edgerun-vfs`, `edgerun-virtual-disk` | Provides no_std JSON/YAML/TOML paths, encoding utilities, HPACK/QPACK, the workspace crypto boundary, CLI parsing, logging, RAM-backed VFS/write-back tooling, virtual disks, NBD, and block protocols. |

## Current Workspace State

The tree currently contains 113 first-level directories under `crates/`, of
which 110 have `Cargo.toml` manifests. `cargo metadata --no-deps` succeeds in
this checkout and reports 110 workspace packages/members.

The textual `members` list in the root `Cargo.toml` still has 108 entries with
`crates/edgerun-tftp` listed twice. `edgerun-android-hardware`,
`edgerun-error`, and `edgerun-gpu` have manifests but are not listed directly in
that root member array; Cargo still includes them through workspace/path
resolution. `edgerun-css`, `edgerun-ecmascript`, and `edgerun-html` are
directories without crate manifests.

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

QEMU boot wrappers:

```bash
scripts/qemu-unikernel.sh
scripts/qemu-unikernel-net-pump.sh
scripts/qemu-unikernel-swtpm.sh
```

## Protocol-Critical Crates

- `edgerun-core`: canonical protocol records, domain-separated hashing/signing,
  command validation, delegation validation, trust/revocation handling, route
  and query proof validation, conformance vector loader.
- `edgerun-stream`: event signing and stream-chain validation.
- `edgerun-storage`: authoritative event log, encrypted object/blob store,
  indexes, snapshots, command replay cache, rebuild and integrity checks.
- `edgerun-node`: validates and dispatches commands, records outcomes, serves
  queries, bridges TCP/mesh peers, and keeps all authoritative mutation behind
  stored events.
- `edgerun-hardware-signing`: converts hardware-backed keys into mesh identities
  and signs domain-separated record hashes without exposing private keys.

## Important Caveats

- Route advertisements, query proofs, snapshots, and cached views are evidence
  or acceleration structures; they do not replace stream authority.
- Many hardware crates have host-Linux implementations and bare-target stubs so
  the type surface can compile in no_std contexts.
- Browser/web-platform files under `proto/` and `docs/` are generated type
  catalogs and design/reference material. The current crate tree is not a full
  browser engine implementation.
- The docs in this repo are a mix of current implementation notes and older
  design snapshots. Prefer source code and crate-local READMEs for exact status.
