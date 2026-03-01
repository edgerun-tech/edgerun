#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SPEC_DIR="${ROOT_DIR}/docs/specs"
KEEP_COUNT=5
STAGE=false

for arg in "$@"; do
  case "${arg}" in
    --stage)
      STAGE=true
      ;;
    --keep=*)
      KEEP_COUNT="${arg#--keep=}"
      ;;
    *)
      ;;
  esac
done

if ! [[ "${KEEP_COUNT}" =~ ^[0-9]+$ ]]; then
  echo "[prune-change-specs] invalid --keep value: ${KEEP_COUNT}" >&2
  exit 1
fi

if [[ ! -d "${SPEC_DIR}" ]]; then
  exit 0
fi

mapfile -t files < <(ls -1t "${SPEC_DIR}"/????-??-??-*.md 2>/dev/null || true)

if (( ${#files[@]} <= KEEP_COUNT )); then
  exit 0
fi

for ((i=KEEP_COUNT; i<${#files[@]}; i++)); do
  rm -f "${files[$i]}"
done

if [[ "${STAGE}" == "true" ]]; then
  git -C "${ROOT_DIR}" add -A docs/specs
fi
