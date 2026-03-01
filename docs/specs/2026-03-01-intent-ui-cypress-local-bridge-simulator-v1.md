# Intent UI Cypress Local Bridge Simulator v1

## Goal
Provide a deterministic Cypress helper for local bridge behavior so integration/eventbus UI flows can be validated without a live node-manager.

## Non-goals
- Replacing production bridge transport.
- Mocking business logic reducers.
- Simulating full protobuf node info payloads beyond minimum fields needed for runtime readiness.

## Security and Constraint Requirements
- Test helper must run only in Cypress browser context.
- No secrets are introduced in helper defaults.
- Bridge simulator should mirror event envelopes only; it must not fabricate integration success.

## Design
- Add `installLocalBridgeSimulator(win)` helper under `frontend/cypress/helpers/`.
- Helper behavior:
  - monkeypatch `window.WebSocket` with deterministic open/send/close lifecycle,
  - echo binary event frames back through `onmessage` to satisfy event-native reducers,
  - monkeypatch `window.fetch` for `/v1/local/node/info.pb` to return minimal valid protobuf response (`ok=true`).
- Update integration e2e specs to call the helper in `onBeforeLoad`.

## Acceptance Criteria
1. Integration Cypress specs no longer hand-roll WS mocks per file.
2. Bridge-required runtime reaches connected state in tests using helper.
3. Integration verify/link test passes with helper.
4. Tailscale integration test passes with helper.

## Rollout
- Test-only change in Cypress files.

## Rollback
- Remove helper import/usages in specs and restore prior per-spec stubs.
