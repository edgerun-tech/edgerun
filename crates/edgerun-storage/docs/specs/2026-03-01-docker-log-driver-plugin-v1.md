# 2026-03-01 Docker Log Driver Plugin V1

## Goal
- Provide a real Docker logging-driver entrypoint in `edgerun-storage` so Docker containers can stream logs directly into storage-backed VFS log mode.
- Keep storage source-agnostic while supporting Docker plugin lifecycle endpoints.

## Non-Goals
- Full Docker plugin packaging/install automation.
- `ReadLogs` support (tail/read-back via Docker daemon API).
- Multi-host fan-in orchestration.

## Security / Constraints
- Bind only to a Unix domain socket path explicitly configured by operator.
- Validate request size and reject oversized bodies.
- Keep append path idempotent using stable keys derived from container/source/time/message.
- Use bounded in-memory batching before flush to storage.

## Design
1. Add binary `docker_log_driver` with minimal HTTP-over-UDS server implementing:
- `POST /Plugin.Activate`
- `POST /LogDriver.Capabilities`
- `POST /LogDriver.StartLogging`
- `POST /LogDriver.StopLogging`
2. `StartLogging` spawns a worker per `File` path.
3. Worker opens Docker-provided log FIFO/file and decodes entries as length-delimited protobuf `LogEntry` records; fallback to line mode if decoding fails.
4. Worker maps entries into `LogIngestEntry` and flushes via `StorageBackedVirtualFs::ingest_log_entries` in batches.
5. `StopLogging` signals worker to stop and joins it.

## Acceptance Criteria
1. Endpoint contract works for activate/capabilities/start/stop with JSON request/response payloads expected by Docker log-driver plugin flow.
2. Ingested records produce `log_entry_appended` events in VFS for target branch.
3. `cargo check -p edgerun-storage`, `cargo clippy -p edgerun-storage --bin docker_log_driver -- -D warnings`, and focused tests pass.
4. Operator guide includes concrete run steps for the log-driver binary and socket path usage.

## Rollout / Rollback
- Rollout: run `docker_log_driver` as a host service and point Docker plugin/runtime config to its Unix socket.
- Rollback: disable log-driver integration and continue using `docker_log_ingest` stdin adapter mode.
