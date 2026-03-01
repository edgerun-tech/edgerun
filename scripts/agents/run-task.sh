#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
set -euo pipefail

if [[ $# -lt 2 ]]; then
  echo "Usage: scripts/agents/run-task.sh <AGENT_ID> <PROMPT...>" >&2
  exit 1
fi

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
AGENT_ID="$1"
shift
PROMPT="$*"

: "${EDGERUN_AGENT_STORAGE_DATA_DIR:=${ROOT_DIR}/out/vfs-ops/storage}"
: "${EDGERUN_AGENT_STORAGE_REPO_ID:=repo-main}"
: "${EDGERUN_AGENT_STORAGE_BRANCH:=main}"
: "${EDGERUN_AGENT_STORAGE_AUTOSUBMIT:=1}"
: "${EDGERUN_AGENT_STORAGE_AUTO_DRY_RUN:=1}"
: "${EDGERUN_AGENT_STORAGE_VFS_OPERATOR_BIN:=/var/cache/build/rust/target/release/vfs_operator}"
: "${EDGERUN_AGENT_STORAGE_GATEKEEPER_BIN:=/var/cache/build/rust/target/release/proposal_gatekeeper}"
: "${EDGERUN_AGENT_STORAGE_FMT_CMD:=cargo fmt --all}"
: "${EDGERUN_AGENT_STORAGE_CHECK_CMD:=cargo check -p edgerun-event-bus}"
: "${EDGERUN_AGENT_STORAGE_TIMEOUT_SECS:=300}"

export EDGERUN_AGENT_STORAGE_DATA_DIR
export EDGERUN_AGENT_STORAGE_REPO_ID
export EDGERUN_AGENT_STORAGE_BRANCH
export EDGERUN_AGENT_STORAGE_AUTOSUBMIT
export EDGERUN_AGENT_STORAGE_AUTO_DRY_RUN
export EDGERUN_AGENT_STORAGE_VFS_OPERATOR_BIN
export EDGERUN_AGENT_STORAGE_GATEKEEPER_BIN
export EDGERUN_AGENT_STORAGE_FMT_CMD
export EDGERUN_AGENT_STORAGE_CHECK_CMD
export EDGERUN_AGENT_STORAGE_TIMEOUT_SECS

require_file() {
  local path="$1"
  local what="$2"
  if [[ ! -e "${path}" ]]; then
    echo "preflight failed: missing ${what} at ${path}" >&2
    exit 1
  fi
}

require_exec() {
  local path="$1"
  local what="$2"
  if [[ ! -x "${path}" ]]; then
    echo "preflight failed: ${what} is not executable at ${path}" >&2
    exit 1
  fi
}

require_file "${ROOT_DIR}/scripts/agents/mcp-tool-call.sh" "mcp tool wrapper"
require_exec "${EDGERUN_AGENT_STORAGE_GATEKEEPER_BIN}" "proposal_gatekeeper"
mkdir -p "${EDGERUN_AGENT_STORAGE_DATA_DIR}"

if ! curl --silent --show-error --fail --max-time 2 "${MCP_SYSCALL_URL:-http://127.0.0.1:7047}/health" >/dev/null; then
  echo "preflight failed: mcp-syscall health check failed at ${MCP_SYSCALL_URL:-http://127.0.0.1:7047}/health" >&2
  exit 1
fi

if ! command -v nc >/dev/null 2>&1; then
  echo "preflight failed: nc is required for NATS reachability check" >&2
  exit 1
fi
if ! nc -z -w2 127.0.0.1 4222 >/dev/null 2>&1; then
  echo "preflight failed: nats is not reachable at 127.0.0.1:4222" >&2
  exit 1
fi

TOOL_BIN="${EDGERUN_TOOL_BIN:-${ROOT_DIR}/out/tooling/edgertool}"
if [[ -x "${TOOL_BIN}" ]]; then
  exec "${TOOL_BIN}" agent-launch \
    --agent-id "${AGENT_ID}" \
    --prompt "${PROMPT}" \
    --repo-root "${ROOT_DIR}"
fi

cd "${ROOT_DIR}/tooling"
exec go run ./cmd/edgertool agent-launch \
  --agent-id "${AGENT_ID}" \
  --prompt "${PROMPT}" \
  --repo-root "${ROOT_DIR}"
