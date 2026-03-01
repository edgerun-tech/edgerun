# 2026-03-01 Tailscale Integration Quickstart v2

## Goal

Make Tailscale integration setup actionable inside Integrations by generating concrete join and App Connector policy outputs from user inputs.

## Non-goals

- No automatic mutation of tailnet policy via API.
- No automatic remote installation of Tailscale.
- No change to backend Tailscale proxy contract in this slice.

## Security and Constraints

- Keep integration truth fail-closed: only link after explicit verification and link action.
- Continue using user-entered runtime values; do not introduce new server-side secret persistence.
- Preserve existing profile and connector gating behavior.

## Acceptance Criteria

1. Tailscale setup includes editable connector tag and app domains fields.
2. Step 2 renders generated, copyable-ready quickstart artifacts:
   - `tailscale up` command with connector tag
   - `tailscale funnel` command
   - tailnet policy `nodeAttrs` snippet for App Connector domains
3. Existing Tailscale verification/link path remains functional.
4. Cypress verifies user-visible quickstart artifacts.

## Rollout

- Add setup inputs and command/policy generation in Integrations panel.
- Persist Tailscale setup values in local storage for repeat sessions.
- Extend Tailscale Cypress coverage.

## Rollback

- Revert Tailscale setup UI additions and test updates.
