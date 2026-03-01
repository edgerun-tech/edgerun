#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
MODE="${1:-all}"

print_hint() {
  echo "[e2e] Platform: $(uname -srm)"
  echo "[e2e] Rendering hint: software mode enabled; NVIDIA GPU is not required."
  echo "[e2e] Exporting ELECTRON_DISABLE_GPU=1 and LIBGL_ALWAYS_SOFTWARE=1"
}

export ELECTRON_DISABLE_GPU="${ELECTRON_DISABLE_GPU:-1}"
export LIBGL_ALWAYS_SOFTWARE="${LIBGL_ALWAYS_SOFTWARE:-1}"

print_hint

case "${MODE}" in
  all)
    bash "${ROOT_DIR}/scripts/e2e-frontend-all.sh"
    ;;
  core)
    bash "${ROOT_DIR}/scripts/e2e-frontend-core.sh"
    ;;
  compose|terminal)
    bash "${ROOT_DIR}/scripts/e2e-local-terminal.sh"
    ;;
  --help|-h|help)
    echo "Usage: scripts/e2e-frontend-runner.sh [all|core|compose]"
    ;;
  *)
    echo "Unknown mode: ${MODE}" >&2
    echo "Usage: scripts/e2e-frontend-runner.sh [all|core|compose]" >&2
    exit 2
    ;;
esac
