# 2026-02-28 Intent UI Remove Profile Onboarding V1

## Goal
- Remove profile onboarding/bootstrap gate from Intent UI startup and account flows.
- Treat local TPM-backed node session as default-ready for UI capability access.

## Non-Goals
- Removing integration capability checks.
- Redesigning OIDC scope contracts.
- Reworking encrypted profile blob format.

## Security / Constraints
- Keep fail-closed node security at TPM layer unchanged.
- Keep scope-gated windows functional by ensuring default runtime carries local scope set.
- Avoid breaking GitHub PAT/user-owned integration persistence after onboarding removal.

## Acceptance Criteria
1. `/intent-ui/` no longer renders `ProfileBootstrapGate` or profile onboarding actions.
2. `profileRuntime` hydrates to ready/loaded local session by default when no stored profile session exists.
3. Integration token lifecycle still works (including GitHub) without onboarding-derived profile secret context.
4. Cypress coverage reflects “no onboarding gate” behavior.

## Rollout / Rollback
- Rollout: ship Intent UI runtime/UI updates and frontend build.
- Rollback: revert this change set to restore profile onboarding gate and previous profile session behavior.
