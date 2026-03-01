# 2026-03-01 Google Photos Native Panel v2

## Goal

Replace the browser-embed Photos window with a native Google Photos panel that reads media items from local bridge API.

## Non-goals

- No full Google Photos album management.
- No media write/delete operations.
- No background sync daemon.

## Security and Constraints

- Keep token handling request-scoped and local-only.
- Keep fail-closed behavior when token/API is unavailable.
- Avoid adding heavyweight photo libraries.

## Acceptance Criteria

1. Opening `photos` window renders a native panel, not BrowserApp.
2. Panel fetches from `/api/google/photos` using existing Google token.
3. Panel displays preview thumbnails and basic metadata for returned items.
4. Panel provides clear empty/error state when token is missing or API fails.
5. Frontend checks/build and targeted Cypress pass.

## Rollout

- Add `GooglePhotosPanel` component.
- Route WindowManager `photos` window id to the new panel.
- Update Photos Cypress spec to assert native panel behavior.

## Rollback

- Revert WindowManager route and panel/test additions.
