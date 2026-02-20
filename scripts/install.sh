#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TARGET="$ROOT_DIR/edgerun"
SOURCE="$ROOT_DIR/scripts/edgerun"

if [[ ! -x "$SOURCE" ]]; then
  echo "source CLI missing or not executable: $SOURCE" >&2
  exit 1
fi

if [[ -L "$TARGET" || -f "$TARGET" ]]; then
  rm -f "$TARGET"
fi

ln -s "./scripts/edgerun" "$TARGET"
echo "installed: $TARGET -> ./scripts/edgerun"
