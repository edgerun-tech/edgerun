# 2026-03-01 Node Manager Local Bridge Modularization Phase 2 v1

## Goal
Reduce `crates/edgerun-node-manager/src/main.rs` integration coupling by extracting Cloudflare and Docker local-bridge helper logic into focused modules without changing runtime behavior.

## Non-goals
- No endpoint contract changes.
- No route path changes.
- No auth/permission behavior changes.

## Security and Constraint Requirements
- Preserve existing token validation thresholds and fail-closed behavior.
- Preserve local bridge loopback assumptions.
- Keep Docker container action allowlist unchanged.
- Do not use port 8080.

## Design
- Add `crates/edgerun-node-manager/src/local_bridge/cloudflare.rs` for Cloudflare token/API helper logic used by handlers.
- Add `crates/edgerun-node-manager/src/local_bridge/docker_local.rs` for Docker container selector/action validation helpers.
- Keep handler functions in `main.rs` for this phase; only shared helper internals move.

## Acceptance Criteria
1. Node manager compiles with helper logic sourced from modules.
2. Existing Cloudflare and Docker local-bridge handler behavior remains unchanged.
3. No API route/path regressions.

## Rollout
- Introduce modules and import wiring in one change set.

## Rollback
- Revert module files and restore helpers inline in `main.rs`.
