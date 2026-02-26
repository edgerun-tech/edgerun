# CI-GROUND-UP-REBUILD V1

Date: 2026-02-26
Status: Implemented
Owner: Codex

## Goal
- Replace the current CI workflow with a deterministic, fast baseline that is reliable on self-hosted runners and removes out-of-spec jobs.
- Keep required signal quality (format/lint/build/test sanity) while reducing wall-clock and flake risk.

## Non-Goals
- Redesign of standalone specialized workflows (containerd runtime smoke/soak, release, deploy, provenance).
- Deep test strategy rewrite for runtime fuzz/soak/security suites.
- Benchmark/timing strategy design (deferred to a separate spec).

## Security and Constraint Requirements
- Use bun for JS workflows; do not introduce npm/pnpm.
- Keep frontend root canonical at frontend/.
- Keep generated/build/temp artifacts under out/.
- Use real chain/RPC sources for chain-derived views.
- Do not bind services to port 8080.
- Enforce least-privilege default `permissions` in workflow jobs.

## Acceptance Criteria
- `.github/workflows/ci.yml` is rebuilt with a clean structure, explicit concurrency cancellation, and changed-scope gating.
- Broken/out-of-spec jobs are removed from CI: `build-timing-main`, `program-localnet`.
- CI summary remains functional and only references existing jobs.
- Rust job executes stable deterministic checks without known self-hosted sccache path flake.
- Frontend job runs `bun`-based quality + production build when frontend scope changes.
- Workflow passes static sanity checks (`check-workflow-references`, `check-workflow-drift`) locally.

## Rollout
- Replace `ci.yml` in one atomic change.
- Validate with repo workflow guard scripts.
- Trigger manual `workflow_dispatch` after merge for runtime evidence.

## Rollback
- Revert `.github/workflows/ci.yml` to previous commit if unexpected regressions appear.
- Re-run workflow guard scripts to confirm rollback integrity.
