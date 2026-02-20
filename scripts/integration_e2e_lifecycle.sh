#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TMP_DIR="$(mktemp -d)"
SCHED_LOG="$TMP_DIR/scheduler.log"
WORKER_LOG="$TMP_DIR/worker.log"
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
WORKER_PUBKEY="worker-e2e-1"

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

echo "creating assigned job..."
create_body='{
  "runtime_id":"0000000000000000000000000000000000000000000000000000000000000000",
  "wasm_base64":"AA==",
  "input_base64":"",
  "limits":{"max_memory_bytes":1048576,"max_instructions":10000},
  "escrow_lamports":1,
  "assignment_worker_pubkey":"'"$WORKER_PUBKEY"'"
}'
create_resp="$(curl -fsS -X POST "$SCHED_URL/v1/job/create" -H "content-type: application/json" -d "$create_body")"
job_id="$(python3 - "$create_resp" <<'PY'
import json, sys
doc = json.loads(sys.argv[1])
print(doc["job_id"])
PY
)"
echo "job_id=$job_id"

echo "waiting for worker to report failure+replay..."
status_json=""
ok=0
for _ in $(seq 1 240); do
  if ! kill -0 "$WORKER_PID" 2>/dev/null; then
    echo "worker exited unexpectedly"
    break
  fi
  if ! kill -0 "$SCHED_PID" 2>/dev/null; then
    echo "scheduler exited unexpectedly"
    break
  fi
  status_json="$(curl -fsS "$SCHED_URL/v1/job/$job_id")"
  if python3 - "$status_json" <<'PY'
import json, sys
doc = json.loads(sys.argv[1])
ok = bool(doc.get("failures")) and bool(doc.get("replay_artifacts"))
raise SystemExit(0 if ok else 1)
PY
  then
    ok=1
    break
  fi
  sleep 0.5
done

if [[ "$ok" != "1" ]]; then
  echo "timeout waiting for failure+replay"
  echo "scheduler log:"
  cat "$SCHED_LOG" || true
  echo "worker log:"
  cat "$WORKER_LOG" || true
  exit 1
fi

python3 - "$status_json" <<'PY'
import json, sys
doc = json.loads(sys.argv[1])
fails = doc.get("failures", [])
replays = doc.get("replay_artifacts", [])
if not fails:
    raise SystemExit("expected at least one failure report")
if not replays:
    raise SystemExit("expected at least one replay artifact")
last_fail = fails[-1]
last_replay = replays[-1]
if not last_fail.get("error_code"):
    raise SystemExit("failure error_code missing")
artifact = last_replay.get("artifact", {})
if artifact.get("ok") is not False:
    raise SystemExit("expected replay artifact ok=false for invalid wasm")
print("e2e lifecycle assertions passed")
PY

echo "integration e2e lifecycle test passed"
