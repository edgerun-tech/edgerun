# Edgerun Core Documentation

Read the protocol first. Most other docs are either implementation notes,
component notes, generated spec snapshots, or older design documents.

## Core Protocol

- [Core Protocol v0](../edgerun_core_protocol_v0_single_file.md) - normative
  working draft for the identity-based append-only fabric.
- [Protocol source layout](../proto/README.md) - protobuf tree notes.
- `proto/edgerun/v0/common.proto` - shared identifiers, refs, digests,
  signatures, storage/execution/transport classes.
- `proto/edgerun/v0/identity.proto` - identity records.
- `proto/edgerun/v0/trust.proto` - capabilities, delegation, revocation,
  assurance, route trust policy.
- `proto/edgerun/v0/stream.proto` - events, commands, command results, action
  lifecycle, secret-service event payloads.
- `proto/edgerun/v0/object.proto` - logical objects, stored representations,
  chunks, manifests.
- `proto/edgerun/v0/access.proto` - snapshots, queries, result fragments, proof
  objects, aggregates.
- `proto/edgerun/v0/network.proto` - reachability hints, routes, session
  hello/accept, relay envelopes.

## Current Implementation

- [Root README](../README.md) - current code-grounded overview and build caveats.
- [Project state assessment](project-state.md) - honest implemented/partial/
  generated/host-only/bare-target status from the current code.
- [Crate layout](../crates/README.md) - implementation-oriented crate map.
- [Crate organization map](crate-organization.md) - current workspace inventory
  grouped by protocol, runtime, mesh, services, hardware, adapters, and tools.
- [HTTP](http/README.md) - HTTP stack notes.
- [HTTP conformance report](http/CONFORMANCE_REPORT.md)
- [h2spec analysis](http/H2SPEC_ANALYSIS.md)
- [TLS](tls/README.md)
- [OCI runtime](oci-runtime/README.md)
- [Virtual disk](virtual-disk/README.md)
- [Remote capability](remote-capability/README.md)
- [Benchmark methodology](benchmarks/methodology.md) - evidence classes,
  isolation rules, workload rules, and publication guardrails.
- [Email benchmark baseline](email-stack-benchmark-baseline.md) - host-only and
  rootless-Podman benchmark plan and evidence references.
- [Email stack research status](benchmarks/email-stack/research-status-20260430.md) -
  current evidence reading, safe public claim wording, and next measurements.
- [ESP32-S3 Wi-Fi MMIO bring-up](hardware/esp32s3-wifi-mmio.md) - bare-target
  no-blob Wi-Fi diagnostics and current blocker notes.
- [Fuzzing](fuzz-README.md)
- [TPM overview](security/tpm.md)
- [TPM implementation notes](security/tpm-implementation-notes.md)

## Design Notes

These are useful context, but verify claims against code before treating them as
current status.

- [Machine daemon](design/machine_daemon.md)
- [Compute marketplace](design/compute-marketplace.md)
- [Browser nodes and Wasm agents](design/browser-nodes.md)
- [Capability gap analysis](design/capability-gap-analysis.md)
- [Android hardware](design/android-hardware.md)
- [Goodix Framework 13 hardware test](design/goodix_framework13_hardware_test.md)

## RFCs

The RFC directory is mostly design history and planning. Some rendering and web
platform RFCs describe generated schemas or earlier goals, not current
workspace-member implementations.

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

## Specification Snapshots

These files are large upstream/reference snapshots or generated-source inputs.

- [Fetch spec snapshot](fetch_spec.md)
- [Encoding spec snapshot](encoding_spec.md)
- [URL spec snapshot](url_spec.md)
- [HTML spec snapshot](html_spec.md)
- [ECMAScript spec snapshot](ecmascript_spec.md)
- [WebIDL spec snapshot](webidl_spec.md)
- [UI Events spec snapshot](uievents_spec.md)
- [Wayland protocol specs](wayland/protocol-specs/README.md)

## Current Facts From The Tree

| Metric | Value |
|---|---:|
| Protocol draft length | 3299 lines |
| `.proto` files under `proto/edgerun/v0` | 44 |
| Files under `proto/` including README/generated notes | 47 |
| First-level directories under `crates/` | 110 |
| Crate manifests under `crates/` | 110 |
| `cargo metadata --no-deps` packages/members | 110 |
| Root `Cargo.toml` textual member entries | 110 |
| Known inventory quirks | No crate directories without manifests found under `crates/` in this checkout |
