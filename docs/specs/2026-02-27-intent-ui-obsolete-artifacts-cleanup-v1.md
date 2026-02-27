# 2026-02-27 Intent UI Obsolete Artifacts Cleanup V1

## Goal
- Remove obsolete nested runtime/deploy artifacts from `frontend/intent-ui` now that Intent UI is built via the canonical `frontend/` pipeline.

## Non-Goals
- No functional changes to `/intent/` or `/intent-ui/` UX.
- No rewrite of Intent UI source components/stores.

## Security and Constraints
- Keep Bun-only workflow in canonical frontend.
- Keep files required by `frontend/scripts/build-intent-ui.mjs`.

## Acceptance Criteria
1. Obsolete nested runtime/deploy files are removed from `frontend/intent-ui` (server entry, deploy/docker wrappers, nested build scripts/dist artifacts).
2. Required Intent UI source remains: `src/**` and `tailwind.config.cjs`.
3. `cd frontend && bun run check` passes.
4. `cd frontend && bun run build` passes.

## Rollout
- Delete obsolete files/directories only.
- Verify with full frontend checks.

## Rollback
- Restore removed artifacts from git if needed.
