# 2026-03-01 VFS Pluggable Docker Logger V1

## Goal
- Add a pluggable Docker log ingestion path to `edgerun-storage` so container logs can be written into VFS `log_mode` as `LogEntryAppendedV1` events.
- Keep storage backend generic: ingestion must be adapter-driven and not hard-coupled to Docker daemon internals.
- Provide an operator-friendly CLI path that can ingest structured log lines without requiring JSON payloads.

## Non-Goals
- Implementing a Docker daemon logging driver plugin API.
- Replacing existing node-manager event bus flows.
- Persisting raw daemon metadata beyond log payload and stable keys in this phase.

## Security and Constraints
- Event log remains append-only.
- Ingestion must be idempotency-aware to avoid duplicate log records.
- Decoder failures should be explicit and not silently corrupt ordering.
- Line format should avoid JSON requirement by default.

## Design
### 1. Pluggable Interfaces
- Introduce decoder and adapter traits:
  - Decoder: line -> `DockerLogRecord`
  - Adapter: `DockerLogRecord` -> `LogIngestEntry` + partition key
- Keep adapter contract generic so other runtimes (containerd/podman/custom) can reuse the same pipeline.

### 2. Default Non-JSON Wire Format
- Add default decoder for pipe-delimited lines:
  - `container_id|container_name|stream|ts_unix_ms|message`
- Use `splitn(5, '|')` to preserve `|` inside message body.

### 3. VFS Ingestion API
- Add helper on `StorageBackedVirtualFs` to ingest batches of docker log lines via decoder+adapter.
- Group produced entries by partition and call `ingest_log_entries` per partition.
- Aggregate per-partition outcomes into a total batch outcome.

### 4. Operator CLI
- Add `docker_log_ingest` binary that:
  - reads framed lines from stdin
  - ingests in configurable batches (default 1000 lines) to bound memory usage
  - decodes/adapts records
  - writes to VFS storage log mode
  - prints ingest counters

## Acceptance Criteria
1. Operator can pipe non-JSON framed log lines to CLI and observe appended log events.
2. Decoder/adapter are swappable without changing storage core.
3. Duplicate idempotency keys are skipped deterministically.
4. Existing VFS tests pass; new tests cover decode and ingestion behavior.

## Rollout
1. Add `docker_logger` module with traits/default implementations.
2. Add VFS helper for adapter-driven docker line ingestion.
3. Add CLI binary and tests.
4. Validate with `cargo check`, `cargo test`, `cargo clippy --lib`.

## Rollback
- Stop using `docker_log_ingest` path and continue current log/event ingestion routes.
- Revert module/API additions if integration issues appear.
