#!/usr/bin/env bash
set -euo pipefail

IMAGE="${1:-docker.io/library/alpine:3.20}"
NAMESPACE="${NAMESPACE:-default}"
ALLOW_FAIL="${ALLOW_FAIL:-0}"

if ! command -v ctr >/dev/null 2>&1; then
  echo "error: ctr not found"
  exit 2
fi

run_case() {
  local runtime="$1"
  local name="$2"
  local id="edgerun-smoke-${name}-$(date +%s)"
  local status=0
  local output=""

  set +e
  output="$(sudo ctr -n "${NAMESPACE}" run --rm --runtime "${runtime}" "${IMAGE}" "${id}" /bin/sh -lc "echo runtime=${name} ok" 2>&1)"
  status=$?
  set -e

  if [[ $status -eq 0 ]]; then
    if ! grep -q "runtime=${name} ok" <<<"${output}"; then
      echo "FAIL runtime=${runtime} image=${IMAGE} id=${id}"
      echo "expected marker runtime=${name} ok not found in output"
      echo "${output}" | tail -n 20
      return 1
    fi
    echo "PASS runtime=${runtime} image=${IMAGE} id=${id}"
    echo "${output}" | tail -n 1
    return 0
  fi

  echo "FAIL runtime=${runtime} image=${IMAGE} id=${id}"
  echo "${output}" | tail -n 20
  return 1
}

failures=0

run_case "io.containerd.runc.v2" "oci-crun-default" || failures=$((failures + 1))
run_case "io.containerd.edgerun.v1" "edgerun-wasi" || failures=$((failures + 1))

if [[ "${failures}" -ne 0 ]]; then
  echo "matrix_result=failed failures=${failures}"
  if [[ "${ALLOW_FAIL}" -eq 1 ]]; then
    exit 0
  fi
  exit 1
fi

echo "matrix_result=passed failures=0"
