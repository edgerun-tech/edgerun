#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-2.0-only
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

usage() {
  cat <<'USAGE'
Usage: tools/devctl.sh <command> [args]

Commands:
  check               Run warnings-clean checks
  test                Run test suite
  perf-gate           Run perf gate
  sweep [args]        Run mixed RW tuning sweep (forwards args)
  crash [args]        Run crash campaign (forwards args)
  bench [args]        Run cargo bench (forwards args)
  rep-bench [args]    Run replication group-commit benchmark (forwards args)
  enc-demo [args]     Run encrypted append demo (forwards args)
  ci-smoke            Run local CI-equivalent smoke pipeline
USAGE
}

cmd="${1:-}"
shift || true

case "$cmd" in
  check)
    RUSTFLAGS='-D warnings' cargo check --all-targets
    ;;
  test)
    cargo test -q
    ;;
  perf-gate)
    tools/perf_gate.sh
    ;;
  sweep)
    tools/mixed_rw_tuning_sweep.sh "$@"
    ;;
  crash)
    cargo run -q --bin crash_campaign -- "$@"
    ;;
  bench)
    cargo bench "$@"
    ;;
  rep-bench)
    cargo run -q --bin replication_group_commit_benchmark -- "$@"
    ;;
  enc-demo)
    cargo run -q --bin encrypted_append_demo -- "$@"
    ;;
  ci-smoke)
    cargo fmt --check
    RUSTFLAGS='-D warnings' cargo check --all-targets
    cargo test -q
    MIXED_RW_SWEEP_DURATION=1 \
    MIXED_RW_SWEEP_MAX_CASES=1 \
    MIN_TOP_SCORE=1 \
    MIN_TOP_WRITES_OPS=1 \
    MIN_TOP_READS_OPS=1 \
    MAX_TOP_COMP_FAILED=999 \
    tools/perf_gate.sh
    ;;
  *)
    usage
    exit 2
    ;;
esac
