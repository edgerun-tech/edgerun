# Frontend Economics Models and Screens V1

## Goal
- Align economics-facing frontend screens with the real deterministic protocol model (scheduler + on-chain program).
- Remove placeholder economics blocks that imply unsupported/fake flows.
- Prevent run-job submission defaults from failing deterministic escrow minimum checks.

## Non-goals
- No protocol-level formula changes in scheduler or on-chain program.
- No new billing, custody, or token acquisition integrations.
- No changes to non-economics routes beyond shared utility wiring.

## Security and constraints
- Economics formulas rendered in frontend must match runtime formulas:
  - Scheduler minimum escrow formula (`required_instruction_escrow_lamports`).
  - Program committee tier boundaries and required lock formula.
- Keep values SOL/lamports explicit; avoid ambiguous units.
- Keep runtime behavior deterministic and avoid fake pool/mock APR style placeholders.
- Keep implementation in `frontend/` and use `bun` workflows.

## Acceptance criteria
1. `/token/` renders concrete economics content: formulas, tier thresholds, and payout model references, without placeholder pool cards.
2. Run page default escrow is set to a deterministic-safe baseline (not an unrealistically tiny value that is below scheduler minimum in default topology).
3. Shared frontend economics utility exists and is used by economics-facing screen(s) to reduce drift from protocol formulas.
4. Cypress coverage verifies economics route content and absence of placeholder pool artifacts.
5. `cd frontend && bun run check` and `cd frontend && bun run build` pass.

## Rollout and rollback
- Rollout: ship frontend utility + token screen + run default escrow hardening + Cypress assertions in one release.
- Rollback: revert this change set; runtime protocol remains unchanged.
