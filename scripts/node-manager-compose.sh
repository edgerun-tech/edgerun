#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
COMPOSE_FILE="${ROOT_DIR}/docker-compose.node-manager.yml"
ENV_FILE_DEFAULT="${ROOT_DIR}/config/node-manager.compose.env"
ENV_FILE="${EDGERUN_NODE_MANAGER_COMPOSE_ENV_FILE:-${ENV_FILE_DEFAULT}}"
PREPARE_BINARIES_SCRIPT="${ROOT_DIR}/scripts/prepare-node-manager-image-binaries.sh"
VERIFY_LOCAL_STACK_SCRIPT="${ROOT_DIR}/scripts/verify-local-stack.sh"
TUNNEL_CONFIG_FILE="${ROOT_DIR}/config/cloudflared/node-manager-tunnel.yml"
TUNNEL_CREDENTIALS_FILE="${ROOT_DIR}/config/cloudflared/node-manager-tunnel-credentials.json"

usage() {
  cat <<'USAGE'
Usage:
  scripts/node-manager-compose.sh up
  scripts/node-manager-compose.sh up-dev
  scripts/node-manager-compose.sh up-tunnel
  scripts/node-manager-compose.sh up-tunnel-verify
  scripts/node-manager-compose.sh status
  scripts/node-manager-compose.sh ps
  scripts/node-manager-compose.sh prepare-binaries
  scripts/node-manager-compose.sh down
  scripts/node-manager-compose.sh logs [SERVICE]
  scripts/node-manager-compose.sh logs-tunnel
  scripts/node-manager-compose.sh logs-nats
  scripts/node-manager-compose.sh logs-mcp
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
  prepare-binaries)
    "${PREPARE_BINARIES_SCRIPT}"
    ;;
  up)
    "${PREPARE_BINARIES_SCRIPT}"
    compose up -d --build
    ;;
  up-dev)
    if [[ ! -f "${TUNNEL_CONFIG_FILE}" ]]; then
      echo "missing tunnel config: ${TUNNEL_CONFIG_FILE}" >&2
      exit 1
    fi
    if [[ ! -f "${TUNNEL_CREDENTIALS_FILE}" ]]; then
      echo "missing tunnel credentials: ${TUNNEL_CREDENTIALS_FILE}" >&2
      exit 1
    fi
    "${PREPARE_BINARIES_SCRIPT}"
    compose --profile tunnel up -d --build
    ;;
  up-tunnel)
    if [[ ! -f "${TUNNEL_CONFIG_FILE}" ]]; then
      echo "missing tunnel config: ${TUNNEL_CONFIG_FILE}" >&2
      exit 1
    fi
    if [[ ! -f "${TUNNEL_CREDENTIALS_FILE}" ]]; then
      echo "missing tunnel credentials: ${TUNNEL_CREDENTIALS_FILE}" >&2
      exit 1
    fi
    "${PREPARE_BINARIES_SCRIPT}"
    compose --profile tunnel up -d --build
    ;;
  up-tunnel-verify)
    if [[ ! -f "${TUNNEL_CONFIG_FILE}" ]]; then
      echo "missing tunnel config: ${TUNNEL_CONFIG_FILE}" >&2
      exit 1
    fi
    if [[ ! -f "${TUNNEL_CREDENTIALS_FILE}" ]]; then
      echo "missing tunnel credentials: ${TUNNEL_CREDENTIALS_FILE}" >&2
      exit 1
    fi
    "${PREPARE_BINARIES_SCRIPT}"
    compose --profile tunnel up -d --build
    "${VERIFY_LOCAL_STACK_SCRIPT}"
    ;;
  status)
    compose ps
    if command -v curl >/dev/null 2>&1; then
      if curl --silent --show-error --fail --max-time 2 \
        "http://${EDGERUN_LOCAL_BRIDGE_LISTEN:-127.0.0.1:7777}/v1/local/node/info.pb" \
        >/dev/null; then
        echo "local bridge probe: ok"
      else
        echo "local bridge probe: failed (${EDGERUN_LOCAL_BRIDGE_LISTEN:-127.0.0.1:7777})" >&2
        exit 1
      fi
    else
      echo "curl not found; skipped local bridge probe"
    fi
    ;;
  ps)
    compose ps
    ;;
  down)
    compose down
    ;;
  logs)
    shift || true
    service="${1:-node-manager}"
    compose logs -f "${service}"
    ;;
  logs-tunnel)
    compose logs -f cloudflared
    ;;
  logs-nats)
    compose logs -f nats
    ;;
  logs-mcp)
    compose logs -f mcp-syscall
    ;;
  shell)
    compose exec node-manager /bin/bash
    ;;
  *)
    usage
    exit 1
    ;;
esac
