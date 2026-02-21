#!/usr/bin/env bash
set -euo pipefail

# Checks SPDX headers across tracked source files in this repository.
# Usage:
#   scripts/spdx-check.sh

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

license_for_path() {
  local path="$1"

  case "$path" in
    crates/edgerun-scheduler/*) echo "LicenseRef-Edgerun-Proprietary" ;;
    crates/edgerun-storage/*) echo "GPL-2.0-only" ;;
    crates/edgerun-cli/*|crates/edgerun-runtime/*|crates/edgerun-worker/*|program/*|docs/*|scripts/*)
      echo "Apache-2.0"
      ;;
    *)
      echo ""
      ;;
  esac
}

is_supported_source() {
  local path="$1"
  case "$path" in
    *.rs|*.ts|*.tsx|*.js|*.jsx|*.mjs|*.cjs|*.sh|*.bash|*.zsh|*.py|*.yml|*.yaml|*.toml)
      return 0
      ;;
    *)
      return 1
      ;;
  esac
}

errors=0

while IFS= read -r path; do
  [[ -z "$path" ]] && continue
  [[ "$path" == *"/LICENSE" || "$path" == "LICENSE" ]] && continue
  is_supported_source "$path" || continue

  expected="$(license_for_path "$path")"
  [[ -z "$expected" ]] && continue

  actual_line="$(head -n 5 "$path" | grep -m1 "SPDX-License-Identifier:" || true)"
  if [[ -z "$actual_line" ]]; then
    echo "missing SPDX header: $path (expected $expected)"
    errors=$((errors + 1))
    continue
  fi

  if [[ "$actual_line" != *"SPDX-License-Identifier: $expected"* ]]; then
    echo "wrong SPDX header: $path"
    echo "  expected: SPDX-License-Identifier: $expected"
    echo "  actual:   $actual_line"
    errors=$((errors + 1))
  fi
done < <(git ls-files)

if [[ "$errors" -gt 0 ]]; then
  echo "SPDX check failed with $errors issue(s)."
  exit 1
fi

echo "SPDX check passed."

