# 2026-02-28 OS Dev Frontend Service V1

## Goal
- Add a long-running frontend dev service in the node-manager compose stack that continuously rebuilds frontend output and serves it for Cloudflare tunnel ingress.
- Expose the service at `osdev.edgerun.tech`.

## Non-Goals
- Replacing production frontend deploy pipelines.
- Using port `8080`.

## Security / Constraints
- Use Bun runtime only for JS workflows.
- Keep service bound to localhost host-network port and expose publicly only through Cloudflare tunnel ingress.

## Acceptance Criteria
1. Compose stack contains an `osdev-frontend` service that stays up and rebuilds frontend output on source changes.
2. Service serves static output on a non-8080 port (`4174`).
3. Tunnel ingress routes `osdev.edgerun.tech` to `http://127.0.0.1:4174`.
4. `scripts/node-manager-compose.sh up-tunnel` brings up the service and tunnel together.

## Rollout / Rollback
- Rollout: run `scripts/node-manager-compose.sh up-tunnel` and verify `https://osdev.edgerun.tech`.
- Rollback: remove `osdev-frontend` service + ingress rule and restart tunnel profile.
