#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

cleanup() {
  for pid in "${PIDS[@]:-}"; do
    if [[ -n "${pid:-}" ]] && kill -0 "$pid" >/dev/null 2>&1; then
      kill "$pid" >/dev/null 2>&1 || true
      wait "$pid" >/dev/null 2>&1 || true
    fi
  done
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

require_free_port() {
  local port="$1"
  if ss -ltn | grep -q ":${port}[[:space:]]"; then
    echo "Port ${port} is already in use; stop conflicting service and retry" >&2
    exit 1
  fi
}

require_free_port 8090
require_free_port 8080
require_free_port 4173

if [[ ! -f out/frontend/site/index.html ]]; then
  echo "Missing build output: out/frontend/site/index.html" >&2
  echo "Run: cd frontend && bun run build" >&2
  exit 1
fi

PIDS=()
LOG_DIR="${TMPDIR:-/tmp}/edgerun-e2e"
mkdir -p "$LOG_DIR"

echo "[e2e-local] starting scheduler on :8090"
EDGERUN_SCHEDULER_ADDR=127.0.0.1:8090 \
EDGERUN_SCHEDULER_BASE_URL=http://127.0.0.1:8090 \
EDGERUN_SCHEDULER_DATA_DIR=.edgerun-scheduler-data/e2e-local \
EDGERUN_SCHEDULER_REQUIRE_CHAIN_CONTEXT=false \
EDGERUN_SCHEDULER_REQUIRE_POLICY_SESSION=false \
EDGERUN_SCHEDULER_REQUIRE_WORKER_SIGNATURES=false \
EDGERUN_SCHEDULER_REQUIRE_RESULT_ATTESTATION=false \
EDGERUN_SCHEDULER_QUORUM_REQUIRES_ATTESTATION=false \
cargo run -p edgerun-scheduler >"${LOG_DIR}/scheduler.log" 2>&1 &
PIDS+=("$!")

echo "[e2e-local] starting term-server on :8080"
EDGERUN_HARDWARE_MODE=allow-software \
EDGERUN_TERM_SERVER_ADDR=127.0.0.1:8080 \
EDGERUN_ROUTE_CONTROL_BASE=http://127.0.0.1:8090 \
EDGERUN_TERM_PUBLIC_BASE_URL=http://127.0.0.1:8080 \
cargo run -p edgerun-term-server >"${LOG_DIR}/term-server.log" 2>&1 &
PIDS+=("$!")

echo "[e2e-local] starting static frontend on :4173"
bunx --bun serve out/frontend/site -p 4173 --no-port-switching >"${LOG_DIR}/frontend-serve.log" 2>&1 &
PIDS+=("$!")

echo "[e2e-local] waiting for services"
wait_for_url "http://127.0.0.1:8090/health" "scheduler health"
wait_for_url "http://127.0.0.1:8080/v1/device/identity" "term-server identity"
wait_for_url "http://127.0.0.1:4173/" "frontend"

device_id="$(curl -fsS http://127.0.0.1:8080/v1/device/identity | jq -r '.device_pubkey_b64url')"
if [[ -z "$device_id" || "$device_id" == "null" ]]; then
  echo "Failed to get device_id from term-server identity endpoint" >&2
  exit 1
fi

echo "[e2e-local] waiting for scheduler route resolve for device ${device_id}"
for _ in $(seq 1 90); do
  if curl -fsS "http://127.0.0.1:8090/v1/route/resolve/${device_id}" | jq -e '.ok == true and .found == true' >/dev/null 2>&1; then
    break
  fi
  sleep 1
done
curl -fsS "http://127.0.0.1:8090/v1/route/resolve/${device_id}" | jq -e '.ok == true and .found == true' >/dev/null

echo "[e2e-local] running cypress terminal compose spec"
cd frontend
bunx --bun cypress run --config-file cypress.config.ts --browser electron --spec cypress/e2e/terminal-compose-stack.cy.js
