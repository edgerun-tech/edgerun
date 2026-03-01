#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
set -euo pipefail

if [[ $# -lt 2 ]]; then
  echo "Usage: scripts/agents/mcp-context.sh <pack|symbols|refs> <path-or-name> [extra-json]" >&2
  exit 1
fi

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "${SCRIPT_DIR}/../.." && pwd)"
TOOL_WRAP="${ROOT_DIR}/scripts/agents/mcp-tool-call.sh"
if [[ ! -x "${TOOL_WRAP}" ]]; then
  # When mounted directly into agent containers at /edgerun-agent-tools.
  TOOL_WRAP="${SCRIPT_DIR}/mcp-tool-call.sh"
fi
if [[ ! -x "${TOOL_WRAP}" ]]; then
  echo "mcp tool wrapper not found: ${TOOL_WRAP}" >&2
  exit 1
fi
MODE="$1"
ARG1="$2"
EXTRA="${3:-{}}"

call_or_fallback() {
  local tool_name="$1"
  local json_args="$2"
  local fallback_mode="$3"
  local out
  if out="$("${TOOL_WRAP}" "${tool_name}" "${json_args}" 2>&1)"; then
    if [[ "${out}" == *'"isError":true'* ]]; then
      fallback_local "${fallback_mode}" "${ARG1}" "${out}"
      return 0
    fi
    echo "${out}"
    return 0
  fi
  fallback_local "${fallback_mode}" "${ARG1}" "${out}"
}

fallback_local() {
  local mode="$1"
  local arg="$2"
  local reason="$3"
  echo "mcp-context fallback mode=${mode} reason=$(printf '%s' "${reason}" | tr '\n' ' ')" >&2
  case "${mode}" in
    pack)
      if [[ -f "${arg}" ]]; then
        sed -n '1,220p' "${arg}"
      elif [[ -d "${arg}" ]]; then
        find "${arg}" -maxdepth 3 -type f | sed -n '1,220p'
      else
        echo "path not found: ${arg}" >&2
        exit 1
      fi
      ;;
    symbols)
      if [[ -f "${arg}" ]]; then
        grep -nE '^(pub )?(fn|struct|enum|trait|impl|class|interface) ' "${arg}" || true
      else
        echo "file not found: ${arg}" >&2
        exit 1
      fi
      ;;
    refs)
      if command -v rg >/dev/null 2>&1; then
        rg -n --no-heading --color never "${arg}" . || true
      else
        grep -RIn --exclude-dir=.git -- "${arg}" . || true
      fi
      ;;
    *)
      echo "unsupported fallback mode: ${mode}" >&2
      exit 1
      ;;
  esac
}

case "${MODE}" in
  pack)
    ARGS="{\"path\":\"${ARG1}\",\"mode\":\"symbols\",\"context_lines\":40,\"max_bytes\":20000}"
    call_or_fallback context_pack "${ARGS}" pack
    ;;
  symbols)
    ARGS="{\"path\":\"${ARG1}\",\"format\":\"compact\",\"max_symbols\":300}"
    call_or_fallback code_symbols "${ARGS}" symbols
    ;;
  refs)
    ARGS="{\"name\":\"${ARG1}\",\"scope\":\"workspace\",\"format\":\"summary\",\"max_results\":300}"
    call_or_fallback code_find_refs "${ARGS}" refs
    ;;
  *)
    echo "unsupported mode: ${MODE}" >&2
    exit 1
    ;;
esac
