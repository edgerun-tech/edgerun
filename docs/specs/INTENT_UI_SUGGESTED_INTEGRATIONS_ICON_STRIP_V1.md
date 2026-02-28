# INTENT_UI_SUGGESTED_INTEGRATIONS_ICON_STRIP_V1

## Goal and Non-Goals

### Goal
- Update drawer suggested integrations to icon-only actions with hover tooltip labels.
- Render suggested integration icons behind the `SUGGESTED INTEGRATIONS` label as a low-contrast strip.

### Non-Goals
- No changes to integration connection lifecycle.
- No changes to panel suggestion scoring.

## Security and Constraints
- Suggestions remain local UI affordances; no credential handling changes.
- Keep event-driven flow unchanged.

## Acceptance Criteria
- Suggested integrations render as icon-only controls.
- Each icon has tooltip text via native hover title.
- Header displays a subtle icon strip behind the label text.
- No text cards/buttons remain in the suggestion section.

## Rollout and Rollback
- Rollout by replacing drawer suggestion rendering logic in `WorkflowOverlay`.
- Rollback by restoring previous button-grid layout.
