# Agent Diff Event-First Acceptance V1

Superseded on 2026-03-01 by `2026-03-01-agent-tooling-surface-removal-and-repo-coherence-v1.md`.

## Goal
Ensure accepted agent/code-edit diffs are propagated through the event bus (NATS) first, with no implicit local repository mutation.

## Non-goals
- Replacing the patch format or agent virtualized workspace model.
- Building a full distributed patch-application worker in this change.
- Altering executor build/test trigger subject contracts.

## Security and Constraints
- Default behavior must be non-mutating for local repo state.
- Accepted diffs must be emitted as explicit events containing provenance (`run_id`, `agent_id`, `patch_sha256`).
- Local mutation is allowed only through an explicit operator action (`--apply`).
- Keep existing NATS publishing primitive (`scripts/executors/nats-pub.sh`).

## Acceptance Criteria
1. `scripts/agents/apply-accepted-diff.sh <RUN_DIR|PATCH_PATH>` publishes an `agent.diff.accepted` event and does not run `git apply`.
2. `scripts/agents/apply-accepted-diff.sh --apply <RUN_DIR|PATCH_PATH>` preserves current explicit apply behavior and still publishes acceptance + code update events.
3. Documentation reflects event-first default and explicit apply mode.
4. `scripts/agents/merge-agent.sh <RUN_DIR|PATCH_PATH>` uses event-first mode by default.

## Rollout
- Immediate switch to event-first default.
- Existing operators can still use explicit apply flag.

## Rollback
- Revert this spec's implementation commit to restore previous default local-apply behavior.
