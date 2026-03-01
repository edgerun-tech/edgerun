# 2026-03-01 OpenCode-Only Assistant Integration v1

## Goal

Replace the current split assistant provider surface (`codex` and `qwen`) with a single clean provider and integration path based on OpenCode CLI.

## Non-goals

- No change to browser-facing endpoint shape (`/api/assistant`).
- No change to node-manager local bridge endpoint path (`/v1/local/assistant`).
- No expansion of assistant provider options in this slice.

## Security and Constraints

- Fail closed when the OpenCode executor is unavailable or returns invalid output.
- Keep the local bridge loopback-only contract unchanged.
- Keep host port policy unchanged (do not use port `8080`).
- Keep runtime deterministic: one assistant provider path in frontend, backend, and compose runtime.

## Acceptance Criteria

1. Node-manager local assistant handler accepts only `opencode` provider semantics and executes OpenCode CLI.
2. Compose stack provides an OpenCode execution target container used by node-manager (`docker exec ...`).
3. Frontend assistant provider state and command parsing remove `codex`/`qwen` branches and use `opencode` only.
4. Integration catalog/lifecycle removes `qwen` and `codex_cli`; adds `opencode_cli` as the sole assistant integration.
5. Intent UI assistant gating continues to require connected device + linked assistant integration.
6. Cypress assistant/gating/session tests pass under OpenCode naming and payload expectations.
7. Docs and spec catalog reflect OpenCode-only assistant path.

## Rollout

- Add this spec and catalog entry.
- Land backend execution swap and compose wiring.
- Land frontend integration/provider cleanup.
- Update Cypress tests and docs.

## Rollback

- Revert the full change set to restore previous codex/qwen assistant behavior.
