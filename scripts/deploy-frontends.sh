#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TARGET="${1:-all}"

cd "${ROOT_DIR}"
./scripts/verify-cloudflare-targets.sh

deploy_site() {
  bunx --bun wrangler deploy --config wrangler.jsonc
}

case "${TARGET}" in
  site)
    deploy_site
    ;;
  all)
    deploy_site
    ;;
  *)
    echo "usage: $0 {site|all}" >&2
    exit 1
    ;;
esac
