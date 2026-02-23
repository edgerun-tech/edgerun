# Frontend Nav Full Width + Stats Layout V1

## Goal
- Remove the top-nav max-width clamp so the primary nav uses full viewport width.
- Reposition operational stats out of the crowded primary action cluster into a dedicated status rail that fits reliably.

## Non-goals
- No change to route diagnostics data sources or semantics.
- No change to terminal drawer behavior, wallet behavior, or nav link destinations.
- No redesign of mobile sheet navigation content beyond minor placement updates needed for fit.

## Security and constraints
- Preserve existing diagnostic `data-testid` attributes used by Cypress.
- Keep implementation in `frontend/components/nav.tsx` without introducing parallel app/layout roots.
- Keep deterministic rendering with no new runtime dependencies.

## Acceptance criteria
- Desktop/tablet top nav container is full width (`w-full`) and no longer constrained by `max-w-7xl`.
- Route/scheduler/overlay stats render in a dedicated row beneath the primary nav controls on non-mobile breakpoints.
- Stats remain readable and non-overlapping across breakpoints.
- Existing route diagnostics selectors remain present and valid for E2E assertions.

## Rollout and rollback
- Rollout: ship additive layout-only update to nav component and validate via frontend check/build.
- Rollback: revert `frontend/components/nav.tsx` to previous single-row stats placement if regressions appear.
