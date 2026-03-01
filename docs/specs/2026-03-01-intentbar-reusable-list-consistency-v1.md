# 2026-03-01 IntentBar Reusable List Consistency v1

## Goal
Adopt the shared `VirtualAnimatedList` primitive in IntentBar list-heavy sections to keep list behavior and animation consistency aligned with other panels.

## Non-goals
- No command-processing logic changes.
- No result ranking/ordering changes.
- No visual redesign beyond list rendering mechanics.

## Security and Constraints
- Preserve existing command execution and action callbacks.
- Keep item click/run handlers unchanged.
- Avoid any backend/API changes.
- Do not use port 8080.

## Design
- Update `IntentBar.jsx` to replace direct `<For>` list mapping in:
  - filter results list,
  - command explorer group item lists,
  - pinned and regular result-history lists.
- Use `VirtualAnimatedList` stack layout with per-section sizing/overscan tuned for current row heights.
- Keep Motion wrappers for existing result-card transitions where already present.

## Acceptance Criteria
1. IntentBar lists render via shared virtual/animated list primitive.
2. Existing actions (run, pin, remove, apply command) remain unchanged.
3. Frontend check/build pass.

## Rollout
- Ship IntentBar list refactor with existing reusable list foundation.

## Rollback
- Revert IntentBar list sections to prior direct `<For>` rendering.
