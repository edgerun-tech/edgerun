# 2026-03-01 Integrations Dedup Unified Flow v1

## Goal

Eliminate duplicated integration behavior definitions so worker/UI/store all follow one integration lifecycle and auth-mode source of truth.

## Non-goals

- No change to provider list.
- No change to existing local bridge endpoint contracts.
- No redesign of integrations dialog visuals.

## Security and Constraints

- Keep fail-closed verification behavior for missing credentials.
- Keep matrix bridge auto-secret flow unchanged.
- Keep MCP runtime start/stop contracts unchanged.

## Acceptance Criteria

1. Integration worker no longer ships a duplicated lifecycle class/catalog.
2. Integrations panel derives auth behavior from catalog (`authMethod` / `requiresToken`) instead of duplicated flags.
3. Runtime-backed integration detection in panel aligns with official bridge canonical IDs.
4. Frontend checks/build and targeted integration Cypress pass.

## Rollout

- Replace worker-local catalog/lifecycle with imports from shared integration catalog.
- Normalize panel helpers for token and OAuth flow checks.
- Validate integration regression specs.

## Rollback

- Revert worker and panel refactor slices in this change.
