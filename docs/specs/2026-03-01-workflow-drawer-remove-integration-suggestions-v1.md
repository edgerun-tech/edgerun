# 2026-03-01 Workflow Drawer Remove Integration Suggestions v1

## Goal

Remove the "Suggested Integrations" strips from workflow drawers to reduce unexpected UI clutter and keep drawer content focused.

## Non-goals

- No changes to integration onboarding flows.
- No removal of Integrations panel itself.
- No changes to drawer open/close mechanics.

## Security and Constraints

- Preserve existing navigation and action behavior for conversations and devices panels.
- Keep deterministic rendering in left/right drawers.
- Do not introduce new runtime dependencies.

## Acceptance Criteria

1. Left and right workflow drawers no longer render suggested integrations sections.
2. Opening integrations remains possible through existing drawer controls.
3. Conversations and device connect flows continue working without suggestion-strip affordances.
4. Frontend check/build and relevant Cypress tests pass.

## Rollout

- Remove suggestion-strip rendering and related computation from workflow overlay.
- Update affected Cypress tests to assert core drawer behavior without suggestion strip selectors.

## Rollback

- Revert workflow overlay and Cypress changes in this slice.
