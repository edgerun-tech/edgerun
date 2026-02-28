# Terminal Crates Path Consolidation (V1)

## Goal
- Move terminal-related crates under a single folder root: `crates/edgerun-terminal/`.
- Keep crate package names unchanged (`edgerun-term-*`) to minimize code churn.

## Non-Goals
- Renaming crate/package identifiers.
- Changing terminal runtime behavior or feature semantics.

## Constraints
- Workspace must remain buildable with default features.
- Path dependency updates must be complete and deterministic.

## Acceptance Criteria
1. Terminal crates exist under `crates/edgerun-terminal/*`.
2. Workspace member paths are updated.
3. All path dependencies to terminal crates resolve.
4. Validation passes:
   - `cargo check --workspace`
   - `cargo clippy --workspace --all-targets -- -D warnings`
   - `cargo test --workspace`

## Rollout / Rollback
- Rollout: move directories + update paths in one change set.
- Rollback: move crates back to previous paths and restore Cargo.toml path entries.
