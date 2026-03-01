# 2026-03-01 Intent UI Google Photos Panel Surfacing V1

## Goal
- Surface Google Photos as a first-class panel from IntentBar interactions.
- Allow users to open a dedicated Photos window via quick action and command.

## Non-Goals
- Building a native Google Photos API browser in this change.
- Introducing new backend APIs for photos metadata.
- Altering existing Gmail/Drive panel behavior.

## Security and Constraints
- Reuse existing browser proxy surface for external pages.
- Keep behavior deterministic and local-first in window state.
- Do not add external dependencies.

## Design
1. Add a new window id `photos` to window state typings/presets/positions.
2. Render `photos` windows via `BrowserApp` with `https://photos.google.com` as initial URL.
3. Surface Photos in IntentBar:
   - quick action icon button,
   - command routes (`google photos`, `open photos`, `photos`).
4. Add a Cypress regression to verify IntentBar can open the Photos window.

## Acceptance Criteria
1. Clicking the Photos quick action in IntentBar opens a `Photos` window.
2. Entering `google photos` in IntentBar opens the same window.
3. Photos window launches on `https://photos.google.com` through browser proxy flow.
4. Frontend validation commands pass once `bun` is available.

## Rollout
- Land window-id surfacing + IntentBar entry points + Cypress coverage.

## Rollback
- Revert Photos window-id additions and IntentBar command/button paths.
