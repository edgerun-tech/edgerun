# SPDX-License-Identifier: LicenseRef-Edgerun-Proprietary

# No `serde_json` in Internal Code (v1 Migration)

## Status
In progress

## Goal
Eliminate `serde_json` usage from backend/internal Rust code paths and replace internal contracts with protobuf/typed binary formats.

## Non-goals
- Removing JSON from frontend presentation concerns.
- One-shot protocol redesign across every module in one commit.

## Constraints
- Deterministic internal wire formats.
- Explicit error handling through centralized error types.
- No hidden compatibility readers.

## Acceptance Criteria
1. `rg -n "serde_json" crates edgerun-apps -g '!Cargo.lock'` returns no runtime/internal usages in targeted slices.
2. Updated slices compile and pass lint/tests.
3. Internal protocol boundaries use protobuf bytes or typed structs, not `serde_json::Value`.

## Slice Plan
1. Storage (manifest + event bus + tests) — done.
2. Small protocol crates:
   - `edgerun-transport-ws`
   - `edgerun-discovery`
   - `edgerun-observability`
3. Runtime/CLI helper layers.
4. Worker and scheduler control plane (largest migration).
5. Terminal subsystems (`term-server`, `term-web`, `term-native`) if still in internal scope.

## Current Progress (2026-02-23)
- `edgerun-worker`: `serde_json` removed fully (runtime + tests), queue/control WS tests now use typed binary messages.
- `edgerun-scheduler`: internal persistence and internal event payload encoding moved from JSON to binary (`bincode`) for:
  - state snapshot (`state.bin`)
  - route shared state (`route-state.bin`)
  - trust policy (`trust-policy.bin`)
  - attestation policy (`attestation-policy.bin`)
  - chain progress sink event payload
- Remaining `serde_json` usage in `edgerun-scheduler` is isolated to browser-facing WebRTC signaling message exchange.

## Rollout
- Keep migration incremental per crate with compile/lint/test proof for each slice.
- Remove `serde_json` dependency from each crate immediately after last usage is removed.

## Rollback
- Revert affected crate-level migration commit(s).
- No mixed dual-format compatibility readers are introduced in this proposal.
