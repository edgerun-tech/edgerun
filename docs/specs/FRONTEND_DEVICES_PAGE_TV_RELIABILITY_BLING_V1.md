# Frontend Devices Page TV Reliability + Bling V1

## Goal and non-goals
- Goal:
  - Ensure the devices experience renders reliably on TV browsers.
  - Keep the devices dashboard visually rich ("latest bling") while reducing render risk.
  - Support both `/devices/` and `/device/` route paths.
- Non-goals:
  - No changes to terminal drawer protocol or scheduler transport behavior.
  - No new backend services or runtime dependencies.
  - No mocked on-chain data changes (devices page remains non-chain telemetry presentation).

## Security and constraints
- Keep implementation deterministic and static-render friendly.
- Preserve `frontend/` as the canonical app root.
- Keep existing `data-testid` coverage and extend it for new controls.
- Use Bun workflows only for validation.

## Acceptance criteria
- Visiting `/devices/` renders a working devices dashboard on desktop/TV viewport sizes.
- Visiting `/device/` renders the same devices dashboard (route alias).
- Page includes user-visible controls (status filter + search + bling toggle) that update visible fleet rows.
- Cypress validates key panels and confirms filter controls exist and are functional.

## Rollout and rollback
- Rollout: ship additive route alias and devices page refresh, then run `bun run check`, `bun run build`, and a targeted Cypress spec.
- Rollback: revert `frontend/app/devices/page.tsx`, route map entries, and devices Cypress assertions if regressions are observed.
