# 2026-03-01 Remove Platform Connector Mode v1

## Goal

Simplify integrations setup by removing platform connector mode and using user-owned token/OAuth flow only.

## Non-goals

- No changes to MCP runtime start/stop behavior.
- No changes to token encryption/vault persistence behavior.
- No removal of existing integrations.

## Security and Constraints

- Keep fail-closed verification behavior.
- Keep token handling unchanged (no token logging/event leaks).
- Keep deterministic connector state transitions.

## Acceptance Criteria

1. Integrations setup no longer defaults/selects platform connector mode.
2. Token fields and guidance are shown for token-backed integrations without mode switching.
3. OAuth integrations continue to redirect via existing user-owned OAuth flow.
4. Frontend checks/build and relevant Cypress coverage pass.

## Rollout

- Normalize connector mode resolution to `user_owned` in lifecycle logic.
- Remove UI conditionals tied to platform mode in Integrations panel.
- Add concise helper copy clarifying single-mode setup.

## Rollback

- Revert lifecycle and Integrations panel changes in this slice.
