# 2026-02-28 Intent UI Local Bridge Strict Mode V1

## Goal
- Enforce localhost bridge as required runtime transport for Intent UI.
- Remove fallback behavior when bridge is unavailable.
- Show a clear blocking error: `Can't connect to local bridge, is it running?`

## Non-Goals
- Changing node-manager bridge API contract.
- Replacing websocket transport.

## Security / Constraints
- Fail closed in UI transport path: no local-fallback event mode when bridge is down.
- Avoid websocket retry storms and console spam.

## Acceptance Criteria
1. Event publish path does not run in fallback mode when localhost bridge is disconnected.
2. Runtime exposes explicit bridge error state.
3. App renders a blocking bridge error screen with retry action when bridge is unavailable.
4. On successful reconnect, normal UI resumes.

## Rollout / Rollback
- Rollout: update Intent UI eventbus runtime + app shell rendering.
- Rollback: revert this change set to restore previous reconnect/fallback behavior.
