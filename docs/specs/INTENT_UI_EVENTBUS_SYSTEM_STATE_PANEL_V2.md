# Intent UI Eventbus System State Panel V2

## Goal
Improve readability and operational usefulness of the existing system state panel without changing data sources.

## Non-goals
- No backend/API additions.
- No event contract changes.

## Security and Constraints
- Keep state derivation strictly from eventbus timeline + existing local runtime stores.
- Preserve fail-closed local bridge gating behavior.

## Acceptance Criteria
1. System state rows include severity styling for quick scanning.
2. Executor rows are ordered by criticality (failure first, then started/running, then success).
3. Panel includes executor summary counts.
4. Rows show freshness (`...ago`) based on event timestamp when available.
5. Frontend check/build pass.

## Rollout
- Immediate UI-only enhancement.

## Rollback
- Revert `App.jsx` panel rendering/memo logic to V1 behavior.
