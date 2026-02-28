# 2026-03-01 Swarm Worker And Eventbus Crate Executors V1

## Goal
- Add swarm worker node `10.13.37.2` into local swarm operations.
- Provide build and test executors for every workspace crate.
- Drive executor triggers via eventbus subject (`NATS`) and emit status events for build/test lifecycle.

## Non-Goals
- Full distributed source sync protocol for remote nodes in this phase.
- Full scheduler integration for executor assignment decisions.

## Security And Constraints
- Event channels are append-only status feeds; no direct privileged command payload execution from events.
- Executors run predefined cargo operations only (`check` for build, `test` for tests).
- No port `8080` usage.

## Design
1. Worker onboarding:
   - `scripts/swarm/add-worker-node.sh` obtains worker join token and joins `10.13.37.2` via SSH.
2. Eventbus protocol:
   - Trigger subject: `edgerun.code.updated` (default).
   - Build status subject: `edgerun.executors.<crate>.build.status`.
   - Test status subject: `edgerun.executors.<crate>.test.status`.
3. Executor runtime:
   - Generic executor script subscribes to trigger subject and runs crate-local operation.
   - Each run emits `started` then `success|failure` event JSON payload to status subjects.
4. Per-crate services:
   - Stack generator emits one build executor service + one test executor service per crate.
   - Services are docker-swarm deployable.

## Acceptance Criteria
1. Worker join helper script exists and validates manager node state.
2. Workspace crate list script resolves crate package names from `Cargo.toml` files.
3. Executor scripts publish status events for both success and failure.
4. Stack generator creates per-crate service definitions for build + test executors.
5. Script syntax validation passes.

## Rollout
- Join worker node and label it for executor workloads.
- Deploy generated crate-executor stack.
- Publish code update events on `edgerun.code.updated` to trigger runs.

## Rollback
- Remove worker from swarm.
- Remove executor stack.
- Stop publishing trigger events.
