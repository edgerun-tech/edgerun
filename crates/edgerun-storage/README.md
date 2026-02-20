<!-- SPDX-License-Identifier: GPL-2.0-only -->
# Storage Engine

High-performance storage engine with async `io_uring` I/O, durability tiers, LSM indexing, crash recovery validation, and replicated durability primitives.

## Current Status

- Phase A-D core architecture is implemented.
- 10k crash campaign passed (`10000/10000`).
- Perf and mixed workload gates are automated through `tools/perf_gate.sh`.

Authoritative planning docs:
- `PRODUCTION_PLAN.md`
- `ROADMAP.md`

## Repository Layout

- `src/`: core engine modules
- `benches/`: microbenchmarks (`cargo bench`, Criterion)
- `tests/`: integration tests
- `tools/`: scenario benchmarks, gates, and operational tooling
- `docs/`: architecture/development/operations docs

## Quickstart

```bash
cargo check --all-targets
cargo test -q
```

Performance gate (smoke-friendly):

```bash
MIXED_RW_SWEEP_DURATION=1 MIXED_RW_SWEEP_MAX_CASES=1 \
MIN_TOP_SCORE=1 MIN_TOP_WRITES_OPS=1 MIN_TOP_READS_OPS=1 MAX_TOP_COMP_FAILED=999 \
tools/perf_gate.sh
```

## Key Commands

- `cargo run -q --bin crash_campaign -- --iterations 1000 --data-dir /tmp/crash_campaign --target-mb 2 --random true`
- `tools/mixed_rw_tuning_sweep.sh --duration 8`
- `cargo run -q --bin replication_group_commit_benchmark -- --mode batch --events 20000 --batch-size 128`
- `tools/devctl.sh enc-demo --provider passphrase --passphrase "dev-secret" --events 1000` (includes verify+decrypt readback)
- `tools/devctl.sh ci-smoke`

## Security

- Dependency vulnerability scan is automated in `.github/workflows/security.yml` (`cargo audit`).

## Professionalization Path

Phase 1 (implemented):
- Baseline docs and CI workflows.
- Explicit gate commands and operational runbooks.
- Security scanning workflow (`cargo audit`).
- Unified local developer CLI (`tools/devctl.sh`).

Phase 2 (next):
- Split runtime binaries (`tools/`) into curated benchmark profiles and archived experiments.
- Introduce release checklist artifact publishing in CI.
- Add API stability and compatibility policy.
