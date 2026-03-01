# 2026-03-01 Intent UI Conversations Scroll Stability V1

## Goal
- Make chat thread scrolling deterministic in the Conversations panel.
- Prevent jumpy position changes when older messages are incrementally loaded.
- Keep bottom-follow behavior predictable when switching between threads.

## Non-Goals
- Reworking conversation data sources.
- Changing message rendering style or visual design.
- Introducing new runtime dependencies.

## Security and Constraints
- Preserve existing local-only storage behavior for conversation messages.
- Keep rendering deterministic and avoid duplicate message insertion.
- Avoid destructive state resets outside the active conversation view.

## Design
1. Add explicit conversation-scroll container test hook for deterministic E2E targeting.
2. When near top and loading older messages, preserve viewport anchor by compensating `scrollTop` with post-render `scrollHeight` delta.
3. Reset thread scroll-follow state when active conversation changes so thread switches consistently start in follow-bottom mode.

## Acceptance Criteria
1. Scrolling up to load older messages does not snap the user to a different part of the thread unexpectedly.
2. Thread switch does not inherit stale non-follow scroll state from the previous thread.
3. A Cypress test covers anchored scroll behavior during incremental load.
4. Frontend validation commands pass once `bun` is available.

## Rollout
- Land panel scroll anchor fix and thread-switch reset.
- Add Cypress regression coverage.

## Rollback
- Revert changes in Conversations panel and workflow overlay scroll state effects.
- Remove the new Cypress regression spec.
