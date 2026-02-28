# Local-First Solana Removal Phase 1 V1

## Goal
- Remove Solana runtime dependency pressure from core local-first node startup path.
- Ensure `edgerun-node-manager` no longer requires local Solana validator lifecycle (`agave-validator`, `solana-keygen`) for boot-to-workload operation.
- Remove unused workspace-level `solana-*` Rust dependencies that are not required by active workspace crates.

## Non-goals
- No removal of the isolated `program/` Solana workspace in this phase.
- No full scheduler architectural split in this phase.
- No protocol redesign of all control-plane APIs in this phase.

## Security and constraints
- Keep TPM-required identity and existing fail-closed boot policy verification intact.
- Keep `api_base` lock behavior unchanged.
- Do not introduce insecure fallback paths.
- Keep changes deterministic and compile-safe.

## Acceptance criteria
1. `edgerun-node-manager` `run` path no longer spawns or health-checks local Solana validator binaries.
2. `edgerun-node-manager` no longer attempts stake init API flow that is Solana lifecycle specific.
3. `edgerun-node-manager` no longer injects `EDGERUN_CHAIN_RPC_URL` into worker environment.
4. Workspace root `Cargo.toml` no longer contains `solana-client`, `solana-sdk`, or `solana-system-interface` in `[workspace.dependencies]`.
5. Required checks pass for touched crates.

## Rollout
- Land as phase-1 local-first decoupling.
- Follow up with scheduler/contract cleanup phases to remove remaining chain scaffolding.

## Rollback
- Revert this change set to restore prior node-manager validator/stake lifecycle behavior and workspace dependency set.
