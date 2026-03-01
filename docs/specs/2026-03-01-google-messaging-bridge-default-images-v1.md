# 2026-03-01 Google Messaging Bridge Default Images v1

## Goal

Enable zero-extra-config startup for Google messaging bridge integrations by providing deterministic default MCP image mappings in node-manager for:
- `google_messages`
- `gvoice`
- `googlechat`

## Non-goals

- No changes to integration token storage flow.
- No changes to OpenCode managed MCP stdio config (still only applies to supported managed entries).
- No changes to bridge-specific runtime environment contracts beyond image selection defaults.

## Security and Constraints

- Keep fail-closed behavior when container launch fails.
- Preserve explicit environment override precedence (`EDGERUN_MCP_*_IMAGE`).
- Keep host-only local bridge contract unchanged.
- Keep port policy unchanged (do not use port `8080`).

## Acceptance Criteria

1. Node-manager `mcp_image_for` returns non-empty defaults for `google_messages`, `gvoice`, and `googlechat`.
2. Existing `EDGERUN_MCP_*_IMAGE` overrides continue to take precedence over defaults.
3. Compose env example documents default images for Google messaging bridge integrations.
4. Unit tests validate default image selection and override precedence.

## Rollout

- Land spec + catalog update.
- Land node-manager image mapping + tests.
- Update compose documentation comments.

## Rollback

- Revert this slice to restore env-only image mapping behavior for these integrations.
