# Anchor Program Scaffold

This directory is intentionally separate from the Rust workspace and is scaffolded for Anchor workflows.

Planned program name: `edgerun_program`

## Local toolchain usage (workspace-confined)

Use the local wrappers so Cargo/Anchor state stays inside this `program/` directory:

- `./scripts/cargo-local ...`
- `./scripts/anchor-local ...`

Examples:

- Install `cargo-build-sbf` locally: `./scripts/cargo-local install cargo-build-sbf --version 4.0.0`
- Build SBF locally: `./scripts/cargo-local build-sbf --manifest-path programs/edgerun_program/Cargo.toml`
- Build with Anchor locally: `./scripts/anchor-local build`
- Install JS deps with Bun: `bun install`
- Run tests with Bun + local validator: `./scripts/test-bun-local`
