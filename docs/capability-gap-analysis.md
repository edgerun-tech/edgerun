# Edgerun Reference Core: Implementation Status & Gap Analysis

**Last updated:** 2026-04-28
**Project:** edgerun_core
**Status:** Active development — core protocol, crypto, storage, mesh, hardware signing, and most hardware I/O are functional.

---

## Project Overview

A Rust multi-crate workspace with 113 first-level directories under `crates/`
and 110 crate manifests implementing the **edgerun v0 protocol**: an
identity-routed, append-only information fabric with capability delegation,
local trust policy, hardware-backed signing, mesh transport, object storage,
and infrastructure services.

The generated web-platform protobuf catalogs under `proto/edgerun/v0/` are
schema and design assets. They are not evidence that the current workspace
contains a complete browser engine implementation.

### Architecture Summary

```
                    edgerund (edgerun-node)
 ┌──────────────────────────────────────────────────────────────┐
 │  CLI (init/run/status)                                       │
 │  ┌────────────────────────────────────────────────────────┐  │
 │  │  Identity: TPM 2.0 | YubiKey | Android Keystore | SW  │  │
 │  └────────────────────────────────────────────────────────┘  │
 │  ┌────────────────────────────────────────────────────────┐  │
 │  │  Mesh: edgerun-mesh + mesh-link + mesh-session          │  │
 │  │    ECDSA-signed frames | Bellman-Ford routing          │  │
 │  └────────────────────────────────────────────────────────┘  │
 │  ┌────────────────────────────────────────────────────────┐  │
 │  │  Command dispatch: signature verify | delegation      │  │
 │  │  Domain-separated signatures | Root trust verification │  │
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
 JSON:     edgerun-json (published on crates.io v1.0.149)
 Crypto:   edgerun-crypto (single boundary, no ring/openssl)
```

### What Works

- **Core protocol**: Full protobuf bindings, canonical encoding, deterministic serialization
- **Crypto**: Domain-separated signing (`SHA-256(domain_tag || 0x00 || record_hash)`), SHA-256/384/512, HKDF, HMAC, ECDSA P-256 sign/verify
- **Domain separation**: 14 hash tags + 13 signature tags defined AND applied across all signing paths (signer.rs, command_dispatch.rs, session.rs, mesh_node.rs, mesh_network.rs, all hardware signing backends)
- **Command validation**: Structural, cryptographic, replay detection (hash-keyed), timing, delegation chain
- **Delegation chain**: Signature verification, recipient/issuer continuity, revocation checks, timing bounds, action attenuation, **root trust verification** against `trusted_root_ids`
- **Controller set management**: Projection from event log, survives restarts
- **Storage**: Append-only event log, AES-GCM encrypted blobs, content-addressed storage, index rebuild
- **Mesh networking**: Frame types, ECDSA signing, raw Ethernet + UDP link layer, Bellman-Ford routing, multicast discovery
- **TCP sessions**: SessionHello/SessionAccept handshake, TCP listener, peer reconnection
- **OCI workloads**: Container pull, cgroups v2, namespaces, pivot_root, seccomp-BPF, resource tracking, rate limiting
- **Seccomp-BPF**: Architecture-specific syscall allow-lists (x86_64: ~80, aarch64: ~80), applied in `pre_exec` before exec — fail-closed
- **Metering**: WorkMeter for RC-microsecond billing
- **Health**: HTTP /health endpoint
- **Ingress**: Rate limiting, duplicate screening (FNV-1a 64-bit)
- **Conformance corpus**: 610 YAML test vectors across 11 suites
- **Hardware signing abstraction**: Unified `MeshSigner` trait with TPM 2.0, YubiKey, Android Keystore, software backends
- **edgerun-json**: Published crate (v1.0.149), zero-dep, with fast tape parsing for structural scans
- **HTTP stack**: HTTP/1.1, HTTP/2, HTTP/3 with HPACK/QPACK, stream multiplexing, flow control
- **DNS server**: Authoritative + recursive DNS with YAML config, hot-reload
- **DHCPv6**: Full DHCPv6 implementation
- **WiFi**: nl80211 netlink scan/connect/disconnect/AP mode
- **Bluetooth**: HCI management socket, discovery/connect/pair
- **Audio**: ALSA capture/playback with mixer control
- **Camera**: V4L2 streaming capture with mmap
- **Input**: evdev event loop with poll(), EVIOCGRAB
- **Face detection**: Haar cascade CPU-based detection
- **Secret service**: D-Bus Secret Service daemon with biometric sessions
- **Audio calibration**: Sweep-based speaker/mic calibration
- **Audio liveness**: VAD + challenge/response anti-spoofing
- **CSS rasterizer**: Scanline renderer with AVX2 SIMD, tile multicore (~6× on 6-core), 148-color LUT, 9 border patterns, 16 blend modes
- **CSS cascade engine**: Full per-property cascade with specificity, origin/importance, combinators, attr selectors, pseudo-classes
- **CSS value parser**: 30+ length units, all color formats, calc()/min()/max()/clamp(), var(), url() — 101 tests
- **WebGPU renderer**: Fragment shader rasterizer with painter's algorithm, solid/linear/radial/conic gradients, borders, shadows
- **CSS minifier**: 60-70% compression via proto canonical varint property IDs
- **CSS property knowledge graph**: 98 properties with metadata, fluent query API, shorthand expansion
- **WCAG accessibility analyzer**: Contrast ratio, font-size, spacing, animation duration, focus indicators
- **CSS complexity analyzer**: Specificity scoring, cascade depth, dead rules, layout trigger percentage
- **Deterministic replay engine**: Binary format, record/playback for bug reproduction
- **CSS rule optimizer**: Merge selectors, remove dead rules, flatten combinators, reorder by specificity
- **Layout budget estimator**: Predict rendering cost, dirty tracking, incremental relayout
- **Cascade debugger**: CLI tool showing rule ranking, per-property winners, inheritance chain
- **Render proof**: CPU vs GPU pixel-by-pixel comparison via fuzz corpus
- **HTML render pipeline**: Block layout pipeline (HTML+CSS → DOM → cascade → layout → paint → rasterize → PNG). Flexbox/grid not yet implemented.
- **edgerun-edit**: AST-level Rust code editor using syn 2 + prettyplease

---

## Resolved Gaps (Previously Reported)

| # | Gap | Resolution |
|---|-----|-----------|
| P0-1 | Domain separation not applied to signatures | ✅ **RESOLVED** — `sign_record()`, `sign_record_with_tpm_checked()`, `sign_record_with_yubikey_checked()`, `sign_record_with_keystore_checked()` all apply domain tags via `signature_input()` in `edgerun-core/src/crypto.rs` and all hardware backends |
| P0-3 | Delegation chain does not verify root trust | ✅ **RESOLVED** — `command.rs:296-303` checks root issuer against `ctx.trusted_root_ids`, with test `delegation_chain_root_not_in_trusted_roots_is_rejected` |
| P0-4 | PerformanceCertificate not cryptographically bound | ✅ **RESOLVED** — `PerformanceCertificate::verify()` validates digest integrity and ECDSA signature |
| P1-5 | Seccomp-BPF filter not applied | ✅ **RESOLVED** — `crates/edgerun-oci/src/seccomp.rs` with architecture-specific allow-lists, applied in `pre_exec` |
| P1-6 | Action lifecycle events partially implemented | ✅ **RESOLVED** — `record_action_event()` emits `ActionCompleted`/`ActionFailed` events in `command_dispatch.rs` |

---

## Current Gaps

### P0 — Security Correctness

#### 1. Assurance requirements ignored during command validation
**Severity:** High — marketplace trust model incomplete
**Spec:** `CommandEnvelope.requested_assurance` field (field 14)
**Status:** The `requested_assurance` field is **never inspected** by the command validation pipeline.

A command can request hardware-backed or attested-runtime execution, but the receiving node has no logic to evaluate whether it can satisfy that requirement. AssuranceClaim types are fully defined in `trust.proto` and validators exist in `validators.rs`, but the daemon:
- Does NOT generate assurance claims during init
- Does NOT attach them to genesis or events
- Does NOT evaluate incoming assurance requirements during command validation

**File:** `crates/edgerun-core/src/command.rs` — `validate_command()` does not check `command.requested_assurance`

### P1 — Security Hardening

#### 2. WorkSettlement bilateral signing missing
**Spec:** Buyer+seller signed settlement as cryptographic proof
**Status:** Only unilateral `WorkAccounting` recorded on provider side. No bilateral settlement protocol.

#### 3. ComputeAdvertisement not published/discovered
**Status:** Type defined in `accounting.rs` but no mesh-based publish/discover flow exists.

#### 4. Query signatures not verified
**Status:** `QueryRequest.signature` is optional and **never checked** even when present. Any peer can query without authorization.

#### 5. Query proof generation missing
**Spec:** `QueryResultFragment` should include `proof_objects` (SnapshotSetProof, EventSetProof, ObjectAssertionProof)
**Status:** Query engine returns basic event refs and snapshot refs but does NOT generate proof bundles.

#### 6. ACTION_STARTED not emitted before work begins
**Status:** ✅ **RESOLVED** — `dispatch_execute_workload` emits `ActionStarted` before any work begins, and `ActionFailed` on all early-exit error paths (missing spec, policy violation, invalid image ref, rate limit exceeded). The complete lifecycle is now: Started → Committed → Completed/Failed.

### P2 — Protocol Completeness

#### 7. Protocol messages defined but not handled
- **Secret management** (types 1001-1003): `PutSecret`, `DeleteSecret`, `ListSecrets` — proto + `edgerun-secret-service` crate exist, but **no dispatch handlers** in `edgerund` command router
- **RelayEnvelope**: Store-and-forward semantics defined but **no inbound relay processing**
- **IdentityRecord**: Identity registration/supersession defined but **not used** — identity is static (genesis only), no rotation or supersession
- **StoreObject** (type 5): Proto command defined, **no handler**

#### 8. Storage model gaps
- **Chunking**: Protocol defines `ChunkManifest` + `ChunkEntry` but storage uses single encrypted blobs
- **Storage tiers**: Hot/warm/cold/archive tiers defined but single blob store only
- **Object descriptors**: `put_object` doesn't create `LogicalObjectDescriptor` or `StoredRepresentationHeader`

#### 9. Fetch queue response processing incomplete
**Status:** ✅ **RESOLVED** — `run_fetch_queue_consumer` now handles `bundled_result_object`, logs object refs with proper hex formatting, and reports fetch completion stats. Full object storage requires the mesh object protocol to fetch the actual blob data from the peer.

---

## Hardware Capability Gaps

### FULLY FUNCTIONAL (complete I/O pipelines)

| Crate | What It Does | I/O Method |
|-------|-------------|------------|
| **edgerun-tpm** | TPM 2.0 signing, key provisioning, PCR policies | `/dev/tpmrm0` direct I/O, TPM wire encoding/decoding |
| **edgerun-yubikey** | YubiKey PIV signing, attestation, PIN verification | Raw USB CCID transport (`/dev/bus/usb/`), `USBDEVFS_CONTROL` ioctl, APDU chaining |
| **edgerun-goodix-fingerprint** | Full fingerprint enrollment + identification | Raw USB bulk (`/dev/bus/usb/`), `ioctl(USBDEVFS_CLAIMINTERFACE)`, CRC8/CRC32 validation |
| **edgerun-evdev-input** | Input device event loop (keyboard, mouse, touch, pen, gamepad) | `poll()` event loop, `EVIOCGRAB` exclusive access, `/dev/input/eventN`, sysfs capability bitmaps |
| **edgerun-alsa-microphone** | Audio capture with format/channel/rate negotiation | `/dev/snd/pcmC*D*c`, ALSA ioctls (`HW_PARAMS`, `PREPARE`, `READI_FRAMES`) |
| **edgerun-alsa-speaker** | Audio playback + mixer control (volume, mute) | `/dev/snd/pcmC*D*p` + `/dev/snd/controlC*`, ALSA + mixer ioctls |
| **edgerun-v4l2-camera** | Streaming camera capture (MJPEG, YUYV, NV12, RGB24, Gray8/Y10/Y12/Y16) | V4L2 ioctls (`REQBUFS`, `QBUF`/`DQBUF`, `STREAMON`/`OFF`), `mmap()` |
| **edgerun-linux-wifi** | WiFi scan, connect (WPA-PSK), AP mode, regulatory domain | nl80211 generic netlink (`NETLINK_GENERIC`), full scan result parsing (SSID IE, RSN IE) |
| **edgerun-mgmt-bluetooth** | BT controller mgmt: powered, discoverable, pair, connect, scan | HCI mgmt socket (`AF_BLUETOOTH`, `BTPROTO_HCI`), `poll()` event loop |
| **edgerun-face-detection** | Haar cascade face detection with bounding boxes, landmarks, tracking | CPU-based integral image + Viola-Jones, NMS for overlapping detections |
| **edgerun-secret-service** | Freedesktop Secret Service D-Bus daemon with biometric sessions | D-Bus wire protocol over Unix socket, encrypted blob store, append-only audit log |
| **edgerun-audio-calibration** | Speaker/mic calibration sweeps with RMS/peak dBFS measurement | Uses `AlsaSpeakerBackend` + `AlsaMicrophoneBackend` for real I/O |
| **edgerun-audio-liveliness** | Voice activity detection, challenge/response anti-spoofing | Real mic capture, spectral analysis, latency checks |

### PARTIAL I/O (some operations work, missing complementary paths)

| Crate | What Works | What's Missing |
|-------|-----------|---------------|
| **edgerun-linux-netif** | Discovery via sysfs, interface up/down via `SIOCGIFFLAGS`/`SIOCSIFFLAGS` ioctl | IP address config, routing table, DHCP client, DNS config |
| **edgerun-linux-cec** | CEC transmit via `CEC_TRANSMIT` ioctl (Active Source, Standby, Wake, etc.) | CEC receive/event loop for incoming messages |
| **edgerun-amd-xdna** | DRM ioctls for context create/destroy, BO create/info/sync, exec cmd, power modes | Workload result polling, error handling, model execution API |

### DISCOVERY ONLY (sysfs/proc reads, no I/O)

| Crate | What It Does | Missing |
|-------|-------------|---------|
| **edgerun-drm-display** | DRM connector discovery, mode listing, connection status from sysfs | Actual modesetting — `present()` returns `CapabilityError::Unsupported`, no framebuffer control |
| **edgerun-linux-pci** | PCI bus scanning via sysfs (vendor/device IDs, class codes, driver, link speed, topology) | Driver binding, config space access, interrupt setup |
| **edgerun-linux-usb** | USB device enumeration via sysfs (bus/dev, vendor/product, strings, speed, driver, children) | USB device I/O, transfer endpoints, bulk/iso/ctl communications |
| **edgerun-linux-nfc** | NFC adapter enumeration from sysfs (power state, protocols) | Target polling, NDEF read/write, ISO-DEP communication, SNEP/LLCP |
| **edgerun-linux-npu** | NPU sysfs device info (driver, firmware, PCI addr, char device path) | Workload submission, ioctls, command queue |
| **edgerun-linux-gpu** | GPU enumeration via sysfs + DRM (vendor/device IDs, driver, card/render nodes, connector modes) | Modesetting, rendering, command submission |
| **edgerun-linux-power** | Power supply sysfs reads (battery capacity, voltage, current, AC online, lid state) | Power state changes, suspend/resume, CPU frequency control |

### JUST TYPES/TRAITS (no runtime code)

| Crate | Description | What Would Be Needed |
|-------|-------------|---------------------|
| **edgerun-bluetooth** | Abstract `BluetoothScanner`/`BluetoothConnectionProvider` traits | GATT client/server, LE links, security (SMP), MTU, L2CAP — implement using `edgerun-mgmt-bluetooth` as backend |
| **edgerun-wifi** | Abstract `WifiScanner`/`WifiController`/`WifiAccessPointController` traits | Implement using `edgerun-linux-wifi` as backend (nl80211 is already functional) |
| **edgerun-input** | Abstract `InputDevice` trait, event type union | Implement using `edgerun-evdev-input` as backend |
| **edgerun-display** | Abstract `DisplayDevice` trait, mode/config descriptors | Implement using `edgerun-drm-display` (once modesetting is added) |
| **edgerun-microphone** | Abstract `MicrophoneDevice` trait, capture request types | Implement using `edgerun-alsa-microphone` as backend |
| **edgerun-speaker** | Abstract `SpeakerDevice` trait, playback request types | Implement using `edgerun-alsa-speaker` as backend |
| **edgerun-fingerprint** | Abstract `FingerprintReader` trait, enrollment/verification types | `edgerun-goodix-fingerprint` is already a concrete implementation |
| **edgerun-camera-biometrics** | Abstract `CameraReader`/`CameraBiometricReader` traits, liveness challenge | Implement using `edgerun-v4l2-camera` as backend |
| **edgerun-biometrics** | Biometric state, modality enums, verification policy types | Orchestration layer — delegates to camera/fingerprint backends |
| **edgerun-network-interface** | Abstract `NetworkInterfaceController` trait, descriptor types | Implement using `edgerun-linux-netif` as backend |
| **edgerun-usb** | Abstract `UsbInventory` trait, device/interface model | Discovery is `edgerun-linux-usb`; I/O would need `libusb` or raw `usbfs` |
| **edgerun-pci** | Abstract `PciInventory` trait, device model | Discovery is `edgerun-linux-pci`; config access would need `/proc/bus/pci` or `sysfs` mmap |
| **edgerun-npu** | Abstract `NpuDevice` trait, workload request/result types | Implement using `edgerun-amd-xdna` or `edgerun-linux-npu` as backend |
| **edgerun-android-hardware** | Android NDK hardware access types | JNI + Android NDK bindings |

### INFRASTRUCTURE / ORCHESTRATION

| Crate | Status | Notes |
|-------|--------|-------|
| **edgerun-capabilities** | ✅ Trait + enforcement layer | `CapabilityProvider` trait, `CapabilityError`, descriptor helpers, validation — no hardware I/O by design |
| **edgerun-hardware-signing** | ✅ Orchestration + feature-gated adapters | `MeshSigner` trait, `HardwareMeshSigner<K>` wrapper, TPM/YubiKey/Android adapters behind features |
| **edgerun-remote-capability** | ⚠️ RPC protocol + transport, no hardware | Framed Unix/TCP socket transport, capability invocation serialization, policy wrapping, signing/verification — needs hardware proxy adapters |
| **edgerun-capability-policy** | ✅ Policy enforcement | Depends on `edgerun-capabilities`, `edgerun-crypto`, `edgerun-proto`, `edgerun-core` |
| **edgerun-machine-report** | ✅ Machine capability reporting | Enumerates all available hardware backends |

---

## How Capability Layers Connect

```
Abstract trait crate          Platform backend crate           Result
─────────────────            ────────────────────             ──────
edgerun-input        ←──────  edgerun-evdev-input             Event loop + classification
edgerun-microphone   ←──────  edgerun-alsa-microphone         ALSA PCM capture
edgerun-speaker      ←──────  edgerun-alsa-speaker            ALSA PCM playback + mixer
edgerun-camera-biometrics ←─  edgerun-v4l2-camera             V4L2 streaming capture
edgerun-display      ←──────  edgerun-drm-display             (once modesetting is added)
edgerun-wifi         ←──────  edgerun-linux-wifi              nl80211 netlink (already functional)
edgerun-bluetooth    ←──────  edgerun-mgmt-bluetooth          HCI mgmt socket (already functional)
edgerun-fingerprint  ←──────  edgerun-goodix-fingerprint      USB bulk enrollment/ID
edgerun-network-interface ←─  edgerun-linux-netif             sysfs + ioctl up/down
edgerun-npu          ←──────  edgerun-amd-xdna                DRM ioctls (partial)
edgerun-usb          ←──────  edgerun-linux-usb               sysfs discovery only
edgerun-pci          ←──────  edgerun-linux-pci               sysfs discovery only
```

The pattern is: **abstract trait crate defines the interface, platform backend crate provides the Linux implementation**. Most platform backends are already functional — the trait crates are thin wrappers that haven't been wired up.

---

## How to Use This Document

1. **P0 gaps** block security-correctness — fix before any production use
2. **P1 gaps** are hardening — needed for marketplace trust model
3. **P2 gaps** are protocol completeness — can be deferred
4. Hardware gaps are optional (behind `all-hardware` feature flag)
5. Reference for implementers adding platform-specific provider crates
