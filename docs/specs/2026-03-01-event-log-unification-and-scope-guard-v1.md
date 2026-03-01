# 2026-03-01 Event Log Unification and Scope Guard V1

## Goal and non-goals
- Goal:
  - Unify immutable event-log behavior by reusing existing envelope/storage implementations.
  - Prevent parallel event-envelope definitions from being added across crates.
  - Keep iteration scope tight by adding adapters and validations, not new write models.
- Non-goals:
  - Introduce a new canonical envelope crate in this step.
  - Replace existing `edgerun-storage` append/replication internals in this step.

## Existing implementations to unify (source of truth map)
1. `LocalEventEnvelopeV1`
   - Path: `crates/edgerun-runtime-proto/proto/local/v1/node_local_bridge.proto`
   - Role: local bridge transport envelope.
2. `VfsEventEnvelopeV1`
   - Path: `crates/edgerun-storage/proto/storage/v1/storage.proto`
   - Role: append log semantics (`seq`, `prev_event_hash`, `event_hash`).
3. Storage internal event primitive
   - Path: `crates/edgerun-storage/src/event.rs`
   - Role: immutable event record + HLC + dependency/hash chain.
4. Storage-backed bus envelope
   - Path: `crates/edgerun-storage/src/event_bus.rs`
   - Role: policy-aware event bus with nonce/replay controls.

## Scope guard rules
- Do not add new top-level event envelope structs in other crates while unification is in progress.
- Extend existing proto/storage envelopes where needed and add conversion adapters.
- Enforce immutable behavior at ingest/projection boundaries:
  - per-writer sequence monotonicity,
  - idempotent replay,
  - conflict rejection for sequence/event-id reuse.

## Acceptance criteria
1. Remove parallel duplicate event-envelope implementation introduced outside existing map.
2. Shared contract work continues in existing `edgerun-runtime-proto` + `edgerun-storage` paths.
3. Future behavior changes must be adapter/validation-focused with no new write model.
4. `edgerun-types` tests/build remain green after duplicate removal.

## Rollout and rollback
- Rollout:
  - Remove duplicate envelope module.
  - Record unification map and guard rules in-repo.
- Rollback:
  - Re-add removed module only if existing crates are proven insufficient (requires explicit spec revision).
