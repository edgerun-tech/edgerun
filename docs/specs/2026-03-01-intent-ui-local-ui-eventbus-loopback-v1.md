# 2026-03-01 Intent UI Local UI Eventbus Loopback V1

## Goal
- Restore deterministic local UI behavior when the local bridge is unavailable or reconnecting.
- Ensure UI-intent topics (window actions, intent bar actions, integration UI events) execute immediately in-browser.
- Prevent duplicate handling when events are both emitted locally and echoed back from the bridge.

## Non-Goals
- Replacing the local bridge transport for node/runtime events.
- Redesigning topic namespaces or removing existing `intent.ui.*` and `ui.*` topics.
- Changing integration verification/runtime container semantics.

## Security and Constraints
- Keep local UI state transitions in-process to avoid cross-origin/popup side effects from delayed transport behavior.
- Preserve bridge transport for non-UI topics.
- Avoid duplicate event execution by deduplicating echoed envelope IDs.
- Do not add external dependencies.

## Design
1. In `frontend/intent-ui/src/stores/eventbus.js`, make `publishEvent(...)` always dispatch locally via in-memory listeners and timeline.
2. Mark `intent.ui.*` and `ui.*` topics as local-first and do not require bridge availability for execution.
3. Keep bridge forwarding for non-UI topics when socket is open.
4. Track recently published event IDs and ignore matching echoed bridge envelopes to prevent duplicate reducer execution.

## Acceptance Criteria
1. IntentBar quick actions that call `openWindow(...)` work even when local bridge socket is disconnected.
2. Opening Integrations and other UI panels no longer depends on bridge round-trip timing.
3. Echoed bridge events for locally-published IDs do not execute local reducers twice.
4. Frontend validation (`bun run check`, `bun run build`) passes.

## Rollout
- Land local loopback + dedupe in eventbus store.
- Validate with intent-ui checks and Cypress behavior that relies on UI intents.

## Rollback
- Revert eventbus loopback and dedupe changes in `frontend/intent-ui/src/stores/eventbus.js`.
- UI intents return to transport-only behavior.
