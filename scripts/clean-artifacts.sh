#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

remove_tree() {
  local dir="$1"
  if [ ! -d "$dir" ]; then
    return 0
  fi
  find "$dir" -depth -mindepth 1 -delete
  rmdir "$dir" 2>/dev/null || true
}

remove_file() {
  local file="$1"
  if [ -f "$file" ]; then
    rm -f "$file"
  fi
}

# Canonical output roots
remove_tree "out"

# Local roots outside canonical out/
remove_tree "target"
remove_tree "test-ledger"
remove_tree "frontend/test-results"
remove_tree "frontend/playwright-report"

remove_file "solana-validator.log"

echo "cleaned artifacts"
