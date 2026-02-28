# 2026-02-28 Node Manager Cloudflared Config Ingress V1

## Goal
- Run Cloudflare Tunnel in the node-manager compose stack using a local `config.yml` ingress definition.
- Remove token-only tunnel startup from this stack.

## Non-Goals
- Managing Cloudflare DNS records automatically in this change.
- Replacing the existing systemd tunnel workflows.

## Security / Constraints
- Tunnel credentials stay local to the operator machine and are mounted read-only into the container.
- Keep node-manager local bridge on `127.0.0.1:7777`.
- Do not use port `8080`.

## Acceptance Criteria
1. `docker-compose.node-manager.yml` `cloudflared` service runs with `--config /etc/cloudflared/config.yml`.
2. Repo includes local tunnel config templates under `config/cloudflared/`.
3. Runbook documents tunnel creation, credentials placement, and `up-tunnel` startup.
4. `docker compose -f docker-compose.node-manager.yml config` validates.

## Rollout / Rollback
- Rollout: create tunnel credentials + config locally, then `scripts/node-manager-compose.sh up-tunnel`.
- Rollback: stop tunnel profile and revert to previous token-based compose definition.
