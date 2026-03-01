# 2026-03-01 Beeper Sync Backend Flow v1

## Goal

Run Beeper chat synchronization through backend local bridge endpoints instead of browser-only integration worker logic.
Also serve Beeper media/avatar assets through backend proxy routes so browser UI can render profile pictures and media previews reliably.

## Non-goals

- No full message-body sync/import in this slice.
- No replacement of conversation composer/send pipeline.
- No background daemon scheduling beyond existing UI-triggered refresh.

## Security and Constraints

- Keep Beeper access token request-scoped.
- Keep loopback-only access through local bridge.
- Preserve fail-closed behavior when Beeper Desktop API is offline.

## Acceptance Criteria

1. Node-manager exposes a local Beeper chats endpoint.
2. Caddy routes `/api/beeper/chats` to local bridge.
3. Node-manager exposes a local Beeper chat-messages endpoint.
4. Conversations source loader reads Beeper chats and per-chat recent messages from backend endpoints when Beeper is connected.
5. Beeper conversation rows show human-readable preview text (not only message type labels).
6. Conversation list can surface profile pictures when provided by Beeper participants data.
7. Media messages include usable attachment indicators/URLs in thread content.
8. Beeper `file://` and `mxc://` media URIs are resolved through backend proxy endpoints and render in browser UI.
9. Sending a message in a Beeper conversation uses backend `/api/beeper/send` and fails visibly when delivery fails.
10. UI no longer labels non-email providers as matrix bridge-specific.
11. Required checks pass.

## Rollout

- Add `/v1/local/beeper/chats` endpoint.
- Add Caddy Beeper route rewrite.
- Update conversation source loader to consume Beeper chat list.

## Rollback

- Revert Beeper chats endpoint, routing, and source-loader updates.
