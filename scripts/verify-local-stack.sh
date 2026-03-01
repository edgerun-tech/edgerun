#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
set -euo pipefail

DOMAIN="${EDGERUN_VERIFY_DOMAIN:-framework.bengal-salary.ts.net}"
RESOLVE_IP="${EDGERUN_VERIFY_RESOLVE_IP:-127.0.0.1}"
LOCAL_BRIDGE_URL="${EDGERUN_VERIFY_LOCAL_BRIDGE_URL:-http://127.0.0.1:7777/v1/local/node/info.pb}"
FRONTEND_URL="${EDGERUN_VERIFY_FRONTEND_URL:-http://127.0.0.1:4175/}"

pass() { printf '[pass] %s\n' "$1"; }
fail() { printf '[fail] %s\n' "$1" >&2; exit 1; }

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

curl -fsS "${LOCAL_BRIDGE_URL}" -o /tmp/edgerun-node-info.pb || fail "local bridge probe failed: ${LOCAL_BRIDGE_URL}"
pass "local bridge endpoint is reachable"

curl -fsS "${FRONTEND_URL}" -o /tmp/edgerun-frontend-index.html || fail "frontend listener probe failed: ${FRONTEND_URL}"
pass "frontend listener is reachable"

curl -kfsS --resolve "${DOMAIN}:443:${RESOLVE_IP}" "https://${DOMAIN}/" -o /tmp/edgerun-framework-host.html \
  || fail "framework domain HTTPS probe failed: https://${DOMAIN}/"
pass "framework domain HTTPS route is reachable via caddy"

printf '\nLocal stack verification complete.\n'
