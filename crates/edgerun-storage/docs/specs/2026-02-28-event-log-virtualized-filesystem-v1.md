# 2026-02-28 Event Log Virtualized Filesystem V1

## Goal
- Define a custom (non-git-compatible) virtual filesystem (VFS) reconstructed from `edgerun-storage` event logs.
- Support multiple source adapters (Git, plain filesystem snapshot, log/event stream, custom dataset) as initial storage state.
- Support deterministic time travel (move backward/forward in history), multiple branches, and concurrent agent suggestion flows.
- Keep branch mutation explicit: proposals are separate from accepted/applied changes.

## Non-Goals
- Bit-for-bit Git object compatibility or serving `.git` plumbing commands.
- Distributed merge consensus in V1.
- FUSE/mount integration in V1.
- Replacing existing storage segment/writer internals.
- Restricting the architecture to a Git-only worldview.

## Security And Constraints
- Event log is append-only; no event mutation or deletion.
- Branch head movement is monotonic by append order, except explicit checkout cursor movement (read-only view).
- Proposal events must not mutate canonical branch state until an explicit apply event is appended.
- All state reconstruction must be deterministic from persisted events + snapshots.
- Avoid unbounded replay cost by periodic materialized snapshots and bounded tail replay.

## Design
### 1. Storage Layout In `edgerun-storage`
- Use one segmented journal per repository:
  - `vfs.<repo_id>.journal`
- Continue using `StorageEngine::append_event_to_segmented_journal` and `query_segmented_journal_raw`.
- Use protobuf payloads for typed VFS events (new proto package under `proto/storage/v1`).

### 2. Core Model
- `RepoId`: stable ID for one imported repository.
- `SourceKind`: `git | fs_snapshot | log_stream | custom`.
- `Mode`: working behavior for projection and merge policies:
  - `code_mode`: path/tree semantics optimized for source files and diffs.
  - `log_mode`: append/partition semantics optimized for high-rate logs/events.
  - `hybrid_mode`: mixed tree + log projections in one repo namespace.
- `BranchId`: logical branch name (`main`, `agent/alice`, `review/x`).
- `Cursor`: `{branch_id, head_event_hash}` or `{branch_id, seq}` for deterministic reads.
- `Blob`: content-addressed file bytes (`blake3` hash).
- `Tree`: canonical directory map (`path -> inode entry`) referencing blob hashes.
- `Snapshot`: materialized tree at a known event hash/seq for fast checkout.

### 3. Event Types (V1)
1. `SourceImportedV1`
2. `BlobStoredV1`
3. `BranchCreatedV1`
4. `BranchHeadMovedV1`
5. `FsDeltaProposedV1`
6. `FsDeltaAppliedV1`
7. `FsDeltaRejectedV1`
8. `CheckoutMovedV1` (optional audit event; does not mutate branch state)
9. `SnapshotCheckpointedV1`
10. `LogEntryAppendedV1`
11. `PartitionDeclaredV1`

Notes:
- `FsDeltaProposedV1` contains suggestion metadata (`agent_id`, `base_cursor`, patch ops/diff text, intent).
- `FsDeltaAppliedV1` references one proposal ID (or inline delta) and becomes canonical mutation.
- `FsDeltaRejectedV1` records explicit rejection reason/conflicts.
- `BranchHeadMovedV1` records resulting head after applied delta/snapshot import.
- `SourceImportedV1` carries `source_kind`, source metadata, root hash/checkpoint refs, and initial mode.
- `LogEntryAppendedV1` and `PartitionDeclaredV1` support log-native mode without forcing file-tree-only modeling.

### 4. Source Adapter Flows
Common contract:
1. User chooses `{source_kind, source_locator, source_ref_or_checkpoint}`.
2. Adapter emits canonical events:
   - `BlobStoredV1` for deduplicated payload units.
   - `SourceImportedV1` with source metadata + projection roots.
   - `BranchCreatedV1(main)` and initial `BranchHeadMovedV1`.
3. Import result records deterministic mapping:
   - adapter metadata (`source_kind`, normalized locator/ref)
   - imported object count / byte count
   - root projection hash/checkpoint

Git adapter:
- Walk Git tree at selected ref and emit file-tree blobs.
- Persist `git_ref`/commit metadata inside `SourceImportedV1`.

Filesystem snapshot adapter:
- Walk directory tree (explicit include/exclude rules), emit file-tree blobs.
- Persist snapshot fingerprint and scan policy in `SourceImportedV1`.

Log/event-stream adapter:
- Ingest ordered records with partition + offset metadata.
- Emit `LogEntryAppendedV1` (and `PartitionDeclaredV1` when needed).
- Optionally expose projected files under virtual paths (for analysis/export), but canonical source remains log-native.

### 5. Virtual Filesystem Reconstruction
- Read path:
1. Resolve target cursor (`branch head` or requested historical cursor).
2. Load nearest `SnapshotCheckpointedV1`.
3. Replay tail events up to target cursor:
   - apply only canonical mutation events (`SourceImported`, `FsDeltaApplied`, `LogEntryAppended`, branch ops).
   - ignore proposals/rejections for filesystem state.
4. Return immutable `VirtualTreeView`.

- Write path:
1. Append proposal (`FsDeltaProposedV1`) to the repo journal.
2. Arbiter/human/agent decides to apply.
3. Validate `base_cursor` against current branch head:
   - clean apply -> append `FsDeltaAppliedV1`, then `BranchHeadMovedV1`.
   - conflict -> append `FsDeltaRejectedV1`.

### 6. Branching And Time Travel
- Branch create:
  - `BranchCreatedV1 {new_branch, from_cursor}`
- Move back/forward:
  - Checkout uses any historical cursor without rewriting log.
  - Optional `CheckoutMovedV1` event for audit/session continuity.
- Divergence:
  - Each branch tracks independent head pointer by events.

### 7. Multi-Agent Suggestion Workflow
- Each agent can append many `FsDeltaProposedV1` events against the same branch/head.
- Proposals are first-class records; they do not alter branch content.
- An arbiter process (human or policy engine) chooses proposals and appends apply/reject events.
- Support batched apply by appending multiple `FsDeltaAppliedV1` events in deterministic order.

### 8. Conflict Policy (V1)
- Deterministic three-way patch apply:
  - `base_tree` from proposal cursor
  - `target_tree` at current branch head
  - `proposal_delta`
- On conflict, reject explicitly (`FsDeltaRejectedV1`) with structured conflict paths.
- No implicit auto-merge outside policy-defined rules in V1.
- In `log_mode`, conflict policy is offset/order-based (duplicate key, partition boundary, or idempotency key rules) instead of path-merge rules.

### 9. API Surface (crate-level target)
- New module sketch: `src/virtual_fs.rs`
1. `import_source(import_request) -> ImportReport`
2. `import_git_repo(repo_path, git_ref, repo_id) -> ImportReport` (adapter helper over `import_source`)
3. `import_fs_snapshot(root_path, policy, repo_id) -> ImportReport` (adapter helper)
4. `ingest_log_entries(repo_id, branch_id, partition, entries) -> IngestOutcome`
5. `propose_delta(repo_id, branch_id, proposal) -> ProposalId`
6. `apply_proposal(repo_id, branch_id, proposal_id) -> ApplyOutcome`
7. `create_branch(repo_id, new_branch, from_cursor) -> ()`
8. `checkout(repo_id, cursor) -> VirtualTreeView`
9. `read_file(repo_id, cursor, path) -> Vec<u8>`
10. `list_dir(repo_id, cursor, path) -> Vec<DirEntry>`

### 10. Snapshot Strategy
- Emit `SnapshotCheckpointedV1` every N canonical mutations or T seconds.
- Snapshot payload stores compact tree index + blob refs.
- Replay bounded by configured tail length; full replay remains fallback path.

## Acceptance Criteria
1. Git, filesystem snapshot, and log-stream adapters can initialize repositories via `SourceImportedV1`.
2. `checkout` at imported head can read/list tree content for tree-backed modes.
3. Log mode can query deterministic ordered entries by partition/offset.
4. Creating a branch from any cursor yields independent future history.
5. Reconstructing older/newer cursors moves backward/forward deterministically.
6. Proposal events are queryable but do not mutate canonical filesystem state.
7. Applying a proposal mutates branch head and appears in subsequent checkout state.
8. Conflicting proposals are rejected with explicit structured metadata.
9. Snapshot + tail replay reproduces same state as full replay.

## Rollout
1. Phase 1: protobuf schemas + journal append/query helpers.
2. Phase 2: in-memory materializer + snapshot checkpoints.
3. Phase 3: source adapters (git + fs snapshot + log ingest baseline).
4. Phase 4: proposal/apply/reject workflow + branch/time-travel query APIs + integration tests.

## Rollback
- Keep feature behind crate-level API boundary (`virtual_fs` module not used by default paths).
- If instability occurs, disable callers and leave event data append-only for later reprocessing.
- Existing storage/timeline/event-bus behavior remains unchanged.
