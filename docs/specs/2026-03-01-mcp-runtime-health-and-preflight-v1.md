# 2026-03-01 MCP Runtime Health and Preflight v1

## Goal

Improve integration runtime clarity by treating only `running` container state as healthy and surfacing MCP preflight info (image mapping + token env) before link.

## Non-goals

- No changes to container images themselves.
- No automatic image pulling or remediation.
- No changes to integration credential persistence.

## Security and Constraints

- Keep local bridge enforcement and node-id checks intact.
- Keep token values out of logs and preflight responses.
- Keep existing MCP start/stop contracts backward compatible.

## Acceptance Criteria

1. Runtime health in integrations store does not treat `restarting` as healthy.
2. Node-manager exposes MCP preflight endpoint with image mapping and token env metadata.
3. Integrations stepper displays runtime preflight status for runtime-backed integrations.
4. Frontend and node-manager validations pass.

## Rollout

- Add `/v1/local/mcp/integration/preflight` endpoint.
- Consume preflight in integrations store/panel.
- Tighten runtime health checks to `status === running`.

## Rollback

- Revert runtime health and preflight endpoint/UI changes.
