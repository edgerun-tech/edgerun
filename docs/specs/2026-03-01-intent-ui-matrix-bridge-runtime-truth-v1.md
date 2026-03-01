# 2026-03-01 Intent UI Matrix Bridge Runtime Truth v1

## Goal
Make Matrix bridge integrations (WhatsApp/Signal/Telegram/etc.) fail-closed and runtime-truthful so "connected" means a running local bridge runtime, not only stored token/profile state.

## Non-goals
- Full Matrix bridge deployment/orchestration UX.
- New backend protocols for bridge health beyond existing local bridge MCP status/start endpoints.

## Security / Constraint Requirements
- Matrix bridge providers are user-owned only in this phase (no platform-mode fallback).
- Verification must require runtime health (`/v1/local/mcp/integration/status`) and start attempt (`/v1/local/mcp/integration/start`) where needed.
- Connection hydration/check-all must correct stale optimistic state to runtime truth.
- Worker and main-thread integration lifecycles must stay behaviorally aligned.

## Acceptance Criteria
1. Matrix bridge providers default to and enforce `user_owned` connector mode.
2. Matrix bridge verify fails on missing/invalid token and performs pre-link validation only; it does not boot runtime containers.
3. Matrix bridge connect (Link Integration) is the step that boots runtime containers and fails if runtime start/health checks fail.
4. `checkAll()` reconciles persisted integration state with runtime status for MCP-backed integrations.
5. Disconnected/non-linked MCP integrations do not keep runtime containers running.
6. Cypress local bridge simulator supports MCP start/stop/status endpoints and matrix connection-truth coverage.

## Rollout / Rollback
- Rollout: frontend/store/worker update + simulator/test update.
- Rollback: revert this spec companion diffs to restore previous optimistic token-only behavior.
