#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
SYSTEMD_USER_DIR="${HOME}/.config/systemd/user"
EDGERUN_CFG_DIR="${HOME}/.config/edgerun"
WORKERS_CFG_DIR="${EDGERUN_CFG_DIR}/workers"

mkdir -p "${SYSTEMD_USER_DIR}" "${EDGERUN_CFG_DIR}" "${WORKERS_CFG_DIR}"

sed \
  -e "s|__ROOT_DIR__|${ROOT_DIR}|g" \
  -e "s|__HOME__|${HOME}|g" \
  "${ROOT_DIR}/scripts/systemd/user/edgerun-scheduler.service" \
  > "${SYSTEMD_USER_DIR}/edgerun-scheduler.service"
chmod 0644 "${SYSTEMD_USER_DIR}/edgerun-scheduler.service"

sed \
  -e "s|__ROOT_DIR__|${ROOT_DIR}|g" \
  -e "s|__HOME__|${HOME}|g" \
  "${ROOT_DIR}/scripts/systemd/user/edgerun-worker@.service" \
  > "${SYSTEMD_USER_DIR}/edgerun-worker@.service"
chmod 0644 "${SYSTEMD_USER_DIR}/edgerun-worker@.service"

sed \
  -e "s|__ROOT_DIR__|${ROOT_DIR}|g" \
  -e "s|__HOME__|${HOME}|g" \
  "${ROOT_DIR}/scripts/systemd/user/edgerun-term-server.service" \
  > "${SYSTEMD_USER_DIR}/edgerun-term-server.service"
chmod 0644 "${SYSTEMD_USER_DIR}/edgerun-term-server.service"

sed \
  -e "s|__ROOT_DIR__|${ROOT_DIR}|g" \
  -e "s|__HOME__|${HOME}|g" \
  "${ROOT_DIR}/scripts/systemd/user/edgerun-cloudflared-term.service" \
  > "${SYSTEMD_USER_DIR}/edgerun-cloudflared-term.service"
chmod 0644 "${SYSTEMD_USER_DIR}/edgerun-cloudflared-term.service"

if [[ ! -f "${EDGERUN_CFG_DIR}/scheduler.env" ]]; then
  cp "${ROOT_DIR}/scripts/systemd/env/scheduler.env.example" "${EDGERUN_CFG_DIR}/scheduler.env"
fi

if [[ ! -f "${EDGERUN_CFG_DIR}/worker-common.env" ]]; then
  cp "${ROOT_DIR}/scripts/systemd/env/worker-common.env.example" "${EDGERUN_CFG_DIR}/worker-common.env"
fi

if [[ ! -f "${EDGERUN_CFG_DIR}/term-server.env" ]]; then
  cp "${ROOT_DIR}/scripts/systemd/env/term-server.env.example" "${EDGERUN_CFG_DIR}/term-server.env"
fi

if [[ ! -f "${EDGERUN_CFG_DIR}/cloudflared-term.env" ]]; then
  cp "${ROOT_DIR}/scripts/systemd/env/cloudflared-term.env.example" "${EDGERUN_CFG_DIR}/cloudflared-term.env"
fi

for i in 1 2 3; do
  target="${WORKERS_CFG_DIR}/${i}.env"
  if [[ ! -f "${target}" ]]; then
    cp "${ROOT_DIR}/scripts/systemd/env/worker-instance.env.example" "${target}"
    sed -i "s/worker-demo-1/worker-demo-${i}/" "${target}"
  fi
done

systemctl --user daemon-reload

echo "Installed user services:"
echo "  ${SYSTEMD_USER_DIR}/edgerun-scheduler.service"
echo "  ${SYSTEMD_USER_DIR}/edgerun-worker@.service"
echo "  ${SYSTEMD_USER_DIR}/edgerun-term-server.service"
echo "  ${SYSTEMD_USER_DIR}/edgerun-cloudflared-term.service"
echo
echo "Config directory:"
echo "  ${EDGERUN_CFG_DIR}"
echo
echo "Next:"
echo "  systemctl --user enable --now edgerun-scheduler.service"
echo "  systemctl --user enable --now edgerun-worker@1.service edgerun-worker@2.service edgerun-worker@3.service"
echo "  systemctl --user enable --now edgerun-term-server.service edgerun-cloudflared-term.service"
