# 2026-03-01 Segmented Journal Batch Append Compatible V1

## Goal
- Reduce append overhead in storage-backed VFS/log ingest by batching many events into one segment append session.
- Preserve current reader compatibility by sealing each batch segment before query.

## Non-Goals
- Reusing unsealed segments across calls.
- Changing segment binary format.
- Introducing background compaction.

## Constraints
- Query paths continue to read only sealed segment files.
- Registry file remains source of truth for segment ordering.
- Event ordering and hash chaining semantics remain unchanged.

## Design
1. Add `StorageEngine::append_batch_to_segmented_journal`:
- create one new segment part,
- append all provided events,
- flush durability once per batch policy,
- seal once,
- register segment in `.segments`.
2. Keep `append_event_to_segmented_journal` as compatibility wrapper around the batch API.
3. In `StorageBackedVirtualFs`, add internal batched envelope append path and use it for log ingestion so partition declaration + entry events can be committed in one segment.

## Acceptance Criteria
1. Existing behavior remains correct for single-event callers.
2. Log ingestion writes fewer segments for batched input and keeps idempotency behavior.
3. `cargo check`, `cargo clippy -D warnings`, and targeted VFS tests pass.
4. Benchmark shows improved lines/sec for larger ingest runs versus prior per-event segment behavior.

## Rollout / Rollback
- Rollout: default-on through internal VFS usage.
- Rollback: switch VFS append path back to single-event engine API and keep batch API unused.
