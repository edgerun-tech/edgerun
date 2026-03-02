#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
MODE="${1:---staged}"

if [[ "${MODE}" != "--staged" && "${MODE}" != "--all" ]]; then
  echo "usage: scripts/scan-secrets.sh [--staged|--all]" >&2
  exit 2
fi

cd "${ROOT_DIR}"

collect_files() {
  if [[ "${MODE}" == "--staged" ]]; then
    git diff --cached --name-only -z --diff-filter=ACMR
  else
    git ls-files -z
  fi
}

is_scan_target() {
  local file="$1"
  [[ -f "${file}" ]] || return 1
  [[ "${file}" == out/* ]] && return 1
  [[ "${file}" == target/* ]] && return 1
  [[ "${file}" == frontend/node_modules/* ]] && return 1
  [[ "${file}" == .git/* ]] && return 1

  # Skip obvious binaries for fast, lower-noise scans.
  if grep -q $'\x00' "${file}" 2>/dev/null; then
    return 1
  fi

  return 0
}

patterns=(
  'AKIA[0-9A-Z]{16}'
  'ASIA[0-9A-Z]{16}'
  'ghp_[A-Za-z0-9]{36}'
  'github_pat_[A-Za-z0-9_]{20,}'
  'xox[baprs]-[A-Za-z0-9-]{10,}'
  'sk-[A-Za-z0-9]{20,}'
  'AIza[0-9A-Za-z_-]{35}'
  '-----BEGIN (RSA|EC|OPENSSH|DSA|PGP)? ?PRIVATE KEY-----'
)

tmp_findings="$(mktemp)"
trap 'rm -f "${tmp_findings}"' EXIT

has_findings=0
while IFS= read -r -d '' file; do
  is_scan_target "${file}" || continue
  for pattern in "${patterns[@]}"; do
    if grep -nE "${pattern}" "${file}" >>"${tmp_findings}"; then
      has_findings=1
    fi
  done
done < <(collect_files)

if [[ "${has_findings}" -ne 0 ]]; then
  echo "credential sweep failed: potential secrets detected" >&2
  echo "review and remove before committing/pushing:" >&2
  sed 's/^/  /' "${tmp_findings}" >&2
  exit 1
fi

echo "credential sweep passed (${MODE})"
