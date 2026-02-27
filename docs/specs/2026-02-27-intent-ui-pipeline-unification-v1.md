# 2026-02-27 Intent UI Pipeline Unification V1

## Goal
- Remove duplicate Intent UI runtime/build assumptions and make Intent UI assets build from the canonical `frontend/` pipeline only.

## Non-Goals
- No full rewrite of Intent UI internals in this step.
- No changes to `/intent/` functional UX.

## Security and Constraints
- Bun-only workflows.
- Keep canonical frontend root in `frontend/`.
- Do not require nested package install/build steps under `frontend/intent-ui` during normal `frontend` build.

## Acceptance Criteria
1. `frontend/scripts/build-intent-ui.mjs` does not call nested `bun install` or `bun run build` in `frontend/intent-ui`.
2. Intent UI JS bundle is produced directly via Edgerun frontend tooling (`esbuild` + Solid plugin) into `frontend/public/intent-ui/client`.
3. Intent UI CSS is produced directly via Edgerun frontend tooling (`tailwindcss`) into `frontend/public/intent-ui/app.css`.
4. `cd frontend && bun run check` passes.
5. `cd frontend && bun run build` passes.

## Rollout
- Replace script implementation only; keep script name (`intent-ui:sync`) stable.

## Rollback
- Restore previous script that shells into nested `frontend/intent-ui` build.
