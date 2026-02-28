# 2026-02-28 Intent Pipeline Contracts v1

## Goal
- Prevent contract duplication across scheduler/worker/node-manager components.
- Define a single, canonical intent pipeline with explicit start/end stages.
- Define core objects for policy-gated capability routing across a pool of nodes.
- Define canonical event payload contracts and topic/payload-type identifiers in one shared module.

## Non-Goals
- No runtime behavior change in this phase (types/contracts only).
- No replacement of existing control-plane wire schemas.

## Requirements
- Shared contracts live in one place: `crates/edgerun-types`.
- Pipeline stages are explicit, ordered, and terminal-safe.
- Terminal states are unambiguous (`completed`, `failed`, `denied`, `canceled`, `timed_out`).
- Capability allocation is represented as leases with scope and TTL.
- Runtime event payload structs used by scheduler/worker/term-server are defined in the same shared module.
- Event topic names and payload-type identifiers are defined as shared constants to avoid string drift.

## Canonical Pipeline
1. `ingress_received` (start)
2. `intent_normalized`
3. `policy_evaluated`
4. `capability_planned`
5. `capability_leased`
6. `execution_routed`
7. `execution_running`
8. terminal:
   - `execution_completed`
   - `execution_failed`
   - `policy_denied`
   - `execution_canceled`
   - `execution_timed_out`

## Contract Objects
- `IntentEnvelope`
- `IntentDecision`
- `CapabilityLease`
- `StepAssignment`
- `ExecutionReceipt`
- Scheduler ingest event payloads:
  - `WorkerHeartbeatIngestEvent`
  - `WorkerAssignmentsPollIngestEvent`
  - `WorkerResultIngestEvent`
  - `WorkerFailureIngestEvent`
  - `WorkerReplayIngestEvent`
  - `RouteChallengeIngestEvent`
  - `RouteRegisterIngestEvent`
  - `RouteHeartbeatIngestEvent`
- Worker and term-server lifecycle/status event payloads:
  - `WorkerLifecycleEvent`
  - `WorkerHeartbeatEvent`
  - `TermServerLifecycleEvent`
  - `RouteAnnouncerEvent`

## Acceptance Criteria
1. `edgerun-types` exports canonical intent pipeline contracts.
2. Stage enum includes clear start and terminal end states.
3. Utility helpers exist for terminal-state checks.
4. Scheduler/worker/term-server compile using shared event payload contracts from `edgerun-types` with local duplicates removed.
5. Event topic and payload-type strings used by scheduler/worker/term-server are sourced from shared constants.
6. `cargo clippy -p edgerun-types --all-targets -- -D warnings` passes.
7. `cargo test -p edgerun-types` passes.

## Rollout
- Consumers should migrate to these shared contracts incrementally.
- Old duplicated structs can be removed after call sites are updated.
- Rollback: switch consumers back to previous local structs/constants; no persisted data migration is required because wire payload contents remain unchanged.
