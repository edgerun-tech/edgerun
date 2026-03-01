# 2026-03-01 — Intent UI WorkflowOverlay Refactor Phase 1 (v1)

## Goal
Reduce `frontend/intent-ui/src/components/layout/WorkflowOverlay.jsx` coupling and size without behavior changes by extracting constants, pure helpers, and event-channel emit wrappers.

## Non-goals
- No visual redesign.
- No protocol or API contract changes.
- No store schema changes.
- No drawer behavior changes.

## Constraints
- Keep local-bridge-first behavior unchanged.
- Keep event topics unchanged (`conversation.*`, `clipboard.*`).
- Maintain Bun/ESM frontend workflow.

## Phase 1 scope
1. Extract shared constants/class strings/panel metadata into `workflow-overlay.constants.js`.
2. Extract pure utility functions into `workflow-overlay.utils.js`.
3. Extract event emitter wrappers into `workflow-overlay.events.js`.
4. Replace duplicated rail button markup with metadata-driven rendering.

## Acceptance criteria
- `WorkflowOverlay` behavior remains unchanged.
- `bun run check` passes in `frontend/`.
- `bun run build` passes in `frontend/`.
- No remaining direct `publishEvent(...)` calls in `WorkflowOverlay.jsx` for extracted topics.

## Rollback
- Revert the four touched files and remove extracted helper modules.
