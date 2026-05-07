# Current Architecture and Consolidation Map

Edgerun Core is now organized around one internal protocol boundary: concrete
rkyv-archived protocol records. External standards such as HTTP, DNS, TLS, QUIC,
OCI, HPACK, QPACK, JSON, DHCP, NFC, TPM, and device protocols are implementation
domains, not alternate Edgerun wire protocols.

## Authority flow

```text
key material / hardware signer
  -> protocol signer
  -> signed event or command
  -> append-only stream
  -> durable event log
  -> derived indexes, caches, snapshots, routes, views
```

Only signed streams and immutable objects are authoritative. Indexes, query
results, route hints, replay caches, dashboards, health endpoints, and local
materialized views are derived and must be rebuildable or rejectable.

## Runtime layers

| Layer | Owner crates | Boundary rule |
|---|---|---|
| Wire boundary | `edgerun-wire` | rkyv-only archive/access API. No compatibility schemas. |
| Protocol records and validation | `edgerun-core`, `edgerun-verify`, `edgerun-sign`, `edgerun-sign-p256` | Native records, domain-separated hashes, canonical signing input, signature verification, and protocol validators. |
| Stream construction | `edgerun-stream` | Single-writer stream IDs, genesis creation, contiguous append, prev-hash linkage, signing, and stream verification. |
| Durable authority | `edgerun-storage`, `edgerun-edgefs` | Event logs and objects are stored durably; indexes and snapshots are derived from those logs. |
| Node runtime | `edgerun-node`, `edgerun-node-bootstrap`, `edgerun-keygen` | Node identity creation, bootstrap, command dispatch, status, provisioning listener, and stream append integration. |
| Capabilities and policy | `edgerun-capabilities`, `edgerun-capability-policy`, `edgerun-remote-capability`, `edgerun-mesh-capability` | Capability descriptors, grants, policy decisions, remote invocation envelopes, and mesh-carried capability messages. |
| Mesh and sessions | `edgerun-mesh`, `edgerun-mesh-link`, `edgerun-mesh-session` | Identity-addressed routing, signed mesh frames, peer/session state, and ECDH session setup. |
| Hardware identity | `edgerun-hardware-signing`, `edgerun-tpm`, `edgerun-yubikey`, `edgerun-android-keystore` | Hardware-backed P-256 `NodeID` signing adapters. |
| Services | `edgerun-node`, `edgerun-http`, `edgerun-tls`, `edgerun-quic`, `edgerun-dns`, `edgerun-email`, `edgerun-oci`, `edgerun-acme`, `edgerun-oauth` | External service runtimes built on top of the runtime and protocol fabric; transport-independent protocol pieces live in `edgerun-protocols`. |
| Device abstractions | `edgerun-*-capability`, `edgerun-linux-*`, `edgerun-alsa-*`, `edgerun-drm-display`, `edgerun-evdev-input`, `edgerun-v4l2-camera`, `edgerun-goodix-fingerprint` | Small platform-neutral type crates plus Linux or device-specific adapters. |
| Bare target | `edgerun-rt`, `edgerun-platform`, `edgerun-unikernel`, `edgerun-virtio`, `edgerun-rtl8125`, `edgerun-ipxe` | no_std-first runtime and hardware boot/device path; boot protocol codecs live in `edgerun-protocols`. |
| Local utilities | `edgerun-json`, `edgerun-encoding`, `edgerun-hpack`, `edgerun-qpack`, `edgerun-crypto`, `edgerun-log`, `edgerun-clap`, `edgerun-url`, `edgerun-glob`, `edgerun-regex` | Utility crates may parse external formats but must not define Edgerun protocol authority. |

## Consolidation rules

1. Do not add a second protocol model. If data crosses an Edgerun internal wire
   boundary, archive the concrete rkyv type.
2. Keep signing and verification in `edgerun-sign`, `edgerun-sign-p256`, and
   `edgerun-verify`; do not duplicate signer-specific verification in storage,
   node, mesh, or service crates.
3. Keep stream sequencing in `edgerun-stream`; storage should persist and index
   events, not reimplement stream rules.
4. Keep durable authority in event logs and objects. Derived state must be
   explicitly marked as derived and rebuildable.
5. Platform-neutral device crates should remain type/interface crates. Linux,
   ALSA, DRM, evdev, V4L2, TPM, YubiKey, Android, and board-specific behavior
   belongs in adapter crates or target-gated modules.
6. External protocol crates should be consolidated by shared primitives, not by
   merging unrelated protocol implementations. For example, HTTP/2 and HTTP/3
   can share header/value validation helpers, but HPACK and QPACK remain separate
   protocol codecs.
7. New docs should describe implemented behavior, blocked behavior, or explicit
   active design. Avoid roadmap-only documents that name crates or modules that
   do not exist.

## Near-term consolidation candidates

| Area | Current shape | Consolidation direction |
|---|---|---|
| Signing adapters | `edgerun-stream` signs through `ProtocolSigner`; storage and node adapt hardware signers. | Keep the adapter at the edge. Avoid exposing adapter types from storage APIs. |
| Stream validation | Storage can validate chain integrity and optionally call `edgerun-stream::validate_stream`. | Move any remaining sequence/hash/signature rule duplication out of storage and into `edgerun-stream`. |
| Command outcomes | Command status appears in core validators, node dispatch, storage replay, and exchange projections. | Keep command semantics in `edgerun-core`; keep service-specific projections as derived views. |
| Capability transport | `edgerun-remote-capability` and `edgerun-mesh-capability` both carry invocation/session semantics. | Share envelope/domain helpers through core capability types; keep transport-specific queue/session code separate. |
| Device capability crates | Many type crates pair with one Linux/device adapter crate. | Merge only when a type crate has no reusable platform-neutral boundary. Otherwise keep neutral crate + adapter crate. |
| Linux-only adapters | `edgerun-linux-*`, ALSA, DRM, evdev, V4L2, TPM/YubiKey paths are host-specific. | Keep target gates strict and prevent these crates from leaking into bare-target or non-Linux default paths. |
| Encoding helpers | `edgerun-encoding`, `edgerun-hpack`, `edgerun-qpack`, and local string field helpers overlap at byte/string parsing edges. | Move generic byte/string helpers into `edgerun-encoding`; keep HPACK/QPACK table and instruction logic in their own crates. |
| Service deployment config | `deploy/server` is active deployment state consumed by `edgerun-server`. | Keep deployment docs/config together; avoid separate roadmap docs for server surfaces. |

## What should not be consolidated

- `edgerun-wire` and external codecs. Rkyv is the internal Edgerun protocol;
  HTTP, DNS, TLS, QUIC, JSON, HPACK, QPACK, and OCI are external protocol domains.
- Hardware signer backends. TPM, YubiKey, Android Keystore, and software/P-256
  signer paths share traits but have different security and platform properties.
- Runtime targets. Hosted Linux services, Linux device adapters, Android, and
  bare-metal/unikernel code should stay target-gated rather than hidden behind
  runtime compatibility shims.
- Derived service projections and authoritative event logs. Projections are
  views; logs are authority.

## Documentation boundary

The authoritative high-level references are:

- `README.md` for protocol rules and implementation map.
- `crates/README.md` for crate layout.
- this document for architecture and consolidation direction.
- crate READMEs for crate-local usage.
- `deploy/server/README.md` for active deployment configuration.

Delete or update documents that introduce independent protocol rules, duplicate
old crate maps, or describe modules that do not exist.

## Target domain folder layout

The flat `crates/` tree is becoming hard to navigate. The target shape should be
foldered by domain while preserving crate names, package names, and public APIs.
The folder name is navigation and ownership metadata, not part of the package
identity.

```text
crates/
  protocol/
    edgerun-wire
    edgerun-core
    edgerun-verify
    edgerun-sign
    edgerun-sign-p256
    edgerun-keygen
    edgerun-node-bootstrap
    edgerun-sign-verify-e2e
  authority/
    edgerun-stream
    edgerun-storage
    edgerun-edgefs
    edgerun-vfs
    edgerun-virtual-disk
  node/
    edgerun-node
    edgerun-server
    edgerun-machine-report
  capability/
    edgerun-capabilities
    edgerun-capability-policy
    edgerun-remote-capability
    edgerun-mesh-capability
  mesh/
    edgerun-mesh
    edgerun-mesh-link
    edgerun-mesh-session
  identity/
    edgerun-hardware-signing
    edgerun-tpm
    edgerun-yubikey
    edgerun-android-keystore
  service/
    edgerun-http
    edgerun-tls
    edgerun-quic
    edgerun-protocols::dns
    edgerun-protocols::http
    edgerun-protocols::dhcp
    edgerun-protocols::dhcpv6
    edgerun-protocols::smtp
    edgerun-protocols::lmtp
    edgerun-protocols::imap
    edgerun-email
    edgerun-email-auth
    edgerun-acme
    edgerun-oauth
    edgerun-protocols::proxy
    edgerun-oci
    edgerun-analytics
    edgerun-edit
    edgerun-exchange
    edgerun-exchange-api
    zen-client
  device-types/
    edgerun-bluetooth
    edgerun-biometrics
    edgerun-camera-biometrics
    edgerun-cec
    edgerun-display
    edgerun-fingerprint
    edgerun-gpu
    edgerun-input
    edgerun-microphone
    edgerun-network-interface
    edgerun-nfc
    edgerun-npu
    edgerun-pci
    edgerun-power
    edgerun-speaker
    edgerun-usb
    edgerun-wifi
  linux-adapters/
    edgerun-alsa-microphone
    edgerun-alsa-speaker
    edgerun-amd-xdna
    edgerun-bluetooth-gatt
    edgerun-drm-display
    edgerun-evdev-input
    edgerun-face-detection
    edgerun-goodix-fingerprint
    edgerun-linux-cec
    edgerun-linux-gpu
    edgerun-linux-netif
    edgerun-linux-nfc
    edgerun-linux-npu
    edgerun-linux-pci
    edgerun-linux-power
    edgerun-linux-sysfs
    edgerun-linux-usb
    edgerun-linux-wifi
    edgerun-mgmt-bluetooth
    edgerun-passport-nfc
    edgerun-v4l2-camera
  bare-target/
    edgerun-rt
    edgerun-platform
    edgerun-unikernel
    edgerun-virtio
    edgerun-rtl8125
    edgerun-ipxe
    edgerun-protocols::tftp
    edgerun-event
  appliance/
    edgerun-tcl-ac
    edgerun-tcl-ac-cli
    edgerun-matter
    edgerun-tuya
    edgerun-quectel-ec200a
  utility/
    edgerun-crypto
    edgerun-encoding
    edgerun-json
    edgerun-log
    edgerun-clap
    edgerun-clap-derive
    edgerun-url
    edgerun-glob
    edgerun-regex
    edgerun-error
    edgerun-hpack
    edgerun-qpack
    edgerun-bench
```

### Migration order for domain folders

1. Normalize internal dependencies to use `[workspace.dependencies]` where the
   workspace already defines the crate. This reduces relative `../crate` path
   churn before moves.
2. Move low-dependency utility crates first: `utility/`, then `protocol/`.
3. Move authority crates next: `authority/`, then update node/storage dependents.
4. Move service and device domains in separate commits so failures localize to a
   domain.
5. Move active bare-target work last, especially `edgerun-unikernel`,
   `edgerun-virtio`, and scripts that assume `crates/<name>` paths.
6. Keep package names unchanged. Do not rename crates during folder migration.
7. After each domain move, run `cargo metadata --no-deps --format-version 1` and
   `cargo check --workspace` before continuing.

### Folder move rules

- Do not leave compatibility symlinks or forwarding crates.
- Do not move active work in the same commit as unrelated folder moves.
- Do not use folder moves to hide stale crates; delete stale crates first.
- Update `Cargo.toml` workspace members and path dependencies in the same commit
  as each domain move.
- Prefer domain commits over one huge repository-wide move so breakage identifies
  the affected domain.
