# 2026-03-01 VFS Log Partition State Cache V1

## Goal
- Reduce repeated replay cost in `ingest_log_entries` by caching per-partition state (`declared`, max offset, idempotency keys).
- Improve throughput for batched log ingestion (including docker logger batches) where the same partition is flushed multiple times.

## Non-Goals
- Cross-process cache coherence.
- Changes to wire schema or event semantics.

## Constraints
- Append-only event behavior must remain unchanged.
- Deduplication correctness must remain deterministic.
- Cache must be derivable from persisted events and safe to rebuild.

## Design
1. Add in-memory map keyed by `branch_id + partition`.
2. On first ingest for a key, hydrate cache from current envelopes.
3. On subsequent ingests, reuse cached state without full replay.
4. Update cache incrementally when partition/log events are appended.

## Acceptance Criteria
1. Existing ingest behavior remains unchanged for idempotency and offsets.
2. Repeated batched ingests do not require full envelope scans per flush for known partitions.
3. Existing tests pass, and batched ingest tests continue passing.

## Rollback
- Remove cache usage and fall back to replay-per-call logic.
