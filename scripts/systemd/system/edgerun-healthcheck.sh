#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
set -euo pipefail

fail() {
  echo "FAIL: $*" >&2
  exit 1
}

check_active() {
  local svc="$1"
  systemctl is-active --quiet "$svc" || fail "$svc is not active"
}

wait_for_tcp() {
  local port="$1"
  local retries="${2:-15}"
  local i
  for i in $(seq 1 "$retries"); do
    if ss -ltnH | awk '{print $4}' | grep -qE "[:.]${port}$"; then
      return 0
    fi
    sleep 1
  done
  fail "tcp port ${port} is not listening"
}

wait_for_udp() {
  local port="$1"
  local retries="${2:-15}"
  local i
  for i in $(seq 1 "$retries"); do
    if ss -lunH | awk '{print $4}' | grep -qE "[:.]${port}$"; then
      return 0
    fi
    sleep 1
  done
  fail "udp port ${port} is not listening"
}

check_active edgerun-scheduler
check_active edgerun-term-server
check_active coturn

wait_for_tcp 5566
wait_for_tcp 5577
wait_for_udp 3478

curl -sS --max-time 5 -o /dev/null http://127.0.0.1:5577/ || fail "term-server HTTP connect check failed"

python3 - <<'PY' || fail "scheduler control-ws route.resolve check failed"
import asyncio
import json
import websockets

async def main():
    uri = "ws://127.0.0.1:5566/v1/control/ws?client_id=healthcheck"
    async with websockets.connect(uri) as ws:
        req = {
            "request_id": "health-route-resolve",
            "op": "route.resolve",
            "payload": {"device_id": "healthcheck-device"},
        }
        await ws.send(json.dumps(req))
        raw = await asyncio.wait_for(ws.recv(), timeout=5)
        data = json.loads(raw)
        if not data.get("ok", False):
            raise RuntimeError(f"control ws response not ok: {data}")

asyncio.run(main())
PY

echo "OK: edgerun core services healthy"
