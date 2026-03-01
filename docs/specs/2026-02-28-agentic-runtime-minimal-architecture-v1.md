# 2026-02-28 Agentic Runtime Minimal Architecture V1

## Goal
- Simplify agent automation to one seamless event-driven path: task -> shard -> diff -> gate -> apply.
- Remove redundant wrappers and legacy merge paths so data flow is explicit and deterministic.
- Keep `edgerun-storage` as source of truth for proposal/gate lifecycle state.

## Non-Goals
- Building a fully autonomous self-merging planner in this phase.
- Replacing existing frontend with a new UI framework.
- Introducing additional queue/event technologies beyond current stack.

## Security And Constraints
- Fail-closed: if proposal submit/gate/apply fails, workflow exits non-zero.
- Lead/planner behavior is read-only; only gate-approved diffs can be applied.
- One canonical write path to repo state (`accepted -> apply`).
- No fallback-only modes and no polling-based truth surfaces.
- No new service on port `8080`.

## Design
1. Canonical command path:
   - `scripts/agents/run-task.sh <AGENT_ID> <PROMPT...>` as single operator entrypoint.
   - It enables storage autosubmit + gate dry-run and delegates to `edgertool agent-launch`.
2. Storage-native lifecycle:
   - `agent-launch` emits diff proposal event, then optional storage submission and gate dry-run.
   - Proposal IDs default to run ID for deterministic traceability.
3. Diff stability hardening:
   - Diff generation must ignore symlink target availability (no dereference failures in virtual views).
4. Noise reduction:
   - Remove obsolete wrappers that duplicate Go tooling behavior and are not part of canonical flow.

## Acceptance Criteria
1. A real Codex container run produces run artifacts and a storage proposal without manual extra commands.
2. Gatekeeper dry-run executes automatically in same run when autosubmit is enabled.
3. Diff emission is robust with symlink entries in workspace snapshots.
4. Canonical docs and scripts point to a single operator entrypoint.
5. Removed wrappers are not referenced by Makefile/docs entrypoints.

## Rollout
- Introduce `run-task.sh` and update docs/Make targets.
- Keep core direct commands available (`launch-agent`, `storage-proposal-*`) for debugging.
- Use two real assignment runs to verify end-to-end flow.

## Rollback
- Restore removed wrappers from git history.
- Disable autosubmit env and run manual proposal/gate commands.
