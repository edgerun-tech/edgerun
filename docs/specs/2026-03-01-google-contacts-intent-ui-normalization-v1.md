# 2026-03-01 Google Contacts Intent UI Normalization v1

## Goal

Make Google Contacts show reliably in Intent UI conversation sources by normalizing People API response shapes into stable `name` and `email` fields.

## Non-goals

- No change to OAuth onboarding flow for Google integrations.
- No write operations against Google People API.
- No change to non-contact Google surfaces (Gmail, Drive, Calendar, Photos).

## Security and Constraints

- Keep token handling unchanged (token still provided explicitly by caller).
- Keep fail-closed behavior on upstream errors.
- Avoid leaking raw API failures into unstable UI state.

## Acceptance Criteria

1. Contacts payloads shaped as People API `connections` entries (`names`, `emailAddresses`, `resourceName`) render usable contacts in Conversations.
2. Legacy contact payload fields (`name`, `email`, `emails`) continue to work.
3. Failed `/api/google/contacts` responses do not leave stale contacts in UI state.
4. Frontend check/build remain passing.

## Rollout

- Land spec and status catalog entry.
- Land contact normalization in conversation source loader.
- Run frontend validation checks.

## Rollback

- Revert this slice to restore previous contact parsing behavior.
