# 2026-03-01 Cloud Panel Consolidation Phase 1 v1

## Goal
Reduce duplication and payload drift in Cloud panel resource loading by extracting provider fetch/normalize logic into a shared module and aligning Cloudflare reads with local-bridge canonical endpoints.

## Non-goals
- No Cloud panel visual redesign.
- No provider coverage expansion.
- No node-manager route behavior changes.

## Security and Constraint Requirements
- Keep existing token sourcing behavior unchanged.
- Do not log provider tokens.
- Keep Cloudflare reads on loopback local-bridge pathing.
- Do not use port 8080.

## Design
- Add `frontend/intent-ui/src/lib/cloud/cloud-panel-providers.js` containing provider-specific fetch + normalization helpers.
- Move Docker, Cloudflare, GitHub workflow runs, and local workflow runner run ingestion out of `CloudPanel.jsx`.
- Update `CloudPanel.jsx` to orchestrate helper outputs and keep rendering-only concerns.
- Use local-bridge Cloudflare endpoints (`/v1/local/cloudflare/*`) for Cloud panel reads to match Cloudflare panel data contracts.

## Acceptance Criteria
1. Cloud panel continues rendering Docker/GitHub workflow resources and actions as before.
2. Cloudflare resources in Cloud panel come from local-bridge canonical endpoints.
3. Frontend checks and build pass.
4. Existing Cloud panel targeted Cypress tests pass.

## Rollout
- Ship helper module and Cloud panel refactor together.

## Rollback
- Revert Cloud panel helper module + panel wiring introduced by this document.
