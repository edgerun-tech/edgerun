#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
set -euo pipefail

DOMAIN="${EDGERUN_VERIFY_DOMAIN:-framework.bengal-salary.ts.net}"
RESOLVE_IP="${EDGERUN_VERIFY_RESOLVE_IP:-127.0.0.1}"
LOCAL_BRIDGE_URL="${EDGERUN_VERIFY_LOCAL_BRIDGE_URL:-http://127.0.0.1:7777/v1/local/node/info.pb}"
FRONTEND_URL="${EDGERUN_VERIFY_FRONTEND_URL:-http://127.0.0.1:4175/}"
HOST_BRIDGE_URL="${EDGERUN_VERIFY_HOST_BRIDGE_URL:-https://${DOMAIN}/v1/local/node/info.pb}"

pass() { printf '[pass] %s\n' "$1"; }
fail() { printf '[fail] %s\n' "$1" >&2; exit 1; }

curl_retry() {
  local label="$1"
  local output_path="$2"
  local url="$3"
  local resolve_hostport="${4:-}"
  local retries="${5:-12}"
  local delay_sec="${6:-1}"
  local attempt=1
  while [[ ${attempt} -le ${retries} ]]; do
    if [[ -n "${resolve_hostport}" ]]; then
      if curl -kfsS --resolve "${resolve_hostport}" "${url}" -o "${output_path}"; then
        return 0
      fi
    else
      if curl -fsS "${url}" -o "${output_path}"; then
        return 0
      fi
    fi
    if [[ ${attempt} -lt ${retries} ]]; then
      printf '[wait] %s (attempt %s/%s)\n' "${label}" "${attempt}" "${retries}"
      sleep "${delay_sec}"
    fi
    attempt=$((attempt + 1))
  done
  return 1
}

if ! command -v docker >/dev/null 2>&1; then
  fail "docker is required"
fi
if ! command -v curl >/dev/null 2>&1; then
  fail "curl is required"
fi

if docker ps --format '{{.Names}}' | grep -qx 'edgerun-framework-caddy'; then
  pass "framework caddy container is running"
elif docker ps --format '{{.Names}}' | grep -qx 'edgerun-osdev-caddy'; then
  pass "legacy caddy container is running"
else
  fail "caddy container is not running"
fi

if docker ps --format '{{.Names}}' | grep -qx 'edgerun-cloudflared'; then
  pass "cloudflared container is running"
else
  printf '[warn] cloudflared container not running\n'
fi

curl_retry "local bridge" /tmp/edgerun-node-info.pb "${LOCAL_BRIDGE_URL}" "" 20 1 || fail "local bridge probe failed: ${LOCAL_BRIDGE_URL}"
pass "local bridge endpoint is reachable"

curl_retry "frontend listener" /tmp/edgerun-frontend-index.html "${FRONTEND_URL}" "" 12 1 || fail "frontend listener probe failed: ${FRONTEND_URL}"
pass "frontend listener is reachable"

curl_retry "framework https" /tmp/edgerun-framework-host.html "https://${DOMAIN}/" "${DOMAIN}:443:${RESOLVE_IP}" 12 1 \
  || fail "framework domain HTTPS probe failed: https://${DOMAIN}/"
pass "framework domain HTTPS route is reachable via caddy"

if ! grep -q "<title>Intent UI</title>" /tmp/edgerun-framework-host.html; then
  fail "framework root is not serving Intent UI shell"
fi
pass "framework root serves intent-ui shell"

curl_retry "framework bridge" /tmp/edgerun-framework-host-node-info.pb "${HOST_BRIDGE_URL}" "${DOMAIN}:443:${RESOLVE_IP}" 20 1 \
  || fail "framework host bridge probe failed: ${HOST_BRIDGE_URL}"
pass "framework host bridge route is reachable"

printf '\nLocal stack verification complete.\n'
