# 2026-03-01 Reusable Virtual Animated List v1

## Goal
Introduce a reusable list primitive with virtual scrolling and optional row animations, then adopt it in high-volume Intent UI surfaces for consistent behavior.

## Non-goals
- No global style redesign.
- No third-party virtualization dependency introduction.
- No changes to backend APIs.

## Security and Constraints
- Keep rendering local-only and deterministic.
- Avoid excessive runtime allocations for large lists.
- Preserve existing content interactions (click/context menu/selection).

## Design
- Upgrade `useVirtualList` to support viewport-based windowing:
  - container-ref driven scroll/height tracking,
  - explicit `scrollTop`/`viewportHeight` injection,
  - top/bottom pad calculations.
- Add reusable `VirtualAnimatedList` component with two layouts:
  - `stack` (pads + normal flow rows)
  - `absolute` (translated rows in fixed-height container)
- Adopt in:
  - `GmailPanel` (absolute layout)
  - `ConversationsPanel` (stack layout, row animation enabled)
  - `CloudflarePanel` inventory lists (stack layout, row animation enabled)
  - `CloudPanel` provider resource rows (stack layout, row animation enabled)
  - `LauncherGuidePanel` startup/guide task lists (stack layout, row animation enabled)
  - `IntegrationsPanel` known BLE device selectors (stack layout, row animation enabled)
  - `IntentBar` filter/help/history list blocks (stack layout, row animation enabled)

## Acceptance Criteria
1. Gmail, conversation thread, Cloudflare inventory, Cloud provider resource rows, launcher task lists, integration device selectors, and IntentBar list blocks use the same reusable list primitive.
2. List rendering remains stable while scrolling and loading additional rows.
3. Frontend check/build pass.

## Rollout
- Ship hook + component + panel adoptions together.

## Rollback
- Revert hook/component and restore per-panel list rendering.
