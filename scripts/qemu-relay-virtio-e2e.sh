#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
exec bash "$ROOT/crates/edgerun-unikernel/scripts/qemu-relay-virtio-e2e.sh" "$@"
