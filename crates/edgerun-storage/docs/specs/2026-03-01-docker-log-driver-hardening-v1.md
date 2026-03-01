# 2026-03-01 Docker Log Driver Hardening V1

## Goal
- Improve production safety for global Docker log-driver rollout by bounding per-stream memory and cleaning up ended sessions.

## Non-Goals
- Protocol changes to Docker log-driver API.
- Adding `ReadLogs` support.

## Security / Constraints
- Streaming decode must not allow unbounded buffer growth.
- Ended worker sessions should be removed from in-memory session registry.

## Design
1. Add `--max-stream-buffer-bytes` config (also plugin env `MAX_STREAM_BUFFER_BYTES`).
2. In worker stream read loop, clear buffered bytes and log warning when cap is exceeded.
3. Reap finished worker threads on each control request before processing endpoints.

## Acceptance Criteria
1. Driver compiles and passes clippy/tests.
2. Plugin packaging includes env for stream buffer cap.
3. Operator guide includes the new tuning env.

## Rollout / Rollback
- Rollout: keep default 8 MiB cap, tune if needed.
- Rollback: raise cap substantially or remove flag in a follow-up patch.
