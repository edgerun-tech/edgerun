#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<USAGE
Usage:
  $0 --pcr-list sha256:7 --session ./var/tpm-policy/signing.session [--tpm-path /dev/tpmrm0]

Creates a live TPM policy session bound to the requested PCR selection and leaves
it on disk for tools like edgerund/tpm2_sign to use via:
  session:<session-path>

Cleanup when finished:
  tpm2_flushcontext <session-path>
USAGE
}

pcr_list=""
session=""
tpm_path=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    --pcr-list) pcr_list="$2"; shift 2 ;;
    --session) session="$2"; shift 2 ;;
    --tpm-path) tpm_path="$2"; shift 2 ;;
    -h|--help) usage; exit 0 ;;
    *) echo "unknown arg: $1" >&2; usage; exit 1 ;;
  esac
done

[[ -n "$pcr_list" && -n "$session" ]] || { usage; exit 1; }
mkdir -p "$(dirname "$session")"
rm -f "$session"

tpm_args=()
if [[ -n "$tpm_path" ]]; then
  tpm_args=(-T "device:$tpm_path")
fi

tpm2_startauthsession -Q "${tpm_args[@]}" --policy-session -S "$session"
tpm2_policypcr -Q "${tpm_args[@]}" -S "$session" -l "$pcr_list"

printf 'Session ready: %s\nUse with: --tpm-key-auth session:%s\n' "$session" "$session"
