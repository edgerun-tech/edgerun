#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
set -euo pipefail

slot="${1:-9c}"
out_dir="${2:-$(pwd)/out/nodeos/secureboot}"
subject="${EDGERUN_SECUREBOOT_SUBJECT:-CN=EdgeRun Secure Boot DB}"

for cmd in ykman openssl mkdir; do
  if ! command -v "$cmd" >/dev/null 2>&1; then
    echo "required command not found: $cmd" >&2
    exit 1
  fi
done

mkdir -p "$out_dir"
pubkey="$out_dir/yubikey-slot-${slot}-pubkey.pem"
cert="$out_dir/edgerun-secureboot-db-cert.pem"

# Generates a non-exportable private key on the YubiKey PIV slot.
ykman piv keys generate --algorithm RSA2048 "$slot" "$pubkey"

# Issues a self-signed cert anchored to the hardware-backed key.
ykman piv certificates generate --subject "$subject" "$slot" "$pubkey"

# Export cert for sbverify / enrollment artifacts.
ykman piv certificates export "$slot" "$cert"

openssl x509 -in "$cert" -noout -subject -issuer -fingerprint -sha256

echo "yubikey_slot=$slot"
echo "public_key=$pubkey"
echo "certificate=$cert"
