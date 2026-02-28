#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
set -euo pipefail

if [[ $# -lt 1 ]]; then
  echo "Usage: scripts/agents/emit-diff-events.sh <RUN_DIR>" >&2
  exit 1
fi

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
run_dir="$1"
TOOL_BIN="${EDGERUN_TOOL_BIN:-${ROOT_DIR}/out/tooling/edgertool}"

if [[ -x "${TOOL_BIN}" ]]; then
  exec "${TOOL_BIN}" agent-diff-proposed \
    --run-dir "${run_dir}" \
    --repo-root "${ROOT_DIR}" \
    --nats-url "${EDGERUN_EVENTBUS_NATS_URL:-nats://127.0.0.1:4222}"
fi

cd "${ROOT_DIR}/tooling"

exec go run ./cmd/edgertool agent-diff-proposed \
  --run-dir "${run_dir}" \
  --repo-root "${ROOT_DIR}" \
  --nats-url "${EDGERUN_EVENTBUS_NATS_URL:-nats://127.0.0.1:4222}"
