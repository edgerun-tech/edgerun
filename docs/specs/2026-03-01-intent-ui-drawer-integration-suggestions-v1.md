# INTENT_UI_DRAWER_INTEGRATION_SUGGESTIONS_V1

## Goal
Add integration tags and use them to drive panel-aware integration suggestions in workflow drawers.

## Non-Goals
- No integration recommendation service or remote ranking model.
- No backend schema/API changes.
- No automatic connection flow; suggestions only deep-link to Integrations setup.

## Security and Constraints
- Suggestion state is derived from existing integration state and static tag metadata only.
- No credential/token values are rendered in suggestion UI.
- Keep event-driven architecture and existing integration connection checks unchanged.

## Design
- Add `tags: string[]` to integration catalog entries in `integrationStore`.
- Expose tags via `integrationStore.list()`.
- In `WorkflowOverlay`, define panel -> topic tags mapping (messages/storage/code/workflows/etc.).
- Add a reserved bottom section in drawer panels: "Suggested Integrations".
- Render top matching providers by tag overlap and current readiness.
- Suggestion item click opens Integrations drawer targeted to that provider.

## Acceptance Criteria
1. Every integration has at least one topic tag.
2. Drawer panels reserve a bottom area for integration suggestions.
3. Suggestions are context-aware per panel based on tags.
4. Suggestion rows show provider name and readiness state.
5. Existing conversations/devices drawer e2e coverage includes the new suggestion area.

## Rollout / Rollback
- Rollout: frontend-only deploy.
- Rollback: revert this commit to remove tag metadata + drawer suggestions.
