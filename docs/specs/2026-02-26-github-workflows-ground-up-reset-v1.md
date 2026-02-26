# GitHub Workflows Ground-Up Reset V1

## Goal
Remove all current GitHub Actions workflow definitions from the repository so CI/CD can be rebuilt from first principles.

## Non-goals
- No immediate replacement workflow set in this change.
- No behavior changes to runtime/frontend product code.
- No removal of local validation scripts (`scripts/*`) unless required for zero-workflow compatibility.

## Security and Constraints
- Keep repository deterministic and buildable locally without GitHub workflow files.
- Preserve local CI harness usability (`act` scripts) with explicit no-workflow messaging.
- Avoid introducing npm/pnpm flows.

## Acceptance Criteria
1. `.github/workflows/` contains no workflow YAML files tracked by git.
2. Local drift and action helper scripts do not fail merely because workflow files are absent.
3. Local validation commands complete and evidence is reported.

## Rollout
1. Add this spec.
2. Remove tracked workflow YAML files.
3. Update local helper scripts for no-workflow state.
4. Run validation and publish evidence.

## Rollback
- Restore workflow YAML files from git history.
- Revert helper script adjustments.
