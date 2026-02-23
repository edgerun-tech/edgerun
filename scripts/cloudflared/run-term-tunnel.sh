#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
set -euo pipefail

TUNNEL_ID="${EDGERUN_TERM_TUNNEL_ID:-c5fab4c7-c68e-411e-880b-c1da359a18e9}"
TUNNEL_HOSTNAME="${EDGERUN_TERM_TUNNEL_HOSTNAME:-term.edgerun.tech}"
TERM_SERVER_PORT="${EDGERUN_TERM_SERVER_PORT:-5577}"
TERM_SERVER_UPSTREAM="${EDGERUN_TERM_TUNNEL_UPSTREAM:-http://127.0.0.1:${TERM_SERVER_PORT}}"
EDGERUN_SCHEDULER_TUNNEL_HOSTNAME="${EDGERUN_SCHEDULER_TUNNEL_HOSTNAME:-}"
EDGERUN_SCHEDULER_TUNNEL_UPSTREAM="${EDGERUN_SCHEDULER_TUNNEL_UPSTREAM:-http://127.0.0.1:5566}"
CREDENTIALS_FILE="${EDGERUN_TERM_TUNNEL_CREDENTIALS_FILE:-$HOME/.cloudflared/${TUNNEL_ID}.json}"

if ! command -v cloudflared >/dev/null 2>&1; then
  echo "cloudflared is required but not found on PATH" >&2
  exit 1
fi

if [[ ! -f "${CREDENTIALS_FILE}" ]]; then
  echo "Missing tunnel credentials file: ${CREDENTIALS_FILE}" >&2
  echo "Create or copy it via: cloudflared tunnel create <name>" >&2
  exit 1
fi

TMP_DIR="$(mktemp -d)"
cleanup() {
  rm -rf "${TMP_DIR}"
}
trap cleanup EXIT

CONFIG_FILE="${TMP_DIR}/config.yml"
cat >"${CONFIG_FILE}" <<EOF
tunnel: ${TUNNEL_ID}
credentials-file: ${CREDENTIALS_FILE}
ingress:
  - hostname: ${TUNNEL_HOSTNAME}
    service: ${TERM_SERVER_UPSTREAM}
EOF

if [[ -n "${EDGERUN_SCHEDULER_TUNNEL_HOSTNAME}" ]]; then
cat >>"${CONFIG_FILE}" <<EOF
  - hostname: ${EDGERUN_SCHEDULER_TUNNEL_HOSTNAME}
    service: ${EDGERUN_SCHEDULER_TUNNEL_UPSTREAM}
EOF
fi

cat >>"${CONFIG_FILE}" <<EOF
  - service: http_status:404
EOF

if [[ -n "${EDGERUN_SCHEDULER_TUNNEL_HOSTNAME}" ]]; then
  echo "Starting tunnel ${TUNNEL_ID} for ${TUNNEL_HOSTNAME} -> ${TERM_SERVER_UPSTREAM}, ${EDGERUN_SCHEDULER_TUNNEL_HOSTNAME} -> ${EDGERUN_SCHEDULER_TUNNEL_UPSTREAM}"
else
  echo "Starting tunnel ${TUNNEL_ID} for ${TUNNEL_HOSTNAME} -> ${TERM_SERVER_UPSTREAM}"
fi
exec cloudflared tunnel --config "${CONFIG_FILE}" run "${TUNNEL_ID}"
