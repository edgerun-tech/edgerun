# Edgerun Systemd User Services

This sets up an internal-only Edgerun stack with:
- `edgerun-scheduler.service`
- `edgerun-worker@.service` (templated workers)

Default bind is loopback only:
- Scheduler: `127.0.0.1:5566`

## Prerequisites
- Binaries built at `out/target/release/`:
  - `edgerun-scheduler`
  - `edgerun-worker`
- User systemd session running (`systemctl --user status`)

## Install Unit Files + Env Templates

```bash
./scripts/systemd/install-user-services.sh
```

This creates:
- `~/.config/systemd/user/edgerun-scheduler.service`
- `~/.config/systemd/user/edgerun-worker@.service`
- `~/.config/edgerun/scheduler.env`
- `~/.config/edgerun/worker-common.env`
- `~/.config/edgerun/workers/{1,2,3}.env`

## Start Stack (Scheduler + 3 Workers)

```bash
./scripts/systemd/start-user-stack.sh 3
```

Health:

```bash
curl -fsS http://127.0.0.1:5566/health
```

## Service Controls

```bash
systemctl --user restart edgerun-scheduler.service
systemctl --user restart edgerun-worker@1.service

systemctl --user stop edgerun-worker@1.service edgerun-worker@2.service edgerun-worker@3.service
systemctl --user stop edgerun-scheduler.service

journalctl --user -u edgerun-scheduler.service -f
journalctl --user -u edgerun-worker@1.service -f
```
