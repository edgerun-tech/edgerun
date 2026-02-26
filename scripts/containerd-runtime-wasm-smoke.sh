#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
NAMESPACE="${NAMESPACE:-default}"
IMAGE="${IMAGE:-docker.io/library/debian:bookworm-slim}"
RUNTIME_BIN="${RUNTIME_BIN:-/var/cache/build/rust/target/release/edgerun-runtime}"
RUNTIME_CLASS="${RUNTIME_CLASS:-io.containerd.edgerun.v1}"
RUNS="${RUNS:-1}"
ID="${ID:-edgerun-wasm-smoke-$(date +%s)-$$-${RANDOM}}"
ARTIFACT_PATH="${ARTIFACT_PATH:-/tmp/er-artifact.json}"
SNAPSHOTTER="${SNAPSHOTTER:-}"

if [[ ! -x "${RUNTIME_BIN}" ]]; then
  (
    cd "${ROOT_DIR}"
    cargo build --release -p edgerun-runtime --features cli
  )
fi

sudo ctr --namespace "${NAMESPACE}" image pull "${IMAGE}" >/dev/null 2>&1 || true

cleanup() {
  sudo ctr --namespace "${NAMESPACE}" task kill -s 15 "${ID}" >/dev/null 2>&1 || true
  sudo ctr --namespace "${NAMESPACE}" task rm -f "${ID}" >/dev/null 2>&1 || true
  sudo ctr --namespace "${NAMESPACE}" container rm "${ID}" >/dev/null 2>&1 || true
  sudo ctr --namespace "${NAMESPACE}" snapshot rm "${ID}" >/dev/null 2>&1 || true
}

trap cleanup EXIT

set +e
snapshotter_args=()
if [[ -n "${SNAPSHOTTER}" ]]; then
  snapshotter_args=(--snapshotter "${SNAPSHOTTER}")
fi

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

if [[ ${rc} -ne 0 ]] && grep -q "all_passed=true" <<<"${raw_out}" \
  && grep -q "failed precondition" <<<"${raw_out}"; then
  rc=0
fi

if [[ ${rc} -ne 0 ]]; then
  echo "FAIL wasm-runtime-smoke id=${ID} exit=${rc}" >&2
  exit "${rc}"
fi

if ! grep -q "all_passed=true" <<<"${out}"; then
  echo "FAIL wasm-runtime-smoke id=${ID} exit=${rc}" >&2
  exit 1
fi

echo "PASS wasm-runtime-smoke id=${ID} runtime=${RUNTIME_CLASS}"
