# 2026-03-01 GitHub Integration Sticky Runtime Node Id Race v1

## Goal
Prevent GitHub integration from silently dropping back to disconnected when MCP runtime checks run before local host node identity is hydrated.

## Non-goals
- No backend protocol redesign for local bridge node identity.
- No change to GitHub PAT verification endpoint behavior.
- No multi-node orchestration changes.

## Security and Constraint Requirements
- Keep local bridge fail-closed behavior for explicit non-local node targeting.
- Do not widen token exposure in UI events or logs.
- Preserve loopback-only local bridge assumptions.
- Do not use port 8080.

## Design
- Frontend MCP requests must treat `node_id` as optional until a host device id is known.
  - If no online host device exists in `knownDevices()`, omit `node_id` from request payload/query.
- Runtime truth reconciliation must avoid destructive demotion on transient MCP status errors.
  - For GitHub user-owned integrations with a valid token and already-linked state, keep linked/connected state when runtime status call fails.
  - Keep explicit disconnect behavior when token is missing or connector mode is invalid.
- Add Cypress coverage for host-identity-unavailable startup mode.
  - Simulate strict local-node enforcement in local bridge test helper.
  - Assert GitHub link remains connected across revisit.

## Acceptance Criteria
1. GitHub integration link succeeds when host device identity is unavailable at connect time.
2. MCP start/status requests do not send fallback fake node ids.
3. `checkAll()` no longer force-disconnects GitHub solely due transient MCP status read errors.
4. Cypress test reproduces host-id-unavailable mode and validates sticky connected state.

## Rollout
- Frontend-only rollout plus Cypress helper/test updates.
- No data migration required.

## Rollback
- Revert frontend integration store changes and Cypress helper/spec updates tied to this document.
