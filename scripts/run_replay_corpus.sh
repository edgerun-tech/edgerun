#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT_DIR="${1:-$ROOT_DIR/.edgerun-replay-corpus}"

mkdir -p "$OUT_DIR"

PROFILE_NAME_DEBUG="${REPLAY_PROFILE_DEBUG:-local-debug}"
PROFILE_NAME_RELEASE="${REPLAY_PROFILE_RELEASE:-local-release}"
RUNS="${REPLAY_CORPUS_RUNS:-3}"

cd "$ROOT_DIR"

echo "running replay corpus for $PROFILE_NAME_DEBUG"
cargo run -p edgerun-runtime -- replay-corpus \
  --profile "$PROFILE_NAME_DEBUG" \
  --artifact "$OUT_DIR/${PROFILE_NAME_DEBUG}.json" \
  --runs "$RUNS"

echo "running replay corpus for $PROFILE_NAME_RELEASE"
cargo run --release -p edgerun-runtime -- replay-corpus \
  --profile "$PROFILE_NAME_RELEASE" \
  --artifact "$OUT_DIR/${PROFILE_NAME_RELEASE}.json" \
  --runs "$RUNS"

echo "comparing replay profiles"
"$ROOT_DIR/scripts/compare_replay_profiles.sh" \
  "$OUT_DIR/${PROFILE_NAME_DEBUG}.json" \
  "$OUT_DIR/${PROFILE_NAME_RELEASE}.json"

echo "replay corpus artifacts written to $OUT_DIR"
