#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"

BIN_NAME=edgerun-unikernel-relay-virtio \
FEATURES=relay-virtio-smoke \
LOG_NAME=qemu-relay-virtio-smoke \
SUCCESS_LINE="edgerun-unikernel relay virtio: poll ok" \
exec bash "$ROOT/crates/edgerun-unikernel/scripts/qemu-relay-wss-smoke.sh" \
  -device virtio-rng-pci,disable-legacy=on \
  -netdev user,id=relaynet \
  -device virtio-net-pci,disable-legacy=on,netdev=relaynet,mac=52:54:00:12:34:56
