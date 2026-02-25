# 2026-02-25 Frontend Landing Mystic Hero Stats V1

## Goal
- Introduce a Mystic UI-inspired hero stats band on the landing page that improves scannability and trust signals without introducing mock data.
- Reuse existing on-chain/runtime data wiring (`data-chain-field`, deployment status selectors) so the section remains deterministic and architecture-consistent.

## Non-Goals
- No new RPC endpoints, polling loops, or alternate data sources.
- No redesign of non-hero landing sections.
- No dependency additions for UI libraries or animation frameworks.

## Security/Constraint Requirements
- Keep using live chain/RPC-derived values already provided by runtime status wiring; do not hardcode synthetic KPI claims.
- Keep frontend stack bun-first; no npm/pnpm workflow changes.
- Maintain centralized token/theme usage and existing professional visual language.
- Preserve existing CTA routing behavior (`/run/`, `/workers/`) and deployment status messaging.

## Acceptance Criteria
- Landing hero includes a new stats band with at least cluster, slot, TPS, and block height surfaced via existing `data-chain-field` bindings.
- Hero continues to show deployment badge/details via existing selectors (`data-deployment-badge`, `data-deployment-detail`).
- Existing landing CTA links remain unchanged and valid.
- Cypress coverage validates user-visible landing behavior for the new stats band wiring (presence + expected labels/fields).
- `cd frontend && bun run check` passes.
- `cd frontend && bun run build` passes.

## Rollout and Rollback
- Rollout: ship as a landing-only visual enhancement; no runtime config migration required.
- Rollback: revert `frontend/components/landing/hero-section.tsx`, associated Cypress test updates, and this spec file if regression is observed.
