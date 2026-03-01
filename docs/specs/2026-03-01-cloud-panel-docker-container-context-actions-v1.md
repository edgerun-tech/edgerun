# 2026-03-01 Cloud Panel Docker Container Context Actions v1

## Goal
Allow operators to edit local Docker container runtime state directly from Intent UI via right-click actions in the Cloud panel.

## Non-goals
- No bulk container operations.
- No destructive delete/prune actions.
- No service-level swarm orchestration changes.

## Security and Constraints
- Only allow explicit container state actions: `start`, `stop`, `restart`.
- Validate container selectors before executing docker commands.
- Keep operations behind local bridge endpoints and loopback trust boundary.
- Do not use port 8080.

## Design
- Add local bridge endpoint:
  - `POST /v1/local/docker/container/state`
  - request: `{ container, action }`
  - response: `{ ok, container, action, state, message }`
- Extend Cloud panel Docker container rows with right-click context menu actions.
- After action success, refresh cloud resources to show updated status.
- Add Cypress simulator support for Docker summary and container state actions.
- Add Cypress coverage for right-click menu + state transition.

## Acceptance Criteria
1. Right-clicking a Docker container row in Cloud panel shows `Start`, `Stop`, `Restart` actions.
2. Selecting an action calls local bridge container state endpoint and updates visible state after refresh.
3. Invalid action/selector requests fail with deterministic errors.
4. Frontend checks/build and targeted Cypress test pass.

## Rollout
- Ship node-manager endpoint + Cloud panel UI together.
- Use dockerized Caddy profile (`docker-compose.node-manager.yml` caddy service) unchanged; local bridge pathing remains `/v1/local/*`.

## Rollback
- Revert endpoint route/handler and Cloud panel context menu changes tied to this document.
