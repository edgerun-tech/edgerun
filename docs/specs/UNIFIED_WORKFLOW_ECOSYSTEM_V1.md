# Unified Workflow Ecosystem Spec (V1)

## Status
- Proposed and implemented in this change set.

## Goal
- Consolidate repository execution into a single cohesive workflow surface that is deterministic, bun-first for JavaScript tooling, and verifiable with explicit proof commands.

## Non-goals
- Re-architecting Rust crate internals or CI job topology.
- Changing runtime protocol behavior or on-chain business logic.
- Replacing toolchains that are externally constrained beyond local repository workflow orchestration.

## Security and Constraint Requirements
- Repository-default JavaScript execution must use `bun`/`bunx` and must not introduce new `npm`/`pnpm`/`yarn` workflow paths.
- Frontend canonical app root remains `frontend/`.
- Required frontend baseline checks remain mandatory:
  - `cd frontend && bun run check`
  - `cd frontend && bun run build`
- Workflow evidence must be reproducible via explicit command list + pass/fail outcomes.
- Generated/build/temp artifacts remain under `out/`.

## Acceptance Criteria
1. A root-level unified workflow entrypoint exists for:
   - drift detection
   - check/lint/type validation
   - build validation
   - test execution
   - end-to-end verification wrapper
2. Drift detection explicitly guards against package manager drift in operational workflow files.
3. Existing obvious `npm`/`npx` operational workflow usage is replaced with `bun`/`bunx` where safe.
4. Root documentation references the unified workflow commands and expected usage.
5. Required validations are executed and reported with exact commands and outcomes.

## Rollout
1. Add spec document.
2. Add root scripts implementing unified workflow stages (`check`, `build`, `test`, `verify`, `drift`).
3. Add root `package.json` scripts so operators can run one coherent command surface.
4. Update existing operational scripts to `bun`/`bunx`.
5. Run validation commands and publish evidence.

## Rollback
- Remove unified workflow scripts and root package scripts.
- Revert operational script changes and return to prior per-subproject invocation.
- Keep this spec as historical design record, marked superseded.
