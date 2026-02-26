#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${ROOT_DIR}"

SITE_CONFIG="wrangler.jsonc"
CLOUD_OS_CONFIG="cloud-os/wrangler.jsonc"

for cfg in "${SITE_CONFIG}" "${CLOUD_OS_CONFIG}"; do
  if [[ ! -f "${cfg}" ]]; then
    echo "[targets] missing config: ${cfg}" >&2
    exit 1
  fi
done

extract_field() {
  local file="$1"
  local field="$2"
  sed -nE "s/^[[:space:]]*\"${field}\"[[:space:]]*:[[:space:]]*\"([^\"]+)\".*/\\1/p" "${file}" | head -n1
}

extract_assets_directory() {
  local file="$1"
  sed -nE 's/^[[:space:]]*"directory"[[:space:]]*:[[:space:]]*"([^"]+)".*/\1/p' "${file}" | head -n1
}

site_name="$(extract_field "${SITE_CONFIG}" "name")"
site_assets="$(extract_assets_directory "${SITE_CONFIG}")"
cloud_os_name="$(extract_field "${CLOUD_OS_CONFIG}" "name")"
cloud_os_assets="$(extract_assets_directory "${CLOUD_OS_CONFIG}")"

if [[ -z "${site_name}" || -z "${site_assets}" || -z "${cloud_os_name}" || -z "${cloud_os_assets}" ]]; then
  echo "[targets] failed to parse one or more required fields" >&2
  exit 1
fi

if [[ "${site_name}" == "${cloud_os_name}" ]]; then
  echo "[targets] worker name collision: ${site_name}" >&2
  exit 1
fi

if [[ "${site_assets}" == "${cloud_os_assets}" ]]; then
  echo "[targets] assets directory collision: ${site_assets}" >&2
  exit 1
fi

echo "[targets] PASS"
echo "[targets] site frontend -> config=${SITE_CONFIG} worker=${site_name} assets=${site_assets}"
echo "[targets] cloud-os frontend -> config=${CLOUD_OS_CONFIG} worker=${cloud_os_name} assets=${cloud_os_assets}"
