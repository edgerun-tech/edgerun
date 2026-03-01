# 2026-03-01 Matrix Integration Token Guidance v1

## Goal

Make Matrix bridge integrations easier to connect by explicitly explaining what token is required and where to obtain it inside the Integrations dialog.

## Non-goals

- No change to matrix bridge runtime startup flow.
- No change to token storage/encryption behavior.
- No change to supported bridge provider list.

## Security and Constraints

- Keep fail-closed token validation behavior.
- Do not display token values in logs or event payloads.
- Keep guidance provider-aware but deterministic and static.

## Acceptance Criteria

1. For matrix-bridge providers in user-owned mode, Step 1 displays explicit guidance clarifying token origin.
2. Guidance states this is a bridge provisioning/API secret (not OAuth).
3. Guidance is concise in-product copy focused on what value to paste (no external doc dependency).
4. Frontend checks/build and targeted Cypress coverage pass.

## Rollout

- Add matrix token guidance helper copy to IntegrationsPanel.
- Add Cypress assertion for a representative matrix provider.

## Rollback

- Revert IntegrationsPanel guidance block and test additions.
