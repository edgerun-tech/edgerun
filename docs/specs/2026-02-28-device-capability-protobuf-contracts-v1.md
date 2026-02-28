# Device Capability Protobuf Contracts V1

## Goal
- Replace JSON contract artifacts for device capability and benchmark outputs with protobuf (`proto3`) binary contracts.
- Keep report semantics consistent across Linux, Android, Windows, router-class systems, and containerized environments.

## Non-goals
- No scheduler integration changes in this phase.
- No transport protocol changes beyond file artifact format.
- No removal of internal Rust-native report structures.

## Security and constraints
- Contracts must be deterministic and backward-compatible by field-number policy.
- Schema evolution must use additive fields and `reserved` markers when removing fields.
- Benchmarks remain performance-focused; capability presence checks stay in capability probe paths.
- Generated artifacts must be written under `out/`.

## Contract scope
1. Capability report contract:
- `CapabilityReportV1`
- `CapabilityReportWithDetailsV1`
- Supporting enums/messages for host, domains, diagnostics, metrics, metadata, and resolved source paths.

2. Benchmark report contract:
- `BenchmarkReportV1`
- `BenchmarkCaseResultV1`
- `DomainAvailabilitySummaryV1`
- `BenchmarkStatusV1`

## Acceptance criteria
- `proto3` schemas are added in-repo and compiled via `prost-build`.
- `edgerun-device-cap-host` can encode:
  - capability report to `capability_report.pb`
  - capability report with details to `capability_report_details.pb`
  - benchmark report to `report.pb`
- Existing benchmark runner writes protobuf artifact by default.
- Rust checks/tests pass for the touched crates.

## Rollout
1. Add `proto` schema and codegen wiring (`build.rs` + generated include module).
2. Add conversion/encoding functions from core/host runtime structs to protobuf messages.
3. Update CLI bins to emit protobuf files in `out/bench/`.
4. Validate with `cargo check`, `cargo clippy`, `cargo test`, and a local run artifact.

## Rollback
- Revert bin output wiring to prior artifact format while keeping probe internals intact.
