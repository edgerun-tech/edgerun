# Intent UI Browser-Only Devices Panel V1

## Goal
- Make the Intent UI devices panel show real, currently connected runtime devices for web mode.
- In browser runtime mode, this must resolve to browser-only device presence with constrained capabilities.

## Non-goals
- No host discovery (`/api/host/status`) integration in this slice.
- No Wi-Fi/Bluetooth/LAN discovery scanning in this slice.
- No cluster/remote node enumeration in this slice.

## Security and constraints
- Fail-closed: do not claim host/network devices when they are not actively connected in this runtime.
- Capabilities shown for browser mode must be explicit and limited.
- Keep state deterministic and local-first.

## Acceptance criteria
1. Devices panel lists current browser device as the connected device in browser runtime.
2. Discovery scan controls are removed from the panel UI.
3. Browser device details include a limited capability map (no shell/filesystem host claims).
4. Existing frontend checks/build pass.

## Rollout / rollback
- Rollout: update `frontend/intent-ui/src/stores/devices.js` and `frontend/intent-ui/src/components/layout/WorkflowOverlay.jsx`.
- Rollback: revert this change set to restore prior scanner/host panel behavior.
