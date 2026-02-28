# Cloudflare Tunnel For Browser Terminal

This folder wires `term.edgerun.tech` to local `edgerun-term-server`.

## One-time DNS setup

```bash
scripts/cloudflared/setup-term-tunnel-dns.sh
```

Defaults:
- Tunnel ID: `c5fab4c7-c68e-411e-880b-c1da359a18e9`
- Hostname: `term.edgerun.tech`

Override with:
- `EDGERUN_TERM_TUNNEL_ID`
- `EDGERUN_TERM_TUNNEL_HOSTNAME`

## Cloudflare Access (Zero Trust) setup

Use an API token with Access app/policy write permissions:

```bash
export CF_ACCOUNT_ID=<cloudflare-account-id>
export CF_API_TOKEN=<zero-trust-access-api-token>
scripts/cloudflared/setup-term-access.sh
```

Optional knobs:
- `EDGERUN_TERM_ACCESS_HOSTNAME` (default: `term.edgerun.tech`)
- `EDGERUN_TERM_ACCESS_APP_NAME` (default: `edgerun-term`)
- `EDGERUN_TERM_ACCESS_POLICY_NAME` (default: `allow-edgerun-term`)
- `EDGERUN_TERM_ACCESS_ALLOW_DOMAIN` (default: `edgerun.tech`)
- `EDGERUN_TERM_ACCESS_ALLOW_EMAIL` (if set, takes precedence over domain allow)
- `EDGERUN_TERM_ACCESS_SESSION_DURATION` (default: `24h`)

## Run term-server with route auto-registration

```bash
EDGERUN_HARDWARE_MODE=allow-software \
EDGERUN_TERM_SERVER_ADDR=127.0.0.1:5577 \
EDGERUN_TERM_PUBLIC_BASE_URL=https://term.edgerun.tech \
EDGERUN_ROUTE_CONTROL_BASE=https://api.edgerun.tech \
cargo run -p edgerun-term-server --features term
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
