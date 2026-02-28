# NODE_MANAGER_DOCKER_COMPOSE_BOOTSTRAP_V1

## Goal
- Make the device/server bootstrap path primary by running `edgerun-node-manager` via Docker Compose on the host.
- Ensure node manager has direct access to:
- Docker socket for immediate job/service execution orchestration.
- TPM device(s) for hardware-backed identity and secret material.
- Local bridge endpoint for browser/control-plane connectivity on `127.0.0.1:7777`.
- Swarm services stack that includes a local Docker registry service for image push/pull workflows.

## Non-Goals
- Replace existing systemd flow in this change.
- Redesign scheduler/worker internals.
- Introduce cloud-hosted bootstrap as a required dependency.

## Security / Constraints
- Keep TPM required semantics (`HardwareSecurityMode::TpmRequired`) intact.
- Keep local bridge reachable on localhost, fail-closed when TPM is unavailable.
- Avoid port `8080`.
- Persist state/config on host-backed volumes (`/var/lib/edgerun`, `/etc/edgerun`).
- Do not compile Rust binaries in Docker image build for node-manager runtime; only copy host-built artifacts.

## Acceptance Criteria
- Repo includes a Compose stack that starts `edgerun-node-manager` as the primary process.
- Stack mounts Docker socket and host `/dev` (TPM path visibility).
- Stack can be configured using a simple env file and runbook.
- Node-manager image Dockerfile copies prebuilt host binaries and does not run `cargo build`.
- Compose config validates (`docker compose config`) and local image build validates using copied binaries.
- Swarm services stack includes a `registry` service (`registry:2`) exposed on a non-8080 host port.

## Rollout / Rollback
- Rollout:
- Build binaries on host, then start compose stack with `docker compose -f docker-compose.node-manager.yml up -d --build`.
- Pair device with `edgerun-node-manager tunnel-connect ...`.
- Rollback:
- `docker compose -f docker-compose.node-manager.yml down`.
- Revert to prior systemd installer flow.
