# 2026-03-01 Agent Virtualized Context Diff Events And Test Executors V1

Superseded on 2026-03-01 by `2026-03-01-agent-tooling-surface-removal-and-repo-coherence-v1.md`.

## Goal
- Move agent execution away from direct git worktrees to virtualized workspace views.
- Use `mcp-syscall` as the context/code-edit tool backend for agent context gathering.
- Make agents emit diff proposal events; repository state is composed from accepted diffs.
- Provide always-available test executor entrypoints for candidate workspaces.

## Non-Goals
- Full autonomous merge policy in this phase.
- Replacing existing human review process.
- Rewriting scheduler/worker runtimes in this change.

## Security And Constraints
- Agents do not receive `.git` metadata or direct branch access.
- Agent workspace is ephemeral under `out/agents/runs/`.
- Diff application is explicit and operator-controlled.
- Event payloads are append-only logs with deterministic metadata.
- No new service on port `8080`.

## Design
1. Virtualized workspace:
   - Build a repo snapshot (excluding `.git`, `target`, `out`, `node_modules`) for each run.
   - Agent modifies a writable copy of that snapshot.
2. MCP-backed context tools:
   - Add helper script to call `mcp-syscall` JSON-RPC `tools/call`.
   - Add context helper wrappers (`context_pack`, `code_symbols`, `code_find_refs`) usable by agent containers.
3. Diff events:
   - Generate unified patch from `base` -> `work`.
   - Emit event JSON file + append NDJSON stream under `out/agents/events`.
   - Optionally publish event to NATS core subject (best-effort, if `nc` available).
4. Accepted diff application:
   - Add script to apply a selected patch to repo root with `git apply`.
5. Test executors:
   - Add script to run validation profiles against any workspace path (`frontend`, `rust-event-bus`, `node-manager`, `quick`).

## Acceptance Criteria
1. Agent launcher produces run directories in `out/agents/runs/<run_id>/{base,work,events}`.
2. Launcher does not mount `.git` into agent container.
3. Diff events are emitted as JSON + NDJSON for each run.
4. Operator can apply accepted diff patch by run id.
5. Test executor runs selected profile on requested workspace.
6. Script syntax checks pass and docs updated.

## Rollout
- Start using new launcher for all parallel agent tasks.
- Keep old merge script temporarily for compatibility.
- Transition review/merge to accepted-diff flow.

## Rollback
- Revert script changes and continue with git-worktree based launcher.
