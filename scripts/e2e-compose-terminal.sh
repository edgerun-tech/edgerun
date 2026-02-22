#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

KEEP_STACK="${KEEP_STACK:-0}"

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

echo "[compose] building and starting stack"
docker compose up -d --build scheduler term-server frontend

echo "[compose] waiting for services"
wait_for_url "http://127.0.0.1:8090/health" "scheduler health"
wait_for_url "http://127.0.0.1:8080/v1/device/identity" "term-server identity"
wait_for_url "http://127.0.0.1:4173/" "frontend"

device_id="$(curl -fsS http://127.0.0.1:8080/v1/device/identity | jq -r '.device_pubkey_b64url')"
if [[ -z "$device_id" || "$device_id" == "null" ]]; then
  echo "Failed to get device_id from term-server identity endpoint" >&2
  exit 1
fi

echo "[compose] waiting for scheduler route resolve for device ${device_id}"
for _ in $(seq 1 90); do
  if curl -fsS "http://127.0.0.1:8090/v1/route/resolve/${device_id}" | jq -e '.ok == true and .found == true' >/dev/null 2>&1; then
    break
  fi
  sleep 1
done
curl -fsS "http://127.0.0.1:8090/v1/route/resolve/${device_id}" | jq -e '.ok == true and .found == true' >/dev/null

echo "[e2e] running cypress terminal compose spec"
cd frontend
bunx --bun cypress run --config-file cypress.config.ts --browser electron --spec cypress/e2e/terminal-compose-stack.cy.js
