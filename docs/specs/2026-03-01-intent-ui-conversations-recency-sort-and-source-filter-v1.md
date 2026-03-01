# 2026-03-01 Intent UI Conversations Recency Sort and Source Filter V1

## Goal and non-goals
- Goal:
  - Sort conversation threads by most recent activity first, regardless of provider/source grouping.
  - Add a lightweight source filter in the Threads view so operators can quickly narrow to channels (for example `call`, `beeper`, `email`, `ai`).
  - Keep thread selection and compose/send workflows stable while filtering/sorting is active.
- Non-goals:
  - Introduce backend query parameters for server-side filtering.
  - Change message payload schema or storage format.

## Security and constraint requirements
- Filtering and sorting stay client-side and deterministic.
- No additional PII persistence beyond existing local conversation/message storage keys.
- Preserve existing integration availability and bridge error behavior.

## Acceptance criteria
1. Threads list ordering is recency-first (`most recent activity` to `oldest`) and not grouped by source insertion order.
2. Recency uses latest known activity timestamp from thread metadata/messages, including local in-session messages when present.
3. Threads panel exposes a source filter with an `All` option and channel-specific options derived from available threads.
4. Switching source filter updates visible thread rows without breaking thread selection.
5. Add/update Cypress coverage that validates source filter behavior and recency order within a filtered source set.
6. Frontend validation passes:
   - `cd frontend && bun run check`
   - `cd frontend && bun run build`

## Rollout and rollback
- Rollout:
  - Add recency-normalized thread list derivation in Workflow Overlay.
  - Add source filter controls in Conversations panel.
  - Add Cypress regression for sorting/filtering behavior.
- Rollback:
  - Revert thread derivation and source filter UI to previous static source-grouped behavior.
