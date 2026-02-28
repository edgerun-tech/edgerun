# 2026-02-28 Integrations MCP Runtime and GitHub Direct Verify V1

## Goal
Improve integration iteration speed and reliability by:
1) removing frontend dependency on `/api/github/*` for GitHub PAT verification,
2) running integration adapters as local MCP containers via node-manager local bridge,
3) preventing unintended bootstrap/workflow overlays from opening due generic event relay.

## Non-Goals
- Full multi-node MCP scheduling.
- Production secret-management redesign.
- Replacing existing integration UI stepper flow.

## Security and Constraints
- Fail-closed local bridge: MCP lifecycle endpoints only accept local node id.
- Tokens passed only over loopback local bridge and stored encrypted in profile as existing flow.
- Keep loopback-only bridge binding.
- No port 8080 usage.

## Acceptance Criteria
1. Node-manager exposes `/v1/local/mcp/integration/start`, `/stop`, `/status`.
2. Integration connect for token-based providers starts MCP container best-effort; disconnect stops it.
3. GitHub PAT verification uses `https://api.github.com/user` directly and no `/api/github/*` dependency.
4. Random bootstrap gate opening from eventbus relay is prevented.

## Rollout
- Deploy updated node-manager and frontend together.
- Validate connect/disconnect with GitHub PAT and inspect MCP status endpoint.

## Rollback
- Revert this implementation commit.
