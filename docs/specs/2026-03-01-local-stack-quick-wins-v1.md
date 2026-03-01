# 2026-03-01 Local Stack Quick Wins V1

## Goal and non-goals
- Goal:
  - Add high-impact operator quick wins for local stack reliability and conversations UX speed.
  - Remove lingering `osdev` naming on Caddy config labels that now serve the framework tailnet domain.
  - Add a minimal host-routing smoke test to catch Caddy host-header regressions early.
- Non-goals:
  - Rework full compose service naming across all containers in this pass.
  - Add certificate rotation automation (explicitly deferred).

## Security and constraint requirements
- Keep certificate files local to repo `out/` artifacts and do not expose private keys in logs.
- Keep runtime checks read-only except for explicit service restarts.
- Preserve existing proxy behavior for `/v1/local/*` and `/api/*` routes.

## Acceptance criteria
1. Add one-command local verification script for Caddy + bridge routing and service state checks.
2. Rename Caddy config file reference away from `osdev` naming and keep Docker Caddy loading the renamed file.
3. Conversations thread source/search filters persist across reloads.
4. Pressing `/` focuses thread search in Conversations (when not typing in another input).
5. Add Cypress smoke test validating framework host-header routing through Caddy listener.
6. Validation passes:
   - `cd frontend && bun run check`
   - `cd frontend && bun run build`
   - targeted Cypress specs for conversations and framework host routing

## Rollout and rollback
- Rollout:
  - Add local verify script.
  - Rename Caddyfile and update compose mount.
  - Ship persisted filters + `/` shortcut in conversations.
  - Add Caddy host routing smoke Cypress spec.
- Rollback:
  - Restore prior Caddy filename/mount and remove quick-win script + UI persistence/shortcut changes.
