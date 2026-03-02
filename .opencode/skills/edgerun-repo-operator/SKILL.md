---
name: edgerun-repo-operator
description: Execute implementation, debugging, and delivery tasks in edgerun with spec-first planning and command-backed evidence
license: Apache-2.0
compatibility: opencode
metadata:
  audience: agents
  repository: edgerun
---

## Mission

- Operate with production-grade discipline and verifiable evidence.
- Keep changes deterministic, minimal, and aligned with `AGENTS.md` constraints.
- Execute end-to-end autonomously unless blocked by external constraints.

## Required workflow

1. Read `AGENTS.md` and task-relevant docs before non-trivial edits.
2. Add/update a spec in `docs/specs/` before non-trivial behavior changes.
3. Implement the smallest viable deterministic slice.
4. Run required validation for touched scope.
5. Report exact commands and pass/fail outcomes.

## Hard guardrails

- Use `bun` for JS workflows.
- Keep frontend canonical in `frontend/`.
- Keep generated/build artifacts under `out/`.
- Do not use port `8080`.
- Do not revert unrelated workspace changes.

## Validation matrix

- Frontend: `cd frontend && bun run check && bun run build`
- Rust core: `cargo check --workspace && cargo test --workspace`
- Go tooling: `cd tooling && go test ./...`

## Evidence format

- Scope
- Commands run
- Results
- Artifacts
- Known limitations
