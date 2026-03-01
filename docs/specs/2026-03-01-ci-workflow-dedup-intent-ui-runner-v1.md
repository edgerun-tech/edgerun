# 2026-03-01 CI Workflow Dedup Intent UI Runner v1

## Goal
Remove duplicate frontend CI executions by making the dedicated Intent UI runner workflow reusable/manual instead of auto-triggered on push/pull_request.

## Non-goals
- No change to primary `CI` workflow job coverage.
- No change to Rust checks/tests behavior.
- No required-check policy edits in this slice.

## Security and Constraints
- Preserve least-privilege GitHub workflow permissions.
- Keep shared runner script usage intact.
- Avoid introducing new external actions.

## Design
- Update `.github/workflows/intent-ui-workflow-runner-ci.yml`:
  - remove `push`/`pull_request` triggers,
  - add `workflow_call` and `workflow_dispatch` triggers.
- Keep existing job implementation unchanged so it can be run manually or called from other workflows.

## Acceptance Criteria
1. Intent UI runner workflow no longer auto-runs on push/pull_request.
2. Workflow can be invoked manually and via `workflow_call`.
3. Existing CI workflow lint/reference checks pass.

## Rollout
- Merge workflow trigger change and monitor Actions run count for duplicate reduction.

## Rollback
- Restore previous push/pull_request triggers in `intent-ui-workflow-runner-ci.yml`.
