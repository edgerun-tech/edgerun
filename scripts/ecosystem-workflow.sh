#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0

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
  if [[ -f "cloud-os/eslint.config.js" || -f "cloud-os/eslint.config.mjs" || -f "cloud-os/eslint.config.cjs" || -f "cloud-os/.eslintrc" || -f "cloud-os/.eslintrc.json" || -f "cloud-os/.eslintrc.js" || -f "cloud-os/.eslintrc.cjs" ]]; then
    (cd cloud-os && bun run lint)
  else
    echo "[check] cloud-os lint skipped: no eslint config present"
  fi
}

run_build() {
  (cd frontend && bun run build)
  (cd cloud-os && bun run build)
}

run_test() {
  cargo test --workspace
  (cd cloud-os && bun run test:run)
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
