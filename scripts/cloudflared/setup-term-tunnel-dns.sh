#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
set -euo pipefail

TUNNEL_ID="${EDGERUN_TERM_TUNNEL_ID:-c5fab4c7-c68e-411e-880b-c1da359a18e9}"
TUNNEL_HOSTNAME="${EDGERUN_TERM_TUNNEL_HOSTNAME:-term.edgerun.tech}"

if ! command -v cloudflared >/dev/null 2>&1; then
  echo "cloudflared is required but not found on PATH" >&2
  exit 1
fi

cloudflared tunnel route dns "${TUNNEL_ID}" "${TUNNEL_HOSTNAME}"
echo "DNS route ensured: ${TUNNEL_HOSTNAME} -> tunnel ${TUNNEL_ID}"
