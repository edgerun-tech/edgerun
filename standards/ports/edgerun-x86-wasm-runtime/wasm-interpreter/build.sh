#!/usr/bin/env bash
# Build a compiler-ARCH.wat from parts
# Usage: ./build.sh <arch>  (e.g., aarch64, arm32, x86-64)
set -euo pipefail

ARCH="${1:?Usage: $0 <arch>}"
DIR="$(cd "$(dirname "$0")" && pwd)"
OUT="compiler-${ARCH}.wat"
OUTDIR="${DIR}/${OUT}"

# Concatenate: base + emit + templates/* + dispatch
{
  cat "${DIR}/${ARCH}/base.wat"
  echo ""
  cat "${DIR}/${ARCH}/emit.wat"
  echo ""
  cat "${DIR}/templates/all.wat"
  echo ""
  cat "${DIR}/dispatch.wat"
  echo ""
  echo ")"
} > "${OUTDIR}"

echo "Assembled ${OUTDIR} ($(wc -l < "${OUTDIR}") lines)"

# Compile with wat2wasm
wat2wasm "${OUTDIR}" -o "${DIR}/${OUT%.wat}.wasm" 2>&1 && \
  echo "Compiled ${OUT%.wat}.wasm successfully" || \
  echo "FAILED to compile ${OUTDIR}"
