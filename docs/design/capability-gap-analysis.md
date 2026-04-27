# edgerun Reference Core: Implementation Status & Gap Analysis

**Last updated:** 2026-04-28
**Project:** edgerun_core
**Status:** Historical gap analysis, refreshed with current crate names and
workspace caveats. Verify individual gap claims against code before treating
them as current release status.

---

## Project Overview

A Rust multi-crate workspace with 113 first-level directories under `crates/`
and 110 crate manifests implementing the **edgerun v0 protocol**: a
distributed, identity-based, append-only information fabric for peer-to-peer
mesh networking, capability-discovered hardware abstraction, and
compute-marketplace flows.

Current workspace caveat: `cargo metadata --no-deps --format-version 1`
succeeds and reports 110 workspace packages/members, but the root
`Cargo.toml` textual `members` array still contains a duplicate
`crates/edgerun-tftp` entry.

### Architecture Summary

```
                    edgerund (edgerun-node)
 ┌──────────────────────────────────────────────────────────────┐
 │  CLI (init/run/status)                                       │
 │  ┌────────────────────────────────────────────────────────┐  │
 │  │  Identity: TPM 2.0 | YubiKey | Android Keystore | SW  │  │
 │  └────────────────────────────────────────────────────────┘  │
 │  ┌────────────────────────────────────────────────────────┐  │
 │  │  Mesh: edgerun-mesh + mesh-link + mesh-router          │  │
 │  │    ECDSA-signed frames | Bellman-Ford routing          │  │
 │  └────────────────────────────────────────────────────────┘  │
 │  ┌────────────────────────────────────────────────────────┐  │
 │  │  Command dispatch: signature verify | delegation      │  │
 │  └────────────────────────────────────────────────────────┘  │
 │  ┌────────────────────────────────────────────────────────┐  │
 │  │  Workload: OCI pull | cgroups v2 | namespaces | exec  │  │
 │  │  Metering: WorkMeter | RC-us billing                   │  │
 │  └────────────────────────────────────────────────────────┘  │
 │  ┌────────────────────────────────────────────────────────┐  │
 │  │  Storage: append-only event log | AES-GCM blobs        │  │
 │  └────────────────────────────────────────────────────────┘  │
 │  ┌────────────────────────────────────────────────────────┐  │
 │  │  Health: HTTP /health endpoint                         │  │
 │  └────────────────────────────────────────────────────────┘  │
 └──────────────────────────────────────────────────────────────┘

 Protocol: edgerun-proto (prost) + edgerun-core (validation, crypto, conformance)
 Runtime:  edgerun-rt for no_std/bare node paths; host crates use std where gated
 JSON:     edgerun-json (zero-dep, published on crates.io v1.0.149)
```

### What Works

- **Core protocol**: Full protobuf bindings, canonical encoding, deterministic serialization
- **Crypto**: Inline SHA-256/384/512, HKDF, HMAC, ECDSA P-256 sign/verify, domain tag constants defined
- **Command validation**: Structural, cryptographic, replay detection (hash-keyed), timing, delegation chain
- **Command dispatch**: AddController, RemoveController, TransferControl, PublishSnapshot, FetchObject, Query, ExecuteWorkload, TerminateWorkload, Custom
- **Delegation chain**: Signature verification, recipient/issuer continuity, revocation checks, timing bounds, action attenuation checks
- **Controller set management**: Projection from event log, survives restarts
- **Storage**: Append-only event log, AES-GCM encrypted blobs, content-addressed storage, index rebuild
- **Mesh networking**: Frame types, ECDSA signing, raw Ethernet + UDP link layer, Bellman-Ford routing, multicast discovery
- **TCP sessions**: SessionHello/SessionAccept handshake, TCP listener, peer reconnection
- **OCI workloads**: Container pull, cgroups v2, namespaces, pivot_root, seccomp-BPF stub, resource tracking, rate limiting
- **Metering**: WorkMeter for RC-microsecond billing
- **Health**: HTTP /health endpoint
- **Ingress**: Rate limiting, duplicate screening (FNV-1a 64-bit)
- **Conformance corpus**: 610 YAML test vectors across 11 suites (canonical, command, control, crypto, delegation, network, object, query, snapshot, stream, trust)
- **Hardware signing abstraction**: Unified `MeshSigner` trait with TPM, YubiKey, Android Keystore, software backends
- **edgerun-json**: Published crate (v1.0.149), zero-dep, up to 3.78x faster than serde_json on tape path

---

## Critical Implementation Gaps (P0 — Security Correctness)

### 1. Domain separation not applied to signatures
**Severity:** Critical — signature replay attacks possible across record types  
**Spec:** §17.9, §17.11 — `sig_input = sig_domain_tag || 0x00 || record_hash_bytes`  
**Status:** Domain tag constants are **defined** in `edgerun-core/src/crypto.rs` (14 hash tags, 13 signature tags) but **NOT applied** during signing.

All signing code (`signer.rs`, `command_dispatch.rs`, `session.rs`, `mesh_node.rs`, `mesh_network.rs`) calls `sign_prehash(digest)` with the **raw SHA-256 digest** without prepending the domain tag. This means a signature intended for one record type (e.g., CommandEnvelope) could potentially be replayed against another (e.g., EventEnvelope).

**Files affected:**
- `crates/edgerun-node/src/signer.rs:48` — `sign_prehash(digest)`
- `crates/edgerun-node/src/command_dispatch.rs:1567` — `signer.sign_digest(&digest_bytes)` where `digest_bytes` is raw SHA-256
- `crates/edgerun-node/src/session.rs:343` — `sign_prehash(digest)`
- `crates/edgerun-node/src/mesh_node.rs:260` — `sign_prehash(digest)`
- `crates/edgerun-node/src/mesh_network.rs:139` — `sign_prehash(digest)`

**Fix required:** Prepend domain tag bytes before hashing: `SHA-256(domain_tag || 0x00 || canonical_bytes)`

### 2. Assurance requirements ignored during command validation
**Severity:** High — marketplace trust model broken  
**Spec:** `CommandEnvelope.requested_assurance` field (field 14)  
**Status:** The `requested_assurance` field is **never inspected** by the command validation pipeline.

A command can request hardware-backed or attested-runtime execution, but the receiving node has no logic to evaluate whether it can satisfy that requirement. AssuranceClaim types are fully defined in `trust.proto` and validators exist in `validators.rs`, but the daemon:
- Does NOT generate assurance claims during init
- Does NOT attach them to genesis or events
- Does NOT evaluate incoming assurance requirements during command validation

**File:** `crates/edgerun-core/src/command.rs` — `validate_command()` does not check `command.requested_assurance`

### 3. Delegation chain does not verify root trust
**Severity:** High — unauthorized delegations accepted  
**Spec:** Root issuer must be "acceptable under local trust policy"  
**Status:** `validate_delegation_chain()` in `command.rs` checks chain continuity, revocation, and timing, but does **NOT** verify that the first delegation's issuer is in `ctx.trusted_root_ids`.

The conformance validator in `validators.rs` does check `trust_roots`, but the **runtime** `command.rs` delegation validator does not.

**File:** `crates/edgerun-core/src/command.rs` — `validate_delegation_chain()` never references `ctx.trusted_root_ids`

### 4. PerformanceCertificate not cryptographically bound
**Severity:** Medium-High — billing fraud possible  
**Status:** ✅ **RESOLVED** — `PerformanceCertificate::verify()` validates both digest integrity and ECDSA signature. New certificates are signed during creation via `cache_cert()`. Legacy unsigned certs are signed on first use and re-cached. Node identity mismatch is also detected.

---

## High-Priority Gaps (P1 — Security Hardening)

### 5. Seccomp-BPF filter not applied
**Spec:** ~75 essential syscalls whitelist  
**Status:** ✅ **RESOLVED** — Seccomp-BPF is implemented in `crates/edgerun-oci/src/seccomp.rs` with architecture-specific allow-lists. Applied in the container child setup before exec — fail-closed, container startup aborts if seccomp can't be applied.

### 6. Action lifecycle events partially implemented
**Status:** `record_action_event()` function exists and is called in `command_dispatch.rs`, emitting `ActionCompleted`/`ActionFailed` events. However, `ACTION_STARTED` is NOT emitted before work begins.

### 7. WorkSettlement bilateral signing missing
**Spec:** Buyer+seller signed settlement as cryptographic proof  
**Status:** Only unilateral `WorkAccounting` recorded on provider side. No bilateral settlement protocol.

### 8. ComputeAdvertisement not published/discovered
**Status:** Type defined in `accounting.rs` but no mesh-based publish/discover flow exists.

### 9. Query signatures not verified
**Status:** `QueryRequest.signature` is optional and **never checked** even when present. Any peer can query without authorization.

### 10. Query proof generation missing
**Spec:** `QueryResultFragment` should include `proof_objects` (SnapshotSetProof, EventSetProof, ObjectAssertionProof)  
**Status:** Query engine returns basic event refs and snapshot refs but does NOT generate proof bundles.

---

## Medium-Priority Gaps (P2 — Protocol Completeness)

### 11. Protocol messages defined but not handled
- **Secret management** (types 1001-1003): `PutSecret`, `DeleteSecret`, `ListSecrets` — proto + crate exist, **no dispatch handlers**
- **RelayEnvelope**: Store-and-forward semantics defined but **no inbound relay processing**
- **IdentityRecord**: Identity registration/supersession defined but **not used** — identity is static (genesis only)
- **StoreObject** (type 5): Proto command defined, **no handler**

### 12. Storage model gaps
- **Chunking**: Protocol defines `ChunkManifest` + `ChunkEntry` but storage uses single encrypted blobs
- **Storage tiers**: Hot/warm/cold/archive tiers defined but single blob store only
- **Object descriptors**: `put_object` doesn't create `LogicalObjectDescriptor` or `StoredRepresentationHeader`

### 13. Fetch queue response processing incomplete
**Status:** `run_fetch_queue_consumer` in `daemon.rs` logs object refs but does NOT decode `bundled_result_object`, store fetched objects, or record fetch events.

### 14. Identity lifecycle missing
**Status:** Node identity is static (genesis payload + config). No `IdentityRecord` creation, storage, or verification. No identity rotation or supersession.

---

## Hardware Capability Gaps

39 hardware adapter crates exist, mostly at **trait definition + discovery-only** stage.

### Fully Implemented
| Crate | Status |
|-------|--------|
| `edgerun-goodix-fingerprint` | USB protocol parsing, enrollment control |
| `edgerun-tpm` | TPM 2.0 commands, Linux TPM adapter, key operations |
| `edgerun-yubikey` | YubiKey PIV/APDU signing path (subset: PIV only, no OTP/U2F) |

### Discovery + Trait Definitions (No I/O)
| Crate | Missing |
|-------|---------|
| `linux-pci`, `linux-usb` | Full driver binding, resource mapping, interrupt setup, transfer endpoints |
| `evdev-input`, `drm-display` | Event loop, device-specific translation, atomic modesetting |
| `amd-xdna`, `linux-npu` | Command queue, workload submission, model execution API |

### Control Interface Only (No Runtime I/O)
| Crate | Missing |
|-------|---------|
| `bluetooth`, `mgmt-bluetooth` | GATT client/server, LE links, security (SMP), MTU, L2CAP |
| `wifi`, `linux-wifi` | WPA, SSID/auth management, AP mode traffic handling |
| `display`, `drm-display` | Render/commit pathway, composable modesetting |
| `npu` | Backend implementation, scheduler, load management |

### Protocol Layer Not Exposed
| Crate | Missing Protocol |
|-------|-----------------|
| `bluetooth` | GATT discover/read/write/notify |
| `wifi` | WPA/hostapd integration |
| `nfc`, `linux-nfc` | LLCP, SNEP, tag read/write, card emulation |
| `usb`, `linux-usb` | HCI transfers, bulk/iso/ctl communications |

### Mediation / Remote Wrappers
| Crate | Status |
|-------|--------|
| `remote-capability` | RPC layer + proxy adapters, **no network transport backend** |
| `capabilities` | Core trait layer only, no hardware runtime enforcement |

---

## Known TODOs (Project-Specific)

| Location | TODO |
|----------|------|
| `edgerun-http2/src/connection.rs:183` | Implement missing HTTP/2 frame types |
| `crates/edgerun-oci/src/lib.rs:128` | Historical note: verify aarch64 syscall handling against current `seccomp` module before acting |
| `edgerun-virtual-disk/src/` | FlushAck implementation |
| `edgerun-node/src/daemon.rs` | Fetch queue: decode bundled_result_object, store objects |

---

## Conformance Status

- **Corpus:** 610 YAML test vectors across 11 suites ✅
- **Harness:** `conformance.rs` validates canonical, command, control, crypto, delegation, network, object, query, snapshot, stream, trust ✅
- **Independent implementations:** 1 of 2 required ❌
- **Query validator:** Less mature than delegation/command/trust validators ⚠️

---

## How to Use This Document

1. **P0 gaps** block security-correctness — fix before any production use
2. **P1 gaps** are hardening — needed for marketplace trust model
3. **P2 gaps** are protocol completeness — can be deferred
4. Hardware gaps are optional (behind `all-hardware` feature flag)
5. Reference for implementers adding platform-specific provider crates

---

## 3. edgerun-alsa-microphone
- Has: backend stub, capability descriptor, trait implementations
- Missing: complete microphone capture pipeline, device open/read, full ALSA integration

## 4. edgerun-alsa-speaker
- Has: backend info, playback card/PCM info, backend capability provider
- Missing: streaming output implementation (ring buffers, format conversion) inside this crate; minimal driver wrapper

## 5. edgerun-amd-xdna
- Has: NPU discovery + capability metadata for AMD xDNA
- Missing: command execution path, workload submission and response polling

## 6. edgerun-android-keystore
- Has: provider traits, key metadata, signing helpers
- Missing: key lifecycle operations (generate/import/delete), full Android keystore handshake flows

## 7. edgerun-audio-calibration
- Has: calibration request/response types and analysis helpers
- Missing: direct audio I/O or integration with capture/playback hardware

## 8. edgerun-biometrics
- Has: biometric state, modality enums2, policy helpers
- Missing: direct sensor pipeline (delegate to camera/fingerprint backends)

## 9. edgerun-bluetooth
- Has:
  - `BluetoothScanner` scan_nearby
  - `BluetoothConnectionProvider` list_connections
  - beacon, connection info models
- Missing (full BLE):
  - GATT client/server (discover/read/write/notify)
  - connect/disconnect/control over LE links
  - security (pair/bond/SMP), MTU, L2CAP channels
  - active scan/filter params and advertisement control

## 10. edgerun-camera-biometrics
- Has: biometric camera traits + validation patterns
- Missing: platform camera capture and HAL binding (in v4l2 adapter)

## 11. edgerun-capabilities
- Has: core capability provider traits, descriptor/operation model
- Missing: actual hardware runtime enforcement; abstract layer only

## 12. edgerun-core
- Has: core primitives and non-device-specific APIs (crypto/normalization)
- Missing: explicit hardware capability contract; not a platform driver crate

## 13. edgerun-display
- Has: `DisplayDevice` trait, mode/config descriptors
- Missing: concrete display pathway (render/commit) in this crate

## 14. edgerun-drm-display
- Has: DRM connector discovery, mode info, backend provider
- Missing: atomic/composable modesetting and full pipeline state commit

## 15. edgerun-evdev-input
- Has: evdev discovery/metadata and backend provider
- Missing: event loop plus device-specific input translation

## 16. edgerun-fingerprint
- Has: reader trait, enroll/verify structures, policies
- Missing: sensor implementation (Goodix adapter only; generic devices missing)

## 17. edgerun-goodix-fingerprint
- Has: Goodix protocol parsing, USB packet flows, enrollment control
- Missing: generic fingerprint interface and extensive error conditions mapping

## 18. edgerun-hardware-signing
- Has: provider composition for Android, TPM, YubiKey
- Missing: full physical transport and device discovery beyond adapters

## 19. edgerun-input
- Has: generic `InputDevice` trait, event type union, descriptors
- Missing: backends (evdev, windows, etc.) and runtime event consumer

## 20. edgerun-linux-netif
- Has: Linux interface enumeration, flags, capabilities
- Missing: network configuration (IP, routes, DHCP), advanced state updates

## 21. edgerun-linux-nfc
- Has: NFC adapter enumeration provider
- Missing: tag read/write, peer-to-peer activation, SNEP/LLCP implementation

## 22. edgerun-linux-npu
- Has: Linux NPU backend discovery and capability provider
- Missing: command queue and model execution API

## 23. edgerun-linux-pci
- Has: PCI bus scanning and device enumeration
- Missing: full driver binding, resource mapping, interrupt/setup flows

## 24. edgerun-linux-usb
- Has: USB device discovery and descriptor modeling
- Missing: transfer endpoints, bulk/iso/ctl communications

## 25. edgerun-linux-wifi
- Has: Wi-Fi interface backend + capability provider
- Missing: network connection control (WPA, SSID/auth management) in this crate

## 26. edgerun-mgmt-bluetooth
- Has: BlueZ mgmt API device/controller operations
- Missing: GATT profile and BLE streaming actions, module-level policy for connectivity

## 27. edgerun-microphone
- Has: microphone trait and capture request types
- Missing: device-specific adapters, buffering, format negotiation

## 28. edgerun-network-interface
- Has: abstract interface control trait and descriptor
- Missing: per-OS implementation (linux, windows) plus config mutation operations

## 29. edgerun-nfc
- Has: `NfcDevice` trait and target observation types
- Missing: tag read/write and card emulation on platform-specific paths

## 30. edgerun-npu
- Has: NPU device trait, request/result types, operation modes
- Missing: backend implementation (e.g., linux-npu, amd-xdna), scheduler and load management

## 31. edgerun-pci
- Has: PCI inventory trait and device model
- Missing: direct bus config access, userland driver controls

## 32. edgerun-proto
- Has: protobuf message bindings and generated types
- Missing: explicit hardware onboarding logic; this is wire-format only

## 33. edgerun-remote-capability
- Has: remote provider/transport protocol and proxy adapters for each capability
- Missing: network transport backend and production security (auth/encryption)

## 34. edgerun-speaker
- Has: speaker trait, playback request/result, capability descriptor
- Missing: platform-specific playback driver (ALSA, etc.) in this crate

## 35. edgerun-tpm
- Has: TPM command model, key operations, and Linux TPM adapter
- Missing: fully conformant TSS policy engine; hardware TPM behaviors still in lower primitives

## 36. edgerun-usb
- Has: USB inventory trait, device/interface model
- Missing: detailed transfer APIs and endpoint I/O

## 37. edgerun-v4l2-camera
- Has: V4L2 probe/capture, and camera-biometric specifics
- Missing: cross-platform camera capability (non-Linux), advanced queue controls

## 38. edgerun-wifi
- Has: Wifi scanner/controller/AP traits and result structs
- Missing: sophisticated connection flows, credential management, AP mode traffic handling

## 39. edgerun-yubikey
- Has: YubiKey reader probe, PIV/APDU signing path
- Missing: full OTP/U2F interfaces (only subset currently)

---

## Hardware capability gap categories

1. Device discovery only (no control or I/O): `linux-pci`, `linux-usb`, `linux-netif`, `linux-nfc`, `evdev-input`, `drm-display`, `amd-xdna`.
2. Control-only interface with no runtime I/O: `bluetooth`, `wifi`, `display`, `network-interface`, `npu`, `tpm`.
3. Protocol layer not exposed: `bluetooth` (GATT), `wifi` (WPA/hostapd), `nfc` (LLCP), `usb` (HCI transfers).
4. Mediation / remote wrappers: `remote-capability` adds RPC layer but no hardware backend.
