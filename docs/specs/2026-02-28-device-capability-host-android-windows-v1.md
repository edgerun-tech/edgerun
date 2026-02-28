# Device Capability Host Adapter Android/Windows V1

## Goal
- Extend `edgerun-device-cap-host` so Android and Windows produce the same core report shape as Linux.
- Preserve deterministic semantics: unsupported probes emit explicit `Unknown`/`None` with diagnostics.

## Non-goals
- No deep vendor SDK integrations (WMI/DXGI/Android JNI HAL) in this phase.
- No scheduler integration changes in this phase.

## Security and constraint requirements
- Probes remain read-only and side-effect free.
- No shelling out to platform commands.
- Platform-specific paths/APIs must be gated with `cfg` and degrade safely.

## Acceptance criteria
- Android branch populates host capabilities/domains/metrics/metadata using `/proc` + `/sys` + cgroup and existing POSIX probes.
- Windows branch populates host capabilities/domains/metrics/metadata via `std` + Win32 APIs (memory/storage baseline).
- Diagnostics and resolved-source details are emitted on both platforms with explicit source attribution.
- Core report schema remains unchanged; outputs are shape-compatible across Linux/Android/Windows.

## Rollout
1. Add Android/Windows host probe branches with shared schema.
2. Validate crate checks/tests on host platform and verify platform-gated code compiles.
3. Extend with deeper GPU/USB/input/output metrics in later phases.

## Rollback
- Remove platform branches and return to Linux-only host probing.
