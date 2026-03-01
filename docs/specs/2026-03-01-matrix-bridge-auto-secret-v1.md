# 2026-03-01 Matrix Bridge Auto Secret v1

## Goal

Remove manual token entry friction for Matrix bridge integrations by generating and using a local bridge secret automatically during setup.

## Non-goals

- No change to bridge runtime start/stop lifecycle.
- No change to credential vault persistence behavior after linking.
- No automatic third-party account login flows.

## Security and Constraints

- Generated secret must be high entropy and local-only until user links.
- Secret must not be emitted in UI events/logs.
- Keep fail-closed behavior if runtime verification cannot execute.

## Acceptance Criteria

1. Matrix bridge providers no longer require manual token field input in setup.
2. Verification/link uses generated or existing secret automatically.
3. UI copy clearly states that EdgeRun generates/manages this bridge secret.
4. Frontend checks/build and relevant Cypress coverage pass.

## Rollout

- Add local secret generation helper in Integrations panel.
- Apply auto-secret behavior only to providers tagged `matrix-bridge`.
- Update Cypress integrations availability test to remove manual token entry step.

## Rollback

- Revert Integrations panel auto-secret logic and related test updates.
