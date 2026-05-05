#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
qemu_log="${QEMU_LOG:-/tmp/edgerun-qemu-virtio-smoke.log}"
console_log="${QEMU_CONSOLE_LOG:-/tmp/edgerun-qemu-virtio-smoke-console.log}"

rm -f "$qemu_log" "$console_log"

QEMU_TIMEOUT="${QEMU_TIMEOUT:-10}" \
QEMU_LOG="$qemu_log" \
QEMU_CONSOLE_LOG="$console_log" \
QEMU_EXPECT="${QEMU_EXPECT:-VirtIO net init ok}" \
    "$repo_root/scripts/qemu-unikernel.sh"

required_markers=(
    "RNG mixed VirtIO entropy"
    "VirtIO console init ok"
    "VirtIO block init ok"
    "VirtIO block first sector read ok"
    "VirtIO storage adapter read ok"
    "VirtIO net init ok"
    "DHCP lease accepted"
)

for marker in "${required_markers[@]}"; do
    if ! grep -q "$marker" "$qemu_log"; then
        echo "QEMU VirtIO smoke missing marker: $marker" >&2
        echo "Serial log: $qemu_log" >&2
        exit 1
    fi
done

if ! grep -q "edgerun: virtio-console online" "$console_log"; then
    echo "QEMU VirtIO smoke did not observe virtio-console write" >&2
    echo "Console log: $console_log" >&2
    exit 1
fi

echo "QEMU VirtIO smoke reached all markers"
