# 2026-03-01 Intent UI Daly BMS Web Bluetooth V1

## Goal and non-goals
- Goal:
  - Add a new `daly_bms` integration in Intent UI using browser Web Bluetooth.
  - Reuse `BluetoothIntegration` base lifecycle for deterministic behavior.
  - Provide user flow for select, verify, and probe with structured diagnostics.
- Non-goals:
  - Full Daly protocol implementation in this phase.
  - Configuration writes/actuation against BMS.

## Security and constraint requirements
- Require secure context and explicit device selection gesture.
- Fail closed on missing BLE permissions/API.
- Keep probe read-oriented and bounded by timeouts.

## Acceptance criteria
1. `daly_bms` provider appears in integrations panel.
2. User can select a BLE device and verify connectivity.
3. Probe returns structured diagnostics:
- service/characteristic availability,
- optional packet samples from notification stream,
- detected protocol hints (`D2`/`A5`) when present.
4. Frontend validation passes:
- `cd frontend && bun run check`
- `cd frontend && bun run build`

## Rollout and rollback
- Rollout:
  - Add Daly transport module + catalog/store/panel wiring.
- Rollback:
  - Revert Daly-specific files/branches and provider registration.
