# CloudOS Consolidation + Containerd Runtime Integration V1

## Goal
- Consolidate product direction into the canonical `frontend/` surface by removing the parallel `cloud-os/` app root while preserving the CloudOS vision as documentation/content.
- Import the containerd snapshotter/shim/runtime-proto integration work from `origin/main` into the active line without importing unrelated cloud-os/frontend churn.

## Non-goals
- No resurrection of `cloud-os/` runtime code or deployment target.
- No full rebase/merge of all `origin/main` commits.
- No behavioral redesign of runtime execution semantics beyond importing existing containerd/snapshotter integration artifacts.

## Security and Constraint Requirements
- Keep frontend canonical in `frontend/`; no second frontend root.
- Keep Bun-only JS workflows.
- Preserve deterministic workspace structure and avoid generated artifact commits.
- Avoid introducing mocked on-chain views.
- Keep containerd integration additive and explicit (no hidden runtime overrides beyond documented config snippets).

## Acceptance Criteria
1. `cloud-os/` directory is removed from repository tracking.
2. CloudOS README direction is preserved in-repo under docs/frontend-owned content.
3. Root scripts/configs no longer require `cloud-os/` for drift/build/deploy checks.
4. Workspace includes imported containerd integration components from `origin/main`:
   - `crates/edgerun-containerd-shim`
   - `crates/edgerun-snapshotter`
   - `crates/edgerun-runtime-proto`
   - supporting containerd/systemd scripts and config snippet
5. Validation passes (or blockers are explicitly reported) for:
   - `cd frontend && bun run check`
   - `cd frontend && bun run build`
   - `cargo check --workspace`
   - targeted Rust tests for new crates (when feasible)

## Rollout
1. Add this spec.
2. Import containerd/snapshotter/runtime-proto artifacts.
3. Move CloudOS vision content into docs/frontend, remove `cloud-os/`, and update scripts/docs.
4. Run validation and publish evidence.

## Rollback
- Revert this change set to restore previous dual-root layout and pre-import runtime integration state.
- If partial rollback is required, revert containerd import and cloud-os removal independently in separate commits.
