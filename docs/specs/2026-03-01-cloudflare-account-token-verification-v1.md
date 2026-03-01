# 2026-03-01 Cloudflare Account Token Verification v1

## Goal

Make Cloudflare integration reliably work with account API tokens by verifying tokens through local bridge endpoints instead of unavailable upstream `/api/cloudflare/*` routes.

## Non-goals

- No automatic Cloudflare resource provisioning in this slice.
- No long-lived server-side storage of Cloudflare tokens beyond existing integration token persistence behavior.
- No redesign of Cloud panel resource fetch flows.

## Security and Constraints

- Keep token handling request-scoped for verification calls.
- Keep fail-closed behavior for missing/invalid Cloudflare token.
- Keep local bridge loopback-only network boundary.

## Acceptance Criteria

1. Integrations Cloudflare verification accepts account API token and returns deterministic success/failure.
2. Caddy routes `/api/cloudflare/verify` to local bridge endpoint.
3. Node-manager exposes local Cloudflare token verify endpoint.
4. Integrations UI clarifies expected credential as Cloudflare account API token.
5. Frontend and node-manager validations pass.

## Rollout

- Add local bridge endpoint that proxies Cloudflare token verify API.
- Add Caddy route override for `/api/cloudflare/verify`.
- Update Integrations verification code (worker + lifecycle) and copy.

## Rollback

- Revert this slice to previous generic token-accepted behavior.
