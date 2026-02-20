<!-- SPDX-License-Identifier: GPL-2.0-only -->
# Repo Structure Plan

## Current (Phase 1)

- `src/`: core runtime and data-plane modules
- `tests/`: integration behavior
- `tools/`: benchmark and gate binaries/scripts
- `docs/`: architecture and operational guidance

## Recommended Phase 2

1. Keep only actively used operational tools under `tools/`.
2. Move one-off or legacy experimental binaries into `tools/archive/`.
3. Add `xtask` (or equivalent) for consistent local/CI orchestration.
4. Add machine-readable benchmark/crash artifact publishing in CI.

## Recommended Phase 3

1. Split transport/replication and storage core into internal modules/crates if API boundaries stabilize.
2. Introduce semver + compatibility docs for persisted formats.
