# 2026-02-28 Node Manager Remove Docker Logs HTTP Endpoint V1

## Goal
- Remove `/v1/local/docker/logs` from `edgerun-node-manager`.
- Ensure local control-plane docker log visibility uses the local event bus path only.

## Non-Goals
- No changes to `/v1/local/docker/summary`.
- No frontend visual redesign.
- No remote/cloud log transport in this slice.

## Security and Constraint Requirements
- Preserve loopback-only local bridge binding.
- Keep fail-closed control-plane behavior (no fallback polling transport).
- Keep docker event publication scoped to local event bus topics.

## Acceptance Criteria
1. `edgerun-node-manager` no longer exposes `/v1/local/docker/logs`.
2. Node manager docker log storage/dedupe path for that endpoint is removed.
3. Node manager emits docker runtime activity on local event bus topic `local.docker.events`.
4. Intent UI docker panel derives data only from event bus timeline events.
5. Frontend and node-manager validation commands pass.

## Rollout
- Deploy node-manager update with removed route and event publication.
- Deploy frontend update with event-only docker panel path.

## Rollback
- Revert this change set to restore `/v1/local/docker/logs` and prior frontend behavior.
