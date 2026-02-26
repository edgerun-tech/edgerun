#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${ROOT_DIR}"

echo "[drift] scanning operational workflow files for package-manager drift"

failures=0

check_pattern() {
  local label="$1"
  local pattern="$2"
  shift 2
  local files=("$@")
  if rg -n --color=never "${pattern}" "${files[@]}"; then
    echo "[drift] FAIL (${label}): found disallowed pattern '${pattern}'"
    failures=$((failures + 1))
  else
    echo "[drift] PASS (${label})"
  fi
}

workflow_files=(
  "Makefile"
  "scripts/verify-cloudflare-targets.sh"
  "scripts/actions-local-check.sh"
  "scripts/actions-local-run.sh"
  "scripts/e2e-local-terminal.sh"
  "frontend/package.json"
  "package.json"
  "program/package.json"
  "program/Anchor.toml"
)

check_pattern "package manager commands" "\\b(npm|pnpm|yarn|npx)\\b" "${workflow_files[@]}"
check_pattern "anchor package manager" '^package_manager\\s*=\\s*"npm"$' "program/Anchor.toml"

if rg -n --color=never "wrangler deploy" "scripts/deploy-frontends.sh" | rg -v -- "--config"; then
  echo "[drift] FAIL (wrangler deploy pinning): found wrangler deploy without --config"
  failures=$((failures + 1))
else
  echo "[drift] PASS (wrangler deploy pinning)"
fi

./scripts/verify-cloudflare-targets.sh

if [[ "${failures}" -gt 0 ]]; then
  echo "[drift] detected ${failures} workflow drift issue(s)"
  exit 1
fi

echo "[drift] no workflow drift detected"
