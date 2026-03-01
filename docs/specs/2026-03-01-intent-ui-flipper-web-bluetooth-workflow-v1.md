# Intent UI Flipper Web Bluetooth + Workflow v1

## Goal
Enable connecting a Flipper device from Intent UI using Web Bluetooth, provide one-click workflow bootstrap after successful verification/linking, and support a real probe action that reads device metadata over BLE GATT.

## Non-goals
- Full Flipper protocol implementation.
- Background BLE scanning without user gesture.
- Native bridge/device-agent integration for Flipper.
- Firmware flashing or file transfer.

## Security and Constraint Requirements
- Browser BLE access must require explicit user gesture (`navigator.bluetooth.requestDevice`).
- No fallback mode: if Web Bluetooth is unavailable, verification fails with explicit error.
- Integration state remains event-driven via integration intents/events.
- Keep Bun-based frontend workflows only.

## Design
- Add `flipper` integration definition in catalog/worker.
- Add Flipper provider UI in Integrations panel:
  - Step 2 includes `Select Flipper` button (Web Bluetooth chooser),
  - stores selected device id/name in dialog state.
  - show clear security UX note that BLE pairing passkey/PIN is handled by browser+OS and the hardware device, not by the app.
  - show previously granted Web Bluetooth devices (`navigator.bluetooth.getDevices`) with quick-select.
  - include one-click `Select + Verify` path.
- Add store-side Web Bluetooth verify path (main thread only) for `flipper` so worker cannot falsely verify.
- Add store-side probe path for `flipper`:
  - Connect GATT,
  - Read `battery_service` level where available,
  - Read `device_information` characteristics where available,
  - Emit probe event onto eventbus (`ui.integration.flipper.probed`).
- On successful Flipper verify/link, expose workflow bootstrap action (`Create Flipper Workflow`) that opens a prepared workflow session.

## Acceptance Criteria
1. Flipper provider appears in integrations icon list.
2. Clicking `Select Flipper` triggers `navigator.bluetooth.requestDevice`.
3. Verify step fails with clear error if Web Bluetooth unavailable.
4. Verify step succeeds with selected/mock Flipper device and allows linking.
5. Success step offers workflow bootstrap action.
6. Probe action reads at least one real GATT field when available and reports deterministic structured result in UI/eventbus.
7. Flipper setup UX clearly communicates pairing model and exposes previously granted devices for quick reconnect.

## Rollout
- Frontend-only feature; no backend changes required.

## Rollback
- Revert changes in integration catalog, store verify path, panel UI, and workflow helper.
