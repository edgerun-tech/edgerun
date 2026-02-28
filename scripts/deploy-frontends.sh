#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TARGET="${1:-all}"

cd "${ROOT_DIR}"
./scripts/verify-cloudflare-targets.sh

prepare_os_assets() {
  if [[ ! -d out/frontend/site ]]; then
    echo "missing out/frontend/site; run 'cd frontend && bun run build' first" >&2
    exit 1
  fi
  mkdir -p out/frontend
  rm -rf out/frontend/os
  cp -a out/frontend/site out/frontend/os
}

deploy_site() {
  bunx wrangler deploy --config wrangler.jsonc
}

deploy_os() {
  prepare_os_assets
  bunx wrangler deploy --config wrangler-os.jsonc
}

case "${TARGET}" in
  site)
    deploy_site
    ;;
  os|cloud-os)
    deploy_os
    ;;
  all)
    deploy_site
    deploy_os
    ;;
  *)
    echo "usage: $0 {site|os|cloud-os|all}" >&2
    exit 1
    ;;
esac
