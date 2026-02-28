#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
STACK_NAME="${EDGERUN_SWARM_STACK_NAME:-edgerun-services}"
STACK_FILE="${EDGERUN_SWARM_STACK_FILE:-${ROOT_DIR}/docker-compose.services.yml}"

if ! command -v docker >/dev/null 2>&1; then
  echo "docker command not found" >&2
  exit 1
fi

if [[ ! -f "${STACK_FILE}" ]]; then
  echo "stack compose file not found: ${STACK_FILE}" >&2
  exit 1
fi

state="$(docker info --format '{{.Swarm.LocalNodeState}}' 2>/dev/null || true)"
if [[ "${state}" != "active" ]]; then
  echo "initializing docker swarm on this host"
  docker swarm init >/dev/null
fi

echo "deploying stack ${STACK_NAME} from ${STACK_FILE}"
docker stack deploy -c "${STACK_FILE}" "${STACK_NAME}"

echo "stack services:"
docker stack services "${STACK_NAME}"
