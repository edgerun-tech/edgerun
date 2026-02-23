# OS UI vs Edgerun Substrate Boundary (v1)

## Status
- Draft v1
- Date: 2026-02-23
- Legacy reference UI: `browser-os/`

## Goal
- Separate product UI concerns from execution concerns.
- Make execution deterministic, replayable, and auditable on edgerun substrate.
- Make append-only event log the canonical source of truth.

## Non-goals
- Rebuilding the full `browser-os/` UX in this phase.
- Final cryptographic signature policy for all appenders (tracked separately).
- Full migration of every integration/provider in one step.

## Core Principles
- Event log is immutable.
- Rollback is append-only control events, never mutation/deletion.
- UI renders projections; UI does not perform direct system mutation.
- Execution happens through substrate handlers only.
- Every state transition must be representable as events and replayable.

## Layer Model

### 1) `os-ui` Layer
- Responsibilities:
  - Render projections (windows, activity, command history, execution state).
  - Emit user intents and UI interaction events.
  - Show previews, pending actions, failures, and rollback controls.
- Forbidden:
  - Direct shell execution.
  - Direct file mutation as source of truth.
  - Direct provider-side mutations bypassing substrate contracts.

### 2) `execution` Layer on Edgerun Substrate
- Responsibilities:
  - Validate intents against policy.
  - Plan and execute deterministic steps.
  - Emit execution lifecycle events and artifact diffs.
  - Emit explicit failure and halt events.
- Forbidden:
  - Hidden side effects not represented as events.
  - Non-deterministic retries that mutate existing event history.

### 3) `event bus + storage` Layer
- Responsibilities:
  - Sequence, store, and deliver events.
  - Enforce schema version and policy checks.
  - Preserve hash-linked append-only timeline.
- Forbidden:
  - Business logic execution.
  - Mutable rewrite/delete semantics.

### 4) `adapter` Layer (GitHub, Cloudflare, Gmail, terminal, etc.)
- Responsibilities:
  - Translate provider operations into canonical events.
  - Consume execution instructions from substrate contracts.
- Forbidden:
  - Acting as canonical state store.
  - Policy authority.

## Canonical Event Classes (v1 minimal)
- `intent.submitted`
- `intent.accepted`
- `intent.rejected`
- `plan.created`
- `execution.started`
- `execution.step.started`
- `execution.step.finished`
- `execution.failed`
- `artifact.diff.produced`
- `projection.updated`
- `control.rollback.applied`
- `policy.updated`

## Minimal Event Envelope Contract
- `schema_version`
- `event_id`
- `seq`
- `ts_unix_ms`
- `run_id`
- `session_id`
- `actor_type`
- `actor_id`
- `event_type`
- `payload_type`
- `payload`
- `prev_event_hash`
- `event_hash`

## Boundary API (conceptual)
- UI -> Substrate:
  - `SubmitIntent(intent)`
  - `RequestPreview(intent_ref | draft)`
  - `ApplyControl(control_event)`
- Substrate -> UI (via projections):
  - `GetTimeline(query)`
  - `GetExecutionState(run_id)`
  - `GetProjection(view_id, cursor)`

## Determinism Rules
- Planner inputs must be explicit and event-referenced.
- Execution outputs must be emitted as ordered events.
- Rebuild at `seq=N` must produce the same projection result.
- Any non-deterministic source must be normalized by adapter contract.

## Rollback Semantics
- Rollback is represented by control events:
  - targeted exclusion by `event_id`
  - rewind by `to_seq` / `last_n`
- Materializer computes effective active set from control events.
- Original events remain immutable and queryable.

## Security/Constraints
- No secret-bearing tokens in canonical event payloads.
- Token material remains in adapter/runtime secret stores.
- Policy must reject execution events before policy initialization.
- Policy updates must not permanently lock policy update capability.

## Acceptance Criteria (v1)
1. A UI intent can be submitted without direct side effects.
2. Substrate executes the intent and emits lifecycle events.
3. UI state updates from projection stream only.
4. Rollback applies via appended control event and re-projection.
5. Replaying from snapshot + tail reproduces identical projection hash.

## Rollout Plan
1. Define proto contracts for envelope + core event payloads.
2. Implement one vertical slice:
   - `intent.submitted -> execution.started -> execution.step.* -> artifact.diff.produced -> projection.updated`
3. Wire `os-ui` command box to `SubmitIntent`.
4. Render execution timeline in UI from event query only.
5. Add rollback controls that append control events.

## Rollback Plan (deployment)
- If projection pipeline regresses:
  - Keep appending events.
  - Pin UI to last known-good projection interpreter version.
  - Rebuild projections from stable snapshot + event tail.

## Open Questions
- Final signed-append policy activation point.
- Snapshot cadence and compaction policy.
- Adapter determinism contract per provider.
- Multi-tenant partitioning model for timeline queries.

