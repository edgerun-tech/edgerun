# 2026-03-01 Intent UI Daly BMS Probe Resilience V2

## Goal and non-goals
- Goal:
  - Make Daly probe resilient across `FFF0/FFF1/FFF2`, swapped `FFF0` mappings, and `FFE0/FFE1`.
  - Eliminate false-negative `protocol unknown`/`packets 0` outcomes caused by narrow notify/write assumptions.
  - Keep probe bounded and safe while improving protocol discovery reliability.
  - Expose operator-facing probe transport stats (packet counts/sizes, notify/write coverage, frame attempts, elapsed time).
- Non-goals:
  - Full Daly protocol decoding and metrics extraction.
  - Any write operation that changes BMS state.

## Security and constraint requirements
- Continue Web Bluetooth secure-context + user gesture requirements.
- Fail closed on permission/service mismatch.
- Only send read/poll command frames; no control/setpoint writes.
- Keep probe time-bounded and deterministic.

## Acceptance criteria
1. Probe subscribes to all notify-capable Daly characteristics in active profile session.
2. Probe attempts writes on all writable Daly characteristics in active profile session.
3. A5 probe frames are checksum-correct and include common Daly read commands.
4. Probe diagnostics mention when no writable or no notify characteristics are present.
5. Probe response includes a structured `stats` object with transport-level metrics and the integrations panel surfaces these stats in Daly probe details.
6. Frontend validations pass:
- `cd frontend && bun run check`
- `cd frontend && bun run build`

## Rollout and rollback
- Rollout:
  - Update Daly BLE probe path in `frontend/intent-ui/src/lib/integrations/daly-bms-ble.js`.
- Rollback:
  - Revert Daly probe changes to previous single-char probe behavior.
