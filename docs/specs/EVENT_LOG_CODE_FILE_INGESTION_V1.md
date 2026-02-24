# EVENT_LOG_CODE_FILE_INGESTION_V1

## Goal
Add a deterministic, append-only ingestion path into the existing observer event log for repository code/file changes, with emphasis on machine-readable change metadata per file.

## Non-Goals
- No replacement of existing `observe append` or `observe ingest-stdio` commands.
- No mutable/rewritable event records.
- No dependency on external telemetry services.

## Security and Constraints
- Reuse current append-only storage path (`edgerun-storage` via observer sink).
- Derive change metadata from local git state only.
- Emit one event per changed file and chain hashes via `prev_event_hash`.
- Keep command output deterministic and JSON.

## Acceptance Criteria
1. New CLI path ingests code/file changes into observer storage without mutating prior events.
2. Events include file path, git status, optional line stats, and scope classification (`code` vs `file`).
3. Command works with configurable base reference (default `HEAD`) and optional untracked-file inclusion.
4. Dry-run mode returns planned payloads without writing.
5. Unit tests cover parser/classification logic.
6. Validation evidence includes:
   - `cargo check -p edgerun-cli`
   - `cargo test -p edgerun-cli`

## Rollout / Rollback
- Rollout: additive CLI subcommand under `observe`.
- Rollback: remove subcommand and parser helpers; previously appended events remain immutable.
