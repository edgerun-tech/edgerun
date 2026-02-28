# 2026-02-28 Proposal Gatekeeper Auto Format V1

## Goal
- Automatically reject proposals that fail compile validation.
- Automatically format compile-valid proposals before apply.
- Keep decision trail append-only in VFS event log (`FsDeltaRejectedV1`, `FsDeltaProposedV1`, `FsDeltaAppliedV1`).

## Non-Goals
- Replacing human review policy.
- Multi-language format/check orchestration in this phase.
- Mutating the operator's active working tree directly.

## Security And Constraints
- Validation must run in isolated temporary git worktree.
- Original repository working tree must remain untouched.
- Rejection/apply decisions must be persisted as events.
- If validation tooling is unavailable (`git`, `cargo`, `rustfmt`), fail closed and reject proposal.

## Design
1. Add `proposal_gatekeeper` binary under `tools/`.
2. Add `proposal_batch_gatekeeper` binary under `tools/`.
3. Single-proposal workflow:
   - Load proposal by `{repo_id, branch_id, proposal_id}` from VFS journal.
   - Create detached temporary worktree from target repo root.
   - Apply proposal patch.
   - Run `cargo fmt --all`.
   - Run `cargo check --workspace`.
   - On failure:
     - append `FsDeltaRejectedV1` with error reason.
   - On success:
     - compute formatted diff from worktree.
     - if formatted diff differs, append formatted proposal event.
     - append `FsDeltaAppliedV1` referencing winning proposal id.
4. Batch workflow:
   - Input ordered list of proposal ids.
   - In isolated worktree, apply proposal `i`, run format/check, commit checkpoint.
   - If step `i` fails, append rejection for `i` and stop (later proposals not applied).
   - If all steps pass:
     - extract per-step committed patch (`git show --binary --format=`),
     - append formatted proposal event when needed,
     - append apply event in original order.
5. Cleanup temporary worktree regardless of outcome.

## Acceptance Criteria
1. Gatekeeper can locate a proposal by id and branch.
2. Invalid patch/format/check failures append rejection event automatically.
3. Valid proposal applies automatically.
4. If formatting changes patch, formatted proposal event is appended before apply.
5. Batch mode validates each proposal incrementally and stops on first failing step.
6. Existing crate tests and checks pass after integration.

## Rollout
1. Add proposal lookup helper in `virtual_fs`.
2. Add `proposal_gatekeeper` binary.
3. Validate with crate check/tests and binary build check.

## Rollback
- Remove binary and helper APIs; leave recorded events append-only.
