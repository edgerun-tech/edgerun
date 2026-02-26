#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
SCENARIO="${1:-healthy}"
SOURCE_FILE="${ROOT_DIR}/docker/caddy/scenarios/${SCENARIO}.caddy"
TARGET_FILE="${ROOT_DIR}/docker/caddy/routes/scenario.caddy"

if [[ ! -f "${SOURCE_FILE}" ]]; then
  echo "Unknown Caddy scenario '${SCENARIO}'." >&2
  echo "Available scenarios:" >&2
  ls -1 "${ROOT_DIR}/docker/caddy/scenarios" | sed 's/\.caddy$//' >&2
  exit 1
fi

cp "${SOURCE_FILE}" "${TARGET_FILE}"
echo "[caddy] scenario set to ${SCENARIO}"

if docker compose ps caddy >/dev/null 2>&1; then
  if docker compose ps --status running caddy | grep -q caddy; then
    docker compose exec -T caddy caddy reload --config /etc/caddy/Caddyfile >/dev/null
    echo "[caddy] reloaded"
  fi
fi
