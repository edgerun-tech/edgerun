# Intent UI Device Connect Dialog V1

## Goal
- Move the device onboarding/connect flow in the Devices panel behind an explicit `Add device` action.
- Open the existing connect flow in a modal dialog when `Add device` is clicked.

## Non-goals
- Changing relay/domain reservation logic.
- Changing pairing token semantics.
- Changing node-manager install/connect commands.

## Security and Constraints
- Keep existing fail-closed behavior for missing domain/token/profile key inputs.
- Preserve existing local storage persistence keys for domain/token/pairing code/profile key.
- Do not expose onboarding controls outside the authenticated UI surface already in place.

## Acceptance Criteria
1. Devices panel shows an `Add device` button.
2. Device connect flow is hidden by default and visible only inside a dialog opened by `Add device`.
3. Existing Linux connect flow UI and behavior remain unchanged once dialog is open.
4. Existing Cypress device-connect flow is updated to open dialog first and passes.

## Rollout / Rollback
- Rollout: additive UI behavior change in `WorkflowOverlay` and Cypress update.
- Rollback: revert dialog/button changes to restore inline connect block.
