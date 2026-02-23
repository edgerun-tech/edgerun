# Runtime CI Always-On V1

## Goal
- Start running runtime CI gates automatically for normal development flow (`pull_request` and `push` to `main`).
- Make runtime regressions visible before release tags.

## Non-goals
- No expansion of heavyweight extended campaigns already handled by scheduled/manual workflows.
- No runtime logic changes.
- No changes to release provenance behavior.

## Security and constraints
- Keep runtime security/fuzz/UB deep checks opt-in/tag-gated to control CI cost and reduce noisy flakes.
- Keep deterministic/runtime-quality checks in the primary CI workflow so they are required in day-to-day merges.

## Acceptance criteria
- In `.github/workflows/ci.yml`, these jobs no longer require tag/manual dispatch only:
- `runtime-determinism`
- `runtime-calibration`
- `runtime-slo`
- Workflow remains valid YAML and existing commands remain unchanged.

## Rollout and rollback
- Rollout: remove restrictive `if` conditions from the three runtime quality jobs.
- Rollback: reintroduce the tag/manual `if` guards if CI load becomes unacceptable.
