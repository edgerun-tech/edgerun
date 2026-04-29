#!/usr/bin/env bash
set -euo pipefail

RUNTIME=${1:-./youki}
SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
RUNC_DIR="${SCRIPT_DIR}/src/github.com/opencontainers/runc"
PATTERN_FILE=${2:-"${SCRIPT_DIR}/runc_test_pattern"}
RUNC_REPO=${RUNC_REPO:-https://github.com/opencontainers/runc.git}
RUNC_REF=${RUNC_REF:-main}

if [[ ! -f "$RUNC_DIR/Makefile" ]]; then
  echo "runc checkout not found; cloning ${RUNC_REPO} into ${RUNC_DIR}" >&2
  mkdir -p "$(dirname "$RUNC_DIR")"
  git clone --depth 1 --branch "$RUNC_REF" "$RUNC_REPO" "$RUNC_DIR"
elif [[ -d "$RUNC_DIR/.git" ]]; then
  git -C "$RUNC_DIR" fetch --depth 1 origin "$RUNC_REF"
  git -C "$RUNC_DIR" checkout --detach FETCH_HEAD
fi

if [[ ! -x "$RUNTIME" ]]; then
  echo "$RUNTIME binary not found"
  exit 1
fi
cp "$RUNTIME" "$RUNC_DIR/runc"
chmod +x "$RUNC_DIR/runc"

cd "$RUNC_DIR"

sudo make test-binaries

readarray -t TEST_NAMES < "$PATTERN_FILE"
for name in "${TEST_NAMES[@]}"; do
  if [[ $name =~ ^\[skip\] ]]; then
    echo "skip: $name"
    continue
  fi

  # escape [](){}+?*.,'
  TEST_CASE=$(echo "$name" | sed 's/\\/\\\\/g; s/\[/\\[/g; s/\]/\\]/g; s/(/\\(/g; s/)/\\)/g; s/+/\\+/g; s/?/\\?/g; s/*/\\*/g; s/\./\\./g; s/{/\\{/g; s/}/\\}/g; s/,/\\,/g;')
  echo $TEST_CASE
  sudo -E PATH="$PATH" script -q -e -c "bats  -f \"^$TEST_CASE$\" -t tests/integration"
done
