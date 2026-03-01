#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
set -euo pipefail

if [[ $# -lt 4 ]]; then
  echo "Usage: scripts/agents/storage-proposal-apply.sh <DATA_DIR> <REPO_ID> <BRANCH> <PROPOSAL_ID> [--dry-run] [--repo-root PATH] [--fmt-cmd CMD] [--check-cmd CMD] [--timeout-secs N]" >&2
  exit 1
fi

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
DATA_DIR="$1"
REPO_ID="$2"
BRANCH="$3"
PROPOSAL_ID="$4"
shift 4
DRY_RUN=0
REPO_ROOT="${ROOT_DIR}"
FMT_CMD="${STORAGE_PROPOSAL_FMT_CMD:-cargo fmt --all}"
CHECK_CMD="${STORAGE_PROPOSAL_CHECK_CMD:-cargo check -p edgerun-storage}"
TIMEOUT_SECS="${STORAGE_PROPOSAL_TIMEOUT_SECS:-300}"
TOOL_BIN="${EDGERUN_TOOL_BIN:-${ROOT_DIR}/out/tooling/edgertool}"

while [[ $# -gt 0 ]]; do
  case "$1" in
    --dry-run)
      DRY_RUN=1
      shift
      ;;
    --repo-root)
      if [[ $# -lt 2 ]]; then
        echo "missing value for --repo-root" >&2
        exit 1
      fi
      REPO_ROOT="$2"
      shift 2
      ;;
    --fmt-cmd)
      if [[ $# -lt 2 ]]; then
        echo "missing value for --fmt-cmd" >&2
        exit 1
      fi
      FMT_CMD="$2"
      shift 2
      ;;
    --check-cmd)
      if [[ $# -lt 2 ]]; then
        echo "missing value for --check-cmd" >&2
        exit 1
      fi
      CHECK_CMD="$2"
      shift 2
      ;;
    --timeout-secs)
      if [[ $# -lt 2 ]]; then
        echo "missing value for --timeout-secs" >&2
        exit 1
      fi
      TIMEOUT_SECS="$2"
      shift 2
      ;;
    *)
      echo "unknown argument: $1" >&2
      exit 1
      ;;
  esac
done

run_tool() {
  if [[ -x "${TOOL_BIN}" ]]; then
    exec "${TOOL_BIN}" "$@"
  fi
  cd "${ROOT_DIR}/tooling"
  exec go run ./cmd/edgertool "$@"
}

args=(
  storage-proposal-apply
  --data-dir "${DATA_DIR}"
  --repo-id "${REPO_ID}"
  --branch "${BRANCH}"
  --proposal-id "${PROPOSAL_ID}"
  --repo-root "${REPO_ROOT}"
  --fmt-cmd "${FMT_CMD}"
  --check-cmd "${CHECK_CMD}"
  --timeout-secs "${TIMEOUT_SECS}"
  --proposal-gatekeeper-bin "${PROPOSAL_GATEKEEPER_BIN:-/var/cache/build/rust/target/release/proposal_gatekeeper}"
)

if [[ ${DRY_RUN} -eq 1 ]]; then
  args+=(--dry-run)
fi

run_tool "${args[@]}"
