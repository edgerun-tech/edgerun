# Architecture (Scaffold)

## Components

1. Solana program (`program/`) for escrow, stake, assignment, result settlement, and slashing.
2. Deterministic runtime (`crates/edgerun-runtime`) to execute constrained `wasm32` bundles.
3. Worker daemon (`crates/edgerun-worker`) for heartbeat, assignment polling, execution, and submit.
4. Scheduler (`crates/edgerun-scheduler`) for committee selection and lifecycle orchestration.
5. Storage (`crates/edgerun-storage`) for content-addressed immutable bundle delivery.

## Frozen Protocol Decisions

- Hash: BLAKE3-256
- Signature: Ed25519
- Committee size: 3
- Quorum: 2
- Currency: SOL (MVP)
- On-chain result payload: `output_hash` only
