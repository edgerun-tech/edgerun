# 2026-03-01 Spec Retention Latest Five Pre-commit v1

## Goal
Keep `docs/specs/` clean by automatically retaining only the latest five change-spec files on each commit.

## Non-goals
- No retention changes for non change-spec docs (indexes, policy docs, static named specs).
- No remote/history rewriting.

## Security and Constraints
- Retention rule only targets date-prefixed change specs (`YYYY-MM-DD-*.md`).
- Hook must stage removals automatically so commits remain deterministic.
- Hook must be idempotent and safe when fewer than five change specs exist.

## Design
- Add `scripts/prune-change-specs.sh`:
  - collect date-prefixed spec files,
  - sort by modification time descending,
  - keep latest 5, delete older ones,
  - optionally stage spec directory updates.
- Install local git pre-commit hook to invoke pruning script before commit.

## Acceptance Criteria
1. Committing runs pruning automatically.
2. Only the latest 5 date-prefixed specs remain after commit.
3. Hook does not touch non date-prefixed spec docs.

## Rollout
- Ship script + hook installation together in this repo workspace.

## Rollback
- Remove pre-commit hook invocation and restore deleted specs from git history if needed.
