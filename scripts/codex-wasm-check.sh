#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CODEX="$ROOT/crates/edgerun-codex"
TARGET="${TARGET:-wasm32-unknown-unknown}"

cd "$CODEX"

PACKAGES=(
  codex-app-server-protocol
  codex-protocol
  codex-tools
  codex-shell-command
  codex-code-mode
  codex-model-provider-info
  codex-models-manager
  codex-response-debug-context
  codex-core
  codex-api
  codex-client
  codex-model-provider
)

printf '== EdgeRun Codex WASM audit ==\n'
"$ROOT/scripts/codex-wasm-audit.py" || true
printf '\n== Checking packages for %s ==\n' "$TARGET"

for pkg in "${PACKAGES[@]}"; do
  printf '\n-- %s --\n' "$pkg"
  cargo check -p "$pkg" --target "$TARGET" --no-default-features
done

printf '\n== done ==\n'
