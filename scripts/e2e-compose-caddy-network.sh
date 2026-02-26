#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${ROOT_DIR}"

KEEP_STACK="${KEEP_STACK:-0}"

cleanup() {
  "${ROOT_DIR}/scripts/caddy/set-scenario.sh" healthy || true
  if [[ "${KEEP_STACK}" != "1" ]]; then
    docker compose down -v
  fi
}
trap cleanup EXIT

wait_for_url() {
  local url="$1"
  local name="$2"
  local attempts="${3:-90}"
  local sleep_s="${4:-1}"
  for _ in $(seq 1 "${attempts}"); do
    if curl -fsS "${url}" >/dev/null 2>&1; then
      return 0
    fi
    sleep "${sleep_s}"
  done
  echo "Timed out waiting for ${name} at ${url}" >&2
  return 1
}

echo "[compose-caddy] building service binaries"
"${ROOT_DIR}/scripts/build-docker-binaries.sh"

echo "[compose-caddy] building frontend"
cd "${ROOT_DIR}/frontend"
bun run build
cd "${ROOT_DIR}"

echo "[compose-caddy] starting stack"
docker compose up -d scheduler term-server worker frontend caddy
"${ROOT_DIR}/scripts/caddy/set-scenario.sh" healthy

echo "[compose-caddy] waiting for caddy-routed services"
wait_for_url "http://127.0.0.1:9000/" "caddy frontend"
wait_for_url "http://127.0.0.1:9000/v1/control/ws?client_id=e2e" "scheduler via caddy"
wait_for_url "http://127.0.0.1:9000/v1/device/identity" "term-server via caddy"

echo "[compose-caddy] running cypress network condition spec"
cd "${ROOT_DIR}/frontend"
bunx --bun cypress run --config-file cypress.config.ts --browser electron \
  --spec cypress/e2e/terminal-caddy-network-conditions.cy.js
