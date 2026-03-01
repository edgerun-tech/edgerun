# Intent UI TPM Vault Credentials v1

## Goal
- Move Intent UI credential and integration-token persistence to the TPM-backed node-manager vault surface.
- Remove dependency on deprecated profile-encryption secret flow for integration credentials.

## Non-goals
- Building a remote/cloud credential escrow service.
- Replacing the existing credentials UI/UX in this phase.
- Migrating unrelated profile/session features outside credential persistence.

## Security and Constraint Requirements
- Credential persistence must be served by local bridge endpoints under `/v1/local/credentials/*`.
- Backing store must be TPM-backed node-manager configuration storage.
- Integration token writes must persist to TPM-backed vault and remain available across page reloads.
- Frontend must not require profile-encryption context to read/write integration credentials.
- Bun-based frontend workflows and existing local bridge architecture remain unchanged.

## Acceptance Criteria
1. Node manager exposes local credential endpoints for status/list/store/delete and integration token lookup.
2. Credentials panel uses local bridge credential endpoints instead of `/api/credentials/*`.
3. Integration store vault mirroring uses local bridge credential endpoints.
4. GitHub/Tailscale integration token behavior works without profile-secret context.
5. `cd frontend && bun run check` passes.
6. `cd frontend && bun run build` passes.
7. Targeted Cypress integration specs for integration persistence/ownership pass.

## Rollout / Rollback
- Rollout:
  - Add node-manager local credential handlers and route wiring.
  - Repoint frontend credential/integration code to `/v1/local/credentials/*`.
  - Validate with frontend checks/build/Cypress.
- Rollback:
  - Revert node-manager credential route additions.
  - Revert frontend credential endpoint rewiring.
  - Restore previous profile-secret dependent integration persistence path.
