# 2026-03-01 Contacts Merge Multi-Channel Import v1

## Goal

Merge contacts across imported history and live integrations so contacts preserve all known communication channels and thread links.

## Non-goals

- No remote address-book writeback.
- No fuzzy/AI identity resolution beyond deterministic normalization.
- No destructive deduplication of raw thread history.

## Security and Constraints

- Keep all contact aggregation local-only.
- Preserve source provenance (Google/email/Beeper/imported) in merged contact metadata.
- Keep fail-closed behavior when imported dataset is unavailable.

## Acceptance Criteria

1. Contacts list includes contacts derived from imported thread history in addition to existing sources.
2. Merged contacts preserve all discovered channels (for example email + beeper + imported).
3. Contact selection prefers linked known thread IDs when available.
4. Frontend checks/build and targeted Cypress coverage pass.

## Rollout

- Expose imported thread participants/channels from backend endpoint.
- Build merged contacts map in conversation source loader.
- Render channel indicators in Contacts tab and route contact selection to known threads.

## Rollback

- Revert imported-contact merge logic and Contacts tab metadata rendering.
