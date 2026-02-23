# Routed Terminal Connect Resilience V1

## Goal
- Ensure terminal pane routing remains deterministic when a user clicks `Connect` on a `route://` device, even if the WebRTC supervisor is temporarily unavailable.

## Non-goals
- No protocol changes for routed terminal frames.
- No change to route discovery, owner import, or scheduler signaling contracts.
- No new background polling behavior.

## Security and constraints
- Preserve existing wallet-gated terminal drawer behavior.
- Keep control/data paths best-effort without introducing additional network privileges.
- Avoid fallback to iframe-based terminal embedding for `route://` targets.

## Acceptance criteria
- Clicking `Connect` on a routed device always binds the active tab pane to that `route://` target immediately.
- If signaling/probe infrastructure is unavailable, status is marked `offline` but the routed pane still renders.
- Existing route UI diagnostics remain unchanged.
- Cypress `terminal-route-resolution` passes with deterministic routed-pane rendering assertions.

## Rollout and rollback
- Rollout: single codepath adjustment in `use-terminal-drawer-controller.ts`.
- Rollback: revert the connect flow change to prior behavior if regressions appear in routed terminal connection handling.
