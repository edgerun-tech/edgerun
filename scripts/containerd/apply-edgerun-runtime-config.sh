#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
SRC_SNIPPET="${SRC_SNIPPET:-${ROOT_DIR}/config/containerd/edgerun-runtime-snippet.toml}"
DST_DIR="${DST_DIR:-/etc/containerd/conf.d}"
DST_FILE="${DST_FILE:-${DST_DIR}/50-edgerun-runtime.toml}"
CONFIG_FILE="${CONFIG_FILE:-/etc/containerd/config.toml}"
RESTART_CONTAINERD="${RESTART_CONTAINERD:-0}"
RUN_SMOKE="${RUN_SMOKE:-0}"
REQUIRE_IMPORT="${REQUIRE_IMPORT:-1}"

if [[ "${EUID}" -ne 0 ]]; then
  echo "This script must run as root."
  exit 1
fi

if [[ ! -f "${SRC_SNIPPET}" ]]; then
  echo "missing source snippet: ${SRC_SNIPPET}" >&2
  exit 1
fi

mkdir -p "${DST_DIR}"
install -Dm0644 "${SRC_SNIPPET}" "${DST_FILE}"

if [[ "${REQUIRE_IMPORT}" == "1" ]]; then
  if [[ ! -f "${CONFIG_FILE}" ]]; then
    echo "missing containerd config: ${CONFIG_FILE}" >&2
    exit 1
  fi
  if ! rg -q "/etc/containerd/conf\\.d/\\*\\.toml" "${CONFIG_FILE}"; then
    echo "containerd imports do not include /etc/containerd/conf.d/*.toml" >&2
    echo "update ${CONFIG_FILE} imports and re-run." >&2
    exit 1
  fi
fi

if [[ "${RESTART_CONTAINERD}" == "1" ]]; then
  systemctl restart containerd.service
fi

dump="$(containerd config dump 2>/dev/null || true)"
if [[ -z "${dump}" ]]; then
  echo "containerd config dump failed (is containerd running?)" >&2
  exit 1
fi

if ! printf '%s\n' "${dump}" | rg -q "runtime_type = 'io\\.containerd\\.edgerun\\.v1'"; then
  echo "effective config missing io.containerd.edgerun.v1 runtime_type" >&2
  exit 1
fi
if ! printf '%s\n' "${dump}" | rg -q "\\[proxy_plugins\\.edgerun\\]"; then
  echo "effective config missing proxy_plugins.edgerun" >&2
  exit 1
fi

echo "applied snippet: ${DST_FILE}"
echo "effective containerd config includes edgerun runtime + proxy plugin"

if [[ "${RUN_SMOKE}" == "1" ]]; then
  (
    cd "${ROOT_DIR}"
    ./scripts/containerd-runtime-matrix-smoke.sh
  )
fi
