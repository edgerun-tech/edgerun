# 2026-03-01 Super+V Conversation Composer Launch v1

## Goal

Make `Super+V` open the conversation composer with emoji and clipboard affordances, mirroring system-level paste/emoji launch expectations inside Intent UI.

## Non-goals

- No OS-level global hotkey registration outside the browser tab.
- No replacement of native paste behavior inside focused text inputs.
- No changes to conversation message semantics.

## Security and Constraints

- Preserve existing clipboard permission boundaries (`navigator.clipboard` may fail or be unavailable).
- Do not intercept `Super+V` while typing in input/textarea/contenteditable fields.
- Keep behavior deterministic and local to Intent UI runtime.

## Acceptance Criteria

1. Pressing `Super+V` in Intent UI opens the right drawer on `conversations` if not already open.
2. The conversation composer is shown and emoji palette is expanded.
3. If clipboard read is available, latest clipboard text is pushed into local clipboard history (best-effort).
4. Existing conversation composer Cypress flow continues to pass.

## Rollout

- Land spec + status catalog update.
- Implement hotkey dispatch + workflow overlay listener.
- Add Cypress regression test for `Super+V` launcher behavior.

## Rollback

- Revert hotkey dispatch/listener and associated Cypress/spec updates.
