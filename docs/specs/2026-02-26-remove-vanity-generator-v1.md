# Remove Vanity Generator (v1)

## Goal
Remove the Solana vanity generator application from the workspace and frontend UX so users can no longer select or run it.

## Non-goals
- Redesigning the run flow beyond what is needed to remove vanity-specific behavior.
- Changing scheduler protocol semantics.
- Introducing new preset modules.

## Security and Constraint Requirements
- Preserve existing deterministic run submission and validation behavior for remaining presets.
- Keep JavaScript workflows bun-only.
- Keep frontend app root under `frontend/`.
- Do not introduce mock on-chain data paths.
- Do not bind anything to port 8080.

## Acceptance Criteria
1. Rust workspace no longer includes vanity generator crates.
2. `edgerun-apps/solana-vanity-address-generator/` is removed.
3. Frontend `/run/` no longer references a vanity preset or vanity-only fields.
4. Cypress run-job UX coverage is updated to validate the post-vanity flow.
5. Docs source catalog no longer points to vanity client/payload docs or API specs.
6. Repo-wide search for `vanity-generator`, `edgerun-vanity`, and `solana-vanity-address-generator` returns no active feature references.

## Rollout
- Ship as a single removal change.
- Existing links to vanity docs/pages will stop being generated and should be treated as removed content.

## Rollback
- Revert this change set to restore workspace members, app directory, and frontend preset wiring.
