#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
set -euo pipefail

if [[ $# -lt 2 ]]; then
  echo "Usage: scripts/agents/mcp-tool-call.sh <TOOL_NAME> <JSON_ARGS>" >&2
  exit 1
fi

TOOL_NAME="$1"
JSON_ARGS="$2"
MCP_URL="${MCP_SYSCALL_URL:-http://127.0.0.1:7047}"
RPC_ID="$(date +%s%N)"

payload="$(cat <<JSON
{"jsonrpc":"2.0","id":${RPC_ID},"method":"tools/call","params":{"name":"${TOOL_NAME}","arguments":${JSON_ARGS}}}
JSON
)"

curl --silent --show-error --fail \
  -H 'content-type: application/json' \
  -d "${payload}" \
  "${MCP_URL}/message"
