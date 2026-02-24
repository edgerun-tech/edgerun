# 2026-02-24 Mock Surface Elimination V1

## Goal
Eliminate production-facing mock/placeholder execution paths that can appear successful while returning fabricated or stale data.

## Non-goals
- Implement full missing backends (for example, a complete log provider integration).
- Change public API schemas unless required for correctness.
- Re-architect scheduler quorum/finalization behavior.

## Security and Constraint Requirements
- On-chain derived scheduler flows must not silently run without real chain context.
- Attestation verification code must not present itself as a stub.
- Storage MSI must not replay tail events using a no-op implementation.
- Cloud OS file/terminal/log tools must not return synthetic mock data as real results.
- Frontend landing page must not present unverifiable hardcoded KPI claims.

## Acceptance Criteria
1. Scheduler startup requires chain context by default and does not advertise placeholder mode.
2. `job_create` fails with explicit service errors when chain transaction artifacts cannot be built.
3. Attestation claim validation helper is non-stub naming and remains policy-bound.
4. MSI read path does not apply no-op tail replay; reads with pending tails are treated as cache misses.
5. Cloud OS terminal worker returns explicit capability errors when WebContainer is unavailable (no mock command/filesystem execution).
6. Cloud OS main-thread MCP handler removes mock file/search/read/log fallbacks and returns explicit error responses.
7. Browser MCP client handles these error responses without hanging on timeout.
8. Landing hero removes hardcoded KPI metric block.

## Rollout and Rollback
- Rollout: deploy as strict mode defaults with explicit error messages for unavailable capabilities.
- Rollback: revert this change set to restore prior permissive mock behavior.
- Operational note: environments missing required chain or browser capabilities will now fail fast instead of returning synthetic success payloads.
