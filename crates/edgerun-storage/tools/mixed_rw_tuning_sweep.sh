#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-2.0-only
set -euo pipefail

duration=8
out_dir="/tmp/mixed_rw_tuning_sweep_$(date +%Y%m%d_%H%M%S)"
max_cases=0
min_top_score="${MIN_TOP_SCORE:-700000}"
min_top_writes_ops="${MIN_TOP_WRITES_OPS:-250000}"
min_top_reads_ops="${MIN_TOP_READS_OPS:-80000}"
max_top_comp_failed="${MAX_TOP_COMP_FAILED:-0}"

usage() {
  cat <<'EOF'
Usage: tools/mixed_rw_tuning_sweep.sh [--duration N] [--out-dir PATH] [--max-cases N]

Runs a parameter sweep against `mixed_rw_compaction_benchmark` and writes:
  - results.csv
  - summary.md
  - per-run logs in logs/
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --duration)
      duration="${2:-}"
      shift 2
      ;;
    --out-dir)
      out_dir="${2:-}"
      shift 2
      ;;
    --max-cases)
      max_cases="${2:-}"
      shift 2
      ;;
    --help|-h)
      usage
      exit 0
      ;;
    *)
      echo "unknown arg: $1" >&2
      usage
      exit 2
      ;;
  esac
done

mkdir -p "$out_dir/logs"
csv="$out_dir/results.csv"
md="$out_dir/summary.md"

echo "case_id,writers,readers,write_batch,read_batch,key_space,hot_key_space,writes_ops,reads_ops,hit_rate_pct,comp_sched,comp_done,comp_failed,comp_skipped,comp_total_ms,score,log" > "$csv"

# Tuned around recent stable ranges.
cases=(
  "2 4 512 2048 2000000 200000"
  "2 4 1024 2048 2000000 200000"
  "2 6 512 4096 2500000 250000"
  "3 6 512 4096 2500000 250000"
  "3 8 512 4096 3000000 300000"
  "4 8 512 4096 3000000 300000"
  "4 8 1024 4096 3000000 300000"
  "4 10 1024 4096 3500000 350000"
)

run_count=0
for i in "${!cases[@]}"; do
  if [[ "$max_cases" -gt 0 && "$run_count" -ge "$max_cases" ]]; then
    break
  fi
  case_id=$((i + 1))
  read -r writers readers write_batch read_batch key_space hot_key_space <<<"${cases[$i]}"
  log="$out_dir/logs/case_${case_id}.log"
  echo "[case ${case_id}] writers=${writers} readers=${readers} write_batch=${write_batch} read_batch=${read_batch}"

  cargo run -q --bin mixed_rw_compaction_benchmark -- \
    --duration "$duration" \
    --writers "$writers" \
    --readers "$readers" \
    --write-batch "$write_batch" \
    --read-batch "$read_batch" \
    --key-space "$key_space" \
    --hot-key-space "$hot_key_space" \
    > "$log"

  writes_ops="$(grep '^writes:' "$log" | tail -1 | sed -E 's/.*\(([0-9]+) ops\/s\).*/\1/')"
  reads_ops="$(grep '^reads:' "$log" | tail -1 | sed -E 's/.*\(([0-9]+) ops\/s\).*/\1/')"
  hit_rate_pct="$(grep '^reads:' "$log" | tail -1 | sed -E 's/.*hit_rate=([0-9.]+)%.*/\1/')"

  comp_line="$(grep '^compaction:' "$log" | tail -1)"
  comp_sched="$(echo "$comp_line" | sed -E 's/.*scheduled=([0-9]+).*/\1/')"
  comp_done="$(echo "$comp_line" | sed -E 's/.*completed=([0-9]+).*/\1/')"
  comp_failed="$(echo "$comp_line" | sed -E 's/.*failed=([0-9]+).*/\1/')"
  comp_skipped="$(echo "$comp_line" | sed -E 's/.*skipped=([0-9]+).*/\1/')"
  comp_total_ms="$(echo "$comp_line" | sed -E 's/.*total_ms=([0-9]+).*/\1/')"

  # Weighted score: prioritize writes, then reads; penalize failed compactions heavily.
  score="$(awk -v w="$writes_ops" -v r="$reads_ops" -v f="$comp_failed" 'BEGIN{printf "%.2f", w + (r*4.0) - (f*1000000.0)}')"

  echo "${case_id},${writers},${readers},${write_batch},${read_batch},${key_space},${hot_key_space},${writes_ops},${reads_ops},${hit_rate_pct},${comp_sched},${comp_done},${comp_failed},${comp_skipped},${comp_total_ms},${score},${log}" >> "$csv"
  run_count=$((run_count + 1))
done

best_row="$(tail -n +2 "$csv" | sort -t, -k16,16nr | head -1)"
if [[ -z "$best_row" ]]; then
  echo "FAIL: no sweep results found" >&2
  exit 1
fi

best_score="$(echo "$best_row" | awk -F, '{print $16}')"
best_writes="$(echo "$best_row" | awk -F, '{print $8}')"
best_reads="$(echo "$best_row" | awk -F, '{print $9}')"
best_comp_failed="$(echo "$best_row" | awk -F, '{print $13}')"
best_case="$(echo "$best_row" | awk -F, '{print $1}')"

gate_failed=0

awk -v s="$best_score" -v min="$min_top_score" '
  BEGIN { exit((s+0 >= min+0) ? 0 : 1) }
' || {
  echo "FAIL: top score ${best_score} < min ${min_top_score}" >&2
  gate_failed=1
}

awk -v w="$best_writes" -v min="$min_top_writes_ops" '
  BEGIN { exit((w+0 >= min+0) ? 0 : 1) }
' || {
  echo "FAIL: top writes/s ${best_writes} < min ${min_top_writes_ops}" >&2
  gate_failed=1
}

awk -v r="$best_reads" -v min="$min_top_reads_ops" '
  BEGIN { exit((r+0 >= min+0) ? 0 : 1) }
' || {
  echo "FAIL: top reads/s ${best_reads} < min ${min_top_reads_ops}" >&2
  gate_failed=1
}

awk -v f="$best_comp_failed" -v max="$max_top_comp_failed" '
  BEGIN { exit((f+0 <= max+0) ? 0 : 1) }
' || {
  echo "FAIL: top comp_failed ${best_comp_failed} > max ${max_top_comp_failed}" >&2
  gate_failed=1
}

{
  echo "# Mixed RW Tuning Sweep"
  echo
  echo "- Duration per case: ${duration}s"
  echo "- Cases run: ${run_count}"
  echo "- CSV: \`${csv}\`"
  echo "- Gate thresholds:"
  echo "  - min_top_score: ${min_top_score}"
  echo "  - min_top_writes_ops: ${min_top_writes_ops}"
  echo "  - min_top_reads_ops: ${min_top_reads_ops}"
  echo "  - max_top_comp_failed: ${max_top_comp_failed}"
  echo "- Top case: ${best_case} (score=${best_score}, writes/s=${best_writes}, reads/s=${best_reads}, comp_failed=${best_comp_failed})"
  echo
  echo "## Ranked Results"
  echo
  echo "| Rank | Case | W | R | WB | RB | writes/s | reads/s | hit% | comp_failed | score |"
  echo "|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|"
  tail -n +2 "$csv" | sort -t, -k16,16nr | awk -F, '{
    rank=NR;
    printf("| %d | %s | %s | %s | %s | %s | %s | %s | %s | %s | %s |\n",
      rank, $1, $2, $3, $4, $5, $8, $9, $10, $13, $16);
  }'
} > "$md"

echo
echo "Sweep complete."
echo "CSV: $csv"
echo "Summary: $md"
if [[ "$gate_failed" -ne 0 ]]; then
  echo "Mixed RW tuning sweep gate FAILED"
  exit 1
fi
echo "Mixed RW tuning sweep gate passed."
