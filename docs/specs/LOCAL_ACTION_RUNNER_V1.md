# Local Action Runner V1

## Goal
- Provide a deterministic local command to execute GitHub Actions workflows/jobs with `act`.
- Support fast iteration for runtime/CI jobs without waiting for remote GitHub runs.

## Non-goals
- No replacement of GitHub-hosted CI.
- No custom workflow engine changes.
- No changes to workflow job logic.

## Security and constraints
- Reuse repository workflow files from `.github/workflows/`.
- Keep command wrapper explicit about `workflow`, `event`, and optional `job`.
- Write logs under `out/actions-local/`.
- Keep bun-first policy unchanged (this tool only wraps `act`).

## Acceptance criteria
- A script exists to run a selected workflow locally with optional job filter.
- Script supports real execution and dry-run mode.
- Makefile exposes convenient targets for runtime-local workflow execution.
- At least one local dry-run command is validated and reported.

## Rollout and rollback
- Rollout: add `scripts/actions-local-run.sh` and Makefile targets.
- Rollback: remove script + targets and continue using ad-hoc `act` commands.
