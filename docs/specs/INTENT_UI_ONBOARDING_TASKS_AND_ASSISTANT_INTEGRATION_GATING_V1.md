# Intent UI Onboarding Tasks And Assistant Integration Gating V1

## Goal
- Keep profile onboarding reachable at any time after first-run.
- Replace launcher drawer shortcuts with actionable startup tasks that remind users to connect devices and integrations.
- Treat assistant backends as integrations and enforce fail-closed gating before any assistant request.
- Consolidate session/profile controls into an always-available account-circle popup menu.

## Non-goals
- No backend auth or relay protocol changes in this slice.
- No new remote device discovery implementation in this slice.
- No change to assistant API contract (`/api/assistant`) in this slice.

## Security and constraints
- Fail-closed: assistant execution must not run unless a device is connected and the selected assistant integration is connected.
- Onboarding access must not bypass existing profile/session security checks.
- Keep implementation local-first and deterministic (browser/runtime state only).
- Keep JS workflow on `bun`; no npm/pnpm additions.

## Acceptance criteria
1. Onboarding gate can be reopened from normal UI after initial bootstrap.
2. Launcher panel removes drawer actions and presents startup task reminders (profile/onboarding, device, integrations, assistant).
3. Assistant preflight blocks execution with explicit error status when:
   - no connected device is available, or
   - selected assistant integration is not connected.
4. Assistant integrations include local `codex_cli` and `qwen` entries in Integrations UI.
5. App shell exposes an account-circle popup showing:
   - session mode
   - profile id/backend details
   - registered domain when available, otherwise an explicit unregistered state
   - profile/onboarding actions
6. Frontend validation passes (`check`, `build`) and cypress coverage is updated for the new behavior.

## Rollout / rollback
- Rollout:
  - `frontend/intent-ui/src/App.jsx`
  - `frontend/intent-ui/src/components/onboarding/ProfileBootstrapGate.jsx`
  - `frontend/intent-ui/src/components/panels/LauncherGuidePanel.jsx`
  - `frontend/intent-ui/src/stores/workflow-ui.js`
  - `frontend/intent-ui/src/stores/integrations.js`
  - `frontend/intent-ui/src/components/layout/IntegrationsPanel.jsx`
  - `frontend/cypress/e2e/intent-ui-onboarding-assistant-gating.cy.js`
- Rollback: revert this change set to restore previous launcher and assistant behavior.
