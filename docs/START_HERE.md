# Start Here

EdgeRun is a public preservation snapshot for user-owned internet
infrastructure. The Pages site is intentionally rendered by
`edgerun-ui-core`: Rust builds the docs and shadcn showcase as `GpuScene`
buffers, while the browser page is only a JavaScript/WebGL byte bridge.

## What To Inspect First

- `crates/protocol/edgerun-work`: admitted work, routes, channels, proofs,
  receipts, verifier/notary reports, custody, and settlement.
- `crates/protocol/edgerun-wire`: rkyv-only records for browser/native,
  node/app, capability, runtime, and transport boundaries.
- `crates/utility/edgerun-ui-core`: Rust UI scene system, shadcn component
  showcase, shell/workspace surfaces, packed scene buffers, and host contracts.
- `crates/node/edgerun-docs-ui-web`: the GitHub Pages WASM host for the public
  docs and shadcn showcase.

## Current Status

This is not production infrastructure. The strongest implemented area is the
`edgerun-work` proof/admission/settlement model and the shared `edgerun-ui-core`
scene/component system. Many runtime, storage, compute, payment, and deployment
paths are still experimental.

## Useful Checks

```bash
rustup run stable cargo check -p edgerun-docs-ui-web --target wasm32-unknown-unknown
rustup run stable cargo test -p edgerun-ui-core
rustup run stable cargo test -p edgerun-work
```

The repository also contains a local `./cargo` shim. If that shim cannot
bootstrap on a machine, use `rustup run stable cargo ...` for public snapshot
checks until the shim path is repaired.
