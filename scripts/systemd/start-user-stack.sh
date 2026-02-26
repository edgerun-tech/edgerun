#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
WORKER_COUNT=3
PROFILE="${EDGERUN_STACK_PROFILE:-local}"
PROFILE_SET=0

usage() {
  echo "usage: $0 [worker_count] [profile] [--workers N] [--profile PROFILE]"
  echo
  echo "examples:"
  echo "  $0"
  echo "  $0 3"
  echo "  $0 --profile local --workers 3"
  echo "  $0 3 local"
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --profile)
      PROFILE="${2:?missing --profile value}"
      PROFILE_SET=1
      shift 2
      ;;
    --profile=*)
      PROFILE="${1#*=}"
      PROFILE_SET=1
      shift
      ;;
    --workers)
      WORKER_COUNT="${2:?missing --workers value}"
      shift 2
      ;;
    --workers=*)
      WORKER_COUNT="${1#*=}"
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    [0-9]*)
      WORKER_COUNT="$1"
      shift
      ;;
    *)
      if [[ "${PROFILE_SET}" -eq 0 ]]; then
        PROFILE="$1"
        PROFILE_SET=1
        shift
      else
        echo "ERR: unexpected argument '$1'" >&2
        usage
        exit 1
      fi
      ;;
  esac
done

if ! [[ "${WORKER_COUNT}" =~ ^[0-9]+$ ]] || [[ "${WORKER_COUNT}" -lt 1 ]]; then
  echo "ERR: usage: $0 [worker_count>=1]" >&2
  exit 1
fi

enable_user_linger() {
  if ! command -v loginctl >/dev/null 2>&1; then
    echo "WARN: loginctl unavailable; boot-start cannot be verified automatically."
    return 0
  fi

  if [[ "$(loginctl show-user "$USER" -p Linger --value 2>/dev/null)" == "yes" ]]; then
    return 0
  fi

  echo "Enabling user lingering so services continue across reboot..."
  if ! loginctl enable-linger "$USER"; then
    echo "ERR: failed to enable user lingering for $USER."
    echo "Manual fix: sudo loginctl enable-linger \"$USER\""
    exit 1
  fi
}

"${ROOT_DIR}/scripts/systemd/build-user-binaries.sh" stack
"${ROOT_DIR}/scripts/systemd/install-user-services.sh" "${PROFILE}" "${WORKER_COUNT}"

if [[ "${PROFILE}" == "local" ]]; then
  systemctl --user enable --now solana-test-validator.service
fi

enable_user_linger

mkdir -p "${HOME}/.config/edgerun/workers"
for i in $(seq 1 "${WORKER_COUNT}"); do
  target="${HOME}/.config/edgerun/workers/${i}.env"
  if [[ ! -f "${target}" ]]; then
    cp "${ROOT_DIR}/scripts/systemd/env/worker-instance.env.example" "${target}"
    sed -i "s/worker-demo-1/worker-demo-${i}/" "${target}"
  fi
done

systemctl --user enable --now edgerun-scheduler.service

worker_units=()
for i in $(seq 1 "${WORKER_COUNT}"); do
  worker_units+=("edgerun-worker@${i}.service")
done
systemctl --user enable --now "${worker_units[@]}"

echo "Stack started with ${WORKER_COUNT} workers."
echo "Health check:"
echo "  curl -sS -o /dev/null -w \"%{http_code}\\n\" \"http://127.0.0.1:5566/v1/control/ws?client_id=health\"   # expect 400 (non-WebSocket probe)"
if [[ "${PROFILE}" == "local" ]]; then
  echo "  curl -fsS http://127.0.0.1:8899"
fi
echo
systemctl --user --no-pager --full status edgerun-scheduler.service "${worker_units[@]}" | sed -n '1,160p'
