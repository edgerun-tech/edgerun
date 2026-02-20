#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TMP_DIR="$(mktemp -d)"
SCHED_LOG="$TMP_DIR/scheduler.log"
WORKER_LOG="$TMP_DIR/worker.log"
SCHED_DATA="$TMP_DIR/scheduler-data"
WORKER_PUBKEY="worker-abi-rollover"

SCHED_PORT="$(python3 - <<'PY'
import socket
s = socket.socket()
s.bind(("127.0.0.1", 0))
print(s.getsockname()[1])
s.close()
PY
)"
SCHED_ADDR="127.0.0.1:${SCHED_PORT}"
SCHED_URL="http://${SCHED_ADDR}"

cleanup() {
  for pid_var in WORKER_PID SCHED_PID; do
    local pid="${!pid_var:-}"
    if [[ -n "$pid" ]] && kill -0 "$pid" 2>/dev/null; then
      kill "$pid" 2>/dev/null || true
      wait "$pid" 2>/dev/null || true
    fi
  done
  rm -rf "$TMP_DIR"
}
trap cleanup EXIT

mkdir -p "$SCHED_DATA"

echo "starting scheduler..."
(
  cd "$ROOT_DIR"
  EDGERUN_SCHEDULER_DATA_DIR="$SCHED_DATA" \
  EDGERUN_SCHEDULER_ADDR="$SCHED_ADDR" \
  cargo run -p edgerun-scheduler >"$SCHED_LOG" 2>&1
) &
SCHED_PID=$!

for _ in $(seq 1 240); do
  if curl -fsS "$SCHED_URL/health" >/dev/null 2>&1; then
    break
  fi
  sleep 0.5
done
curl -fsS "$SCHED_URL/health" >/dev/null

echo "starting worker..."
(
  cd "$ROOT_DIR"
  EDGERUN_WORKER_PUBKEY="$WORKER_PUBKEY" \
  EDGERUN_SCHEDULER_URL="$SCHED_URL" \
  cargo run -p edgerun-worker >"$WORKER_LOG" 2>&1
) &
WORKER_PID=$!

create_job() {
  local abi_version="$1"
  local response
  response="$(curl -fsS -X POST "$SCHED_URL/v1/job/create" \
    -H "content-type: application/json" \
    -d '{"runtime_id":"0000000000000000000000000000000000000000000000000000000000000000","abi_version":'"$abi_version"',"wasm_base64":"AA==","input_base64":"","limits":{"max_memory_bytes":1048576,"max_instructions":10000},"escrow_lamports":1,"assignment_worker_pubkey":"'"$WORKER_PUBKEY"'"}')"
  python3 - "$response" <<'PY'
import json, sys
print(json.loads(sys.argv[1])["job_id"])
PY
}

wait_for_runtime_execute_failure() {
  local job_id="$1"
  local status_json=""
  for _ in $(seq 1 240); do
    status_json="$(curl -fsS "$SCHED_URL/v1/job/$job_id")"
    if python3 - "$status_json" <<'PY'
import json, sys
fails = json.loads(sys.argv[1]).get("failures", [])
if not fails:
    raise SystemExit(1)
phase = fails[-1].get("phase", "")
raise SystemExit(0 if phase == "runtime_execute" else 1)
PY
    then
      return 0
    fi
    sleep 0.5
  done
  echo "timed out waiting for runtime_execute failure for job $job_id"
  echo "scheduler log:" && cat "$SCHED_LOG" || true
  echo "worker log:" && cat "$WORKER_LOG" || true
  exit 1
}

echo "creating N-1 ABI job (abi_version=1)..."
job_v1="$(create_job 1)"
wait_for_runtime_execute_failure "$job_v1"

echo "creating current ABI job (abi_version=2)..."
job_v2="$(create_job 2)"
wait_for_runtime_execute_failure "$job_v2"

echo "verifying scheduler rejects unsupported ABI (abi_version=3)..."
status="$(curl -s -o "$TMP_DIR/unsupported.json" -w '%{http_code}' -X POST "$SCHED_URL/v1/job/create" \
  -H "content-type: application/json" \
  -d '{"runtime_id":"0000000000000000000000000000000000000000000000000000000000000000","abi_version":3,"wasm_base64":"AA==","input_base64":"","limits":{"max_memory_bytes":1048576,"max_instructions":10000},"escrow_lamports":1,"assignment_worker_pubkey":"'"$WORKER_PUBKEY"'"}')"
if [[ "$status" != "400" ]]; then
  echo "expected HTTP 400 for unsupported abi_version, got $status"
  cat "$TMP_DIR/unsupported.json" || true
  exit 1
fi

echo "integration ABI rollover test passed"
