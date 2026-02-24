# Frontend Nav Diagnostics Signal Decoupling V1

## Goal
- Prevent misleading "all red" nav diagnostics by decoupling scheduler/control-ws status from overlay peer-signal status.
- Make `scheduler` and `ws` indicators reflect direct control endpoint reachability, while `overlay-ws` reflects overlay signaling state.

## Non-goals
- No change to nav layout, links, wallet flow, or terminal drawer controls.
- No change to route-advert freshness logic.
- No change to backend APIs or websocket server contracts.

## Security and constraints
- Preserve all existing nav diagnostics `data-testid` attributes.
- Keep implementation in `frontend/components/nav.tsx` using existing `frontend/lib/webrtc-route-client.ts` probe utilities.
- Avoid new runtime dependencies.
- Keep behavior deterministic with bounded probe cadence.

## Acceptance criteria
- `scheduler online/offline` and `ws ok/down` are derived from control websocket probe results, not from overlay peer supervisor connection state.
- `overlay-ws on/off` remains derived from overlay supervisor status (`controlSignalConnected`).
- Status title text remains informative and includes overlay signal context.
- Existing Cypress diagnostics selectors remain valid.

## Rollout and rollback
- Rollout: ship nav diagnostics source decoupling and validate via `frontend` check/build.
- Rollback: revert `frontend/components/nav.tsx` to previous coupling logic if regressions occur.
