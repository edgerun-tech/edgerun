# Worker Policy Refresh 404 Fallback V1

## Goal
- Stop repeated worker polling/noise when scheduler policy endpoint is not exposed in a given deployment profile.
- Keep worker assignment/heartbeat flow running without requiring policy-info HTTP route availability.

## Non-Goals
- Do not remove policy verifier support.
- Do not change scheduler control WebSocket protocol.
- Do not change event-bus/storage initialization behavior.

## Security And Constraints
- Worker must retain configured/default policy verifiers when refresh is disabled.
- Only disable periodic policy refresh for explicit missing-endpoint condition (`404` from `/v1/policy/info`).
- Do not disable refresh for authorization failures or other transient network errors.

## Acceptance Criteria
1. Worker logs a single explicit warning and disables further periodic policy refresh after receiving `404` from policy info endpoint.
2. Worker continues heartbeat and assignment polling loop after refresh disablement.
3. Existing policy refresh behavior remains unchanged for successful responses and non-404 failures.
4. Worker crate check/clippy/tests pass.

## Rollout
- Land worker fallback behavior as a local-safe default requiring no environment changes.
- Verify on user stack logs that policy-info 404 spam stops while worker services stay active.

## Rollback
- Revert this patch to restore prior periodic polling behavior.
