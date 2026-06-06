  ;; ===================================================================
  ;; EdgeRun WASM Interpreter — ported to WAT
  ;;
  ;; Self-hosting WASM meta-circular interpreter: loads, validates, and
  ;; interprets WASM binary modules. Mirrors the x86_64 assembly runtime.
  ;;
  ;; Memory:
  ;;   0x00000 - 0x6FFFF: Interpreter state
  ;;   0x70000 - 0x7FFFF: Frame save area (64KB)
  ;;   0x80000 - 0x8FFFF: Scratch workspace
  ;;   0x90000 - 0x9FFFF: Names buffer (64KB)
  ;;   0xA0000 - 0xFFFFF: Decoded ops cache (384KB, 32768 * 32 = 1MB)
  ;;   0x100000+        : Guest WASM binary
  ;;
  ;; Imports memory from edgerun-core (shared linear memory).
  ;; Exports:
  ;;   load(wasm_ptr, wasm_len) -> error_code
  ;;   call(func_idx, args_ptr, args_len) -> error_code
  ;;   get_result_value(idx) -> i64
  ;;   get_result_count() -> i32
  ;;   dbg
  ;; ===================================================================


  ;; ── Shared constants (from edgerun-core) ─────────────────────────────

  ;; ── Memory offsets ──────────────────────────────────────────────────
  (global $OFF_ERR         i32 (i32.const 0))
  (global $OFF_SCRATCH0    i32 (i32.const 8))
  (global $OFF_SCRATCH1    i32 (i32.const 16))
  (global $OFF_SCRATCH2    i32 (i32.const 24))
  (global $OFF_SCRATCH3    i32 (i32.const 32))
  (global $FAST_STACK_LEN  (mut i32) (i32.const 0))
  (global $OFF_WASM_PTR    i32 (i32.const 64))
  (global $OFF_WASM_LEN    i32 (i32.const 72))

  ;; Type section
  (global $OFF_TYPE_COUNT i32 (i32.const 256))

  ;; Import section
  (global $OFF_IMPORT_COUNT i32 (i32.const 16648))
  (global $OFF_IMPORTS_BUF  i32 (i32.const 16656))

  ;; Function section
  (global $OFF_FUNCTION_COUNT i32 (i32.const 17680))

  ;; Code section
  (global $OFF_CODE_COUNT i32 (i32.const 21784))

  ;; Export section
  (global $OFF_EXPORT_COUNT i32 (i32.const 38176))
  (global $OFF_EXPORTS_BUF  i32 (i32.const 38184))

  ;; Global section
  (global $OFF_GLOBAL_COUNT i32 (i32.const 40232))
  (global $OFF_GLOBALS_BUF  i32 (i32.const 40240))

  ;; Table state
  (global $OFF_TABLE_HAS i32 (i32.const 42288))
  (global $OFF_TABLE_MIN i32 (i32.const 42292))
  (global $OFF_TABLE_MAX i32 (i32.const 42300))
  (global $OFF_TABLE_DATA i32 (i32.const 42308))

  ;; Memory state
  (global $OFF_MEM_MIN i32 (i32.const 43300))
  (global $OFF_MEM_MAX i32 (i32.const 43308))

  ;; Start function index
  (global $OFF_START_FUNC i32 (i32.const 43316))

  ;; Data segments
  (global $OFF_DATA_COUNT i32 (i32.const 43324))
  (global $OFF_DATA_BUF   i32 (i32.const 43332))

  ;; Element segments
  (global $OFF_ELEM_COUNT i32 (i32.const 45380))
  (global $OFF_ELEM_BUF   i32 (i32.const 45388))

  (global $OFF_DBG (export "dbg") i32 (i32.const 0x0F0000))

  ;; SIMD encoding control: 0 = old (AArch64), 1 = modern wabt
  ;; Set by the host before calling decode_opcodes for modern-encoding WASM.
  (global $SIMD_MODERN_ENCODING (export "simd_modern_encoding") (mut i32) (i32.const 0))

  ;; Execution state
  (global $OFF_EXEC_LOCAL_COUNT i32 (i32.const 80000))
  (global $OFF_EXEC_LOCALS      i32 (i32.const 80008))
  (global $OFF_EXEC_STACK_LEN   i32 (i32.const 80520))
  (global $OFF_EXEC_STACK       i32 (i32.const 80528))
  (global $OFF_EXEC_CTRL_LEN    i32 (i32.const 88720))
  (global $OFF_EXEC_CTRL        i32 (i32.const 88728))
  (global $OFF_EXEC_DEC_IDX     i32 (i32.const 89752))
  (global $OFF_EXEC_DEC_END     i32 (i32.const 89760))
  (global $OFF_EXEC_RESULT      i32 (i32.const 89768))
  (global $OFF_EXEC_RES_COUNT   i32 (i32.const 89800))
  (global $OFF_EXEC_BODY_PTR    i32 (i32.const 89808))
  (global $OFF_EXEC_BODY_LEN    i32 (i32.const 89816))
  (global $OFF_EXEC_READER_OFF  i32 (i32.const 89824))
  (global $OFF_EXEC_TYPE_IDX    i32 (i32.const 89832))
  (global $OFF_EXEC_CALL_DEPTH  i32 (i32.const 89840))
  (global $OFF_EXEC_FRAME_PTR   i32 (i32.const 89848))
  (global $OFF_EXEC_MOD_VALID   i32 (i32.const 89856))

  (global $OFF_GUEST_MEM_PAGES i32 (i32.const 89868))  ;; current guest memory pages (mutable)

  (global $OFF_FRAME_SAVE i32 (i32.const 0x70000))
  (global $FRAME_SAVE_SZ  i32 (i32.const 65536))
  (global $OFF_NAMES_BUF     i32 (i32.const 0x90000))
  (global $OFF_NAMES_PTR    i32 (i32.const 0x8FFF0))
  (global $OFF_GUEST_MEM_BASE i32 (i32.const 0x200000))
  (global $OFF_CALL_ARGS    i32 (i32.const 0x8FE00))

  ;; Label stack for computing matching block/loop/if/else/end during decode
  (global $OFF_LABEL_STACK     i32 (i32.const 0x8F000))
  (global $OFF_LABEL_STACK_PTR i32 (i32.const 0x8EFFC))

  ;; ── Resource limits ─────────────────────────────────────────────────
  (global $MAX_FUNCTIONS i32 (i32.const 256))
  (global $MAX_IMPORTS   i32 (i32.const 16))
  (global $MAX_TYPES     i32 (i32.const 64))
  (global $MAX_LOCALS    i32 (i32.const 64))
  (global $MAX_STACK     i32 (i32.const 1024))
  (global $MAX_CTRL      i32 (i32.const 64))
  (global $MAX_GLOBALS   i32 (i32.const 64))
  (global $MAX_EXPORTS   i32 (i32.const 64))
  (global $MAX_TABLE     i32 (i32.const 256))
  (global $MAX_DATAS     i32 (i32.const 64))
  (global $MAX_ELEMS     i32 (i32.const 64))
  (global $MAX_DECODED   i32 (i32.const 32768))
  (global $DECODED_SZ    i32 (i32.const 32))

  ;; ── Struct sizes ────────────────────────────────────────────────────
  (global $SZ_EXPORT i32 (i32.const 32))
  (global $SZ_GLOBAL i32 (i32.const 32))
  (global $SZ_IMPORT i32 (i32.const 64))
  (global $SZ_DATA   i32 (i32.const 32))
  (global $SZ_ELEM   i32 (i32.const 512))
  (global $SZ_CTRL   i32 (i32.const 16))

  ;; ── WASM format ─────────────────────────────────────────────────────
  (global $TI32 i32 (i32.const 0x7f))
  (global $TI64 i32 (i32.const 0x7e))
  (global $TF32 i32 (i32.const 0x7d))
  (global $TF64 i32 (i32.const 0x7c))

  (global $SEC_TYPE i32 (i32.const 1))
  (global $SEC_IMPORT i32 (i32.const 2))
  (global $SEC_FUNCTION i32 (i32.const 3))
  (global $SEC_TABLE i32 (i32.const 4))
  (global $SEC_MEMORY i32 (i32.const 5))
  (global $SEC_GLOBAL i32 (i32.const 6))
  (global $SEC_EXPORT i32 (i32.const 7))
  (global $SEC_START i32 (i32.const 8))
  (global $SEC_ELEMENT i32 (i32.const 9))
  (global $SEC_CODE i32 (i32.const 10))
  (global $SEC_DATA i32 (i32.const 11))

  (global $EXT_FUNC i32 (i32.const 0))
  (global $EXT_TABLE i32 (i32.const 1))
  (global $EXT_MEM i32 (i32.const 2))
  (global $EXT_GLOBAL i32 (i32.const 3))

  ;; ── Error codes ─────────────────────────────────────────────────────
  (global $OK           i32 (i32.const 0))
  (global $ERR_UNSUP    i32 (i32.const 1))
  (global $ERR_CORRUPT  i32 (i32.const 2))
  (global $ERR_STK_UND  i32 (i32.const 3))
  (global $ERR_STK_OV   i32 (i32.const 4))
  (global $ERR_PARSE    i32 (i32.const 7))
  (global $ERR_NOIMP    i32 (i32.const 8))
  (global $ERR_TM       i32 (i32.const 9))
  (global $ERR_UNK_OP   i32 (i32.const 10))
  (global $ERR_NO_MEM   i32 (i32.const 12))
  (global $ERR_IDX      i32 (i32.const 13))
  (global $ERR_TRAP     i32 (i32.const 16))
  (global $ERR_ARITH    i32 (i32.const 17))
  (global $ERR_MISS_IMP i32 (i32.const 18))
  (global $ERR_MISS_EXP i32 (i32.const 19))
  (global $ERR_BAD_ARGUMENT i32 (i32.const 20))
  (global $ERR_RECUR    i32 (i32.const 31))
