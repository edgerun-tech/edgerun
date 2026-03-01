# 2026-03-01 Google OAuth Local Bridge Fix v1

## Goal

Fix Google OAuth start/callback in osdev by routing `/api/google/oauth/*` through local node-manager endpoints instead of unavailable upstream paths.

## Non-goals

- No redesign of integrations OAuth UX.
- No replacement of Google token refresh semantics.
- No change to non-Google API routing.

## Security and Constraints

- Keep OAuth client credentials server-side only.
- Keep return path sanitized to local relative paths.
- Remove access token from URL after callback in browser state.

## Acceptance Criteria

1. `GET /api/google/oauth/start` no longer returns 404 in osdev.
2. OAuth callback exchanges code for token and redirects back to app.
3. Frontend captures callback token params, stores `google_token`, and cleans URL.
4. Node-manager and frontend checks pass.

## Rollout

- Add local OAuth start/callback handlers in node-manager.
- Update Caddy OAuth route rewrite to local bridge.
- Add callback param handling in integrations panel.

## Rollback

- Revert Caddy route and local OAuth handlers.
