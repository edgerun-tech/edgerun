#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
set -euo pipefail

repo_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)"
out_dir="${1:-$repo_root/out/nodeos}"
kernel_image="${KERNEL_IMAGE:-}"
initrd_image="${INITRD_IMAGE:-}"
os_release_file="${OS_RELEASE_FILE:-/usr/lib/os-release}"
cmdline_file="${CMDLINE_FILE:-$out_dir/cmdline.locked}"
unsigned_uki="$out_dir/edgerun-node.unsigned.efi"
signed_uki="$out_dir/edgerun-node.signed.efi"
cert_pem="${EDGERUN_SB_CERT_PEM:-$out_dir/secureboot/edgerun-secureboot-db-cert.pem}"
pkcs11_uri="${EDGERUN_SB_PKCS11_URI:-}"
stub="${UKI_STUB:-/usr/lib/systemd/boot/efi/linuxx64.efi.stub}"
api_base="${EDGERUN_API_BASE:-https://api.edgerun.tech}"
owner_pubkey="${EDGERUN_OWNER_PUBKEY:-}"

for cmd in ukify sbsign sbverify sha256sum mkdir grep; do
  if ! command -v "$cmd" >/dev/null 2>&1; then
    echo "required command not found: $cmd" >&2
    exit 1
  fi
done

if [[ -z "$kernel_image" ]]; then
  echo "KERNEL_IMAGE must be set to a built kernel image (e.g., bzImage)" >&2
  exit 1
fi

if [[ ! -f "$kernel_image" ]]; then
  echo "kernel image not found: $kernel_image" >&2
  exit 1
fi

if [[ -n "$initrd_image" && ! -f "$initrd_image" ]]; then
  echo "initrd image not found: $initrd_image" >&2
  exit 1
fi

if [[ ! -f "$os_release_file" ]]; then
  echo "os-release not found: $os_release_file" >&2
  exit 1
fi

if [[ ! -f "$cert_pem" ]]; then
  echo "secure-boot certificate not found: $cert_pem" >&2
  exit 1
fi

if [[ -z "$pkcs11_uri" ]]; then
  cat >&2 <<ERR
EDGERUN_SB_PKCS11_URI is required.
Example:
  export EDGERUN_SB_PKCS11_URI='pkcs11:token=YubiKey%20PIV;id=%02;type=private'
ERR
  exit 1
fi

mkdir -p "$out_dir"

locked_cmdline="console=ttyS0,115200 loglevel=3 edgerun.locked_cmdline=1 api_base=${api_base}"
if [[ -n "$owner_pubkey" ]]; then
  locked_cmdline="$locked_cmdline owner_pubkey=${owner_pubkey}"
fi
printf '%s\n' "$locked_cmdline" >"$cmdline_file"

if grep -qE '(^|[[:space:]])(init=|rdinit=|systemd\.unit=|edgerun\.insecure=)' "$cmdline_file"; then
  echo "locked cmdline contains forbidden flags" >&2
  exit 1
fi

ukify_args=(
  build
  --linux "$kernel_image"
  --stub "$stub"
  --os-release "$os_release_file"
  --cmdline "@$cmdline_file"
  --output "$unsigned_uki"
)

if [[ -n "$initrd_image" ]]; then
  ukify_args+=(--initrd "$initrd_image")
fi

ukify "${ukify_args[@]}"

sbsign --engine pkcs11 --key "$pkcs11_uri" --cert "$cert_pem" --output "$signed_uki" "$unsigned_uki"

sbverify --cert "$cert_pem" "$signed_uki" >/dev/null

echo "unsigned_uki=$unsigned_uki"
echo "signed_uki=$signed_uki"
echo "locked_cmdline_file=$cmdline_file"
echo "signed_uki_sha256=$(sha256sum "$signed_uki" | awk '{print $1}')"
