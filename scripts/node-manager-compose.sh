#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
COMPOSE_FILE="${ROOT_DIR}/docker-compose.node-manager.yml"
ENV_FILE_DEFAULT="${ROOT_DIR}/config/node-manager.compose.env"
ENV_FILE="${EDGERUN_NODE_MANAGER_COMPOSE_ENV_FILE:-${ENV_FILE_DEFAULT}}"

usage() {
  cat <<'USAGE'
Usage:
  scripts/node-manager-compose.sh up
  scripts/node-manager-compose.sh up-tunnel
  scripts/node-manager-compose.sh down
  scripts/node-manager-compose.sh logs
  scripts/node-manager-compose.sh logs-tunnel
  scripts/node-manager-compose.sh pair <PAIRING_CODE>
  scripts/node-manager-compose.sh shell

Environment:
  EDGERUN_NODE_MANAGER_COMPOSE_ENV_FILE  Override env file path
USAGE
}

compose() {
  if [[ -f "${ENV_FILE}" ]]; then
    docker compose --env-file "${ENV_FILE}" -f "${COMPOSE_FILE}" "$@"
  else
    docker compose -f "${COMPOSE_FILE}" "$@"
  fi
}

cmd="${1:-}"
case "${cmd}" in
  up)
    compose up -d --build
    ;;
  up-tunnel)
    compose --profile tunnel up -d --build
    ;;
  down)
    compose down
    ;;
  logs)
    compose logs -f node-manager
    ;;
  logs-tunnel)
    compose logs -f cloudflared
    ;;
  pair)
    shift || true
    pairing_code="${1:-}"
    if [[ -z "${pairing_code}" ]]; then
      echo "missing pairing code" >&2
      usage >&2
      exit 1
    fi
    compose exec node-manager \
      edgerun-node-manager tunnel-connect \
      --relay-control-base https://relay.edgerun.tech \
      --pairing-code "${pairing_code}"
    ;;
  shell)
    compose exec node-manager /bin/bash
    ;;
  *)
    usage
    exit 1
    ;;
esac
