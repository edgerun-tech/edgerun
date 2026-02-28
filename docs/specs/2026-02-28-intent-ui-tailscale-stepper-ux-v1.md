# Intent UI Tailscale Stepper UX V1

## Goal
- Replace the dense Tailscale dialog block with a clear step-by-step workflow.
- Make state and next action explicit for API connection, device selection, connector setup, and integration linking.

## Non-goals
- New backend capabilities.
- Changes to Tailscale API proxy request semantics.
- Replacing connector ownership model.

## Security and Constraints
- Keep existing encrypted profile secret handling for API/auth keys in profile mode.
- Keep fail-closed behavior for missing API key/tailnet/device/routes inputs.
- Do not weaken integration availability gating.

## Acceptance Criteria
1. Tailscale dialog is organized into numbered steps:
- Connect API
- Select device and routes
- Configure App Connector/Funnel
- Link integration
2. Each step has explicit status text and local error/success feedback.
3. Users can clearly distinguish API key vs node auth key purpose.
4. Existing Tailscale Cypress integration test passes with updated selectors/assertions.

## Rollout / Rollback
- Rollout: UI-only refactor within Integrations panel.
- Rollback: revert stepper sections to previous single-block layout.
