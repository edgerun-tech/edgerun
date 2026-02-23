# Cloud OS Learnings and Workflow Plan (v1)

## Status
- Draft v1
- Date: 2026-02-23
- Scope: product/architecture learnings and execution workflow

## What We Learned

### 1) Product intent is clear
- `cloud-os` is not a simple dashboard.
- It is a persistent personal operating surface:
  - open from any browser/device,
  - reconnect to your own environment,
  - execute on edgerun daemons on owned devices and/or cloud infrastructure.

### 2) Integrations are core, not optional extras
- GitHub integration:
  - browse/edit/push code from Cloud OS surface.
- Google integration:
  - email + calendar as first-class operational context.
- Cloudflare integration:
  - manage domains/tunnels/resources and reach infra.
- These are strategic capabilities for “always home” workflow.

### 3) UI and execution need strict separation
- `cloud-os` should be the interaction layer.
- Execution should run on edgerun substrate.
- UI should submit intents and render projections from event history.
- Side effects must be represented as events.

### 4) Storage/event log is the key enabler
- Filesystem entropy caused prior loss/disorientation.
- Append-only event history gives:
  - reproducibility,
  - auditability,
  - replay/resume,
  - better context retrieval for long-running work.

### 5) Recovery path works
- Public deployment artifacts can be mirrored.
- Significant source snapshots were recoverable from `~/.qwen` tool-result history.
- Recovery is feasible but lossy; canonical event history should replace this failure mode.

## Current Architectural Direction

### Canonical model
- Canonical truth: append-only event log (immutable).
- Derived state: materialized views/projections.
- Rollback: append control events, never mutate/delete history.

### Runtime boundaries
- `cloud-os`:
  - input, visualization, workflow control.
- edgerun substrate:
  - planning, policy, deterministic execution.
- storage/event bus:
  - sequence, persistence, delivery, replay.

## Workflow Contract We Should Use

### A) Spec-first for non-trivial changes
1. Write/update spec in-repo.
2. Agree acceptance criteria and non-goals.
3. Implement against spec.
4. Record evidence and deviations.

### B) Long-running job contract
Each job must have:
- `job_id`
- owner
- declared goal
- input assumptions
- current phase
- last checkpoint event
- next action
- blocked reason (if blocked)
- explicit completion condition

### C) Multi-agent operation pattern
- Use specialized agents per domain:
  - `cloud-os-ui`
  - `execution-substrate`
  - `event-bus-policy`
  - `storage-engine`
  - `integrations-github`
  - `integrations-google`
  - `integrations-cloudflare`
- Every agent emits structured events:
  - `plan.created`
  - `step.started`
  - `step.finished`
  - `artifact.diff.produced`
  - `blocked`
  - `handoff`

### D) Context retrieval policy
- Before action:
  - query last relevant events by `job_id`, `run_id`, `area`.
- During action:
  - append progress and decisions continuously.
- After action:
  - append outcome + artifact references + unresolved risks.

## Minimal Event Categories to Start Capturing
- `interaction.user_input`
- `interaction.agent_output`
- `session.started`
- `session.ended`
- `job.opened`
- `job.progress`
- `job.blocked`
- `job.completed`
- `code.edit`
- `command.executed`
- `artifact.created`
- `policy.updated`
- `control.rollback.applied`

## Immediate Next Steps
1. Keep existing provider integrations (GitHub/Google/Cloudflare) as strategic scope.
2. Implement event-first execution path for one vertical slice:
   - Intent submission -> substrate execution -> projection update in Cloud OS.
3. Add job-state panel in Cloud OS sourced from event queries.
4. Add repository-scoped storage query helpers for context hydration.
5. Define agent ownership matrix and emit standardized progress events.

## Risks
- Partial source recovery can hide inconsistencies.
- Mixed legacy direct-mutation paths can bypass event discipline.
- Without strict workflow contract, long-running work will fragment again.

## Decision Log (current)
- Keep multi-provider integration strategy.
- Do not collapse to Codex-only product scope.
- Prioritize storage-backed workflow and event-first operation model.

