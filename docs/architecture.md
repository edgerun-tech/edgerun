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
results, replay caches, dashboards, health endpoints, route bindings, and local
materialized views are derived and must be rebuildable or rejectable.

## Universal Capability Routing

Edgerun treats every addressable capability as an identity-bound node. A node can
be a machine, but it can also be a webcam, microphone, object store, process
adapter, GPU worker, browser tab, database adapter, application UI, or any other
local or remote capability. The `NodeId` is the Ed25519 public key for that
capability. Roles, departments, and work types describe what that identity is
allowed to accept.

The shared work protocol now has a generic capability surface:
`NODE_ROLE_CAPABILITY`, `DEPARTMENT_CAPABILITY`, and capability work types for
request, invoke, event, and close. Device and service crates should describe
what a resource is, then bind it to this surface through admission-defined work
routing instead of creating their own authorization or routing path.

The generic payload is `CapabilityEnvelope`: session id, invocation id,
capability id, source/target node ids, packet kind, operation, content type,
sequence, timestamp, payload hash, and bytes. The capability id is deterministic:
`capability_id_from_descriptor(provider_node_id, descriptor)`. A descriptor is
the stable local resource name or descriptor bytes chosen by the provider, such
as `camera/front`, `object-store/main`, `program/shell`, or a richer binary
descriptor. Session and invocation ids are also deterministic and are derived
from the admitted route hash, source/target identities, capability id, sequence,
operation, and payload hash. Timestamps are event metadata; they are not
authority unless admission policy explicitly makes time part of the decision.

Video frames, audio chunks, input events, render commands, object bytes, and
control messages use the same routed shape. The packet kind maps to work type.
The operation maps to the capability-local verb. The content type constrains the
payload class. Adapters only encode/decode their domain payload; they do not
create alternate authorization paths.

Admission is the control plane. Relays are the packet data plane. Capability
nodes are leaves attached to relays:

```text
admission
  <-> relay mesh
        <-> identity-bound capability leaves
```

A relay connects to admission with a signed `NodeAvailable` that includes its
relay `ChannelEndpoint`. A capability asks admission for a relay assignment.
Admission returns a signed `RelayAssignment` containing the selected relay
endpoint; the capability then connects to that endpoint and proves the path by
sending signed protocol traffic over it. Capability leaves do not advertise
their own endpoints, publish their own availability, or select routes for other
nodes. If the relay branch dies, admission removes the relay and every assigned
capability behind it; each capability must ask admission for a new relay
assignment before receiving new work.

Work routing is predefined by admission. A sender asks admission for access to a
capability. Admission verifies policy and availability, then returns a signed
`WorkAdmission` that commits to budget, validity, route commitment, channel, and
an ordered relay path such as:

```text
sender -> relay_a -> relay_b -> capability
```

The sender feeds signed packets to its relay. Relays forward along the
admission-defined path. The destination capability accepts only packets whose
identity, role, department, work type, and route match the admitted work chain.
Relays do not need to be trusted; they can drop or delay packets, but signatures,
hashes, admitted routes, transit proofs, delivery proofs, and receipts prevent
them from forging capability intent or valid work.

For user-owned resources, the user is the admission authority. The admission
node may be local to the user or delegated by user-signed policy, but capability
access starts with a user signature and a user/admission-signed work order. A
node cannot bind itself into availability, authorize access to itself, or
choose the route that other nodes must accept.

This makes local and remote capability access the same protocol operation. A
local webcam can be gated as an identity-bound capability and routed to a local
app through a local relay. The same capability can later be exposed over the
network through relay policy without giving applications direct hardware access.

Applications follow the same model. An app package can have its own node
identity and local admission scope, while a device can use a TPM-backed
admission identity to assign relays for apps, device adapters, storage, and
network interfaces. The app SDK should expose UI construction and capability
requests, not machine topology. The host/device admission node decides whether a
request routes through memory, WebSocket, TCP, a local file adapter, a network
adapter, or a remote relay.

## App Runtime UX Mapping

User experience is modeled as a projection over runtime records, not as a
separate permission system. A UI action such as "allow this app to use this
storage location" becomes durable runtime state:

```text
UI intent
  -> RuntimeCapabilityGrant
  -> RuntimeStorageBinding / RuntimeNetworkBinding
  -> WorkAdmission
  -> RuntimeCapabilitySession
  -> CapabilityEnvelope packets
  -> RuntimeEvent audit log
```

The UI owns presentation and intent capture. It does not own storage, network,
identity, or device authority. Apps request capabilities through the SDK using
ordinary product concepts such as object storage, scoped fetch, camera input, or
render output. The runtime converts those requests into records:

| UX concept | Runtime record | Meaning |
|---|---|---|
| Install app | `RuntimeAppInstall` | Package identity, declared routes, namespaces, provided and required capabilities. |
| User allows capability | `RuntimeCapabilityGrant` | User/profile/app/release scoped grant with capability kind, operation, scope, constraints, validity, and signature bytes. |
| User chooses storage provider | `RuntimeStorageBinding` | App namespace bound to a provider identity, capability id, and backing kind such as native, browser, memory, or remote. |
| User chooses network access | `RuntimeNetworkBinding` | App network scope bound to a provider identity, origin/protocol/port/method policy, and capability id. |
| App starts using a grant | `RuntimeCapabilitySession` | Volatile live session bound to grant id, provider node id, admission hash, route commitment, and expiry. |
| User audits/revokes | `RuntimeEvent` stream | Append-only record of grants, bindings, session opens, denials, executions, and dispatches. |

This gives the UI a simple model:

```text
installed apps
  -> available capability requests
  -> granted choices
  -> active sessions
  -> audit history
```

It also keeps enforcement deterministic. React state, browser cookies, local
configuration, and service worker memory are only caches. The runtime record and
the admitted work chain are the authority.

Storage is always a capability binding. `RuntimeAppInstall.storage_namespaces`
declares which namespaces an app may request, but that declaration does not
authorize reads or writes. Actual storage use requires a live
`RuntimeStorageBinding` backed by a `RuntimeCapabilityGrant`. The same app
namespace may be backed by native durable storage, browser IndexedDB/local
storage, memory for tests, or a remote object provider such as Google Drive,
GitHub, S3, or another Edgerun node. Apps see `object.get` and `object.put`;
admission and provider binding decide where the objects live.

Network access is also a capability binding. `RuntimeAppInstall.declared_routes`
declares which HTTP routes an app can serve, but that declaration does not
authorize network exposure. Installing a route requires a matching
`RuntimeNetworkBinding` backed by a `RuntimeCapabilityGrant`. Apps request
scoped fetch, socket, HTTP route, or node-message capability. The runtime binds
that request to an origin, method set, protocol, provider identity, and eventual
admission route. Raw network is not ambient app authority.

Live sessions are intentionally separate from grants. A grant may survive app
restart or profile unlock, but `RuntimeCapabilitySession` is short-lived and
must be reopened through admission. Locking the Trust Container, switching
profiles, revoking a grant, or losing a relay branch clears sessions without
rewriting app install state.

## Runtime layers

| Layer | Owner crates | Boundary rule |
|---|---|---|
| Wire boundary | `edgerun-wire` | rkyv-only archive/access API. No compatibility schemas. |
| Protocol records and validation | `edgerun-core`, `edgerun-verify`, `edgerun-sign`, `edgerun-sign-p256` | Native records, domain-separated hashes, canonical signing input, signature verification, and protocol validators. |
| Stream construction | `edgerun-storage::stream` | Single-writer stream IDs, genesis creation, contiguous append, prev-hash linkage, signing, and stream verification. |
| Durable authority | `edgerun-storage` | Event logs, objects, and EdgeFS block filesystem state are stored durably; indexes and snapshots are derived from those logs. |
| Node runtime | `edgerun-node`, `edgerun-node-bootstrap`, `edgerun-keygen` | Node identity creation, bootstrap, command dispatch, status, provisioning listener, and stream append integration. |
| Capabilities and policy | `edgerun-capabilities`, `edgerun-remote-capability` | Capability descriptors, grants, policy decisions, remote invocation envelopes, and mesh-carried capability messages. |
| Mesh and sessions | `edgerun-mesh` | Identity-addressed routing, signed mesh frames, peer/session state, and ECDH session setup. |
| Hardware identity | `edgerun-hardware-signing`, `edgerun-tpm`, `edgerun-yubikey` | Hardware-backed P-256 `NodeID` signing adapters, including the Android Keystore provider under `edgerun-hardware-signing`. |
| Services | `edgerun-node`, protocol modules in `edgerun-protocols`, and node-owned service adapters | The node owns service orchestration, resource binding, ACME challenge handling, routing, and transport decisions. Transport-independent protocol pieces live in `edgerun-protocols`. |
| Device abstractions | `edgerun-*-capability`, `edgerun-linux-*`, `edgerun-evdev-input` | Small platform-neutral type crates plus Linux or device-specific adapters. |
| Bare target | `edgerun-rt`, `edgerun-platform`, `edgerun-unikernel`, `edgerun-virtio`, `edgerun-network-driver` | no_std-first runtime and hardware boot/device path; boot protocol codecs and PXE/iPXE ABI data live in `edgerun-protocols`. |
| Local utilities | `edgerun-json`, `edgerun-encoding`, `edgerun-hpack`, `edgerun-crypto`, `edgerun-log`, `edgerun-url` | Utility crates may parse external formats but must not define Edgerun protocol authority. |

## Consolidation rules

1. Do not add a second protocol model. If data crosses an Edgerun internal wire
   boundary, archive the concrete rkyv type.
2. Keep signing and verification in `edgerun-sign`, `edgerun-sign-p256`, and
   `edgerun-verify`; do not duplicate signer-specific verification in storage,
   node, mesh, or service crates.
3. Keep stream sequencing in `edgerun-storage::stream`; storage should persist
   and index events without duplicating stream rules outside the storage
   authority crate.
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
| Stream validation | Storage can validate chain integrity and call `edgerun-storage::stream::validate_stream`. | Keep sequence/hash/signature rules centralized in the storage stream module. |
| Command outcomes | Command status appears in core validators, node dispatch, storage replay, and exchange projections. | Keep command semantics in `edgerun-core`; keep service-specific projections as derived views. |
| Capability transport | `edgerun-remote-capability` and `edgerun-mesh-capability` both carry invocation/session semantics. | Share envelope/domain helpers through core capability types; keep transport-specific queue/session code separate. |
| Device capability crates | Many type crates pair with one Linux/device adapter crate. | Merge only when a type crate has no reusable platform-neutral boundary. Otherwise keep neutral crate + adapter crate. |
| Linux-only adapters | `edgerun-linux-*`, ALSA, DRM, evdev, V4L2, TPM/YubiKey paths are host-specific. | Keep target gates strict and prevent these crates from leaking into bare-target or non-Linux default paths. |
| Encoding helpers | `edgerun-encoding`, `edgerun-hpack`, and local string field helpers overlap at byte/string parsing edges. | Move generic byte/string helpers into `edgerun-encoding`; keep protocol table and instruction logic with the owning protocol modules. |
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
  authority/
    edgerun-storage
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
    edgerun-node::services::acme_runtime
    edgerun-oauth
    edgerun-protocols::proxy
    edgerun-oci
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
    edgerun-bluetooth-gatt
    edgerun-evdev-input
    edgerun-linux-netif
    edgerun-linux-pci
    edgerun-linux-sysfs
    edgerun-linux-usb
    edgerun-linux-wifi
  bare-target/
    edgerun-rt
    edgerun-platform
    edgerun-unikernel
    edgerun-virtio
    edgerun-network-driver
    edgerun-protocols::pxe
    edgerun-protocols::tftp
  utility/
    edgerun-crypto
    edgerun-encoding
    edgerun-json
    edgerun-log
    edgerun-url
    edgerun-error
    edgerun-hpack
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
