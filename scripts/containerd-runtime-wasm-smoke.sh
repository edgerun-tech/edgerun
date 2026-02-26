#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
NAMESPACE="${NAMESPACE:-default}"
IMAGE="${IMAGE:-docker.io/library/debian:bookworm-slim}"
RUNTIME_BIN="${RUNTIME_BIN:-}"
RUNTIME_CLASS="${RUNTIME_CLASS:-io.containerd.edgerun.v1}"
RUNS="${RUNS:-1}"
RETRIES="${RETRIES:-10}"
ID_BASE="${ID:-edgerun-wasm-smoke-$(date +%s)-$$-${RANDOM}}"
ARTIFACT_PATH="${ARTIFACT_PATH:-/tmp/er-artifact.json}"
SNAPSHOTTER="${SNAPSHOTTER:-}"

if [[ -z "${RUNTIME_BIN}" ]]; then
  for candidate in \
    "${ROOT_DIR}/out/target/release/edgerun-runtime" \
    "/usr/bin/edgerun-runtime" \
    "${ROOT_DIR}/target/release/edgerun-runtime"; do
    if [[ -x "${candidate}" ]]; then
      RUNTIME_BIN="${candidate}"
      break
    fi
  done
fi

if [[ -z "${RUNTIME_BIN}" || ! -x "${RUNTIME_BIN}" ]]; then
  (
    cd "${ROOT_DIR}"
    CARGO_TARGET_DIR="${ROOT_DIR}/out/target" cargo build --release -p edgerun-runtime --features cli
  )
  RUNTIME_BIN="${ROOT_DIR}/out/target/release/edgerun-runtime"
fi

sudo ctr --namespace "${NAMESPACE}" image pull "${IMAGE}" >/dev/null 2>&1 || true

ID="${ID_BASE}"

cleanup() {
  sudo ctr --namespace "${NAMESPACE}" task kill -s 15 "${ID}" >/dev/null 2>&1 || true
  sudo ctr --namespace "${NAMESPACE}" task rm -f "${ID}" >/dev/null 2>&1 || true
  sudo ctr --namespace "${NAMESPACE}" container rm "${ID}" >/dev/null 2>&1 || true
  sudo ctr --namespace "${NAMESPACE}" snapshot rm "${ID}" >/dev/null 2>&1 || true
}

trap cleanup EXIT

snapshotter_args=()
if [[ -n "${SNAPSHOTTER}" ]]; then
  snapshotter_args=(--snapshotter "${SNAPSHOTTER}")
fi

success=0
for attempt in $(seq 1 "${RETRIES}"); do
  ID="${ID_BASE}-${attempt}"
  set +e
  raw_out="$(
    sudo ctr --namespace "${NAMESPACE}" run \
      "${snapshotter_args[@]}" \
      --runtime "${RUNTIME_CLASS}" \
      --mount "type=bind,src=${RUNTIME_BIN},dst=/usr/local/bin/edgerun-runtime,options=rbind:ro" \
      "${IMAGE}" "${ID}" \
      /usr/local/bin/edgerun-runtime replay-corpus \
      --profile smoke \
      --artifact "${ARTIFACT_PATH}" \
      --runs "${RUNS}" 2>&1
  )"
  rc=$?
  set -e

  out="${raw_out//$'ctr: task must be stopped before deletion: running: failed precondition'/}"
  printf '%s\n' "${out}"

  if [[ ${rc} -eq 0 ]] && grep -q "all_passed=true" <<<"${out}"; then
    success=1
    break
  fi

  if [[ ${rc} -ne 0 ]] && grep -q "all_passed=true" <<<"${raw_out}" \
    && grep -q "failed precondition" <<<"${raw_out}"; then
    success=1
    break
  fi

  cleanup
  if [[ ${attempt} -lt ${RETRIES} ]]; then
    if [[ -z "${raw_out}" ]]; then
      echo "WARN wasm-runtime-smoke empty ctr output; tailing logs for diagnostics" >&2
      sudo tail -n 20 /tmp/containerd.log >&2 || true
      sudo tail -n 20 /tmp/edgerun-shim-backend.log >&2 || true
    fi
    echo "WARN wasm-runtime-smoke retry attempt=${attempt} id=${ID} exit=${rc}" >&2
    sleep 1
  fi
done

if [[ ${success} -ne 1 ]]; then
  echo "FAIL wasm-runtime-smoke id=${ID} exit=${rc}" >&2
  exit "${rc}"
fi

echo "PASS wasm-runtime-smoke id=${ID} runtime=${RUNTIME_CLASS}"
