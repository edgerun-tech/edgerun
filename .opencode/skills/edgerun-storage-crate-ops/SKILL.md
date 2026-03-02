---
name: edgerun-storage-crate-ops
description: Implement and review integrations with crates/edgerun-storage including append sessions, durability, sealing, replication, and query behavior
license: Apache-2.0
compatibility: opencode
metadata:
  audience: agents
  repository: edgerun
---

## What I do

- Guide safe edits in `crates/edgerun-storage`.
- Pick correct append/query APIs for each use case.
- Keep durability and seal policy decisions explicit and verifiable.

## Workflow

1. Classify request scope before coding.
- Use `StorageEngine::create_append_session` + `EngineAppendSession` for long-lived append flows.
- Use `StorageEngine::append_event_to_segmented_journal` for one-shot append-and-seal writes.
- Use `query_events_raw` / `query_segmented_journal_raw` when offsets, hashes, and cursors matter.
- Use `timeline.rs` or `event_bus.rs` adapters when protobuf envelopes and domain semantics already exist.

2. Choose durability/seal settings intentionally.
- Pick `DurabilityLevel` (`AckLocal`, `AckBuffered`, `AckDurable`, `AckCheckpointed`, `AckReplicatedN`).
- Use `seal_policy::SealController` + `enable_auto_seal` for auto-seal behavior.
- Feed chain progress (`mark_chain_progress`, `refresh_chain_progress_from_rpc_read`) when chain-aware modes are involved.

3. Preserve crate contracts.
- Return `StorageError` unless boundary translation is explicitly required.
- Keep pagination stable with `QueryCursor.offset` semantics.
- Avoid bypassing manifest/checkpoint paths when requesting checkpoint durability.

4. Validate with crate-scoped evidence.
- `cargo check -p edgerun-storage`
- `cargo test -p edgerun-storage`

## Reference

- `crates/edgerun-storage/docs/operator-guide.md`
