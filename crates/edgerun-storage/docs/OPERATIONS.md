<!-- SPDX-License-Identifier: GPL-2.0-only -->
# Operations

## Release Gates

1. `RUSTFLAGS='-D warnings' cargo check --all-targets`
2. `cargo test -q`
3. `tools/perf_gate.sh`
4. Crash campaign at required scale

## Crash Campaign

Example:

```bash
cargo run -q --bin crash_campaign -- \
  --iterations 10000 \
  --data-dir /tmp/crash_campaign_10k \
  --target-mb 2 \
  --random true \
  --keep-failed true
```

## Perf Gate Tuning Knobs

- `MIXED_RW_SWEEP_DURATION`
- `MIXED_RW_SWEEP_MAX_CASES`
- `MIN_TOP_SCORE`
- `MIN_TOP_WRITES_OPS`
- `MIN_TOP_READS_OPS`
- `MAX_TOP_COMP_FAILED`

## Incident Triage

- Capture failing command + env vars.
- Preserve output artifact directories under `/tmp/*` used by tools.
- Record regression against last known good sweep/crash reports.
