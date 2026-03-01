# 2026-03-01 Intent UI Conversations Thread Search V1

## Goal and non-goals
- Goal:
  - Add fast client-side thread search in the Conversations Threads view to quickly find active chats across channels.
  - Make search work together with existing recency sorting and source filters.
  - Keep selection and compose behavior stable while narrowing thread results.
- Non-goals:
  - Add server-side full-text search APIs.
  - Persist search query across sessions.

## Security and constraint requirements
- Search remains local in browser memory with no additional persistence.
- Keep existing conversation data model unchanged.
- Preserve deterministic ordering (recency-first) before/after filtering.

## Acceptance criteria
1. Threads panel shows a search input for title/subtitle/preview/channel/participants matching.
2. Search results combine with source filter (`All` and channel chips), not replacing it.
3. Thread ordering remains newest-first among filtered results.
4. Clearing query restores full source-filtered list.
5. Empty-state copy for filters/search is explicit and user-friendly.
6. Add/update Cypress coverage to validate search + recency/filter interaction.
7. Validation passes:
   - `cd frontend && bun run check`
   - `cd frontend && bun run build`

## Rollout and rollback
- Rollout:
  - Add search query state in workflow overlay and apply it to derived thread list.
  - Add search input/clear control in conversations threads UI.
  - Add Cypress regression for thread search behavior.
- Rollback:
  - Remove query state and search input and revert to source-filter-only threads view.
