# FRONTEND_DEVICES_DEMO_DASHBOARD_V1

## Goal
Provide a dedicated `/devices/` page that renders a dense demo operations dashboard optimized for large 4K displays, so operators can quickly scan many device panels at once.

## Non-Goals
- No live backend/RPC integration is introduced in this change.
- No replacement of the existing `/dashboard/` chain metrics page.
- No new client-side state stores or websocket subscriptions.

## Security and Constraints
- Use existing frontend architecture and route system under `frontend/`.
- Keep the view deterministic with static demo data only.
- Preserve centralized theming tokens and avoid introducing parallel app roots.
- Keep dependency footprint unchanged.

## Acceptance Criteria
1. A new route `/devices/` is available in static and client route maps.
2. The page presents a high-density, multi-panel layout suitable for 3840x2160 usage.
3. The page includes user-visible dashboard sections (fleet KPIs, device table, alerts, service health, and command queue).
4. Navigation exposes the Devices page.
5. Cypress coverage verifies core user-visible sections render on `/devices/`.
6. Frontend checks and build pass:
   - `cd frontend && bun run check`
   - `cd frontend && bun run build`

## Rollout / Rollback
- Rollout: ship as static route addition; no migration required.
- Rollback: remove `/devices/` route entries and page file, and remove its nav/test additions.
