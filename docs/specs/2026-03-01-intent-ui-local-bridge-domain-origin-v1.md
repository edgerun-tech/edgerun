# 2026-03-01 Intent UI Local Bridge Domain Origin v1

## Goal
Route browser local-bridge/eventbus traffic via the active Intent UI origin (for deployed usage: `osdev.edgerun.tech`) instead of hardcoded `127.0.0.1:7777`.

## Non-goals
- Removing device-side local bridge listen address requirements.
- Changing node-manager bridge protocol or paths.

## Security/Constraints
- Keep fail-closed behavior when bridge is unreachable.
- Keep `/v1/local/*` transport behind Caddy/cloudflared ingress.
- No fallback-only mode.

## Acceptance Criteria
1. Eventbus WebSocket URL resolves from web origin and path `/v1/local/eventbus/ws`.
2. Node info and docker summary HTTP calls resolve from web origin and `/v1/local/*`.
3. Local bridge error UI shows the resolved endpoint.
4. `https://osdev.edgerun.tech/v1/local/node/info.pb` remains reachable.

## Rollout/Rollback
- Rollout by recreating `osdev-frontend`, `caddy`, `cloudflared`.
- Rollback by restoring hardcoded localhost bridge URLs.
