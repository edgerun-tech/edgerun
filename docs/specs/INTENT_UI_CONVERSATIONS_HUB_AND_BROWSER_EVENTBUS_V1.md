# Intent UI Conversations Hub And Browser EventBus V1

## Goal
- Make conversations drawer self-explanatory when empty.
- Surface message-provider integrations (email, whatsapp, messenger, telegram) in conversations and settings with live status.
- Add customizable chat heads, emoji composition, and clipboard history usage in conversation composer.
- Route browser clipboard/conversation events through a frontend event bus with a WASM runtime hook.

## Non-goals
- No backend message-provider adapters in this slice.
- No remote peer event-bus transport protocol implementation in this slice.
- No replacement of existing Rust event bus yet.

## Security and constraints
- Fail-closed provider status: unavailable until integration prerequisites are satisfied.
- Browser event bus remains local-first and deterministic; remote sync is explicit and opt-in.
- WASM event bus integration must degrade safely to JS runtime if wasm artifact is unavailable.

## Acceptance criteria
1. Empty conversations state explains purpose and prompts integration connections.
2. Conversations settings popup lists message-provider integrations with availability status.
3. Users can customize chat head color/emoji per conversation.
4. Composer supports emoji insertion and sending local messages.
5. Clipboard history is persisted, visible in composer, and clipboard actions publish event-bus events.
6. Browser event bus runtime initializes and records runtime/clipboard/conversation events.
7. Frontend `check`, `build`, and targeted Cypress tests pass.

## Rollout / rollback
- Rollout:
  - `frontend/intent-ui/src/components/layout/WorkflowOverlay.jsx`
  - `frontend/intent-ui/src/stores/eventbus.js`
  - `frontend/intent-ui/src/stores/clipboard-history.js`
  - `frontend/intent-ui/src/stores/integrations.js`
  - `frontend/intent-ui/src/components/layout/IntegrationsPanel.jsx`
  - `frontend/intent-ui/src/App.jsx`
  - `frontend/cypress/e2e/intent-ui-conversations-hub.cy.js`
- Rollback: revert this change set to restore previous drawer behavior.
