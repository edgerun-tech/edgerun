# 2026-03-01 Docker Log Ingest Benchmark Tool V1

## Goal
- Add a focused benchmark binary to measure docker log ingest throughput and latency by batch size.

## Non-Goals
- Production telemetry export.
- Cross-machine benchmark normalization.

## Design
- New tool: `tools/docker_log_ingest_benchmark.rs`.
- Generates synthetic non-JSON docker log lines in-memory.
- Runs ingest using `ingest_docker_log_lines_batched` for multiple batch sizes.
- Prints:
  - lines total
  - elapsed ms
  - lines/sec
  - per-batch p50/p95 ms
  - appended/skipped counters

## Acceptance Criteria
1. Binary runs with no external dependencies.
2. Output includes clear per-batch-size metrics.
3. Benchmark helps pick a default `--batch-lines` value.
