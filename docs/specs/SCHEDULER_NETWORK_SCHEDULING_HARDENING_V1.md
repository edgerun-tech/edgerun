# SCHEDULER_NETWORK_SCHEDULING_HARDENING_V1

## Goal and non-goals
- Goal: harden scheduler/network behavior for production by improving worker auth, assignment queue backpressure, and endpoint failover.
- Goal: keep existing control-plane protocol shape and MVP quorum flow compatible where possible.
- Non-goal: redesign consensus/quorum semantics.
- Non-goal: introduce new transport stacks beyond existing control websocket + WebRTC signaling.

## Security and constraint requirements
- Worker assignment polling must require signed worker identity with bounded replay window.
- Scheduler assignment queues must be bounded to avoid unbounded in-memory growth.
- Committee selection must account for worker pressure (pending assignment load) in addition to liveness/runtime eligibility.
- Frontend route-control resolution must attempt failover candidates instead of pinning first-selected base indefinitely.

## Acceptance criteria
- `WorkerAssignmentsRequest` includes signature metadata and scheduler verifies it before draining assignments.
- Scheduler rejects assignment enqueue when per-worker or global backlog limits are exceeded.
- Worker committee selection is load-aware using worker capacity and current pending queue depth.
- Frontend route resolution (`route.resolve` and owner routes) iterates available control-base candidates and remembers successful base.
- Existing frontend checks/build and relevant Rust crate checks pass.

## Rollout and rollback
- Rollout:
  1. Deploy scheduler + worker binaries together so signed assignment polling is active.
  2. Monitor queue pressure logs to tune backlog env limits.
  3. Deploy frontend failover logic.
- Rollback:
  1. Revert this change set to prior unsigned polling and single-base resolution behavior.
  2. Restore previous scheduler/worker binaries as a matched pair.
