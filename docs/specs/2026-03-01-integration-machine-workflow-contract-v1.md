# 2026-03-01 Integration Machine Workflow Contract V1

## Goal
Define implementable contracts for:
- integration lifecycle,
- machine capability model,
- workflow/executor model,
so separate machines can execute workflows reliably (github actions, local actions, AI agent tasks, ingest/transform).

## Non-Goals
- UI polish or onboarding UX details.
- Full engine implementation in this document.

## Security and Constraint Requirements
- All integration secrets must be encrypted at rest (profile or TPM-backed host store).
- All execution requests must pass policy gate before scheduler placement.
- Executor invocation must be deterministic: same request => same declared side effects.
- Every decision and state transition emits an event.
- Components run independently on separate nodes; no singleton assumptions.

## Integration Lifecycle Contract

### Integration States
`unconfigured -> configured -> verified -> connected -> degraded -> disconnected`

### Required Integration Operations
1. `verify(input) -> VerificationResult`
2. `capabilities() -> IntegrationCapabilities`
3. `execute(operation, payload) -> IntegrationExecutionResult`
4. `health() -> IntegrationHealth`
5. `rotate_secret(new_secret_ref) -> RotateResult`

### Verification Result
- `ok: bool`
- `reason: string`
- `granted_scopes: repeated string`
- `expires_unix_ms: uint64`

### Integration Execution Result
- `ok: bool`
- `status: enum { queued, running, completed, failed }`
- `external_run_id: string`
- `artifact_refs: repeated string`

## Machine Capability Contract

### Capability Dimensions
- compute: `cpu_cores`, `mem_bytes`, `gpu_vendor`, `gpu_mem_bytes`
- storage: `fs_read`, `fs_write`, `disk_bytes_free`
- network: `egress`, `ingress`, `overlay_joined`
- device: `usb`, `camera`, `microphone`, `display`, `tpm`
- runtime: `executor.local_action`, `executor.github_actions`, `executor.agent_task`, `executor.ingest_transform`, `executor.wasm`, `executor.oci`, `executor.vm`, `executor.lxc`

### Snapshot Semantics
- node publishes full capability snapshot periodically and on change.
- scheduler uses latest healthy snapshot.
- stale snapshot beyond TTL => node marked unschedulable.

## Workflow Contract

### Workflow Definition
- immutable workflow graph for a run
- steps with:
  - `executor_kind`
  - `integration_refs`
  - `required_capabilities`
  - `input`
  - `depends_on`
  - retry/timeouts

### Workflow Run State Machine
`created -> planned -> scheduled -> running -> completed|failed|cancelled`

### Scheduling Contract
- scheduler must output a `SchedulingDecision` per runnable step:
  - selected node
  - selected executor
  - rationale (capability match summary)

### Failure Contract
- step failure emits failure event with retryability flag.
- retry policy evaluated deterministically by scheduler/planner.

## Policy Scope Model
- `integration:{id}:{operation}`
- `executor:{kind}:run`
- `machine:{node_id}:use`
- `data:{namespace}:{action}`
- `workflow:{workflow_id}:trigger`

Policy evaluation returns:
- allow/deny
- matched policy ids
- denial reason

## Event Model Requirements
Required event classes:
1. `intent.received`
2. `policy.decided`
3. `workflow.planned`
4. `step.scheduled`
5. `step.started`
6. `step.progress`
7. `step.completed`
8. `step.failed`
9. `integration.health.changed`
10. `machine.capability.updated`

## Rollout Plan
1. Add proto contracts (`control.v1`) and compile gating.
2. Add scheduler and worker skeletons against those contracts.
3. Add github-actions/local-action executors first.
4. Add ingest/transform + agent task executors.
5. Enable capability-aware placement by default.

## Rollback Plan
- disable new planner path by feature flag.
- route intents to existing execution path.
- preserve events for replay but stop scheduling on new contracts.

## Acceptance Criteria
- contracts are specific enough for independent teams to implement executor/integration/scheduler components in parallel without touching each other’s internals.
- each contract has explicit input/output/state behavior.
- policy and event requirements are mandatory and testable.
