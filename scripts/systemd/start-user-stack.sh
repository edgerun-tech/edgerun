#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
WORKER_COUNT="${1:-3}"

if ! [[ "${WORKER_COUNT}" =~ ^[0-9]+$ ]] || [[ "${WORKER_COUNT}" -lt 1 ]]; then
  echo "usage: $0 [worker_count>=1]" >&2
  exit 1
fi

"${ROOT_DIR}/scripts/systemd/install-user-services.sh"

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
