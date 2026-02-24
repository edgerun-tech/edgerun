# Frontend Email Collection with Cloudflare KV V1

## Goal
- Enable footer email collection on the frontend and persist submissions in Cloudflare serverless storage (Workers KV).

## Non-goals
- No CRM, double-opt-in, or campaign automation integration.
- No admin UI for viewing/exporting collected emails.
- No change to existing site navigation or page structure beyond enabling the footer form.

## Security and constraints
- Keep the canonical frontend app under `frontend/`.
- Keep runtime and deployment deterministic; no `npm`/`pnpm` workflows.
- Accept only valid email input and return explicit client-visible success/failure states.
- Persist submissions in Cloudflare KV through a Worker endpoint (`/api/lead`) on the same frontend deployment target.
- Keep static asset serving behavior intact for all non-API routes.

## Acceptance criteria
- Footer email input is enabled and submit-capable in the UI.
- Frontend sends submissions to `POST /api/lead`.
- Cloudflare Worker handles `POST /api/lead`, validates email, and stores normalized records in KV.
- Cypress covers user-visible submit success behavior.
- `cd frontend && bun run check` passes.
- `cd frontend && bun run build` passes.
- Relevant frontend E2E tests pass.

## Rollout and rollback
- Rollout:
  - Deploy updated frontend worker config with KV binding.
  - Create/bind KV namespace `EMAIL_SIGNUPS` in Cloudflare environment before production rollout.
- Rollback:
  - Revert footer form to disabled state and remove `/api/lead` handler routing from worker.
