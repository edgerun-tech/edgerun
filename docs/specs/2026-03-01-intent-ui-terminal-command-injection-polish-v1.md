# 2026-03-01 Intent UI Terminal Command Injection Polish V1

## Goal and non-goals
- Goal:
  - Deliver a polished Intent UI terminal flow where commands sent from IntentBar are injected directly into term-web when the iframe session is available.
  - Preserve a visible, user-auditable fallback queue so forwarded commands are never silently dropped.
  - Keep Intent UI terminal behavior compatible with existing term-web session transport (`raw` and `mux`).
- Non-goals:
  - Introduce a new backend terminal RPC path.
  - Add routed `route://` device selection UX to Intent UI terminal in this change.

## Security and constraint requirements
- Accept injected command payloads only via explicit postMessage envelope shape and text fields.
- Keep iframe target restricted to validated `http(s)` endpoints in Intent UI.
- Do not persist command payload history to storage; keep forwarded command status in-memory only.

## Acceptance criteria
1. When IntentBar sends terminal input, Intent UI terminal attempts to postMessage command payloads into the active term-web iframe, including stable `commandId` and `sid` metadata.
2. term-web listens for command-injection postMessage events and writes received input to the active terminal session, appending newline only when `execute=true`.
3. term-web publishes explicit parent postMessage events for `ready` and `stdin-ack` so host UI can reflect delivery progress deterministically.
4. If iframe is not ready, commands remain visible in queue with pending status and are retried on iframe load/ready.
5. UI surfaces per-command delivery status (`pending`, `posted`, `sent`, `error`) so operators can tell what happened.
6. Add/update Cypress coverage for Intent UI terminal flow to verify command forwarding does not regress iframe rendering workflow.
7. Validation passes:
   - `cd frontend && bun run check`
   - `cd frontend && bun run build`
   - targeted Cypress specs for Intent UI terminal and terminal drawer term-web coverage

## Rollout and rollback
- Rollout:
  - Add postMessage command bridge in Intent UI terminal panel.
  - Add postMessage listener in term-web wasm frontend.
  - Update Cypress coverage for polished command-forwarding path.
- Rollback:
  - Remove postMessage injection/listener logic and revert to queue-only manual paste behavior.
