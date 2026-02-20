<!-- SPDX-License-Identifier: GPL-2.0-only -->
# Contributing

## Development Standards

- Keep `cargo check --all-targets` and `cargo test -q` green.
- Keep warnings clean (`RUSTFLAGS='-D warnings'`).
- Prefer additive, test-backed changes.
- Do not weaken durability semantics without explicit plan updates.

## Before Opening a PR

```bash
cargo fmt
RUSTFLAGS='-D warnings' cargo check --all-targets
cargo test -q
MIXED_RW_SWEEP_DURATION=1 MIXED_RW_SWEEP_MAX_CASES=1 \
MIN_TOP_SCORE=1 MIN_TOP_WRITES_OPS=1 MIN_TOP_READS_OPS=1 MAX_TOP_COMP_FAILED=999 \
tools/perf_gate.sh
```

## Commit Scope

- Keep commits focused and reviewable.
- Update `PRODUCTION_PLAN.md` and `ROADMAP.md` when behavior/roadmap changes.
