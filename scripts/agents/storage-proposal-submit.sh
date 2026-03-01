#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
set -euo pipefail

if [[ $# -lt 3 ]]; then
  echo "Usage: scripts/agents/storage-proposal-submit.sh <RUN_DIR|PATCH_PATH> <DATA_DIR> <REPO_ID> [BRANCH]" >&2
  exit 1
fi

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
INPUT="$1"
DATA_DIR="$2"
REPO_ID="$3"
BRANCH="${4:-main}"
TOOL_BIN="${EDGERUN_TOOL_BIN:-${ROOT_DIR}/out/tooling/edgertool}"

run_tool() {
  if [[ -x "${TOOL_BIN}" ]]; then
    exec "${TOOL_BIN}" "$@"
  fi
  cd "${ROOT_DIR}/tooling"
  exec go run ./cmd/edgertool "$@"
}

run_tool storage-proposal-submit \
  --input "${INPUT}" \
  --data-dir "${DATA_DIR}" \
  --repo-id "${REPO_ID}" \
  --branch "${BRANCH}" \
  --vfs-operator-bin "${VFS_OPERATOR_BIN:-/var/cache/build/rust/target/release/vfs_operator}"
