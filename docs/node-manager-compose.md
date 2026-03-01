# Node Manager Compose Bootstrap

Run Edgerun as server-first bootstrap on a Linux host with TPM and Docker.

## 1) Configure env

```bash
cp config/node-manager.compose.env.example config/node-manager.compose.env
```

## 2) Start stack

```bash
scripts/node-manager-compose.sh up
```

This command first prepares host-built binaries for the image (`edgerun-node-manager`, `edgerun-worker`) and then builds the runtime image without Rust compilation inside Docker.

Optional (with Cloudflare Tunnel ingress):

1) Create a tunnel in your Cloudflare account and route DNS:

```bash
cloudflared tunnel create edgerun-node-manager
cloudflared tunnel route dns <TUNNEL_ID> <YOUR_NODE_HOSTNAME>
```

2) Put credentials and config in repo-local paths used by compose:

```bash
cp ~/.cloudflared/<TUNNEL_ID>.json config/cloudflared/node-manager-tunnel-credentials.json
cp config/cloudflared/node-manager-tunnel.yml.example config/cloudflared/node-manager-tunnel.yml
```

Edit `config/cloudflared/node-manager-tunnel.yml`:
- set `tunnel` to `<TUNNEL_ID>`
- set `ingress[0].hostname` to `<YOUR_NODE_HOSTNAME>`
- optionally set `ingress[1].hostname` to `osdeve2e.edgerun.tech` (or your e2e hostname)

3) Start tunnel profile:

```bash
scripts/node-manager-compose.sh up-tunnel
```

This starts:
- `node-manager` local bridge on `127.0.0.1:7777`
- `cloudflared` tunnel ingress
- `osdev-frontend` dev service (rebuilds Intent UI when source files change)
- `osdeve2e-manager` dev/e2e manager service (rebuilds Intent UI into `out/frontend/osdeve2e` and runs Cypress on file changes)
- `opencode-cli` container as the execution target for node-manager `docker exec ... opencode ...`
- `caddy` ingress service on `127.0.0.1:4175` serving generated output from `out/frontend/site` + proxying `/v1/local/*`, `/api/assistant` -> local backend, and `/api/*`

Host mapping through the same Caddy listener:
- `framework.bengal-salary.ts.net` -> `/workspace/out/frontend/site`

## 3) Pair device (one-time)

```bash
scripts/node-manager-compose.sh pair "<PAIRING_CODE>"
```

## 4) Verify bridge

```bash
curl -fsS http://127.0.0.1:7777/v1/local/node/info.pb -o /tmp/node-info.pb
```

Expected outcome:
- command exits `0`
- local bridge is reachable by browser UI on the same host

Optional full local stack verification:

```bash
scripts/verify-local-stack.sh
```

## 5) Enable Swarm and deploy services (optional)

```bash
scripts/swarm/deploy-services-stack.sh
```

This deploys `docker-compose.services.yml` as a swarm stack (`edgerun-services` by default).
Default services include:
- `whoami` sample service on `:9081`
- `registry:2` on `:5001` for local image push/pull workflows

Example push target:

```bash
docker tag your-image:latest 127.0.0.1:5001/your-image:latest
docker push 127.0.0.1:5001/your-image:latest
```

Cloud panel reads running swarm services and containers from:

```text
GET http://127.0.0.1:7777/v1/local/docker/summary
```

Local bridge MCP integration lifecycle endpoints:

```text
POST http://127.0.0.1:7777/v1/local/mcp/integration/start
POST http://127.0.0.1:7777/v1/local/mcp/integration/stop
GET  http://127.0.0.1:7777/v1/local/mcp/integration/status
```

Google productivity API proxy endpoints (used by Gmail/Drive/Contacts/Photos UI paths):

```text
GET  http://127.0.0.1:7777/v1/local/google/messages
GET  http://127.0.0.1:7777/v1/local/google/message/{id}
GET  http://127.0.0.1:7777/v1/local/google/events
GET  http://127.0.0.1:7777/v1/local/google/contacts
GET  http://127.0.0.1:7777/v1/local/google/drive/files
GET  http://127.0.0.1:7777/v1/local/google/drive/file/{id}
GET  http://127.0.0.1:7777/v1/local/google/photos
POST http://127.0.0.1:7777/v1/local/google/refresh
```

`/api/google/*` on the Caddy listener rewrites to these local bridge routes.

Default MCP image mappings in node-manager include:
- `github` -> `ghcr.io/github/github-mcp-server:latest`
- `google_messages` -> `dock.mau.dev/mautrix/gmessages:latest`
- `gvoice` -> `dock.mau.dev/mautrix/gvoice:latest`
- `googlechat` -> `dock.mau.dev/mautrix/googlechat:latest`

All can be overridden via `EDGERUN_MCP_*_IMAGE` in `config/node-manager.compose.env`.

## Notes
- This stack uses `network_mode: host` and `privileged: true` by design for TPM and Docker socket access.
- State is persisted in named volumes for `/var/lib/edgerun` and `/etc/edgerun`.
- Do not bind or use port `8080`.
- For logs and shell access:
- `scripts/node-manager-compose.sh logs`
- `scripts/node-manager-compose.sh shell`
- Explicit binary prep only:
- `scripts/node-manager-compose.sh prepare-binaries`
- Tunnel logs:
- `scripts/node-manager-compose.sh logs-tunnel`
- Tunnel service uses local config ingress at:
- `config/cloudflared/node-manager-tunnel.yml`
- Cloudflare Worker-side API functions are removed; browser API workflows must go through node-manager/local services.
