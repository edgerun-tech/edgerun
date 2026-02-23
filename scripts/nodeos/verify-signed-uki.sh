#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
set -euo pipefail

uki_path="${1:-}"
cert_pem="${2:-}"
cmdline_file="${3:-}"

if [[ -z "$uki_path" || -z "$cert_pem" || -z "$cmdline_file" ]]; then
  echo "usage: $0 <signed_uki.efi> <cert.pem> <cmdline.locked>" >&2
  exit 1
fi

for cmd in sbverify grep sha256sum; do
  if ! command -v "$cmd" >/dev/null 2>&1; then
    echo "required command not found: $cmd" >&2
    exit 1
  fi
done

sbverify --cert "$cert_pem" "$uki_path" >/dev/null

grep -q 'edgerun.locked_cmdline=1' "$cmdline_file"
grep -q 'api_base=https://api.edgerun.tech' "$cmdline_file"
if grep -qE '(^|[[:space:]])(init=|rdinit=|systemd\.unit=|edgerun\.insecure=)' "$cmdline_file"; then
  echo "forbidden cmdline token found" >&2
  exit 1
fi

echo "verification=ok"
echo "uki_sha256=$(sha256sum "$uki_path" | awk '{print $1}')"
