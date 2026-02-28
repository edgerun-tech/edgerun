# Intent UI Tailscale App Connector Setup V1

## Goal
- Make Tailscale integration operational for real node onboarding by including required App Connector setup steps.
- Provide generated commands and policy snippet in Integrations dialog so users can copy/paste and complete setup quickly.

## Non-goals
- Automated mutation of tailnet policy via API.
- Automatic installation of Tailscale on remote nodes.
- Replacing existing relay/device pairing flows.

## Security and Constraints
- Keep connection truth fail-closed: connected only after explicit link action.
- Keep profile-gated availability behavior unchanged.
- Avoid hidden defaults by showing connector tag/domains inputs and generated outputs explicitly.

## Acceptance Criteria
1. Tailscale dialog includes editable connector tag and app domains.
2. Generated join command includes `--advertise-connector` and tag advertisement.
3. Dialog shows copyable tailnet policy snippet for App Connector `nodeAttrs`.
4. Existing Funnel command remains available and copyable.
5. Cypress validates user-visible App Connector setup artifacts.

## Rollout / Rollback
- Rollout: additive dialog UX and test updates.
- Rollback: revert dialog additions and related test.
