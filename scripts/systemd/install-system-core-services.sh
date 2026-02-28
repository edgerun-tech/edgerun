#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "${ROOT_DIR}"

if [[ "${EUID}" -ne 0 ]]; then
  echo "This script must run as root." >&2
  exit 1
fi

EDGERUN_USER="${EDGERUN_USER:-edgerun}"
EDGERUN_GROUP="${EDGERUN_GROUP:-edgerun}"
EDGERUN_ROOT="${EDGERUN_ROOT:-/opt/edgerun}"
EDGERUN_ETC_DIR="${EDGERUN_ETC_DIR:-/etc/edgerun}"
PUBLIC_IP="${PUBLIC_IP:-}"
INSTALL_BINARIES="${INSTALL_BINARIES:-1}"
INSTALL_PACKAGES="${INSTALL_PACKAGES:-1}"
ENABLE_SERVICES="${ENABLE_SERVICES:-1}"
INSTALL_HEALTHCHECK="${INSTALL_HEALTHCHECK:-1}"
INSTALL_COTURN="${INSTALL_COTURN:-1}"
FORCE_OVERWRITE_ENV="${FORCE_OVERWRITE_ENV:-0}"

if [[ -z "${PUBLIC_IP}" ]]; then
  PUBLIC_IP="$(hostname -I 2>/dev/null | awk '{print $1}')"
fi
if [[ -z "${PUBLIC_IP}" ]]; then
  echo "PUBLIC_IP is required (set PUBLIC_IP=x.x.x.x)." >&2
  exit 1
fi

STUN_SERVER="${STUN_SERVER:-${PUBLIC_IP}:3478}"

if [[ "${INSTALL_PACKAGES}" == "1" ]] && command -v apt-get >/dev/null 2>&1; then
  export DEBIAN_FRONTEND=noninteractive
  apt-get update -qq
  apt-get install -y -qq ca-certificates curl python3 python3-websockets coturn >/dev/null
fi

if ! getent group "${EDGERUN_GROUP}" >/dev/null; then
  groupadd --system "${EDGERUN_GROUP}"
fi
if ! id -u "${EDGERUN_USER}" >/dev/null 2>&1; then
  useradd --system --gid "${EDGERUN_GROUP}" --home-dir /var/lib/edgerun --create-home --shell /usr/sbin/nologin "${EDGERUN_USER}"
fi

install -d -m 0755 -o "${EDGERUN_USER}" -g "${EDGERUN_GROUP}" \
  "${EDGERUN_ROOT}" \
  "${EDGERUN_ROOT}/out/target/release" \
  /var/lib/edgerun \
  /var/lib/edgerun/scheduler \
  /var/lib/edgerun/term-server \
  /var/lib/edgerun/workers
install -d -m 0750 -o root -g "${EDGERUN_GROUP}" "${EDGERUN_ETC_DIR}"

if [[ "${INSTALL_BINARIES}" == "1" ]]; then
  export CARGO_TARGET_DIR="${ROOT_DIR}/out/target"
  cargo build --locked --release -p edgerun-scheduler
  cargo build --locked --release -p edgerun-term-server --features term
  install -m 0755 -o "${EDGERUN_USER}" -g "${EDGERUN_GROUP}" \
    "${CARGO_TARGET_DIR}/release/edgerun-scheduler" \
    "${EDGERUN_ROOT}/out/target/release/edgerun-scheduler"
  install -m 0755 -o "${EDGERUN_USER}" -g "${EDGERUN_GROUP}" \
    "${CARGO_TARGET_DIR}/release/edgerun-term-server" \
    "${EDGERUN_ROOT}/out/target/release/edgerun-term-server"
fi

render_env_template() {
  local src="$1"
  local dest="$2"
  local tmp
  tmp="$(mktemp)"
  sed \
    -e "s|__PUBLIC_IP__|${PUBLIC_IP}|g" \
    -e "s|__STUN_SERVER__|${STUN_SERVER}|g" \
    "${src}" > "${tmp}"
  install -m 0640 -o root -g "${EDGERUN_GROUP}" "${tmp}" "${dest}"
  rm -f "${tmp}"
}

if [[ "${FORCE_OVERWRITE_ENV}" == "1" || ! -f "${EDGERUN_ETC_DIR}/scheduler.env" ]]; then
  render_env_template "${ROOT_DIR}/scripts/systemd/env/system-core/scheduler.env.example" "${EDGERUN_ETC_DIR}/scheduler.env"
fi
if [[ "${FORCE_OVERWRITE_ENV}" == "1" || ! -f "${EDGERUN_ETC_DIR}/term-server.env" ]]; then
  render_env_template "${ROOT_DIR}/scripts/systemd/env/system-core/term-server.env.example" "${EDGERUN_ETC_DIR}/term-server.env"
fi

install -Dm0644 "${ROOT_DIR}/scripts/systemd/system/edgerun-scheduler.system.service" /etc/systemd/system/edgerun-scheduler.service
install -Dm0644 "${ROOT_DIR}/scripts/systemd/system/edgerun-term-server.system.service" /etc/systemd/system/edgerun-term-server.service

if [[ "${INSTALL_COTURN}" == "1" ]]; then
  install -Dm0644 "${ROOT_DIR}/scripts/systemd/system/turnserver.stun-only.conf" /etc/turnserver.conf
  install -Dm0644 "${ROOT_DIR}/scripts/systemd/system/coturn.liveness.conf" /etc/systemd/system/coturn.service.d/10-liveness.conf
fi

if [[ "${INSTALL_HEALTHCHECK}" == "1" ]]; then
  install -Dm0755 "${ROOT_DIR}/scripts/systemd/system/edgerun-healthcheck.sh" /usr/local/bin/edgerun-healthcheck.sh
  install -Dm0644 "${ROOT_DIR}/scripts/systemd/system/edgerun-healthcheck.service" /etc/systemd/system/edgerun-healthcheck.service
  install -Dm0644 "${ROOT_DIR}/scripts/systemd/system/edgerun-healthcheck.timer" /etc/systemd/system/edgerun-healthcheck.timer
fi

if command -v ufw >/dev/null 2>&1; then
  if ufw status | grep -q "Status: active"; then
    ufw allow 3478/udp >/dev/null || true
    ufw allow 3478/tcp >/dev/null || true
  fi
fi

systemctl daemon-reload
if [[ "${ENABLE_SERVICES}" == "1" ]]; then
  systemctl enable --now edgerun-scheduler.service edgerun-term-server.service
  if [[ "${INSTALL_COTURN}" == "1" ]]; then
    systemctl enable --now coturn.service
  fi
  if [[ "${INSTALL_HEALTHCHECK}" == "1" ]]; then
    systemctl enable --now edgerun-healthcheck.timer
    systemctl start edgerun-healthcheck.service || true
  fi
fi

echo

echo "Installed core services:"
echo "  /etc/systemd/system/edgerun-scheduler.service"
echo "  /etc/systemd/system/edgerun-term-server.service"
if [[ "${INSTALL_COTURN}" == "1" ]]; then
  echo "  /etc/turnserver.conf"
  echo "  /etc/systemd/system/coturn.service.d/10-liveness.conf"
fi
if [[ "${INSTALL_HEALTHCHECK}" == "1" ]]; then
  echo "  /usr/local/bin/edgerun-healthcheck.sh"
  echo "  /etc/systemd/system/edgerun-healthcheck.service"
  echo "  /etc/systemd/system/edgerun-healthcheck.timer"
fi

echo
echo "Config files:"
echo "  ${EDGERUN_ETC_DIR}/scheduler.env"
echo "  ${EDGERUN_ETC_DIR}/term-server.env"

echo
echo "Next checks:"
echo "  systemctl --no-pager --full status edgerun-scheduler edgerun-term-server coturn edgerun-healthcheck.timer"
echo "  /usr/local/bin/edgerun-healthcheck.sh"
