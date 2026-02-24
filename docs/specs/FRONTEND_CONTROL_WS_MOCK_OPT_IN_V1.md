# Frontend Control WS Mock Opt-In V1

## Goal
- Prevent accidental runtime interception of scheduler control websocket requests via global mock hooks.
- Require explicit opt-in before `__EDGERUN_CONTROL_WS_MOCK__` is honored.

## Non-goals
- No change to control websocket wire format or request semantics.
- No change to scheduler backend behavior.
- No change to Cypress scenario intent; only test setup wiring changes.

## Security and constraints
- Keep implementation localized to frontend control client code.
- Preserve ability to mock in end-to-end tests when explicitly enabled.
- Avoid new dependencies and preserve deterministic runtime behavior.

## Acceptance criteria
- `SchedulerControlWsClient` ignores `__EDGERUN_CONTROL_WS_MOCK__` unless an explicit global opt-in flag is `true`.
- Existing run-job happy-path Cypress test still passes by setting the opt-in flag in test setup.
- Frontend check/build and relevant Cypress spec pass.

## Rollout and rollback
- Rollout: merge client-side guard and updated test setup.
- Rollback: revert guard if test framework constraints require legacy behavior.
