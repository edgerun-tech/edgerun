# Edgerun Reference Core

A multi-crate Rust workspace implementing three major systems from first principles: a **spec-driven browser rendering pipeline**, an **identity-routed peer-to-peer mesh network**, and **infrastructure services** (DNS, DHCP, HTTP, OCI runtime). All systems share a single protocol definition layer (44 Protocol Buffer files) and a custom async runtime — no tokio, no serde_json, no external crypto.

**Status:** Active development. Core protocol, CSS rasterizer, CSS cascade engine, DNS server, OCI runtime, compositor, and hardware signing are functional. See [docs/capability-gap-analysis.md](docs/capability-gap-analysis.md) for detailed implementation status.

---

## What This Project Could Replace

Based on the actual implemented code, edgerun reference core contains functional substitutes for these widely-used tools:

### Infrastructure Services

| Edgerun Component | Replaces | Status |
|---|---|---|
| **edgerun-net** (`edgerun-net` binary) | dnsmasq, BIND, CoreDNS, ISC DHCP, kea | ✅ Functional — authoritative + recursive DNS server + DHCPv6 in a single binary with YAML config and hot-reload |
| **edgerun-http** | hyper, reqwest, h2, quinn | ✅ Functional — HTTP/1.1, HTTP/2, HTTP/3 with HPACK/QPACK compression, stream multiplexing, flow control. Includes `h2spec-server` binary |
| **edgerun-oci-runtime** (`edgerun-oci` binary) | runc, crun, youki | ✅ Functional — OCI container runtime with cgroups v2, namespaces, pivot_root, seccomp-BPF (~80 syscall whitelist) |
| **edgerun-oci-registry** | crane, skopeo | ✅ Functional — OCI registry client for pulling container images |
| **edgerun-compositor** (`edgerun-compositor` binary) | sway, hyprland, river | ⚠️ Partial — Wayland compositor with zero external C dependencies, direct Linux kernel syscalls only. Uses evdev input + DRM |
| **edgerun-dhcp** | dhclient, dhcpcd | ⚠️ Partial — DHCP client/server implementation |
| **edgerun-tls** | rustls, native-tls, openssl (TLS) | ⚠️ Partial — TLS implementation backing edgerun-http |
| **edgerun-storage** | event-sourcing stores, append-only log systems | ✅ Functional — append-only event log with AES-GCM encrypted blobs, content-addressed storage, index rebuild |
| **edgerun-rt** | tokio, async-std, smol | ✅ Functional — epoll-based async runtime with reactor, worker threads, blocking pool, channels, async TCP/UDP |

### Web / Browser Engine

| Edgerun Component | Replaces | Status |
|---|---|---|
| **edgerun-rasterizer** | Skia, Cairo (rasterization layer) | ✅ Functional — scanline renderer with AVX2 SIMD, tile multicore (~6× on 6-core), 148-color LUT, 9 border patterns, 16 blend modes, gradient fills, text bitmap |
| **edgerun-css-cascade** | Blink/Gecko cascade engine | ✅ Functional — full per-property cascade with specificity, origin/importance, combinators, attr selectors, pseudo-classes |
| **edgerun-css-value-parser** | Blink/Gecko CSS parser | ✅ Functional — 30+ length units, all color formats (named/hex/rgb/hsl/hwb/lab/oklab), calc()/min()/max()/clamp(), var(), url(). 101 tests |
| **edgerun-wgpu** | Skia (GPU path), WebRender | ✅ Functional — WebGPU fragment shader renderer, painter's algorithm, solid/linear/radial/conic gradients, borders, drop shadows |
| **edgerun-css-minifier** | cssnano, csso, clean-css | ✅ Functional — 60-70% compression via proto canonical forms (1-byte varint property IDs vs ~12 byte strings) |
| **edgerun-property-graph** | MDN data, CSS spec lookup | ✅ Functional — 98 properties with metadata: layout triggers, paint effects, animation behavior, inheritance. Fluent query API |
| **edgerun-a11y-analyzer** | axe-core, Lighthouse accessibility | ✅ Functional — WCAG 2.2 static analysis: contrast ratio, min font-size, text spacing, animation duration, focus indicators |
| **edgerun-complexity-analyzer** | Stylelint, Parker, CSS stats tools | ✅ Functional — specificity scoring, cascade depth, dead rules, duplicates, layout trigger percentage |
| **edgerun-replay** | rr (record/replay), Chrome DevTools recordings | ✅ Functional — deterministic byte-for-byte render replay, bug reproduction via exported replay files |
| **edgerun-rule-optimizer** | PurgeCSS, UnCSS, cssnano rule merging | ✅ Functional — merge identical selectors, remove dead rules, flatten combinators, reorder by specificity |
| **edgerun-layout-budget** | WebPageTest rendering metrics | ✅ Functional — predict rendering cost before pixels drawn, calibrated per-phase costs, incremental dirty tracking |
| **edgerun-cascade-debugger** | Chrome DevTools Computed panel, Firefox Rules view | ✅ Functional — CLI cascade debugger showing all matching rules ranked by specificity, per-property winners, inheritance chain |
| **edgerun-selectors** | servo/selectors | ✅ Functional — CSS Selectors Level 4 implementation |
| **edgerun-render** | Full browser rendering pipeline | ⚠️ Partial — complete HTML/CSS → DOM → cascade → layout → paint → rasterize pipeline. Block layout only, no flexbox/grid |
| **edgerun-demo** / **edgerun-demo-wgpu** | Browser viewport | ✅ Functional — renders HTML+CSS to PNG, both CPU (rasterizer) and GPU (wgpu) paths |

### Security & Identity

| Edgerun Component | Replaces | Status |
|---|---|---|
| **edgerun-crypto** | ring, aws-lc-rs, openssl (crypto) | ✅ Functional — single crypto boundary: P-256 ECDSA/ECDH, Ed25519, Ed448, X25519, AES-GCM, ChaCha20-Poly1305, SHA-256/384/512, HMAC, HKDF, PBKDF2, X.509 certs (rcgen) |
| **edgerun-json** | serde_json | ✅ Functional — zero-dependency JSON, published on crates.io v1.0.149, up to 3.78× faster on tape path |
| **edgerun-hardware-signing** | tpm2-tools, yubikey-manager, Android KeyStore SDK | ✅ Functional — unified `MeshSigner` trait with TPM 2.0, YubiKey PIV, Android Keystore, and software backends. Keys never leave secure hardware |
| **edgerun-tpm** | tpm2-tools, tpm2-tss | ✅ Functional — TPM 2.0 commands, Linux TPM adapter, key operations |
| **edgerun-yubikey** | yubikey-manager, yubikey-piv-tool | ⚠️ Partial — YubiKey PIV/APDU signing (PIV only, no OTP/U2F) |
| **edgerun-android-keystore** | Android Keystore SDK | ⚠️ Partial — Android Keystore signing with optional JNI backend |
| **edgerun-capabilities** | OPA (Open Policy Agent) — capability model | ✅ Functional — capability types, validation, constraint system, role/modality/operation enums |
| **edgerun-log** | tracing, tracing-subscriber, slog | ✅ Functional — minimal structured logging, zero dependencies |

### Networking & Mesh

| Edgerun Component | Replaces | Status |
|---|---|---|
| **edgerun-mesh** | WireGuard, Tailscale (routing model) | ✅ Functional — identity-routed mesh (ECDSA P-256 public key = network address), signed frames, Bellman-Ford routing, multicast discovery |
| **edgerun-mesh-link** | Low-level tunnel protocols | ✅ Functional — raw Ethernet + UDP link layer |
| **edgerun-mesh-session** | WireGuard handshake, Noise protocol | ✅ Functional — encrypted session layer between mesh nodes |
| **edgerun-remote-capability** | gRPC, RPC frameworks | ⚠️ Partial — RPC layer with proxy adapters for all hardware capabilities, no network transport backend yet |

### Hardware Abstraction

| Edgerun Component | Replaces | Status |
|---|---|---|
| **edgerun-tpm** | tpm2-tools, tpm2-tss | ✅ Functional — `/dev/tpmrm0` direct I/O, signing, key provisioning, PCR policies |
| **edgerun-yubikey** | yubikey-manager, yubikey-piv-tool | ✅ Functional — raw USB CCID transport, PIV APDU flows, PIN verification, attestation |
| **edgerun-goodix-fingerprint** | libfprint (Goodix) | ✅ Functional — raw USB bulk enrollment + identification with CRC validation |
| **edgerun-evdev-input** | libinput | ✅ Functional — `poll()` event loop, `EVIOCGRAB`, device classification (keyboard/mouse/touch/pen/gamepad) |
| **edgerun-alsa-microphone** | ALSA utils, arecord | ✅ Functional — ALSA PCM capture ioctls, format/channel/rate negotiation |
| **edgerun-alsa-speaker** | ALSA utils, aplay, amixer | ✅ Functional — ALSA PCM playback + full mixer control (volume, mute, auto-select Master/Speaker/Headphone) |
| **edgerun-v4l2-camera** | libcamera, GStreamer video | ✅ Functional — V4L2 streaming capture with mmap, QBUF/DQBUF, MJPEG/YUYV/NV12/RGB24/Gray8 |
| **edgerun-linux-wifi** | iw, wpa_supplicant (control plane) | ✅ Functional — nl80211 netlink scan, connect (WPA-PSK/CCMP), disconnect, AP mode, regulatory domain |
| **edgerun-mgmt-bluetooth** | bluetoothctl, btmgmt | ✅ Functional — HCI mgmt socket, powered/discoverable/pairable control, LE scan, connect/pair |
| **edgerun-secret-service** | GNOME Keyring, KeePassXC (D-Bus interface) | ✅ Functional — D-Bus Secret Service daemon, encrypted blob store, biometric sessions, idle auto-lock |
| **edgerun-face-detection** | OpenCV Haar cascades | ✅ Functional — CPU-based integral image + Viola-Jones, bounding boxes, 5-point landmarks, NMS |
| **edgerun-audio-calibration** | REW, Room EQ Wizard | ✅ Functional — sweep-based calibration using ALSA backends, RMS/peak dBFS, clipping detection |
| **edgerun-audio-liveliness** | Voice liveness detection tools | ✅ Functional — VAD, challenge/response, spectral anti-spoofing, latency checks |
| **edgerun-linux-netif** | iproute2 (link layer only) | ⚠️ Partial — sysfs discovery + interface up/down via ioctl. Missing IP/routing/DNS config |
| **edgerun-linux-cec** | cec-utils | ⚠️ Partial — CEC transmit via ioctl (Active Source, Standby, Wake). Missing receive/event loop |
| **edgerun-amd-xdna** | — (AMD NPU SDK) | ⚠️ Partial — DRM ioctls for context/BO/command submission, power modes. Missing result polling |
| **edgerun-drm-display** | libdrm, kmscon | ⚠️ Partial — sysfs discovery only. `present()` returns Unsupported — no modesetting |
| **edgerun-linux-pci** | lspci | ⚠️ Partial — sysfs enumeration only. No config space access or driver binding |
| **edgerun-linux-usb** | lsusb, libusb | ⚠️ Partial — sysfs enumeration only. No USB device I/O |
| **edgerun-linux-nfc** | libnfc | ⚠️ Partial — sysfs adapter enumeration only. No target polling or NDEF read/write |
| **edgerun-linux-npu** | — | ⚠️ Partial — sysfs device info only. No workload submission |
| **edgerun-linux-gpu** | — | ⚠️ Partial — sysfs + DRM node enumeration only. No rendering |
| **edgerun-linux-power** | upower, acpi | ⚠️ Partial — sysfs power supply reads only. No power state control |
| **edgerun-bluetooth** (abstract) | — | Traits only — concrete backend is `edgerun-mgmt-bluetooth` (functional above) |
| **edgerun-wifi** (abstract) | — | Traits only — concrete backend is `edgerun-linux-wifi` (functional above) |
| **edgerun-input** (abstract) | — | Traits only — concrete backend is `edgerun-evdev-input` (functional above) |
| **edgerun-microphone** (abstract) | — | Traits only — concrete backend is `edgerun-alsa-microphone` (functional above) |
| **edgerun-speaker** (abstract) | — | Traits only — concrete backend is `edgerun-alsa-speaker` (functional above) |
| **edgerun-camera-biometrics** (abstract) | — | Traits only — concrete backend is `edgerun-v4l2-camera` (functional above) |
| **edgerun-fingerprint** (abstract) | — | Traits only — concrete backend is `edgerun-goodix-fingerprint` (functional above) |
| **edgerun-npu** (abstract) | — | Traits only — backends are `edgerun-amd-xdna` (partial) and `edgerun-linux-npu` (discovery) |
| **edgerun-display** (abstract) | — | Traits only — backend `edgerun-drm-display` is discovery-only (no modesetting yet) |
| **edgerun-network-interface** (abstract) | — | Traits only — backend `edgerun-linux-netif` is partial (no IP/routing config) |
| **edgerun-usb** (abstract) | — | Traits only — backend `edgerun-linux-usb` is discovery-only |
| **edgerun-pci** (abstract) | — | Traits only — backend `edgerun-linux-pci` is discovery-only |
| **edgerun-remote-capability** | gRPC, RPC frameworks | ⚠️ Partial — Unix/TCP socket transport with serialization + signing, no hardware proxy adapters wired |
| **edgerun-capabilities** | OPA (capability model) | ✅ Trait + enforcement layer — delegates to platform backends |
| **edgerun-hardware-signing** | tpm2-tools + yubikey-manager + Android Keystore | ✅ Unified `MeshSigner` trait with TPM/YubiKey/Android/software feature-gated backends |

### Developer Tools

| Edgerun Component | Replaces | Status |
|---|---|---|
| **edgerun-edit** | sed, awk, string-based code editors for Rust | ✅ Functional — AST-level Rust code editor using syn 2 + prettyplease. Safe fn add/remove/replace, type rename, derive add, use insert |

---

## Workspace Structure (138 Crates)

### Protocol Core

| Crate | Description |
|---|---|
| `edgerun-proto` | 44 proto files → generated `prost` types (CSS 15, HTML 5, ECMAScript 3, DOM 2, Infrastructure 19) |
| `edgerun-core` | Protocol logic: EventEnvelope, command validation, delegation chains, crypto wrappers, varint, fixed-point |
| `edgerun-stream` | Append-only single-writer event log with ECDSA P-256 signing, genesis events, prev_hash chain validation |
| `edgerun-storage` | Durable event log + AES-GCM encrypted blob store + rebuildable SQLite indexes |
| `edgerun-capabilities` | Capability types, validation, constraint system, provider trait |
| `edgerun-capability-policy` | Policy enforcement for capabilities |
| `edgerun-remote-capability` | Remote capability RPC with proxy adapters for all hardware types |

### Browser Engine — Type Crates (generated from proto)

| Crate | Description |
|---|---|
| `edgerun-css` / `edgerun-css-*` | CSS properties, values, box model, sizing, display, text, UI, page, cascade, syntax, writing modes |
| `edgerun-html` | HTML element definitions from WHATWG |
| `edgerun-dom` / `edgerun-dom-events` | DOM tree structure and event types |
| `edgerun-ecmascript` | 127 intrinsics, 121 abstract ops, 27 built-in objects |
| `edgerun-selectors` | CSS Selectors Level 4 |
| `edgerun-media-queries` | CSS media query types |
| `edgerun-flexbox` / `edgerun-grid` | Flexbox and grid layout types |
| `edgerun-animations` / `edgerun-transitions` / `edgerun-transforms` | CSS animations, transitions, transforms |
| `edgerun-fonts` / `edgerun-images` / `edgerun-color` | Font, image, and color space types |
| `edgerun-webidl` / `edgerun-uievents` / `edgerun-xml` | WebIDL, UI events, XML types |
| `edgerun-fetch` / `edgerun-url` / `edgerun-encoding` | Fetch API, URL parsing, text encoding |
| `edgerun-indexeddb` / `edgerun-service-workers` / `edgerun-trusted-types` | Browser storage and security types |
| `edgerun-browser` | Core browser engine types from WHATWG/W3C/ECMA-262 |

### Browser Engine — Behavioral Crates

| Crate | Description |
|---|---|
| `edgerun-rasterizer` | Scanline renderer: fills, borders, gradients, text bitmap, AVX2 SIMD, tile multicore |
| `edgerun-wgpu` | WebGPU fragment shader renderer: painter's algorithm, GPU compute passes |
| `edgerun-css-cascade` | Full CSS cascade engine: specificity, combinators, attr selectors, pseudo-classes |
| `edgerun-css-value-parser` | CSS value parser: 30+ length units, all color formats, calc(), var(), url(). 101 tests |
| `edgerun-css-minifier` | CSS minification via proto canonical forms. 60-70% compression |
| `edgerun-property-graph` | CSS knowledge graph: 98 properties, fluent query API, shorthand expansion |
| `edgerun-a11y-analyzer` | WCAG 2.2 static analysis: contrast, font-size, spacing, animation, focus |
| `edgerun-complexity-analyzer` | CSS complexity scoring: specificity, cascade depth, dead rules |
| `edgerun-replay` | Deterministic render replay: binary format, record/playback |
| `edgerun-rule-optimizer` | CSS optimization: merge selectors, remove dead rules, flatten combinators |
| `edgerun-layout-budget` | Predict rendering cost: calibrated per-phase costs, dirty tracking |
| `edgerun-cascade-debugger` | CLI cascade debugger: rule ranking, per-property winners, inheritance chain |
| `edgerun-render` | Complete HTML/CSS → DOM → cascade → layout → paint → rasterize pipeline |
| `edgerun-demo` / `edgerun-demo-wgpu` | Demo apps rendering HTML+CSS to PNG (CPU and GPU paths) |
| `edgerun-render-proof` | CPU vs GPU render proof: pixel-by-pixel comparison, fuzz corpus |
| `edgerun-incremental-layout` | Incremental layout: recompute only affected subtree |

### Networking & Infrastructure

| Crate | Description |
|---|---|
| `edgerun-http` | HTTP/1.1, HTTP/2, HTTP/3 with HPACK/QPACK, stream multiplexing, flow control |
| `edgerun-tls` | TLS implementation |
| `edgerun-dns` | DNS server implementation |
| `edgerun-dhcp` / `edgerun-dhcpv6` | DHCP client/server, DHCPv6 |
| `edgerun-net` | Unified DNS + DHCP server binary |
| `edgerun-config` | Configuration management |
| `edgerun-fetch` | Fetch API implementation |

### Mesh Networking

| Crate | Description |
|---|---|
| `edgerun-mesh` | Identity-routed mesh: signed frames, routing table, peer discovery |
| `edgerun-mesh-link` | Link layer: raw Ethernet + UDP |
| `edgerun-mesh-session` | Encrypted session layer |
| `edgerun-mesh-capability` | Capability discovery/exchange over mesh |
| `edgerun-mesh-daemon` | Full mesh networking daemon |

### Hardware Signing & Identity

| Crate | Description |
|---|---|
| `edgerun-hardware-signing` | Abstract `MeshSigner` trait with feature-gated backends |
| `edgerun-tpm` | TPM 2.0 signing backend |
| `edgerun-yubikey` | YubiKey PIV/APDU signing backend |
| `edgerun-android-keystore` | Android Keystore signing backend |
| `edgerun-fingerprint` | Abstract fingerprint types |
| `edgerun-goodix-fingerprint` | Goodix fingerprint sensor implementation |

### Linux System & Inventory

| Crate | Description |
|---|---|
| `edgerun-linux-sysfs` | Low-level sysfs access via libc |
| `edgerun-linux-pci` / `edgerun-pci` | PCI device enumeration + abstract types |
| `edgerun-linux-usb` / `edgerun-usb` | USB device enumeration + abstract types |
| `edgerun-linux-netif` / `edgerun-network-interface` | Network interface enumeration + abstract types |
| `edgerun-linux-wifi` / `edgerun-wifi` | WiFi management + abstract types |
| `edgerun-linux-nfc` / `edgerun-nfc` | NFC device access + abstract types |
| `edgerun-linux-npu` / `edgerun-npu` | NPU access + abstract types |
| `edgerun-linux-gpu` / `edgerun-gpu` | GPU device access + abstract types |
| `edgerun-linux-cec` / `edgerun-cec` | HDMI CEC access + abstract types |
| `edgerun-linux-power` / `edgerun-power` | Power management + abstract types |
| `edgerun-machine-report` | Machine capability reporting |

### Media & Input

| Crate | Description |
|---|---|
| `edgerun-input` | Abstract input device capability trait |
| `edgerun-evdev-input` | Linux evdev input device (keyboard, mouse, touchpad) |
| `edgerun-microphone` / `edgerun-alsa-microphone` | Abstract mic trait + ALSA capture |
| `edgerun-speaker` / `edgerun-alsa-speaker` | Abstract speaker trait + ALSA playback |
| `edgerun-camera-biometrics` / `edgerun-v4l2-camera` | Camera biometric types + V4L2 capture |
| `edgerun-display` / `edgerun-drm-display` | Abstract display trait + DRM/KMS backend |
| `edgerun-bluetooth` / `edgerun-mgmt-bluetooth` | Abstract Bluetooth + BlueZ mgmt API |
| `edgerun-biometrics` | Abstract biometrics types |
| `edgerun-audio-calibration` | Audio calibration utilities |
| `edgerun-audio-liveliness` | Audio liveness detection |
| `edgerun-face-detection` | Face detection processing |
| `edgerun-amd-xdna` | AMD XDNA NPU support |
| `edgerun-secret-service` | Linux secret service (freedesktop.org) |

### Runtime & Container

| Crate | Description |
|---|---|
| `edgerun-rt` | epoll-based async runtime: reactor, workers, blocking pool, channels, async TCP/UDP |
| `edgerun-json` | Zero-dependency JSON (crates.io v1.0.149) |
| `edgerun-log` | Minimal structured logging |
| `edgerun-crypto` | Single crypto boundary: 40+ upstream crates aggregated |
| `edgerun-oci-runtime` | OCI container runtime: cgroups v2, namespaces, pivot_root, seccomp-BPF |
| `edgerun-oci-registry` | OCI registry client |
| `edgerun-compositor` | Wayland compositor: zero external C dependencies |
| `edgerun-virtual-disk` | Virtual disk storage with NBD support |

### Node Daemon & Testing

| Crate | Description |
|---|---|
| `edgerun-node` | `edgerund` binary — mesh networking, capability discovery, hardware signing, OCI workloads |
| `edgerun-e2e` | End-to-end test suite (software only) |
| `edgerun-e2e-capability` | Hardware capability E2E tests (requires devices) |
| `edgerun-conformance` | Auto-generated conformance tests from proto definitions (42 tests) |
| `edgerun-fuzz` | Proto-driven fuzz testing via libfuzzer-sys |
| `edgerun-bench` / `edgerun-bench-correctness` | Benchmark suite |

### Developer Tools

| Crate | Description |
|---|---|
| `edgerun-edit` | AST-level Rust code editor: `dump`, `list`, `find-fn`, `replace-fn-body`, `add-fn`, `rename-type`, `add-use`, `add-derive`, `remove-fn` |
| `edgerun-spec-watch` | Monitor proto files for spec changes, baseline/save state |
| `edgerun-proto-watch` | Proto file change monitoring |

---

## Binaries

| Binary | Crate | What It Does |
|---|---|---|
| `edgerund` | edgerun-node | Mesh networking daemon with capability discovery and hardware signing |
| `edgerun-net` | edgerun-net | DNS + DHCP server (replaces dnsmasq/BIND/ISC DHCP) |
| `edgerun-oci` | edgerun-oci-runtime | OCI container runtime (replaces runc) |
| `edgerun-compositor` | edgerun-compositor | Wayland compositor (replaces sway/hyprland) |
| `edgerun-demo` | edgerun-demo | HTML+CSS → PNG via CPU rasterizer |
| `edgerun-demo-wgpu` | edgerun-demo-wgpu | HTML+CSS → PNG via GPU (WebGPU) |
| `edgerun-edit` | edgerun-edit | AST-level Rust code editor |
| `edgerun-cascade-debugger` | edgerun-cascade-debugger | CLI CSS cascade resolution debugger |
| `h2spec-server` | edgerun-http | HTTP/2 spec compliance test server |
| `edgerun-secret-service` | edgerun-secret-service | Linux secret service daemon |
| `edgerun-render-proof` | edgerun-render-proof | CPU vs GPU render proof tool |
| `edgerun-spec-watch` | edgerun-spec-watch | Spec change detector |
| `edgerun-proto-watch` | edgerun-proto-watch | Proto file watcher |

Plus 30+ hardware inventory CLI tools (`linux-pci-tool`, `linux-usb-tool`, `linux-netif-tool`, `linux-wifi-tool`, `linux-nfc-tool`, `linux-npu-tool`, `linux-gpu-tool`, `linux-cec-tool`, `linux-power-tool`, `evdev-input-tool`, `v4l2-camera-tool`, `alsa-mic-tool`, `alsa-speaker-tool`, `amd-xdna-tool`, `goodix-tool`, `mgmt-bluetooth-tool`, `audio-calibration-tool`).

---

## Key Design Decisions

1. **Proto is the single source of truth** — 44 `.proto` files define the entire web platform type system AND the distributed protocol. Generated via `buf generate`.
2. **All type crates are `no_std` with `alloc`** — runs on bare metal, no OS needed.
3. **Behavioral code is generated from proto data** — border patterns, color LUTs, blend modes, gradient math, font rasterization all come from proto definitions, not handwritten.
4. **Single crypto boundary** — all crypto flows through `edgerun-crypto`. No crate can import ring, openssl, or sodiumoxide directly.
5. **`serde_json` is banned** — only `edgerun-json` (vendored, published on crates.io) or protobuf for serialization.
6. **Custom async runtime** — `edgerun-rt` is an epoll-based runtime with reactor + worker threads, not tokio.
7. **Identity-routed mesh** — every node's ECDSA P-256 public key IS its network address.
8. **Append-only event log** — single-writer streams with cryptographic hash chains (like a blockchain per stream).
9. **Hardware signing mandatory** — private keys NEVER leave secure hardware (TPM, YubiKey, Android Keystore).
10. **Dual renderers (CPU + GPU)** — enables deterministic proof and visual regression detection.
11. **Zero external C dependencies for compositor** — direct Linux kernel syscalls only.

---

## Quick Start

```bash
# Check all crates compile
cargo check --workspace

# Run tests (197 passing)
cargo test --workspace

# Run clippy
cargo clippy --workspace --all-targets

# Render HTML+CSS to PNG (CPU)
cargo run -q -p edgerun-demo

# Render HTML+CSS to PNG (GPU)
cargo run -q -p edgerun-demo-wgpu

# Run the CSS cascade debugger
cargo run -q -p edgerun-cascade-debugger

# Run the DNS+DHCP server
cargo run -q -p edgerun-net -- --config config.yaml

# Run the OCI container runtime
cargo run -q -p edgerun-oci-runtime --bundle /path/to/bundle

# Run the mesh daemon
cargo run -q -p edgerun-node --bin edgerund -- init --config node.yaml
cargo run -q -p edgerun-node --bin edgerund -- run --config node.yaml
```

---

## Test Summary

| Area | Tests | Status |
|---|---|---|
| CSS value parser | 101 | ✅ |
| Computed style | 17 | ✅ |
| CSS minifier | 6 | ✅ |
| Layout budget | 4 | ✅ |
| A11y analyzer | 6 | ✅ |
| Complexity analyzer | 3 | ✅ |
| Replay engine | 5 | ✅ |
| Rule optimizer | 5 | ✅ |
| Incremental layout | 3 | ✅ |
| Render proof | 5 | ✅ |
| Conformance | 42 | ✅ |
| **Total** | **197** | **All passing** |

---

## Known Gaps

See [docs/capability-gap-analysis.md](docs/capability-gap-analysis.md) for the full breakdown. High-level:

- **P0 (1 remaining):** `requested_assurance` field ignored during command validation — assurance claims never generated or evaluated
- **P1 (5 remaining):** WorkSettlement bilateral signing missing, ComputeAdvertisement not published, query signatures not verified, query proof generation missing, ACTION_STARTED not emitted
- **P2 (3 remaining):** Secret management/RelayEnvelope/IdentityRecord/StoreObject messages have no handlers, storage chunking/tiers/object descriptors not implemented, fetch queue doesn't decode/store results
- **Rendering:** No text shaping (HarfBuzz), no flexbox/grid layout, no image decoding, no media queries, no JavaScript engine
- **Hardware:** Abstract trait crates (`edgerun-wifi`, `edgerun-bluetooth`, `edgerun-input`, etc.) not wired to their functional platform backends. Platform backends for WiFi (nl80211), Bluetooth (HCI), audio (ALSA), camera (V4L2), input (evdev), fingerprint (Goodix), TPM, YubiKey are all fully functional I/O — just need wiring through the trait layer. Display (DRM), PCI, USB, NFC, NPU, GPU are discovery-only.

---

## License

MIT
