# FRONTEND_FAVICON_BRAND_REFRESH_V1

## Goal
Replace the current emoji-based browser tab favicon with a cleaner Edgerun-branded icon system that remains legible at 16x16/32x32 and still communicates runtime status.

## Non-Goals
- Redesigning the full logo/wordmark family.
- Changing page title/status semantics.
- Changing runtime event sources that feed tab status.

## Security / Constraint Requirements
- Keep favicon generation deterministic and local (no external fetches/assets).
- Preserve reduced-motion behavior (no forced animation when `prefers-reduced-motion: reduce`).
- Do not add dependencies; use existing build/runtime tooling only.

## Acceptance Criteria
1. Dynamic favicon in `frontend/src/client.tsx` no longer uses emoji glyphs.
2. Dynamic favicon uses an Edgerun geometric mark with status color indicator.
3. Pulsing/running state remains visually distinct without relying on emoji animation.
4. Static generated favicon assets from `frontend/scripts/generate-brand-assets.mjs` are refreshed to match improved branding.
5. Frontend validation passes:
   - `cd frontend && bun run check`
   - `cd frontend && bun run build`

## Rollout Notes
- UI-only change; no backend contract impact.
- Shipped as part of normal frontend build output regeneration.

## Rollback Notes
- Revert `frontend/src/client.tsx` favicon rendering helpers.
- Revert `frontend/scripts/generate-brand-assets.mjs` favicon asset template changes.
- Regenerate assets via standard build if needed.
