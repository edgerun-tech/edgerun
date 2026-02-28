#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
set -euo pipefail

if [[ $# -lt 2 ]]; then
  echo "Usage: scripts/agents/mcp-context.sh <pack|symbols|refs> <path-or-name> [extra-json]" >&2
  exit 1
fi

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
TOOL_WRAP="${ROOT_DIR}/scripts/agents/mcp-tool-call.sh"
MODE="$1"
ARG1="$2"
EXTRA="${3:-{}}"

case "${MODE}" in
  pack)
    ARGS="{\"path\":\"${ARG1}\",\"mode\":\"symbols\",\"context_lines\":40,\"max_bytes\":20000}"
    exec "${TOOL_WRAP}" context_pack "${ARGS}"
    ;;
  symbols)
    ARGS="{\"path\":\"${ARG1}\",\"format\":\"compact\",\"max_symbols\":300}"
    exec "${TOOL_WRAP}" code_symbols "${ARGS}"
    ;;
  refs)
    ARGS="{\"name\":\"${ARG1}\",\"scope\":\"workspace\",\"format\":\"summary\",\"max_results\":300}"
    exec "${TOOL_WRAP}" code_find_refs "${ARGS}"
    ;;
  *)
    echo "unsupported mode: ${MODE}" >&2
    exit 1
    ;;
esac
