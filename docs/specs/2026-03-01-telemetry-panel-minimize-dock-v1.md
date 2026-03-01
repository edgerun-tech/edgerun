# 2026-03-01 Telemetry Panel Minimize Dock v1

## Goal

Make telemetry overlays (`Event Bus`, `Docker Logs`, `System State`) minimizable and restoreable from a stable dock so they do not pop over the workspace unexpectedly.

## Non-goals

- No changes to telemetry data collection/filtering logic.
- No redesign of panel content rows.
- No backend/API changes.

## Security and Constraints

- Keep existing drag/resize behavior for expanded panels.
- Keep persistence local-only (browser storage).
- Preserve panel render determinism and avoid hidden background polling.

## Acceptance Criteria

1. Each telemetry panel has a visible minimize control.
2. Minimized panels disappear from floating viewport and appear in a dock tray.
3. Dock tray allows restoring individual panels.
4. Minimized state persists across reloads.
5. Existing docker logs panel rendering remains functional.

## Rollout

- Add per-panel minimize control to floating panel shell.
- Add telemetry dock tray and minimized-state persistence.
- Extend Cypress coverage to verify minimize/restore behavior.

## Rollback

- Revert this slice to restore always-visible floating telemetry panels.
