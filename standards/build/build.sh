#!/bin/sh
# EdgeRun Build — self-hosted (pure wasm-tools, no JS tooling)
# Usage: build/build.sh [--no-wasm]
set -e

ROOT="$(dirname "$0")/.."
MANIFEST="$ROOT/build/manifest.txt"
OUT_WAT="$ROOT/edgerun.wat"
OUT_WASM="$ROOT/edgerun.wasm"
OUT_STRIPPED="$ROOT/edgerun-stripped.wasm"
NO_WASM=0

for arg; do
  case "$arg" in
    --no-wasm) NO_WASM=1 ;;
  esac
done

echo "EdgeRun Build — $(date -Iseconds)"
echo ""

# Generate metadata
META_SCRIPT="$ROOT/tools/generate_metadata.mjs"
if command -v bun >/dev/null 2>&1; then
  bun "$META_SCRIPT" 2>/dev/null && echo "  ✓ metadata generated" || echo "  ⚠  metadata generation skipped"
elif command -v node >/dev/null 2>&1; then
  node "$META_SCRIPT" 2>/dev/null && echo "  ✓ metadata generated" || echo "  ⚠  metadata generation skipped"
else
  echo "  ⚠  metadata generation skipped (no bun/node)"
fi

# Concatenate manifest files in order.
# Strip trailing whitespace from each file.
awk -v root="$ROOT" '
  BEGIN { first = 1 }
  /^[[:space:]]*$/ { next }
  {
    file = root "/" $0
    if (first) first = 0; else print ""
    print ";; ── " $0 " ──"
    while ((rc = getline < file) > 0) {
      sub(/[[:space:]]+$/, "")
      print
    }
    close(file)
  }
' "$MANIFEST" > "$OUT_WAT"

WAT_SIZE=$(wc -c < "$OUT_WAT")
echo "✓ $OUT_WAT ($WAT_SIZE bytes)"

[ "$NO_WASM" = 1 ] && exit 0

# Compile to WASM
wasm-tools parse "$OUT_WAT" -o "$OUT_WASM"
WASM_SIZE=$(stat -c%s "$OUT_WASM" 2>/dev/null || stat -f%z "$OUT_WASM")
echo "✓ $OUT_WASM ($(( WASM_SIZE / 1024 )) KB)"

# Strip debug names (~20% saving)
wasm-tools strip --all "$OUT_WASM" -o "$OUT_STRIPPED"
STRIPPED_SIZE=$(stat -c%s "$OUT_STRIPPED" 2>/dev/null || stat -f%z "$OUT_STRIPPED")
SAVED=$(( (WASM_SIZE - STRIPPED_SIZE) * 100 / WASM_SIZE ))
echo "✓ $OUT_STRIPPED ($(( STRIPPED_SIZE / 1024 )) KB, -${SAVED}%)"