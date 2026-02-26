#!/usr/bin/env bash
set -euo pipefail

ROUNDS="${ROUNDS:-20}"
IMAGE="${IMAGE:-docker.io/library/alpine:3.20}"
NAMESPACE="${NAMESPACE:-default}"
ARTIFACT_DIR="${ARTIFACT_DIR:-out/containerd-soak}"

if ! command -v ctr >/dev/null 2>&1; then
  echo "error: ctr not found"
  exit 2
fi

passes=0
fails=0
started_at="$(date -u +"%Y-%m-%dT%H:%M:%SZ")"
mkdir -p "${ARTIFACT_DIR}"
summary_file="${ARTIFACT_DIR}/summary.txt"
failures_file="${ARTIFACT_DIR}/failures.log"
: >"${summary_file}"
: >"${failures_file}"

{
  echo "started_at=${started_at}"
  echo "rounds=${ROUNDS}"
  echo "image=${IMAGE}"
  echo "namespace=${NAMESPACE}"
} >>"${summary_file}"

for i in $(seq 1 "${ROUNDS}"); do
  echo "round=${i}/${ROUNDS}"
  set +e
  out="$(NAMESPACE="${NAMESPACE}" ./scripts/containerd-runtime-matrix-smoke.sh "${IMAGE}" 2>&1)"
  code=$?
  set -e

  if [[ ${code} -eq 0 ]]; then
    passes=$((passes + 1))
    echo "round_result=pass"
    echo "round=${i} result=pass" >>"${summary_file}"
  else
    fails=$((fails + 1))
    echo "round_result=fail"
    echo "${out}" | tail -n 40
    echo "round=${i} result=fail" >>"${summary_file}"
    {
      echo "----- round=${i} begin -----"
      echo "${out}"
      echo "----- round=${i} end -----"
    } >>"${failures_file}"
    printf '%s\n' "${out}" >"${ARTIFACT_DIR}/round-${i}.log"
  fi
done

finished_at="$(date -u +"%Y-%m-%dT%H:%M:%SZ")"
echo "finished_at=${finished_at}" >>"${summary_file}"
echo "soak_rounds=${ROUNDS} pass=${passes} fail=${fails}" | tee -a "${summary_file}"
echo "soak_artifacts=${ARTIFACT_DIR}"
if [[ ${fails} -ne 0 ]]; then
  exit 1
fi
