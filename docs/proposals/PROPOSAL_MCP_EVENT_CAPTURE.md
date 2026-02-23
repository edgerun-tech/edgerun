# Proposal: MCP Event Capture and Storage Integration

## Status

Proposal + implementation track starter for MCP surface.  
Signing is explicitly out of scope and delegated to storage-engine proposal.

## Goal

Expose storage lifecycle/query APIs via MCP and auto-capture tool events into storage through MCP middleware.

## Non-goals

- Implementing storage-engine signing.
- Granting assistant direct append privileges.
- Full desktop surveillance by default.

## API Surface (Assistant-visible)

1. `create_storage`
2. `list_storages`
3. `query_storage`

No assistant-callable `append` API.

## Internal MCP Ingestion Path (not assistant-callable)

- `ingest_event_intent` for middleware/host integrations.
- Used by wrappers around toolcalls and message handling.

## Zero-Trust Storage Partitioning

1. `ops_storage` (default)
   - job/action/tool/file-change summaries
2. `interaction_storage` (restricted)
   - clipboard/raw interaction telemetry (future opt-in)

Cross-reference by IDs only (`run_id`, `job_id`, `event_id`, optional `interaction_ref_id`).

## Auto-Capture Requirements

- MCP wrappers emit:
  - `action.planned`
  - `action.result`
  - `fs.changed` (metadata level)
  - `io.user_input`
  - `io.agent_output`
- Event emission should be best-effort and non-blocking to core operations.

## Query Requirements

- Filter by `storage_id`, `job_id`, `run_id`, `event_type`, time range.
- Cursor-based pagination.
- Deterministic ordering (`ts_unix_ms`, `event_id`).

## Access Control

- `create_storage`: privileged principal only.
- `list_storages`: returns only authorized storages.
- `query_storage`: scope-checked per storage.
- Internal ingest path requires trusted MCP runtime identity.

## Acceptance Criteria

1. MCP can create/list/query storages.
2. Append is internal-only (not assistant-invocable).
3. Toolcall wrappers can emit and store minimal lifecycle events.
4. Query can reconstruct current long-job state from event history.

## Rollout

1. Ship MCP APIs with `ops_storage` only.
2. Add auto-capture wrappers for core toolcalls.
3. Add `interaction_storage` as explicit opt-in.
4. Integrate with storage-engine signing once available.

