# 2026-02-28 Intent UI File Manager Node Targeting V1

## Goal
Enable File Manager local filesystem operations to be explicitly node-scoped and user-selectable from the UI, using the node-manager local bridge as the transport.

## Non-Goals
- Multi-hop/mesh forwarding to remote nodes.
- Full remote node discovery protocol changes.
- Non-local providers (GitHub/Drive/RAMFS) behavioral changes.

## Security and Constraints
- Fail closed: local filesystem APIs must reject requests for node IDs other than the active local node ID.
- Local bridge remains loopback-bound only.
- Filesystem paths must be root-confined with traversal protection.
- No `npm`/`pnpm`; frontend validation stays on `bun` workflows.
- No new listener on port `8080`.

## Acceptance Criteria
1. Node manager exposes local bridge filesystem APIs under `/v1/local/fs/*` for list/read/write/mkdir/delete/move/copy/meta/archive/extract.
2. File Manager shows a node selector for filesystem-capable nodes.
3. Selecting a node updates local filesystem calls to include node context.
4. If selected node is not the local node-manager node, local filesystem operations fail with a clear message (fail-closed).
5. Existing local mount browsing still works when local node is selected.
6. Cypress coverage verifies node selector behavior and fail-closed UX for non-local node selection.

## Rollout
- Ship local bridge fs endpoints and frontend selector together.
- Keep old `/api/fs/*` usage removed from File Manager/local provider in this slice.

## Rollback
- Revert this spec’s implementation commit to restore previous `/api/fs/*` behavior.

