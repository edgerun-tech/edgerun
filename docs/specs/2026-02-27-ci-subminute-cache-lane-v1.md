<!-- SPDX-License-Identifier: Apache-2.0 -->

# CI_SUBMINUTE_CACHE_LANE_V1

Date: 2026-02-27
Status: Proposed
Owner: Codex

## Goal
- Reduce typical CI wall-clock toward sub-minute for scoped PR changes by removing avoidable compile overhead and improving cache reuse across self-hosted machines.
- Keep deterministic behavior and preserve stronger full-quality checks on protected paths (`main`, tags, manual dispatch).

## Non-Goals
- Replacing the entire CI workflow graph.
- Changing release workflows.
- Introducing new package managers, new app roots, or runtime mock data.

## Security and Constraint Requirements
- Keep Bun as the only JS runtime/package manager in CI.
- Preserve canonical frontend root at `frontend/`.
- Preserve least-privilege default workflow permissions.
- Keep CI deterministic with explicit cache keys and controlled conditional execution.

## Acceptance Criteria
- CI no longer compiles `edgerun-cli` solely for SPDX checks; SPDX validation uses `scripts/spdx-check.sh`.
- Frontend dependency install is skipped when lockfile-keyed cache is a hit.
- Rust compilation cache is shared/stabilized for cross-machine self-hosted runner reuse.
- Rust clippy remains enforced for `main`, tags, and manual runs; PRs keep fast rust correctness checks (`fmt` + `check`) to reduce latency.
- Workflow guard scripts continue to pass.

## Rollout
- Update `.github/workflows/ci.yml` in a single change.
- Validate workflow policy scripts locally.
- Measure subsequent GitHub Action run durations and iterate cache keys/scopes if needed.

## Rollback
- Revert `.github/workflows/ci.yml` to previous revision.
- Re-run workflow policy scripts to confirm rollback integrity.
