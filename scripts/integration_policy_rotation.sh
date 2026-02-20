#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TMP_DIR="$(mktemp -d)"
SCHED_LOG="$TMP_DIR/scheduler.log"
WORKER_A_LOG="$TMP_DIR/worker-a.log"
WORKER_B_LOG="$TMP_DIR/worker-b.log"
SCHED_DATA="$TMP_DIR/scheduler-data"
KEY2_HEX="0202020202020202020202020202020202020202020202020202020202020202"
KEY2_ID="rot-key-2"
KEY2_VER="2"
WORKER_A="worker-rot-a"
WORKER_B="worker-rot-b"

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
  for pid_var in WORKER_B_PID WORKER_A_PID SCHED_PID; do
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

echo "starting scheduler with rotated key..."
(
  cd "$ROOT_DIR"
  EDGERUN_SCHEDULER_DATA_DIR="$SCHED_DATA" \
  EDGERUN_SCHEDULER_ADDR="$SCHED_ADDR" \
  EDGERUN_SCHEDULER_POLICY_SIGNING_KEY_HEX="$KEY2_HEX" \
  EDGERUN_SCHEDULER_POLICY_KEY_ID="$KEY2_ID" \
  EDGERUN_SCHEDULER_POLICY_VERSION="$KEY2_VER" \
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

policy_json="$(curl -fsS "$SCHED_URL/v1/policy/info")"
KEY2_PUB="$(python3 - "$policy_json" <<'PY'
import json, sys
print(json.loads(sys.argv[1])["signer_pubkey"])
PY
)"

echo "phase 1: worker accepts next key tuple during overlap..."
(
  cd "$ROOT_DIR"
  EDGERUN_WORKER_PUBKEY="$WORKER_A" \
  EDGERUN_SCHEDULER_URL="$SCHED_URL" \
  EDGERUN_WORKER_POLICY_KEY_ID_NEXT="$KEY2_ID" \
  EDGERUN_WORKER_POLICY_VERSION_NEXT="$KEY2_VER" \
  EDGERUN_WORKER_POLICY_VERIFY_KEY_HEX_NEXT="$KEY2_PUB" \
  cargo run -p edgerun-worker >"$WORKER_A_LOG" 2>&1
) &
WORKER_A_PID=$!

create_job() {
  local worker_pubkey="$1"
  local response
  response="$(curl -fsS -X POST "$SCHED_URL/v1/job/create" \
    -H "content-type: application/json" \
    -d '{"runtime_id":"0000000000000000000000000000000000000000000000000000000000000000","wasm_base64":"AA==","input_base64":"","limits":{"max_memory_bytes":1048576,"max_instructions":10000},"escrow_lamports":1,"assignment_worker_pubkey":"'"$worker_pubkey"'"}')"
  python3 - "$response" <<'PY'
import json, sys
print(json.loads(sys.argv[1])["job_id"])
PY
}

wait_for_failure_phase() {
  local job_id="$1"
  local expected_phase="$2"
  local invert="${3:-0}" # 1 means phase must NOT equal expected_phase
  local status_json=""
  for _ in $(seq 1 240); do
    status_json="$(curl -fsS "$SCHED_URL/v1/job/$job_id")"
    if python3 - "$status_json" "$expected_phase" "$invert" <<'PY'
import json, sys
doc = json.loads(sys.argv[1])
want = sys.argv[2]
invert = sys.argv[3] == "1"
fails = doc.get("failures", [])
if not fails:
    raise SystemExit(1)
phase = fails[-1].get("phase", "")
ok = (phase != want) if invert else (phase == want)
raise SystemExit(0 if ok else 1)
PY
    then
      return 0
    fi
    sleep 0.5
  done
  echo "timed out waiting for failure phase condition on job $job_id"
  echo "scheduler log:"
  cat "$SCHED_LOG" || true
  echo "worker A log:"
  cat "$WORKER_A_LOG" || true
  echo "worker B log:"
  cat "$WORKER_B_LOG" || true
  exit 1
}

job1="$(create_job "$WORKER_A")"
wait_for_failure_phase "$job1" "assignment_policy_verify" 1
echo "phase 1 passed (assignment accepted with next key)"

kill "$WORKER_A_PID" 2>/dev/null || true
wait "$WORKER_A_PID" 2>/dev/null || true

echo "phase 2: worker rejects rotated key after overlap removed..."
(
  cd "$ROOT_DIR"
  EDGERUN_WORKER_PUBKEY="$WORKER_B" \
  EDGERUN_SCHEDULER_URL="$SCHED_URL" \
  cargo run -p edgerun-worker >"$WORKER_B_LOG" 2>&1
) &
WORKER_B_PID=$!

job2="$(create_job "$WORKER_B")"
wait_for_failure_phase "$job2" "assignment_policy_verify" 0
echo "phase 2 passed (assignment rejected without next key)"

echo "integration policy rotation test passed"
