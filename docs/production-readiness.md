# EdgeRun Production Readiness

This file tracks the concrete gates for deploying the current EdgeRun UI, browser
node, and `edgerun-work` integration. It is intentionally release-facing: items
without real state or test coverage stay blocked.

## Current Status

| Area | Status | Required before production |
| --- | --- | --- |
| `edgerun-work` protocol hashes | Done | Keep `tests/golden_hashes.rs` locked and update constants only after deliberate hash-domain changes. |
| UI component system | In progress | Keep app surfaces composed from shadcn-style primitives and prevent direct painter-only controls from returning. |
| Runtime work projection | In progress | Feed real role-instance/admission/capability state from the host; default UI must show pending state, not fabricated proof values. |
| App run/cache flow | Blocked | Add the first-run prompt: Run once, Verify & cache, Cancel. Wire to package verification and cache state. |
| Trust Manager | Blocked | Derive rows from Trust Container, runtime events, package cache, and capability grants. Remove hardcoded proof rows. |
| Admission UX | Blocked | Let the user choose EdgeRun DAO admission or user-owned admission and show policy source, route, and admission hash. |
| Node instances | Blocked | Expose role-specific instances: identity, role, runtime target, policy hash, budget, and route scope. |
| Settlement/payment claims | Blocked | Show only measured receipts and proofs. Unknown values must render as unknown or pending. |
| Frontend CI | In progress | Keep UI core tests and the EdgeRun frontend WASM build in CI. Add browser screenshot checks once the renderer is stable. |

## Release Gates

Run these before a production candidate:

```bash
cargo test --manifest-path crates/protocol/edgerun-work/Cargo.toml
cargo test --manifest-path crates/protocol/edgerun-work/Cargo.toml --no-default-features
cargo build --manifest-path crates/protocol/edgerun-work/Cargo.toml --target wasm32-unknown-unknown --release --no-default-features
crates/protocol/edgerun-work/scripts/wasm-size.sh
cargo test --manifest-path crates/utility/edgerun-ui-core/Cargo.toml --features fontdue-text --lib
cargo test --manifest-path crates/edgerun-codex/Cargo.toml -p edgerun-frontend-web
cargo build --manifest-path crates/edgerun-codex/Cargo.toml -p edgerun-frontend-web --target wasm32-unknown-unknown --release
```

## Non-Negotiables

- Do not reintroduce `codec.rs` compatibility re-exports for signing or identity helpers.
- Do not ship deterministic demo keys, fake signed objects, or fabricated proof hashes in the production host.
- Do not present install semantics in new UX. Use run, verify and cache, cached, and remove cache.
- Do not bypass admission in user-facing flows. Browser nodes sign intent; admission admits or rejects work.
- Do not show preview metrics as measured. Label preview, pending, or unknown explicitly.
- Do not use unchecked receipt settlement for relay payments.

## Next Production Slices

1. Feed real `UiWorkProjection` values from the active role instance and admission state through the WASM host byte API.
2. Add the first-run network app prompt and wire it to verified cache state.
3. Replace Trust Manager mock rows with projections from auth, runtime events, cache, and capability grant stores.
4. Add browser screenshot and hit-test checks for the component gallery and core app surfaces.
5. Add CI coverage for the wasm size script once the script has executable permission or is invoked through `sh`.
