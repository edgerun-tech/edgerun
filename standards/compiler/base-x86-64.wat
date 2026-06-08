  ;; ── x86-64 JIT backend globals ───────────────────────────────────────

  ;; JIT state at low memory (overlaps interpreter scratch space)
  (global $JS_CODE_PTR_x86_64       i32 (i32.const 0))
  (global $JS_CACHE_BASE_x86_64     i32 (i32.const 4))
  (global $JS_CACHE_END_x86_64      i32 (i32.const 8))
  (global $JS_FUNC_IDX_x86_64       i32 (i32.const 12))
  (global $JS_RESULT_COUNT_x86_64   i32 (i32.const 16))
  (global $JS_STACK_DEPTH_x86_64    i32 (i32.const 20))
  (global $JS_MAX_STACK_x86_64      i32 (i32.const 24))
  (global $JS_LABEL_DEPTH_x86_64    i32 (i32.const 28))
  (global $JS_RETURN_EMITTED_x86_64 i32 (i32.const 32))
  (global $JS_LABEL_OFFSETS_x86_64  i32 (i32.const 64))
  (global $JS_LABEL_KINDS_x86_64    i32 (i32.const 1088))
  (global $JS_LABEL_IF_JZ_x86_64    i32 (i32.const 1344))
  (global $JS_FIXUP_COUNT_x86_64    i32 (i32.const 2368))
  (global $JS_FIXUP_LABEL_x86_64    i32 (i32.const 2372))
  (global $JS_FIXUP_OFFSET_x86_64   i32 (i32.const 3396))
  (global $JS_INITIALIZED_x86_64    i32 (i32.const 4420))
  (global $JS_CALL_FIXUP_COUNT_x86_64 i32 (i32.const 4424))

  ;; Label kind constants
  (global $JIT_LABEL_BLOCK_x86_64   i32 (i32.const 0))
  (global $JIT_LABEL_LOOP_x86_64    i32 (i32.const 1))
  (global $JIT_LABEL_IF_x86_64      i32 (i32.const 2))

  ;; Mutable JIT state
  (global $CURRENT_DEC_PTR_x86_64   (mut i32) (i32.const 0))
  (global $JIT_ERROR_x86_64         (mut i32) (i32.const 0))
  (global $NEXT_OP_x86_64           (mut i32) (i32.const 0))
  (global $RESULT_IN_EAX            (mut i32) (i32.const 0))

  ;; JIT slot size (256 KB per function)
  (global $JIT_SLOT_SIZE_x86_64     i32 (i32.const 0x40000))
