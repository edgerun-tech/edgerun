#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
set -euo pipefail

if ! command -v act >/dev/null 2>&1; then
  echo "act is required. Install from https://github.com/nektos/act" >&2
  exit 1
fi

WORKFLOW="${ACT_WORKFLOW:-ci.yml}"
EVENT="${ACT_EVENT:-pull_request}"
JOB="${ACT_JOB:-}"
DRY_RUN="${ACT_DRY_RUN:-0}"
ACT_TIMEOUT_SECONDS="${ACT_TIMEOUT_SECONDS:-3600}"
LOG_DIR="${ACT_LOG_DIR:-out/actions-local}"

usage() {
  cat <<USAGE
Usage: scripts/actions-local-run.sh [options]

Options:
  --workflow <file>   Workflow filename under .github/workflows (default: ci.yml)
  --event <event>     Event name for act (default: pull_request)
  --job <job-id>      Optional specific job id
  --dry-run           Use act dry-run mode (-n)
  --list              List local workflow files and exit
  -h, --help          Show this help

Environment overrides:
  ACT_WORKFLOW, ACT_EVENT, ACT_JOB, ACT_DRY_RUN, ACT_TIMEOUT_SECONDS, ACT_LOG_DIR
USAGE
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --workflow)
      WORKFLOW="${2:?missing value for --workflow}"
      shift 2
      ;;
    --event)
      EVENT="${2:?missing value for --event}"
      shift 2
      ;;
    --job)
      JOB="${2:?missing value for --job}"
      shift 2
      ;;
    --dry-run)
      DRY_RUN="1"
      shift
      ;;
    --list)
      ls -1 .github/workflows/*.yml
      exit 0
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "Unknown option: $1" >&2
      usage >&2
      exit 1
      ;;
  esac
done

WF_PATH=".github/workflows/${WORKFLOW}"
if [[ ! -f "${WF_PATH}" ]]; then
  echo "Workflow not found: ${WF_PATH}" >&2
  exit 1
fi

mkdir -p "${LOG_DIR}"
run_stamp="$(date +%Y%m%d-%H%M%S)"
log_file="${LOG_DIR}/${WORKFLOW%.yml}-${EVENT}-${run_stamp}.log"

cmd=(act -W "${WF_PATH}" "${EVENT}")
if [[ -n "${JOB}" ]]; then
  cmd+=(-j "${JOB}")
fi
if [[ "${DRY_RUN}" == "1" ]]; then
  cmd+=(-n)
fi

run_with_timeout() {
  local timeout_s="$1"
  shift
  if command -v timeout >/dev/null 2>&1; then
    timeout "${timeout_s}" "$@"
    return $?
  fi
  "$@"
}

echo "workflow=${WORKFLOW}"
echo "event=${EVENT}"
if [[ -n "${JOB}" ]]; then
  echo "job=${JOB}"
fi
if [[ "${DRY_RUN}" == "1" ]]; then
  echo "mode=dry-run"
else
  echo "mode=execute"
fi
echo "log=${log_file}"

if run_with_timeout "${ACT_TIMEOUT_SECONDS}" "${cmd[@]}" | tee "${log_file}"; then
  echo "local action run passed"
else
  rc=$?
  echo "local action run failed (exit=${rc})" >&2
  exit "${rc}"
fi
