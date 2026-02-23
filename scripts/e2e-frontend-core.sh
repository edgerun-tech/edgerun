#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR/frontend"

FRONTEND_PORT="${EDGERUN_E2E_FRONTEND_PORT:-4173}"
SERVE_LOG="${EDGERUN_E2E_FRONTEND_SERVE_LOG:-/tmp/edgerun-frontend-serve.log}"
CORE_SPEC_GLOB='cypress/e2e/**/*.cy.{js,ts},!cypress/e2e/terminal-compose-stack.cy.js'

if [[ ! -f ../out/frontend/site/index.html ]]; then
  echo "Missing build output: ../out/frontend/site/index.html" >&2
  echo "Run: cd frontend && bun run build" >&2
  exit 1
fi

SERVER_PID=""
cleanup() {
  if [[ -n "${SERVER_PID}" ]] && kill -0 "${SERVER_PID}" >/dev/null 2>&1; then
    kill "${SERVER_PID}" >/dev/null 2>&1 || true
    wait "${SERVER_PID}" >/dev/null 2>&1 || true
  fi
}
trap cleanup EXIT

bunx --bun serve ../out/frontend/site -p "${FRONTEND_PORT}" --no-port-switching >"${SERVE_LOG}" 2>&1 &
SERVER_PID="$!"
sleep 1

bunx --bun cypress run \
  --config-file cypress.config.ts \
  --browser electron \
  --spec "${CORE_SPEC_GLOB}"
