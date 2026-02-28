# 2026-02-28 Local-First Solana Removal Phase 3 (v1)

## Goal
- Remove active Solana runtime coupling from scheduler control-plane execution paths so local-first operation does not depend on chain RPC availability.
- Remove local user-stack automation that installs/starts `solana-test-validator`.
- Keep fail-closed security semantics for TPM/boot policy and existing scheduler request validation paths.

## Non-Goals
- Remove the `program/` Solana smart-contract workspace in this phase.
- Redesign frontend chain/status terminology in this phase.
- Replace chain-oriented state fields in persisted scheduler snapshots in this phase.

## Security / Constraints
- Scheduler must continue to enforce existing authentication/signature policies unrelated to chain RPC.
- Removal must not relax attestation validation used for worker result quorum decisions.
- Local-first behavior must degrade safely: chain-dependent artifact generation remains disabled rather than partially enabled.
- Preserve deterministic startup without background RPC polling loops.

## Acceptance Criteria
1. `edgerun-scheduler` no longer reads `EDGERUN_SCHEDULER_REQUIRE_CHAIN_CONTEXT` or `EDGERUN_SCHEDULER_CHAIN_AUTO_SUBMIT`.
2. Scheduler startup does not initialize chain context and does not initialize chain progress/event-bus sinks.
3. Scheduler housekeeping loop no longer runs chain-backed discovery/result sync/reconciliation tasks.
4. `scripts/systemd/install-user-services.sh` no longer validates `EDGERUN_SCHEDULER_REQUIRE_CHAIN_CONTEXT` and no longer installs `solana-test-validator.service`.
5. `scripts/systemd/start-user-stack.sh` no longer enables `solana-test-validator.service`.
6. Scheduler env templates/profiles no longer define `EDGERUN_SCHEDULER_REQUIRE_CHAIN_CONTEXT`.
7. `scripts/systemd/README.md` no longer documents local solana-test-validator setup for the user stack.

## Rollout / Rollback
- Rollout: apply phase-3 changes after phase-1 and phase-2; run scheduler/worker checks and workspace check.
- Rollback: revert this phase’s scheduler/systemd commits; previous optional-chain behavior can be restored without data migration.

## Alignment Notes
- This phase intentionally leaves chain-oriented types and parsing helpers in source where required for compatibility and to reduce migration risk.
