# 2026-03-01 Intent UI IntentBar Weather Fallback V1

## Goal
- Restore reliable weather updates in IntentBar when `/api/weather` is unavailable.
- Keep weather status visible and fresh using deterministic fallback behavior.

## Non-Goals
- Building a full weather app or adding new weather providers beyond a single fallback source.
- Changing IntentBar visual design beyond minimal test hooks.
- Introducing new dependencies.

## Security and Constraints
- Keep network access read-only and limited to weather endpoints.
- Preserve existing local weather snapshot persistence semantics.
- Avoid failing the whole IntentBar flow when weather fetch fails.

## Design
1. Keep current `/api/weather` call as primary source.
2. Add fallback fetch to Open-Meteo when primary route fails.
3. Normalize fallback payload to existing weather shape used by IntentBar.
4. Add minimal `data-testid` hooks for stable Cypress weather assertions.
5. Add an explicit weather command (`weather`, `weather now`, `refresh weather`) in IntentBar to trigger a manual refresh.

## Acceptance Criteria
1. Weather in IntentBar updates when `/api/weather` fails but Open-Meteo succeeds.
2. Manual weather command triggers refresh and does not throw user-facing errors on success.
3. Cypress test proves fallback path renders weather temperature in IntentBar.
4. Frontend validation commands pass once `bun` is available.

## Rollout
- Land fallback and command support in IntentBar.
- Add Cypress regression test for fallback path.

## Rollback
- Revert IntentBar fallback/command additions and Cypress spec.
