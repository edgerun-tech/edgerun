# Edgerun Core Documentation

This index points at the documentation that is still relevant to the current
workspace. Some files in `docs/` are historical design notes or copied upstream
specification references; verify implementation details against the crate source
and root `Cargo.toml`.

## Start Here

- [Root README](../README.md) - current workspace overview and build commands.
- [Crate layout](../crates/README.md) - crate families under `crates/`.
- [Protocol README](../proto/README.md) - protobuf source layout.
- [Core protocol single-file spec](../edgerun_core_protocol_v0_single_file.md)
  - protocol reference snapshot.

## Component Documentation

- [HTTP](http/README.md) - HTTP/1.1, HTTP/2, HTTP/3 implementation notes.
- [HTTP conformance report](http/CONFORMANCE_REPORT.md) - HTTP test status.
- [h2spec analysis](http/H2SPEC_ANALYSIS.md) - HTTP/2 conformance notes.
- [TLS](tls/README.md) - TLS 1.2/1.3 crate notes.
- [OCI runtime](oci-runtime/README.md) - OCI runtime design and usage.
- [Virtual disk](virtual-disk/README.md) - virtual disk and NBD support.
- [Remote capability](remote-capability/README.md) - remote capability transport.
- [Fuzzing](fuzz-README.md) - fuzz target notes.
- [TPM overview](security/tpm.md) - TPM workflows.
- [TPM implementation notes](security/tpm-implementation-notes.md) - TPM details.

## Design Notes

- [Machine daemon](design/machine_daemon.md)
- [Compute marketplace](design/compute-marketplace.md)
- [Capability gap analysis](design/capability-gap-analysis.md)
- [Android hardware](design/android-hardware.md)
- [Goodix Framework 13 hardware test](design/goodix_framework13_hardware_test.md)

## RFCs

The RFCs are design documents. They may describe planned or historical browser
and web-platform work that is not present as workspace members today.

| RFC | Topic |
|---|---|
| [RFC-0000](rfc/RFC-0000-overview.md) | Overview |
| [RFC-0001](rfc/RFC-0001-protocol-core.md) | Protocol core |
| [RFC-0002](rfc/RFC-0002-trust-delegation.md) | Trust and delegation |
| [RFC-0003](rfc/RFC-0003-access-query.md) | Access, query, and federation |
| [RFC-0004](rfc/RFC-0004-networking-mesh.md) | Networking and mesh |
| [RFC-0005](rfc/RFC-0005-rendering-architecture.md) | Rendering architecture |
| [RFC-0006](rfc/RFC-0006-html-parser.md) | HTML parser |
| [RFC-0007](rfc/RFC-0007-css-cascade.md) | CSS cascade |
| [RFC-0008](rfc/RFC-0008-layout-engine.md) | Layout engine |
| [RFC-0009](rfc/RFC-0009-cpu-rasterizer.md) | CPU rasterizer |
| [RFC-0010](rfc/RFC-0010-gpu-renderer.md) | GPU renderer |
| [RFC-0011](rfc/RFC-0011-web-platform.md) | Web platform crates |
| [RFC-0012](rfc/RFC-0012-hardware-discovery.md) | Hardware discovery |
| [RFC-0013](rfc/RFC-0013-audio-biometrics.md) | Audio, biometrics, and input |
| [RFC-0014](rfc/RFC-0014-analysis-tools.md) | Analysis and optimization tools |
| [RFC-0015](rfc/RFC-0015-codegen-pipeline.md) | Code generation pipeline |

## Specification References

- [Fetch spec snapshot](fetch_spec.md)
- [Encoding spec snapshot](encoding_spec.md)
- [URL spec snapshot](url_spec.md)
- [HTML spec snapshot](html_spec.md)
- [ECMAScript spec snapshot](ecmascript_spec.md)
- [WebIDL spec snapshot](webidl_spec.md)
- [UI Events spec snapshot](uievents_spec.md)
- [Wayland protocol specs](wayland/protocol-specs/README.md)

## Quick Facts

| Metric | Current value |
|---|---:|
| Workspace members in root `Cargo.toml` | 109 |
| Crate directories under `crates/` | 109 |
| Files under `proto/` including README | 47 |
| Crates with `no_std` or `cfg_attr(..., no_std)` in `src/lib.rs` | 104 |

These numbers were refreshed from the local tree during this documentation
update.
