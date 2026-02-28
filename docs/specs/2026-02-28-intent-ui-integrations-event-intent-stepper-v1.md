# INTENT_UI_WHOLE_UI_EVENT_INTENT_STATE_V1

## Goal
Refactor Intent UI event architecture so core UI actions are intent-driven and state is event-derived across the UI, with integrations as the first deep workflow:
1. Core UI actions (window lifecycle, clipboard actions, integration lifecycle) emit intents to eventbus.
2. Stores subscribe to eventbus topics they own and derive state through reducers from events.
3. Integrations panel presents icon-only providers with hover tooltips.
4. Connecting a provider opens a stepper dialog that collects required values, runs a verification step, and ends on a success screen listing unlocked capabilities.

## Non-Goals
- Full migration of every panel-specific local state in one iteration.
- Replacing current backend APIs for provider auth/verification.
- Building provider-specific deep verification for every third-party API in this iteration.

## Security And Constraints
- Keep fail-closed behavior for availability: integrations are not available unless connected and profile session constraints pass.
- Never log or publish raw integration secrets in event payloads or status messages.
- Maintain existing encrypted profile secret behavior for sensitive tokens.
- Keep Bun-only workflows and frontend root under `frontend/`.

## Design
- Introduce canonical UI intent/event topics in a single constants module (`window`, `clipboard`, `integration` domains).
- Route window store and integration store mutations through event reducers that consume intent/event topics.
- Integrations panel emits intents (`intent.integration.*`) for connect/disconnect/mode/test operations.
- Integrations store subscribes to those topics and derives connection state via a reducer from resulting `integration.*` events.
- Existing UI button handlers should use store APIs that now publish intents, so behavior remains compatible while transport shifts to eventbus.
- Stepper flow in dialog:
  - Step 1: ownership/auth mode
  - Step 2: required provider inputs
  - Step 3: verification (async)
  - Step 4: success (unlocked capabilities)
- Tailscale verification remains real by calling `/api/tailscale/devices`.

## Acceptance Criteria
- Window open/close/minimize/maximize and clipboard actions are published as intents and reduced from events.
- Integrations list in panel is icon-only with hover tooltip metadata.
- Clicking provider connect action opens stepper dialog.
- Stepper does not allow finalize/link until verification passes.
- Success step shows provider unlocked capabilities.
- Integration state updates occur through eventbus topics and reducer application.
- Cypress coverage updated for key changed UX paths and connection outcomes.

## Rollout
- Frontend-only rollout with static build artifacts and backward-compatible store APIs.
- Preserve existing `integrationStore` API so current callers remain compatible.

## Rollback
- Revert modified integrations panel/store and constants module to prior commit.
- Existing localStorage/profile secret schema remains backward compatible.
