# 2026-03-01 Cloudflare Auth Assist Flow v1

## Goal

Make Cloudflare integration setup feel like an auth flow by guiding the user through Cloudflare login/token creation, then surfacing logged-in account identity after verification.

## Non-goals

- No server-side storage of Cloudflare username/password.
- No replacement of Cloudflare-issued account API tokens.
- No automatic Cloudflare token minting without user account login.

## Security and Constraints

- Keep Cloudflare credential handling token-based and fail-closed.
- Do not log token values.
- Keep local bridge verification deterministic.

## Acceptance Criteria

1. Cloudflare setup shows an explicit "Sign in to Cloudflare" assist action in Step 1.
2. Verification returns account identity when available and reflects logged-in label in UI.
3. Integrations store forwards verification account label to the panel.
4. Frontend + node-manager checks and targeted Cypress tests pass.

## Rollout

- Add Cloudflare auth assist controls to Integrations panel.
- Extend local bridge Cloudflare verify endpoint with optional account identity lookup.
- Propagate account label from verify response through store and panel state.

## Rollback

- Revert Cloudflare assist UI and account-label propagation changes.
