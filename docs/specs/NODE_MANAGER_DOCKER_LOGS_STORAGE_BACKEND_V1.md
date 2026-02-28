# NODE_MANAGER_DOCKER_LOGS_STORAGE_BACKEND_V1

## Goal and Non-Goals

### Goal
- Make Docker log retrieval use EdgeRun storage as backend.
- Persist scraped Docker logs into `edgerun-storage` segmented journal.
- Serve `/v1/local/docker/logs` from stored log entries, not direct daemon output only.

### Non-Goals
- No Docker daemon-level log driver replacement in this iteration.
- No cross-node replication for log storage.

## Security and Constraints
- Storage path defaults to local node path and remains node-local.
- Log payloads are bounded via endpoint limit/tail controls.
- Endpoint remains read-only and CORS behavior unchanged.

## Acceptance Criteria
- `edgerun-node-manager` links against `edgerun-storage`.
- `/v1/local/docker/logs` persists discovered logs into storage.
- `/v1/local/docker/logs` responses are returned from storage-backed records.
- Duplicate log records are deduplicated per node-manager process session.

## Rollout and Rollback
- Rollout by enabling storage-backed ingest in local bridge logs path.
- Rollback by restoring previous direct `docker logs` response path.
