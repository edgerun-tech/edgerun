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

Optional (with Cloudflare Tunnel):

```bash
scripts/node-manager-compose.sh up-tunnel
```

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

## 5) Enable Swarm and deploy services (optional)

```bash
scripts/swarm/deploy-services-stack.sh
```

This deploys `docker-compose.services.yml` as a swarm stack (`edgerun-services` by default).
Cloud panel reads running swarm services and containers from:

```text
GET http://127.0.0.1:7777/v1/local/docker/summary
```

## Notes
- This stack uses `network_mode: host` and `privileged: true` by design for TPM and Docker socket access.
- State is persisted in named volumes for `/var/lib/edgerun` and `/etc/edgerun`.
- Do not bind or use port `8080`.
- For logs and shell access:
- `scripts/node-manager-compose.sh logs`
- `scripts/node-manager-compose.sh shell`
- Tunnel logs:
- `scripts/node-manager-compose.sh logs-tunnel`
- Cloudflare Worker-side API functions are removed; browser API workflows must go through node-manager/local services.
