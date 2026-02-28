# Intent UI Linux First Device Connection V1

## Goal
- Add a deterministic first-device onboarding flow in Intent UI where the user chooses platform and gets a runnable command/script with clear instructions.
- Start with Linux as the only active platform option.
- Expose a localhost node-manager bridge port that browser-side eventbus can connect to without polling.

## Non-goals
- Full multi-platform installers (macOS/Windows/Android) in this slice.
- Full remote fleet enrollment orchestration through relay APIs in this slice.
- Replacing cluster transport/eventbus architecture in this slice.

## Security and Constraints
- Keep localhost bridge bound to loopback only by default (`127.0.0.1`), never wildcard by default.
- Keep wire contracts protobuf-based for node-manager local bridge endpoints.
- Fail-closed startup behavior for node-manager remains unchanged.
- Keep frontend in `frontend/`, bun-only JS workflows, no port `8080` usage.

## Acceptance Criteria
1. Devices drawer includes a `Connect Device` section with:
- platform selector,
- Linux selected by default,
- generated Linux command/script and step guidance,
- copy-to-clipboard action.
2. `edgerun-node-manager run` starts a localhost bridge listener (default `127.0.0.1:7777`).
3. Bridge exposes:
- protobuf node info endpoint,
- websocket endpoint for local eventbus transport.
4. Browser eventbus attempts websocket connection to local bridge and reflects runtime/device connection status.
5. Cypress coverage verifies user-visible Linux onboarding connection UI and command generation behavior.

## Rollout / Rollback
- Rollout:
1. Add runtime-proto local bridge schema.
2. Add node-manager local bridge server and websocket route.
3. Add Intent UI devices-panel onboarding UI and bridge connection state handling.
4. Add/update Cypress tests.
- Rollback:
1. Revert new local bridge proto + node-manager server wiring.
2. Revert devices-panel connection UI + browser bridge connection wiring.
3. Remove associated Cypress test.
