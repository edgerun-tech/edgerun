---
name: edgerun-agent-storage-proposal-flow
description: Run Edgerun diff workflows through storage-native propose/list/validate/apply lifecycle with fail-closed behavior
license: Apache-2.0
compatibility: opencode
metadata:
  audience: agents
  repository: edgerun
---

## What I do

- Keep coding changes on event-native rails.
- Submit unified diffs as storage proposals and validate with gatekeeper before apply.
- Avoid direct local patch application as primary workflow.

## Preconditions

- `vfs_operator` and `proposal_gatekeeper` binaries are available.
- Storage data dir is initialized with `import-git` for the target repo.
- MCP server `edgerun_syscall` is connected for proposal submission via `apply_patch`.

## Core workflow

1. Prepare a focused unified diff.
2. Submit proposal (`edgerun_syscall.apply_patch` or `vfs_operator propose-diff`).
3. Verify queue state:
   - `vfs_operator list-proposals --data-dir <DATA_DIR> --repo-id <REPO_ID> --branch <BRANCH> --limit 200`
4. Run dry-run gatekeeper first.
5. Apply only after dry-run succeeds.
6. Confirm outcome via `list-events` (`fs_delta_proposed`, `fs_delta_applied`, `fs_delta_rejected`).

## Required practices

- Fail closed on submit/apply errors.
- Dry-run before non-dry apply.
- Keep proposal IDs traceable and report them in outcomes.
- Do not bypass gatekeeper unless explicitly requested.

## Useful overrides

- `VFS_OPERATOR_BIN=/path/to/vfs_operator`
- `PROPOSAL_GATEKEEPER_BIN=/path/to/proposal_gatekeeper`
- `--repo-root /path/to/repo`
- `--fmt-cmd "..."`
- `--check-cmd "..."`
- `--timeout-secs N`

## Validation checklist

- Proposal appears in queue and `fs_delta_proposed` events.
- Gatekeeper dry-run result is captured.
- Applied/rejected event exists after non-dry run.
- Materialize state reflects new branch head sequence.

## References

- `crates/edgerun-storage/docs/operator-guide.md`
- `docs/specs/2026-02-28-agent-storage-native-proposal-flow-v1.md`
