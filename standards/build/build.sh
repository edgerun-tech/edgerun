#!/bin/sh
# EdgeRun Build — self-hosted (pure shell + wasm-tools, zero JS)
# Usage: build/build.sh [--no-wasm] [--out <path>]
set -e

ROOT="$(dirname "$0")/.."
MANIFEST="$ROOT/build/manifest.txt"
OUT_WAT="${OUT_WAT:-$ROOT/edgerun.wat}"
OUT_WASM="$(echo "$OUT_WAT" | sed 's/\.wat$/.wasm/')"
OUT_STRIPPED="$(echo "$OUT_WASM" | sed 's/\.wasm$/-stripped.wasm/')"
NO_WASM=0

for arg; do
  case "$arg" in
    --no-wasm) NO_WASM=1 ;;
    --out=*) OUT_WAT="${arg#--out=}"; OUT_WASM="$(echo "$OUT_WAT" | sed 's/\.wat$/.wasm/')" ;;
  esac
done

echo "EdgeRun Build — $(date -Iseconds)"
echo "Output: $OUT_WAT"
echo ""

# Backend file index range (0-indexed within MANIFEST)
# base-x86-64.wat = index 27, emit-aarch64.wat = index 35
B_START=27; B_END=36

# Backend suffixes
SUFFIXES="x86_64 x86_64 x86_64 x86_64 x86_64 arm32 arm32 aarch64 aarch64"

# Colliding names to rename
SED_RULES=""
for name in \
  '$jit_compile' '$copy_compiled_code' '$copy_code_to' '$jit_reset_state' \
  '$template_unreachable' '$template_nop' \
  '$template_block' '$template_loop' '$template_if' \
  '$template_else' '$template_end' '$template_br' '$template_br_if' \
  '$template_br_table' '$template_return' '$template_return_call' \
  '$template_drop' '$template_select' \
  '$template_local_get' '$template_local_set' '$template_local_tee' \
  '$template_global_get' '$template_global_set' \
  '$template_call' \
  '$template_memory_size' '$template_memory_grow' \
  '$template_table_get' '$template_table_set' \
  '$template_i32_load' '$template_i32_load8_s' '$template_i32_load8_u' \
  '$template_i32_load16_s' '$template_i32_load16_u' \
  '$template_i32_store' '$template_i32_store8' '$template_i32_store16' \
  '$template_i64_load' '$template_i64_store' \
  '$template_i32_eq' '$template_i32_ne' \
  '$template_i32_lt_s' '$template_i32_lt_u' \
  '$template_i32_gt_s' '$template_i32_gt_u' \
  '$template_i32_le_s' '$template_i32_le_u' \
  '$template_i32_ge_s' '$template_i32_ge_u' \
  '$template_i64_eq' '$template_i64_ne' \
  '$template_i64_lt_s' '$template_i64_lt_u' \
  '$template_i64_gt_s' '$template_i64_gt_u' \
  '$template_i64_le_s' '$template_i64_le_u' \
  '$template_i64_ge_s' '$template_i64_ge_u' \
  '$template_i32_eqz' '$template_i32_clz' '$template_i32_ctz' '$template_i32_popcnt' \
  '$template_i32_const' \
  '$template_i32_add' '$template_i32_sub' '$template_i32_mul' \
  '$template_i32_div_s' '$template_i32_div_u' \
  '$template_i32_rem_s' '$template_i32_rem_u' \
  '$template_i32_and' '$template_i32_or' '$template_i32_xor' \
  '$template_i32_shl' '$template_i32_shr_s' '$template_i32_shr_u' \
  '$template_i32_rotl' '$template_i32_rotr' \
  '$template_i64_eqz' '$template_i64_clz' '$template_i64_ctz' '$template_i64_popcnt' \
  '$template_i64_const' \
  '$template_i64_add' '$template_i64_sub' '$template_i64_mul' \
  '$template_i64_div_s' '$template_i64_div_u' \
  '$template_i64_rem_s' '$template_i64_rem_u' \
  '$template_i64_and' '$template_i64_or' '$template_i64_xor' \
  '$template_i64_shl' '$template_i64_shr_s' '$template_i64_shr_u' \
  '$template_i64_rotl' '$template_i64_rotr' \
  '$template_i32_wrap_i64' \
  '$template_i32_reinterpret_f32' '$template_f32_reinterpret_i32' \
  '$template_i64_reinterpret_f64' '$template_f64_reinterpret_i64' \
  '$template_i64_extend_i32_s' '$template_i64_extend_i32_u' \
  '$template_i32_trunc_f32_s' '$template_i32_trunc_f32_u' \
  '$template_i32_trunc_f64_s' '$template_i32_trunc_f64_u' \
  '$template_i32_trunc_sat_f32_s' '$template_i32_trunc_sat_f32_u' \
  '$template_i32_trunc_sat_f64_s' '$template_i32_trunc_sat_f64_u' \
  '$template_i64_trunc_f32_s' '$template_i64_trunc_f32_u' \
  '$template_i64_trunc_f64_s' '$template_i64_trunc_f64_u' \
  '$template_i64_trunc_sat_f32_s' \
  '$JIT_SLOT_SIZE' '$JIT_STATE' \
  '$JS_CODE_PTR' '$JS_CACHE_BASE' '$JS_CACHE_END' \
  '$JS_FUNC_IDX' '$JS_RESULT_COUNT' '$JS_STACK_DEPTH' \
  '$JS_MAX_STACK' '$JS_LABEL_DEPTH' '$JS_RETURN_EMITTED' \
  '$JS_LABEL_OFFSETS' '$JS_LABEL_KINDS' '$JS_LABEL_IF_JZ' \
  '$JS_FIXUP_COUNT' '$JS_FIXUP_LABEL' '$JS_FIXUP_OFFSET' \
  '$JS_INITIALIZED' \
  '$JIT_LABEL_BLOCK' '$JIT_LABEL_LOOP' '$JIT_LABEL_IF' \
  '$JIT_ERROR' '$CURRENT_DEC_PTR' \
  '$ELF_OUT_BUF' '$ELF_OUT_OFF' '$TEXT_VA' '$BSS_VA' \
  '$EHDR_SIZE' '$PHDR_SIZE' '$ELF_STUB_OFF' '$ELF_CODE_OFF' \
  '$BSS_SIZE' \
  '$BSS_JITGLOBALS' '$BSS_MEM' '$BSS_LOCALS' '$BSS_GLOBALS' '$BSS_TABLE' \
  '$OP_PREFIX_FC' '$OP_PREFIX_FD' '$WASM_TYPE_V128' \
  '$NEXT_OP' '$RESULT_IN_X0' \
  '$BIN_OUT_BUF' '$BIN_OUT_OFF' \
  '$REG_X0' '$REG_X1' '$REG_X2' '$REG_X19' '$REG_X20' \
  '$REG_X21' '$REG_X22' '$REG_XZR' '$REG_SP'; do

  esc_name=$(echo "$name" | sed 's/\$/\\$/g')
  # Build sed rules per-suffix later
  SED_RULES="$SED_RULES $name"
done

# Read manifest, concatenate with backend renaming
idx=0
body=""
total=0
while IFS= read -r file; do
  [ -z "$file" ] && continue
  filepath="$ROOT/$file"
  if [ ! -f "$filepath" ]; then
    echo "  ⚠  $file not found — skipping"
    idx=$((idx + 1))
    continue
  fi

  content=$(cat "$filepath")

  # Apply backend renaming for JIT backend files (indices 27-35)
  if [ "$idx" -ge "$B_START" ] && [ "$idx" -lt "$B_END" ]; then
    rel=$((idx - B_START))
    # Determine suffix
    case "$rel" in
      0|1|2|3|4) suffix="x86_64" ;;
      5|6) suffix="arm32" ;;
      7|8) suffix="aarch64" ;;
    esac

    # cleanBrokenWAT for ARM32/AArch64 (files 5-8)
    if [ "$rel" -ge 5 ]; then
      cleaned=""
      depth=0
      in_str=0
      while IFS= read -r line; do
        # Strip inline comments
        code=""
        i=0; in_str=0
        while [ $i -lt ${#line} ]; do
          ch=$(printf '%s' "$line" | cut -c$((i+1)))
          if [ "$ch" = '"' ] && [ $i -eq 0 ] || [ "$ch" = '"' ] && [ "$(printf '%s' "$line" | cut -c$i)" != '\\' ] 2>/dev/null; then
            in_str=$((1 - in_str))
          fi
          if [ "$ch" = ';' ] && [ $((i+1)) -lt ${#line} ] && [ "$(printf '%s' "$line" | cut -c$((i+2)))" = ';' ] && [ "$in_str" = 0 ]; then
            break
          fi
          code="$code$ch"
          i=$((i+1))
        done
        trimmed=$(echo "$code" | sed 's/^[[:space:]]*//;s/[[:space:]]*$//')
        opens=$(echo "$code" | tr -cd '(' | wc -c)
        closes=$(echo "$code" | tr -cd ')' | wc -c)

        if [ "$depth" -eq 0 ] && echo "$trimmed" | grep -q '^('; then
          first_form=$(echo "$trimmed" | sed 's/^(\([a-z0-9_-]*\).*/\1/')
          case "$first_form" in
            func|global|import|memory|table|data|elem|type|export|module|start)
              depth=$((depth + opens - closes))
              cleaned="$cleaned$line
"
              ;;
          esac
        elif [ "$depth" -eq 0 ] && [ "$closes" -gt "$opens" ] && echo "$trimmed" | grep -q '^\s*)'; then
          :
        else
          depth=$((depth + opens - closes))
          cleaned="$cleaned$line
"
        fi
      done <<EOF
$content
EOF
      content="$cleaned"
    fi

    # Rename colliding names
    for n in $SED_RULES; do
      content=$(printf '%s' "$content" | sed 's/\$'"$(echo "$n" | sed 's/^\$//')"'\([^a-zA-Z0-9_]\)/\$'"$(echo "$n" | sed 's/^\$//')"'_'"$suffix"'\1/g')
    done

    # Rename exports
    content=$(printf '%s' "$content" | sed 's/"jit_compile"/"jit_compile_'"$suffix"'"/g')
    content=$(printf '%s' "$content" | sed 's/"compile_to_elf"/"compile_to_elf_'"$suffix"'"/g')
    content=$(printf '%s' "$content" | sed 's/"compile_to_bin"/"compile_to_bin_'"$suffix"'"/g')
    content=$(printf '%s' "$content" | sed 's/"get_compiled_code"/"get_compiled_code_'"$suffix"'"/g')

    # Give local names to anonymous exported functions
    content=$(printf '%s' "$content" | sed 's/(func (export "compile_to_elf_'"$suffix"'"))/(func $compile_to_elf_'"$suffix"' (export "compile_to_elf_'"$suffix"'"))/g')
    content=$(printf '%s' "$content" | sed 's/(func (export "compile_to_bin_'"$suffix"'"))/(func $compile_to_bin_'"$suffix"' (export "compile_to_bin_'"$suffix"'"))/g')
  fi

  body="$body;; ── $file ──
$(printf '%s' "$content" | sed 's/[[:space:]]*$//')

"
  total=$((total + 1))
  idx=$((idx + 1))
done < "$MANIFEST"

printf '%s' "$body" > "$OUT_WAT"
echo "✓ $total fragments → $OUT_WAT ($(wc -c < "$OUT_WAT") bytes)"

[ "$NO_WASM" = 1 ] && exit 0

# Compile to WASM
echo "Compiling → $OUT_WASM..."
wasm-tools parse "$OUT_WAT" -o "$OUT_WASM"
WASM_SIZE=$(stat -c%s "$OUT_WASM" 2>/dev/null || stat -f%z "$OUT_WASM")
echo "  ✓ $OUT_WASM ($(( WASM_SIZE / 1024 )) KB)"

# Strip debug names
echo "Stripping → $OUT_STRIPPED..."
wasm-tools strip --all "$OUT_WASM" -o "$OUT_STRIPPED"
S_SIZE=$(stat -c%s "$OUT_STRIPPED" 2>/dev/null || stat -f%z "$OUT_STRIPPED")
SAVED=$(( (WASM_SIZE - S_SIZE) * 100 / WASM_SIZE ))
echo "  ✓ $OUT_STRIPPED ($(( S_SIZE / 1024 )) KB, -${SAVED}%)"
