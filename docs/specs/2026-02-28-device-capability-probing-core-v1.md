# Device Capability Probing Core V1

## Goal
- Introduce a deterministic `no_std` Rust crate that provides a canonical capability report foundation for heterogeneous targets.
- Support architecture-gated probing in core for `x86_64` and `aarch64` without requiring OS/runtime facilities.
- Define explicit `Supported`/`Unsupported`/`Unknown` semantics so higher-level host probes can merge richer signals.

## Non-goals
- No attempt to fully discover host/device capabilities requiring syscalls, filesystem access, environment variables, JNI, Win32 APIs, or procfs/sysfs.
- No container/runtime detection in `no_std` core.
- No per-vendor driver or accelerator enumeration in this phase.

## Security and constraint requirements
- Core crate must compile with `#![no_std]` and have zero runtime dependencies.
- Behavior must be deterministic and side-effect free.
- Architecture-specific probing must be isolated behind `cfg(target_arch = ...)` gates.
- Unsupported/unavailable probes must report `Unknown` rather than panicking or fabricating values.
- Preserve portability across `aarch64`, `x86_64`, embedded/router Linux variants, Android, Windows, Linux, and containerized deployments by making platform-dependent fields explicit and nullable via `Unknown`.

## Acceptance criteria
- New crate `crates/edgerun-device-cap-core` exists and is part of the workspace.
- Crate exports:
  - canonical report schema,
  - capability-state enum,
  - probe-source enum,
  - target metadata,
  - device-domain status section for `cpu`, `storage`, `gpu`, `ram`, `usb`, `network`, `input`, `output`, each with `detected`, `available`, `in_use`,
  - `probe_capabilities()` entrypoint.
- `x86_64` probing uses runtime CPUID in `no_std`.
- `aarch64` probing uses compile-time `target_feature` gates in `no_std`.
- Other architectures compile and return a valid report with conservative `Unknown` values where unsupported.
- Tests validate report invariants on supported build targets.
- Core report includes additional deterministic compile-time platform capability signals:
  - target family (`unix`/`windows`/`wasm` style buckets as applicable),
  - atomic width support (`8/16/32/64/ptr`),
  - panic strategy (`abort`/`unwind`/`unknown`).
- Core CPU model includes extended optional crypto/compression/vector signals where directly observable in `no_std` (`x86_64` CPUID and `aarch64` target-feature gates).

## Rollout
1. Add core crate and wire into workspace.
2. Integrate consumers to use core schema as the single capability-report contract.
3. Add future `std` host adapters for Linux/Android/Windows/container-specific probing that merge into core report.

## Rollback
- Remove workspace member and crate directory.
- Revert downstream consumers to previous ad-hoc capability data model.

## Spec alignment notes
- This spec intentionally separates architecture-level probing (`no_std`) from host-environment probing (`std` adapters) to satisfy cross-platform determinism and maintainability.
