# Long-Running Job Contract and Event Logging (Spec v1)

## Goal

Define a deterministic, auditable contract for long-running jobs so operator intent, execution, and current state are always reconstructable from repository-controlled artifacts and append-only logs.

## Non-goals (v1)

- Full desktop surveillance/telemetry capture by default.
- Replacing existing service logs.
- Enforcing production-grade cryptographic attestation on all local/dev events.

## Core Contract

Every long-running job MUST follow this lifecycle:

1. `job.opened`
2. `job.accepted`
3. `job.running`
4. `job.blocked | job.completed | job.failed | job.aborted | job.rolled_back`

No execution steps are allowed before `job.accepted`.

## Required Event Types (v1 minimum)

- `job.opened`
- `job.accepted`
- `job.heartbeat`
- `job.progress`
- `job.blocked`
- `job.completed`
- `job.failed`
- `job.aborted`
- `job.rolled_back`
- `action.planned`
- `action.result`
- `io.user_input`
- `io.agent_output`
- `fs.changed`

## Event Envelope

Each event MUST include:

- `event_id`
- `job_id`
- `run_id`
- `ts_unix_ms`
- `actor` (`user|agent|system`)
- `event_type`
- `payload` (JSON object)
- `prev_event_hash` (nullable)
- `event_hash`

## Required Payload Fields by Event

### `job.opened`

- `goal`
- `owner`
- `scope`
- `success_criteria`
- `cancel_command` (nullable)
- `rollback_plan` (nullable)

### `action.planned`

- `description`
- `commands` (list)
- `targets` (files/services/hosts)

### `action.result`

- `status` (`ok|error`)
- `commands_run`
- `exit_codes`
- `artifacts`
- `notes`

## Storage Backend Requirement

Event log MUST be appended to `edgerun-storage` segment(s).  
JSONL-only local files can exist for local troubleshooting, but are not canonical.

## Security / Privacy Policy (v1)

- Default capture tier is `ops`:
  - include: job/action events, user input, agent output, changed files metadata
  - exclude: clipboard/process/window telemetry unless explicitly enabled
- Secrets must be redacted before append.

## Environment Policy

- `localnet`: permissive capture settings allowed.
- `devnet|testnet|mainnet-beta`: strict redaction and stable schema required.

## Acceptance Criteria

Implementation is accepted when:

1. A CLI/API exists to append contract events into `edgerun-storage`.
2. A long job can be represented end-to-end with required lifecycle events.
3. Event reconstruction can show current state from latest event chain.
4. Proof includes exact commands and resulting stored event records.

## Rollout Plan

1. Implement minimal observer writer (manual append + stdin ingest).
2. Integrate with operator workflows and service scripts.
3. Add optional collectors (process/logs/clipboard) behind explicit opt-in.

## Rollback Plan

- Disable observer ingestion path via flag/env.
- Preserve already written append-only segments.
- Resume legacy workflow without deleting prior audit data.

