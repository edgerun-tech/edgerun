# 2026-03-01 Intent UI Matrix Bridge Integrations v1

## Goal
Add first-class Intent UI integrations for Matrix bridge-backed messaging providers needed now:
- WhatsApp (`mautrix/whatsapp`)
- Telegram (`mautrix/telegram`)
- Google Messages (`mautrix/gmessages`)
- Meta (`mautrix/meta`)

## Non-goals
- No bridge container orchestration/deployment in this slice.
- No transport protocol unification backend in this slice.
- No removal of existing messaging integrations already used by current UI.

## Security / Constraint Requirements
- Treat bridge credentials as user-owned secrets via existing integration token flow.
- Keep deterministic integration availability behavior in both main thread and worker catalogs.
- Keep single-messages UI/provider discovery fail-closed on disconnected integrations.

## Acceptance Criteria
1. Integration catalog includes `google_messages` and `meta` with messaging capabilities.
2. Integrations worker catalog mirrors the same provider ids/capabilities.
3. Integrations panel exposes `google_messages` and `meta` for connect/verify/save.
4. Conversation/message-provider discovery includes `google_messages` and `meta` alongside existing messaging providers.

## Rollout / Rollback
- Rollout: add provider metadata + lifecycle entries in catalog/worker/UI lists.
- Rollback: revert this change set to remove Matrix bridge provider entries.
