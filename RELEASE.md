# Release Process

Edgerun Core releases are source-first while the protocol and crate boundaries
are still alpha.

## Release Channels

- `v0.x.y-alpha.N`: protocol/runtime alpha. APIs and wire-level details may
  still change.
- `v0.x.y`: reserved for a release with documented compatibility expectations
  for the selected v0 surface.

## What Gets Released

GitHub tag releases may include:

- source archive;
- `edgerund` Linux amd64 binary;
- SHA-256 checksums;
- protocol specification and conformance vectors included in the source tree.

Crates.io publication is intentionally disabled by default in the workspace.
Before promoting any crate to crates.io, that crate must have:

- explicit package readiness review;
- version requirements on every internal path dependency;
- clean package contents checked with `cargo package -p <crate> --list`;
- accurate README/docs status labels;
- compatible dependency licenses verified with `cargo deny check licenses`.

Initial candidates for crates.io, after cleanup, are:

- - `edgerun-core`;
- `edgerun-stream`;
- `edgerun-storage`;
- `edgerun-crypto`;
- `edgerun-encoding`;
- `edgerun-log`.

## Pre-Tag Checklist

```bash
cargo metadata --no-deps --format-version 1
cargo deny check licenses
cargo test -p edgerun-core
cargo test -p edgerun-stream
cargo test -p edgerun-storage
cargo test -p edgerun-node
```

For bare/unikernel changes:

```bash
cargo +nightly build --release -p edgerun-unikernel \
  --target x86_64-unknown-none \
  -Zbuild-std=core,alloc
```

## Tagging

Create annotated tags:

```bash
git tag -a v0.1.1-alpha.1 -m "Edgerun Core v0.1.1-alpha.1"
git push origin v0.1.1-alpha.1
```

The GitHub release workflow builds release artifacts from tags matching `v*`.
