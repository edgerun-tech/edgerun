# Cloudflare Tunnel For Browser Terminal

This folder wires `term.edgerun.tech` to local `edgerun-term-server`.

## Prerequisites

- A Cloudflare Tunnel and DNS route for `term.edgerun.tech` already configured.
- Optional Cloudflare Access policy already configured if browser auth is required.

## Run term-server with route auto-registration

```bash
EDGERUN_HARDWARE_MODE=allow-software \
EDGERUN_TERM_SERVER_ADDR=127.0.0.1:5577 \
EDGERUN_TERM_PUBLIC_BASE_URL=https://term.edgerun.tech \
EDGERUN_ROUTE_CONTROL_BASE=https://api.edgerun.tech \
cargo run -p edgerun-term-server
```

## Run Cloudflare tunnel

In another shell:

```bash
scripts/cloudflared/run-term-tunnel.sh
```

Defaults:
- Upstream: `http://127.0.0.1:5577`
- Credentials file: `~/.cloudflared/<tunnel-id>.json`

Override with:
- `EDGERUN_TERM_SERVER_PORT`
- `EDGERUN_TERM_TUNNEL_UPSTREAM`
- `EDGERUN_TERM_TUNNEL_CREDENTIALS_FILE`

## Smoke checks

```bash
curl -fsS https://term.edgerun.tech/v1/device/identity | jq
curl -fsS https://api.edgerun.tech/v1/route/resolve/<device_id> | jq
```

When Access is enabled, unauthenticated browser requests to `https://term.edgerun.tech`
should redirect to `/cdn-cgi/access/login`.
