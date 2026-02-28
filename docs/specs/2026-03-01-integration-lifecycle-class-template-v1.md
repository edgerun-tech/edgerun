# INTEGRATION_LIFECYCLE_CLASS_TEMPLATE_V1

## Goal
Introduce a reusable `IntegrationLifecycle` class that serves as the lifecycle template for all integrations in Intent UI.
Run integration lifecycle and verification execution in a dedicated web worker instead of the main thread.

## Non-Goals
- No provider-specific backend protocol changes.
- No redesign of integration UI flow in this change.

## Security and Constraints
- Existing fail-closed requirements remain unchanged (e.g., GitHub profile requirement).
- No token values are emitted in events or persisted beyond existing secure paths.
- Refactor must preserve current event-driven reducer behavior.

## Design
- Add `IntegrationLifecycle` class with lifecycle transition methods:
  - `hydrateConnection`
  - `connectConnection`
  - `disconnectConnection`
  - `setConnectorModeConnection`
  - `listConnectionView`
- Convert integration catalog entries into `IntegrationLifecycle` instances.
- Add an `integrations.worker` runtime and store bridge.
- Route integration lifecycle transitions and verification through the worker bridge.
- Replace ad-hoc lifecycle branching in `integrationStore` with class method calls.

## Acceptance Criteria
1. Every integration is created as `new IntegrationLifecycle(definition)`.
2. Lifecycle transitions in store use class methods, not duplicated branching logic.
3. Integration lifecycle transitions and verification execute in the web worker.
4. Existing integration behavior remains functionally equivalent.
5. Frontend check/build and integration-related Cypress specs pass.

## Rollout / Rollback
- Rollout: frontend-only deployment.
- Rollback: revert the lifecycle class and store wiring commit.
