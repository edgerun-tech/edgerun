# 2026-03-01 Orchestration Integrations Workflows Executors Parallel V1

## Goal
Create a production-viable control architecture that can run:
- GitHub Actions
- Local actions
- AI agent tasks
- Data ingestion/transformation pipelines
across separate machines with capability-aware scheduling.

## Non-Goals
- Full production completion in one change set.
- Replacing all existing runtime code paths immediately.
- Cross-cloud deployment automation in this phase.

## Hard Constraints
- Event-first system behavior: decisions and state transitions must be representable as events.
- Intent is an event.
- Proto is canonical schema format.
- Components must be independently deployable on separate nodes.
- Multi-instance component operation must be supported (N schedulers/workers/managers).

## Target Architecture
1. Intent API (proto): typed user/system intents.
2. Policy check stage: authorize intent against user/node/integration scopes.
3. Planner: compile intent to workflow DAG.
4. Scheduler: capability + locality + health-based placement.
5. Executor runtime: run step on chosen backend (local, github-actions, agent, wasm/oci/vm/lxc).
6. Event timeline: durable event stream per node, queryable on demand.

## Core Domain Contracts (Proto)
- `control.v1.IntentEnvelope`
- `control.v1.WorkflowDefinition`
- `control.v1.WorkflowRun`
- `control.v1.WorkflowStep`
- `control.v1.ExecutorTarget`
- `control.v1.MachineCapability`
- `control.v1.IntegrationRef`
- `control.v1.PolicyDecision`
- `control.v1.SchedulingDecision`

## Parallel Workstream Plan

### WS-A Runtime Proto Contracts
Ownership:
- `crates/edgerun-runtime-proto/proto/control/v1/*`
- `crates/edgerun-runtime-proto/build.rs`
- `crates/edgerun-runtime-proto/src/control.rs`
- `crates/edgerun-runtime-proto/src/lib.rs`
Deliverables:
- New `control.v1` proto schema set with basic compile-time coverage.
- Build integration and Rust module export.
Acceptance:
- `cargo check -p edgerun-runtime-proto` passes.

### WS-B Scheduler Workflow Domain Skeleton
Ownership:
- `crates/edgerun-scheduler/src/workflow_domain.rs`
- `crates/edgerun-scheduler/src/main.rs` (minimal wiring only)
Deliverables:
- Workflow planning/scheduling domain structs and validation helpers.
- Capability matching helper for step->node eligibility.
Acceptance:
- `cargo check -p edgerun-scheduler` passes.

### WS-C Worker Executor Registry Skeleton
Ownership:
- `crates/edgerun-worker/src/executor_registry.rs`
- `crates/edgerun-worker/src/main.rs` (minimal wiring only)
Deliverables:
- Executor plugin trait and registry.
- Stubs for executor kinds: `local_action`, `github_actions`, `agent_task`, `ingest_transform`.
Acceptance:
- `cargo check -p edgerun-worker` passes.

### WS-D Integration + Machine + Workflow Interface Spec
Ownership:
- `docs/specs/2026-03-01-integration-machine-workflow-contract-v1.md`
Deliverables:
- Integration interface (verify/capabilities/execute/health/rotate_secret).
- Machine capability model and workflow authoring contract.
- Rollout/rollback plan from current runtime paths.
Acceptance:
- Spec quality gate: complete goal/non-goals/constraints/acceptance/rollback sections.

## Execution Dependencies
- WS-A is required before full runtime wiring in WS-B/WS-C, but WS-B/WS-C can scaffold against temporary local structs now.
- WS-D runs independently and informs follow-up implementation slices.

## Phase Gate Criteria
Phase-1 is complete when:
1. WS-A/B/C compile successfully.
2. WS-D spec is committed.
3. No ownership overlaps between parallel streams.
4. Evidence report includes exact commands and pass/fail.

## Rollback
- Remove `control.v1` proto module and revert scheduler/worker skeleton wiring.
- Keep existing runtime/event paths active.
