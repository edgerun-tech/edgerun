# 2026-03-01 Codex Assistant Direct Local Backend v1

## Goal
Route Intent UI assistant requests directly to node-manager local backend (`/v1/local/assistant`) instead of the standalone codex bridge shim on port `7788`.

## Non-goals
- No change to assistant request/response payload contract used by the frontend (`/api/assistant` remains the browser endpoint).
- No change to onboarding/integration gating semantics in Intent UI.
- No change to model/provider policy beyond current node-manager assistant behavior.

## Security / Constraint Requirements
- Keep fail-closed behavior when backend is unavailable.
- Keep host port constraints (do not introduce new `8080` listeners).
- Keep deterministic local-stack routing through Caddy + node-manager.
- Preserve compatibility with existing `edgerun-codex-cli` container execution path used by node-manager.

## Acceptance Criteria
1. Caddy `/api/assistant` requests are rewritten/proxied to node-manager `/v1/local/assistant` on `127.0.0.1:7777`.
2. `codex-cli` service no longer runs a separate assistant HTTP bridge process on `127.0.0.1:7788`.
3. `edgerun-codex-cli` remains available for `docker exec ... codex ...` invocation by node-manager.
4. Stack documentation reflects direct local-backend assistant routing.

## Rollout / Rollback
- Rollout:
  - Update Caddy assistant routing to node-manager local backend.
  - Simplify `codex-cli` container command to provide an always-running Codex CLI execution target.
  - Update compose docs accordingly.
- Rollback:
  - Revert this change set to restore bridge-on-`7788` behavior.
