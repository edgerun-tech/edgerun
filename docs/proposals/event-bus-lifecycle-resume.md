# SPDX-License-Identifier: LicenseRef-Edgerun-Proprietary

# Event Bus Lifecycle and Resumability (Proposal)

## Status
Draft

## Goal
Define a deterministic lifecycle for the event bus as the system sequencer, with explicit halt and resume behavior and no implicit retry loops.

## Core Decisions
- Event bus starts first and is the sequencer.
- `init(config)` is the policy initialization event.
- Scheduler does not read Solana RPC directly; chain ingress is owned by the event bus.
- No automatic recovery state. Failures transition to `Halted`.
- Recovery is explicit through `resume_from(...)` and/or `init(config)`.

## Lifecycle Phases
- `AwaitingInit`
- `Running`
- `Halted`
- `EmergencySealOnly`

## Phase Semantics

### `AwaitingInit`
- Allowed:
  - `status`
  - `query`
  - `submit(policy_update_request)` where this is `init(config)`
- Rejected:
  - All non-init operational events.
- Transition:
  - On accepted `init(config)` -> `Running`.

### `Running`
- Allowed:
  - Normal policy-approved event submission.
  - Chain progress ingestion from Solana subscription (owned by bus).
- Transition:
  - On invariant/storage/replay error -> emit `bus_halt(reason, last_event_id)` and transition to `Halted`.

### `Halted`
- Allowed:
  - `status`
  - `query`
  - `submit(resume_from { event_id | offset })`
  - `submit(policy_update_request)` for re-init/update
- Rejected:
  - Normal operational event flow.
- Transition:
  - On valid `resume_from` -> `Running`.
  - On valid re-init policy path -> `Running`.

### `EmergencySealOnly`
- Manual operator safety mode.
- Only emergency sealing and introspection endpoints are allowed.

## Resumability Contract

## Persisted checkpoint
Bus persists:
- `phase`
- `last_applied_event_id`
- `last_offset` (or equivalent cursor)
- `policy_version`
- `latest_chain_progress_event_id`
- nonce state (`last_nonce_by_publisher` or integrity-checked equivalent)

## Resume rules
- Resume point must be monotonic and exact.
- Replay starts from checkpoint+1.
- If checkpoint and log diverge, emit `bus_halt(reason=resume_mismatch)` and remain `Halted`.
- No implicit retries or auto-advance.

## Chain Ingress Contract
- Event bus owns Solana subscription and emits canonical `chain_progress` events into the same sequenced log.
- Scheduler consumes bus events only.
- Scheduler does not perform direct Solana RPC polling/subscription for canonical state.

## Scheduler Coupling
- Scheduler persists `last_consumed_bus_event_id`.
- On startup/reconnect, scheduler requests catch-up from that id.
- Retries are represented as new jobs/events, not hidden re-execution.

## Operational Debug Surface
`status` should minimally expose:
- `phase`
- `policy_version`
- `last_applied_event_id`
- `last_offset`
- `latest_chain_progress_event_id`
- `storage_ok`

## Non-Goals
- Signed append enforcement (separate proposal/phase).
- Complex dynamic policy language in first implementation.
- Automated self-healing retry loops.

## Implementation Order
1. Add phase machine and `Halted` transitions in event bus.
2. Add persisted checkpoint fields and strict resume logic.
3. Move/complete Solana ingress in event bus.
4. Remove scheduler direct RPC path for canonical chain progress.
5. Add tests for halt/resume monotonicity and replay determinism.
