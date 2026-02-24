# CI/CD Required Checks, Workflow Hygiene, and Deploy Smoke V1

## Goal
- Make branch protection requirements explicit and reviewable in-repo.
- Detect dead workflow references and missing workflow files continuously.
- Add post-deploy smoke verification signals so deploy workflows report route health pass/fail.

## Non-goals
- No change to core build/test/deploy commands.
- No external monitoring vendor integration.
- No secret exposure in reports.

## Security and constraints
- Required checks policy must be declarative and deterministic.
- Workflow reference audit must fail on missing targets rather than silently skipping.
- Deploy smoke checks must avoid secret values and use public/non-sensitive endpoints only.
- Bun-first and existing CI guardrails remain unchanged.

## Acceptance criteria
1. A committed required-checks policy exists with exact checks to enforce on `main`.
2. A workflow-hygiene job runs on schedule and manual dispatch, failing on:
   - missing workflow files referenced by local workflow runner lists,
   - missing workflow names referenced by `workflow_run` triggers,
   - invalid required-check policy references.
3. `wiki-sync` references an existing source workflow for `workflow_run` triggers.
4. Deployment workflows include smoke-check summary status with explicit pass/fail/skipped.

## Rollout and rollback
- Rollout: merge policy + scripts + workflow updates; validate with local checks and dry-runs.
- Rollback: remove hygiene workflow/scripts and smoke checks, restore previous trigger/policy behavior.
