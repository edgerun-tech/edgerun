# Agent Workflow Script Resilience V1

## Goal
Harden agent and executor shell workflows for event-native operation with deterministic failure signaling.

## Non-goals
- No protocol/schema changes for core events.
- No scheduler/storage crate behavior changes.
- No replacement of Codex container execution model.

## Security and Constraints
- Event-critical actions must fail clearly when event publish fails.
- Script input handling must avoid shell-quoting injection risks.
- Diff metadata must report user-code line changes accurately.
- Keep implementation Bash-only and lightweight.

## Acceptance Criteria
1. `scripts/executors/nats-pub.sh` has dependency checks and retry behavior.
2. `scripts/agents/emit-diff-events.sh` uses canonical publisher and reports accurate added/deleted line counts (excluding diff headers).
3. `scripts/agents/apply-accepted-diff.sh` publishes accepted events fail-closed and uses robust dirty-tree checks.
4. `scripts/agents/launch-agent.sh` no longer interpolates prompt text directly into shell command.
5. Updated docs describe operational behavior where changed.

## Rollout
- Immediate replacement; no migration needed.

## Rollback
- Revert this patchset and return to prior script implementations.
