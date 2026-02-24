# FRONTEND_PRODUCTION_READINESS_GATES_V1

## Goal and non-goals
- Goal: add a deterministic production build path for `frontend/` that fails fast when release-critical environment and Solana wiring are missing or invalid.
- Goal: preserve current developer workflow (`bun run build`) for local iteration.
- Non-goal: change runtime UX, routing, theming, or data model.
- Non-goal: force a specific Solana cluster for all builds.

## Security and constraint requirements
- Production builds must not silently fall back to weak defaults for release metadata.
- Production builds must require explicit release identity and site metadata:
  - `EDGERUN_VERSION`
  - `EDGERUN_BUILD_NUMBER`
  - `EDGERUN_SITE_URL` (HTTPS)
  - `EDGERUN_SITE_DOMAIN`
  - `SOLANA_CLUSTER`
  - `SOLANA_RPC_URL` (HTTPS except `localnet`)
- For `SOLANA_CLUSTER=mainnet-beta`, `EDGERUN_TREASURY_ACCOUNT` must be non-empty.
- Solana program deployment config for the selected cluster must be present and non-empty in `frontend/config/solana-deployments.json`.

## Acceptance criteria
- `frontend/package.json` includes `build:prod` command that validates production configuration before build output generation.
- Validation exits non-zero with clear actionable error messages when required values are missing/invalid.
- `bun run build:prod` succeeds with valid production env and still writes output under `out/frontend/site`.
- Existing `bun run build` remains available for non-production local use.
- Frontend documentation includes production build instructions and required environment variables.

## Rollout and rollback
- Rollout:
  1. Use `bun run build:prod` in CI/release pipelines.
  2. Keep `bun run build` for local and exploratory builds.
- Rollback:
  1. Revert `build:prod` integration and validation script.
  2. Continue using existing `bun run build` path.
