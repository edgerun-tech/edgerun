# Frontend Run Price Precalculation V1

## Goal
- Let users pre-calculate deterministic run price before submitting jobs.
- Expose compute envelope controls used by scheduler pricing checks so price expectations are explicit.

## Non-goals
- No scheduler pricing formula changes.
- No on-chain fee policy changes.

## Security and constraints
- Use the same deterministic escrow formula as scheduler (`required_instruction_escrow_lamports`).
- Submission must fail client-side when escrow is below computed deterministic minimum.
- Keep values unit-explicit (lamports + SOL).

## Acceptance criteria
1. `/run` exposes editable pricing inputs for compute envelope at least `max_instructions` and `escrow_lamports`.
2. `/run` shows live deterministic minimum escrow based on current compute envelope.
3. Submission payload uses user-selected compute envelope values.
4. Client-side validation rejects submits when escrow is below computed minimum.
5. Cypress coverage asserts estimator visibility and guardrail behavior.
6. `cd frontend && bun run check`, `cd frontend && bun run build`, `cd frontend && bun run e2e:core` pass.

## Rollout and rollback
- Rollout: ship UI, validation, and tests together.
- Rollback: revert this changeset.
