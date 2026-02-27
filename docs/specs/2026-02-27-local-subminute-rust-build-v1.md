<!-- SPDX-License-Identifier: Apache-2.0 -->

# LOCAL_SUBMINUTE_RUST_BUILD_V1

Date: 2026-02-27
Status: Proposed
Owner: Codex

## Goal
- Reduce local Rust release-like build wall-clock toward ~1 minute on developer machines.
- Keep CI/release correctness intact while improving day-to-day inner-loop throughput.

## Non-Goals
- Replacing production release profile semantics used for shipped artifacts.
- Changing scheduler/worker runtime behavior.
- Introducing non-Rust build orchestration tools.

## Security and Constraint Requirements
- Keep deterministic build configuration committed in-repo.
- Preserve canonical artifact location under `out/`.
- Preserve current CI compatibility and avoid introducing npm/pnpm workflows.

## Acceptance Criteria
- `.cargo/config.toml` enables `mold` linker for Linux Rust builds.
- Workspace `Cargo.toml` defines a fast local profile (`dev-release`) suitable for release-like iterative builds.
- Existing release profile remains available for production-quality outputs.
- Benchmark evidence includes exact before/after commands and timings for scheduler+worker local builds.

## Rollout
- Add linker/profile configuration in one change.
- Re-run timed local builds with the same command shape to measure improvement.

## Rollback
- Revert `.cargo/config.toml` and `Cargo.toml` profile additions.
- Re-run baseline command to confirm previous behavior.
