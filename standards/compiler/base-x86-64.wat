;; ═════════════════════════════════════════════════════════════════════
  ;; EdgeRun WASM → x86_64 JIT Compiler
  ;;
  ;; Compiles WASM decoded ops into x86_64 machine code in a memory buffer.
  ;; The generated code can be exported and executed natively by a host.
  ;;
  ;; Uses the same linear memory as the interpreter (imported).
  ;; ═════════════════════════════════════════════════════════════════════


  ;; ── Shared constants (from edgerun-core) ─────────────────────────────
  ;; ── x86_64 Linux syscall numbers (from asm/unistd_64.h) ──────────────
  (global $JIT_SLOT_SIZE  i32 (i32.const 0x40000))  ;; 256KB per function

  ;; JIT state at 0x200000 (after guest binary at 0x200000, so use 0x200000+
  ;; Actually guest binary starts at 0x200000. Let's use 0x300000 for JIT state.
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
  (global $JIT_LABEL_ELSE    i32 (i32.const 3))

  ;; Error codes

  ;; Peephole optimization flag: set to 1 when result of current op
  ;; is left in eax instead of being pushed to the x86 stack.
  ;; The next op (e.g. local.set) will read from eax directly.
  (global $RESULT_IN_EAX     (mut i32) (i32.const 0))

  ;; Current decoded op pointer (set before each compile iteration dispatch)
  ;; Used by $emit_maybe_push_rax for peephole lookahead
  (global $CURRENT_DEC_PTR   (mut i32) (i32.const 0))
