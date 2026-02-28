# 2026-02-28 Local-First Solana Removal Phase 4 (v1)

## Goal
- Remove remaining unused on-chain compatibility scaffolding from `edgerun-scheduler` introduced for Solana integration.
- Remove broad `#![allow(dead_code)]` suppression by deleting dead chain code paths instead of suppressing warnings.

## Non-Goals
- Remove all chain-related status fields from `JobQuorumState` persisted model in this phase.
- Remove Solana references outside scheduler/runtime core paths (frontend/program workspace/docs) in this phase.

## Security / Constraints
- Preserve fail-closed scheduling and assignment-policy enforcement.
- Preserve current API response shape for `job.create` and `job.status`.
- Keep deterministic local-first behavior: `post_job_tx`/`post_job_sig` remain intentionally empty until new non-chain transport is defined.

## Acceptance Criteria
1. `crates/edgerun-scheduler/src/main.rs` no longer includes `#![allow(dead_code)]`.
2. Scheduler no longer carries unused chain transaction/rpc compatibility structs and helpers (`ChainContext`/`RpcClient` scaffold, post/assign/finalize/cancel/slash tx builders, on-chain parsing/sync helpers).
3. `job_create_inner` returns empty chain artifacts without attempting chain-context/proof construction.
4. Scheduler, worker, and workspace checks pass (`cargo check/clippy/test` commands used in prior phases).

## Rollout / Rollback
- Rollout: apply after phase-3; validate scheduler and workspace build/test.
- Rollback: revert phase-4 commit(s), restoring removed compatibility scaffolding.

## Alignment Notes
- This phase intentionally keeps existing response fields (`post_job_tx`, `post_job_sig`, quorum onchain metadata fields) for compatibility while their generation/sync remains disabled.
