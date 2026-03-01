# 2026-03-01 Connected Devices IP Flag Widget v1

## Goal

Add a centered on-screen widget that shows devices currently connected to the page, including device IP and country flag indicators.

## Non-goals

- No geolocation provider integration in this slice.
- No device discovery protocol changes.
- No changes to existing devices panel workflows.

## Security and Constraints

- Keep widget read-only (display only).
- Do not transmit device data to external services.
- Keep rendering deterministic and local-state driven.

## Acceptance Criteria

1. Workflow overlay renders a centered widget listing online devices.
2. Each row shows device name, IP, and a country flag indicator.
3. Flag derivation supports explicit country code metadata with a safe fallback flag.
4. Widget updates as known device online state changes.
5. Frontend checks/build and Cypress coverage pass.

## Rollout

- Add known-devices memo and flag helper in workflow overlay.
- Render centered connected devices widget with stable test IDs.
- Add Cypress assertion for widget visibility and row rendering.

## Rollback

- Revert workflow overlay widget and Cypress test additions.
