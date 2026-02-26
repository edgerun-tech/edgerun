# Remove Solana Dependencies From Scheduler and Worker v1 (2026-02-26)

## Goal
Remove direct Rust crate dependencies on `solana-client`, `solana-sdk`, and `solana-system-interface` from:
- `crates/edgerun-scheduler`
- `crates/edgerun-worker`

## Non-Goals
- Preserve on-chain transaction construction/submission features in these crates.
- Preserve scheduler on-chain account discovery/sync while Solana crates are absent.
- Change control-plane API schemas.

## Security and Constraints
- Preserve existing worker/client signature verification semantics for control-plane payloads.
- Keep scheduler/worker deterministic and compile-safe without hidden runtime panics.
- If chain-specific features are requested via env flags, fail/disable explicitly with clear logs.

## Acceptance Criteria
1. `edgerun-scheduler/Cargo.toml` has no `solana-*` deps.
2. `edgerun-worker/Cargo.toml` has no `solana-*` deps.
3. Workspace builds and tests pass under standard validation commands.
4. Scheduler and worker still serve core off-chain control-plane flows.
5. Chain-specific paths are explicitly disabled/no-op with observable warnings/errors.

## Rollout
- Ship as dependency reduction/refactor.
- Keep chain env flags recognized but non-operative in these binaries.

## Rollback
- Revert this change set and restore Solana deps plus prior chain integration functions.
