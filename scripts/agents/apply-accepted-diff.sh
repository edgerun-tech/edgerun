#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
set -euo pipefail

usage() {
  echo "Usage:" >&2
  echo "  scripts/agents/apply-accepted-diff.sh <RUN_DIR|PATCH_PATH>" >&2
  echo "  scripts/agents/apply-accepted-diff.sh --apply <RUN_DIR|PATCH_PATH>" >&2
}

APPLY=0
if [[ "${1:-}" == "--apply" ]]; then
  APPLY=1
  shift
fi
if [[ $# -lt 1 ]]; then
  usage
  exit 1
fi

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
input="$1"
TOOL_BIN="${EDGERUN_TOOL_BIN:-${ROOT_DIR}/out/tooling/edgertool}"

run_tool() {
  if [[ -x "${TOOL_BIN}" ]]; then
    exec "${TOOL_BIN}" "$@"
  fi
  cd "${ROOT_DIR}/tooling"
  exec go run ./cmd/edgertool "$@"
}

if [[ ${APPLY} -eq 1 ]]; then
  run_tool agent-diff-accept \
    --input "${input}" \
    --apply \
    --repo-root "${ROOT_DIR}" \
    --nats-url "${EDGERUN_EVENTBUS_NATS_URL:-nats://127.0.0.1:4222}" \
    --subject "${AGENT_DIFF_ACCEPTED_SUBJECT:-edgerun.agents.diff.accepted}"
fi

run_tool agent-diff-accept \
  --input "${input}" \
  --repo-root "${ROOT_DIR}" \
  --nats-url "${EDGERUN_EVENTBUS_NATS_URL:-nats://127.0.0.1:4222}" \
  --subject "${AGENT_DIFF_ACCEPTED_SUBJECT:-edgerun.agents.diff.accepted}"
