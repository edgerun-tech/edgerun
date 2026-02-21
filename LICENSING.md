# Edgerun Licensing Matrix

This repository uses a mixed licensing model.

## Intended Policy

- Core protocol: Apache 2.0
- Scheduler + advanced infra: Proprietary (for now)

## Current Directory Mapping

- `program/` -> Apache 2.0
- `crates/edgerun-runtime/` -> Apache 2.0
- `crates/edgerun-cli/` (verifier CLI) -> Apache 2.0
- `docs/` -> Apache 2.0
- `crates/edgerun-worker/` (worker daemon) -> Apache 2.0
- `crates/edgerun-scheduler/` -> Proprietary

## Notes

- Each listed subproject has its own `LICENSE` file.
- `crates/edgerun-scheduler/Cargo.toml` is marked with:
  `license = "LicenseRef-Edgerun-Proprietary"`.
- Apache-licensed components remain usable for community adoption and ecosystem growth.
- Proprietary scheduler code preserves commercialization and fundraising flexibility.
- SPDX headers are enforced in CI via `scripts/spdx-check.sh`.
- You can normalize/update headers locally with `scripts/spdx-normalize.sh`.
