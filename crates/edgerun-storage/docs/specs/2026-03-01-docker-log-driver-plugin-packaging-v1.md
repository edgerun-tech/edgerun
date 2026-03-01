# 2026-03-01 Docker Log Driver Plugin Packaging V1

## Goal
- Package `docker_log_driver` as a Docker-managed plugin bundle (`config.json` + `rootfs`) so operators can install/enable it with `docker plugin create`.

## Non-Goals
- Publishing to a plugin registry.
- Multi-arch image build pipeline.
- Implementing `ReadLogs` in the plugin API.

## Security / Constraints
- Plugin must bind to Docker plugin socket namespace only.
- Plugin must run with minimal privileges and no extra capabilities.
- Persistent data path must be explicit and operator-controlled.
- Generated artifacts must live under repository `out/`.

## Design
1. Add a packaging script that:
- builds `docker_log_driver` release binary,
- creates `out/docker-log-driver-plugin/rootfs/...`,
- emits `config.json` with log-driver interface type `docker.logdriver/1.0`.
2. Use a shell wrapper entrypoint inside plugin rootfs to convert env vars into CLI args for `docker_log_driver`.
3. Document install/enable/daemon default-log-driver flow in operator guide.

## Acceptance Criteria
1. Script generates bundle directory containing `config.json` and executable rootfs binaries.
2. `config.json` is valid JSON and declares `docker.logdriver/1.0` interface + socket.
3. Build/check/clippy for crate remain passing.
4. Operator guide includes exact commands for create/enable/use.

## Rollout / Rollback
- Rollout: create and enable plugin locally; set Docker daemon default log-driver or per-container log-driver.
- Rollback: disable/remove plugin and revert Docker log-driver setting to prior value.
