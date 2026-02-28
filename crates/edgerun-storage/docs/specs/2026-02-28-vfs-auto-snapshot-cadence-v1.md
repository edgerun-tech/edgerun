# 2026-02-28 VFS Auto Snapshot Cadence V1

## Goal
- Add deterministic automatic snapshot checkpoints for the virtual filesystem so replay cost stays bounded as event history grows.
- Keep the storage abstraction source-agnostic (Git, filesystem snapshot, log stream, custom), with snapshot behavior independent of source kind.
- Improve append-path performance by avoiding repeated full-log scans when assigning envelope sequence and previous hash.

## Non-Goals
- Changing storage wire formats or protobuf schemas in this phase.
- Background compaction/pruning implementation.
- Cross-process cache invalidation guarantees beyond current crate behavior.

## Security And Constraints
- Event log remains append-only; snapshots are additive events.
- Materialized state from `snapshot + tail replay` must match full replay.
- Auto-snapshot cadence must be deterministic and derived from persisted events.
- Snapshot trigger cannot recurse infinitely (snapshot append must not re-trigger snapshot append).

## Design
### 1. Snapshot Policy
- Introduce `VfsSnapshotPolicy` on `StorageBackedVirtualFs`.
- Initial policy field:
  - `auto_checkpoint_every_applied: u64`
- Semantics:
  - `0` disables auto snapshotting.
  - `N > 0` triggers snapshot after every N `FsDeltaApplied` events since the latest snapshot.

### 2. Snapshot Progress Tracking
- Add in-memory progress tracker to avoid rescanning on every apply.
- Bootstrap tracker lazily from persisted envelopes:
  - find latest snapshot seq
  - count `FsDeltaApplied` events after that snapshot
- On each new applied delta:
  - increment counter
  - when threshold reached, append `SnapshotCheckpointedV1`
  - reset tracker by invalidating/rebootstrapping from persisted state

### 3. Append Path Optimization
- For envelope assignment (`seq`, `prev_event_hash`), use cached repo envelopes when available.
- After successful append, update caches incrementally with the new envelope instead of blanket invalidation.
- Preserve fallback behavior: if cache is cold, load from storage once.

## Acceptance Criteria
1. Auto snapshot can be enabled/disabled by policy.
2. With interval `N`, exactly one snapshot is appended after each N applied deltas (per repo event stream progression).
3. `materialize()` output remains unchanged versus pre-change semantics.
4. Existing tests pass; new tests assert auto snapshot trigger and replay correctness.
5. No protobuf/schema breaking changes introduced.

## Rollout
1. Add policy + tracker + append cache updates in `virtual_fs.rs`.
2. Add/extend tests for auto snapshot trigger and materialize replay.
3. Validate with crate-scoped `cargo check` and `cargo test`.

## Rollback
- Set `auto_checkpoint_every_applied = 0` (disable behavior) while retaining append-only data.
- Revert `virtual_fs.rs` changes if cache/path regressions occur.
