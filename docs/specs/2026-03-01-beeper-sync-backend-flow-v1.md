# 2026-03-01 Beeper Sync Backend Flow v1

## Goal

Run Beeper chat synchronization through backend local bridge endpoints instead of browser-only integration worker logic.

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
3. Conversations source loader reads Beeper chat list from backend endpoint when Beeper is connected.
4. UI no longer labels non-email providers as matrix bridge-specific.
5. Required checks pass.

## Rollout

- Add `/v1/local/beeper/chats` endpoint.
- Add Caddy Beeper route rewrite.
- Update conversation source loader to consume Beeper chat list.

## Rollback

- Revert Beeper chats endpoint, routing, and source-loader updates.
