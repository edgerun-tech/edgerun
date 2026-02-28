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
- User systemd session running (`systemctl --user status`)
- `cloudflared` installed and tunnel credentials at `~/.cloudflared/<tunnel-id>.json`

## Canonical Binary Build Path

Always build service binaries into `out/target/release`:

```bash
./scripts/systemd/build-user-binaries.sh all
```

Modes:
- `all` (default): scheduler + worker + term-server
- `stack`: scheduler + worker
- `terminal`: term-server

`start-user-stack.sh` and `start-user-terminal.sh` run this build step automatically before service start/restart, so systemd units do not drift from source changes.

This installer now also validates the rendered config values for required keys, address formats, ports, and UUID/base64-like fields, then fails fast if anything is invalid.

## Install Unit Files + Env Templates

```bash
./scripts/systemd/install-user-services.sh
```

This creates:
- `~/.config/systemd/user/edgerun-scheduler.service`
- `~/.config/systemd/user/edgerun-worker@.service`
- `~/.config/systemd/user/edgerun-term-server.service`
- `~/.config/systemd/user/edgerun-cloudflared-term.service`- `~/.config/edgerun/scheduler.env`
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

## System Services (Containerd Runtime Path)

For host-level containerd integration, install EdgeRun snapshotter + shim backend units:

```bash
sudo ./scripts/systemd/install-system-containerd-services.sh
```

Options:

```bash
sudo ENABLE_SERVICES=0 ./scripts/systemd/install-system-containerd-services.sh
sudo RESTART_CONTAINERD=1 ./scripts/systemd/install-system-containerd-services.sh
sudo INSTALL_BINARIES=0 ./scripts/systemd/install-system-containerd-services.sh
```

Installed system units:
- `/etc/systemd/system/edgerun-snapshotter.service`
- `/etc/systemd/system/edgerun-shim-backend.service`

Binary layout for containerd compatibility:
- backend daemon: `/usr/lib/edgerun/containerd-shim-edgerun-backend`
- containerd shim entrypoints: `/usr/bin/containerd-shim-edgerun-v2` and `/usr/bin/containerd-shim-edgerun-v1` (compat alias to v2 binary)

Status checks:

```bash
systemctl --no-pager --full status edgerun-snapshotter.service edgerun-shim-backend.service
journalctl -u edgerun-snapshotter.service -f
journalctl -u edgerun-shim-backend.service -f
```

Apply containerd runtime snippet + verify effective config:

```bash
sudo ./scripts/containerd/apply-edgerun-runtime-config.sh
sudo RESTART_CONTAINERD=1 RUN_SMOKE=1 ./scripts/containerd/apply-edgerun-runtime-config.sh
```

Run WASM smoke under the EdgeRun runtime class:

```bash
./scripts/containerd-runtime-wasm-smoke.sh
```

## System Core Services (Always-On Scheduler/Term/STUN Node)

For a minimal headless host that should keep control-plane routing online 24/7, install:

- `edgerun-scheduler.service`
- `edgerun-term-server.service`
- `coturn.service` (STUN-only baseline)
- `edgerun-healthcheck.timer` + `edgerun-healthcheck.service`

Install on a Debian-like host:

```bash
sudo PUBLIC_IP=172.245.67.49 ./scripts/systemd/install-system-core-services.sh
```

Important environment knobs:

- `INSTALL_BINARIES=0` to skip local cargo build/install.
- `INSTALL_PACKAGES=0` to skip apt package installation.
- `ENABLE_SERVICES=0` to install files without starting/enabling services.
- `FORCE_OVERWRITE_ENV=1` to regenerate `/etc/edgerun/{scheduler,term-server}.env`.
- `EDGERUN_ROOT=/opt/edgerun` and `EDGERUN_ETC_DIR=/etc/edgerun` to change install paths.

Generated configs and units:

- `/etc/edgerun/scheduler.env`
- `/etc/edgerun/term-server.env`
- `/etc/systemd/system/edgerun-scheduler.service`
- `/etc/systemd/system/edgerun-term-server.service`
- `/etc/turnserver.conf`
- `/etc/systemd/system/coturn.service.d/10-liveness.conf`
- `/usr/local/bin/edgerun-healthcheck.sh`
- `/etc/systemd/system/edgerun-healthcheck.service`
- `/etc/systemd/system/edgerun-healthcheck.timer`

Post-install checks:

```bash
systemctl --no-pager --full status edgerun-scheduler edgerun-term-server coturn edgerun-healthcheck.timer
/usr/local/bin/edgerun-healthcheck.sh
```
