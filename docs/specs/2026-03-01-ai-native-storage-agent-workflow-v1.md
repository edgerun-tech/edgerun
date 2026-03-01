# 2026-03-01 AI Native Storage Agent Workflow V1

## Goal
- Remove agent workflow glue between proposal submission and gatekeeping.
- Make `edgerun-storage` the native authority for `submit -> validate -> apply/reject`.
- Keep fail-closed behavior for all agent patch flows.

## Non-Goals
- Replacing protobuf event schema versions in this phase.
- Changing frontend UX or introducing polling-based status flows.
- Introducing new runtime services or ports.

## Security And Constraints
- Fail-closed: any submit/gate/apply failure exits non-zero.
- Preserve existing `FsDeltaProposedV1`, `FsDeltaAppliedV1`, `FsDeltaRejectedV1` event semantics.
- Patch handling must remain deterministic and reproducible.
- No fallback-only execution path.
- No new service on port `8080`.

## Design
- Extend `proposal_gatekeeper` with a storage-native ingest mode:
  - Accept `--diff-file`, `--agent-id`, `--intent`, optional `--proposal-id`.
  - Append proposal event directly via `StorageBackedVirtualFs::propose_delta`.
  - Run existing gate logic in-process (dry-run or apply mode).
  - Support `--submit-only` for submit-without-gate.
- Keep existing proposal-id mode for compatibility.
- Update `edgertool agent-launch` autosubmit path to call only `proposal_gatekeeper` in ingest mode, removing dual-binary glue for normal agent runs.
- Keep legacy `vfs_operator propose-diff` path available for manual/debug workflows.

## Acceptance Criteria
1. Agent autosubmit + dry-run uses a single storage binary (`proposal_gatekeeper`) for submit+gate.
2. `agent-task` succeeds through proposal submission and dry-run gate with no `vfs_operator` call in that path.
3. Existing manual proposal-id gate flow remains functional.
4. Failure reasons remain explicit and fail-closed.

## Rollout
- Add ingest mode flags to `proposal_gatekeeper`.
- Wire `edgertool agent-launch` autosubmit path to ingest mode.
- Validate with `go test` for tooling and crate-scoped checks for storage.

## Rollback
- Revert to previous `edgertool` autosubmit sequence (`vfs_operator` then `proposal_gatekeeper`).
- Use historical proposal submission scripts for manual operation.
