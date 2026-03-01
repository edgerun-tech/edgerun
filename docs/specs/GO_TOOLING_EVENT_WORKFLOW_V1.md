# Go Tooling Event Workflow V1

Superseded on 2026-03-01 by `2026-03-01-agent-tooling-surface-removal-and-repo-coherence-v1.md`.

## Goal
Replace ad-hoc shell implementations for event and diff workflow operations with a typed Go CLI.

## Non-goals
- Full replacement of every existing shell script in one change.
- Rewriting Docker/swarm orchestration flow in this step.
- Changing event contracts used by UI/executors.

## Security and Constraints
- Event operations remain fail-closed: publish failures return non-zero.
- Payload and line-count semantics remain deterministic.
- Local apply remains explicit (`--apply`) and guarded by clean working tree checks.
- Keep dependencies minimal (std lib only).

## Acceptance Criteria
1. Add `tooling/cmd/edgertool` with commands:
   - `nats-pub`
   - `agent-diff-proposed`
   - `agent-diff-accept`
2. Existing shell entrypoints (`scripts/executors/nats-pub.sh`, `scripts/agents/emit-diff-events.sh`, `scripts/agents/apply-accepted-diff.sh`) delegate to Go CLI.
3. Existing event schema fields remain compatible.
4. Validation commands complete successfully.

## Rollout
- Introduce Go tooling and use wrapper scripts for compatibility.
- Later phase can delete wrappers and invoke Go binary directly everywhere.

## Rollback
- Restore prior script implementations from git history.
