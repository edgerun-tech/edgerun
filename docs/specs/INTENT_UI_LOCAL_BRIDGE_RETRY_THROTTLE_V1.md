# INTENT_UI_LOCAL_BRIDGE_RETRY_THROTTLE_V1

## Goal
- Prevent runaway retry loops and browser console spam when the local eventbus WebSocket bridge (`ws://127.0.0.1:7777/v1/local/eventbus/ws`) is unavailable.

## Non-Goals
- No protocol change to the local bridge.
- No change to remote eventbus behavior.
- No UI redesign for device connection flows.

## Security / Constraints
- Keep fail-closed semantics: if local bridge is unavailable, do not fake connection success.
- Keep behavior deterministic and local-first.
- Do not introduce new runtime dependencies.

## Acceptance Criteria
- Local bridge failed connections do not retry at a fixed rapid interval.
- Retry logic uses bounded exponential backoff and a cooldown pause after repeated failures.
- A successful connection resets retry state.
- Runtime connected/disconnected state remains accurate.

## Rollout / Rollback
- Rollout: ship with frontend deploy.
- Rollback: revert `eventbus.js` retry-throttle changes; prior fixed-interval retry behavior returns.
