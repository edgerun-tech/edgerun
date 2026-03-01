# 2026-03-01 Tailscale Local Bridge Routing Fix v1

## Goal

Fix `404` failures for Tailscale integration verification by serving `/api/tailscale/devices` through local node-manager bridge routing instead of relying on unavailable upstream paths.

## Non-goals

- No automatic mutation of Tailscale routes or ACL policy.
- No persistent server-side storage of Tailscale API keys.
- No redesign of Integrations panel UX.

## Security and Constraints

- Keep Tailscale API key runtime-only and request-scoped.
- Keep fail-closed behavior for missing/invalid API key or tailnet.
- Keep local bridge loopback-only behavior.

## Acceptance Criteria

1. `POST /api/tailscale/devices` resolves through Caddy to node-manager local bridge.
2. Node-manager exposes `POST /v1/local/tailscale/devices` and returns `ok + devices` JSON payload.
3. Frontend verification no longer fails with 404 when `/api/tailscale/devices` is unavailable and can fall back to local path.
4. Node-manager tests and frontend required checks pass.

## Rollout

- Add node-manager local Tailscale devices proxy endpoint.
- Add Caddy rewrite for `/api/tailscale/*` to `/v1/local/tailscale/*`.
- Add frontend fallback path handling for verification calls.

## Rollback

- Revert node-manager endpoint, Caddy rewrite, and frontend fallback changes.
