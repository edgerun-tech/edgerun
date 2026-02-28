# 2026-02-28 Local-First Solana Removal Phase 5 (v1)

## Goal
- Remove remaining Solana/program-era references from active Edgerun codepaths now that local-first architecture is canonical.
- Remove the legacy `program/` workspace integration surface from CLI, scripts, frontend runtime wiring, and operational docs.

## Non-Goals
- Rewrite historical changelog/spec/archive documents that are intentionally retained for historical record.
- Migrate persisted scheduler fields that still use `onchain_*` naming in shared types in this phase.

## Security / Constraints
- Preserve fail-closed boot and scheduler behavior from phases 1-4.
- Keep frontend and Rust workspace in a buildable/lint-clean state after removals.
- Do not add replacement blockchain coupling.

## Acceptance Criteria
1. `program/` directory is absent.
2. `edgerun-cli` no longer exposes Solana/program command surfaces or checks (`Program` command, Solana cluster enums, anchor/solana validator checks, program build/test targets).
3. Frontend no longer imports or wires Solana wallet/rpc modules in active runtime/build paths.
4. System scripts/docs no longer prescribe Solana program workflows.
5. Required validations pass:
   - `cargo check --workspace`
   - `cargo clippy -p edgerun-scheduler --all-targets -- -D warnings`
   - `cargo test -p edgerun-scheduler`
   - `cd frontend && bun run check`
   - `cd frontend && bun run build`

## Rollout / Rollback
- Rollout: apply phase-5 patch set after phase-4.
- Rollback: revert phase-5 commit(s); all removed Solana/program assets remain recoverable from git history.
