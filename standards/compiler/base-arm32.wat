  ;; ═════════════════════════════════════════════════════════════════════
  ;; EdgeRun WASM → ARM32 (ARMv7-A) JIT Compiler
  ;;
  ;; Compiles WASM decoded ops into ARM32 machine code in a memory buffer.
  ;; Follows the same architecture as the x86_64 JIT (compiler.wat).
  ;; ARM32 instructions are fixed 4 bytes, little-endian.
  ;; ═════════════════════════════════════════════════════════════════════


  ;; ── Shared constants (from edgerun-core) ─────────────────────────────

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

  ;; Peephole optimization flag
  (global $RESULT_IN_X0      (mut i32) (i32.const 0))
  (global $NEXT_OP           (mut i32) (i32.const 0))
  (global $JIT_ERROR         (mut i32) (i32.const 0))

  ;; Current decoded op pointer
  (global $CURRENT_DEC_PTR   (mut i32) (i32.const 0))

  ;; ═════════════════════════════════════════════════════════════════════
  ;; ARM32 Register constants (X-named for template compatibility)
  ;; ═════════════════════════════════════════════════════════════════════
  ;; R0 = TOS/result (like rax)
  ;; R1 = scratch (like rcx)
  ;; R2 = scratch (like rdx)
  ;; R8 = JitGlobals (like r15/x19)
  ;; R9 = locals cache (like rbx/x20)
  ;; R10 = register-allocated local 0 (like r12/x21)
  ;; R11 = register-allocated local 1 (like r13/x22)
  ;; R12 = intra-call scratch / zero reg (XZR alias)
  ;; R13 = SP, R14 = LR, R15 = PC

  (global $REG_X0  i32 (i32.const 0))
  (global $REG_X1  i32 (i32.const 1))
  (global $REG_X2  i32 (i32.const 2))
  (global $REG_X19 i32 (i32.const 8))
  (global $REG_X20 i32 (i32.const 9))
  (global $REG_X21 i32 (i32.const 10))
  (global $REG_X22 i32 (i32.const 11))
  (global $REG_XZR i32 (i32.const 12))
  (global $REG_SP  i32 (i32.const 13))

  ;; ═════════════════════════════════════════════════════════════════════
  ;; ELF output buffer (for compile_to_elf)
  ;; ═════════════════════════════════════════════════════════════════════
  (global $ELF_OUT_BUF  i32 (i32.const 0x800000))
  (global $ELF_OUT_OFF  i32 (i32.const 0x700000))  ;; ELF_OUT_BUF - JIT_CACHE

  (global $TEXT_VA      i32 (i32.const 0x400000))
  (global $BSS_VA       i32 (i32.const 0x500000))

  (global $EHDR_SIZE    i32 (i32.const 52))    ;; 32-bit ELF header
  (global $PHDR_SIZE    i32 (i32.const 32))    ;; 32-bit ELF phdr
  (global $ELF_STUB_OFF i32 (i32.const 84))    ;; after ehdr (52) + 1 phdr (32)
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
