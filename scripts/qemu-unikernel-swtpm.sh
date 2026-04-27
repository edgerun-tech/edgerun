#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
tpm_dir="${SWTPM_DIR:-/tmp/edgerun-swtpm}"
tpm_socket="${SWTPM_SOCKET:-$tpm_dir/sock}"
tpm_log="${SWTPM_LOG:-$tpm_dir/swtpm.log}"
qemu_log="${QEMU_LOG:-/tmp/edgerun-qemu-swtpm.log}"

if ! command -v swtpm >/dev/null 2>&1; then
    echo "swtpm is not installed or not on PATH" >&2
    exit 1
fi

rm -rf "$tpm_dir"
mkdir -p "$tpm_dir"

swtpm socket \
    --tpm2 \
    --tpmstate "dir=$tpm_dir" \
    --ctrl "type=unixio,path=$tpm_socket" \
    --log "file=$tpm_log,level=20" &
swtpm_pid=$!

cleanup() {
    kill "$swtpm_pid" 2>/dev/null || true
    wait "$swtpm_pid" 2>/dev/null || true
}
trap cleanup EXIT

for _ in {1..50}; do
    if [[ -S "$tpm_socket" ]]; then
        break
    fi
    sleep 0.1
done

if [[ ! -S "$tpm_socket" ]]; then
    echo "swtpm did not create socket: $tpm_socket" >&2
    echo "swtpm log: $tpm_log" >&2
    exit 1
fi

QEMU_TPM_SOCKET="$tpm_socket" \
QEMU_LOG="$qemu_log" \
QEMU_EXPECT="${QEMU_EXPECT:-TPM2 startup ok}" \
QEMU_TIMEOUT="${QEMU_TIMEOUT:-10}" \
    "$repo_root/scripts/qemu-unikernel.sh"
