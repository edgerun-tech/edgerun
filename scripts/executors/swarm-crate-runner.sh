#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
set -euo pipefail

CRATE="${CRATE_NAME:-}"
MODE="${EXECUTOR_MODE:-build}"
TRIGGER_SUBJECT="${CODE_UPDATE_SUBJECT:-edgerun.code.updated}"
NATS_URL="${EDGERUN_EVENTBUS_NATS_URL:-nats://127.0.0.1:4222}"
REPO_URL="${EDGERUN_CODE_REPO_URL:-}"
REPO_REF="${EDGERUN_CODE_REF:-main}"
WORK_DIR="${EXECUTOR_WORK_DIR:-/workspace/edgerun-src}"
NODE_ID="${NODE_ID_OVERRIDE:-$(hostname)}"

if [[ -z "${CRATE}" ]]; then
  echo "CRATE_NAME is required" >&2
  exit 1
fi
if [[ -z "${REPO_URL}" ]]; then
  echo "EDGERUN_CODE_REPO_URL is required" >&2
  exit 1
fi
if [[ "${MODE}" != "build" && "${MODE}" != "test" ]]; then
  echo "EXECUTOR_MODE must be build|test" >&2
  exit 1
fi

if ! command -v git >/dev/null 2>&1 || ! command -v nc >/dev/null 2>&1; then
  export DEBIAN_FRONTEND=noninteractive
  apt-get update -qq >/dev/null
  apt-get install -y -qq git netcat-openbsd ca-certificates >/dev/null
fi

status_subject="edgerun.executors.${CRATE}.${MODE}.status"
hostport="${NATS_URL#nats://}"
NATS_HOST="${hostport%%:*}"
NATS_PORT="${hostport##*:}"

nats_pub() {
  local subject="$1"
  local payload="$2"
  local bytes="${#payload}"
  {
    printf 'CONNECT {"verbose":false,"pedantic":false}\r\n'
    printf 'PUB %s %s\r\n' "$subject" "$bytes"
    printf '%s\r\n' "$payload"
    printf 'PING\r\n'
  } | nc -w 2 "$NATS_HOST" "$NATS_PORT" >/dev/null || true
}

publish_status() {
  local status="$1" detail="$2" started_at="$3" ended_at="$4"
  local payload
  payload="$(cat <<JSON
{"event_type":"executor_status","crate":"${CRATE}","mode":"${MODE}","status":"${status}","detail":"${detail}","node_id":"${NODE_ID}","started_at":"${started_at}","ended_at":"${ended_at}"}
JSON
)"
  nats_pub "$status_subject" "$payload"
}

sync_code() {
  if [[ ! -d "${WORK_DIR}/.git" ]]; then
    rm -rf "${WORK_DIR}"
    if ! git clone --depth 1 --branch "${REPO_REF}" "${REPO_URL}" "${WORK_DIR}"; then
      git clone --depth 1 "${REPO_URL}" "${WORK_DIR}"
    fi
    return
  fi
  if (
    cd "${WORK_DIR}"
    git fetch --depth 1 origin "${REPO_REF}"
  ); then
    (
      cd "${WORK_DIR}"
      git reset --hard "origin/${REPO_REF}"
      git clean -fd
    )
    return
  fi

  (
    cd "${WORK_DIR}"
    git fetch --depth 1 origin
    default_ref="$(git symbolic-ref -q --short refs/remotes/origin/HEAD || true)"
    if [[ -z "${default_ref}" ]]; then
      default_ref="origin/main"
    fi
    git reset --hard "${default_ref}"
    git clean -fd
  )
}

run_once() {
  local started_at ended_at out rc summary
  started_at="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
  publish_status "started" "executor run started" "$started_at" ""

  set +e
  if [[ "$MODE" == "build" ]]; then
    out="$(cd "$WORK_DIR" && cargo check -p "$CRATE" 2>&1)"
  else
    out="$(cd "$WORK_DIR" && cargo test -p "$CRATE" 2>&1)"
  fi
  rc=$?
  set -e

  ended_at="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
  if [[ $rc -eq 0 ]]; then
    publish_status "success" "$MODE passed" "$started_at" "$ended_at"
    return 0
  fi

  summary="$(printf '%s' "$out" | tail -n 8 | tr '\n' ' ' | sed 's/"/\\"/g')"
  publish_status "failure" "$MODE failed: $summary" "$started_at" "$ended_at"
  return $rc
}

run_triggered_once() {
  local started_at ended_at
  started_at="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
  if ! sync_code; then
    ended_at="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
    publish_status "failure" "code sync failed from ${REPO_URL}@${REPO_REF}" "$started_at" "$ended_at"
    return 1
  fi
  run_once
}

subscribe_loop() {
  while true; do
    coproc NATS { nc "$NATS_HOST" "$NATS_PORT"; }
    exec 3>&${NATS[1]}
    exec 4<&${NATS[0]}

    printf 'CONNECT {"verbose":false,"pedantic":false}\r\n' >&3
    printf 'SUB %s 1\r\n' "$TRIGGER_SUBJECT" >&3
    printf 'PING\r\n' >&3

    local line payload
    while IFS= read -r line <&4; do
      line="${line%$'\r'}"
      if [[ "$line" == PING ]]; then
        printf 'PONG\r\n' >&3
        continue
      fi
      if [[ "$line" == MSG* ]]; then
        IFS= read -r payload <&4 || true
        run_triggered_once || true
      fi
    done

    sleep 1
  done
}

subscribe_loop
