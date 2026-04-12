# RFC-0000: Edgerun Reference Core — System Overview

**Status:** Working Draft
**Date:** 2026-04-12
**Author:** System-generated from codebase audit
**Supersedes:** None

---

## Abstract

Edgerun Reference Core implements **two independent systems** within a single Rust workspace:

1. **The Protocol Fabric** — A distributed, identity-based, append-only information fabric implementing the edgerun Core Protocol v0 specification. It provides single-writer streams, capability-based delegation, immutable objects, snapshots, federated queries, and mesh networking.

2. **The Rendering Engine** — A spec-driven HTML/CSS rendering pipeline where W3C/WHATWG specifications are extracted into Protocol Buffers, then used to generate behavioral code (parsers, rasterizers, WGSL shaders, layout engines). No browser generates its renderer from machine-readable spec data.

The unifying principle across both systems is **proto-as-single-source-of-truth**: types, enums, state machines, and behavioral data are defined in `.proto` files, then code-generated into Rust crates via `buf generate` + custom generators.

---

## Architecture at a Glance

```
┌─────────────────────────────────────────────────────────────────────┐
│                     EDGERUN REFERENCE CORE                          │
│                        137 Rust Crates                              │
├──────────────────────────┬──────────────────────────────────────────┤
│   PROTOCOL FABRIC        │     RENDERING ENGINE                     │
│   (~40 crates)           │     (~97 crates)                         │
│                          │                                          │
│  Spec: core-protocol-v0  │     Specs: W3C/WHATWG HTML/CSS/DOM/ECMA │
│  3295 lines              │     44 proto files                       │
│  7 packages              │     Go codegen pipeline                  │
│  27 message families     │     2 code generators (buf + html-codegen)│
│                          │                                          │
│  Identity + Trust        │     HTML Parser (WHATWG generated)       │
│  Streams + Events        │     CSS Cascade (proto-driven)           │
│  Commands + Delegation   │     Layout Engine (block, flex, grid)    │
│  Objects + Snapshots     │     CPU Rasterizer (scanline, SIMD)      │
│  Queries + Federation    │     GPU Rasterizer (wgpu, WGSL)          │
│  Mesh Networking         │     Deterministic Proof (CPU = GPU)      │
│  Capability System       │     Analysis Tools (a11y, complexity)    │
│  Hardware Discovery      │     Optimization Tools (minifier, rules) │
└──────────────────────────┴──────────────────────────────────────────┘
```

---

## Core Innovation

### Innovation #1: Spec → Proto → Generated Code

Every behavioral component in the rendering engine is generated from spec data:

| Spec Source | Proto Output | Generated Code |
|-------------|-------------|----------------|
| WHATWG HTML §13.2.5 | `tokenizer_states.proto` (87 states) | `tokenizer.rs` (table-driven FSM) |
| WHATWG HTML §13.2.6 | `tree_builder.proto` (24 modes) | `tree_builder.rs` (insertion rules) |
| WHATWG §13.1.4.22 | `entities.proto` (2,231 entities) | `entity_decoder/` (binary search lookup) |
| W3C CSS properties | `css_properties.proto` (175 props) | CSS cascade + computed style resolution |
| W3C CSS colors | `css_value_types.proto` | `color_convert.rs` (HSL/OKLCh → sRGB) |
| W3C CSS box model | `css_box.proto` | `box_model.rs` (edge values, BFC detection) |
| W3C CSS images | `css_images.proto` | Gradient math in CPU + WGSL |
| Color/border/blend LUTs | Proto data tables | `color_lut.rs` (148 colors), `border_lut.rs` (9 patterns), `blend_lut.rs` (16 modes) |
| CSS properties + box model | Combined proto data | `shaders/render.wgsl` (823 lines, fragment shader) |
| CSS cascade + layout | `css_cascade.proto` + selectors | `shaders/layout.wgsl` (GPU compute pipeline) |

Chrome's HTML parser is ~500,000 lines of hand-written C++. Ours is ~5,600 lines of generated Rust, compiled from spec data, with conformance tests mapped to every spec rule.

### Innovation #2: Dual Renderers with Deterministic Proof

Both CPU and GPU renderers produce pixels from the same inputs. `edgerun-render-proof` verifies byte-for-byte equality across the fuzz corpus, saving mismatch PNGs for investigation.

### Innovation #3: Proto Canonical CSS Encoding

CSS property names encoded as 1-byte varints (98 unique property IDs) instead of ~12-byte average string names. 60-70% size reduction with round-trip encode/decode.

### Innovation #4: GPU CSS Cascade

3-pass GPU compute shader: cascade resolution → style inheritance → height computation. Full per-property cascade with `!important`, specificity, source order tie-breaking — running on the GPU, not CPU.

---

## RFC Index

### Master Documents
| RFC | Title | Scope | Status |
|-----|-------|-------|--------|
| [RFC-0000](RFC-0000-overview.md) | System Overview | This document | ✅ |
| [RFC-0001](RFC-0001-protocol-core.md) | Protocol Core v0 | Identity, streams, events, commands, objects, canonicalization | ✅ |
| [RFC-0002](RFC-0002-trust-delegation.md) | Trust & Delegation | Capabilities, delegation chains, revocation, assurance claims | ✅ |
| [RFC-0003](RFC-0003-access-query.md) | Access, Query & Federation | Query model, snapshots, federated aggregation, proof bundles | ✅ |
| [RFC-0004](RFC-0004-networking-mesh.md) | Networking & Mesh | Sessions, routing, reachability, relay, mesh networking | ✅ |
| [RFC-0005](RFC-0005-rendering-architecture.md) | Rendering Architecture | Spec-to-code pipeline, CPU/GPU dual rendering, conformance | ✅ |
| [RFC-0006](RFC-0006-html-parser.md) | HTML Parser | WHATWG tokenizer, tree builder, entity decoding | ✅ |
| [RFC-0007](RFC-0007-css-cascade.md) | CSS Cascade & Computed Style | Selector matching, cascade resolution, inheritance | ✅ |
| [RFC-0008](RFC-0008-layout-engine.md) | Layout Engine | Block layout, flex, grid, positioning, incremental relayout | ✅ |
| [RFC-0009](RFC-0009-cpu-rasterizer.md) | CPU Rasterizer | Scanline rendering, gradients, borders, SIMD, tile multicore | ✅ |
| [RFC-0010](RFC-0010-gpu-renderer.md) | GPU Renderer (wgpu) | WebGPU pipeline, WGSL shaders, font atlas, GPU layout compute | ✅ |
| [RFC-0011](RFC-0011-web-platform.md) | Web Platform Crates | ECMAScript, fetch, encoding, URL, DOM events, IndexedDB | ✅ |
| [RFC-0012](RFC-0012-hardware-discovery.md) | Hardware Discovery & Linux Backends | GPU, PCI, USB, NFC, WiFi, netif, sysfs, power, NPU | ✅ |
| [RFC-0013](RFC-0013-audio-biometrics.md) | Audio, Biometrics & Input | ALSA, microphone, speaker, camera, fingerprint, face detection | ✅ |
| [RFC-0014](RFC-0014-analysis-tools.md) | Analysis & Optimization Tools | A11y, complexity, CSS minifier, rule optimizer, replay | ✅ |
| [RFC-0015](RFC-0015-codegen-pipeline.md) | Code Generation Pipeline | buf, html-codegen, generate-* tools, proto extraction | ✅ |

---

## Crate Registry

### Protocol Fabric Crates (30)

| Crate | Status | Tests | Description |
|-------|--------|-------|-------------|
| `edgerun-core` | ⚠️ Partial | — | Protocol core types and interfaces |
| `edgerun-proto` | ✅ Functional | — | Generated prost bindings from proto files |
| `edgerun-capabilities` | ✅ Functional | — | Capability descriptor types |
| `edgerun-capability-policy` | ⚠️ Partial | — | Policy evaluation for capabilities |
| `edgerun-remote-capability` | ✅ Functional | — | Remote capability negotiation |
| `edgerun-e2e-capability` | ✅ Functional | — | End-to-end capability tests |
| `edgerun-node` | ✅ Functional | — | Main `edgerund` binary |
| `edgerun-storage` | ✅ Functional | — | Storage abstraction layer |
| `edgerun-stream` | ✅ Functional | — | Stream management and validation |
| `edgerun-rt` | ✅ Functional | — | Async runtime and utilities |
| `edgerun-config` | ✅ Functional | — | Configuration loading |
| `edgerun-log` | ✅ Functional | — | Logging infrastructure |
| `edgerun-mesh` | ⚠️ Partial | — | Mesh networking protocol |
| `edgerun-mesh-link` | ⚠️ Partial | — | Mesh link management |
| `edgerun-mesh-capability` | ⚠️ Partial | — | Mesh capability exchange |
| `edgerun-mesh-daemon` | ⚠️ Partial | — | Mesh daemon process |
| `edgerun-mesh-session` | ⚠️ Partial | — | Mesh session management |
| `edgerun-mgmt-bluetooth` | ⚠️ Partial | — | Bluetooth management |
| `edgerun-oci-runtime` | ⚠️ Partial | — | OCI runtime implementation |
| `edgerun-oci-registry` | ⚠️ Partial | — | OCI registry implementation |
| `edgerun-tftp` | ⚠️ Partial | — | TFTP protocol |
| `edgerun-dns` | ⚠️ Partial | — | DNS resolution |
| `edgerun-dhcp` | ⚠️ Partial | — | DHCP client |
| `edgerun-dhcpv6` | ⚠️ Partial | — | DHCPv6 client |
| `edgerun-network-interface` | ✅ Functional | — | Network interface abstraction |
| `edgerun-linux-netif` | ✅ Functional | — | Linux network interface discovery |
| `edgerun-linux-sysfs` | ✅ Functional | — | Linux sysfs abstraction |
| `edgerun-machine-report` | ✅ Functional | — | Machine capability reporting |
| `edgerun-json` | ✅ Functional | 100+ | JSON serialization (serde_json replacement) |
| `edgerun-crypto` | ✅ Functional | — | Cryptographic boundary (single crate) |

### Rendering Engine Crates (~97)

#### HTML/CSS Core
| Crate | Status | Tests | Description |
|-------|--------|-------|-------------|
| `edgerun-html` | ✅ Generated | — | Generated HTML types from proto |
| `edgerun-css` | ✅ Generated | — | Generated CSS types from proto |
| `edgerun-css-cascade` | ✅ Functional | — | Full CSS cascade with specificity |
| `edgerun-css-value-parser` | ✅ Functional | 101 | Length units, colors, calc(), var(), url() |
| `edgerun-css-values` | ✅ Generated | — | Generated CSS value types |
| `edgerun-css-syntax` | ✅ Generated | — | Generated CSS syntax tokens |
| `edgerun-css-text` | ✅ Generated | — | Generated CSS text properties |
| `edgerun-css-ui` | ✅ Generated | — | Generated CSS UI properties |
| `edgerun-css-page` | ✅ Generated | — | Generated CSS page properties |
| `edgerun-css-box` | ✅ Generated | — | Generated CSS box model types |
| `edgerun-css-sizing` | ✅ Generated | — | Generated CSS sizing properties |
| `edgerun-css-display` | ✅ Generated | — | Generated CSS display types |
| `edgerun-contain` | ✅ Generated | — | Generated CSS containment types |
| `edgerun-writing-modes` | ✅ Generated | — | Generated writing modes types |
| `edgerun-media-queries` | ✅ Generated | — | Generated media query types |
| `edgerun-property-graph` | ✅ Functional | 16 | CSS property knowledge graph (98 props) |
| `edgerun-css-minifier` | ✅ Functional | 5 | Proto-indexed varint encoding (60-70% compression) |
| `edgerun-selectors` | ✅ Generated | — | CSS selectors (pseudo-classes, combinators) |
| `edgerun-dom` | ✅ Generated | — | DOM interfaces as Rust traits |
| `edgerun-dom-events` | ✅ Generated | — | DOM event types |
| `edgerun-xml` | ⚠️ Partial | — | XML parsing |
| `edgerun-webidl` | ⚠️ Partial | — | Web IDL type system |
| `edgerun-uievents` | ✅ Generated | — | UI Events key codes |

#### Layout
| Crate | Status | Tests | Description |
|-------|--------|-------|-------------|
| `edgerun-render` | ✅ Functional | 40+ | Complete CPU HTML/CSS renderer (entry point) |
| `edgerun-layout` *(generated)* | ⚠️ Partial | — | Generated layout crate (superseded by edgerun-render/layout/) |
| `edgerun-flexbox` | ✅ Generated | — | Generated flexbox types |
| `edgerun-grid` | ✅ Generated | — | Generated grid types |
| `edgerun-animations` | ✅ Generated | — | Generated animation types |
| `edgerun-transitions` | ✅ Generated | — | Generated transition types |
| `edgerun-transforms` | ✅ Generated | — | Generated transform types |

#### Rendering
| Crate | Status | Tests | Description |
|-------|--------|-------|-------------|
| `edgerun-rasterizer` | ✅ Functional | — | CPU scanline rasterizer (SIMD, tile, gradients, borders) |
| `edgerun-wgpu` | ✅ Functional | — | WebGPU renderer with GPU CSS cascade + layout compute |
| `edgerun-compositor` | ⚠️ Partial | — | Compositor integration |
| `edgerun-compositor-tests` | ⚠️ Partial | — | Compositor tests |
| `edgerun-render-proof` | ✅ Functional | 0 | CPU vs GPU pixel equality verification (tests pending) |
| `edgerun-color` | ✅ Functional | — | Named colors, color type conversions |
| `edgerun-images` | ✅ Generated | — | Generated image types |
| `edgerun-fonts` | ✅ Generated | — | Generated font types |

#### Web Platform
| Crate | Status | Tests | Description |
|-------|--------|-------|-------------|
| `edgerun-browser` | ⚠️ Partial | — | Browser glue code |
| `edgerun-ecmascript` | ✅ Generated | — | Generated ECMAScript types (127 intrinsics, 121 abstract ops, 27 objects) |
| `edgerun-fetch` | ✅ Generated | — | Generated fetch types |
| `edgerun-encoding` | ✅ Generated | — | Generated encoding types (38 encodings) |
| `edgerun-url` | ✅ Generated | — | Generated URL types |
| `edgerun-indexeddb` | ✅ Generated | — | Generated IndexedDB types |
| `edgerun-service-workers` | ✅ Generated | — | Generated service worker types |
| `edgerun-trusted-types` | ✅ Generated | — | Generated trusted types |
| `edgerun-http` | ⚠️ Partial | — | HTTP implementation (H2 analysis in progress) |
| `edgerun-tls` | ⚠️ Partial | — | TLS implementation (RFC 8446 test vectors) |

#### Demo & Testing
| Crate | Status | Tests | Description |
|-------|--------|-------|-------------|
| `edgerun-demo` | ✅ Functional | — | CPU rendering demo (HTML+CSS → PNG) |
| `edgerun-demo-wgpu` | ✅ Functional | — | GPU rendering demo (HTML+CSS → PNG via wgpu) |
| `edgerun-fuzz` | ⚠️ Partial | — | Fuzz testing infrastructure |
| `edgerun-conformance` | ✅ Functional | 42 | Conformance test suite |
| `edgerun-bench` | ✅ Functional | — | Performance benchmarks |
| `edgerun-bench-correctness` | ✅ Functional | — | Correctness benchmarks |
| `edgerun-e2e` | ✅ Functional | — | End-to-end tests |

#### Analysis Tools
| Crate | Status | Tests | Description |
|-------|--------|-------|-------------|
| `edgerun-cascade-debugger` | ✅ Functional | 0 | CLI cascade resolution debugger |
| `edgerun-complexity-analyzer` | ✅ Functional | 3 | CSS complexity scoring |
| `edgerun-a11y-analyzer` | ✅ Functional | 6 | WCAG 2.2 accessibility conformance |
| `edgerun-rule-optimizer` | ✅ Functional | 5 | CSS rule optimization suggestions |
| `edgerun-replay` | ✅ Functional | 5 | Deterministic binary replay engine |
| `edgerun-spec-watch` | ✅ Functional | — | Spec change detection |
| `edgerun-proto-watch` | ✅ Functional | — | Proto file change detection |

#### Hardware Backends
| Crate | Status | Tests | Description |
|-------|--------|-------|-------------|
| `edgerun-gpu` | ✅ Functional | — | Platform-agnostic GPU types & traits |
| `edgerun-linux-gpu` | ✅ Functional | — | Linux sysfs/DRM GPU discovery |
| `edgerun-linux-pci` | ✅ Functional | — | Linux PCI device discovery |
| `edgerun-linux-usb` | ⚠️ Partial | — | Linux USB device discovery |
| `edgerun-linux-nfc` | ⚠️ Partial | — | Linux NFC device discovery |
| `edgerun-linux-wifi` | ⚠️ Partial | — | Linux WiFi device discovery |
| `edgerun-linux-power` | ⚠️ Partial | — | Linux power management |
| `edgerun-linux-npu` | ⚠️ Partial | — | Linux NPU device discovery |
| `edgerun-linux-cec` | ⚠️ Partial | — | Linux CEC (HDMI Consumer Electronics Control) |
| `edgerun-pci` | ✅ Generated | — | Generated PCI types |
| `edgerun-usb` | ✅ Generated | — | Generated USB types |
| `edgerun-nfc` | ✅ Generated | — | Generated NFC types |
| `edgerun-wifi` | ✅ Generated | — | Generated WiFi types |
| `edgerun-power` | ✅ Generated | — | Generated power management types |
| `edgerun-npu` | ✅ Generated | — | Generated NPU types |
| `edgerun-cec` | ✅ Generated | — | Generated CEC types |
| `edgerun-bluetooth` | ✅ Generated | — | Generated Bluetooth types |
| `edgerun-display` | ✅ Generated | — | Generated display types |
| `edgerun-drm-display` | ⚠️ Partial | — | DRM/KMS display output |
| `edgerun-evdev-input` | ⚠️ Partial | — | Linux evdev input handling |
| `edgerun-input` | ✅ Generated | — | Generated input types |
| `edgerun-touch` | ✅ Generated | — | Generated touch event types |
| `edgerun-amd-xdna` | ⚠️ Partial | — | AMD XDNA NPU support |

#### Audio & Biometrics
| Crate | Status | Tests | Description |
|-------|--------|-------|-------------|
| `edgerun-alsa-microphone` | ⚠️ Partial | — | ALSA microphone capture |
| `edgerun-alsa-speaker` | ⚠️ Partial | — | ALSA speaker output |
| `edgerun-microphone` | ✅ Generated | — | Generated microphone types |
| `edgerun-speaker` | ✅ Generated | — | Generated speaker types |
| `edgerun-audio-calibration` | ⚠️ Partial | — | Audio calibration |
| `edgerun-audio-liveliness` | ⚠️ Partial | — | Audio liveliness detection |
| `edgerun-biometrics` | ⚠️ Partial | — | Biometric abstraction |
| `edgerun-camera-biometrics` | ⚠️ Partial | — | Camera-based biometrics |
| `edgerun-face-detection` | ⚠️ Partial | — | Face detection |
| `edgerun-fingerprint` | ⚠️ Partial | — | Fingerprint sensor abstraction |
| `edgerun-goodix-fingerprint` | ⚠️ Partial | — | Goodix fingerprint driver |

#### Security & Identity
| Crate | Status | Tests | Description |
|-------|--------|-------|-------------|
| `edgerun-tpm` | ⚠️ Partial | — | TPM (Trusted Platform Module) |
| `edgerun-yubikey` | ⚠️ Partial | — | YubiKey integration |
| `edgerun-hardware-signing` | ⚠️ Partial | — | Hardware-backed signing |
| `edgerun-android-keystore` | ⚠️ Partial | — | Android Keystore integration |
| `edgerun-android-hardware` | ⚠️ Partial | — | Android hardware abstraction |
| `edgerun-secret-service` | ⚠️ Partial | — | Secret service (Secrets API compatible) |
| `edgerun-virtual-disk` | ⚠️ Partial | — | Virtual disk management |

#### Utilities
| Crate | Status | Tests | Description |
|-------|--------|-------|-------------|
| `edgerun-edit` | ✅ Functional | — | AST-level Rust code editor (syn 2) |

---

## Key Design Decisions

1. **Proto is the single source of truth** — 40+ proto files define the entire system's type system
2. **Behavioral code is generated, not handwritten** — border patterns, color LUTs, blend modes, gradient math, font rasterization, HTML tokenizer, tree builder — all from proto data
3. **All type crates are `no_std` with `alloc`** — runs on bare metal, no OS needed
4. **Single crypto boundary** — all cryptography flows through `edgerun-crypto`
5. **Dual renderers (CPU + GPU)** — enables deterministic proof and visual regression detection
6. **AST-level code editing** — `edgerun-edit` uses syn 2 to edit Rust code safely
7. **Local-first, partition-tolerant** — the protocol fabric works without global consensus
8. **Receiver-driven retrieval** — no sender-side push guarantees; missing data repaired by query

---

## Current Coverage

- **Conformance dashboard**: 2,888 spec items across 44 proto files — **7.7% tested**
- **Total tests**: 227+ passing across all crates
- **Total proto files**: 44 in `proto/edgerun/v0/`
- **Total Go codegen tools**: 29 (13 extractors, 11 generators, 5 CLI/misc)
- **Corpus files**: 804
- **Shader files**: 2 WGSL (`render.wgsl`: 823 lines, `layout.wgsl`)

---

## See Also

- [edgerun Core Protocol v0](../../edgerun_core_protocol_v0_single_file.md) — 3295-line specification
- [DESIGN.md](../../DESIGN.md) — Architecture design and journey
- [PHASE1_WGSL_DESIGN.md](../../PHASE1_WGSL_DESIGN.md) — Proto → WGSL code generation
- [HTML_PARSER_DESIGN.md](../../HTML_PARSER_DESIGN.md) — Generated HTML parser design
