# Term Native Removal / Wayland-Free Workspace V1

## Goal
- Remove `edgerun-term-native` from the workspace so it is not built or validated as part of workspace operations.
- Remove Wayland dependencies from active workspace term crates.

## Non-goals
- No deletion of `crates/edgerun-term-native` source directory in this phase.
- No feature redesign for terminal UX behavior.

## Security and constraints
- Keep changes minimal and deterministic.
- Preserve buildability of remaining term crates.
- Ensure no active workspace crate pulls `wayland*` crates.

## Acceptance criteria
- Workspace root `Cargo.toml` no longer lists `crates/edgerun-term-native`.
- `edgerun-term-ui` no longer enables `winit` Wayland features.
- `cargo tree -p edgerun-term-ui | rg wayland` returns no matches.
- Targeted term crate checks/tests pass.

## Rollout
1. Remove `edgerun-term-native` from workspace members.
2. Update `edgerun-term-ui` dependency features for `winit`.
3. Validate dependency graph and term crate builds/tests.

## Rollback
- Re-add `crates/edgerun-term-native` to workspace members.
- Re-enable Wayland features in `edgerun-term-ui`.
