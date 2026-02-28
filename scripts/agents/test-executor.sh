#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
set -euo pipefail

if [[ $# -lt 2 ]]; then
  echo "Usage: scripts/agents/test-executor.sh <WORKSPACE_DIR> <PROFILE>" >&2
  echo "Profiles: quick | frontend | rust-event-bus | node-manager" >&2
  exit 1
fi

WORKSPACE_DIR="$1"
PROFILE="$2"

if [[ ! -d "${WORKSPACE_DIR}" ]]; then
  echo "workspace dir not found: ${WORKSPACE_DIR}" >&2
  exit 1
fi

case "${PROFILE}" in
  quick)
    (cd "${WORKSPACE_DIR}" && cargo check -p edgerun-event-bus -p edgerun-node-manager)
    ;;
  frontend)
    (cd "${WORKSPACE_DIR}/frontend" && bun run check && bun run build)
    ;;
  rust-event-bus)
    (cd "${WORKSPACE_DIR}" && cargo test -p edgerun-event-bus)
    ;;
  node-manager)
    (cd "${WORKSPACE_DIR}" && cargo check -p edgerun-node-manager)
    ;;
  *)
    echo "unsupported profile: ${PROFILE}" >&2
    exit 1
    ;;
esac

echo "test executor profile passed: ${PROFILE}"
