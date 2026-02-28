#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
CRATE="${CRATE_NAME:-${1:-}}"
MODE="${EXECUTOR_MODE:-${2:-build}}"
WATCH="${EXECUTOR_WATCH:-${3:-1}}"
TRIGGER_SUBJECT="${CODE_UPDATE_SUBJECT:-edgerun.code.updated}"
NATS_URL="${EDGERUN_EVENTBUS_NATS_URL:-nats://127.0.0.1:4222}"
NODE_ID="${NODE_ID_OVERRIDE:-$(hostname)}"

if [[ -z "${CRATE}" ]]; then
  echo "crate name is required" >&2
  exit 1
fi
if [[ "${MODE}" != "build" && "${MODE}" != "test" ]]; then
  echo "mode must be build|test" >&2
  exit 1
fi

status_subject="edgerun.executors.${CRATE}.${MODE}.status"

publish_status() {
  local status="$1"
  local detail="$2"
  local started_at="$3"
  local ended_at="$4"
  local payload
  payload="$(cat <<JSON
{"event_type":"executor_status","crate":"${CRATE}","mode":"${MODE}","status":"${status}","detail":"${detail}","node_id":"${NODE_ID}","started_at":"${started_at}","ended_at":"${ended_at}"}
JSON
)"
  "${ROOT_DIR}/scripts/executors/nats-pub.sh" "${status_subject}" "${payload}" || true
}

run_once() {
  local started_at ended_at
  started_at="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
  publish_status "started" "executor run started" "${started_at}" ""

  local cmd=(cargo check -p "${CRATE}")
  if [[ "${MODE}" == "test" ]]; then
    cmd=(cargo test -p "${CRATE}")
  fi

  local out rc
  set +e
  out="$(cd "${ROOT_DIR}" && "${cmd[@]}" 2>&1)"
  rc=$?
  set -e

  ended_at="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
  if [[ ${rc} -eq 0 ]]; then
    publish_status "success" "${MODE} passed" "${started_at}" "${ended_at}"
    return 0
  fi

  local summary
  summary="$(printf '%s' "${out}" | tail -n 8 | tr '\n' ' ' | sed 's/"/\\"/g')"
  publish_status "failure" "${MODE} failed: ${summary}" "${started_at}" "${ended_at}"
  return ${rc}
}

subscribe_loop() {
  local hostport host port
  hostport="${NATS_URL#nats://}"
  host="${hostport%%:*}"
  port="${hostport##*:}"

  coproc NATS { nc "${host}" "${port}"; }
  exec 3>&${NATS[1]}
  exec 4<&${NATS[0]}

  printf 'CONNECT {"verbose":false,"pedantic":false}\r\n' >&3
  printf 'SUB %s 1\r\n' "${TRIGGER_SUBJECT}" >&3
  printf 'PING\r\n' >&3

  local line
  while IFS= read -r line <&4; do
    line="${line%$'\r'}"
    if [[ "${line}" == PING ]]; then
      printf 'PONG\r\n' >&3
      continue
    fi
    if [[ "${line}" == MSG* ]]; then
      # Next line is payload; we don't require parsing for trigger semantics.
      IFS= read -r _payload <&4 || true
      run_once || true
    fi
  done
}

if [[ "${WATCH}" == "0" ]]; then
  run_once
  exit $?
fi

subscribe_loop
