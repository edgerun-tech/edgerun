# CI_RUNTIME_SMART_GATING_V1

## Goal and non-goals
- Goal: reduce CI wall-clock and runner cost by skipping heavyweight jobs when changed files are unrelated.
- Goal: preserve production safety by always running full gates on release tags and manual dispatch.
- Non-goal: reduce validation depth for runtime- or frontend-touching changes.
- Non-goal: change build artifacts, deployment behavior, or test semantics.

## Security and constraint requirements
- Change detection must be deterministic from repository diff state in GitHub Actions.
- Heavy jobs must still execute when:
  - runtime/frontend/CI workflow files are modified, or
  - the run is a release tag (`refs/tags/v*`), or
  - the run is manually dispatched.
- Bun-only policy remains unchanged for JavaScript workflows.

## Acceptance criteria
- `.github/workflows/ci.yml` contains a `changes` job producing scoped outputs (`frontend`, `rust`, `runtime`, `ci`).
- `frontend-gates` runs only when frontend-relevant scope changed, plus tags/manual runs.
- `rust-checks` runs only when rust/CI scope changed, plus tags/manual runs.
- `runtime-determinism`, `runtime-calibration`, and `runtime-slo` run only when runtime/CI scope changed, plus tags/manual runs.

## Rollout and rollback
- Rollout:
  1. Merge gated workflow.
  2. Compare CI duration on docs-only vs runtime/frontend PRs.
- Rollback:
  1. Revert this change set.
  2. Return to always-on heavy jobs while preserving previous correctness baseline.
