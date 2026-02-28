# 2026-02-28 OS Dev Caddy Ingress V1

## Goal
- Add a Caddy service in the node-manager compose tunnel profile to serve frontend files and reverse-proxy local bridge + API paths.
- Make `osdev.edgerun.tech` terminate into Caddy instead of direct static server.

## Non-Goals
- Replacing Cloudflare tunnel.
- Changing node-manager bridge API routes.

## Security / Constraints
- Keep host networking for services that need localhost coupling.
- Keep node-manager bridge bound to `127.0.0.1:7777`.
- Do not use port `8080`.

## Acceptance Criteria
1. Compose includes `caddy` service in tunnel profile.
2. Caddy serves frontend static content as default route.
3. Caddy reverse-proxies `/v1/local/*` to `127.0.0.1:7777` (including websocket upgrades).
4. Tunnel ingress for `osdev.edgerun.tech` points to Caddy listener.

## Rollout / Rollback
- Rollout: `scripts/node-manager-compose.sh up-tunnel` and verify `https://osdev.edgerun.tech`.
- Rollback: remove caddy service + restore previous direct frontend ingress mapping.
