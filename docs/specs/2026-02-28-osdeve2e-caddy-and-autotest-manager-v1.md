# 2026-02-28 osdeve2e Caddy Route and Auto-Test Manager V1

## Goal
Add an `osdeve2e.edgerun.tech` route and a dedicated compose service that continuously rebuilds Intent UI and runs e2e checks automatically.

## Non-Goals
- Replacing the existing `osdev.edgerun.tech` workflow.
- Changing production deployment topology.
- Opening new public listeners beyond existing tunnel/caddy flow.

## Security and Constraints
- Keep ingress through existing Caddy + Cloudflare tunnel path.
- No port `8080` usage.
- Keep local bridge routes unchanged and loopback-only.
- Use Bun workflows for frontend build/test automation.

## Acceptance Criteria
1. Cloudflared ingress includes `osdeve2e.edgerun.tech`.
2. Caddy serves `osdeve2e.edgerun.tech` from a dedicated output root.
3. Compose includes a long-running manager service that rebuilds the e2e output and executes e2e checks on source changes.
4. Existing `osdev.edgerun.tech` route keeps current behavior.

## Rollout
- Deploy updated compose stack and tunnel config.
- Validate host routing and manager logs.

## Rollback
- Revert compose, caddy, and tunnel config updates.
