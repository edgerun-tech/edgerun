# SOLANA_ANCHOR_WORKSPACE_DETACH_V1

## Goal
Remove Anchor/Solana coupling from the main Rust workspace surface while preserving runtime Solana behavior in worker/scheduler and leaving frontend unchanged.

## Non-Goals
- No changes to `frontend/` behavior or dependency graph.
- No changes to on-chain program source under `program/` in this step.
- No redesign of event bus payload schemas in this step.

## Security and Constraint Requirements
- Preserve existing runtime behavior for scheduler/worker Solana interactions.
- Ensure `edgerun-cli` no longer requires local Anchor/Solana toolchain to run core workspace operations.
- Detach vanity generator crates from the root workspace so workspace-level checks/builds do not pull them by default.
- Keep deterministic build behavior and avoid new package managers (`bun` remains JS standard).

## Acceptance Criteria
1. Root `Cargo.toml` no longer includes vanity generator crates as workspace members.
2. Root `Cargo.toml` no longer exports Solana dependencies via `[workspace.dependencies]`.
3. `edgerun-worker` and `edgerun-scheduler` compile with explicit local Solana dependency versions.
4. `edgerun-cli` has no `Program` command surface and no Anchor/Solana CLI checks/tasks.
5. CLI task/CI routing no longer references program-localnet or anchor-based program tasks.
6. `frontend/` remains untouched.
7. Remove unused Solana crate dependencies where code no longer requires them (e.g. worker `solana-system-interface`).
8. Vanity generator payload no longer requires Solana SDK in test-only paths when equivalent local key validation is available.

## Rollout
- Phase 1: Remove workspace-level coupling and CLI command/task wiring.
- Phase 2: Validate Rust workspace checks/tests and CLI compile path.
- Phase 3: Move worker on-chain submission out-of-process: worker publishes signed execution/result artifacts and no longer sends Solana transactions directly.
- Phase 4: Scheduler no longer auto-submits transactions; it emits transaction artifacts only for an external subscriber/submission service.
- Phase 5: Scheduler consumes Solana job-status updates from event bus payloads and stops direct RPC polling for on-chain status.
- Phase 6: Follow-up event bus subscriber owns Solana transaction submission lifecycle.

## Rollback
- Reintroduce removed workspace members/dependencies and restore CLI program command/task wiring from git history if needed.
