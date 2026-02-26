# CI Reset Status

As of February 26, 2026, repository GitHub Actions workflow definitions under `.github/workflows/` are intentionally removed.

## Current state

- `.github/workflows/` contains no tracked workflow YAML files.
- Local workflow helpers are still available and intentionally no-op/diagnostic when no workflow files exist:
  - `scripts/actions-local-check.sh`
  - `scripts/actions-local-run.sh`

## Why

CI/CD is being rebuilt from first principles to avoid carrying forward broken or legacy workflow behavior.

## Operator guidance

- Use local deterministic validation commands until workflows are rebuilt:
  - `bun run drift:check`
  - `cd frontend && bun run check`
  - `cd frontend && bun run build`
  - `cargo check --workspace`
  - `cargo test --workspace`

- When new workflows are introduced, add them explicitly and validate via:
  - `scripts/actions-local-run.sh --list`
  - `scripts/actions-local-check.sh`
