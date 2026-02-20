#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TMP_DIR="$(mktemp -d)"
SCHED_LOG="$TMP_DIR/scheduler.log"
SCHED_DATA="$TMP_DIR/scheduler-data"
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
  if [[ -n "${SCHED_PID:-}" ]] && kill -0 "$SCHED_PID" 2>/dev/null; then
    kill "$SCHED_PID" 2>/dev/null || true
    wait "$SCHED_PID" 2>/dev/null || true
  fi
  rm -rf "$TMP_DIR"
}
trap cleanup EXIT

mkdir -p "$SCHED_DATA"

echo "starting scheduler for integration test..."
(
  cd "$ROOT_DIR"
  EDGERUN_SCHEDULER_DATA_DIR="$SCHED_DATA" \
  EDGERUN_SCHEDULER_ADDR="$SCHED_ADDR" \
  EDGERUN_SCHEDULER_MAX_REPORTS_PER_JOB=2 \
  EDGERUN_SCHEDULER_MAX_FAILURES_PER_JOB=2 \
  EDGERUN_SCHEDULER_MAX_REPLAYS_PER_JOB=2 \
  cargo run -p edgerun-scheduler >"$SCHED_LOG" 2>&1
) &
SCHED_PID=$!

for _ in $(seq 1 240); do
  if ! kill -0 "$SCHED_PID" 2>/dev/null; then
    break
  fi
  if curl -fsS "$SCHED_URL/health" >/dev/null 2>&1; then
    break
  fi
  sleep 0.5
done

if ! curl -fsS "$SCHED_URL/health" >/dev/null 2>&1; then
  echo "scheduler failed to start; log:"
  cat "$SCHED_LOG"
  exit 1
fi

post_json() {
  local path="$1"
  local body="$2"
  curl -fsS -X POST "$SCHED_URL$path" \
    -H "content-type: application/json" \
    -d "$body"
}

assert_json() {
  local json="$1"
  local expr="$2"
  python3 - "$json" "$expr" <<'PY'
import json
import sys

doc = json.loads(sys.argv[1])
expr = sys.argv[2]
ok = eval(expr, {}, {"doc": doc})
if not ok:
    raise SystemExit(f"assertion failed: {expr}\njson={json.dumps(doc, indent=2)}")
PY
}

echo "checking dedupe behavior for result/failure/replay..."

R1='{"idempotency_key":"k-result-1","worker_pubkey":"worker-a","job_id":"job-a","bundle_hash":"b1","output_hash":"o1","output_len":10}'
resp="$(post_json /v1/worker/result "$R1")"
assert_json "$resp" 'doc.get("ok") is True and doc.get("duplicate") is False'
resp="$(post_json /v1/worker/result "$R1")"
assert_json "$resp" 'doc.get("ok") is True and doc.get("duplicate") is True'

F1='{"idempotency_key":"k-failure-1","worker_pubkey":"worker-a","job_id":"job-a","bundle_hash":"b1","phase":"runtime_execute","error_code":"InstructionLimitExceeded","error_message":"out of fuel"}'
resp="$(post_json /v1/worker/failure "$F1")"
assert_json "$resp" 'doc.get("ok") is True and doc.get("duplicate") is False'
resp="$(post_json /v1/worker/failure "$F1")"
assert_json "$resp" 'doc.get("ok") is True and doc.get("duplicate") is True'

P1='{"idempotency_key":"k-replay-1","worker_pubkey":"worker-a","job_id":"job-a","artifact":{"bundle_hash":"b1","ok":false,"abi_version":1,"runtime_id":"r1","output_hash":null,"output_len":null,"input_len":3,"max_memory_bytes":1024,"max_instructions":1000,"fuel_limit":1000,"fuel_remaining":0,"error_code":"InstructionLimitExceeded","error_message":"out of fuel","trap_code":"OutOfFuel"}}'
resp="$(post_json /v1/worker/replay "$P1")"
assert_json "$resp" 'doc.get("ok") is True and doc.get("duplicate") is False'
resp="$(post_json /v1/worker/replay "$P1")"
assert_json "$resp" 'doc.get("ok") is True and doc.get("duplicate") is True'

echo "checking retention pruning per job (cap=2)..."

R2='{"idempotency_key":"k-result-2","worker_pubkey":"worker-a","job_id":"job-a","bundle_hash":"b1","output_hash":"o2","output_len":20}'
R3='{"idempotency_key":"k-result-3","worker_pubkey":"worker-a","job_id":"job-a","bundle_hash":"b1","output_hash":"o3","output_len":30}'
post_json /v1/worker/result "$R2" >/dev/null
post_json /v1/worker/result "$R3" >/dev/null

F2='{"idempotency_key":"k-failure-2","worker_pubkey":"worker-a","job_id":"job-a","bundle_hash":"b1","phase":"post_execution_verify","error_code":"BundleHashMismatch","error_message":"mismatch"}'
F3='{"idempotency_key":"k-failure-3","worker_pubkey":"worker-a","job_id":"job-a","bundle_hash":"b1","phase":"runtime_execute","error_code":"Trap","error_message":"trap"}'
post_json /v1/worker/failure "$F2" >/dev/null
post_json /v1/worker/failure "$F3" >/dev/null

P2='{"idempotency_key":"k-replay-2","worker_pubkey":"worker-a","job_id":"job-a","artifact":{"bundle_hash":"b1","ok":true,"abi_version":1,"runtime_id":"r1","output_hash":"o2","output_len":20,"input_len":3,"max_memory_bytes":1024,"max_instructions":1000,"fuel_limit":1000,"fuel_remaining":900,"error_code":null,"error_message":null,"trap_code":null}}'
P3='{"idempotency_key":"k-replay-3","worker_pubkey":"worker-a","job_id":"job-a","artifact":{"bundle_hash":"b1","ok":true,"abi_version":1,"runtime_id":"r1","output_hash":"o3","output_len":30,"input_len":3,"max_memory_bytes":1024,"max_instructions":1000,"fuel_limit":1000,"fuel_remaining":800,"error_code":null,"error_message":null,"trap_code":null}}'
post_json /v1/worker/replay "$P2" >/dev/null
post_json /v1/worker/replay "$P3" >/dev/null

status_json="$(curl -fsS "$SCHED_URL/v1/job/job-a")"
python3 - "$status_json" <<'PY'
import json
import sys

doc = json.loads(sys.argv[1])
reports = doc.get("reports", [])
failures = doc.get("failures", [])
replays = doc.get("replay_artifacts", [])
if len(reports) != 2:
    raise SystemExit(f"expected 2 reports, got {len(reports)}")
if len(failures) != 2:
    raise SystemExit(f"expected 2 failures, got {len(failures)}")
if len(replays) != 2:
    raise SystemExit(f"expected 2 replay artifacts, got {len(replays)}")
if reports[-1].get("output_hash") != "o3":
    raise SystemExit("expected newest result output_hash=o3")
if failures[-1].get("idempotency_key") != "k-failure-3":
    raise SystemExit("expected newest failure idempotency k-failure-3")
if replays[-1].get("idempotency_key") != "k-replay-3":
    raise SystemExit("expected newest replay idempotency k-replay-3")
PY

echo "integration scheduler API test passed"
