# 2026-03-01 Workflow Modal Gating v1

## Goal

Prevent the `Code Edit Workflow` modal from appearing during non-demo flows (for example, when opening Integrations drawers).

## Non-goals

- No redesign of workflow overlay visuals.
- No change to assistant request/streaming backend behavior.
- No change to drawer panel routing semantics.

## Security and Constraints

- Keep existing keyboard close behavior unchanged.
- Keep integration and drawer state transitions deterministic.
- Avoid introducing new runtime dependencies.

## Acceptance Criteria

1. Opening integrations no longer displays the `Code Edit Workflow` modal.
2. Explicit demo flow (`openWorkflowDemo`) still opens the modal.
3. Existing drawer navigation remains functional.
4. Frontend check/build and relevant Cypress coverage pass.

## Rollout

- Add explicit modal visibility state in workflow UI store.
- Gate modal rendering on the new flag.
- Verify integrations and conversation flows no longer trigger modal unexpectedly.

## Rollback

- Revert this slice to restore previous modal behavior tied directly to global workflow open state.
