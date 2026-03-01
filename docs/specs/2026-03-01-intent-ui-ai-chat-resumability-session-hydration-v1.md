# 2026-03-01 Intent UI AI Chat Resumability Session Hydration V1

## Goal
- Make AI chat resumability deterministic across reloads by loading all stored sessions.
- Ensure session switching always uses stored message history for the selected session.

## Non-Goals
- Changing assistant provider transport behavior.
- Introducing server-side session persistence in this change.
- Redesigning conversation drawer UI.

## Security and Constraints
- Keep session persistence local-only in browser storage.
- Preserve compatibility with current and legacy storage keys.
- Do not add dependencies.

## Design
1. Load and merge session history from both current and legacy keys.
2. Load and merge session message maps from both current and legacy keys.
3. Derive missing history entries from stored message maps so session-only message stores are discoverable.
4. Improve session switching to match by sessionId, threadId, numeric index, and load matching messages.

## Acceptance Criteria
1. Workflow state hydrates all discoverable stored sessions, not just one source key.
2. Selecting/swapping session restores the correct message history in chat.
3. Session switching remains available via numeric index and id prefix.
4. Frontend validation commands pass when `bun` is available.

## Rollout
- Land storage merge + hydration/session-switch improvements.
- Add Cypress coverage for multi-source session hydration and switching.

## Rollback
- Revert workflow session hydration/switch logic to prior single-source behavior.
