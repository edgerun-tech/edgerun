# Intent UI Connector Ownership Mode V1

## Goal
- Make platform-managed connectors available by default in Intent UI.
- Allow users to replace a platform connector with their own connector configuration per provider.
- Keep this as a deterministic local-first UX state in the integrations store.

## Non-goals
- No backend connector orchestration in this change.
- No cloud tenant provisioning for connector apps.
- No policy-engine coupling changes.

## Security and constraints
- Connector ownership mode is explicit and persisted (`platform` or `user_owned`).
- Switching to `user_owned` must fail closed until user credentials/auth are configured.
- Platform mode must not silently consume user tokens from prior sessions.
- Web3 remains user-owned only.

## Acceptance criteria
- Integrations store tracks connector ownership mode per provider.
- Providers that support platform mode are available by default in `platform` mode.
- UI exposes mode selector in provider dialog (`Platform connector`, `Use my own`).
- Users can switch from platform to user-owned and state updates immediately.
- Cypress test verifies mode switch behavior.
- Frontend checks/build pass.

## Rollout and rollback
- Rollout: additive mode metadata and UI controls.
- Rollback: remove mode metadata and revert to previous connect/disconnect behavior.
