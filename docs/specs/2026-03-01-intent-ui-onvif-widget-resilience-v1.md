# 2026-03-01 Intent UI ONVIF Widget Resilience V1

## Goal and non-goals
- Goal:
  - Make ONVIF panel scan behavior deterministic when ONVIF discovery backend is unavailable.
  - Prefer node-local bridge discovery path for scan attempts while preserving legacy API compatibility.
  - Keep manual camera add flow fully functional even when scan endpoint is missing.
- Non-goals:
  - Implement a full ONVIF discovery service in this change.
  - Add ONVIF auth/session management or PTZ control.

## Security and constraint requirements
- Do not persist embedded credentials from user-entered camera URLs.
- Keep ONVIF scan calls read-only and bounded to explicit endpoints.
- Surface explicit operator-facing error/status text for unavailable scan backends.

## Acceptance criteria
1. ONVIF scan attempts node-local discovery first (`/v1/local/onvif/discover`) and then legacy `/api/onvif/discover` fallback.
2. If neither discovery endpoint is available, panel shows actionable status text instead of generic failure.
3. Manual camera add remains available and produces normalized URLs.
4. Add/update Cypress coverage that opens ONVIF panel, runs scan with mocked response, and adds a scanned camera.
5. Frontend validation passes:
- `cd frontend && bun run check`
- `cd frontend && bun run build`

## Rollout and rollback
- Rollout:
  - Update ONVIF panel scan path + status behavior.
  - Add ONVIF panel E2E regression coverage.
- Rollback:
  - Revert ONVIF panel scan fallback/status changes and ONVIF Cypress spec.
