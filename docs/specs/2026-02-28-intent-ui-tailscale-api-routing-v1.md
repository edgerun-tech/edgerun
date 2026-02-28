# Intent UI Tailscale API Routing V1

## Goal
- Let users enter a Tailscale API key and tailnet in Intent UI.
- Use that key to call Tailscale API through the `os` worker and drive route configuration from the UI.

## Non-goals
- Long-lived backend secret storage for user API keys.
- Automatic tailnet policy mutation in this slice.
- Replacing CLI-based `tailscale up` onboarding.

## Security and Constraints
- API key is user-supplied at runtime and sent only to `os` worker proxy endpoints.
- Worker must fail-closed on missing/invalid key/tailnet/device parameters.
- Worker returns only required route/device fields for UI decisions.
- UI should continue operating when API calls fail, with explicit error state.

## Acceptance Criteria
1. Tailscale dialog has inputs for API key and tailnet.
2. UI can fetch tailnet devices via worker proxy.
3. UI can set enabled routes for a selected device via worker proxy.
4. Cypress validates user-visible API routing controls with intercepted API responses.

## Rollout / Rollback
- Rollout: additive worker API endpoints + Tailscale dialog controls.
- Rollback: revert worker Tailscale endpoints and dialog API controls.
