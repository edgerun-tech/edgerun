# Edgerun Reference Core — Documentation

## RFCs (Request for Comments)

Design documentation for all 136 crates, organized by domain.

### Protocol Fabric
| RFC | Title | Status |
|-----|-------|--------|
| [RFC-0001](rfc/RFC-0001-protocol-core.md) | Protocol Core v0 | Identity, streams, objects, canonicalization, storage |
| [RFC-0002](rfc/RFC-0002-trust-delegation.md) | Trust & Delegation | Capabilities, delegation, revocation, assurance |
| [RFC-0003](rfc/RFC-0003-access-query.md) | Access, Query & Federation | Snapshots, queries, federation |
| [RFC-0004](rfc/RFC-0004-networking-mesh.md) | Networking & Mesh | Sessions, routing, relay, mesh networking |

### Rendering Engine
| RFC | Title | Status |
|-----|-------|--------|
| [RFC-0005](rfc/RFC-0005-rendering-architecture.md) | Rendering Architecture | Spec-to-code pipeline, dual CPU/GPU |
| [RFC-0006](rfc/RFC-0006-html-parser.md) | HTML Parser | WHATWG tokenizer, tree builder, entities |
| [RFC-0007](rfc/RFC-0007-css-cascade.md) | CSS Cascade | Full cascade + simple cascade, property graph |
| [RFC-0008](rfc/RFC-0008-layout-engine.md) | Layout Engine | Block layout, incremental, budget estimator |
| [RFC-0009](rfc/RFC-0009-cpu-rasterizer.md) | CPU Rasterizer | Scanline, gradients, borders, SIMD, tiles |
| [RFC-0010](rfc/RFC-0010-gpu-renderer.md) | GPU Renderer (wgpu) | WebGPU, WGSL shaders, GPU CSS cascade |

### Web Platform & Hardware
| RFC | Title | Status |
|-----|-------|--------|
| [RFC-0011](rfc/RFC-0011-web-platform.md) | Web Platform Crates | HTTP, TLS, ECMAScript, Fetch, DOM, etc. |
| [RFC-0012](rfc/RFC-0012-hardware-discovery.md) | Hardware Discovery | GPU, PCI, USB, netif, sysfs, NPU |
| [RFC-0013](rfc/RFC-0013-audio-biometrics.md) | Audio, Biometrics & Input | ALSA, camera, fingerprint, evdev |

### Tooling & Codegen
| RFC | Title | Status |
|-----|-------|--------|
| [RFC-0014](rfc/RFC-0014-analysis-tools.md) | Analysis & Optimization | A11y, complexity, CSS minifier, replay |
| [RFC-0015](rfc/RFC-0015-codegen-pipeline.md) | Code Generation Pipeline | buf, html-codegen, generate-* tools |

## Component Documentation

### HTTP/2 + HTTP/3
- [edgerun-http Overview](http/README.md) — HTTP/1.1, HTTP/2 (from scratch), HTTP/3 (QUIC, 371 tests)
- [HTTP Conformance Report](http/CONFORMANCE_REPORT.md) — 730+ test results across HPACK, H2 frames, semantics, TLS
- [h2spec Analysis](http/H2SPEC_ANALYSIS.md) — Systematic h2spec conformance remediation (131/140 pass)

### OCI Runtime
- [edgerun-oci-runtime](oci-runtime/README.md) — Full OCI v1.0.2 container runtime, 149 tests, zero deps

### TLS
- [edgerun-tls](tls/README.md) — TLS 1.2/1.3 client, zero external deps

### Virtual Disk
- [edgerun-virtual-disk](virtual-disk/README.md) — Disk images (raw/qcow2/vhd/vhdx), NBD server, block protocol

### Compositor
- [edgerun-compositor](compositor/PROTOCOL_CONFORMANCE.md) — Wayland compositor, 32 globals, 100% conformance

### Remote Capability
- [edgerun-remote-capability](remote-capability/README.md) — TCP/Unix socket capability transport

### Fuzz Testing
- [Fuzz Infrastructure](fuzz-README.md) — 75 targets, 804 seed corpus files

### Hardware
- [edgerun-gpu](gpu-README.md) — GPU types and capability descriptors
- [edgerun-linux-gpu](linux-gpu-README.md) — Linux sysfs/DRM GPU discovery

## Design Documents

- [Android Hardware Support](design/android-hardware.md) — Android backend implementation
- [Capability Gap Analysis](design/capability-gap-analysis.md) — Implementation status & gaps (2026-04-10)
- [Compute Marketplace](design/compute-marketplace.md) — Design document
- [Machine Daemon (edgerund)](design/machine_daemon.md) — Daemon goal and architecture
- [Goodix Framework 13 Test](design/goodix_framework13_hardware_test.md) — Hardware test status

## Security
- [TPM Overview](security/tpm.md) — TPM workflows
- [TPM Implementation Notes](security/tpm-implementation-notes.md) — Implementation details

## Wayland Protocol Specs
- [protocol-specs/](wayland/protocol-specs/) — Upstream XML protocol specs for reference

## Specifications

- [edgerun Core Protocol v0](../edgerun_core_protocol_v0_single_file.md) — 3295-line protocol specification
- [DESIGN.md](../DESIGN.md) — Architecture design and journey (historical reference)
- [PHASE1_WGSL_DESIGN.md](../PHASE1_WGSL_DESIGN.md) — Proto → WGSL code generation
- [HTML_PARSER_DESIGN.md](../HTML_PARSER_DESIGN.md) — Generated HTML parser design

## Quick Stats

| Metric | Value |
|--------|-------|
| Total crates | 137 (in workspace members) |
| Total crate directories | 103 |
| Total proto files | 44 |
| Files with `#[test]` | 268 |
| Integration test files | 142 |
| Unit test modules (`mod tests`) | 241 |
| Go codegen tools (cmd/) | 29 (13 extractors, 11 generators, 5 CLI/misc) |
| Conformance coverage | 7.7% (tested across 40 proto domains) |
| TODOs in codebase | 13 (DNS TLS wrapper, Android GPU/display stubs, OCI checkpoint, exec cgroup, storage index rebuild, qpack optimizations, DOCTYPE node, image decode stub) |
| Shader files (WGSL) | 2 (render.wgsl: 823 lines, layout.wgsl) |
| HTML element catalog | 108 elements |
| Fuzz corpus files | 804 |
