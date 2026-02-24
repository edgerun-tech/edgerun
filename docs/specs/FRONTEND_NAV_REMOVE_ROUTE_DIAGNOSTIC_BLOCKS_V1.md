# Frontend Nav Remove Route Diagnostic Blocks V1

## Goal
- Remove the nav route diagnostics blocks that have been persistently unreliable and creating operator confusion.
- Keep the primary nav focused on navigation and core actions only.

## Non-goals
- No backend or websocket behavior changes.
- No changes to terminal drawer behavior, wallet flow, or route supervisor internals.
- No replacement diagnostics UI in this change.

## Security and constraints
- Keep implementation localized to `frontend/components/nav.tsx`.
- Preserve deterministic rendering and avoid adding dependencies.
- Do not bind any new service ports.

## Acceptance criteria
- Desktop `route-debug-rail` block is removed from nav.
- Mobile compact diagnostics chips are removed from nav.
- Nav layout remains functional on desktop and mobile.
- Cypress coverage is updated to validate absence of removed blocks.

## Rollout and rollback
- Rollout: ship nav simplification and validate frontend check/build plus updated diagnostics E2E spec.
- Rollback: restore previous `nav.tsx` diagnostics rail/chips if operational need returns.
