# 2026-02-28 Intent UI Frontend Consolidation V1

## Goal
- Consolidate duplicated telemetry/view-model logic in `frontend/intent-ui/src/App.jsx` into reusable helpers.
- Keep UI behavior unchanged while reducing duplicate sorting/filtering and formatting code paths.

## Non-Goals
- No visual redesign.
- No protocol changes to local bridge, event bus, or integration flows.
- No changes to onboarding/auth model in this slice.

## Security and Constraint Requirements
- Preserve fail-closed local bridge behavior.
- Keep event/state derivation deterministic from runtime events.
- Use existing Bun-based frontend workflow only.
- Do not introduce new network endpoints or service ports.

## Acceptance Criteria
1. `App.jsx` no longer owns telemetry/system-state derivation internals directly; logic is moved to a shared module.
2. Event bus list, docker logs list, and system-state panel continue rendering with the same data semantics.
3. Existing event-bus filtering and severity mapping behavior remains intact.
4. Frontend checks pass:
   - `cd frontend && bun run check`
   - `cd frontend && bun run build`
5. `App.jsx` context-menu and account-menu UI blocks are extracted into dedicated layout components with unchanged behavior.
6. Layer indicator, local-bridge-required overlay, and telemetry panel composition are extracted into dedicated layout components with unchanged behavior.
7. Global mount/cleanup lifecycle and window event wiring are extracted from `App.jsx` into a dedicated hook with unchanged behavior.
8. Context-menu state/actions (copy, paste, quick actions) are extracted from `App.jsx` into a dedicated hook with unchanged behavior.
9. Telemetry derivation and docker-log fetch/filter plumbing are extracted from `App.jsx` into a dedicated hook with unchanged behavior.
10. Account/profile session state and handlers are extracted from `App.jsx` into a dedicated hook with unchanged behavior.
11. Docker log updates are strictly event-driven from the local event bus; polling-based log refresh is removed.

## Rollout
- Ship helper module and App wiring in a single change set.
- Validate with local frontend check/build.

## Rollback
- Revert helper module import/wiring and restore previous in-file logic in `App.jsx`.
