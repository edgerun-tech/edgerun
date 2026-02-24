# Dashboard Chain Metrics Deterministic Cypress Coverage V1

## Goal
Enable deterministic end-to-end verification for dashboard chain metrics without relying on external Solana RPC availability.

## Non-Goals
- No runtime behavior changes for production users.
- No replacement of real RPC usage in app runtime.
- No new dashboard widgets or visual redesign.

## Security and Constraints
- Mocking must be opt-in and only active in Cypress runtime.
- Default behavior remains real Solana RPC calls against configured cluster endpoints.
- Do not bind any service to port 8080.
- Keep frontend runtime dependency footprint unchanged.

## Proposed Change
- Add a Cypress-only RPC response hook in `frontend/lib/solana-rpc-ws.ts` for HTTP-style JSON-RPC methods.
- Keep hook disabled unless explicit global flag is set in test runtime.
- Add Cypress E2E test for dashboard chain metrics that injects deterministic RPC responses and asserts user-visible values.

## Acceptance Criteria
1. Dashboard page can render deterministic chain metrics in Cypress via explicit test-only mock setup.
2. Existing dashboard control-plane test remains passing.
3. `cd frontend && bun run check` passes.
4. `cd frontend && bun run build` passes.
5. Cypress specs for dashboard control-plane and dashboard chain metrics pass.

## Rollout and Rollback
- Rollout: ship test-only hook and new test in same change.
- Rollback: remove Cypress hook and delete new spec if it introduces maintenance overhead.

## Alignment Notes
- Aligns with AGENTS.md requirement for verifiable, non-mocked production data paths while allowing deterministic test harnesses.
