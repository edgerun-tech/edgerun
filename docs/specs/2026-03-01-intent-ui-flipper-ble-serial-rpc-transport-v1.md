# 2026-03-01 Intent UI Flipper BLE Serial RPC Transport V1

## Goal and non-goals
- Goal:
  - Replace the current generic GATT probe with Flipper-specific BLE serial transport behavior in `intent-ui`.
  - Verify control readiness using Flipper serial service characteristics, flow-control notifications, and protobuf-delimited RPC exchange.
  - Keep user-visible verification and probe outcomes in the integration dialog flow.
- Non-goals:
  - Full implementation of every Flipper RPC domain and command.
  - Persistent background Flipper daemon in browser tabs.
  - Firmware-specific hacks outside published BLE/protobuf contracts.

## Security and constraint requirements
- Web Bluetooth must run only in secure context (HTTPS) and fail closed otherwise.
- Device selection must remain explicit user gesture-driven (`requestDevice`).
- Pairing/auth remains OS/browser BLE security; app does not collect/store BLE passkeys.
- No polling loops for transport state; use notifications + bounded wait loops.
- Keep implementation dependency-light (no new heavy protobuf runtime).

## Acceptance criteria
1. Flipper verify step checks:
- Serial service + required characteristics are present.
- TX indication and flow-control notifications can be subscribed.
- Ping RPC round-trip succeeds over protobuf-delimited framing.
2. Flipper probe step returns:
- BLE identity (`deviceId`, `deviceName`).
- Serial transport diagnostics (`flowBudget`, characteristic/service list).
- RPC diagnostics (`ping` result and sampled device info key/values).
- Normalized summary fields (`model`, `hardware`, `firmware`, `serial`) derived from common Flipper RPC key variants.
3. Integration dialog surfaces verification/probe errors without leaking to unrelated panels.
4. Frontend validation passes:
- `cd frontend && bun run check`
- `cd frontend && bun run build`
- Updated Flipper Cypress coverage passes for new transport behavior.

## Rollout and rollback
- Rollout:
  - Ship additive transport implementation in `frontend/intent-ui/src/lib/integrations/flipper-ble.js`.
  - Update Flipper integration Cypress spec for serial transport mocking.
- Rollback:
  - Revert transport file + Cypress spec + this spec doc to restore prior generic probe behavior.
