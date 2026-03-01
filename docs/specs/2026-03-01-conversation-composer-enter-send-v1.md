# 2026-03-01 Conversation Composer Enter Send v1

## Goal

Make conversation composer keyboard behavior intuitive: `Enter` sends message, `Shift+Enter` inserts newline.

## Non-goals

- No change to send button behavior.
- No change to message transport/backends.
- No markdown/editor formatting features.

## Security and Constraints

- Keep message send fail-closed via existing send pipeline.
- Preserve multiline composition using Shift+Enter.
- Avoid intercepting IME composition enter events.

## Acceptance Criteria

1. Pressing Enter in composer sends the draft message.
2. Pressing Shift+Enter inserts a newline and does not send.
3. Existing button-based send still works.
4. Frontend checks/build and targeted Cypress coverage pass.

## Rollout

- Add textarea keydown handling in ConversationsPanel.
- Add Cypress test for Enter/Shift+Enter behavior.

## Rollback

- Revert keydown handler and test additions.
