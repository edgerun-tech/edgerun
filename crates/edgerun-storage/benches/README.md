# Benchmarks

This repo uses two benchmark layers:

- `benches/`: microbenchmarks intended for `cargo bench` (Criterion harness).
- `tools/`: scenario/system benchmarks and soak workloads (custom binaries and scripts).

Use `benches/` when you need statistically robust micro-level perf comparisons.
Use `tools/` when you need end-to-end workload modeling, durability sweeps, and operational gates.
