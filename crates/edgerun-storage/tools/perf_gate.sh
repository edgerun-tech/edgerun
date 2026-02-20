#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-2.0-only
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

# Conservative defaults based on current observed local results.
MIN_END_TO_END_P1_MBPS="${MIN_END_TO_END_P1_MBPS:-120}"
MIN_IO_ONLY_P1_MBPS="${MIN_IO_ONLY_P1_MBPS:-1800}"
MIN_END_TO_END_P8_MBPS="${MIN_END_TO_END_P8_MBPS:-450}"
MIXED_RW_SWEEP_DURATION="${MIXED_RW_SWEEP_DURATION:-4}"
MIXED_RW_SWEEP_MAX_CASES="${MIXED_RW_SWEEP_MAX_CASES:-4}"

extract_mbps() {
  local text="$1"
  local mode="$2"
  awk -v mode="$mode" '
    $0 ~ ("--- Mode: " mode " ---") { in_mode=1; next }
    /^--- Mode: / { in_mode=0 }
    in_mode && /Throughput: [0-9.]+ MB\/s/ {
      val=$2
      gsub(/[^0-9.]/, "", val)
      print val
      exit
    }
  ' <<<"$text"
}

assert_ge() {
  local value="$1"
  local min="$2"
  local label="$3"
  awk -v v="$value" -v m="$min" -v l="$label" '
    BEGIN {
      if (v+0 < m+0) {
        printf("FAIL: %s %.2f MB/s < %.2f MB/s\n", l, v+0, m+0);
        exit 1;
      }
      printf("PASS: %s %.2f MB/s >= %.2f MB/s\n", l, v+0, m+0);
    }
  '
}

echo "Running Phase A perf gate..."

out_p1="$(cargo run -q --bin async_writer_benchmark -- --mode both --producers 1)"
echo "$out_p1"
e2e_p1="$(extract_mbps "$out_p1" "end_to_end")"
io_p1="$(extract_mbps "$out_p1" "io_only")"

out_p8="$(cargo run -q --bin async_writer_benchmark -- --mode end_to_end --producers 8)"
echo "$out_p8"
e2e_p8="$(extract_mbps "$out_p8" "end_to_end")"

if [[ -z "$e2e_p1" || -z "$io_p1" || -z "$e2e_p8" ]]; then
  echo "FAIL: unable to parse benchmark throughput output"
  exit 1
fi

assert_ge "$e2e_p1" "$MIN_END_TO_END_P1_MBPS" "end_to_end producers=1"
assert_ge "$io_p1" "$MIN_IO_ONLY_P1_MBPS" "io_only producers=1"
assert_ge "$e2e_p8" "$MIN_END_TO_END_P8_MBPS" "end_to_end producers=8"

echo "Phase A perf gate passed."

echo
echo "Running mixed RW tuning sweep gate..."
tools/mixed_rw_tuning_sweep.sh \
  --duration "$MIXED_RW_SWEEP_DURATION" \
  --max-cases "$MIXED_RW_SWEEP_MAX_CASES"

echo "Mixed RW tuning sweep gate passed."
