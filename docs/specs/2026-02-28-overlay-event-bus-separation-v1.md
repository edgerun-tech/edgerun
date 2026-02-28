# 2026-02-28 Overlay Event Bus Separation v1

## Goal
- Separate runtime event-bus communication from storage concerns.
- Eliminate polling-based subscription semantics for runtime event delivery.
- Establish explicit overlay domains for event flow:
  - `edge-internal`
  - `edge-cluster`
- Route events by topic on top of an overlay domain (topic-isolated channels).

## Non-Goals
- This phase does not remove timeline/storage persistence features.
- This phase does not complete a full cross-process broker rollout for every crate.
- This phase does not redesign all existing control-plane message schemas.

## Security and Constraints
- Fail-closed ingestion: publish path must return explicit error on bus failure.
- Topic namespace is explicit and deterministic (no implicit wildcard routing).
- Overlay and topic are part of event identity for policy/audit clarity.
- Runtime event delivery is push-based (subscriber receives events through channel semantics, not cursor polling).

## Architecture
- Introduce a standalone runtime crate: `crates/edgerun-event-bus`.
- Runtime API provides:
  - overlay enum (`edge-internal`, `edge-cluster`)
  - topic identity (`overlay + topic_name`)
  - publish
  - subscribe (push receiver)
- Scheduler ingestion publishes to runtime bus topics.
- Storage is treated as an optional downstream sink (separate concern), not the event bus transport itself.

## Acceptance Criteria
1. New crate `edgerun-event-bus` exists in workspace.
2. The crate exposes non-polling subscribe API (`tokio::sync::broadcast::Receiver`-based).
3. Scheduler no longer publishes control-plane ingestion events through `StorageBackedEventBus`.
4. Scheduler maps ingestion payload types onto explicit overlay topics.
5. Build and checks pass for scheduler/frontend baseline validation commands.

## Rollout and Rollback
- Rollout:
  - Add new crate and wire scheduler ingress to runtime bus.
  - Keep storage persistence paths independent.
- Rollback:
  - Revert commit to restore storage-backed ingest path.
  - Existing state snapshots remain backward-compatible.
