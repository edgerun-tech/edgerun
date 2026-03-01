# OpenCode Storage Proposal Enforcement V1

## Goal

Ensure OpenCode-operated Codex sessions use the same storage-native, no-direct-edit behavior as MCP syscall submit mode.

## Non-Goals

- Rewriting OpenCode internals or model-side native tool behavior.
- Changing unrelated MCP integration lifecycles.
- Introducing a new storage contract.

## Security and Constraint Requirements

- Protected repository paths must route code edits through storage proposal submission only.
- MCP syscall must fail closed when storage submission cannot run.
- Defaults in compose should be safe for OpenCode-driven sessions and support bootstrap from recent storage state.
- No direct local file mutation should be the default inside protected roots.

## Acceptance Criteria

1. Compose service config for `mcp-syscall` sets storage proposal mode defaults suitable for OpenCode sessions.
2. Environment example includes the new variables and explains required values.
3. Node-manager compose docs describe how OpenCode + MCP syscall run in storage-proposal mode.
4. Compose files validate successfully with `docker compose config`.

## Rollout

- Enable storage proposal defaults in both `docker-compose.yml` and `docker-compose.node-manager.yml`.
- Add env knobs to `config/node-manager.compose.env.example` so operators can override per machine.
- Keep behavior explicit and reversible through environment changes.

## Rollback

- Set `MCP_CODE_EDIT_SUBMIT_ONLY=0` and disable protected roots.
- Remove/clear storage proposal env vars from compose env overrides.
- Restart compose stack.
