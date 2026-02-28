#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
export CARGO_TARGET_DIR="${ROOT_DIR}/out/target"
OUT_DIR="${ROOT_DIR}/docker/node-manager/bin"

mkdir -p "${OUT_DIR}"

echo "building host binaries into ${CARGO_TARGET_DIR}/release"
cargo build --release -p edgerun-node-manager -p edgerun-worker

install -m0755 "${CARGO_TARGET_DIR}/release/edgerun-node-manager" "${OUT_DIR}/edgerun-node-manager"
install -m0755 "${CARGO_TARGET_DIR}/release/edgerun-worker" "${OUT_DIR}/edgerun-worker"

echo "prepared image binaries in ${OUT_DIR}"
