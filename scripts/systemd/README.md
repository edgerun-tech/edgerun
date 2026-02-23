# Edgerun Systemd User Services

This sets up an internal-only Edgerun stack with:
- `edgerun-scheduler.service`
- `edgerun-worker@.service` (templated workers)
- `edgerun-term-server.service`
- `edgerun-cloudflared-term.service`

Default bind is loopback only:
- Scheduler: `127.0.0.1:5566`

## Prerequisites
- Binaries built at `out/target/release/`:
  - `edgerun-scheduler`
  - `edgerun-worker`
  - `edgerun-term-server`
- User systemd session running (`systemctl --user status`)
- `cloudflared` installed and tunnel credentials at `~/.cloudflared/<tunnel-id>.json`

## Install Unit Files + Env Templates

```bash
./scripts/systemd/install-user-services.sh
```

This creates:
- `~/.config/systemd/user/edgerun-scheduler.service`
- `~/.config/systemd/user/edgerun-worker@.service`
- `~/.config/systemd/user/edgerun-term-server.service`
- `~/.config/systemd/user/edgerun-cloudflared-term.service`
- `~/.config/edgerun/scheduler.env`
- `~/.config/edgerun/worker-common.env`
- `~/.config/edgerun/workers/{1,2,3}.env`
- `~/.config/edgerun/term-server.env`
- `~/.config/edgerun/cloudflared-term.env`

## Start Stack (Scheduler + 3 Workers)

```bash
./scripts/systemd/start-user-stack.sh 3
```

The startup scripts now enable user lingering for boot-persistent services if possible.

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
