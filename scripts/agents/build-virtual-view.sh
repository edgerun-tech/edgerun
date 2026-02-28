#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
set -euo pipefail

if [[ $# -lt 1 ]]; then
  echo "Usage: scripts/agents/build-virtual-view.sh <DEST_DIR>" >&2
  exit 1
fi

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
DEST_DIR="$1"
mkdir -p "${DEST_DIR}"

if ! command -v rsync >/dev/null 2>&1; then
  echo "rsync is required" >&2
  exit 1
fi

rsync -a --delete \
  --exclude='.git/' \
  --exclude='.codex/' \
  --exclude='out/' \
  --exclude='target/' \
  --exclude='frontend/node_modules/' \
  --exclude='**/.DS_Store' \
  "${ROOT_DIR}/" "${DEST_DIR}/"

manifest_dir="${DEST_DIR}/.agent-meta"
mkdir -p "${manifest_dir}"
{
  echo "generated_at=$(date -u +%Y-%m-%dT%H:%M:%SZ)"
  echo "source_root=${ROOT_DIR}"
} > "${manifest_dir}/view.env"

( cd "${DEST_DIR}" && find . -type f | LC_ALL=C sort ) > "${manifest_dir}/files.txt"
