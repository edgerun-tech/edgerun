#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${ROOT_DIR}"

echo "[actions-local-list] validating workflow entries in scripts/actions-local-check.sh"

failures=0
mapfile -t entries < <(rg -o '"[^\"]+\.yml:[^\"]+"' scripts/actions-local-check.sh | tr -d '"' | sort -u)

for entry in "${entries[@]}"; do
  workflow="${entry%%:*}"
  wf_path=".github/workflows/${workflow}"
  if [[ -f "${wf_path}" ]]; then
    echo "[actions-local-list] PASS ${workflow}"
  else
    echo "[actions-local-list] FAIL ${workflow} (missing ${wf_path})"
    failures=$((failures + 1))
  fi
done

if [[ "${failures}" -gt 0 ]]; then
  echo "[actions-local-list] detected ${failures} missing workflow file(s)"
  exit 1
fi

echo "[actions-local-list] all referenced workflow files exist"
