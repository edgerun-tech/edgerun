# 2026-03-01 Reusable Virtual List Next Batch v1

## Goal
Extend reusable list consistency to media/feed surfaces by adding a reusable virtualized grid primitive and adopting shared virtual list rendering in additional panels.

## Non-goals
- No backend/API changes.
- No media processing changes.
- No design-language overhaul.

## Security and Constraints
- Keep existing interaction handlers and navigation behavior unchanged.
- Keep rendering deterministic and local-only.
- Avoid introducing new runtime dependencies.
- Do not use port 8080.

## Design
- Add `VirtualAnimatedGrid` in `components/common`:
  - viewport-aware row virtualization,
  - responsive column count from container width,
  - optional row animations.
- Adopt `VirtualAnimatedGrid` in `GooglePhotosPanel` media grid.
- Adopt `VirtualAnimatedList` in `FloatingFeedPanel` event list.
- Update reusable-list spec coverage accordingly.

## Acceptance Criteria
1. GooglePhotosPanel uses reusable virtualized grid rendering.
2. FloatingFeedPanel uses shared virtualized list rendering.
3. Existing panel tests pass with unchanged user-visible behavior.
4. Frontend check/build pass.

## Rollout
- Ship component + panel adoptions together.

## Rollback
- Revert new grid component and panel wiring to previous direct list mapping.
