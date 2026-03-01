# 2026-03-01 CI/Build Determinism and Hygiene Wins V1

## Goal

Apply a focused set of low-risk, high-impact repository hygiene fixes that improve CI signal quality, deterministic builds, and operational script correctness.

## Non-Goals

- Re-architect CI workflows beyond the existing job structure.
- Re-categorize all uncataloged historical specs in a single pass.
- Introduce new runtime dependencies or change product behavior.

## Security and Constraints

- Keep JavaScript workflows bun-first; do not introduce npm/pnpm flows.
- Preserve deterministic build posture by pinning toolchains and lockfile usage.
- Keep generated/build artifacts rooted under `out/` where practical.
- Avoid destructive repo operations and avoid touching unrelated in-flight workspace edits.

## Acceptance Criteria

1. CI runs clippy on PRs and uses locked dependency resolution for Rust checks.
2. CI uses the repository-pinned Rust toolchain version (not floating stable) in touched workflows.
3. Main CI includes at least one concrete test execution lane (`cargo test`) for Rust scope.
4. Stale workflow references (`release.yml`/`Release`) are removed from local checks and pipeline health wiring.
5. Snapshotter references are removed from containerd smoke + install scripts where the crate/binary no longer exists.
6. Intent UI production build no longer injects a timestamp-only cache-buster in tracked output.
7. Frontend intent UI build scripts default cargo artifacts to `out/target`.
8. Frontend build scripts avoid GNU-only `timeout` command usage in package scripts.
9. Spec index generation removes GNU-only `find -printf` usage.
10. Duplicate unused frontend JSON config mirrors are removed.

## Rollout Notes

- Land as one hygiene batch.
- Validate via workflow lint/hygiene scripts plus frontend and Rust checks.

## Rollback Notes

- Revert this change set as a single commit to restore prior workflow/script behavior.
