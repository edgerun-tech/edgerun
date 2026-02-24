#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${ROOT_DIR}"

POLICY_FILE=".github/required-checks.txt"
if [[ ! -f "${POLICY_FILE}" ]]; then
  echo "[required-checks] FAIL missing ${POLICY_FILE}"
  exit 1
fi

echo "[required-checks] validating ${POLICY_FILE}"

failures=0
while IFS= read -r line; do
  [[ -z "${line}" || "${line}" =~ ^# ]] && continue
  IFS=':' read -r workflow_file job_id check_name <<<"${line}"

  if [[ -z "${workflow_file}" || -z "${job_id}" || -z "${check_name}" ]]; then
    echo "[required-checks] FAIL malformed entry: ${line}"
    failures=$((failures + 1))
    continue
  fi

  if [[ ! -f "${workflow_file}" ]]; then
    echo "[required-checks] FAIL missing workflow file: ${workflow_file}"
    failures=$((failures + 1))
    continue
  fi

  if ! rg -n --color=never "^  ${job_id}:" "${workflow_file}" >/dev/null; then
    echo "[required-checks] FAIL missing job_id '${job_id}' in ${workflow_file}"
    failures=$((failures + 1))
    continue
  fi

  echo "[required-checks] PASS ${workflow_file}:${job_id} -> ${check_name}"
done < "${POLICY_FILE}"

if [[ "${failures}" -gt 0 ]]; then
  echo "[required-checks] detected ${failures} policy issue(s)"
  exit 1
fi

echo "[required-checks] policy validation passed"
