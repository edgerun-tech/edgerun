# Intent UI Integration Connection Truth V1

## Goal
- Ensure Integrations panel reports truthful state for each provider.
- Separate `connected` (user/profile linked) from `available` (currently usable in this session/runtime).
- Remove false-positive connected state for platform-mode providers.

## Non-goals
- No backend connector provisioning changes.
- No remote policy-engine integration in this slice.

## Security and constraints
- Fail-closed: providers must not report connected unless explicitly linked.
- Availability must be derived from runtime constraints (profile session/device), not optimistic defaults.
- Existing local-first behavior remains deterministic.

## Acceptance criteria
1. Platform-capable providers do not auto-show as connected on fresh state.
2. Integrations panel shows both `Connected` and `Available` status.
3. `available` is false when prerequisites are missing (e.g., no profile session or no device for local CLI).
4. Assistant dispatch gates against integration availability, not just connection.
5. Frontend `check`, `build`, and targeted Cypress coverage pass.

## Rollout / rollback
- Rollout:
  - `frontend/intent-ui/src/stores/integrations.js`
  - `frontend/intent-ui/src/components/layout/IntegrationsPanel.jsx`
  - `frontend/intent-ui/src/stores/workflow-ui.js`
  - `frontend/cypress/e2e/intent-ui-integrations-availability.cy.js`
- Rollback: revert this change set to restore previous semantics.
