# Intent UI Tailscale Integration V1

## Goal
- Add Tailscale as a first-class integration in Intent UI so users can quickly connect existing tailnets.
- Provide an in-product quick-start for node join and Funnel exposure suitable for browser-first control-plane entry.

## Non-goals
- Replacing EdgeRun transport internals with Tailscale-native data plane in this slice.
- Automatic device-side Tailscale installation/execution from browser.
- Advanced ACL/policy management UI for tailnet.

## Security and Constraints
- Follow existing integration connection truth model: not connected by default, fail-closed availability.
- Require profile session for availability, aligned with current integration policy checks.
- Keep token handling under existing user-owned/platform connector semantics.

## Acceptance Criteria
1. Integrations panel lists a `Tailscale` provider.
2. Tailscale dialog includes user-visible quick-start commands for:
- `tailscale up` with auth key.
- `tailscale funnel` for local browser/service entry.
3. Tailscale supports connector ownership mode switching, aligned with existing providers.
4. Cypress covers user-visible Tailscale connect path.

## Rollout / Rollback
- Rollout: additive provider metadata/store and dialog UI only.
- Rollback: remove Tailscale provider entries and dialog quick-start block.
