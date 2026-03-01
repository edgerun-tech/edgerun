# 2026-03-01 Intent UI ONVIF Widget Resilience V1

## Goal and non-goals
- Goal:
  - Make ONVIF panel scan behavior deterministic by providing a node-local ONVIF discovery backend and explicit UI fallback behavior.
  - Prefer node-local bridge discovery path for scan attempts while preserving legacy API compatibility.
  - Keep manual camera add flow fully functional even when scan endpoint is missing.
  - Resolve discovered ONVIF service endpoints to operator-usable stream URLs (default `stream1` path).
  - Prefer direct frontend ONVIF query for stream URI resolution after discovery (no stream relay/proxy in backend path).
  - Allow operator-defined default ONVIF stream auth settings (username/password) without persisting credentials inside per-camera URLs.
- Non-goals:
  - Implement a full ONVIF discovery service in this change.
  - Add ONVIF auth/session management or PTZ control.

## Security and constraint requirements
- Do not persist embedded credentials from user-entered camera URLs.
- Keep ONVIF scan calls read-only and bounded to explicit endpoints.
- Surface explicit operator-facing error/status text for unavailable scan backends.
 - Keep default credentials isolated in ONVIF panel defaults storage and apply them only at runtime for preview/open actions.

## Acceptance criteria
1. ONVIF scan attempts node-local discovery first (`/v1/local/onvif/discover`) and then legacy `/api/onvif/discover` fallback.
2. Local bridge provides `/v1/local/onvif/discover` via WS-Discovery probe and returns normalized candidate URLs.
3. When WS-Discovery payloads include `rtsp://...` stream URIs, local bridge includes `streamUrl` hints and ONVIF panel prefers those over default service-path mapping.
4. ONVIF panel attempts direct frontend ONVIF Media query (`GetProfiles` + `GetStreamUri`) against discovered service URLs and uses returned RTSP URI when available.
5. If neither discovery endpoint is available, panel shows actionable status text instead of generic failure.
6. Manual camera add remains available and produces normalized URLs.
7. Add/update Cypress coverage that opens ONVIF panel, runs scan with mocked response, and adds a scanned camera.
8. Backend validation passes:
- `cargo check -p edgerun-node-manager`
9. ONVIF panel maps scanned `/onvif/device_service` URLs to stream URLs using configurable defaults (`protocol`, `stream path`, optional auth) and stores camera entries without embedded credentials.
10. Frontend validation passes:
- `cd frontend && bun run check`
- `cd frontend && bun run build`

## Rollout and rollback
- Rollout:
  - Add node-local ONVIF WS-Discovery endpoint.
  - Update ONVIF panel scan path + status behavior.
  - Add ONVIF stream defaults controls and runtime stream URL/auth mapping.
  - Add ONVIF panel E2E regression coverage.
- Rollback:
  - Revert node-local ONVIF discovery endpoint, ONVIF panel scan fallback/status changes, and ONVIF Cypress spec.
