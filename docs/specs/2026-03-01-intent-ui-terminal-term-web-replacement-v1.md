# 2026-03-01 Intent UI Terminal Term-Web Replacement V1

## Goal and non-goals
- Goal:
  - Replace the mock/browser-only Intent UI terminal panel with a real term-web surface rendered in an iframe.
  - Keep terminal target selection explicit so operators can point Intent UI at local or remote term-server endpoints.
  - Preserve IntentBar "Open Terminal" workflow while making the terminal window production-meaningful.
- Non-goals:
  - Add routed `route://` WebRTC terminal transport inside Intent UI in this change.
  - Implement parent-to-iframe command injection protocol for term-web in this change.

## Security and constraint requirements
- Only allow `http://` and `https://` terminal targets for iframe embedding.
- Persist only terminal endpoint text locally; do not persist command history or credentials.
- Use existing `/term` entrypoint contract and avoid introducing new backend runtime dependencies.

## Acceptance criteria
1. Opening the Intent UI `Terminal` window renders term-web iframe surface instead of mock command simulation.
2. Terminal panel includes a configurable target input and connect action, with endpoint persistence across reloads.
3. Target normalization appends `/term` when needed and sets a stable `sid` query param for pane identity.
4. Invalid targets show a clear inline validation state instead of rendering a broken iframe.
5. Existing forwarded terminal input events remain visible to the user as queued suggestions (no silent drop).
6. Add/update Cypress test coverage validating Intent UI terminal iframe rendering for non-route HTTP targets.
7. Frontend validation passes:
   - `cd frontend && bun run check`
   - `cd frontend && bun run build`

## Rollout and rollback
- Rollout:
  - Replace Intent UI terminal panel implementation with term-web iframe host panel.
  - Add terminal endpoint persistence key and input/connect controls.
  - Add Cypress regression for iframe rendering in `/intent-ui/`.
- Rollback:
  - Restore previous mock terminal panel component and remove new term-web test.
