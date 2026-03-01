# 2026-03-01 Intent UI Weather Location Pattaya Default V1

## Goal
- Pin IntentBar weather location to Pattaya, Thailand.
- Ensure weather requests consistently use Pattaya coordinates.

## Non-Goals
- Building location selection UX.
- Adding geolocation-based weather switching.
- Changing weather visual styling.

## Security and Constraints
- Keep weather fetching read-only.
- Preserve deterministic behavior across sessions.
- Avoid adding dependencies.

## Design
1. Set runtime default weather coordinates to Pattaya, Thailand.
2. Make IntentBar weather refresh always resolve coordinates from the fixed default location.
3. Add Cypress coverage verifying the widget displays Pattaya location with weather payload fallback.

## Acceptance Criteria
1. Weather fetch resolves using Pattaya coordinates.
2. IntentBar location label shows `Pattaya, Thailand` when provider response lacks explicit location text.
3. Cypress test covers the fixed-location behavior.
4. Frontend validation commands pass when `bun` is available.

## Rollout
- Land coordinate pin + IntentBar resolver update + Cypress test.

## Rollback
- Revert default coordinates and resolver behavior to previous implementation.
