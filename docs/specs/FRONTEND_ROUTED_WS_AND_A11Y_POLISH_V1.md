# FRONTEND_ROUTED_WS_AND_A11Y_POLISH_V1

## Goal and non-goals
- Goal: harden frontend production behavior by improving terminal accessibility and ensuring routed terminal connectivity remains scheduler-websocket mediated.
- Goal: remove legacy direct-socket branches in routed terminal UI that bypass scheduler-mediated routing.
- Goal: improve keyboard and assistive-tech usability for terminal drawer/device controls.
- Non-goal: redesign terminal architecture or replace the existing WebRTC routed transport model.
- Non-goal: change backend scheduler protocols.

## Security and constraint requirements
- Routed terminal session traffic initiation must not depend on legacy direct browser sockets.
- Scheduler websocket signaling must recover from transient socket close events (reconnect).
- Interactive controls must expose accessible names and keep predictable button semantics.
- Changes must stay within `frontend/` and preserve deterministic static build outputs.

## Acceptance criteria
- Routed terminal pane no longer attempts a direct websocket path; it uses routed supervisor transport only.
- WebRTC signaling client reconnects automatically after scheduler ws disconnect/close.
- Terminal/device input controls have explicit `aria-label` or equivalent persistent labels.
- Resize handle is keyboard operable for users who cannot use pointer drag.
- Control panel action buttons explicitly declare `type="button"`.
- Cypress coverage includes websocket routing behavior and a11y smoke assertions for terminal controls.

## Rollout and rollback
- Rollout:
  1. Ship frontend with updated routed terminal behavior and a11y fixes.
  2. Run frontend Cypress suite in CI.
- Rollback:
  1. Revert this spec’s implementation commit.
  2. Restore prior routed pane/signaling behavior if regressions are observed.
