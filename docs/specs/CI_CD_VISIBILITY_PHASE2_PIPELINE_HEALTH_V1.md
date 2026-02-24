# CI/CD Visibility Phase 2 Pipeline Health V1

## Goal
- Add a consolidated CI/CD visibility surface that summarizes workflow outcomes across core pipelines.
- Publish machine-readable quality artifacts from CI so failures and trends can be consumed by tools.
- Improve release/deploy traceability by surfacing commit identity, artifact hashes, and target context in workflow summaries.

## Non-goals
- No changes to build/test/deploy semantics.
- No migration to external observability SaaS in this phase.
- No secret value exposure in summaries or artifacts.

## Security and constraints
- Summaries must not print secret values or host credentials.
- Artifact traces must use deterministic, reproducible data derived from workflow outputs/files.
- Keep bun-first JS policy unchanged and avoid adding npm/pnpm workflows.
- Keep workflows compatible with local `act` dry-runs.

## Acceptance criteria
1. New workflow `pipeline-health.yml` runs on completion of core workflows and writes a consolidated run summary.
2. `CI` uploads machine-readable artifacts at minimum:
   - Rust nextest JUnit XML,
   - coverage summary JSON,
   - existing runtime quality JSON artifacts remain uploaded.
3. `CI` summary includes artifact pointers and a direct run link.
4. Release/deploy workflows include traceability metadata in summaries:
   - ref + commit SHA,
   - artifact hash info (where artifact files exist),
   - target/deploy context (non-secret).

## Rollout and rollback
- Rollout: merge workflow changes, validate with local `act` dry-runs, then observe first remote runs.
- Rollback: remove `pipeline-health.yml` and summary/hash additions; restore prior workflow-only visibility model.
