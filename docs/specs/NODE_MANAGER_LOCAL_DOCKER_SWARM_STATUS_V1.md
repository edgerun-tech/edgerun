# NODE_MANAGER_LOCAL_DOCKER_SWARM_STATUS_V1

## Goal
- Expose local Docker + Swarm runtime status through node-manager local bridge.
- Show local running services/containers in Intent UI Cloud panel.

## Non-Goals
- Full Docker orchestration API in this iteration.
- Remote multi-node swarm management UI.

## Security / Constraints
- Keep bridge on localhost only (`127.0.0.1:7777`).
- Read-only status endpoint in this iteration.
- Fail closed: endpoint returns explicit error if docker CLI is unavailable.

## Acceptance Criteria
- `GET /v1/local/docker/summary` exists on node-manager local bridge.
- Response includes swarm state and running services/containers.
- Cloud panel displays local Docker provider resources when bridge is available.
- Frontend and Rust validation pass.

## Rollout / Rollback
- Rollout with node-manager + frontend deploy.
- Rollback by reverting route/handler and frontend provider integration.
