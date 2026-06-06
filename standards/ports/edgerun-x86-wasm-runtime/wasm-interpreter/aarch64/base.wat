(module
  ;; ═════════════════════════════════════════════════════════════════════
  ;; EdgeRun WASM → AArch64 (ARM64) JIT Compiler
  ;;
  ;; Compiles WASM decoded ops into AArch64 machine code in a memory buffer.
  ;; Follows the same architecture as the x86_64 JIT (compiler.wat) but emits
  ;; AArch64 instructions (all 4 bytes fixed-length).
  ;; ═════════════════════════════════════════════════════════════════════

  (import "edgerun-core" "memory" (memory 1))

  ;; ── Shared constants (from edgerun-core) ─────────────────────────────
  (import "edgerun-core" "OFF_TYPES_BUF" (global $OFF_TYPES_BUF i32))
  (import "edgerun-core" "OFF_CODE_BUF" (global $OFF_CODE_BUF i32))
  (import "edgerun-core" "OFF_FUNCTIONS_BUF" (global $OFF_FUNCTIONS_BUF i32))
  (import "edgerun-core" "OFF_DECODED_OPS" (global $OFF_DECODED_OPS i32))
  (import "edgerun-core" "OFF_DECODED_COUNT" (global $OFF_DECODED_COUNT i32))
  (import "edgerun-core" "DEC_SZ" (global $DEC_SZ i32))
  (import "edgerun-core" "SZ_TYPE" (global $SZ_TYPE i32))
  (import "edgerun-core" "SZ_CODE" (global $SZ_CODE i32))
  (import "edgerun-core" "SZ_FUNC" (global $SZ_FUNC i32))

  ;; JIT code cache: 1MB starting at 0x100000
  (global $JIT_CACHE      i32 (i32.const 0x100000))
  (global $JIT_CACHE_SIZE i32 (i32.const 0x100000))
  (global $JIT_SLOT_SIZE  i32 (i32.const 0x40000))  ;; 256KB per function

  ;; JIT state at 0x300000
  (global $JIT_STATE      i32 (i32.const 0x300000))

  ;; JIT state field offsets (relative to JIT_STATE)
  (global $JS_CODE_PTR       i32 (i32.const 0))
  (global $JS_CACHE_BASE     i32 (i32.const 4))
  (global $JS_CACHE_END      i32 (i32.const 8))
  (global $JS_FUNC_IDX       i32 (i32.const 12))
  (global $JS_RESULT_COUNT   i32 (i32.const 16))
  (global $JS_STACK_DEPTH    i32 (i32.const 20))
  (global $JS_MAX_STACK      i32 (i32.const 24))
  (global $JS_LABEL_DEPTH    i32 (i32.const 28))
  (global $JS_RETURN_EMITTED i32 (i32.const 32))
  (global $JS_LABEL_OFFSETS  i32 (i32.const 64))    ;; 256*i32 = 1024 bytes
  (global $JS_LABEL_KINDS    i32 (i32.const 1088))   ;; 256*byte = 256 bytes
  (global $JS_LABEL_IF_JZ    i32 (i32.const 1344))   ;; 256*i32 = 1024 bytes
  (global $JS_FIXUP_COUNT    i32 (i32.const 2368))
  (global $JS_FIXUP_LABEL    i32 (i32.const 2372))   ;; 256*i32 = 1024 bytes
  (global $JS_FIXUP_OFFSET   i32 (i32.const 3396))   ;; 256*i32 = 1024 bytes
  (global $JS_INITIALIZED    i32 (i32.const 4420))

  ;; Label kinds
  (global $JIT_LABEL_BLOCK   i32 (i32.const 0))
  (global $JIT_LABEL_LOOP    i32 (i32.const 1))
  (global $JIT_LABEL_IF      i32 (i32.const 2))

  ;; Error codes
  (global $OK                i32 (i32.const 0))
  (global $ERR_UNSUP         i32 (i32.const -1))

  ;; ── WASM type constants ───────────────────────────────────────────────
  (global $WASM_TYPE_V128    i32 (i32.const 0x7B))
  (global $OP_PREFIX_FC      i32 (i32.const 0xFC))
  (global $OP_PREFIX_FD      i32 (i32.const 0xFD))

  ;; ── AArch64 NEON register constants ─────────────────────────────────
  ;; V0-V31 are the 128-bit SIMD/FP registers (aliased as Q0-Q31)
  (global $REG_V0  i32 (i32.const 0))

  ;; Peephole optimization flag
  (global $RESULT_IN_X0      (mut i32) (i32.const 0))
  (global $NEXT_OP           (mut i32) (i32.const 0))
  (global $JIT_ERROR         (mut i32) (i32.const 0))

  ;; ═════════════════════════════════════════════════════════════════════
  ;; ELF output buffer (for compile_to_elf)
  ;; ═════════════════════════════════════════════════════════════════════
  (global $ELF_OUT_BUF  i32 (i32.const 0x800000))
  (global $ELF_OUT_OFF  i32 (i32.const 0x700000))  ;; ELF_OUT_BUF - JIT_CACHE

  (global $TEXT_VA      i32 (i32.const 0x400000))
  (global $BSS_VA       i32 (i32.const 0x500000))

  (global $EHDR_SIZE    i32 (i32.const 64))    ;; 64-bit ELF header
  (global $PHDR_SIZE    i32 (i32.const 56))    ;; 64-bit ELF phdr
  (global $ELF_STUB_OFF i32 (i32.const 120))   ;; after ehdr (64) + 1 phdr (56)
  (global $ELF_CODE_OFF i32 (i32.const 256))   ;; aligned after stub
  (global $BSS_SIZE     i32 (i32.const 0x40000)) ;; 256KB

  ;; BSS item VAs (relative to BSS_VA)
  (global $BSS_JITGLOBALS i32 (i32.const 0x500000))
  (global $BSS_MEM       i32 (i32.const 0x500080))
  (global $BSS_LOCALS    i32 (i32.const 0x510080))
  (global $BSS_GLOBALS   i32 (i32.const 0x520080))
  (global $BSS_TABLE     i32 (i32.const 0x530080))

  ;; ═════════════════════════════════════════════════════════════════════
  ;; Flat binary output buffer (for compile_to_bin)
  ;; ═════════════════════════════════════════════════════════════════════
  (global $BIN_OUT_BUF  i32 (i32.const 0x900000))
  (global $BIN_OUT_OFF  i32 (i32.const 0x800000))  ;; BIN_OUT_BUF - JIT_CACHE
