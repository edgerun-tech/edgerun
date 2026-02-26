# DOCKER_COMPOSE_CADDY_NETWORK_HARNESS_V1

## Goal
Provide a deterministic Docker Compose stack that runs core Edgerun services and routes inter-service/network traffic through a Caddy container, enabling tests to mutate Caddy routing behavior to simulate network conditions.

## Non-goals
- No production deployment topology changes.
- No migration from existing direct local scripts/systemd flows.
- No application-layer protocol redesign.

## Security and Constraints
- Do not use port 8080.
- Keep frontend canonical in `frontend/`; build artifacts remain under `out/`.
- JS build workflow must remain Bun-based.
- Caddy config mutation must be explicit and reversible in tests.

## Acceptance Criteria
1. A compose stack exists with services: `scheduler`, `term-server`, `worker`, `frontend`, `caddy`.
2. Caddy routes traffic to backend services and frontend.
3. Tests can switch between network scenarios by replacing Caddy route snippet and reloading Caddy without stack restart.
4. At least one Cypress e2e spec demonstrates scenario switching and user-visible effect.
5. Validation evidence includes frontend check/build, relevant e2e run, and Rust checks.

## Rollout
- Use compose harness for local/CI e2e scenarios requiring controllable network behavior.
- Keep direct service scripts available for non-compose workflows.

## Rollback
- Stop using compose harness scripts and revert added compose/Caddy files.
