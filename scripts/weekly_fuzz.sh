#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
FUZZ_DIR="$ROOT_DIR/crates/edgerun-runtime/fuzz"
ARTIFACT_DIR="${1:-$ROOT_DIR/.edgerun-fuzz-weekly}"
SECONDS_PER_TARGET="${FUZZ_SECONDS_PER_TARGET:-300}"

mkdir -p "$ARTIFACT_DIR"

if ! command -v cargo-fuzz >/dev/null 2>&1; then
  echo "cargo-fuzz is not installed. Install with: cargo install cargo-fuzz"
  exit 1
fi

cd "$FUZZ_DIR"

TARGETS=(
  "fuzz_bundle_decode"
  "fuzz_validate_wasm"
  "fuzz_hostcall_boundary"
)

echo "weekly fuzz run started at $(date -u +%Y-%m-%dT%H:%M:%SZ)" | tee "$ARTIFACT_DIR/summary.log"
echo "seconds_per_target=$SECONDS_PER_TARGET" | tee -a "$ARTIFACT_DIR/summary.log"

for target in "${TARGETS[@]}"; do
  echo "running $target" | tee -a "$ARTIFACT_DIR/summary.log"
  cargo fuzz run "$target" -- -max_total_time="$SECONDS_PER_TARGET" \
    2>&1 | tee "$ARTIFACT_DIR/${target}.log"
done

CRASH_COUNT="$(find artifacts -type f 2>/dev/null | wc -l | tr -d ' ')"
echo "crash_artifacts=$CRASH_COUNT" | tee -a "$ARTIFACT_DIR/summary.log"

if [[ "$CRASH_COUNT" != "0" ]]; then
  echo "fuzz crashes detected. See scripts/fuzz_triage_template.md"
  exit 2
fi

echo "weekly fuzz run completed cleanly" | tee -a "$ARTIFACT_DIR/summary.log"
