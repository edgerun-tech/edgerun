#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CRATE_DIR="${ROOT_DIR}/crates/edgerun-runtime-web"
OUT_DIR="${CRATE_DIR}/pkg-web"
FRONTEND_MIRROR_DIR="${ROOT_DIR}/frontend/public/wasm/edgerun-runtime-web"

if ! command -v wasm-pack >/dev/null 2>&1; then
  echo "wasm-pack is required. Install from https://rustwasm.github.io/wasm-pack/installer/" >&2
  exit 1
fi

pushd "${CRATE_DIR}" >/dev/null
wasm-pack build --target web --release --out-dir pkg-web
popd >/dev/null

if [[ -d "${ROOT_DIR}/frontend" ]]; then
  rm -rf "${FRONTEND_MIRROR_DIR}"
  mkdir -p "${FRONTEND_MIRROR_DIR}"
  cp -a "${OUT_DIR}/." "${FRONTEND_MIRROR_DIR}/"
fi

echo "runtime-web output: ${OUT_DIR}"
if [[ -d "${ROOT_DIR}/frontend" ]]; then
  echo "runtime-web frontend mirror: ${FRONTEND_MIRROR_DIR}"
fi
