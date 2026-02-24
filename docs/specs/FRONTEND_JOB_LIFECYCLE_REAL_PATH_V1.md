# Frontend Job Lifecycle Real Path V1

## Goal
- Ensure a user can submit a job, track execution state, and collect execution artifacts from real scheduler state (not static placeholders).
- Restore protocol compatibility between browser control WS client and scheduler control WS endpoint.

## Non-goals
- No changes to on-chain program economics or settlement formulas.
- No custodial wallet/payment UX redesign.

## Security and constraints
- Preserve existing binary bincode control WS protocol for workers/CLI.
- Add browser-safe control WS compatibility without weakening existing request validation.
- Frontend tracking must consume live `job.status` responses and avoid static fake result records.

## Acceptance criteria
1. Scheduler `/v1/control/ws` accepts browser text/JSON control messages for `job.create` and `job.status` (while retaining binary protocol).
2. Frontend run flow polls and renders live `job.status` after successful submission.
3. Frontend exposes downloadable execution artifact snapshot (status/reports/failures/quorum) for submitted jobs.
4. Frontend no longer relies on static `frontend/lib/content.ts` data for submitted job tracking path.
5. `cd frontend && bun run check`, `cd frontend && bun run build`, `cd frontend && bun run e2e:core` pass.
6. Scheduler compiles with protocol compatibility changes (`cargo check -p edgerun-scheduler`).

## Rollout and rollback
- Rollout: ship scheduler compatibility + frontend tracking in same release.
- Rollback: revert this change set; binary worker/CLI path remains intact.
