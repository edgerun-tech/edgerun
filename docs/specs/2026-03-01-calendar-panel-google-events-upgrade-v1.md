# 2026-03-01 Calendar Panel Google Events Upgrade v1

## Goal
Replace the placeholder Calendar panel with a functional, token-aware calendar view backed by real Google Calendar events.

## Non-goals
- No write/create/update/delete calendar actions.
- No multi-provider calendar aggregation.
- No recurring event expansion logic beyond API payload.

## Security and Constraints
- Reuse existing Google token sources and avoid token logging.
- Use existing `/api/google/events` endpoint; no new backend routes.
- Keep panel behavior deterministic when token is missing or API fails.
- Do not use port 8080.

## Design
- Implement `CalendarPanel` with:
  - month header and day grid,
  - selected day state,
  - upcoming events list filtered by selected day/all,
  - refresh action and clear status/error messaging.
- Fetch events via `/api/google/events?limit=50&token=...`.
- Use reusable `VirtualAnimatedList` for event list consistency.
- Add Cypress test for panel load + event rendering using API intercept.

## Acceptance Criteria
1. Calendar panel fetches and displays Google events when token is available.
2. Missing-token and API-failure states are user-visible and actionable.
3. Event list rendering uses shared reusable list primitive.
4. Frontend check/build and targeted Cypress test pass.

## Rollout
- Ship panel implementation and Cypress test together.

## Rollback
- Revert calendar panel implementation and new test to prior placeholder.
