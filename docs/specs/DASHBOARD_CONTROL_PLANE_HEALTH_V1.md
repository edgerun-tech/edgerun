<!-- SPDX-License-Identifier: Apache-2.0 -->

# DASHBOARD_CONTROL_PLANE_HEALTH_V1

## Goal and Non-Goals
- Goal: augment `/dashboard` with live scheduler control-plane health fields so operators can quickly tell whether control WS is reachable.
- Goal: show control base, reachability, latency, and last-check timestamp.
- Non-goal: replacing existing chain metric widgets or adding non-RPC telemetry backends.

## Security and Constraints
- Reuse existing frontend runtime patterns and control-base selection logic.
- Keep build deterministic and dependency-light.
- Keep data read-only and user-visible.

## Acceptance Criteria
1. `/dashboard` shows a control-plane health panel with `data-control-field` markers.
2. Runtime probes control WS using existing route-control utilities and updates panel fields.
3. Probe failures are reflected as explicit `unreachable`/error text, not silent loading.
4. `cd frontend && bun run check` and `cd frontend && bun run build` pass.
5. Cypress test validates control panel renders and fields update from loading state.

## Rollout and Rollback
- Rollout: deploy updated static frontend output.
- Rollback: revert `frontend/app/dashboard/page.tsx`, `frontend/src/runtime/chain-status.ts`, and `frontend/cypress/e2e/dashboard-control-plane.cy.js`.
