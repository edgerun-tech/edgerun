# 2026-03-01 IntentBar Calendar Popover Removal and List Consistency v1

## Goal
Simplify IntentBar by removing the inline mini calendar popover, and continue reusable list consistency upgrades across remaining high-traffic side panels.

## Non-goals
- No redesign of CallApp (deferred by operator).
- No backend/API changes.
- No changes to Calendar window functionality.

## Security and Constraints
- Preserve existing keyboard shortcuts and core command behavior.
- Keep calendar access available through window actions (Calendar panel remains primary).
- Keep list interaction handlers unchanged while swapping rendering primitives.
- Do not use port 8080.

## Design
- Remove mini calendar state, popover UI, and outside-click wiring from `IntentBar`.
- Keep clock/date display in IntentBar, but no embedded calendar popup.
- Continue list consistency by adopting shared virtualized primitives in:
  - `ConversationsPanel` side lists (threads/contacts/settings provider rows where practical),
  - `SettingsPanel` bookmark list,
  - `IntegrationsPanel` dynamic loading/diagnostic sublists.

## Acceptance Criteria
1. IntentBar no longer renders mini calendar popover.
2. Calendar panel remains accessible as standalone window.
3. Targeted side-list sections render via shared list primitive.
4. Frontend check/build and relevant Cypress tests pass.

## Rollout
- Ship IntentBar cleanup and list consistency updates together.

## Rollback
- Restore mini calendar popover blocks and revert list primitive adoptions.
