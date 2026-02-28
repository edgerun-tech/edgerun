#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
set -euo pipefail

if [[ $# -lt 2 ]]; then
  echo "Usage: scripts/executors/nats-pub.sh <SUBJECT> <JSON_PAYLOAD>" >&2
  exit 1
fi

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
subject="$1"
payload="$2"
TOOL_BIN="${EDGERUN_TOOL_BIN:-${ROOT_DIR}/out/tooling/edgertool}"

if [[ -x "${TOOL_BIN}" ]]; then
  exec "${TOOL_BIN}" nats-pub \
    --subject "${subject}" \
    --payload "${payload}" \
    --nats-url "${EDGERUN_EVENTBUS_NATS_URL:-nats://127.0.0.1:4222}"
fi

cd "${ROOT_DIR}/tooling"

exec go run ./cmd/edgertool nats-pub \
  --subject "${subject}" \
  --payload "${payload}" \
  --nats-url "${EDGERUN_EVENTBUS_NATS_URL:-nats://127.0.0.1:4222}"
