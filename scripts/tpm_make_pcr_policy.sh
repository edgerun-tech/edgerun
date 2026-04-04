#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<USAGE
Usage:
  $0 --pcr-list sha256:7 --out-dir ./var/tpm-policy [--tpm-path /dev/tpmrm0]

Builds a reusable TPM PCR policy digest for key provisioning.
Outputs:
  <out-dir>/policy.digest
USAGE
}

pcr_list=""
out_dir=""
tpm_path=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    --pcr-list) pcr_list="$2"; shift 2 ;;
    --out-dir) out_dir="$2"; shift 2 ;;
    --tpm-path) tpm_path="$2"; shift 2 ;;
    -h|--help) usage; exit 0 ;;
    *) echo "unknown arg: $1" >&2; usage; exit 1 ;;
  esac
done

[[ -n "$pcr_list" && -n "$out_dir" ]] || { usage; exit 1; }
mkdir -p "$out_dir"
policy="$out_dir/policy.digest"
session="$out_dir/policy-build.session"
rm -f "$policy" "$session"

tpm_args=()
if [[ -n "$tpm_path" ]]; then
  tpm_args=(-T "device:$tpm_path")
fi

cleanup() {
  if [[ -f "$session" ]]; then
    tpm2_flushcontext "${tpm_args[@]}" "$session" >/dev/null 2>&1 || true
    rm -f "$session"
  fi
}
trap cleanup EXIT

tpm2_startauthsession -Q "${tpm_args[@]}" --policy-session -S "$session"
tpm2_policypcr -Q "${tpm_args[@]}" -S "$session" -l "$pcr_list" -L "$policy"

printf 'Policy digest: %s\n' "$policy"
