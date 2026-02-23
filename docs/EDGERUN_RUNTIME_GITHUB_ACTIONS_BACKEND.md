# edgerun-runtime as a Self-Hosted GitHub Actions Runner Backend

## Scope

This document defines a minimal, deterministic integration model for using `edgerun-runtime` as the execution backend for self-hosted GitHub Actions jobs.

Current implementation status:
- A feature-gated scaffold exists in `crates/edgerun-runtime/src/gha_runner.rs` behind `gha-runner-backend`.
- The scaffold includes:
  - control-plane contract (`ControlPlane`)
  - execution contract (`JobExecutor`)
  - runner lifecycle tick (`RunnerBackend::run_tick`)
  - placeholder execution path (`NoopJobExecutor`)

## Lifecycle Mapping (Queue -> Fetch -> Execute -> Logs -> Status)

Target lifecycle:
1. Queue: scheduler/control-plane stores pending job envelopes.
2. Fetch: runtime worker acquires a lease (`fetch_next_job`).
3. Execute: backend marks `InProgress`, then delegates to an executor.
4. Stream logs: backend sends line/chunk updates (`append_log_chunk`).
5. Report status: backend emits terminal state + result metadata (`report_state`, `report_result`).

`RunnerBackend::run_tick` already models this flow with no-op execution for now.

## Trust and Security Model

Trust boundaries:
- Untrusted: job payload (WASM bundle and user input), workflow provenance, and job metadata from scheduler.
- Trusted: runtime binary, local node policy, and control-plane authentication channel.

Required checks before real execution (TODO in scaffold):
- Verify ABI and runtime identity via:
  - `expected_runtime_id`
  - `expected_abi_version`
- Execute only through strict runtime entrypoints that enforce:
  - canonical bundle decoding
  - WASM import/export policy
  - memory and instruction limits

Operational security requirements:
- Control-plane requests must be authenticated and replay-resistant (lease IDs, monotonic sequence IDs).
- Log and status reporting must be idempotent and retry-safe.
- Artifacts must be content-addressed (hash first, then upload).

## Isolation Boundaries

Isolation levels (from current runtime guarantees):
- WASM-level isolation in-process via `wasmi`.
- Hostcall boundary restricted to `edgerun::{input_len,read_input,write_output}`.
- Deterministic resource limits (`max_memory_bytes`, `max_instructions`).

Recommended hardening for production backend:
- Run each job in an isolated worker process (one bundle per process).
- Apply per-job filesystem sandbox roots and no ambient credentials.
- Use outbound network egress policy appropriate for self-hosted runners.

## Artifact Handling

Desired artifact contract:
- Runtime output is emitted as:
  - `output_hash` (always)
  - optional buffered bytes (policy-based)
- Control plane stores metadata and blob separately:
  - metadata in job result
  - blob under content-addressed path keyed by hash

For GitHub Actions semantics:
- Step/job logs are streamed incrementally.
- Final job conclusion maps from runtime disposition:
  - `Succeeded` -> `success`
  - `Failed` -> `failure`
  - `Cancelled` -> `cancelled`
  - `InfrastructureError` -> runner/system failure path

## Scheduler / Control-Plane Contract

Minimal contract (implemented as trait methods today):
- `fetch_next_job() -> Option<ActionsJobLease>`
- `append_log_chunk(lease, text)`
- `report_state(lease, state, detail)`
- `report_result(lease, result)`

Job lease requirements:
- Unique `lease_id`
- Job identity (`job_id`, `run_id`, `attempt`)
- Routing labels (`label_selector`)
- Bundle bytes + expected runtime policy

Reliability requirements:
- Lease expiration + renewal heartbeat (future extension).
- At-least-once delivery with deduplication by `lease_id`.
- Terminal status must be exactly-once from scheduler perspective.

## Implementation Notes

Current scaffold intentionally does not execute job bundles. It records the full control-plane flow while returning an explicit unsupported/infrastructure result. This keeps architecture deterministic and integration-ready without introducing speculative runtime behavior.

Next implementation step:
1. Add a concrete executor that calls `execute_bundle_payload_bytes_for_runtime_and_abi_strict`.
2. Translate `ExecutionReport` and `RuntimeError` into job conclusions + structured diagnostics.
3. Add durable retry semantics for logs/status updates and coverage tests for duplicate delivery.
