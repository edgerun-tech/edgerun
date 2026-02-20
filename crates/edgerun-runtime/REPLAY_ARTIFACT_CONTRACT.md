# Replay Artifact Contract

Last updated: 2026-02-20

This contract defines the JSON payload accepted by the scheduler endpoint:

- `POST /v1/worker/replay`
- Related failure endpoint:
  - `POST /v1/worker/failure`

## Envelope

```json
{
  "idempotency_key": "hex-32-bytes",
  "worker_pubkey": "string",
  "job_id": "string",
  "artifact": {
    "bundle_hash": "hex-32-bytes",
    "ok": true,
    "abi_version": 1,
    "runtime_id": "hex-32-bytes",
    "output_hash": "hex-32-bytes",
    "output_len": 12,
    "input_len": 12,
    "max_memory_bytes": 1048576,
    "max_instructions": 50000,
    "fuel_limit": 50000,
    "fuel_remaining": 49210,
    "error_code": null,
    "error_message": null,
    "trap_code": null
  }
}
```

## Field semantics

- `idempotency_key`: deterministic hash used by scheduler for deduplication.
- `ok`: `true` for successful execution, `false` for runtime failure.
- `bundle_hash`: hash of canonical bundle payload bytes.
- `abi_version`: bundle ABI version (`BundlePayload.v`) when decode is available.
- `runtime_id`: bundle runtime ID when decode is available.
- `output_hash`: present only when execution succeeds.
- `fuel_*`: may be `null` if unavailable for a specific failure path.
- `error_code`: stable runtime error enum string (`RuntimeErrorCode`).
- `trap_code`: runtime trap category when applicable (for example `OutOfFuel`).

## Failure artifact rules

For failed runs:

- `ok` must be `false`
- `error_code` should be populated
- `output_hash` must be `null`
- `output_len` should be `null`

## Persistence

Scheduler stores replay artifacts by `job_id` and returns them in:

- `GET /v1/job/{job_id}`

Scheduler also stores worker failures by `job_id` and returns them in:

- `GET /v1/job/{job_id}` as `failures`

Scheduler deduplicates `result`, `failure`, and `replay` submissions by `idempotency_key`.
