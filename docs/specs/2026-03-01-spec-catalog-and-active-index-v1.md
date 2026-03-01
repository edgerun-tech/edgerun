# 2026-03-01 Spec Catalog And Active Index V1

## Goal
- Introduce a deterministic spec-status catalog for `docs/specs`.
- Generate a compact active-spec index from that catalog.
- Mark stale agent-tooling-era specs as superseded after removal of `edgertool` and agent/executor wrapper surfaces.

## Non-Goals
- Rewriting historical spec content wholesale.
- Deleting large batches of historical specs in this phase.
- Changing runtime behavior or protocol implementations.

## Security and Constraints
- Spec status metadata must be explicit and version-controlled.
- Generation must be deterministic and runnable locally.
- Historical specs remain preserved in git history and docs tree.
- No new network dependencies or runtime services.

## Design
1. Add `docs/specs/spec-status.tsv` as status source of truth with:
   - `spec`, `status`, `domain`, `replaces`, `note`.
2. Add `scripts/generate-spec-index.sh` to:
   - read status catalog,
   - enumerate all `docs/specs/*.md` files,
   - generate `docs/specs/ACTIVE_SPECS_INDEX.md` with active and superseded sections,
   - list uncataloged specs for follow-up cleanup.
3. Add superseded banners to stale specs tied to removed agent tooling.

## Acceptance Criteria
1. `docs/specs/ACTIVE_SPECS_INDEX.md` is generated from catalog + file scan.
2. Stale agent-tooling specs are explicitly marked superseded.
3. Generator runs successfully and reports counts for active/superseded/historical/uncataloged.
4. Repo validation commands continue to pass.

## Rollout
- Land catalog + generator + initial stale annotations.
- Expand catalog coverage incrementally in future cleanup passes.

## Rollback
- Remove generator and catalog files.
- Revert superseded banners and generated index.
