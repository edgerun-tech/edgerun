# 2026-02-24 Scheduler Ingestion And Dialing Enforcement

## Goal
Implement an enforced, durable ingestion path for scheduler control-plane events into `edgerun-storage`, and restore deterministic component communication where components dial the scheduler first and scheduler-mediated connectivity is authoritative.

## Non-Goals
- Replacing the existing scheduler in-memory state model in this change.
- Replacing routed terminal signaling protocol formats.
- Introducing a new external message broker.

## Security And Constraints
- Control-plane event ingestion must be append-only and durable via scheduler event-bus storage under scheduler data dir.
- Scheduler must be able to enforce ingestion at runtime (`fail closed`) when configured.
- Worker submissions and route registration/heartbeat flows must not bypass scheduler verification paths.
- Signature verification requirements for existing worker flows remain unchanged.
- Component communication should remain scheduler-mediated: components publish presence/route state to scheduler, and consumers resolve/connect via scheduler APIs.
- Control websocket frames use bincode; optional fields in control-plane structs must not rely on skipped serialization in a way that changes field order for non-self-describing formats.

## Design
1. Extend scheduler event-bus policy to allow explicit control-plane payload types beyond chain progress.
2. Add scheduler ingest helpers that publish typed control-plane events to storage-backed event bus.
3. Add enforcement switch:
- When enabled, scheduler rejects relevant control-plane requests if ingestion is unavailable or publish fails.
- When disabled, scheduler logs warnings and continues.
4. Restore route announcer compatibility by extending scheduler control websocket request/response variants for route challenge/register/heartbeat (single dial path).
5. Wire worker and route-control handlers through ingestion helper before state mutation so accepted state transitions are durably recorded.
6. Preserve stable bincode wire compatibility for control websocket payloads (no field elision that shifts subsequent fields).

## Acceptance Criteria
1. Scheduler ingests worker heartbeat/result/failure/replay and route challenge/register/heartbeat events to storage-backed event bus.
2. Scheduler policy allows these payload types and rejects unknown types by policy.
3. Enforcement mode blocks acceptance of relevant events when ingestion fails.
4. Term-server route announcer succeeds via scheduler control websocket route challenge/register/heartbeat requests.
5. Worker communication with scheduler control websocket remains functional.
6. Route registration payloads with `relay_session_id = None` decode correctly over control websocket bincode transport.

## Rollout / Rollback
- Rollout:
  - Deploy scheduler with updated event-bus policy rules.
  - Enable enforcement in environments where fail-closed ingest is required.
- Rollback:
  - Disable enforcement env toggle to fail open while preserving ingestion attempts.
  - Revert scheduler route/ingestion wiring if required; existing appended events remain immutable.
