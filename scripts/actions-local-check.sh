#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
set -euo pipefail

if ! command -v act >/dev/null 2>&1; then
  echo "act is required. Install from https://github.com/nektos/act" >&2
  exit 1
fi

WORKFLOWS=(
  "ci.yml:push"
  "codeql.yml:push"
  "dependency-review.yml:pull_request"
  "docker-images.yml:push"
  "frontend-release.yml:push"
  "push-scheduler.yml:workflow_dispatch"
  "release.yml:push"
  "runtime-compliance-matrix.yml:workflow_dispatch"
  "runtime-provenance.yml:push"
  "site-pages.yml:push"
  "wiki-sync.yml:workflow_dispatch"
)

ACT_TIMEOUT_SECONDS="${ACT_TIMEOUT_SECONDS:-300}"
FAILED=0
LOG_DIR="${ACT_LOG_DIR:-out/actions-local}"
mkdir -p "${LOG_DIR}"

run_with_timeout() {
  local timeout_s="$1"
  shift
  if command -v timeout >/dev/null 2>&1; then
    timeout "${timeout_s}" "$@"
    return $?
  fi
  "$@"
}

for entry in "${WORKFLOWS[@]}"; do
  IFS=':' read -r workflow event <<<"$entry"
  wf_path=".github/workflows/${workflow}"
  if [[ ! -f "${wf_path}" ]]; then
    echo "SKIP ${workflow} (missing ${wf_path})"
    continue
  fi

  echo
  echo "=== ${workflow} (${event}) ==="

  log_file="${LOG_DIR}/${workflow%.yml}.log"

  if run_with_timeout "${ACT_TIMEOUT_SECONDS}" act -W "${wf_path}" "${event}" -n >"${log_file}" 2>&1; then
    echo "PASS ${workflow}"
  else
    rc=$?
    echo "FAIL ${workflow} (exit=${rc})"
    tail -n 30 "${log_file}" || true
    FAILED=1
  fi
done

if [[ "${FAILED}" -ne 0 ]]; then
  echo
  echo "One or more local workflow checks failed."
  exit 1
fi

echo

echo "All local workflow checks passed."
