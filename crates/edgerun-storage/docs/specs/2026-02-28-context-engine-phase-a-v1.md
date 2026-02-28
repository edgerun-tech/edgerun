# 2026-02-28 Context Engine Phase A V1

## Goal
- Add a storage-native context engine that keeps editing context automatically available for agents and tooling.
- Provide indexed views for:
  - symbols
  - references
  - diagnostics
  - touched files
- Support branch-aware context materialization and compact context bundles for a requested file set.

## Non-Goals
- Implementing language parsing in this crate (Tree-sitter/rust-analyzer integration can publish into this API later).
- Full semantic call graph/type graph in this phase.
- Replacing existing VFS event model.

## Security And Constraints
- Context events are append-only and deterministic by sequence order.
- Branch isolation is required: context queries must not cross branch scope unless requested.
- Snapshot checkpoints must be replay-safe (snapshot + tail replay equals full replay).
- No external service binding changes and no use of port `8080`.

## Design
1. New typed protobuf contracts in `proto/storage/v1/storage.proto`:
   - `ContextEventEnvelopeV1`
   - `ContextSymbolUpsertedV1`
   - `ContextReferenceRecordedV1`
   - `ContextDiagnosticRecordedV1`
   - `ContextTouchRecordedV1`
   - `ContextSnapshotCheckpointedV1`
   - materialized snapshot payload types
2. New module `src/context_engine.rs`:
   - append/query primitives over segmented journal `ctx.<repo_id>.journal`
   - materialization with snapshot + tail replay
   - query helpers and context bundle builder
3. Diff-touch helper:
   - parse unified diff headers and emit touch events for affected files.
4. Idempotency and dedupe:
   - context payloads include optional `idempotency_key`.
   - when key is present, append path is idempotent (re-ingest returns prior offset; no duplicate event).

## Acceptance Criteria
1. Can append symbol/reference/diagnostic/touch context events.
2. Can materialize branch context deterministically.
3. Snapshot checkpoint and replay produce same materialized counts as full replay.
4. Context bundle API returns relevant symbols/diagnostics/references for requested file paths.
5. Unified diff helper records touched files.
6. Re-ingesting the same analyzer payload with identical idempotency keys does not duplicate context events.
7. `cargo check -p edgerun-storage` and `cargo test -p edgerun-storage` pass.

## Rollout
1. Land protobuf contracts and module scaffolding.
2. Add materialization and snapshot logic.
3. Add bundle/diff-touch helpers and tests.

## Rollback
- Keep the module isolated (`context_engine`) and unused by default runtime paths.
- If needed, disable callers while retaining append-only data for later reindex.
