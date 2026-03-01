# 2026-03-01 Reusable List Next Batch 2 v1

## Goal
Continue list/rendering consistency by applying shared virtualized primitives to additional dynamic surfaces and reducing bespoke list rendering code.

## Non-goals
- No backend/protocol changes.
- No interaction semantics changes for existing actions.
- No visual redesign beyond list rendering mechanics.

## Security and Constraints
- Preserve existing click/action callbacks.
- Keep UI deterministic and local.
- Do not introduce new third-party runtime dependencies.
- Do not use port 8080.

## Design
- Apply `VirtualAnimatedList` in `IntegrationsPanel` dynamic diagnostic lists where size can grow.
- Apply shared virtualized rendering to `OnvifPanel` camera card list.
- Add a small reusable `VirtualAnimatedResultList` wrapper for result-history style rows and adopt it in `IntentBar` history blocks.

## Acceptance Criteria
1. Integrations diagnostic list blocks use shared list primitive.
2. Onvif camera list uses shared virtualized primitive.
3. IntentBar history rendering uses shared result-list wrapper.
4. Frontend checks/build and relevant Cypress specs pass.

## Rollout
- Ship wrapper + panel adoptions together.

## Rollback
- Revert wrapper/component adoptions to previous direct list loops.
