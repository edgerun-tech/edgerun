#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${ROOT_DIR}"

SCHEDULER_HOST="${SCHEDULER_HOST:-}"
SCHEDULER_USER="${SCHEDULER_USER:-root}"
SCHEDULER_PORT="${SCHEDULER_PORT:-22}"
SCHEDULER_PASSWORD="${SCHEDULER_PASSWORD:-}"
SCHEDULER_SSH_KEY_FILE="${SCHEDULER_SSH_KEY_FILE:-}"
SCHEDULER_REMOTE_ROOT="${SCHEDULER_REMOTE_ROOT:-/root/src/edgerun}"
SCHEDULER_BUILD_ON_REMOTE="${SCHEDULER_BUILD_ON_REMOTE:-1}"
SCHEDULER_RESTART_SERVICES="${SCHEDULER_RESTART_SERVICES:-1}"
SCHEDULER_SYSTEMD_SCOPE="${SCHEDULER_SYSTEMD_SCOPE:-user}"
SCHEDULER_SERVICES="${SCHEDULER_SERVICES:-edgerun-scheduler.service edgerun-worker@1.service edgerun-term-server.service}"
SCHEDULER_HEALTH_URL="${SCHEDULER_HEALTH_URL:-http://127.0.0.1:5566/health}"
SCHEDULER_DRY_RUN="${SCHEDULER_DRY_RUN:-0}"

if [[ -z "${SCHEDULER_HOST}" ]]; then
  echo "SCHEDULER_HOST is required" >&2
  exit 1
fi

if [[ "${SCHEDULER_SYSTEMD_SCOPE}" != "user" && "${SCHEDULER_SYSTEMD_SCOPE}" != "system" ]]; then
  echo "SCHEDULER_SYSTEMD_SCOPE must be 'user' or 'system'" >&2
  exit 1
fi

if [[ -n "${SCHEDULER_SSH_KEY_FILE}" && ! -f "${SCHEDULER_SSH_KEY_FILE}" ]]; then
  echo "SCHEDULER_SSH_KEY_FILE does not exist: ${SCHEDULER_SSH_KEY_FILE}" >&2
  exit 1
fi

SSH_TARGET="${SCHEDULER_USER}@${SCHEDULER_HOST}"
SSH_OPTS=(
  -p "${SCHEDULER_PORT}"
  -o StrictHostKeyChecking=accept-new
  -o UserKnownHostsFile="${HOME}/.ssh/known_hosts"
  -o ConnectTimeout=12
)
if [[ -n "${SCHEDULER_SSH_KEY_FILE}" ]]; then
  SSH_OPTS+=( -i "${SCHEDULER_SSH_KEY_FILE}" )
fi

run_ssh() {
  if [[ -n "${SCHEDULER_PASSWORD}" ]]; then
    if ! command -v sshpass >/dev/null 2>&1; then
      echo "sshpass is required when SCHEDULER_PASSWORD is set" >&2
      exit 1
    fi
    sshpass -p "${SCHEDULER_PASSWORD}" ssh "${SSH_OPTS[@]}" "${SSH_TARGET}" "$@"
  else
    ssh "${SSH_OPTS[@]}" "${SSH_TARGET}" "$@"
  fi
}

if [[ "${SCHEDULER_DRY_RUN}" == "1" ]]; then
  echo "[push-scheduler] dry-run enabled"
  echo "[push-scheduler] target: ${SSH_TARGET}:${SCHEDULER_REMOTE_ROOT}"
  echo "[push-scheduler] build_on_remote=${SCHEDULER_BUILD_ON_REMOTE} restart_services=${SCHEDULER_RESTART_SERVICES} scope=${SCHEDULER_SYSTEMD_SCOPE}"
  exit 0
fi

echo "[push-scheduler] checking remote SSH connectivity"
run_ssh "hostname; uname -sr"

echo "[push-scheduler] syncing repository to ${SSH_TARGET}:${SCHEDULER_REMOTE_ROOT}"
# Prefer git-indexed file selection to avoid syncing local build artifacts
# (e.g. target/node_modules) and nested repo working trees.
if command -v git >/dev/null 2>&1 && git rev-parse --is-inside-work-tree >/dev/null 2>&1; then
  git ls-files -z --cached --others --exclude-standard \
    | tar --null -T - -czf - \
    | run_ssh "mkdir -p '${SCHEDULER_REMOTE_ROOT}' && tar -xzf - -C '${SCHEDULER_REMOTE_ROOT}'"
else
  # Fallback when git metadata is unavailable.
  tar \
    --exclude-vcs \
    --exclude='out' \
    --exclude='target' \
    --exclude='**/target' \
    --exclude='program/target-local' \
    --exclude='**/node_modules' \
    --exclude='**/dist' \
    --exclude='**/.next' \
    --exclude='**/.turbo' \
    --exclude='**/.cache' \
    -czf - . \
    | run_ssh "mkdir -p '${SCHEDULER_REMOTE_ROOT}' && tar -xzf - -C '${SCHEDULER_REMOTE_ROOT}'"
fi

echo "[push-scheduler] running remote build/restart"
run_ssh \
  "SCHEDULER_REMOTE_ROOT='${SCHEDULER_REMOTE_ROOT}' \
   SCHEDULER_BUILD_ON_REMOTE='${SCHEDULER_BUILD_ON_REMOTE}' \
   SCHEDULER_RESTART_SERVICES='${SCHEDULER_RESTART_SERVICES}' \
   SCHEDULER_SYSTEMD_SCOPE='${SCHEDULER_SYSTEMD_SCOPE}' \
   SCHEDULER_SERVICES='${SCHEDULER_SERVICES}' \
   SCHEDULER_HEALTH_URL='${SCHEDULER_HEALTH_URL}' \
   bash -s" <<'REMOTE_EOF'
set -euo pipefail

cd "${SCHEDULER_REMOTE_ROOT}"

if [[ "${SCHEDULER_BUILD_ON_REMOTE}" == "1" ]]; then
  echo "[remote] building scheduler stack binaries"
  cargo build --release \
    -p edgerun-scheduler \
    -p edgerun-worker \
    -p edgerun-term-server
fi

if [[ "${SCHEDULER_RESTART_SERVICES}" == "1" ]]; then
  echo "[remote] restarting services (${SCHEDULER_SYSTEMD_SCOPE})"
  if [[ "${SCHEDULER_SYSTEMD_SCOPE}" == "system" ]]; then
    systemctl daemon-reload
    systemctl restart ${SCHEDULER_SERVICES}
    systemctl --no-pager --full status ${SCHEDULER_SERVICES} | sed -n '1,120p' || true
  else
    systemctl --user daemon-reload
    systemctl --user restart ${SCHEDULER_SERVICES}
    systemctl --user --no-pager --full status ${SCHEDULER_SERVICES} | sed -n '1,120p' || true
  fi
fi

if command -v curl >/dev/null 2>&1; then
  echo "[remote] scheduler health check: ${SCHEDULER_HEALTH_URL}"
  curl -fsS "${SCHEDULER_HEALTH_URL}" || true
fi
REMOTE_EOF

echo "[push-scheduler] done"
