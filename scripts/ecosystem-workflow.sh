#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
MODE="${1:-verify}"

cd "${ROOT_DIR}"

run_drift() {
  ./scripts/check-workflow-drift.sh
}

run_check() {
  run_drift
  cargo check --workspace
  (cd frontend && bun run check)
  (cd program && bun run lint)
}

run_build() {
  (cd frontend && bun run build)
}

run_test() {
  cargo test --workspace
}

case "${MODE}" in
  drift)
    run_drift
    ;;
  check)
    run_check
    ;;
  build)
    run_build
    ;;
  test)
    run_test
    ;;
  verify)
    run_check
    run_build
    run_test
    ;;
  *)
    echo "usage: $0 {drift|check|build|test|verify}" >&2
    exit 1
    ;;
esac
