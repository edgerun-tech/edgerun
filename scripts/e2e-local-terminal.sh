#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

SCHEDULER_PORT="${EDGERUN_E2E_SCHEDULER_PORT:-8090}"
TERM_SERVER_PORT="${EDGERUN_E2E_TERM_SERVER_PORT:-8081}"
FRONTEND_PORT="${EDGERUN_E2E_FRONTEND_PORT:-4173}"
ROUTE_OWNER_PUBKEY="${EDGERUN_E2E_ROUTE_OWNER_PUBKEY:-}"
SCHEDULER_BASE="http://127.0.0.1:${SCHEDULER_PORT}"
TERM_SERVER_BASE="http://127.0.0.1:${TERM_SERVER_PORT}"

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

require_free_port() {
  local port="$1"
  if command -v ss >/dev/null 2>&1; then
    if ss -ltn | grep -q ":${port}[[:space:]]"; then
      echo "Port ${port} is already in use; stop conflicting service and retry" >&2
      exit 1
    fi
    return
  fi
  if command -v lsof >/dev/null 2>&1; then
    if lsof -iTCP:"${port}" -sTCP:LISTEN -Pn >/dev/null 2>&1; then
      echo "Port ${port} is already in use; stop conflicting service and retry" >&2
      exit 1
    fi
    return
  fi
  if command -v netstat >/dev/null 2>&1; then
    if netstat -an 2>/dev/null | grep -E "[:\\.]${port}[[:space:]].*LISTEN" >/dev/null 2>&1; then
      echo "Port ${port} is already in use; stop conflicting service and retry" >&2
      exit 1
    fi
    return
  fi
  python3 - <<PY
import socket, sys
port = int("${port}")
s = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
try:
    s.bind(("127.0.0.1", port))
except OSError:
    sys.exit(1)
finally:
    s.close()
PY
  if [[ "$?" -ne 0 ]]; then
    echo "Port ${port} is already in use; stop conflicting service and retry" >&2
    exit 1
  fi
}

require_free_port "${SCHEDULER_PORT}"
require_free_port "${TERM_SERVER_PORT}"
require_free_port "${FRONTEND_PORT}"

if [[ ! -f out/frontend/site/index.html ]]; then
  echo "Missing build output: out/frontend/site/index.html" >&2
  echo "Run: cd frontend && bun run build" >&2
  exit 1
fi

PIDS=()
LOG_DIR="${EDGERUN_E2E_LOG_DIR:-out/e2e/local-$(date +%Y%m%d-%H%M%S)-$$}"
mkdir -p "$LOG_DIR"

echo "[e2e-local] starting scheduler on :${SCHEDULER_PORT}"
EDGERUN_SCHEDULER_ADDR=127.0.0.1:${SCHEDULER_PORT} \
EDGERUN_SCHEDULER_BASE_URL=${SCHEDULER_BASE} \
EDGERUN_SCHEDULER_DATA_DIR=.edgerun-scheduler-data/e2e-local \
EDGERUN_SCHEDULER_REQUIRE_CHAIN_CONTEXT=false \
EDGERUN_SCHEDULER_REQUIRE_POLICY_SESSION=false \
EDGERUN_SCHEDULER_REQUIRE_WORKER_SIGNATURES=false \
EDGERUN_SCHEDULER_REQUIRE_RESULT_ATTESTATION=false \
EDGERUN_SCHEDULER_QUORUM_REQUIRES_ATTESTATION=false \
cargo run -p edgerun-scheduler >"${LOG_DIR}/scheduler.log" 2>&1 &
PIDS+=("$!")

echo "[e2e-local] starting term-server on :${TERM_SERVER_PORT}"
EDGERUN_HARDWARE_MODE=allow-software \
EDGERUN_TERM_SERVER_ADDR=127.0.0.1:${TERM_SERVER_PORT} \
EDGERUN_ROUTE_CONTROL_BASE=${SCHEDULER_BASE} \
EDGERUN_ROUTE_OWNER_PUBKEY=${ROUTE_OWNER_PUBKEY} \
EDGERUN_TERM_PUBLIC_BASE_URL=${TERM_SERVER_BASE} \
cargo run -p edgerun-term-server >"${LOG_DIR}/term-server.log" 2>&1 &
PIDS+=("$!")

echo "[e2e-local] starting static frontend on :${FRONTEND_PORT}"
bunx --bun serve out/frontend/site -p "${FRONTEND_PORT}" --no-port-switching >"${LOG_DIR}/frontend-serve.log" 2>&1 &
PIDS+=("$!")

echo "[e2e-local] waiting for services"
wait_for_http "${SCHEDULER_BASE}/v1/control/ws" "scheduler control ws endpoint"
wait_for_url "${TERM_SERVER_BASE}/v1/device/identity" "term-server identity"
wait_for_url "http://127.0.0.1:${FRONTEND_PORT}/" "frontend"

device_id="$(curl -fsS "${TERM_SERVER_BASE}/v1/device/identity" | jq -r '.device_pubkey_b64url')"
if [[ -z "$device_id" || "$device_id" == "null" ]]; then
  echo "Failed to get device_id from term-server identity endpoint" >&2
  exit 1
fi

echo "[e2e-local] running cypress terminal compose spec"
cd frontend
bunx --bun cypress run --config-file cypress.config.ts --browser electron \
  --spec cypress/e2e/terminal-compose-stack.cy.js
