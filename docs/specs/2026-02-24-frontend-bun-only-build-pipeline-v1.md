# Frontend Bun-Only Build Pipeline V1

Date: 2026-02-24
Status: Proposed and Implemented
Owner: Codex

## Goal
- Enforce deterministic frontend builds by executing frontend JS and HTML build scripts with `bun` only.

## Non-Goals
- No changes to frontend rendering, routing, or generated content semantics.
- No changes to CI topology beyond existing frontend checks.
- No dependency upgrades.

## Security and Constraint Requirements
- Must align with repository policy to use `bun` as the JavaScript runtime and package manager.
- Must not introduce `npm` or `pnpm` workflows.
- Must preserve existing output locations under `out/`.
- Must not require port `8080`.

## Acceptance Criteria
- `frontend/package.json` build script paths for client and HTML builds run through `bun` only (no `node` fallback).
- `cd frontend && bun run check` passes.
- `cd frontend && bun run build` passes.

## Rollout and Rollback
- Rollout: direct script update in `frontend/package.json`.
- Rollback: restore prior `node` fallback commands if an environment is discovered where `bun` execution is unavailable.
