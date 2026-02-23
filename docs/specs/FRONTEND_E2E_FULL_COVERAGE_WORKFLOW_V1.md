# Frontend E2E Full Coverage Workflow V1

## Goal
- Provide a single deterministic frontend e2e command that executes both:
- static-shell/client-route Cypress coverage, and
- compose/local terminal route integration coverage.

## Non-goals
- No new product behavior in runtime UI.
- No change to business logic or protocol contracts beyond test configuration.
- No introduction of npm/pnpm workflows.

## Security and constraints
- Keep all test orchestration in repository scripts.
- Respect port policy: do not use port `8080`.
- Keep Cypress assertions user-visible and architecture-critical.

## Acceptance criteria
- `cd frontend && bun run e2e:run` executes full frontend e2e coverage end-to-end.
- Compose-backed terminal spec reads scheduler/term-server ports from Cypress env and defaults to repository-standard ports.
- Existing split commands remain available (`e2e:core`, `e2e:compose`) for debugging.
- `cd frontend && bun run check` and `cd frontend && bun run build` continue to pass.

## Rollout and rollback
- Rollout: add dedicated orchestration scripts and wire package scripts to the unified flow.
- Rollback: point `e2e:run` back to core-only command and keep compose coverage as separate manual step.
