# 2026-03-01 Intent UI Bluetooth Integration Base V1

## Goal and non-goals
- Goal:
  - Introduce a reusable Bluetooth integration base class for browser-side BLE integrations.
  - Make Bluetooth integrations predictable with shared selection, connect, reconnect, and permission recovery behavior.
  - Migrate Flipper integration to use the new base as the first reference implementation.
- Non-goals:
  - Implement every future Bluetooth device protocol now (Daly, sensors, etc.).
  - Add new backend APIs.
  - Change existing integration state model (`IntegrationLifecycle`) in this phase.

## Security and constraint requirements
- Fail closed when Web Bluetooth is unavailable or insecure context is used.
- Keep user-gesture device selection as the only path for new device grants.
- Recover permission failures only by explicit re-select flow (`requestDevice`), never by hidden fallback.
- Keep deterministic, bounded recovery behavior for disconnects (single reconnect path).

## Acceptance criteria
1. New `BluetoothIntegration` base exists under `frontend/intent-ui/src/lib/integrations/`.
2. Base provides:
- secure-context validation,
- known-device lookup,
- device selection,
- gatt connect/reconnect,
- permission/disconnect error classification,
- service acquisition with recovery.
3. Flipper integration consumes this base and retains current verify/probe behavior.
4. Frontend checks pass:
- `cd frontend && bun run check`
- `cd frontend && bun run build`

## Rollout and rollback
- Rollout:
  - Add base class and wire Flipper to it in one change.
- Rollback:
  - Revert base class and Flipper migration commits to return to ad-hoc BLE handling.
