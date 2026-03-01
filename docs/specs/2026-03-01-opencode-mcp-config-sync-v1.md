# 2026-03-01 OpenCode MCP Config Sync v1

## Goal

Make node-manager MCP start/stop lifecycle changes deterministically update OpenCode MCP configuration so assistant runs can immediately use enabled local MCP integrations.

## Non-goals

- No changes to local bridge MCP endpoint shapes.
- No changes to MCP container image selection logic.
- No expansion of OpenCode assistant providers.

## Security and Constraints

- Preserve user-managed OpenCode MCP entries; only manage EdgeRun-owned entries.
- Do not store integration secrets in OpenCode config.
- Keep behavior fail-closed on write/sync errors during MCP lifecycle operations.
- Keep host port policy unchanged (do not use port `8080`).

## Design

- Replace node-manager MCP config sync placeholder with real config sync logic.
- Read OpenCode config from the running OpenCode CLI container.
- Parse existing config, preserve non-EdgeRun MCP entries, and update only `edgerun-*` MCP entries.
- Add/remove `edgerun-github` MCP server entry based on `edgerun-mcp-github` container running state.
- Write merged config back to OpenCode CLI container path.

## Acceptance Criteria

1. Starting a supported MCP integration updates OpenCode config with managed `edgerun-*` MCP entry.
2. Stopping that integration removes the managed entry while preserving user-defined MCP entries.
3. Managed config sync errors continue to surface through MCP lifecycle endpoint errors.
4. Unit tests validate merge behavior for managed-entry insert/remove and user-entry preservation.

## Rollout

- Land spec and catalog update.
- Land node-manager implementation and tests.
- Validate with workspace check/test plus frontend required checks.

## Rollback

- Revert node-manager MCP config sync implementation and this spec update.
