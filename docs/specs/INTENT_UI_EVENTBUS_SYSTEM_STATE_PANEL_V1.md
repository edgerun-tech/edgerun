# Intent UI Eventbus System State Panel V1

## Goal
Expose live operational state in frontend directly from eventbus-derived data so operators can see platform state without leaving Intent UI.

## Non-goals
- No new backend endpoints.
- No replacement of existing event timeline panel.
- No schema changes to emitted events.

## Security and Constraints
- State must be derived from existing local bridge event stream and in-memory stores.
- UI must remain fail-closed when bridge is unavailable (existing local bridge gating remains).
- No background polling for this state panel; derive from current timeline + runtime signals.

## Acceptance Criteria
1. Add visible `SYSTEM STATE` floating panel in Intent UI shell.
2. Panel shows at least:
   - bridge connection status,
   - online/total device count,
   - latest code revision event,
   - latest diff proposed/accepted state,
   - recent executor build/test statuses.
3. Panel data updates as event timeline updates.
4. Frontend checks and build pass.

## Rollout
- Immediate in current UI shell.

## Rollback
- Remove the panel rendering and derived state memo from `App.jsx`.
