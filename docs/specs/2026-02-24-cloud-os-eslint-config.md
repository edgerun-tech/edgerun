# Spec: Cloud-OS ESLint Flat Config

## Goal
Make `cloud-os` linting runnable under ESLint v10 by providing a flat config and ensuring `bun run lint` works consistently.

## Non-Goals
- Perform large-scale code cleanup to satisfy strict linting in this change set.
- Introduce new lint tooling or runtime dependencies.

## Security/Constraint Requirements
- Must use Bun as the package manager/runtime.
- Keep config deterministic and local to `cloud-os`.
- Avoid referencing files outside the repository root.

## Acceptance Criteria
- `cd cloud-os && bun run lint` completes successfully.
- ESLint configuration is located at `cloud-os/eslint.config.mjs`.
- Config ignores generated artifacts (node_modules, dist, .astro, out).
 - Rules that currently fail due to existing code issues are downgraded to warnings rather than removed.

## Rollout/Rollback
- Rollout: commit `cloud-os/eslint.config.mjs` and run lint.
- Rollback: revert the config file and restore previous lint behavior.
