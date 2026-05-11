#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CODEX="$ROOT/crates/edgerun-codex"
PROFILE="${PROFILE:-release}"
NO_DEFAULT="${NO_DEFAULT:-1}"

cd "$CODEX"

PACKAGES=(
  codex-app-server-protocol
  codex-protocol
  codex-tools
  codex-shell-command
  codex-core
)

FEATURE_ARGS=()
if [[ "$NO_DEFAULT" == "1" ]]; then
  FEATURE_ARGS+=(--no-default-features)
fi

printf '== EdgeRun Codex native size check ==\n'
printf 'profile=%s no_default=%s\n' "$PROFILE" "$NO_DEFAULT"

for pkg in "${PACKAGES[@]}"; do
  printf '\n-- building %s --\n' "$pkg"
  cargo build -p "$pkg" --profile "$PROFILE" "${FEATURE_ARGS[@]}"
done

printf '\n== artifact sizes ==\n'
TARGET_DIR="$CODEX/target/$PROFILE"
find "$TARGET_DIR" -maxdepth 1 -type f -perm -111 -printf '%s %p\n' 2>/dev/null \
  | sort -n \
  | awk '{ size=$1; $1=""; printf "%9.2f KB %s\n", size/1024, $0 }'

printf '\n== largest deps / target files ==\n'
find "$CODEX/target" -type f -printf '%s %p\n' 2>/dev/null \
  | sort -nr \
  | head -40 \
  | awk '{ size=$1; $1=""; printf "%9.2f MB %s\n", size/1024/1024, $0 }'
