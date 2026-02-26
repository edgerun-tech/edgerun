# Serde JSON Complete Removal v1 (2026-02-26)

## Goal
Eliminate `serde_json` usage from the Edgerun workspace, including direct dependencies and transitive pulls from workspace-level feature configuration.

## Non-Goals
- No API contract changes for existing JSON HTTP/WebSocket endpoints.
- No protocol migration away from JSON payloads for external clients.
- No behavior changes to scheduling, node bootstrap, or worker assignment logic.

## Security and Constraints
- Preserve current auth/signature semantics and payload canonicalization behavior.
- Keep deterministic runtime behavior; only codec and wiring changes are allowed.
- Maintain repository constraints: Bun-only JS workflows, no new frontend roots, no port 8080 usage.

## Acceptance Criteria
1. No `serde_json` crate in workspace dependency declarations (`Cargo.toml` files).
2. No direct `serde_json::*` source usage remains in workspace Rust code.
3. Workspace `reqwest` config does not enable the `json` feature.
4. All prior JSON request/response paths continue to serialize/deserialize correctly using replacement codec plumbing.
5. Rust validation passes for changed scope (`cargo check`, `cargo clippy`, `cargo test`).
6. Frontend baseline checks pass per repo standard (`cd frontend && bun run check`, `cd frontend && bun run build`).

## Rollout
- Ship as an internal dependency/runtime refactor with no API shape changes.
- Monitor control-plane and node-manager JSON endpoints for parse/encode regressions.

## Rollback
- Revert commit(s) introducing codec replacement and dependency feature changes.
- Restore `serde_json` and `reqwest/json` feature if regressions are detected.
