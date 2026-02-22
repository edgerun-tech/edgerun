# AGENTS.md

## Purpose
This repository must be operated with production-grade discipline. Work is accepted only when results are verifiable.

## Non-Negotiable Preferences
- Use `bun` as the JavaScript runtime and package manager.
- Do not introduce `pnpm` or `npm` workflows.
- Prefer static generation and minimal runtime dependencies.
- Keep architecture deterministic and dependable; avoid unnecessary complexity.
- Frontend canonical location is `frontend/`. Do not create parallel/ambiguous app roots.
- On-chain derived views must use real chain/RPC sources (`localnet`, `devnet`, `testnet`, `mainnet-beta`) and must not be mocked.

## Execution Standard
- Do not declare completion without proof.
- Every substantive change must include:
  1. Build validation
  2. Lint/type validation
  3. Relevant automated tests
  4. A short evidence report (exact commands + pass/fail)
- If a check cannot run, state the blocker explicitly and provide the exact reproduction command.

## Testing Requirements (Provable Results)
- Minimum validation for frontend changes:
  - `cd frontend && bun run check`
  - `cd frontend && bun run build`
- For behavior/UI changes, add or update end-to-end tests.
- E2E framework standard: `cypress` (preferred over Playwright).
- Tests must assert user-visible outcomes and architecture-critical behavior (rendering, routing, hydration/interactivity, data wiring), not just snapshots.

## Architecture Guardrails
- Keep theming centralized; style guide is authoritative.
- Prefer server-side/static generation for docs and content pages.
- Keep docs generated from source code where possible.
- Keep generated/build/temp artifacts under `out/` at repo root.
- Avoid dead files, duplicate paths, and legacy outputs.

## Dependency and Performance Policy
- Reduce dependencies where possible.
- Keep generated assets optimized.
- Avoid adding heavy client-side libraries unless strictly justified.
- Track bundle/output size impact when making frontend runtime changes.

## Definition of Done
A task is done only when:
1. Requested behavior is implemented.
2. Required checks/tests pass.
3. Evidence is reported clearly.
4. No known regression is left unreported.

## Suggested Evidence Format
- `Scope`: what changed
- `Commands run`: exact commands
- `Results`: pass/fail per command
- `Artifacts`: key output paths/sizes (if relevant)
- `Known limitations`: explicit, if any

DONT PUT ANYTHING ON PORT 8080!!!

## Operator Override
- If unexpected/foreign workspace changes are detected, pause and ask before proceeding by default.
- If the operator explicitly says to `commit all` and continue, treat that as approval to include all current changes and proceed without further pause.
