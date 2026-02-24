# FRONTEND_CI_PRODUCTION_GATES_V1

## Goal and non-goals
- Goal: enforce frontend production readiness gates in CI and release workflows, not only in local scripts.
- Goal: require both static quality checks and production build validation before publishing frontend artifacts.
- Non-goal: change frontend runtime UX, routing, or component behavior.
- Non-goal: add new frontend dependencies or change bundling architecture.

## Security and constraint requirements
- Use `bun` for install/build/check steps.
- Release-oriented frontend workflow execution must fail fast on:
  - lint/type/style/schema violations (`bun run check`)
  - missing/invalid production metadata and Solana runtime configuration (`bun run build:prod`)
- Solana configuration in CI must use real RPC source for an allowed cluster (`localnet`, `devnet`, `testnet`, `mainnet-beta`) and must not be mocked.
- Workflow defaults must remain deterministic and reproducible from repository state plus explicit env.

## Acceptance criteria
- `.github/workflows/ci.yml` includes a frontend gates job that runs:
  - `bun run check`
  - `bun run build:prod`
- `.github/workflows/frontend-release.yml` runs the same gates before archiving and publishing release artifacts.
- `.github/workflows/wiki-sync.yml` runs the same gates before wiki push.
- All added workflow env values satisfy `frontend/scripts/validate-production-build.mjs` requirements.

## Rollout and rollback
- Rollout:
  1. Merge workflow changes.
  2. Observe next PR and release runs for gate enforcement and timing impact.
- Rollback:
  1. Revert this spec-aligned workflow change set.
  2. Restore previous `bun run build`-only behavior while investigating blockers.
