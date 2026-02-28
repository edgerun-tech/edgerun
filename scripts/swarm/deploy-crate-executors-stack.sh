#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
STACK_NAME="${EDGERUN_CRATE_EXECUTORS_STACK_NAME:-edgerun-crate-executors}"
STACK_FILE="${EDGERUN_CRATE_EXECUTORS_STACK_FILE:-${ROOT_DIR}/out/swarm/crate-executors.stack.yml}"

"${ROOT_DIR}/scripts/swarm/generate-crate-executors-stack.sh" "${STACK_FILE}"
echo "using NATS URL: ${EDGERUN_EVENTBUS_NATS_URL:-auto-detected in generator}"

docker stack deploy -c "${STACK_FILE}" "${STACK_NAME}"

echo "stack services:"
docker stack services "${STACK_NAME}"
