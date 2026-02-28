#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
set -euo pipefail

if [[ $# -lt 2 ]]; then
  echo "Usage: scripts/agents/launch-agent.sh <AGENT_ID> <PROMPT...>" >&2
  exit 1
fi

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
agent_id="$1"
shift
prompt="$*"
TOOL_BIN="${EDGERUN_TOOL_BIN:-${ROOT_DIR}/out/tooling/edgertool}"

if [[ -x "${TOOL_BIN}" ]]; then
  exec "${TOOL_BIN}" agent-launch \
    --agent-id "${agent_id}" \
    --prompt "${prompt}" \
    --repo-root "${ROOT_DIR}" \
    --nats-url "${EDGERUN_EVENTBUS_NATS_URL:-nats://127.0.0.1:4222}" \
    --mcp-syscall-url "${MCP_SYSCALL_URL:-http://127.0.0.1:7047}"
fi

cd "${ROOT_DIR}/tooling"
exec go run ./cmd/edgertool agent-launch \
  --agent-id "${agent_id}" \
  --prompt "${prompt}" \
  --repo-root "${ROOT_DIR}" \
  --nats-url "${EDGERUN_EVENTBUS_NATS_URL:-nats://127.0.0.1:4222}" \
  --mcp-syscall-url "${MCP_SYSCALL_URL:-http://127.0.0.1:7047}"
