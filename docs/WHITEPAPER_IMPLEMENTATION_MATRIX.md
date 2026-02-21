# Whitepaper Implementation Matrix

Reference: `Whitepaper.md` (edgerun-phase1-spec v1.0)

Status legend:
- `Implemented`
- `Partial`
- `Missing`
- `Deferred (Phase-2)`

## Phase-1 Completion Summary

- Tracked phase-1 items: `46`
- Implemented: `46`
- Completion: `100%`
- Deferred from phase-1 scope: `Full challenge/dispute phase-2 mechanics`

## On-chain Instructions

| Instruction | Status | Notes |
|---|---|---|
| `initialize_config` | Implemented | Config init present. |
| `update_config` | Implemented | Admin-gated update present. |
| `register_worker_stake` | Implemented | PDA create + active status. |
| `deposit_stake` | Implemented | SOL transfer + accounting. |
| `withdraw_stake` | Implemented | Active-only, lock/rent guarded. |
| `post_job` | Implemented | Escrow transfer, limits cap, runtime checks, deadline set. |
| `assign_workers` | Implemented | Scheduler-gated, lock calc, active stake checks, duplicate worker rejection. |
| `submit_result` | Implemented | Assigned-only, deadline enforced, Ed25519 pre-instruction checked over digest. |
| `finalize_job` | Implemented | Quorum winner selection, tie reject, payouts, winner unlock. |
| `slash_worker` | Implemented | Finalized-only, contradictory evidence enforced, lock slash to treasury/config. |
| `cancel_expired_job` | Implemented | Deadline check, no-fee refund, lock unlock; caller limited to client/scheduler authority. |

## Critical Protocol Rules

| Rule | Status | Notes |
|---|---|---|
| Result digest = `blake3(job_id || bundle_hash || output_hash || runtime_id)` | Implemented | On-chain + worker + scheduler aligned. |
| Ed25519 attestation required and verified | Implemented | Sysvar instruction parsing in-program. |
| Runtime allowlist guard | Implemented | Current MVP behavior uses exact match when allowlist root != zero. |
| Submit deadline enforcement | Implemented | Rejects late `submit_result`. |
| Quorum tie behavior | Implemented | On-chain reject tie; scheduler now also tie-safe (no winner). |
| Duplicate worker assignment blocked | Implemented | Enforced in `assign_workers`. |
| Cancel authorization (client or scheduler) | Implemented | Enforced in `cancel_expired_job`. |

## Off-chain Scheduler / Worker

| Area | Status | Notes |
|---|---|---|
| Scheduler validates result payload hex fields | Implemented | Rejects malformed `job_id`/`bundle_hash`/`output_hash`. |
| Scheduler attestation verification uses expected runtime+bundle | Implemented | Rejects mismatched bundle/runtime contexts. |
| Scheduler runtime allowlist proof generation for `post_job` | Implemented | Derives Merkle proof from `EDGERUN_ALLOWED_RUNTIME_IDS`, verifies derived root against on-chain config root, injects proof into `post_job` tx args. |
| Worker attestation uses digest contract | Implemented | Uses same 128-byte preimage digest. |
| Auto artifacts for assign/finalize/cancel/slash | Implemented | Tx builders updated to current instruction accounts. |

## Test Coverage (Implemented)

| Scenario | Status | Validation |
|---|---|---|
| Missing Ed25519 pre-instruction rejected | Implemented | `program/tests/edgerun.ts` (`rejects submit_result without ed25519 pre-instruction`) |
| Mismatched attestation message rejected | Implemented | `program/tests/edgerun.ts` (`rejects submit_result when pre-instruction message does not match`) |
| Valid attestation accepted | Implemented | `program/tests/edgerun.ts` (`accepts submit_result with valid ed25519 pre-instruction`) |
| Runtime allowlist rejection | Implemented | `program/tests/edgerun.ts` (`rejects post_job when runtime is not in allowlist`) |
| Runtime Merkle allowlist acceptance | Implemented | `program/tests/edgerun.ts` (`accepts post_job with valid runtime Merkle proof`) |
| Submit after deadline rejection | Implemented | `program/tests/edgerun.ts` (`rejects submit_result after deadline`) |
| Duplicate committee assignment rejection | Implemented | `program/tests/edgerun.ts` (`rejects assign_workers with duplicate workers`) |
| Finalize payout + winner unlock path | Implemented | `program/tests/edgerun.ts` (`finalize_job unlocks winner stake and pays protocol + winners`) |
| Finalize cannot run twice | Implemented | `program/tests/edgerun.ts` (second `finalizeJob` call in `finalize_job unlocks winner stake and pays protocol + winners`) |
| Cancel expired refund + unlock path | Implemented | `program/tests/edgerun.ts` (`cancel_expired_job returns escrow to client after deadline`) |
| Cancel cannot run twice | Implemented | `program/tests/edgerun.ts` (second `cancelExpiredJob` call in `cancel_expired_job returns escrow to client after deadline`) |
| Slash losing worker path | Implemented | `program/tests/edgerun.ts` (`slash_worker burns required lock from stake and transfers to config`) |
| Slash winner rejection | Implemented | `program/tests/edgerun.ts` (winner slash attempt in `slash_worker burns required lock from stake and transfers to config`) |
| Slash cannot run twice on same loser | Implemented | `program/tests/edgerun.ts` (second loser `slashWorker` call in `slash_worker burns required lock from stake and transfers to config`) |
| Scheduler bundle mismatch attestation rejection | Implemented | `crates/edgerun-scheduler/src/main.rs` (`rejects_result_attestation_when_bundle_hash_mismatch`) |
| Scheduler tie-at-quorum returns no winner | Implemented | `crates/edgerun-scheduler/src/main.rs` (`tie_at_quorum_returns_no_winner`) |
| Scheduler randomized tie invariant | Implemented | `crates/edgerun-scheduler/src/main.rs` (`randomized_ties_never_select_winner`) |
| Scheduler randomized unique-max invariant | Implemented | `crates/edgerun-scheduler/src/main.rs` (`randomized_unique_max_selects_winner`) |
| Scheduler winner-set uniqueness invariant | Implemented | `crates/edgerun-scheduler/src/main.rs` (`winner_worker_set_is_unique_and_from_reports`) |
| Scheduler malformed result payload rejection | Implemented | `crates/edgerun-scheduler/src/main.rs` (`validate_worker_result_payload_rejects_bad_hex`) |
| Scheduler Merkle proof utility roundtrip | Implemented | `crates/edgerun-scheduler/src/main.rs` (`merkle_root_and_proof_roundtrip`) |

## Remaining Gaps vs Whitepaper (All Phases)

| Item | Status | Notes |
|---|---|---|
| Merkle membership proof for runtime allowlist root | Implemented | `post_job` now accepts `runtime_proof: Vec<[u8;32]>`; verification uses sorted-pair BLAKE3 Merkle hashing from `runtime_id` leaf to `allowed_runtime_root`. |
| Permissionless finalize/slash modes | Implemented | `finalize_job` and `slash_worker` are callable by any signer; correctness/evidence checks remain enforced on-chain. |
| Deterministic runtime compliance/fuzz matrix in CI | Implemented | Baseline gates run in `ci.yml`; extended multi-OS determinism/calibration/SLO plus nightly large fuzz campaign run in `runtime-compliance-matrix.yml`. |
| Full challenge/dispute phase-2 mechanics | Deferred (Phase-2) | Explicitly out of current phase-1 MVP scope; tracked for next phase. |
