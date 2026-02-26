#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TARGET="${1:-all}"

cd "${ROOT_DIR}"
./scripts/verify-cloudflare-targets.sh

deploy_site() {
  bunx --bun wrangler deploy --config wrangler.jsonc
}

deploy_cloud_os() {
  (
    cd cloud-os
    bunx --bun wrangler deploy --config wrangler.jsonc
  )
}

case "${TARGET}" in
  site)
    deploy_site
    ;;
  cloud-os)
    deploy_cloud_os
    ;;
  all)
    deploy_site
    deploy_cloud_os
    ;;
  *)
    echo "usage: $0 {site|cloud-os|all}" >&2
    exit 1
    ;;
esac
