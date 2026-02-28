# Local-First Solana Removal Phase 2 V1

## Goal
- Remove remaining chain/Solana operational surface from active worker and systemd deployment paths.
- Ensure local-first defaults and validation scripts do not require Solana RPC/wallet/program configuration.

## Non-goals
- No full rewrite of scheduler internal chain stub code in this phase.
- No changes to isolated `program/` Solana workspace in this phase.

## Security and constraints
- Preserve existing scheduler and worker control-plane security semantics.
- Preserve deterministic config validation behavior.
- Avoid introducing permissive config parsing regressions.

## Acceptance criteria
1. `edgerun-worker` no longer reads or logs `EDGERUN_WORKER_CHAIN_SUBMIT_ENABLED`.
2. Systemd env validation no longer requires `EDGERUN_CHAIN_RPC_URL`, `EDGERUN_CHAIN_WALLET`, or `EDGERUN_CHAIN_PROGRAM_ID`.
3. Worker common env templates/profiles no longer include `EDGERUN_WORKER_CHAIN_SUBMIT_ENABLED`.
4. Scheduler env templates/profiles no longer include chain RPC/wallet/program variables.
5. Required checks pass for touched scope.

## Rollout
- Land as a local-first operational cleanup phase after phase-1 node-manager decoupling.

## Rollback
- Revert this change set to restore prior chain-oriented env requirements.
