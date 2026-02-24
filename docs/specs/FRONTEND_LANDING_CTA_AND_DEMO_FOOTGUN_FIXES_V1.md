# Frontend Landing CTA and Demo Footgun Fixes V1

## Goal
- Remove misleading landing-page CTA routing where both hero CTAs send users into docs flows.
- Remove duplicate destination ambiguity in footer resource links.
- Remove private-key-like output from the browser terminal demo payload to prevent unsafe copy/paste behavior.

## Non-goals
- No redesign of landing layout or typography.
- No changes to backend APIs or scheduler behavior.
- No changes to docs generation pipelines beyond link targets.

## Security and constraints
- Keep changes localized to frontend landing/footer/demo components.
- Preserve deterministic demo behavior while reducing sensitive output exposure.
- Keep build/test workflows on `bun`.

## Acceptance criteria
- Hero CTAs route to distinct operational paths (not both docs).
- Footer "Getting Started" and "API Reference" route to distinct canonical docs targets.
- Terminal demo output no longer includes `keypair_hex`.
- Frontend check/build pass and E2E assertions cover these outcomes.

## Rollout and rollback
- Rollout: ship link/output fixes with Cypress coverage for landing CTA/link correctness and demo output guard.
- Rollback: revert touched components/spec if this blocks onboarding flows.
