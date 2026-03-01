# 2026-03-01 Google Productivity Local Bridge v1

## Goal

Make Gmail, Drive, Contacts, Calendar, and Photos flows work reliably in local Intent UI by routing Google API calls through node-manager local bridge endpoints.

## Non-goals

- No migration of provider auth UX away from existing Google integration UI.
- No write operations for Drive in this slice.
- No replacement of remote `api.edgerun.tech` for unrelated API paths.

## Security and Constraints

- Token-backed calls must require explicit bearer token input per request.
- Keep local bridge loopback-only and CORS behavior unchanged.
- Keep provider behavior fail-closed on upstream Google API errors.
- Do not use port `8080`.

## Acceptance Criteria

1. Node-manager exposes local Google proxy routes for Gmail messages/details, contacts, calendar events, Drive file list/read, photos list, and refresh token exchange.
2. osdev Caddy routes `/api/google/*` to node-manager local Google routes.
3. Compose/env docs include optional Google OAuth client credentials needed for refresh endpoint.
4. Node-manager tests remain passing and no existing workspace tests regress.

## Rollout

- Land spec + catalog update.
- Land node-manager route handlers and Caddy rewrites.
- Validate with node-manager and workspace checks/tests.

## Rollback

- Revert this slice to restore `/api/google/*` pass-through behavior to remote API only.
