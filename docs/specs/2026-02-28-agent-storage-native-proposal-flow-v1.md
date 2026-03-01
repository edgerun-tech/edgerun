# 2026-02-28 Agent Storage Native Proposal Flow V1

## Goal
- Connect the current virtualized agent diff workflow to the `edgerun-storage` proposal queue and gatekeeper tools.
- Keep event-first semantics while making proposal lifecycle queryable and enforceable through storage-native events.

## Non-Goals
- Replacing existing NATS diff events.
- Automatic merge to git branches.
- Distributed conflict resolution across multiple repository authorities.

## Security And Constraints
- Fail-closed by default for storage operations: if storage proposal submission or gatekeeper apply fails, command returns non-zero.
- Reuse existing release binaries (`vfs_operator`, `proposal_gatekeeper`, `proposal_batch_gatekeeper`) or explicit paths.
- Preserve deterministic IDs: proposal IDs derive from run IDs unless explicitly overridden.
- No new service on port `8080`.

## Design
1. Add `edgertool` commands:
   - `storage-proposal-submit`: submit a run patch (or patch file) to storage via `vfs_operator propose-diff`.
   - `storage-proposal-apply`: run `proposal_gatekeeper` for a proposal with optional dry-run.
2. Add wrappers in `scripts/agents/` for both commands.
3. Update `scripts/agents/README.md` with the integrated sequence:
   - launch -> emit event -> submit storage proposal -> gatekeeper dry-run -> gatekeeper apply.
4. Wire optional auto-storage flow into `edgertool agent-launch`:
   - when enabled, `agent-launch` automatically submits the produced patch into storage.
   - optional gatekeeper dry-run runs immediately after submit.
   - behavior is fail-closed if storage step is enabled and fails.

## Acceptance Criteria
1. Operator can submit an agent run patch to storage queue with one command.
2. Operator can gatekeeper-check and apply one proposal with one command.
3. Scripts are fail-closed on missing binaries or command failures.
4. Docs show exact commands and expected flow.
5. `agent-launch` can execute storage submission + dry-run automatically via flags/env without manual extra steps.

## Rollout
- Use storage submission and gatekeeper apply for new agent runs.
- Keep existing `apply-accepted-diff.sh` behavior for compatibility.

## Rollback
- Revert new `edgertool` subcommands and wrapper scripts.
- Continue with event-only accepted diff handling.
