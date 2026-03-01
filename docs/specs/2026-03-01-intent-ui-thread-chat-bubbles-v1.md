# 2026-03-01 Intent UI Thread Chat Bubbles V1

## Goal
- Add Messenger-style floating chat bubbles from thread messages.
- Allow right-clicking a thread message to open a movable bubble on screen.

## Non-Goals
- Replacing the existing conversations drawer.
- Synchronizing bubble state across devices/accounts.
- Introducing backend storage or new APIs.

## Security and Constraints
- Keep bubble data local-only in browser storage.
- Avoid mutating message history when creating bubbles.
- Keep drag interactions bounded to viewport.

## Design
1. Add a thread-message right-click hook in Conversations panel.
2. Store floating bubble state in Workflow overlay with local persistence.
3. Render bubbles in a body portal so they can float above drawers/windows.
4. Implement pointer-drag movement and close controls for each bubble.

## Acceptance Criteria
1. Right-clicking a message in thread view opens a chat bubble.
2. Bubble can be dragged to a new screen position and remains visible.
3. Bubble close removes it from the floating layer.
4. Frontend validation commands pass when `bun` is available.

## Rollout
- Land right-click action + floating bubble layer + Cypress regression.

## Rollback
- Revert Conversations panel right-click hook and Workflow overlay bubble layer.
