#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
set -euo pipefail

subject="${1:-edgerun.code.updated}"
revision="${2:-}"
run_id="${3:-}"

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
TOOL_BIN="${EDGERUN_TOOL_BIN:-${ROOT_DIR}/out/tooling/edgertool}"

args=(
  code-update-pub
  --subject "${subject}"
  --repo-root "${ROOT_DIR}"
  --nats-url "${EDGERUN_EVENTBUS_NATS_URL:-nats://127.0.0.1:4222}"
)
if [[ -n "${revision}" ]]; then
  args+=(--revision "${revision}")
fi
if [[ -n "${run_id}" ]]; then
  args+=(--run-id "${run_id}")
fi

if [[ -x "${TOOL_BIN}" ]]; then
  exec "${TOOL_BIN}" "${args[@]}"
fi

cd "${ROOT_DIR}/tooling"
exec go run ./cmd/edgertool "${args[@]}"
