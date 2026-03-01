# 2026-03-01 GitHub Workflow Runs and Local Runner v1

## Goal
Make workflow execution visible and usable from Intent UI Cloud panel by:
1. showing recent GitHub Actions workflow runs,
2. adding a local workflow runner entrypoint that executes the same CI logic,
3. adding a dedicated CI workflow that reuses that shared runner logic.

## Non-goals
- No replacement of existing primary CI pipelines.
- No generic arbitrary-command runner endpoint.
- No persistent long-term logs beyond lightweight local run history metadata.

## Security and Constraints
- Require GitHub token for remote run retrieval.
- Keep local runner workflow ids allowlisted.
- Execute local runner commands through existing dockerized osdev frontend container only.
- Do not log GitHub tokens.
- Do not use port 8080.

## Design
- Add local bridge GitHub workflow endpoints:
  - `GET /v1/local/github/workflow/runs` (aggregated recent remote runs)
  - `GET /v1/local/github/workflow/runner/runs` (local runner history)
  - `POST /v1/local/github/workflow/runner/run` (run allowlisted local workflow)
- Add shared workflow logic script:
  - `scripts/workflow-runner-intent-ui-ci.sh`
  - used by local bridge local-runner execution and GitHub CI workflow.
- Add a dedicated CI workflow:
  - `.github/workflows/intent-ui-workflow-runner-ci.yml`
  - executes the shared script on push/pull_request.
- Update Cloud panel:
  - fetch remote GitHub workflow runs from local bridge,
  - fetch/show local runner runs,
  - provide "Run Local CI" action and render resulting local run entries.

## Acceptance Criteria
1. Cloud panel displays recent remote GitHub workflow runs when GitHub token is connected.
2. Cloud panel displays local runner history entries.
3. Triggering local runner from Cloud panel creates a new local run entry with status.
4. GitHub CI workflow exists and uses the same shared script as local runner execution.
5. Frontend checks/build and relevant Cypress coverage pass.

## Rollout
- Ship local bridge endpoints, script, UI updates, and CI workflow together.

## Rollback
- Revert local bridge GitHub workflow endpoints, runner script, UI wiring, and CI workflow file introduced by this document.
