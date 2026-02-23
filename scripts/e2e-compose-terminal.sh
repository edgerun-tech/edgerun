#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

KEEP_STACK="${KEEP_STACK:-0}"
SCHEDULER_PORT="${EDGERUN_E2E_SCHEDULER_PORT:-8090}"
TERM_SERVER_PORT="${EDGERUN_E2E_TERM_SERVER_PORT:-8081}"
FRONTEND_PORT="${EDGERUN_E2E_FRONTEND_PORT:-4173}"

cleanup() {
  if [[ "$KEEP_STACK" != "1" ]]; then
    docker compose down -v
  fi
}
trap cleanup EXIT

wait_for_url() {
  local url="$1"
  local name="$2"
  local attempts="${3:-90}"
  local sleep_s="${4:-1}"
  for _ in $(seq 1 "$attempts"); do
    if curl -fsS "$url" >/dev/null 2>&1; then
      return 0
    fi
    sleep "$sleep_s"
  done
  echo "Timed out waiting for $name at $url" >&2
  return 1
}

wait_for_http() {
  local url="$1"
  local name="$2"
  local attempts="${3:-90}"
  local sleep_s="${4:-1}"
  for _ in $(seq 1 "$attempts"); do
    if curl -sS -o /dev/null "$url" >/dev/null 2>&1; then
      return 0
    fi
    sleep "$sleep_s"
  done
  echo "Timed out waiting for $name at $url" >&2
  return 1
}

echo "[compose] building and starting stack"
docker compose up -d --build scheduler term-server frontend

echo "[compose] waiting for services"
wait_for_http "http://127.0.0.1:${SCHEDULER_PORT}/v1/control/ws" "scheduler control ws endpoint"
wait_for_url "http://127.0.0.1:${TERM_SERVER_PORT}/v1/device/identity" "term-server identity"
wait_for_url "http://127.0.0.1:${FRONTEND_PORT}/" "frontend"

device_id="$(curl -fsS "http://127.0.0.1:${TERM_SERVER_PORT}/v1/device/identity" | jq -r '.device_pubkey_b64url')"
if [[ -z "$device_id" || "$device_id" == "null" ]]; then
  echo "Failed to get device_id from term-server identity endpoint" >&2
  exit 1
fi

echo "[e2e] running cypress terminal compose spec"
cd frontend
bunx --bun cypress run --config-file cypress.config.ts --browser electron \
  --spec cypress/e2e/terminal-compose-stack.cy.js
