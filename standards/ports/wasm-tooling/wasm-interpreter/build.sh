#!/usr/bin/env bash
# Build compiler/interpreter WAT modules from parts
# Usage:
#   ./build.sh <arch>       (e.g., aarch64, arm32, x86_64)
#   ./build.sh interpreter
set -euo pipefail

ARCH="${1:?Usage: $0 <arch>}"
DIR="$(cd "$(dirname "$0")" && pwd)"

case "${ARCH}" in
  interpreter)
    OUT="interpreter.wat"
    OUTDIR="${DIR}/${OUT}"
    {
      cat "${DIR}/interpreter-base.wat"
      cat "${DIR}/interpreter-decode.wat"
      cat "${DIR}/interpreter-opcodes.wat"
      cat "${DIR}/interpreter-exec.wat"
      cat "${DIR}/interpreter-wat.wat"
      echo ")"
    } > "${OUTDIR}"
    ;;

  x86-64|x86_64)
    OUT="compiler-x86_64.wat"
    OUTDIR="${DIR}/${OUT}"
    {
      cat "${DIR}/x86-64/base.wat"
      echo ""
      cat "${DIR}/x86-64/emit.wat"
      echo ""
      cat "${DIR}/x86-64/templates.wat"
      echo ""
      cat "${DIR}/templates/simd-x86-64.wat"
      echo ""
      cat "${DIR}/x86-64/dispatch.wat"
      echo ""
      echo ")"
    } > "${OUTDIR}"
    ;;

  *)
    OUT="compiler-${ARCH}.wat"
    OUTDIR="${DIR}/${OUT}"
    {
      cat "${DIR}/${ARCH}/base.wat"
      echo ""
      cat "${DIR}/${ARCH}/emit.wat"
      echo ""
      cat "${DIR}/templates/all.wat"
      echo ""
      cat "${DIR}/templates/simd-${ARCH}.wat"
      echo ""
      cat "${DIR}/dispatch.wat"
      echo ""
      echo ")"
    } > "${OUTDIR}"
    ;;
esac

echo "Assembled ${OUTDIR} ($(wc -l < "${OUTDIR}") lines)"

# Prefer native wat2wasm (wabt package) over npm shim
WAT2WASM=$(command -v /usr/bin/wat2wasm || command -v wat2wasm)
# Compile with wat2wasm
"${WAT2WASM}" "${OUTDIR}" -o "${DIR}/${OUT%.wat}.wasm" 2>&1 && \
  echo "Compiled ${OUT%.wat}.wasm successfully" || \
  echo "FAILED to compile ${OUTDIR}"
