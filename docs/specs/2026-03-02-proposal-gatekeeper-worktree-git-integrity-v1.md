# Proposal Gatekeeper Worktree Git Integrity v1

## Goal

Ensure `proposal_gatekeeper` can validate and apply storage proposals reliably by preserving git metadata in its detached worktree during workspace sync.

## Non-goals

- Changing proposal semantics or event schema.
- Altering `proposal_batch_gatekeeper` behavior in this change.
- Changing storage import/propose tooling.

## Security and Constraint Requirements

- Gatekeeper must continue to fail closed on patch/format/check failures.
- Detached worktree git metadata must remain intact through sync operations.
- No direct mutation of source repository during dry-run/apply validation.

## Acceptance Criteria

1. `proposal_gatekeeper --dry-run` succeeds for a valid proposal in a git repo.
2. `proposal_gatekeeper` apply appends `fs_delta_applied` for accepted proposal.
3. `vfs_operator materialize` reflects the applied proposal state.
4. `mcp-syscall` submit-only proposal workflow remains unchanged.

## Rollout

- Update gatekeeper rsync exclusions to preserve `.git` file and `.git/` directory in detached worktree.
- Rebuild `proposal_gatekeeper` binary.
- Validate end-to-end with one live proposal.

## Rollback

- Revert gatekeeper rsync exclusion change and rebuild binary.
- Resume proposal flow with prior behavior if needed.
