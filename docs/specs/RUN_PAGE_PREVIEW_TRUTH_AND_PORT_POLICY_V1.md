<!-- SPDX-License-Identifier: Apache-2.0 -->

# RUN_PAGE_CONTROL_PLANE_SUBMISSION_AND_PORT_POLICY_V1

## Goal
- Remove confirmed policy defects and wire `/run` to an actual control-plane submission path.
- Do not default any flow to port `8080`.
- Replace local preview-only submission simulation with a real `job.create` control-plane request.

## Non-Goals
- Replace scheduler control-plane protocol or add wallet signing on `/run`.
- Change scheduler/worker attestation architecture in this patch.

## Security and Constraints
- Preserve existing validation gates for runtime ID, scheduler URL, and safety acknowledgment.
- Keep frontend rooted in `frontend/` and bun-based workflows only.
- Keep changes deterministic and minimal-risk.
- Keep browser submission payload bounded with conservative default limits/escrow.

## Acceptance Criteria
1. CLI `tailscale bridge` default terminal port is not `8080`.
2. `/run` calls control plane `job.create` through `SchedulerControlWsClient` on submit.
3. Submission success path displays returned scheduler job identifier.
4. Submission failure path surfaces scheduler/control-plane error text.
5. Cypress run-job UX assertions reflect real submission wiring while remaining deterministic.
6. Frontend checks/build pass and relevant E2E test passes.

## Rollout and Rollback
- Rollout: deploy with updated frontend static build and CLI binary.
- Rollback: revert `crates/edgerun-cli/src/main.rs`, `frontend/app/run/page.tsx`, `frontend/lib/scheduler-control-ws.ts`, and `frontend/cypress/e2e/run-job-ux.cy.js`.
