<!-- SPDX-License-Identifier: GPL-2.0-only -->
# Crate Publishing Checklist

Use this checklist before `cargo publish`.

## Required Metadata

- `name`, `version`, `edition`
- `description`
- `readme`
- `repository`
- `homepage` (optional but recommended)
- `documentation` (typically docs.rs)
- `keywords`, `categories`

## Legal/Policy

- Choose and add a license (`license` or `license-file`) in `Cargo.toml`.
- Ensure all dependencies are compatible with chosen license policy.

## Quality Gates

- `cargo fmt --check`
- `RUSTFLAGS='-D warnings' cargo check --all-targets`
- `cargo test -q`
- `tools/perf_gate.sh`
- crash campaign target run complete
- `cargo audit` clean or explicitly triaged

## Package Validation

- `cargo package --allow-dirty` (local sanity)
- Inspect package contents and size.
- Verify no local tmp/test artifacts are included.

## Versioning & Changelog

- Bump version using semver.
- Record notable behavior changes in release notes.
- Tag release in VCS after publish.
