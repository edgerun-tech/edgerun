# Edgerun Systemd User Services

This sets up an internal-only Edgerun stack with:
- `edgerun-scheduler.service`
- `edgerun-worker@.service` (templated workers)
- `edgerun-term-server.service`
- `edgerun-cloudflared-term.service`

Default bind is loopback only:
- Scheduler: `127.0.0.1:5566`

By default, configuration is rendered from:

- `scripts/systemd/env/profiles/local`

Set another profile via argument:

```bash
./scripts/systemd/install-user-services.sh dev
```

Machine-specific overrides (optional):

- `~/.config/edgerun/scheduler.override.env`
- `~/.config/edgerun/worker-common.override.env`
- `~/.config/edgerun/workers/{1,2,3}.override.env`
- `~/.config/edgerun/term-server.override.env`
- `~/.config/edgerun/cloudflared-term.override.env`

## Prerequisites
- Binaries built at `out/target/release/`:
  - `edgerun-scheduler`
  - `edgerun-worker`
  - `edgerun-term-server`
- User systemd session running (`systemctl --user status`)
- `cloudflared` installed and tunnel credentials at `~/.cloudflared/<tunnel-id>.json`

This installer now also validates the rendered config values for required keys, address formats, ports, and UUID/base64-like fields, then fails fast if anything is invalid.

## Install Unit Files + Env Templates

```bash
./scripts/systemd/install-user-services.sh
```

This creates:
- `~/.config/systemd/user/edgerun-scheduler.service`
- `~/.config/systemd/user/edgerun-worker@.service`
- `~/.config/systemd/user/edgerun-term-server.service`
- `~/.config/systemd/user/edgerun-cloudflared-term.service`
- `~/.config/systemd/user/solana-test-validator.service`
- `~/.config/edgerun/scheduler.env`
- `~/.config/edgerun/worker-common.env`
- `~/.config/edgerun/workers/{1,2,3}.env`
- `~/.config/edgerun/term-server.env`
- `~/.config/edgerun/cloudflared-term.env`

## Start Stack (Scheduler + 3 Workers)

```bash
./scripts/systemd/start-user-stack.sh 3 dev
./scripts/systemd/start-user-stack.sh --profile dev --workers 3
```

The startup script now enables user lingering for boot-persistent services if possible.

```bash
loginctl show-user "$USER" -p Linger --value
```

Expected output:

```text
yes
```

Health:

```bash
curl -fsS http://127.0.0.1:5566/health
```

## Start Terminal Routing Services

```bash
./scripts/systemd/start-user-terminal.sh
```

The terminal services also require the same boot persistence check:

```bash
loginctl show-user "$USER" -p Linger --value
```

Check:

```bash
systemctl --user --no-pager --full status edgerun-term-server.service edgerun-cloudflared-term.service
curl -I https://term.edgerun.tech
```

## Verify Boot Persistence

```bash
systemctl --user is-enabled edgerun-scheduler.service
systemctl --user is-enabled edgerun-worker@1.service
systemctl --user is-enabled edgerun-term-server.service
systemctl --user is-enabled edgerun-cloudflared-term.service
```

All outputs should be `enabled`.

## Local Solana Test Validator (Local Dev Profile)

If you use local chain mode, start it after install:

```bash
systemctl --user enable --now solana-test-validator.service
```

Expected log path:

```bash
journalctl --user -u solana-test-validator.service -f
```

## Service Controls

```bash
systemctl --user restart edgerun-scheduler.service
systemctl --user restart edgerun-worker@1.service

systemctl --user stop edgerun-worker@1.service edgerun-worker@2.service edgerun-worker@3.service
systemctl --user stop edgerun-scheduler.service

journalctl --user -u edgerun-scheduler.service -f
journalctl --user -u edgerun-worker@1.service -f
journalctl --user -u edgerun-term-server.service -f
journalctl --user -u edgerun-cloudflared-term.service -f
```
