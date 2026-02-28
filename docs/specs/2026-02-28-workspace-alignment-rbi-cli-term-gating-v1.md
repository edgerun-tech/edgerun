# Workspace Alignment: RBI/CLI Removal + Term Feature Gate (V1)

## Goal
- Remove crates that are currently misaligned with local-first control-plane priorities:
  - `edgerun-rbi-gateway`
  - `edgerun-cli`
- Keep terminal capability available, but behind an explicit compile-time feature gate so it is opt-in.

## Non-Goals
- Redesigning terminal protocol/UI behavior.
- Replacing CLI behavior with a new operator surface.
- Changing scheduler/worker/node-manager business logic.

## Security / Constraints
- Preserve fail-closed behavior in active control-plane paths.
- Do not introduce fallback-only routes.
- Keep workspace deterministic and compile-clean with default features.

## Changes
1. Remove `edgerun-rbi-gateway` from workspace members and delete crate files.
2. Remove `edgerun-cli` from workspace members and delete crate files.
3. Add `term` feature gates to terminal binary crates:
   - `edgerun-term-server`
   - `edgerun-term-native`
   with `default = []`, and `required-features = ["term"]` on binary targets.

## Acceptance Criteria
- `cargo check --workspace` passes.
- `cargo clippy --workspace --all-targets -- -D warnings` passes.
- `cargo test --workspace` passes.
- No remaining workspace references to removed crates.

## Rollout / Rollback
- Rollout: merge in one change set, default builds no longer include RBI/CLI or term binaries.
- Rollback: restore removed crates and workspace entries from git history; remove `required-features` gating in term Cargo manifests.
