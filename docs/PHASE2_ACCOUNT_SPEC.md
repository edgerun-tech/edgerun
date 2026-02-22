<!-- SPDX-License-Identifier: Apache-2.0 -->
# Phase 2 Account Spec (Practical)

Date verified: 2026-02-22  
Reference: `Whitepaper-phase-2.mdx`

## Scope

This document pins the practical Phase 2 account model implemented in
`program/programs/edgerun_program/src/lib.rs`.

## Account Layouts

All account sizes below are `8-byte discriminator + INIT_SPACE`.

| Account | PDA seeds | INIT_SPACE | Total bytes | Rent-exempt (devnet, 2026-02-22) |
| --- | --- | ---: | ---: | ---: |
| `GlobalConfig` | `[b"config"]` | 179 | 187 | 0.0021924 SOL |
| `WorkerStake` | `[b"worker_stake", worker]` | 49 | 57 | 0.0012876 SOL |
| `Job` | `[b"job", job_id]` | 545 | 553 | 0.00473976 SOL |
| `JobResult` | `[b"job_result", job_id, worker]` | 168 | 176 | 0.00211584 SOL |
| `OutputAvailability` | `[b"output", job_id]` | 232 | 240 | 0.00256128 SOL |

## Phase 2 Fields Confirmed

`GlobalConfig` includes Phase 2 control surface:
- `randomness_authority`
- `da_window_slots`
- `non_response_slash_lamports`
- `committee_tiering_enabled`
- `max_committee_size`

`Job` includes Phase 2 deterministic settlement fields:
- `committee_size`, `quorum` (stored per job)
- `assigned_workers: [Pubkey; 9]`, `assigned_count`
- `seed`, `seed_set`
- `winning_output_hash`, `quorum_reached_slot`, `da_deadline_slot`

## Tiering Rules (Frozen)

At `post_job`, committee tier is selected from escrow:

- Tier0: `< 0.1 SOL` -> committee `3`, quorum `2`
- Tier1: `0.1-1 SOL` -> committee `5`, quorum `3`
- Tier2: `1-10 SOL` -> committee `7`, quorum `5`
- Tier3: `>= 10 SOL` -> committee `9`, quorum `6`

Program stores committee/quorum on the job so later instructions remain deterministic.

## Deployment Economics Snapshot

Current program artifact:
- `program/target/deploy/edgerun.so`: `521,880` bytes
- Rent-exempt minimum (devnet): `3.63317568 SOL`

Operational guidance:
- Upgradeable deploys around this artifact size are typically in the ~`3.6-4.0 SOL` budget range.
- `--final` (non-upgradeable at deploy time) removes future upgradeability but does not make rent near-zero; binary footprint still dominates cost.

## CLI Support

Use the project CLI to regenerate a live analysis from RPC:

```bash
cargo run -p edgerun-cli -- program analyze-accounts --cluster devnet
```

Deploy flow (with optional non-upgradeable finalization):

```bash
cargo run -p edgerun-cli -- program deploy --cluster devnet --final
```
