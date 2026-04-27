#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
tpm_dir="${SWTPM_DIR:-/tmp/edgerun-swtpm}"
tpm_socket="${SWTPM_SOCKET:-$tpm_dir/sock}"
tpm_log="${SWTPM_LOG:-$tpm_dir/swtpm.log}"
tpm_server_port="${SWTPM_SERVER_PORT:-$((23210 + $$ % 1000))}"
tpm_ctrl_port="${SWTPM_CTRL_PORT:-$((tpm_server_port + 1))}"
provision="${SWTPM_PROVISION:-1}"
provision_handle="${SWTPM_PROVISION_HANDLE:-0x81000001}"
qemu_log="${QEMU_LOG:-/tmp/edgerun-qemu-swtpm.log}"

if ! command -v swtpm >/dev/null 2>&1; then
    echo "swtpm is not installed or not on PATH" >&2
    exit 1
fi

if [[ "$provision" == "1" ]]; then
    for tool in tpm2_createprimary tpm2_evictcontrol tpm2_readpublic; do
        if ! command -v "$tool" >/dev/null 2>&1; then
            echo "$tool is required for SWTPM_PROVISION=1" >&2
            exit 1
        fi
    done
fi

rm -rf "$tpm_dir"
mkdir -p "$tpm_dir"

if [[ "$provision" == "1" ]]; then
    primary_ctx="$tpm_dir/primary.ctx"
    tcti="swtpm:host=127.0.0.1,port=$tpm_server_port"

    swtpm socket \
        --tpm2 \
        --tpmstate "dir=$tpm_dir" \
        --server "type=tcp,port=$tpm_server_port,bindaddr=127.0.0.1" \
        --ctrl "type=tcp,port=$tpm_ctrl_port,bindaddr=127.0.0.1" \
        --flags "not-need-init,startup-clear" \
        --log "file=$tpm_log,level=20" &
    provision_pid=$!

    TPM2TOOLS_TCTI="$tcti" tpm2_createprimary \
        -C o \
        -G ecc \
        -g sha256 \
        -a "fixedtpm|fixedparent|sensitivedataorigin|userwithauth|sign" \
        -c "$primary_ctx" \
        >/dev/null
    TPM2TOOLS_TCTI="$tcti" tpm2_evictcontrol \
        -C o \
        -c "$primary_ctx" \
        "$provision_handle" \
        >/dev/null
    TPM2TOOLS_TCTI="$tcti" tpm2_readpublic \
        -c "$provision_handle" \
        >/dev/null

    kill "$provision_pid" 2>/dev/null || true
    wait "$provision_pid" 2>/dev/null || true
fi

swtpm socket \
    --tpm2 \
    --tpmstate "dir=$tpm_dir" \
    --ctrl "type=unixio,path=$tpm_socket" \
    --flags "not-need-init,startup-clear" \
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
QEMU_EXPECT="${QEMU_EXPECT:-TPM2 sign ok}" \
QEMU_TIMEOUT="${QEMU_TIMEOUT:-10}" \
    "$repo_root/scripts/qemu-unikernel.sh"
