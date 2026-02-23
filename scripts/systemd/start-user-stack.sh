#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
WORKER_COUNT="${1:-3}"

if ! [[ "${WORKER_COUNT}" =~ ^[0-9]+$ ]] || [[ "${WORKER_COUNT}" -lt 1 ]]; then
  echo "usage: $0 [worker_count>=1]" >&2
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

"${ROOT_DIR}/scripts/systemd/install-user-services.sh"
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
echo "  curl -fsS http://127.0.0.1:5566/health"
echo
systemctl --user --no-pager --full status edgerun-scheduler.service "${worker_units[@]}" | sed -n '1,160p'
