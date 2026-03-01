# 2026-03-01 Intent UI Integrations Two-Step Test Loader V1

## Goal and Non-Goals
- Goal: make Integrations dialog use a deterministic two-step flow:
  1. Required info entry
  2. Test run screen with progressive loader states, then unlocked capabilities on success.
- Goal: keep existing provider verification/link semantics (`integrationStore.verify` then `integrationStore.connect`) intact.
- Non-goal: changing integration verification backends, connector lifecycle state model, or profile/session policy rules.
- Non-goal: introducing React/Next `use client`; implementation remains Solid.js inside `frontend/intent-ui`.

## Security and Constraint Requirements
- Fail closed: linking remains disabled unless verification has succeeded.
- Do not persist new secret material beyond existing paths.
- Keep Bun-only frontend workflow and existing deterministic local-first architecture.
- Preserve provider-specific constraints (OAuth redirect providers, Flipper BLE secure-context requirement, Tailscale required fields).

## Acceptance Criteria
1. Integrations dialog stepper shows exactly two steps: `Required Info` and `Run Tests`.
2. Required fields are entered in step 1; step 2 cannot execute tests when required fields are missing.
3. Step 2 verification displays progressive loader states during test run.
4. On successful tests, unlocked capabilities are rendered in the same step and link action becomes available.
5. Existing Cypress integration flows are updated for the two-step contract and pass for touched specs.

## Rollout and Rollback
- Rollout: UI-only change in `frontend/intent-ui/src/components/layout/IntegrationsPanel.jsx` plus Cypress selector/step updates.
- Rollback: revert this spec's companion changes to restore previous multi-step dialog (`Values/Verify/Success`) and prior Cypress expectations.
