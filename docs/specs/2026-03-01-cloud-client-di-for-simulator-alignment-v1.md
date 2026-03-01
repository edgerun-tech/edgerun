# 2026-03-01 Cloud Client DI for Simulator Alignment v1

## Goal
Adopt dependency-injected cloud clients so production UI and simulated test flows execute the same client logic with different transport/state adapters.

## Non-goals
- No UI redesign.
- No endpoint contract changes.
- No node-manager route changes.

## Security and Constraints
- Keep token lookup behavior unchanged.
- Do not log tokens.
- Keep local bridge network boundary unchanged.
- Do not use port 8080.

## Design
- Add `frontend/intent-ui/src/lib/cloud/cloud-clients.js` with injected transport clients:
  - local bridge transport (`getJson`, `postJson`)
  - docker client
  - cloudflare client
  - github workflow client
- Update `cloud-panel-providers.js` to consume injected real clients instead of direct URL assembly/fetch.
- Update `CloudPanel.jsx` orchestration to build clients once per fetch cycle and reuse them for both read and local run trigger actions.

## Acceptance Criteria
1. Cloud panel provider loaders use injected clients rather than raw fetch path strings.
2. Local workflow run action uses injected GitHub workflow client.
3. Frontend checks/build and targeted cloud panel Cypress specs pass.

## Rollout
- Ship DI client layer and Cloud panel/provider wiring together.

## Rollback
- Revert client module and provider/panel wiring to direct fetch behavior.
