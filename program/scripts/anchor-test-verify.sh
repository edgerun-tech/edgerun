#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

export PATH="$HOME/.local/share/solana/install/active_release/bin:$PATH"

if ! command -v cargo-build-sbf >/dev/null 2>&1; then
  echo "error: cargo-build-sbf not found on PATH" >&2
  exit 1
fi

ensure_fresh_sbf() {
  local so_path="target/deploy/edgerun.so"
  local manifest="programs/edgerun_program/Cargo.toml"
  local source_dir="programs/edgerun_program/src"

  if [ ! -f "$so_path" ]; then
    echo "edgerun.so missing; building SBF artifact"
    cargo build-sbf --manifest-path "$manifest" --sbf-out-dir target/deploy
    return
  fi

  if [ "$manifest" -nt "$so_path" ]; then
    echo "edgerun.so stale vs Cargo.toml; rebuilding SBF artifact"
    cargo build-sbf --manifest-path "$manifest" --sbf-out-dir target/deploy
    return
  fi

  if find "$source_dir" -type f -newer "$so_path" | read -r _; then
    echo "edgerun.so stale vs program sources; rebuilding SBF artifact"
    cargo build-sbf --manifest-path "$manifest" --sbf-out-dir target/deploy
  fi
}

ensure_fresh_sbf

ensure_fresh_idl() {
  local idl_path="target/idl/edgerun_program.json"
  local manifest="programs/edgerun_program/Cargo.toml"
  local source_dir="programs/edgerun_program/src"
  local needs_refresh=0

  if [ ! -f "$idl_path" ]; then
    needs_refresh=1
  elif [ "$manifest" -nt "$idl_path" ]; then
    needs_refresh=1
  elif find "$source_dir" -type f -newer "$idl_path" | read -r _; then
    needs_refresh=1
  fi

  if [ "$needs_refresh" -eq 1 ]; then
    echo "edgerun IDL stale; rebuilding IDL"
    anchor idl build -p edgerun_program -o "$idl_path"
  fi
}

ensure_fresh_idl

mkdir -p target/idl
if [ -f target/idl/edgerun_program.json ] && [ ! -e target/idl/edgerun.json ]; then
  ln -s edgerun_program.json target/idl/edgerun.json
fi

tmp_out="$(mktemp)"
tmp_err="$(mktemp)"

set +e
anchor test --skip-build >"$tmp_out" 2>"$tmp_err"
rc=$?
set -e

cat "$tmp_out"
cat "$tmp_err" >&2

if [ $rc -eq 0 ]; then
  exit 0
fi

if grep -Eq " [0-9]+ passing " "$tmp_out" && ! grep -Eq " [0-9]+ failing" "$tmp_out"; then
  if grep -q "No such file or directory (os error 2)" "$tmp_err"; then
    echo "anchor post-test stream_logs bug detected; tests passed; treating as success" >&2
    exit 0
  fi
fi

exit $rc
