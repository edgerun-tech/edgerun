#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${ROOT_DIR}"

SITE_CONFIG="wrangler.jsonc"

for cfg in "${SITE_CONFIG}"; do
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

if [[ -z "${site_name}" || -z "${site_assets}" ]]; then
  echo "[targets] failed to parse one or more required fields" >&2
  exit 1
fi

echo "[targets] PASS"
echo "[targets] site frontend -> config=${SITE_CONFIG} worker=${site_name} assets=${site_assets}"
