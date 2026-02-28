# 2026-03-01 OSDEV Output Root and Rebuild Watch v1

## Goal
Make `osdev.edgerun.tech` always serve the latest generated Intent UI build from a dedicated output path (`out/frontend/osdev`) instead of serving `frontend/public` directly.

## Non-goals
- Removing profile bootstrap UI flows from source in this change.
- Introducing additional frontend runtime behavior changes.

## Security/Constraints
- Keep `bun` as runtime/package manager.
- Keep generated artifacts under repo-root `out/`.
- Do not use port `8080`.
- Preserve local-bridge routing (`/v1/local/*`) fail-closed behavior.

## Acceptance Criteria
1. `osdev` frontend build output is written to `out/frontend/osdev`.
2. Caddy serves static root from `out/frontend/osdev`.
3. Dev frontend service rebuilds when source assets change and updates output artifacts.
4. `https://osdev.edgerun.tech/` returns `200` and loads the latest built shell.
5. `https://osdev.edgerun.tech/v1/local/node/info.pb` remains proxied and returns `200`.

## Rollout/Rollback
- Rollout: recreate `osdev-frontend`, `caddy`, and `cloudflared` services.
- Rollback: restore previous Caddy root (`frontend/public`) and previous frontend dev loop.
