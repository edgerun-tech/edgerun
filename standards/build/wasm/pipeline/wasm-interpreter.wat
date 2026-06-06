(module
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

  (import "edgerun-core" "memory" (memory 1))

  ;; ── Shared constants (from edgerun-core) ─────────────────────────────
  (import "edgerun-core" "OFF_TYPES_BUF" (global $OFF_TYPES_BUF i32))
  (import "edgerun-core" "OFF_CODE_BUF" (global $OFF_CODE_BUF i32))
  (import "edgerun-core" "OFF_FUNCTIONS_BUF" (global $OFF_FUNCTIONS_BUF i32))
  (import "edgerun-core" "OFF_DECODED_OPS" (global $OFF_DECODED_OPS i32))
  (import "edgerun-core" "OFF_DECODED_COUNT" (global $OFF_DECODED_COUNT i32))
  (import "edgerun-core" "DEC_SZ" (global $DEC_SZ i32))
  (import "edgerun-core" "SZ_TYPE" (global $SZ_TYPE i32))
  (import "edgerun-core" "SZ_FUNC" (global $SZ_FUNC i32))
  (import "edgerun-core" "SZ_CODE" (global $SZ_CODE i32))

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

  ;; ═════════════════════════════════════════════════════════════════════
  ;; LEB128 Decoding
  ;; ═════════════════════════════════════════════════════════════════════

  (func $leb_u32 (param $offset i32) (result i32)
    (local $r i32) (local $s i32) (local $b i32) (local $c i32) (local $p i32) (local $l i32)
    (local.set $p (i32.load (global.get $OFF_WASM_PTR)))
    (local.set $l (i32.load (global.get $OFF_WASM_LEN)))
    (local.set $c (local.get $offset))
    (block $end
      (loop $lp
        (if (i32.ge_u (local.get $c) (local.get $l)) (then (return (global.get $ERR_PARSE))))
        (local.set $b (i32.load8_u (i32.add (local.get $p) (local.get $c))))
        (local.set $c (i32.add (local.get $c) (i32.const 1)))
        (local.set $r (i32.or (local.get $r) (i32.shl (i32.and (local.get $b) (i32.const 0x7f)) (local.get $s))))
        (local.set $s (i32.add (local.get $s) (i32.const 7)))
        (if (i32.gt_u (local.get $s) (i32.const 35)) (then (return (global.get $ERR_PARSE))))
        (if (i32.eqz (i32.and (local.get $b) (i32.const 0x80))) (then (br $end)))
        (br $lp)
      )
    )
    (i32.store (global.get $OFF_SCRATCH0) (local.get $r))
    (i32.store (global.get $OFF_SCRATCH1) (i32.sub (local.get $c) (local.get $offset)))
    (return (global.get $OK))
  )

  (func $leb_i32 (param $offset i32) (result i32)
    (local $r i32) (local $s i32) (local $b i32) (local $c i32) (local $p i32) (local $l i32)
    (local.set $p (i32.load (global.get $OFF_WASM_PTR)))
    (local.set $l (i32.load (global.get $OFF_WASM_LEN)))
    (local.set $c (local.get $offset))
    (block $end
      (loop $lp
        (if (i32.ge_u (local.get $c) (local.get $l)) (then (return (global.get $ERR_PARSE))))
        (local.set $b (i32.load8_u (i32.add (local.get $p) (local.get $c))))
        (local.set $c (i32.add (local.get $c) (i32.const 1)))
        (local.set $r (i32.or (local.get $r) (i32.shl (i32.and (local.get $b) (i32.const 0x7f)) (local.get $s))))
        (local.set $s (i32.add (local.get $s) (i32.const 7)))
        (if (i32.gt_u (local.get $s) (i32.const 35)) (then (return (global.get $ERR_PARSE))))
        (if (i32.eqz (i32.and (local.get $b) (i32.const 0x80))) (then (br $end)))
        (br $lp)
      )
    )
    (if (i32.and (local.get $b) (i32.const 0x40))
      (then (local.set $r (i32.or (local.get $r) (i32.shl (i32.const -1) (local.get $s)))))
    )
    (i32.store (global.get $OFF_SCRATCH0) (local.get $r))
    (i32.store (global.get $OFF_SCRATCH1) (i32.sub (local.get $c) (local.get $offset)))
    (return (global.get $OK))
  )

  (func $leb_i64 (param $offset i32) (result i32)
    (local $r_lo i32) (local $r_hi i32) (local $s i32) (local $b i32) (local $c i32) (local $p i32) (local $l i32)
    (local.set $p (i32.load (global.get $OFF_WASM_PTR)))
    (local.set $l (i32.load (global.get $OFF_WASM_LEN)))
    (local.set $c (local.get $offset))
    (block $end
      (loop $lp
        (if (i32.ge_u (local.get $c) (local.get $l)) (then (return (global.get $ERR_PARSE))))
        (local.set $b (i32.load8_u (i32.add (local.get $p) (local.get $c))))
        (local.set $c (i32.add (local.get $c) (i32.const 1)))
        (if (i32.lt_s (local.get $s) (i32.const 32))
          (then
            (local.set $r_lo (i32.or (local.get $r_lo) (i32.shl (i32.and (local.get $b) (i32.const 0x7f)) (local.get $s))))
          )
          (else
            (local.set $r_hi (i32.or (local.get $r_hi) (i32.shl (i32.and (local.get $b) (i32.const 0x7f)) (i32.sub (local.get $s) (i32.const 32)))))
          )
        )
        (local.set $s (i32.add (local.get $s) (i32.const 7)))
        (if (i32.gt_u (local.get $s) (i32.const 63)) (then (return (global.get $ERR_PARSE))))
        (if (i32.eqz (i32.and (local.get $b) (i32.const 0x80))) (then (br $end)))
        (br $lp)
      )
    )
    (if (i32.and (local.get $b) (i32.const 0x40))
      (then
        (if (i32.lt_s (local.get $s) (i32.const 32))
          (then (local.set $r_lo (i32.or (local.get $r_lo) (i32.shl (i32.const -1) (local.get $s)))))
          (else (local.set $r_hi (i32.or (local.get $r_hi) (i32.shl (i32.const -1) (i32.sub (local.get $s) (i32.const 32))))))
        )
      )
    )
    (i64.store (global.get $OFF_SCRATCH0)
      (i64.or (i64.extend_i32_u (local.get $r_lo)) (i64.shl (i64.extend_i32_u (local.get $r_hi)) (i64.const 32)))
    )
    (i32.store (global.get $OFF_SCRATCH2) (i32.sub (local.get $c) (local.get $offset)))
    (return (global.get $OK))
  )

  ;; ═════════════════════════════════════════════════════════════════════
  ;; String / name reader (length-prefixed UTF-8 byte sequence)
  ;; stores name bytes to OFFSET_NAMES_BUF, returns (name_offset, bytes_consumed)
  ;; in scratch0/scratch1
  ;; ═════════════════════════════════════════════════════════════════════
  (func $read_name (param $offset i32) (result i32)
    (local $len i32) (local $adv i32) (local $dst i32) (local $p i32) (local $i i32)
    (if (call $leb_u32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
    (local.set $len (i32.load (global.get $OFF_SCRATCH0)))
    (local.set $adv (i32.load (global.get $OFF_SCRATCH1)))
    (local.set $offset (i32.add (local.get $offset) (local.get $adv)))
    (local.set $p (i32.load (global.get $OFF_WASM_PTR)))
    (local.set $dst (i32.load (global.get $OFF_NAMES_PTR)))
    (local.set $i (i32.const 0))
    (block $copy_end
      (loop $copy_lp
        (if (i32.ge_u (local.get $i) (local.get $len)) (then (br $copy_end)))
        (i32.store8 (i32.add (local.get $dst) (local.get $i))
          (i32.load8_u (i32.add (local.get $p) (local.get $offset) (local.get $i)))
        )
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $copy_lp)
      )
    )
    (i32.store (global.get $OFF_SCRATCH0) (local.get $dst))
    (i32.store (global.get $OFF_SCRATCH1) (i32.add (local.get $adv) (local.get $len)))
    ;; Advance names buffer
    (i32.store (global.get $OFF_NAMES_PTR) (i32.add (local.get $dst) (local.get $len)))
    (return (global.get $OK))
  )

  ;; ═════════════════════════════════════════════════════════════════════
  ;; Section header reader
  ;; Output: scratch0=sec_id, scratch1=sec_len, scratch2=content_offset, scratch3=end_offset
  ;; ═════════════════════════════════════════════════════════════════════
  (func $read_section_header (param $offset i32) (result i32)
    (local $id i32) (local $p i32) (local $l i32) (local $leb_adv i32)
    (local.set $p (i32.load (global.get $OFF_WASM_PTR)))
    (local.set $l (i32.load (global.get $OFF_WASM_LEN)))
    (if (i32.ge_u (local.get $offset) (local.get $l)) (then (return (global.get $ERR_PARSE))))

    ;; Read section ID (1 byte)
    (local.set $id (i32.load8_u (i32.add (local.get $p) (local.get $offset))))
    (local.set $offset (i32.add (local.get $offset) (i32.const 1)))

    ;; Read section size (LEB128) - result in scratch0, advance in scratch1
    (if (call $leb_u32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
    (local.set $leb_adv (i32.load (global.get $OFF_SCRATCH1)))

    ;; Save section_size before overwriting scratch0
    (i32.store (global.get $OFF_SCRATCH2) (i32.load (global.get $OFF_SCRATCH0)))  ;; sec_size saved in scratch2 temporarily

    ;; Store results
    (i32.store (global.get $OFF_SCRATCH0) (local.get $id))                    ;; sec_id -> scratch0
    ;; scratch1 = sec_size (moved from scratch2 to scratch1)
    (i32.store (global.get $OFF_SCRATCH1) (i32.load (global.get $OFF_SCRATCH2)))  ;; sec_size -> scratch1
    ;; scratch2 = content_offset = offset + leb_adv
    (i32.store (global.get $OFF_SCRATCH2) (i32.add (local.get $offset) (local.get $leb_adv)))
    ;; scratch3 = end_offset = content_offset + sec_size
    (i32.store (global.get $OFF_SCRATCH3)
      (i32.add
        (i32.add (local.get $offset) (local.get $leb_adv))
        (i32.load (global.get $OFF_SCRATCH1))
      )
    )
    (return (global.get $OK))
  )

  ;; ═════════════════════════════════════════════════════════════════════
  ;; Module header: magic + version
  ;; ═════════════════════════════════════════════════════════════════════
  (func $parse_header (result i32)
    (local $p i32) (local $l i32)
    (local.set $p (i32.load (global.get $OFF_WASM_PTR)))
    (local.set $l (i32.load (global.get $OFF_WASM_LEN)))
    (if (i32.lt_u (local.get $l) (i32.const 8)) (then (return (global.get $ERR_PARSE))))
    (if (i32.ne (i32.load (local.get $p)) (i32.const 0x6d736100)) (then (return (global.get $ERR_PARSE))))
    (if (i32.ne (i32.load (i32.add (local.get $p) (i32.const 4))) (i32.const 1)) (then (return (global.get $ERR_PARSE))))
    (return (global.get $OK))
  )

  ;; ═════════════════════════════════════════════════════════════════════
  ;; Parse one type entry (functype)
  ;; Reads at offset, writes to scratch0 = new_offset after consuming type
  ;; Returns error
  ;; ═════════════════════════════════════════════════════════════════════
  (func $parse_one_type (param $offset i32) (result i32)
    (local $tc i32) (local $rc i32) (local $base i32) (local $i i32) (local $wp i32)

    (local.set $tc (i32.load (global.get $OFF_TYPE_COUNT)))
    (if (i32.ge_u (local.get $tc) (global.get $MAX_TYPES)) (then (return (global.get $ERR_PARSE))))

    ;; Type entry starts with 0x60 (functype)
    (local.set $wp (i32.load (global.get $OFF_WASM_PTR)))
    (if (i32.ne (i32.load8_u (i32.add (local.get $wp) (local.get $offset))) (i32.const 0x60))
      (then (return (global.get $ERR_PARSE)))
    )
    (local.set $offset (i32.add (local.get $offset) (i32.const 1)))

    ;; Read param count
    (if (call $leb_u32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
    (local.set $rc (i32.load (global.get $OFF_SCRATCH0)))
    (local.set $offset (i32.add (local.get $offset) (i32.load (global.get $OFF_SCRATCH1))))
    (if (i32.gt_u (local.get $rc) (i32.const 32)) (then (return (global.get $ERR_PARSE))))

    ;; Compute type entry base
    (local.set $base (i32.add (global.get $OFF_TYPES_BUF) (i32.mul (local.get $tc) (global.get $SZ_TYPE))))

    ;; Write param types (up to 32 entries, each 1 byte)
    (local.set $i (i32.const 0))
    (block $pl
      (loop $pl_lp
        (if (i32.ge_u (local.get $i) (local.get $rc)) (then (br $pl)))
        (local.set $wp (i32.load (global.get $OFF_WASM_PTR)))
        (i32.store8 (i32.add (local.get $base) (local.get $i))
          (i32.load8_u (i32.add (local.get $wp) (local.get $offset)))
        )
        (local.set $offset (i32.add (local.get $offset) (i32.const 1)))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $pl_lp)
      )
    )

    ;; Write param_count (16-bit at offset 128 within type entry)
    (i32.store16 (i32.add (local.get $base) (i32.const 128)) (local.get $rc))

    ;; Read result count
    (if (call $leb_u32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
    (local.set $rc (i32.load (global.get $OFF_SCRATCH0)))
    (local.set $offset (i32.add (local.get $offset) (i32.load (global.get $OFF_SCRATCH1))))
    (if (i32.gt_u (local.get $rc) (i32.const 4)) (then (return (global.get $ERR_PARSE))))

    ;; Write result types (start at offset 132 = 128 + 4)
    (local.set $i (i32.const 0))
    (block $rl
      (loop $rl_lp
        (if (i32.ge_u (local.get $i) (local.get $rc)) (then (br $rl)))
        (local.set $wp (i32.load (global.get $OFF_WASM_PTR)))
        (i32.store8 (i32.add (local.get $base) (i32.const 132) (local.get $i))
          (i32.load8_u (i32.add (local.get $wp) (local.get $offset)))
        )
        (local.set $offset (i32.add (local.get $offset) (i32.const 1)))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $rl_lp)
      )
    )

    ;; Write result_count (16-bit at offset 132 + 4 = 136)
    (i32.store16 (i32.add (local.get $base) (i32.const 136)) (local.get $rc))

    ;; Increment type count
    (i32.store (global.get $OFF_TYPE_COUNT) (i32.add (i32.load (global.get $OFF_TYPE_COUNT)) (i32.const 1)))

    ;; Return new offset
    (i32.store (global.get $OFF_SCRATCH0) (local.get $offset))
    (return (global.get $OK))
  )

  ;; ═════════════════════════════════════════════════════════════════════
  ;; Parse type section
  ;; ═════════════════════════════════════════════════════════════════════
  (func $parse_type_section (param $offset i32) (param $size i32) (result i32)
    (local $count i32) (local $err i32) (local $end i32)

    (local.set $end (i32.add (local.get $offset) (local.get $size)))

    ;; Read type count
    (if (call $leb_u32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
    (local.set $count (i32.load (global.get $OFF_SCRATCH0)))
    (local.set $offset (i32.add (local.get $offset) (i32.load (global.get $OFF_SCRATCH1))))

    (block $lp
      (loop $continue
        (if (i32.eqz (local.get $count)) (then (br $lp)))
        (local.set $err (call $parse_one_type (local.get $offset)))
        (if (local.get $err) (then (return (local.get $err))))
        (local.set $offset (i32.load (global.get $OFF_SCRATCH0)))
        (local.set $count (i32.sub (local.get $count) (i32.const 1)))
        (br $continue)
      )
    )

    (return (global.get $OK))
  )

  ;; ═════════════════════════════════════════════════════════════════════
  ;; Parse limits: reads either "min" or "min, max" pair
  ;; Input: offset, returns: scratch0=min, scratch1=max(-1=unset), scratch2=advance
  ;; ═════════════════════════════════════════════════════════════════════
  (func $parse_limits (param $offset i32) (result i32)
    (local $flags i32) (local $min i32) (local $adv i32) (local $p i32)
    (local.set $p (i32.load (global.get $OFF_WASM_PTR)))

    ;; Read flags byte: 0 = min only, 1 = min + max
    (local.set $flags (i32.load8_u (i32.add (local.get $p) (local.get $offset))))
    (local.set $offset (i32.add (local.get $offset) (i32.const 1)))

    ;; Read min
    (if (call $leb_u32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
    (local.set $min (i32.load (global.get $OFF_SCRATCH0)))
    (local.set $adv (i32.load (global.get $OFF_SCRATCH1)))
    (local.set $offset (i32.add (local.get $offset) (local.get $adv)))
    (i32.store (global.get $OFF_SCRATCH0) (local.get $min))

    (if (local.get $flags)
      (then
        ;; Read max
        (if (call $leb_u32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
        (i32.store (global.get $OFF_SCRATCH1) (i32.load (global.get $OFF_SCRATCH0)))
        (local.set $offset (i32.add (local.get $offset) (i32.load (global.get $OFF_SCRATCH1))))
      )
      (else
        ;; max = -1 (unlimited)
        (i32.store (global.get $OFF_SCRATCH1) (i32.const -1))
      )
    )

    (return (global.get $OK))
  )

  ;; ═════════════════════════════════════════════════════════════════════
  ;; Parse function section (list of type indices)
  ;; ═════════════════════════════════════════════════════════════════════
  (func $parse_function_section (param $offset i32) (param $size i32) (result i32)
    (local $count i32) (local $i i32) (local $fc i32)

    (if (call $leb_u32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
    (local.set $count (i32.load (global.get $OFF_SCRATCH0)))
    (local.set $offset (i32.add (local.get $offset) (i32.load (global.get $OFF_SCRATCH1))))
    (if (i32.gt_u (local.get $count) (global.get $MAX_FUNCTIONS)) (then (return (global.get $ERR_PARSE))))

    (local.set $i (i32.const 0))
    (local.set $fc (i32.load (global.get $OFF_FUNCTION_COUNT)))
    (block $lp
      (loop $cont
        (if (i32.ge_u (local.get $i) (local.get $count)) (then (br $lp)))
        ;; Read type index
        (if (call $leb_u32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
        ;; Store to functions_buf[fc + i] = type_index (first 8 bytes of 16-byte entry)
        (i32.store
          (i32.add (global.get $OFF_FUNCTIONS_BUF) (i32.mul (i32.add (local.get $fc) (local.get $i)) (global.get $SZ_FUNC)))
          (i32.load (global.get $OFF_SCRATCH0))
        )
        (local.set $offset (i32.add (local.get $offset) (i32.load (global.get $OFF_SCRATCH1))))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $cont)
      )
    )
    (i32.store (global.get $OFF_FUNCTION_COUNT) (i32.add (local.get $fc) (local.get $count)))
    (return (global.get $OK))
  )

  ;; ═════════════════════════════════════════════════════════════════════
  ;; Parse import section
  ;; Each import: module_name(string) + field_name(string) + kind(1 byte) + kind_data
  ;; For function: kind_data = type_index(LEB128)
  ;; For table: kind_data = elem_type(1) + limits
  ;; For memory: kind_data = limits
  ;; For global: kind_data = val_type(1) + mutability(1)
  ;; ═════════════════════════════════════════════════════════════════════
  (func $parse_import_section (param $offset i32) (param $size i32) (result i32)
    (local $count i32) (local $i i32)
    (local $kind i32) (local $p i32) (local $end i32) (local $flags i32)

    (if (call $leb_u32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
    (local.set $count (i32.load (global.get $OFF_SCRATCH0)))
    (local.set $offset (i32.add (local.get $offset) (i32.load (global.get $OFF_SCRATCH1))))
    (if (i32.gt_u (local.get $count) (global.get $MAX_IMPORTS)) (then (return (global.get $ERR_PARSE))))

    (local.set $end (i32.add (local.get $offset) (local.get $size)))
    (local.set $i (i32.const 0))
    (block $lp
      (loop $cont
        (if (i32.ge_u (local.get $i) (local.get $count)) (then (br $lp)))
        (if (i32.ge_u (local.get $offset) (local.get $end)) (then (return (global.get $ERR_PARSE))))

        ;; Skip module name
        (if (call $read_name (local.get $offset)) (then (return (global.get $ERR_PARSE))))
        (local.set $offset (i32.add (local.get $offset) (i32.load (global.get $OFF_SCRATCH1))))

        ;; Skip field name
        (if (call $read_name (local.get $offset)) (then (return (global.get $ERR_PARSE))))
        (local.set $offset (i32.add (local.get $offset) (i32.load (global.get $OFF_SCRATCH1))))

        ;; Read import kind
        (local.set $p (i32.load (global.get $OFF_WASM_PTR)))
        (local.set $kind (i32.load8_u (i32.add (local.get $p) (local.get $offset))))
        (local.set $offset (i32.add (local.get $offset) (i32.const 1)))

        (if (i32.eq (local.get $kind) (global.get $EXT_FUNC))
          (then
            ;; Function import: skip type index
            (if (call $leb_u32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
            (local.set $offset (i32.add (local.get $offset) (i32.load (global.get $OFF_SCRATCH1))))
          )
        )
        (if (i32.eq (local.get $kind) (global.get $EXT_TABLE))
          (then
            ;; Table import: elem_type (1 byte) + limits
            (local.set $offset (i32.add (local.get $offset) (i32.const 1)))
            ;; flags (1 byte)
            (local.set $p (i32.load (global.get $OFF_WASM_PTR)))
            (local.set $flags (i32.load8_u (i32.add (local.get $p) (local.get $offset))))
            (local.set $offset (i32.add (local.get $offset) (i32.const 1)))
            ;; min
            (if (call $leb_u32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
            (local.set $offset (i32.add (local.get $offset) (i32.load (global.get $OFF_SCRATCH1))))
            ;; optional max
            (if (local.get $flags)
              (then
                (if (call $leb_u32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
                (local.set $offset (i32.add (local.get $offset) (i32.load (global.get $OFF_SCRATCH1))))
              )
            )
          )
        )
        (if (i32.eq (local.get $kind) (global.get $EXT_MEM))
          (then
            ;; Memory import: limits
            (local.set $p (i32.load (global.get $OFF_WASM_PTR)))
            (local.set $flags (i32.load8_u (i32.add (local.get $p) (local.get $offset))))
            (local.set $offset (i32.add (local.get $offset) (i32.const 1)))
            (if (call $leb_u32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
            (local.set $offset (i32.add (local.get $offset) (i32.load (global.get $OFF_SCRATCH1))))
            (if (local.get $flags)
              (then
                (if (call $leb_u32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
                (local.set $offset (i32.add (local.get $offset) (i32.load (global.get $OFF_SCRATCH1))))
              )
            )
          )
        )
        (if (i32.eq (local.get $kind) (global.get $EXT_GLOBAL))
          (then
            ;; Global import: val_type (1 byte) + mutability (1 byte)
            (local.set $offset (i32.add (local.get $offset) (i32.const 2)))
          )
        )

        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $cont)
      )
    )
    ;; Store import count
    (i32.store (global.get $OFF_IMPORT_COUNT) (local.get $count))
    (return (global.get $OK))
  )

  ;; ═════════════════════════════════════════════════════════════════════
  ;; Parse global section
  ;; Each global: val_type(1) + mutability(1) + init_expr(opcodes ending with end)
  ;; ═════════════════════════════════════════════════════════════════════
  (func $parse_global_section (param $offset i32) (param $size i32) (result i32)
    (local $count i32) (local $i i32) (local $gc i32) (local $end i32)
    (local $op i32) (local $p i32)

    (if (call $leb_u32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
    (local.set $count (i32.load (global.get $OFF_SCRATCH0)))
    (local.set $offset (i32.add (local.get $offset) (i32.load (global.get $OFF_SCRATCH1))))
    (if (i32.gt_u (local.get $count) (global.get $MAX_GLOBALS)) (then (return (global.get $ERR_PARSE))))

    (local.set $end (i32.add (local.get $offset) (local.get $size)))
    (local.set $gc (i32.load (global.get $OFF_GLOBAL_COUNT)))
    (local.set $i (i32.const 0))
    (block $lp
      (loop $cont
        (if (i32.ge_u (local.get $i) (local.get $count)) (then (br $lp)))
        (if (i32.ge_u (local.get $offset) (local.get $end)) (then (return (global.get $ERR_PARSE))))

        ;; Skip val_type (1 byte) + mutability (1 byte)
        (local.set $offset (i32.add (local.get $offset) (i32.const 2)))

        ;; Parse init expression: read opcodes until end (0x0B)
        (local.set $p (i32.load (global.get $OFF_WASM_PTR)))
        (local.set $op (i32.load8_u (i32.add (local.get $p) (local.get $offset))))
        (if (i32.eq (local.get $op) (i32.const 0x41))  ;; i32.const
          (then
            (local.set $offset (i32.add (local.get $offset) (i32.const 1)))
            (if (call $leb_i32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
            (i32.store
              (i32.add (global.get $OFF_GLOBALS_BUF) (i32.shl (i32.add (local.get $gc) (local.get $i)) (i32.const 2)))
              (i32.load (global.get $OFF_SCRATCH0))
            )
            (local.set $offset (i32.add (local.get $offset) (i32.load (global.get $OFF_SCRATCH1))))
          )
          (else
            ;; Unsupported init expr: skip to end (0x0B) silently, value stays 0
            (block $skip_expr
              (loop $skip_cont
                (local.set $p (i32.load (global.get $OFF_WASM_PTR)))
                (local.set $op (i32.load8_u (i32.add (local.get $p) (local.get $offset))))
                (local.set $offset (i32.add (local.get $offset) (i32.const 1)))
                (if (i32.eq (local.get $op) (i32.const 0x0B)) (then (br $skip_expr)))
                (br $skip_cont)
              )
            )
            ;; Default to 0
            (i32.store
              (i32.add (global.get $OFF_GLOBALS_BUF) (i32.shl (i32.add (local.get $gc) (local.get $i)) (i32.const 2)))
              (i32.const 0)
            )
          )
        )

        ;; Skip end
        (if (i32.ne (local.get $op) (i32.const 0x0B))
          (then
            ;; Skip remaining opcodes until end (0x0B)
            (block $skip_end
              (loop $skip_end_cont
                (local.set $p (i32.load (global.get $OFF_WASM_PTR)))
                (local.set $op (i32.load8_u (i32.add (local.get $p) (local.get $offset))))
                (local.set $offset (i32.add (local.get $offset) (i32.const 1)))
                (if (i32.eq (local.get $op) (i32.const 0x0B)) (then (br $skip_end)))
                (br $skip_end_cont)
              )
            )
          )
        )

        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $cont)
      )
    )
    (i32.store (global.get $OFF_GLOBAL_COUNT) (i32.add (local.get $gc) (local.get $count)))
    (return (global.get $OK))
  )

  ;; ═════════════════════════════════════════════════════════════════════
  ;; Parse memory section (memory type + limits)
  ;; ═════════════════════════════════════════════════════════════════════
  (func $parse_memory_section (param $offset i32) (param $size i32) (result i32)
    (local $count i32)

    (if (call $leb_u32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
    (local.set $count (i32.load (global.get $OFF_SCRATCH0)))
    (local.set $offset (i32.add (local.get $offset) (i32.load (global.get $OFF_SCRATCH1))))
    (if (i32.ne (local.get $count) (i32.const 1)) (then (return (global.get $ERR_UNSUP))))

    ;; Parse limits
    (if (call $parse_limits (local.get $offset)) (then (return (global.get $ERR_PARSE))))
    (i32.store (global.get $OFF_MEM_MIN) (i32.load (global.get $OFF_SCRATCH0)))
    (i32.store (global.get $OFF_MEM_MAX) (i32.load (global.get $OFF_SCRATCH1)))
    (i32.store (global.get $OFF_GUEST_MEM_PAGES) (i32.load (global.get $OFF_SCRATCH0)))
    (return (global.get $OK))
  )

  ;; ═════════════════════════════════════════════════════════════════════
  ;; Parse table section (elem_type + limits)
  ;; MVP: at most 1 table, elem_type must be 0x70 (funcref)
  ;; ═════════════════════════════════════════════════════════════════════
  (func $parse_table_section (param $offset i32) (param $size i32) (result i32)
    (local $count i32) (local $elem_type i32) (local $flags i32)
    (local $min i32) (local $max i32) (local $p i32) (local $i i32)

    (i32.store (global.get $OFF_DBG) (i32.const 50))  ;; dbg: entered parse_table_section

    (if (call $leb_u32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
    (i32.store (global.get $OFF_DBG) (i32.const 51))  ;; dbg: after leb count
    (local.set $count (i32.load (global.get $OFF_SCRATCH0)))
    (local.set $offset (i32.add (local.get $offset) (i32.load (global.get $OFF_SCRATCH1))))
    (if (i32.gt_u (local.get $count) (i32.const 1)) (then (return (global.get $ERR_UNSUP))))
    (if (i32.eqz (local.get $count)) (then (return (global.get $OK))))

    (i32.store (global.get $OFF_DBG) (i32.const 52))  ;; dbg: count is 1, reading elem_type

    ;; elem_type (1 byte) must be 0x70 (funcref)
    (local.set $p (i32.load (global.get $OFF_WASM_PTR)))
    (local.set $elem_type (i32.load8_u (i32.add (local.get $p) (local.get $offset))))
    (local.set $offset (i32.add (local.get $offset) (i32.const 1)))
    (if (i32.ne (local.get $elem_type) (i32.const 0x70)) (then (return (global.get $ERR_UNSUP))))

    (i32.store (global.get $OFF_DBG) (i32.const 53))  ;; dbg: elem_type is 0x70

    ;; Parse limits: flags (1 byte) + min (LEB128) + [max (LEB128)]
    (local.set $p (i32.load (global.get $OFF_WASM_PTR)))
    (local.set $flags (i32.load8_u (i32.add (local.get $p) (local.get $offset))))
    (local.set $offset (i32.add (local.get $offset) (i32.const 1)))
    (if (call $leb_u32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
    (local.set $min (i32.load (global.get $OFF_SCRATCH0)))
    (local.set $offset (i32.add (local.get $offset) (i32.load (global.get $OFF_SCRATCH1))))
    (local.set $max (i32.const 0))
    (if (i32.and (local.get $flags) (i32.const 1))
      (then
        (if (call $leb_u32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
        (local.set $max (i32.load (global.get $OFF_SCRATCH0)))
        (local.set $offset (i32.add (local.get $offset) (i32.load (global.get $OFF_SCRATCH1))))
      )
    )
    (i32.store (global.get $OFF_DBG) (i32.const 54))  ;; dbg: limits read, checking max
    (if (i32.gt_u (local.get $min) (global.get $MAX_TABLE)) (then (return (global.get $ERR_PARSE))))
    (if (i32.and (local.get $flags) (i32.const 1))
      (then (if (i32.gt_u (local.get $max) (global.get $MAX_TABLE)) (then (return (global.get $ERR_PARSE)))))
    )

    ;; Store table info
    (i32.store8 (global.get $OFF_TABLE_HAS) (i32.const 1))
    (i32.store (global.get $OFF_TABLE_MIN) (local.get $min))
    (i32.store (global.get $OFF_TABLE_MAX) (local.get $max))

    ;; Initialize all table entries to -1 (null funcref)
    (local.set $i (i32.const 0))
    (block $init_lp
      (loop $init_cont
        (if (i32.ge_u (local.get $i) (local.get $min)) (then (br $init_lp)))
        (i32.store
          (i32.add (global.get $OFF_TABLE_DATA) (i32.shl (local.get $i) (i32.const 2)))
          (i32.const -1)
        )
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $init_cont)
      )
    )

    (return (global.get $OK))
  )

  ;; ═════════════════════════════════════════════════════════════════════
  ;; Parse export section (list of exports)
  ;; Each export: name(string) + kind(1 byte) + index(LEB128)
  ;; ═════════════════════════════════════════════════════════════════════
  (func $parse_export_section (param $offset i32) (param $size i32) (result i32)
    (local $count i32) (local $i i32) (local $ec i32) (local $base i32)
    (local $name_off i32) (local $name_len i32) (local $kind i32) (local $idx i32) (local $p i32)

    (if (call $leb_u32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
    (local.set $count (i32.load (global.get $OFF_SCRATCH0)))
    (local.set $offset (i32.add (local.get $offset) (i32.load (global.get $OFF_SCRATCH1))))
    (if (i32.gt_u (local.get $count) (global.get $MAX_EXPORTS)) (then (return (global.get $ERR_PARSE))))

    (local.set $ec (i32.load (global.get $OFF_EXPORT_COUNT)))
    (local.set $i (i32.const 0))
    (block $lp
      (loop $cont
        (if (i32.ge_u (local.get $i) (local.get $count)) (then (br $lp)))

        (local.set $base (i32.add (global.get $OFF_EXPORTS_BUF) (i32.mul (i32.add (local.get $ec) (local.get $i)) (global.get $SZ_EXPORT))))

        ;; Read name string
        (if (call $read_name (local.get $offset)) (then (return (global.get $ERR_PARSE))))
        (local.set $name_off (i32.load (global.get $OFF_SCRATCH0)))
        (local.set $name_len (i32.load (global.get $OFF_SCRATCH1)))
        (local.set $offset (i32.add (local.get $offset) (local.get $name_len)))

        ;; Read kind (1 byte)
        (local.set $p (i32.load (global.get $OFF_WASM_PTR)))
        (local.set $kind (i32.load8_u (i32.add (local.get $p) (local.get $offset))))
        (local.set $offset (i32.add (local.get $offset) (i32.const 1)))

        ;; Read index (LEB128)
        (if (call $leb_u32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
        (local.set $idx (i32.load (global.get $OFF_SCRATCH0)))
        (local.set $offset (i32.add (local.get $offset) (i32.load (global.get $OFF_SCRATCH1))))

        ;; Store to export entry
        (i32.store (local.get $base) (local.get $name_off))           ;; name_ptr
        (i32.store (i32.add (local.get $base) (i32.const 8)) (local.get $name_len))  ;; name_len
        (i32.store8 (i32.add (local.get $base) (i32.const 16)) (local.get $kind))    ;; kind
        (i32.store (i32.add (local.get $base) (i32.const 24)) (local.get $idx))      ;; index

        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $cont)
      )
    )
    (i32.store (global.get $OFF_EXPORT_COUNT) (i32.add (local.get $ec) (local.get $count)))
    (return (global.get $OK))
  )

  ;; ═════════════════════════════════════════════════════════════════════
  ;; Parse start section (single function index)
  ;; ═════════════════════════════════════════════════════════════════════
  (func $parse_start_section (param $offset i32) (param $size i32) (result i32)
    (if (call $leb_u32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
    (i32.store (global.get $OFF_START_FUNC) (i32.load (global.get $OFF_SCRATCH0)))
    (return (global.get $OK))
  )

  ;; ═════════════════════════════════════════════════════════════════════
  ;; Parse code section (function bodies)
  ;; Each body: size(LEB128) + local_count(LEB128) + (locals: count + type)* + bytecode
  ;; We store: body_offset, body_len, local_count
  ;; ═════════════════════════════════════════════════════════════════════
  (func $parse_code_section (param $offset i32) (param $size i32) (result i32)
    (local $count i32) (local $i i32) (local $cc i32) (local $base i32)
    (local $body_size i32) (local $body_end i32)
    (local $local_count i32) (local $lc i32)
    (local $num_locals i32) (local $type i32) (local $j i32)
    (local $p i32)

    (i32.store (global.get $OFF_DBG) (i32.const 90))  ;; dbg: entered parse_code_section
    (local.set $cc (i32.load (global.get $OFF_CODE_COUNT)))

    (if (call $leb_u32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
    (local.set $count (i32.load (global.get $OFF_SCRATCH0)))
    (local.set $offset (i32.add (local.get $offset) (i32.load (global.get $OFF_SCRATCH1))))
    (if (i32.gt_u (local.get $count) (global.get $MAX_FUNCTIONS)) (then (return (global.get $ERR_PARSE))))

    (local.set $i (i32.const 0))
    (block $lp
      (loop $cont
        (if (i32.ge_u (local.get $i) (local.get $count)) (then (br $lp)))

        ;; Read body size
        (if (call $leb_u32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
        (local.set $body_size (i32.load (global.get $OFF_SCRATCH0)))
        (local.set $offset (i32.add (local.get $offset) (i32.load (global.get $OFF_SCRATCH1))))

        ;; body_end = offset + body_size (offset is after the size LEB, body_size is the content size)
        (local.set $body_end (i32.add (local.get $offset) (local.get $body_size)))

        ;; Store body_offset (relative to wasm_base), body_len
        (local.set $base (i32.add (global.get $OFF_CODE_BUF) (i32.mul (i32.add (local.get $cc) (local.get $i)) (global.get $SZ_CODE))))
        (i32.store (i32.add (local.get $base) (i32.const 0)) (local.get $offset))     ;; body_offset
        (i32.store (i32.add (local.get $base) (i32.const 8)) (local.get $body_size))  ;; body_len

        ;; Read local count
        (if (call $leb_u32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
        (local.set $local_count (i32.load (global.get $OFF_SCRATCH0)))
        (local.set $offset (i32.add (local.get $offset) (i32.load (global.get $OFF_SCRATCH1))))

        ;; Parse local declarations
        (local.set $lc (i32.const 0))
        (local.set $j (i32.const 0))
        (block $llp
          (loop $llcont
            (if (i32.ge_u (local.get $j) (local.get $local_count)) (then (br $llp)))
            (if (call $leb_u32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
            (local.set $num_locals (i32.load (global.get $OFF_SCRATCH0)))
            (local.set $offset (i32.add (local.get $offset) (i32.load (global.get $OFF_SCRATCH1))))
            (local.set $p (i32.load (global.get $OFF_WASM_PTR)))
            (local.set $type (i32.load8_u (i32.add (local.get $p) (local.get $offset))))
            (local.set $offset (i32.add (local.get $offset) (i32.const 1)))
            (local.set $lc (i32.add (local.get $lc) (local.get $num_locals)))
            (local.set $j (i32.add (local.get $j) (i32.const 1)))
            (br $llcont)
          )
        )
        (i32.store (i32.add (local.get $base) (i32.const 16)) (local.get $lc))  ;; local_count

        ;; Decode opcodes
        (i32.store (global.get $OFF_DBG) (i32.const 80))  ;; dbg: before decode
        (if (call $decode_opcodes (local.get $offset) (i32.sub (local.get $body_end) (local.get $offset)))
          (then (i32.store (global.get $OFF_DBG) (i32.const 81)) (return (global.get $ERR_PARSE)))  ;; dbg: decode fail
        )
        (i32.store (global.get $OFF_DBG) (i32.const 82))  ;; dbg: after decode
        ;; Compute end targets for structured control flow
        (if (call $compute_end_targets
              (i32.load (global.get $OFF_SCRATCH0))
              (i32.load (global.get $OFF_SCRATCH1))
            )
          (then (i32.store (global.get $OFF_DBG) (i32.const 83)) (return (global.get $ERR_PARSE)))  ;; dbg: compute fail
        )
        (i32.store (global.get $OFF_DBG) (i32.const 84))  ;; dbg: after compute
        ;; Store decoded_start (scratch0) and decoded_count (scratch1) in code entry
        (i32.store (i32.add (local.get $base) (i32.const 24)) (i32.load (global.get $OFF_SCRATCH0)))  ;; decoded_start
        (i32.store (i32.add (local.get $base) (i32.const 32)) (i32.load (global.get $OFF_SCRATCH1)))  ;; decoded_count

        ;; Skip to body_end
        (local.set $offset (local.get $body_end))

        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $cont)
      )
    )
    (i32.store (global.get $OFF_CODE_COUNT) (i32.add (local.get $cc) (local.get $count)))
    (return (global.get $OK))
  )

  ;; ═════════════════════════════════════════════════════════════════════
  ;; Parse data section (list of data segments)
  ;; ═════════════════════════════════════════════════════════════════════
  (func $parse_data_section (param $offset i32) (param $size i32) (result i32)
    (local $count i32) (local $i i32) (local $dc i32) (local $base i32)
    (local $mode i32) (local $seg_size i32) (local $p i32) (local $j i32)
    (local $dest_addr i32) (local $wasm_base i32)

    (if (call $leb_u32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
    (local.set $count (i32.load (global.get $OFF_SCRATCH0)))
    (local.set $offset (i32.add (local.get $offset) (i32.load (global.get $OFF_SCRATCH1))))
    (if (i32.gt_u (local.get $count) (global.get $MAX_DATAS)) (then (return (global.get $ERR_PARSE))))

    (local.set $dc (i32.load (global.get $OFF_DATA_COUNT)))
    (local.set $wasm_base (i32.load (global.get $OFF_WASM_PTR)))
    (local.set $i (i32.const 0))
    (block $lp
      (loop $cont
        (if (i32.ge_u (local.get $i) (local.get $count)) (then (br $lp)))
        (local.set $base (i32.add (global.get $OFF_DATA_BUF) (i32.mul (i32.add (local.get $dc) (local.get $i)) (global.get $SZ_DATA))))

        ;; Read mode (0=active, 1=passive, 2=active with memory index)
        (local.set $mode (i32.load8_u (i32.add (local.get $wasm_base) (local.get $offset))))
        (local.set $offset (i32.add (local.get $offset) (i32.const 1)))

        (local.set $dest_addr (i32.const 0))
        (if (i32.or (i32.eq (local.get $mode) (i32.const 2)) (i32.eq (local.get $mode) (i32.const 0)))
          (then
            ;; Active: parse i32.const <value> 0x0B
            (if (i32.eq (i32.load8_u (i32.add (local.get $wasm_base) (local.get $offset))) (i32.const 0x41))
              (then
                (local.set $offset (i32.add (local.get $offset) (i32.const 1)))
                (if (call $leb_i32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
                (local.set $dest_addr (i32.load (global.get $OFF_SCRATCH0)))
                (local.set $offset (i32.add (local.get $offset) (i32.load (global.get $OFF_SCRATCH1))))
              )
            )
            ;; Skip remaining init expression until 0x0B
            (block $ie
              (loop $iel
                (if (i32.eq (i32.load8_u (i32.add (local.get $wasm_base) (local.get $offset))) (i32.const 0x0B))
                  (then
                    (local.set $offset (i32.add (local.get $offset) (i32.const 1)))
                    (br $ie)
                  )
                )
                (local.set $offset (i32.add (local.get $offset) (i32.const 1)))
                (br $iel)
              )
            )
          )
        )

        ;; Read segment data size
        (if (call $leb_u32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
        (local.set $seg_size (i32.load (global.get $OFF_SCRATCH0)))
        (local.set $offset (i32.add (local.get $offset) (i32.load (global.get $OFF_SCRATCH1))))

        ;; Store data segment info
        (i32.store (local.get $base) (local.get $offset))      ;; data_offset (in guest wasm)
        (i32.store (i32.add (local.get $base) (i32.const 4)) (local.get $dest_addr))  ;; dest_addr (in guest memory)
        (i32.store (i32.add (local.get $base) (i32.const 8)) (local.get $seg_size))  ;; data_len

        ;; Copy data bytes to guest memory (only for active segments)
        (if (i32.eq (local.get $mode) (i32.const 1))
          (then)  ;; passive segment: skip copy to memory
          (else
            (if (i32.and (i32.eqz (local.get $dest_addr)) (i32.eqz (local.get $seg_size)))
              (then)
              (else
            ;; Copy seg_size bytes from wasm_base+data_offset to guest_mem_base+dest_addr
            (local.set $p (i32.const 0))
            (block $copy_lp
              (loop $copy_cont
                (if (i32.ge_u (local.get $p) (local.get $seg_size)) (then (br $copy_lp)))
                (i32.store8
                  (i32.add (global.get $OFF_GUEST_MEM_BASE) (i32.add (local.get $dest_addr) (local.get $p)))
                  (i32.load8_u (i32.add (local.get $wasm_base) (i32.add (local.get $offset) (local.get $p))))
                )
                (local.set $p (i32.add (local.get $p) (i32.const 1)))
                (br $copy_cont)
              )
            )
            )
          )
        )
      )

        ;; Skip data bytes (advance offset past the data)
        (local.set $offset (i32.add (local.get $offset) (local.get $seg_size)))

        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $cont)
      )
    )
    (i32.store (global.get $OFF_DATA_COUNT) (i32.add (local.get $dc) (local.get $count)))
    (return (global.get $OK))
  )

  ;; ═════════════════════════════════════════════════════════════════════
  ;; Parse element section (segment data for table initialization)
  ;; Each element: mode + [table_idx] + offset_expr + count + func_indices
  ;; ═════════════════════════════════════════════════════════════════════
  (func $parse_element_section (param $offset i32) (param $size i32) (result i32)
    (local $count i32) (local $i i32) (local $mode i32) (local $table_idx i32)
    (local $dest_offset i32) (local $elem_count i32) (local $j i32)
    (local $func_idx i32) (local $p i32) (local $end i32)
    (local $elem_base i32) (local $dc i32)

    (if (call $leb_u32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
    (local.set $count (i32.load (global.get $OFF_SCRATCH0)))
    (local.set $offset (i32.add (local.get $offset) (i32.load (global.get $OFF_SCRATCH1))))
    (if (i32.gt_u (local.get $count) (global.get $MAX_ELEMS)) (then (return (global.get $ERR_PARSE))))

    (local.set $end (i32.add (local.get $offset) (local.get $size)))
    (local.set $dc (i32.load (global.get $OFF_ELEM_COUNT)))
    (local.set $i (i32.const 0))
    (block $lp
      (loop $cont
        (if (i32.ge_u (local.get $i) (local.get $count)) (then (br $lp)))
        (if (i32.ge_u (local.get $offset) (local.get $end)) (then (return (global.get $ERR_PARSE))))

        (local.set $elem_base (i32.add (global.get $OFF_ELEM_BUF) (i32.mul (i32.add (local.get $dc) (local.get $i)) (global.get $SZ_ELEM))))

        ;; Read mode
        (local.set $p (i32.load (global.get $OFF_WASM_PTR)))
        (local.set $mode (i32.load8_u (i32.add (local.get $p) (local.get $offset))))
        (local.set $offset (i32.add (local.get $offset) (i32.const 1)))

        (local.set $table_idx (i32.const 0))
        (local.set $dest_offset (i32.const 0))

        (if (i32.eq (local.get $mode) (i32.const 0))
          (then
            ;; Mode 0: active, implicit table 0. Parse init_expr (i32.const <value> 0x0B)
            (local.set $p (i32.load (global.get $OFF_WASM_PTR)))
            (if (i32.eq (i32.load8_u (i32.add (local.get $p) (local.get $offset))) (i32.const 0x41))
              (then
                (local.set $offset (i32.add (local.get $offset) (i32.const 1)))
                (if (call $leb_i32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
                (local.set $dest_offset (i32.load (global.get $OFF_SCRATCH0)))
                (local.set $offset (i32.add (local.get $offset) (i32.load (global.get $OFF_SCRATCH1))))
              )
            )
            ;; Skip to end (0x0B)
            (block $ie
              (loop $iel
                (if (i32.ge_u (local.get $offset) (local.get $end)) (then (return (global.get $ERR_PARSE))))
                (local.set $p (i32.load (global.get $OFF_WASM_PTR)))
                (if (i32.eq (i32.load8_u (i32.add (local.get $p) (local.get $offset))) (i32.const 0x0B))
                  (then
                    (local.set $offset (i32.add (local.get $offset) (i32.const 1)))
                    (br $ie)
                  )
                )
                (local.set $offset (i32.add (local.get $offset) (i32.const 1)))
                (br $iel)
              )
            )
          )
        )
        (if (i32.eq (local.get $mode) (i32.const 1))
          (then
            ;; Mode 1: passive segment - stored for table.init
          )
        )
        (if (i32.eq (local.get $mode) (i32.const 2))
          (then
            ;; Mode 2: active with explicit table index
            (if (call $leb_u32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
            (local.set $table_idx (i32.load (global.get $OFF_SCRATCH0)))
            (local.set $offset (i32.add (local.get $offset) (i32.load (global.get $OFF_SCRATCH1))))
            (if (i32.ne (local.get $table_idx) (i32.const 0)) (then (return (global.get $ERR_UNSUP))))

            ;; Parse init_expr
            (local.set $p (i32.load (global.get $OFF_WASM_PTR)))
            (if (i32.eq (i32.load8_u (i32.add (local.get $p) (local.get $offset))) (i32.const 0x41))
              (then
                (local.set $offset (i32.add (local.get $offset) (i32.const 1)))
                (if (call $leb_i32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
                (local.set $dest_offset (i32.load (global.get $OFF_SCRATCH0)))
                (local.set $offset (i32.add (local.get $offset) (i32.load (global.get $OFF_SCRATCH1))))
              )
            )
            (block $ie2
              (loop $iel2
                (if (i32.ge_u (local.get $offset) (local.get $end)) (then (return (global.get $ERR_PARSE))))
                (local.set $p (i32.load (global.get $OFF_WASM_PTR)))
                (if (i32.eq (i32.load8_u (i32.add (local.get $p) (local.get $offset))) (i32.const 0x0B))
                  (then
                    (local.set $offset (i32.add (local.get $offset) (i32.const 1)))
                    (br $ie2)
                  )
                )
                (local.set $offset (i32.add (local.get $offset) (i32.const 1)))
                (br $iel2)
              )
            )
          )
        )

        ;; Store mode to element buffer (0=dropped flag, then mode, dest_offset)
        (i32.store (local.get $elem_base) (i32.const 0))           ;; dropped = 0 (active)
        (i32.store (i32.add (local.get $elem_base) (i32.const 4)) (local.get $dest_offset))  ;; dest_offset

        ;; Read element count
        (if (call $leb_u32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
        (local.set $elem_count (i32.load (global.get $OFF_SCRATCH0)))
        (local.set $offset (i32.add (local.get $offset) (i32.load (global.get $OFF_SCRATCH1))))

        ;; Store element count
        (i32.store (i32.add (local.get $elem_base) (i32.const 8)) (local.get $elem_count))

        ;; Read and store each func idx in the table and element buffer
        (local.set $j (i32.const 0))
        (block $elem_lp
          (loop $elem_cont
            (if (i32.ge_u (local.get $j) (local.get $elem_count)) (then (br $elem_lp)))

            (if (call $leb_u32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
            (local.set $func_idx (i32.load (global.get $OFF_SCRATCH0)))
            (local.set $offset (i32.add (local.get $offset) (i32.load (global.get $OFF_SCRATCH1))))

            ;; Store func_idx in element buffer for table.init
            (i32.store
              (i32.add (local.get $elem_base) (i32.add (i32.const 12) (i32.shl (local.get $j) (i32.const 2))))
              (local.get $func_idx)
            )

            ;; For active segments (mode 0/2), also write to table immediately
            (if (i32.le_u (local.get $mode) (i32.const 2))
              (then
                (if (i32.ne (local.get $mode) (i32.const 1))
                  (then
                    (i32.store
                      (i32.add (global.get $OFF_TABLE_DATA) (i32.shl (i32.add (local.get $dest_offset) (local.get $j)) (i32.const 2)))
                      (local.get $func_idx)
                    )
                  )
                )
              )
            )

            (local.set $j (i32.add (local.get $j) (i32.const 1)))
            (br $elem_cont)
          )
        )

        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $cont)
      )
    )
    (i32.store (global.get $OFF_ELEM_COUNT) (i32.add (local.get $dc) (local.get $count)))
    (return (global.get $OK))
  )

  ;; ═════════════════════════════════════════════════════════════════════
  ;; Decode opcodes: translate raw bytecode to DecodedOp entries
  ;; Input: offset (in wasm bytes), len (bytecode length)
  ;; Output: decoded_start (index into decoded_ops cache), decoded_count
  ;; Stored to scratch0/scratch1
  ;; ═════════════════════════════════════════════════════════════════════
  (func $decode_opcodes (export "decode_opcodes") (param $offset i32) (param $len i32) (result i32)
    (local $end i32) (local $start_idx i32) (local $dc i32)
    (local $op i32) (local $p i32) (local $imm0 i32) (local $imm1 i32)
    (local $adv i32) (local $base i32)

    (local.set $end (i32.add (local.get $offset) (local.get $len)))
    (local.set $start_idx (i32.load (global.get $OFF_DECODED_COUNT)))  ;; running decoded op count
    (local.set $dc (local.get $start_idx))

    (block $lp
      (loop $cont
        (if (i32.ge_u (local.get $offset) (local.get $end)) (then (br $lp)))

        (local.set $p (i32.load (global.get $OFF_WASM_PTR)))
        (local.set $op (i32.load8_u (i32.add (local.get $p) (local.get $offset))))
        (local.set $offset (i32.add (local.get $offset) (i32.const 1)))

        ;; Compute decoded op base in cache
        (local.set $base (i32.add (global.get $OFF_DECODED_OPS) (i32.mul (local.get $dc) (global.get $DEC_SZ))))
        ;; Store opcode
        (i32.store8 (local.get $base) (local.get $op))

        ;; Decode immediates based on opcode
        (block $op_handled
          ;; ── No immediate ops ──
          (if (i32.le_u (local.get $op) (i32.const 0x01))   ;; unreachable, nop
            (then (br $op_handled))
          )
          (if (i32.eq (local.get $op) (i32.const 0x0B))     ;; end
            (then (br $op_handled))
          )
          (if (i32.eq (local.get $op) (i32.const 0x0F))     ;; return
            (then (br $op_handled))
          )
          (if (i32.eq (local.get $op) (i32.const 0x1A))     ;; drop
            (then (br $op_handled))
          )
          (if (i32.eq (local.get $op) (i32.const 0x45))     ;; i32.eqz
            (then (br $op_handled))
          )
          ;; All comparison, arithmetic, conversion ops (0x46-0xC4 excl block/loop/if)
          (if (i32.and (i32.ge_u (local.get $op) (i32.const 0x46)) (i32.le_u (local.get $op) (i32.const 0xC4)))
            (then
              (if (i32.or (i32.eq (local.get $op) (i32.const 0x02)) (i32.eq (local.get $op) (i32.const 0x03)))
                (then)  ;; block/loop handled below
                (else (br $op_handled))
              )
            )
          )

          ;; ── LEB128 immediate ops ──
          (if (i32.or (i32.eq (local.get $op) (i32.const 0x0C)) (i32.eq (local.get $op) (i32.const 0x0D))) ;; br, br_if
            (then
              (if (call $leb_u32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
              (i32.store (i32.add (local.get $base) (i32.const 4)) (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $offset (i32.add (local.get $offset) (i32.load (global.get $OFF_SCRATCH1))))
              (br $op_handled)
            )
          )
          (if (i32.eq (local.get $op) (i32.const 0x10))     ;; call
            (then
              (if (call $leb_u32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
              (i32.store (i32.add (local.get $base) (i32.const 4)) (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $offset (i32.add (local.get $offset) (i32.load (global.get $OFF_SCRATCH1))))
              (br $op_handled)
            )
          )
          (if (i32.eq (local.get $op) (i32.const 0x11))     ;; call_indirect
            (then
              (if (call $leb_u32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
              (i32.store (i32.add (local.get $base) (i32.const 4)) (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $offset (i32.add (local.get $offset) (i32.load (global.get $OFF_SCRATCH1))))
              (if (call $leb_u32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
              (i32.store (i32.add (local.get $base) (i32.const 8)) (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $offset (i32.add (local.get $offset) (i32.load (global.get $OFF_SCRATCH1))))
              (br $op_handled)
            )
          )
          (if (i32.and (i32.ge_u (local.get $op) (i32.const 0x20)) (i32.le_u (local.get $op) (i32.const 0x26))) ;; local.get/set/tee/global.get/set, table.get/set
            (then
              (if (call $leb_u32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
              (i32.store (i32.add (local.get $base) (i32.const 4)) (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $offset (i32.add (local.get $offset) (i32.load (global.get $OFF_SCRATCH1))))
              (br $op_handled)
            )
          )
          (if (i32.eq (local.get $op) (i32.const 0x41))     ;; i32.const (signed LEB128)
            (then
              (if (call $leb_i32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
              (i32.store (i32.add (local.get $base) (i32.const 4)) (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $offset (i32.add (local.get $offset) (i32.load (global.get $OFF_SCRATCH1))))
              (br $op_handled)
            )
          )
          (if (i32.eq (local.get $op) (i32.const 0x42))     ;; i64.const (signed LEB128)
            (then
              (if (call $leb_i64 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
              (i64.store (i32.add (local.get $base) (i32.const 4)) (i64.load (global.get $OFF_SCRATCH0)))
              (local.set $offset (i32.add (local.get $offset) (i32.load (global.get $OFF_SCRATCH2))))
              (br $op_handled)
            )
          )
          (if (i32.eq (local.get $op) (i32.const 0x43))     ;; f32.const
            (then
              (local.set $p (i32.load (global.get $OFF_WASM_PTR)))
              (i32.store (i32.add (local.get $base) (i32.const 4))
                (i32.load (i32.add (local.get $p) (local.get $offset)))
              )
              (local.set $offset (i32.add (local.get $offset) (i32.const 4)))
              (br $op_handled)
            )
          )
          (if (i32.eq (local.get $op) (i32.const 0x44))     ;; f64.const
            (then
              (local.set $p (i32.load (global.get $OFF_WASM_PTR)))
              (i64.store (i32.add (local.get $base) (i32.const 4))
                (i64.load (i32.add (local.get $p) (local.get $offset)))
              )
              (local.set $offset (i32.add (local.get $offset) (i32.const 8)))
              (br $op_handled)
            )
          )

          ;; ── Memory ops (load/store): align + offset ──
          (if (i32.and (i32.ge_u (local.get $op) (i32.const 0x28)) (i32.le_u (local.get $op) (i32.const 0x3E)))
            (then
              ;; align (LEB128)
              (if (call $leb_u32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
              (local.set $imm0 (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $offset (i32.add (local.get $offset) (i32.load (global.get $OFF_SCRATCH1))))
              ;; offset (LEB128)
              (if (call $leb_u32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
              (local.set $imm1 (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $offset (i32.add (local.get $offset) (i32.load (global.get $OFF_SCRATCH1))))
              (i32.store (i32.add (local.get $base) (i32.const 4)) (local.get $imm0))  ;; align
              (i32.store (i32.add (local.get $base) (i32.const 8)) (local.get $imm1))  ;; mem offset
              (br $op_handled)
            )
          )

          ;; ── Block/loop/if: block type ──
          (if (i32.or (i32.eq (local.get $op) (i32.const 0x02)) (i32.eq (local.get $op) (i32.const 0x03)))
            (then
              ;; Block type: either empty(0x40), a value type byte, or a signed LEB128 type index
              (local.set $p (i32.load (global.get $OFF_WASM_PTR)))
              (local.set $imm0 (i32.load8_u (i32.add (local.get $p) (local.get $offset))))
              (local.set $offset (i32.add (local.get $offset) (i32.const 1)))
              (i32.store (i32.add (local.get $base) (i32.const 4)) (local.get $imm0))
              (br $op_handled)
            )
          )
          (if (i32.eq (local.get $op) (i32.const 0x04))     ;; if
            (then
              ;; Same as block
              (local.set $p (i32.load (global.get $OFF_WASM_PTR)))
              (local.set $imm0 (i32.load8_u (i32.add (local.get $p) (local.get $offset))))
              (local.set $offset (i32.add (local.get $offset) (i32.const 1)))
              (i32.store (i32.add (local.get $base) (i32.const 4)) (local.get $imm0))
              (br $op_handled)
            )
          )

          ;; ── br_table: count + labels + default ──
          (if (i32.eq (local.get $op) (i32.const 0x0E))
            (then
              ;; Read label count
              (if (call $leb_u32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
              (i32.store (i32.add (local.get $base) (i32.const 4)) (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $offset (i32.add (local.get $offset) (i32.load (global.get $OFF_SCRATCH1))))
              ;; Skip labels + default
              (local.set $imm0 (i32.load (i32.add (local.get $base) (i32.const 4))))
              (local.set $imm1 (i32.const 0))
              (block $bt_lp
                (loop $bt_cont
                  (if (i32.ge_u (local.get $imm1) (local.get $imm0)) (then (br $bt_lp)))
                  (if (call $leb_u32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
                  (local.set $offset (i32.add (local.get $offset) (i32.load (global.get $OFF_SCRATCH1))))
                  (local.set $imm1 (i32.add (local.get $imm1) (i32.const 1)))
                  (br $bt_cont)
                )
              )
              ;; Read default label and store as imm1 (at base+8)
              (if (call $leb_u32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
              (i32.store (i32.add (local.get $base) (i32.const 8)) (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $offset (i32.add (local.get $offset) (i32.load (global.get $OFF_SCRATCH1))))
              (br $op_handled)
            )
          )

          ;; ── else (0x05) ──
          (if (i32.eq (local.get $op) (i32.const 0x05))
            (then (br $op_handled))
          )

          ;; ── select (0x1B, 0x1C) ──
          (if (i32.eq (local.get $op) (i32.const 0x1B))
            (then (br $op_handled))
          )
          (if (i32.eq (local.get $op) (i32.const 0x1C))
            (then
              ;; typed select: skip result types
              (if (call $leb_u32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
              (local.set $offset (i32.add (local.get $offset) (i32.load (global.get $OFF_SCRATCH1))))
              (br $op_handled)
            )
          )

          ;; ── memory.size (0x3F), memory.grow (0x40): 1 byte immediate (0x00) ──
          (if (i32.or (i32.eq (local.get $op) (i32.const 0x3F)) (i32.eq (local.get $op) (i32.const 0x40)))
            (then
              (local.set $offset (i32.add (local.get $offset) (i32.const 1)))  ;; skip 0x00 byte
              (br $op_handled)
            )
          )

          ;; ── ref.null (0xD0): 1 byte immediate (reftype) ──
          (if (i32.eq (local.get $op) (i32.const 0xD0))
            (then
              (local.set $p (i32.load (global.get $OFF_WASM_PTR)))
              (i32.store (i32.add (local.get $base) (i32.const 4))
                (i32.load8_u (i32.add (local.get $p) (local.get $offset)))
              )
              (local.set $offset (i32.add (local.get $offset) (i32.const 1)))
              (br $op_handled)
            )
          )
          (if (i32.eq (local.get $op) (i32.const 0xD1))     ;; ref.is_null
            (then (br $op_handled))
          )
          (if (i32.eq (local.get $op) (i32.const 0xD2))     ;; ref.func
            (then
              (if (call $leb_u32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
              (i32.store (i32.add (local.get $base) (i32.const 4)) (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $offset (i32.add (local.get $offset) (i32.load (global.get $OFF_SCRATCH1))))
              (br $op_handled)
            )
          )

          ;; ── Extended prefix (0xFC) ──
          (if (i32.eq (local.get $op) (i32.const 0xFC))
            (then
              ;; Read sub-opcode
              (if (call $leb_u32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
              (i32.store (i32.add (local.get $base) (i32.const 4)) (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $offset (i32.add (local.get $offset) (i32.load (global.get $OFF_SCRATCH1))))
              ;; Consume reserved immediates based on sub-opcode
              (local.set $imm0 (i32.load (i32.add (local.get $base) (i32.const 4))))
              (if (i32.eq (local.get $imm0) (i32.const 0x0A))     ;; memory.copy: 2 reserved bytes
                (then (local.set $offset (i32.add (local.get $offset) (i32.const 2))))
              )
              (if (i32.eq (local.get $imm0) (i32.const 0x0B))     ;; memory.fill: 1 reserved byte
                (then (local.set $offset (i32.add (local.get $offset) (i32.const 1))))
              )
              (if (i32.eq (local.get $imm0) (i32.const 0x08))     ;; memory.init: data_seg idx + 1 reserved byte
                (then
                  (if (call $leb_u32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
                  (i32.store (i32.add (local.get $base) (i32.const 8)) (i32.load (global.get $OFF_SCRATCH0)))
                  (local.set $offset (i32.add (local.get $offset) (i32.load (global.get $OFF_SCRATCH1))))
                  (local.set $offset (i32.add (local.get $offset) (i32.const 1)))
                )
              )
              (if (i32.eq (local.get $imm0) (i32.const 0x09))     ;; data.drop: data_seg idx
                (then
                  (if (call $leb_u32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
                  (i32.store (i32.add (local.get $base) (i32.const 8)) (i32.load (global.get $OFF_SCRATCH0)))
                  (local.set $offset (i32.add (local.get $offset) (i32.load (global.get $OFF_SCRATCH1))))
                )
              )
              (if (i32.eq (local.get $imm0) (i32.const 0x0C))     ;; table.init: elem_idx + table_idx
                (then
                  (if (call $leb_u32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
                  (i32.store (i32.add (local.get $base) (i32.const 8)) (i32.load (global.get $OFF_SCRATCH0)))
                  (local.set $offset (i32.add (local.get $offset) (i32.load (global.get $OFF_SCRATCH1))))
                  (if (call $leb_u32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
                  (i32.store (i32.add (local.get $base) (i32.const 12)) (i32.load (global.get $OFF_SCRATCH0)))
                  (local.set $offset (i32.add (local.get $offset) (i32.load (global.get $OFF_SCRATCH1))))
                )
              )
              (if (i32.eq (local.get $imm0) (i32.const 0x0D))     ;; elem.drop: elem_idx
                (then
                  (if (call $leb_u32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
                  (i32.store (i32.add (local.get $base) (i32.const 8)) (i32.load (global.get $OFF_SCRATCH0)))
                  (local.set $offset (i32.add (local.get $offset) (i32.load (global.get $OFF_SCRATCH1))))
                )
              )
              (if (i32.eq (local.get $imm0) (i32.const 0x0E))     ;; table.copy: dst + src table idx
                (then
                  (if (call $leb_u32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
                  (i32.store (i32.add (local.get $base) (i32.const 8)) (i32.load (global.get $OFF_SCRATCH0)))
                  (local.set $offset (i32.add (local.get $offset) (i32.load (global.get $OFF_SCRATCH1))))
                  (if (call $leb_u32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
                  (i32.store (i32.add (local.get $base) (i32.const 12)) (i32.load (global.get $OFF_SCRATCH0)))
                  (local.set $offset (i32.add (local.get $offset) (i32.load (global.get $OFF_SCRATCH1))))
                )
              )
              (if (i32.eq (local.get $imm0) (i32.const 0x0F))     ;; table.grow: table_idx
                (then
                  (if (call $leb_u32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
                  (i32.store (i32.add (local.get $base) (i32.const 8)) (i32.load (global.get $OFF_SCRATCH0)))
                  (local.set $offset (i32.add (local.get $offset) (i32.load (global.get $OFF_SCRATCH1))))
                )
              )
              (if (i32.eq (local.get $imm0) (i32.const 0x10))     ;; table.size: table_idx
                (then
                  (if (call $leb_u32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
                  (i32.store (i32.add (local.get $base) (i32.const 8)) (i32.load (global.get $OFF_SCRATCH0)))
                  (local.set $offset (i32.add (local.get $offset) (i32.load (global.get $OFF_SCRATCH1))))
                )
              )
              (if (i32.eq (local.get $imm0) (i32.const 0x11))     ;; table.fill: table_idx
                (then
                  (if (call $leb_u32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
                  (i32.store (i32.add (local.get $base) (i32.const 8)) (i32.load (global.get $OFF_SCRATCH0)))
                  (local.set $offset (i32.add (local.get $offset) (i32.load (global.get $OFF_SCRATCH1))))
                )
              )
              (br $op_handled)
            )
          )

          ;; ── SIMD prefix (0xFD) ──
          (if (i32.eq (local.get $op) (i32.const 0xFD))
            (then
              ;; Read sub-opcode (LEB128 u32)
              (if (call $leb_u32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
              (i32.store (i32.add (local.get $base) (i32.const 4)) (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $offset (i32.add (local.get $offset) (i32.load (global.get $OFF_SCRATCH1))))
              (local.set $imm0 (i32.load (i32.add (local.get $base) (i32.const 4))))

              ;; Memory ops (raw sub-opcodes 0x00-0x1F except 0x0C): align + offset
              ;; Uses raw $imm0 BEFORE translation to capture both old and modern encodings
              (if (i32.and (i32.le_u (local.get $imm0) (i32.const 0x1F))
                           (i32.ne (local.get $imm0) (i32.const 0x0C)))
                (then
                  (if (call $leb_u32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
                  (local.set $imm1 (i32.load (global.get $OFF_SCRATCH0)))
                  (local.set $offset (i32.add (local.get $offset) (i32.load (global.get $OFF_SCRATCH1))))
                  (if (call $leb_u32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
                  (i32.store (i32.add (local.get $base) (i32.const 8)) (i32.load (global.get $OFF_SCRATCH0)))
                  (local.set $offset (i32.add (local.get $offset) (i32.load (global.get $OFF_SCRATCH1))))
                  (i32.store (i32.add (local.get $base) (i32.const 12)) (local.get $imm1))  ;; align
                )
              )

              ;; v128.const (sub-opcode 0x0C): read 16 bytes into base+16
              (if (i32.eq (local.get $imm0) (i32.const 0x0C))
                (then
                  (local.set $p (i32.load (global.get $OFF_WASM_PTR)))
                  (i64.store (i32.add (local.get $base) (i32.const 16))
                    (i64.load (i32.add (local.get $p) (local.get $offset))))
                  (i64.store (i32.add (local.get $base) (i32.const 24))
                    (i64.load (i32.add (local.get $p) (i32.add (local.get $offset) (i32.const 8)))))
                  (local.set $offset (i32.add (local.get $offset) (i32.const 16)))
                )
              )

              ;; ── Canonicalize sub-opcode (modern wabt → old encoding) ──
              ;; Only applied when $SIMD_MODERN_ENCODING is set to 1 (default 0).
              ;; Old-encoding input passes through unchanged.
              (if (global.get $SIMD_MODERN_ENCODING)
                (then
                  ;; Memory ops (modern store at 0x0B → canonical 0x1B)
                  (if (i32.eq (local.get $imm0) (i32.const 0x0B)) (then (local.set $imm0 (i32.const 0x1B))))
                  ;; Load-splat extends (modern only)
                  (if (i32.eq (local.get $imm0) (i32.const 0x07)) (then (local.set $imm0 (i32.const 0xA6))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x08)) (then (local.set $imm0 (i32.const 0xA7))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x09)) (then (local.set $imm0 (i32.const 0xA8))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x0A)) (then (local.set $imm0 (i32.const 0xA9))))
                  ;; Swizzle (modern only)
                  (if (i32.eq (local.get $imm0) (i32.const 0x0E)) (then (local.set $imm0 (i32.const 0xAA))))
                  ;; Splats
                  (if (i32.eq (local.get $imm0) (i32.const 0x0F)) (then (local.set $imm0 (i32.const 0x2D))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x10)) (then (local.set $imm0 (i32.const 0x31))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x11)) (then (local.set $imm0 (i32.const 0x35))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x12)) (then (local.set $imm0 (i32.const 0x39))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x13)) (then (local.set $imm0 (i32.const 0x3A))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x14)) (then (local.set $imm0 (i32.const 0x3D))))
                  ;; Extract/replace lanes
                  (if (i32.eq (local.get $imm0) (i32.const 0x15)) (then (local.set $imm0 (i32.const 0x2E))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x16)) (then (local.set $imm0 (i32.const 0x2F))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x17)) (then (local.set $imm0 (i32.const 0x30))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x18)) (then (local.set $imm0 (i32.const 0x32))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x19)) (then (local.set $imm0 (i32.const 0x33))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x1A)) (then (local.set $imm0 (i32.const 0x34))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x1B)) (then (local.set $imm0 (i32.const 0x36))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x1C)) (then (local.set $imm0 (i32.const 0x37))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x1D)) (then (local.set $imm0 (i32.const 0x38))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x1E)) (then (local.set $imm0 (i32.const 0xAB))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x1F)) (then (local.set $imm0 (i32.const 0x3B))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x20)) (then (local.set $imm0 (i32.const 0x3C))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x21)) (then (local.set $imm0 (i32.const 0x3E))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x22)) (then (local.set $imm0 (i32.const 0x3F))))
                  ;; Integer comparisons (modern 0x23-0x40 → canonical)
                  (if (i32.eq (local.get $imm0) (i32.const 0x23)) (then (local.set $imm0 (i32.const 0x47))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x24)) (then (local.set $imm0 (i32.const 0x48))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x25)) (then (local.set $imm0 (i32.const 0x4B))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x26)) (then (local.set $imm0 (i32.const 0x4A))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x27)) (then (local.set $imm0 (i32.const 0x4D))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x28)) (then (local.set $imm0 (i32.const 0x4C))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x29)) (then (local.set $imm0 (i32.const 0x4F))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x2A)) (then (local.set $imm0 (i32.const 0x4E))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x2B)) (then (local.set $imm0 (i32.const 0x49))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x2C)) (then (local.set $imm0 (i32.const 0xAE))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x2D)) (then (local.set $imm0 (i32.const 0x58))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x2E)) (then (local.set $imm0 (i32.const 0x59))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x2F)) (then (local.set $imm0 (i32.const 0x5C))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x30)) (then (local.set $imm0 (i32.const 0x5B))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x31)) (then (local.set $imm0 (i32.const 0x5E))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x32)) (then (local.set $imm0 (i32.const 0x5D))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x33)) (then (local.set $imm0 (i32.const 0x60))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x34)) (then (local.set $imm0 (i32.const 0x5F))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x35)) (then (local.set $imm0 (i32.const 0x5A))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x36)) (then (local.set $imm0 (i32.const 0xAF))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x37)) (then (local.set $imm0 (i32.const 0x69))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x38)) (then (local.set $imm0 (i32.const 0x6A))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x39)) (then (local.set $imm0 (i32.const 0x6D))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x3A)) (then (local.set $imm0 (i32.const 0x6C))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x3B)) (then (local.set $imm0 (i32.const 0x6F))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x3C)) (then (local.set $imm0 (i32.const 0x6E))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x3D)) (then (local.set $imm0 (i32.const 0x71))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x3E)) (then (local.set $imm0 (i32.const 0x70))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x3F)) (then (local.set $imm0 (i32.const 0x6B))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x40)) (then (local.set $imm0 (i32.const 0xB0))))
                  ;; Float comparisons (modern 0x41-0x4C → canonical)
                  (if (i32.eq (local.get $imm0) (i32.const 0x41)) (then (local.set $imm0 (i32.const 0x8C))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x42)) (then (local.set $imm0 (i32.const 0x8B))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x43)) (then (local.set $imm0 (i32.const 0x8E))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x44)) (then (local.set $imm0 (i32.const 0x90))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x45)) (then (local.set $imm0 (i32.const 0x92))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x46)) (then (local.set $imm0 (i32.const 0x94))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x47)) (then (local.set $imm0 (i32.const 0x9D))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x48)) (then (local.set $imm0 (i32.const 0x9C))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x49)) (then (local.set $imm0 (i32.const 0x9F))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x4A)) (then (local.set $imm0 (i32.const 0xA1))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x4B)) (then (local.set $imm0 (i32.const 0xA3))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x4C)) (then (local.set $imm0 (i32.const 0xA5))))
                  ;; v128 bitwise (modern → canonical)
                  (if (i32.eq (local.get $imm0) (i32.const 0x4D)) (then (local.set $imm0 (i32.const 0xAC))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x4E)) (then (local.set $imm0 (i32.const 0x43))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x50)) (then (local.set $imm0 (i32.const 0x44))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x51)) (then (local.set $imm0 (i32.const 0x45))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x52)) (then (local.set $imm0 (i32.const 0xAD))))
                  ;; i8x16 ops (modern → canonical)
                  (if (i32.eq (local.get $imm0) (i32.const 0x61)) (then (local.set $imm0 (i32.const 0x46))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x6E)) (then (local.set $imm0 (i32.const 0x40))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x71)) (then (local.set $imm0 (i32.const 0x41))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x77)) (then (local.set $imm0 (i32.const 0xB8))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x79)) (then (local.set $imm0 (i32.const 0xBA))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x7B)) (then (local.set $imm0 (i32.const 0xBB))))
                  ;; i16x8 ops (modern → canonical)
                  (if (i32.eq (local.get $imm0) (i32.const 0x81)) (then (local.set $imm0 (i32.const 0x57))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x8E)) (then (local.set $imm0 (i32.const 0x51))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x91)) (then (local.set $imm0 (i32.const 0x52))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x95)) (then (local.set $imm0 (i32.const 0x61))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x97)) (then (local.set $imm0 (i32.const 0xBC))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x99)) (then (local.set $imm0 (i32.const 0xBE))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x9B)) (then (local.set $imm0 (i32.const 0xBF))))
                  ;; i32x4 ops (modern → canonical)
                  (if (i32.eq (local.get $imm0) (i32.const 0xA1)) (then (local.set $imm0 (i32.const 0x68))))
                  (if (i32.eq (local.get $imm0) (i32.const 0xAE)) (then (local.set $imm0 (i32.const 0x62))))
                  (if (i32.eq (local.get $imm0) (i32.const 0xB1)) (then (local.set $imm0 (i32.const 0x63))))
                  (if (i32.eq (local.get $imm0) (i32.const 0xB5)) (then (local.set $imm0 (i32.const 0x72))))
                  ;; i64x2 ops (modern → canonical)
                  (if (i32.eq (local.get $imm0) (i32.const 0xC1)) (then (local.set $imm0 (i32.const 0x79))))
                  (if (i32.eq (local.get $imm0) (i32.const 0xCE)) (then (local.set $imm0 (i32.const 0x73))))
                  (if (i32.eq (local.get $imm0) (i32.const 0xD1)) (then (local.set $imm0 (i32.const 0x74))))
                  ;; f32x4 ops (modern → canonical)
                  (if (i32.eq (local.get $imm0) (i32.const 0xE0)) (then (local.set $imm0 (i32.const 0x88))))
                  (if (i32.eq (local.get $imm0) (i32.const 0xE1)) (then (local.set $imm0 (i32.const 0x8A))))
                  (if (i32.eq (local.get $imm0) (i32.const 0xE4)) (then (local.set $imm0 (i32.const 0x84))))
                  (if (i32.eq (local.get $imm0) (i32.const 0xE5)) (then (local.set $imm0 (i32.const 0x85))))
                  (if (i32.eq (local.get $imm0) (i32.const 0xE6)) (then (local.set $imm0 (i32.const 0x87))))
                  (if (i32.eq (local.get $imm0) (i32.const 0xE7)) (then (local.set $imm0 (i32.const 0x89))))
                  (if (i32.eq (local.get $imm0) (i32.const 0xE8)) (then (local.set $imm0 (i32.const 0xE1))))
                  (if (i32.eq (local.get $imm0) (i32.const 0xE9)) (then (local.set $imm0 (i32.const 0xE2))))
                  ;; f64x2 ops (modern → canonical)
                  (if (i32.eq (local.get $imm0) (i32.const 0xEC)) (then (local.set $imm0 (i32.const 0x9A))))
                  (if (i32.eq (local.get $imm0) (i32.const 0xED)) (then (local.set $imm0 (i32.const 0x9B))))
                  (if (i32.eq (local.get $imm0) (i32.const 0xF0)) (then (local.set $imm0 (i32.const 0x95))))
                  (if (i32.eq (local.get $imm0) (i32.const 0xF1)) (then (local.set $imm0 (i32.const 0x96))))
                  (if (i32.eq (local.get $imm0) (i32.const 0xF2)) (then (local.set $imm0 (i32.const 0x98))))
                  (if (i32.eq (local.get $imm0) (i32.const 0xF3)) (then (local.set $imm0 (i32.const 0x99))))
                  (if (i32.eq (local.get $imm0) (i32.const 0xF4)) (then (local.set $imm0 (i32.const 0xE3))))
                  (if (i32.eq (local.get $imm0) (i32.const 0xF5)) (then (local.set $imm0 (i32.const 0xE4))))
                )
              )

              ;; Store canonical sub-opcode (translated or pass-through)
              (i32.store (i32.add (local.get $base) (i32.const 4)) (local.get $imm0))

              ;; Lane index ops (extract_lane, replace_lane only): 1-byte lane index
              ;; Note: splat ops (0x2D, 0x31, 0x35, 0x39, 0x3A, 0x3D) do NOT have a lane index
              ;; Check specific extract/replace ranges: [0x2E-0x30], [0x32-0x34], [0x36-0x38], [0x3B-0x3C], [0x3E-0x3F]
              (local.set $adv (i32.const 0))
              (if (i32.and (i32.ge_u (local.get $imm0) (i32.const 0x2E)) (i32.le_u (local.get $imm0) (i32.const 0x30)))
                (then (local.set $adv (i32.const 1))))
              (if (i32.and (i32.ge_u (local.get $imm0) (i32.const 0x32)) (i32.le_u (local.get $imm0) (i32.const 0x34)))
                (then (local.set $adv (i32.const 1))))
              (if (i32.and (i32.ge_u (local.get $imm0) (i32.const 0x36)) (i32.le_u (local.get $imm0) (i32.const 0x38)))
                (then (local.set $adv (i32.const 1))))
              (if (i32.and (i32.ge_u (local.get $imm0) (i32.const 0x3B)) (i32.le_u (local.get $imm0) (i32.const 0x3C)))
                (then (local.set $adv (i32.const 1))))
              (if (i32.and (i32.ge_u (local.get $imm0) (i32.const 0x3E)) (i32.le_u (local.get $imm0) (i32.const 0x3F)))
                (then (local.set $adv (i32.const 1))))
              (if (i32.eq (local.get $imm0) (i32.const 0xAB))
                (then (local.set $adv (i32.const 1))))  ;; i64x2.replace_lane (new canonical)
              (if (local.get $adv)
                (then
                  (local.set $p (i32.load (global.get $OFF_WASM_PTR)))
                  (i32.store (i32.add (local.get $base) (i32.const 8))
                    (i32.load8_u (i32.add (local.get $p) (local.get $offset))))
                  (local.set $offset (i32.add (local.get $offset) (i32.const 1)))
                )
              )

              ;; All other 0xFD ops: no extra immediate
              (br $op_handled)
            )
          )

          ;; Unhandled opcode
          (return (global.get $ERR_UNSUP))
        )

        ;; Store decoded op marker
        (local.set $dc (i32.add (local.get $dc) (i32.const 1)))
        (br $cont)
      )
    )

    ;; Store start index and count
    (i32.store (global.get $OFF_SCRATCH0) (local.get $start_idx))
    (i32.store (global.get $OFF_SCRATCH1) (i32.sub (local.get $dc) (local.get $start_idx)))
    (i32.store (global.get $OFF_DECODED_COUNT) (local.get $dc))

    (return (global.get $OK))
  )

  ;; ═════════════════════════════════════════════════════════════════════
  ;; Post-decode pass: compute next_idx for block/loop/if/else
  ;; Scans decoded ops for a function and sets next_idx to point to
  ;; the matching end+1 (for block/if/else) or loop start (for loop)
  ;; Input: start_idx, count (decoded op indices)
  ;; ═════════════════════════════════════════════════════════════════════
  (func $compute_end_targets (export "compute_end_targets") (param $start i32) (param $count i32) (result i32)
    (local $pos i32) (local $op_base i32) (local $op i32)
    (local $sp i32) (local $label_base i32) (local $label_pos i32)

    (i32.store (global.get $OFF_LABEL_STACK_PTR) (i32.const 0))
    (local.set $label_base (global.get $OFF_LABEL_STACK))

    (local.set $pos (i32.const 0))
    (block $lp
      (loop $cont
        (local.set $op_base
          (i32.add (global.get $OFF_DECODED_OPS) (i32.mul (i32.add (local.get $start) (local.get $pos)) (global.get $DEC_SZ)))
        )
        (local.set $op (i32.load8_u (local.get $op_base)))

        (block $skip
          ;; block (0x02), loop (0x03), if (0x04): push current position
          (if (i32.or (i32.eq (local.get $op) (i32.const 0x02))
                      (i32.or (i32.eq (local.get $op) (i32.const 0x03))
                              (i32.eq (local.get $op) (i32.const 0x04))))
            (then
              (local.set $sp (i32.load (global.get $OFF_LABEL_STACK_PTR)))
              (i32.store
                (i32.add (local.get $label_base) (i32.shl (local.get $sp) (i32.const 2)))
                (local.get $pos)
              )
              (i32.store (global.get $OFF_LABEL_STACK_PTR) (i32.add (local.get $sp) (i32.const 1)))
              (br $skip)
            )
          )

          ;; else (0x05): pop top (should be if), set its next_idx, push else
          (if (i32.eq (local.get $op) (i32.const 0x05))
            (then
              (local.set $sp (i32.load (global.get $OFF_LABEL_STACK_PTR)))
              (if (i32.eqz (local.get $sp))
                (then (br $skip))  ;; skip if stack empty (malformed but safe)
              )
              (local.set $sp (i32.sub (local.get $sp) (i32.const 1)))
              (i32.store (global.get $OFF_LABEL_STACK_PTR) (local.get $sp))
              (local.set $label_pos (i32.load (i32.add (local.get $label_base) (i32.shl (local.get $sp) (i32.const 2)))))
              ;; Set next_idx of the matching if to current pos + 1 (skip to else/end)
              (i32.store
                (i32.add (global.get $OFF_DECODED_OPS)
                  (i32.add (i32.mul (local.get $label_pos) (global.get $DEC_SZ)) (i32.const 12))
                )
                (i32.add (local.get $pos) (i32.const 1))
              )
              ;; Push current position as else label
              (i32.store
                (i32.add (local.get $label_base) (i32.shl (local.get $sp) (i32.const 2)))
                (local.get $pos)
              )
              (i32.store (global.get $OFF_LABEL_STACK_PTR) (i32.add (local.get $sp) (i32.const 1)))
              (br $skip)
            )
          )

          ;; end (0x0B): pop top, set its next_idx
          (if (i32.eq (local.get $op) (i32.const 0x0B))
            (then
              (local.set $sp (i32.load (global.get $OFF_LABEL_STACK_PTR)))
              (if (i32.eqz (local.get $sp))
                (then (br $skip))  ;; function-level end: skip
              )
              (local.set $sp (i32.sub (local.get $sp) (i32.const 1)))
              (i32.store (global.get $OFF_LABEL_STACK_PTR) (local.get $sp))
              (local.set $label_pos (i32.load (i32.add (local.get $label_base) (i32.shl (local.get $sp) (i32.const 2)))))
              ;; Check the opcode at the label position to determine if it's a loop
              (local.set $op_base
                (i32.add (global.get $OFF_DECODED_OPS) (i32.mul (i32.add (local.get $start) (local.get $label_pos)) (global.get $DEC_SZ)))
              )
              (local.set $sp (i32.load8_u (local.get $op_base)))
              (i32.store
                (i32.add (global.get $OFF_DECODED_OPS)
                  (i32.add (i32.mul (local.get $label_pos) (global.get $DEC_SZ)) (i32.const 12))
                )
                (if (result i32) (i32.eq (local.get $sp) (i32.const 0x03))
                  (then (local.get $label_pos))
                  (else (i32.add (local.get $pos) (i32.const 1)))
                )
              )
              (br $skip)
            )
          )
        )

        (local.set $pos (i32.add (local.get $pos) (i32.const 1)))
        (if (i32.lt_u (local.get $pos) (local.get $count))
          (then (br $cont))
          (else (br $lp))
        )
      )
    )

    (return (global.get $OK))
  )

  ;; ═════════════════════════════════════════════════════════════════════
  ;; Execution engine ── stack operations
  ;; ═════════════════════════════════════════════════════════════════════
  (func $stack_push (param $val i32) (result i32)
    (if (i32.ge_u (global.get $FAST_STACK_LEN) (global.get $MAX_STACK))
      (then (return (global.get $ERR_STK_OV)))
    )
    (i32.store
      (i32.add (global.get $OFF_EXEC_STACK) (i32.shl (global.get $FAST_STACK_LEN) (i32.const 2)))
      (local.get $val)
    )
    (global.set $FAST_STACK_LEN (i32.add (global.get $FAST_STACK_LEN) (i32.const 1)))
    (return (global.get $OK))
  )

  (func $stack_pop (result i32)
    (if (i32.eqz (global.get $FAST_STACK_LEN))
      (then (return (global.get $ERR_STK_UND)))
    )
    (global.set $FAST_STACK_LEN (i32.sub (global.get $FAST_STACK_LEN) (i32.const 1)))
    (i32.store (global.get $OFF_SCRATCH0)
      (i32.load (i32.add (global.get $OFF_EXEC_STACK) (i32.shl (global.get $FAST_STACK_LEN) (i32.const 2))))
    )
    (return (global.get $OK))
  )

  (func $stack_peek (result i32)
    (if (i32.eqz (global.get $FAST_STACK_LEN))
      (then (return (global.get $ERR_STK_UND)))
    )
    (i32.store (global.get $OFF_SCRATCH0)
      (i32.load (i32.add (global.get $OFF_EXEC_STACK) (i32.shl (i32.sub (global.get $FAST_STACK_LEN) (i32.const 1)) (i32.const 2))))
    )
    (return (global.get $OK))
  )

  ;; ── Fast unchecked stack operations (no scratch0, no error checks) ──
  (func $stack_push_val (param $val i32)
    (i32.store
      (i32.add (global.get $OFF_EXEC_STACK) (i32.shl (global.get $FAST_STACK_LEN) (i32.const 2)))
      (local.get $val)
    )
    (global.set $FAST_STACK_LEN (i32.add (global.get $FAST_STACK_LEN) (i32.const 1)))
  )

  (func $stack_pop_val (result i32)
    (global.set $FAST_STACK_LEN (i32.sub (global.get $FAST_STACK_LEN) (i32.const 1)))
    (return (i32.load (i32.add (global.get $OFF_EXEC_STACK) (i32.shl (global.get $FAST_STACK_LEN) (i32.const 2)))))
  )

  (func $stack_peek_val (result i32)
    (return (i32.load (i32.add (global.get $OFF_EXEC_STACK) (i32.shl (i32.sub (global.get $FAST_STACK_LEN) (i32.const 1)) (i32.const 2)))))
  )

  ;; ═════════════════════════════════════════════════════════════════════
  ;; i64 stack operations — each i64 value occupies two 4-byte slots
  ;; (low 32 bits at lower address, high 32 bits at higher address)
  ;; ═════════════════════════════════════════════════════════════════════
  (func $stack_push_i64 (param $lo i32) (param $hi i32) (result i32)
    (if (i32.ge_u (i32.add (global.get $FAST_STACK_LEN) (i32.const 2)) (global.get $MAX_STACK))
      (then (return (global.get $ERR_STK_OV)))
    )
    (i32.store
      (i32.add (global.get $OFF_EXEC_STACK) (i32.shl (global.get $FAST_STACK_LEN) (i32.const 2)))
      (local.get $lo)
    )
    (i32.store
      (i32.add (global.get $OFF_EXEC_STACK) (i32.shl (i32.add (global.get $FAST_STACK_LEN) (i32.const 1)) (i32.const 2)))
      (local.get $hi)
    )
    (global.set $FAST_STACK_LEN (i32.add (global.get $FAST_STACK_LEN) (i32.const 2)))
    (return (global.get $OK))
  )

  (func $stack_pop_i64 (result i32)
    (if (i32.lt_u (global.get $FAST_STACK_LEN) (i32.const 2))
      (then (return (global.get $ERR_STK_UND)))
    )
    (global.set $FAST_STACK_LEN (i32.sub (global.get $FAST_STACK_LEN) (i32.const 2)))
    (i32.store (global.get $OFF_SCRATCH1)  ;; hi
      (i32.load (i32.add (global.get $OFF_EXEC_STACK) (i32.shl (i32.add (global.get $FAST_STACK_LEN) (i32.const 1)) (i32.const 2))))
    )
    (i32.store (global.get $OFF_SCRATCH0)  ;; lo
      (i32.load (i32.add (global.get $OFF_EXEC_STACK) (i32.shl (global.get $FAST_STACK_LEN) (i32.const 2))))
    )
    (return (global.get $OK))
  )

  (func $stack_peek_i64 (result i32)
    (if (i32.lt_u (global.get $FAST_STACK_LEN) (i32.const 2))
      (then (return (global.get $ERR_STK_UND)))
    )
    (i32.store (global.get $OFF_SCRATCH1)  ;; hi
      (i32.load (i32.add (global.get $OFF_EXEC_STACK) (i32.shl (i32.add (global.get $FAST_STACK_LEN) (i32.const 1)) (i32.const 2))))
    )
    (i32.store (global.get $OFF_SCRATCH0)  ;; lo
      (i32.load (i32.add (global.get $OFF_EXEC_STACK) (i32.shl (i32.sub (global.get $FAST_STACK_LEN) (i32.const 1)) (i32.const 2))))
    )
    (return (global.get $OK))
  )

  ;; ═════════════════════════════════════════════════════════════════════
  ;; Control frame stack operations
  ;; ═════════════════════════════════════════════════════════════════════
  (func $ctrl_push (param $kind i32) (param $target i32) (result i32)
    (local $len i32) (local $base i32)
    (local.set $len (i32.load (global.get $OFF_EXEC_CTRL_LEN)))
    (if (i32.ge_u (local.get $len) (global.get $MAX_CTRL))
      (then (return (global.get $ERR_STK_OV)))
    )
    (local.set $base (i32.add (global.get $OFF_EXEC_CTRL) (i32.mul (local.get $len) (global.get $SZ_CTRL))))
    (i32.store (local.get $base) (local.get $kind))
    (i32.store (i32.add (local.get $base) (i32.const 4)) (local.get $target))
    (i32.store (global.get $OFF_EXEC_CTRL_LEN) (i32.add (local.get $len) (i32.const 1)))
    (return (global.get $OK))
  )

  (func $ctrl_pop (result i32)
    (local $len i32) (local $base i32)
    (local.set $len (i32.load (global.get $OFF_EXEC_CTRL_LEN)))
    (if (i32.eqz (local.get $len))
      (then (return (global.get $ERR_STK_UND)))
    )
    (local.set $len (i32.sub (local.get $len) (i32.const 1)))
    (i32.store (global.get $OFF_EXEC_CTRL_LEN) (local.get $len))
    (local.set $base (i32.add (global.get $OFF_EXEC_CTRL) (i32.mul (local.get $len) (global.get $SZ_CTRL))))
    (i32.store (global.get $OFF_SCRATCH0) (i32.load (local.get $base)))       ;; kind
    (i32.store (global.get $OFF_SCRATCH1) (i32.load (i32.add (local.get $base) (i32.const 4))))  ;; target
    (return (global.get $OK))
  )

  (func $ctrl_peek (param $depth i32) (result i32)
    ;; Returns kind of the Nth ancestor frame (0 = innermost)
    ;; Stores target in scratch1
    (local $len i32) (local $base i32)
    (local.set $len (i32.load (global.get $OFF_EXEC_CTRL_LEN)))
    (local.set $len (i32.sub (local.get $len) (i32.const 1)))
    (if (i32.lt_u (local.get $len) (local.get $depth))
      (then (return (global.get $ERR_STK_UND)))
    )
    (local.set $base (i32.add (global.get $OFF_EXEC_CTRL) (i32.mul (i32.sub (local.get $len) (local.get $depth)) (global.get $SZ_CTRL))))
    (i32.store (global.get $OFF_SCRATCH0) (i32.load (local.get $base)))
    (i32.store (global.get $OFF_SCRATCH1) (i32.load (i32.add (local.get $base) (i32.const 4))))
    (return (global.get $OK))
  )

  (func $ctrl_depth (result i32)
    (return (i32.load (global.get $OFF_EXEC_CTRL_LEN)))
  )

  ;; ═════════════════════════════════════════════════════════════════════
  ;; Execute a decoded function by its function index
  ;; Sets up locals from args if provided, runs dispatch on decoded ops
  ;; ═════════════════════════════════════════════════════════════════════
  (func $exec_fn (param $func_idx i32) (param $args_ptr i32) (param $args_len i32) (result i32)
    (local $code_idx i32) (local $code_base i32)
    (local $local_count i32) (local $i i32) (local $val i32)
    (local $dec_start i32) (local $dec_count i32) (local $dec_idx i32)
    (local $op i32) (local $op_base i32)
    (local $p i32) (local $err i32)
    (local $imm0 i32) (local $imm1 i32) (local $imm2 i32) (local $imm3 i32)
    (local $ctrl_len i32) (local $target i32) (local $kind i32)
    (local $tmp64 i64) (local $tmp64b i64)
    (local $tmp_f32 f32) (local $tmp_f32b f32) (local $tmp_f64 f64) (local $tmp_f64b f64)
    (local $type_idx i32) (local $result_count i32)

    ;; Find code index for this function (adjusting for imports)
    (if (i32.lt_u (local.get $func_idx) (i32.load (global.get $OFF_IMPORT_COUNT)))
      (then (return (global.get $ERR_MISS_IMP)))
    )
    (local.set $code_idx (i32.sub (local.get $func_idx) (i32.load (global.get $OFF_IMPORT_COUNT))))

    (local.set $code_base (i32.add (global.get $OFF_CODE_BUF) (i32.mul (local.get $code_idx) (global.get $SZ_CODE))))

    ;; Look up result count from function type
    (local.set $type_idx (i32.load (i32.add (global.get $OFF_FUNCTIONS_BUF) (i32.mul (local.get $code_idx) (global.get $SZ_FUNC)))))
    (local.set $result_count (i32.load16_u (i32.add (global.get $OFF_TYPES_BUF) (i32.add (i32.mul (local.get $type_idx) (global.get $SZ_TYPE)) (i32.const 136)))))

    ;; Read decoded op range
    (local.set $dec_start (i32.load (i32.add (local.get $code_base) (i32.const 24))))
    (local.set $dec_count (i32.load (i32.add (local.get $code_base) (i32.const 32))))
    (local.set $dec_idx (local.get $dec_start))

    ;; Read local count
    (local.set $local_count (i32.load (i32.add (local.get $code_base) (i32.const 16))))
    (i32.store (global.get $OFF_EXEC_LOCAL_COUNT) (local.get $local_count))
    ;; Cache stack length in global for fast access
    (global.set $FAST_STACK_LEN (i32.load (global.get $OFF_EXEC_STACK_LEN)))

    ;; Initialize locals
    (local.set $i (i32.const 0))
    (block $init_lp
      (loop $init_cont
        (if (i32.ge_u (local.get $i) (local.get $local_count)) (then (br $init_lp)))
        (i32.store
          (i32.add (global.get $OFF_EXEC_LOCALS) (i32.shl (local.get $i) (i32.const 2)))
          (i32.const 0)
        )
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $init_cont)
      )
    )

    ;; Copy args into first N locals
    (local.set $i (i32.const 0))
    (block $args_lp
      (loop $args_cont
        (if (i32.ge_u (local.get $i) (local.get $args_len)) (then (br $args_lp)))
        (i32.store
          (i32.add (global.get $OFF_EXEC_LOCALS) (i32.shl (local.get $i) (i32.const 2)))
          (i32.load (i32.add (local.get $args_ptr) (i32.shl (local.get $i) (i32.const 2))))
        )
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $args_cont)
      )
    )

    ;; ── Main dispatch loop ──
    (block $dispatch_end
      (loop $dispatch_loop
        ;; Check if we've exhausted decoded ops
        (if (i32.ge_u (local.get $dec_idx) (i32.add (local.get $dec_start) (local.get $dec_count)))
          (then
            (i32.store (global.get $OFF_EXEC_STACK_LEN) (global.get $FAST_STACK_LEN))
            (br $dispatch_end)
          )
        )

        ;; Read decoded op
        (local.set $op_base (i32.add (global.get $OFF_DECODED_OPS) (i32.mul (local.get $dec_idx) (global.get $DEC_SZ))))
        (local.set $op (i32.load8_u (local.get $op_base)))
        (local.set $imm0 (i32.load (i32.add (local.get $op_base) (i32.const 4))))
        (local.set $imm1 (i32.load (i32.add (local.get $op_base) (i32.const 8))))

        (block $op_done
        (block $unsup
          (block $h255
          (block $h254
          (block $h253
          (block $h252
          (block $h251
          (block $h250
          (block $h249
          (block $h248
          (block $h247
          (block $h246
          (block $h245
          (block $h244
          (block $h243
          (block $h242
          (block $h241
          (block $h240
          (block $h239
          (block $h238
          (block $h237
          (block $h236
          (block $h235
          (block $h234
          (block $h233
          (block $h232
          (block $h231
          (block $h230
          (block $h229
          (block $h228
          (block $h227
          (block $h226
          (block $h225
          (block $h224
          (block $h223
          (block $h222
          (block $h221
          (block $h220
          (block $h219
          (block $h218
          (block $h217
          (block $h216
          (block $h215
          (block $h214
          (block $h213
          (block $h212
          (block $h211
          (block $h210
          (block $h209
          (block $h208
          (block $h207
          (block $h206
          (block $h205
          (block $h204
          (block $h203
          (block $h202
          (block $h201
          (block $h200
          (block $h199
          (block $h198
          (block $h197
          (block $h196
          (block $h195
          (block $h194
          (block $h193
          (block $h192
          (block $h191
          (block $h190
          (block $h189
          (block $h188
          (block $h187
          (block $h186
          (block $h185
          (block $h184
          (block $h183
          (block $h182
          (block $h181
          (block $h180
          (block $h179
          (block $h178
          (block $h177
          (block $h176
          (block $h175
          (block $h174
          (block $h173
          (block $h172
          (block $h171
          (block $h170
          (block $h169
          (block $h168
          (block $h167
          (block $h166
          (block $h165
          (block $h164
          (block $h163
          (block $h162
          (block $h161
          (block $h160
          (block $h159
          (block $h158
          (block $h157
          (block $h156
          (block $h155
          (block $h154
          (block $h153
          (block $h152
          (block $h151
          (block $h150
          (block $h149
          (block $h148
          (block $h147
          (block $h146
          (block $h145
          (block $h144
          (block $h143
          (block $h142
          (block $h141
          (block $h140
          (block $h139
          (block $h138
          (block $h137
          (block $h136
          (block $h135
          (block $h134
          (block $h133
          (block $h132
          (block $h131
          (block $h130
          (block $h129
          (block $h128
          (block $h127
          (block $h126
          (block $h125
          (block $h124
          (block $h123
          (block $h122
          (block $h121
          (block $h120
          (block $h119
          (block $h118
          (block $h117
          (block $h116
          (block $h115
          (block $h114
          (block $h113
          (block $h112
          (block $h111
          (block $h110
          (block $h109
          (block $h108
          (block $h107
          (block $h106
          (block $h105
          (block $h104
          (block $h103
          (block $h102
          (block $h101
          (block $h100
          (block $h99
          (block $h98
          (block $h97
          (block $h96
          (block $h95
          (block $h94
          (block $h93
          (block $h92
          (block $h91
          (block $h90
          (block $h89
          (block $h88
          (block $h87
          (block $h86
          (block $h85
          (block $h84
          (block $h83
          (block $h82
          (block $h81
          (block $h80
          (block $h79
          (block $h78
          (block $h77
          (block $h76
          (block $h75
          (block $h74
          (block $h73
          (block $h72
          (block $h71
          (block $h70
          (block $h69
          (block $h68
          (block $h67
          (block $h66
          (block $h65
          (block $h64
          (block $h63
          (block $h62
          (block $h61
          (block $h60
          (block $h59
          (block $h58
          (block $h57
          (block $h56
          (block $h55
          (block $h54
          (block $h53
          (block $h52
          (block $h51
          (block $h50
          (block $h49
          (block $h48
          (block $h47
          (block $h46
          (block $h45
          (block $h44
          (block $h43
          (block $h42
          (block $h41
          (block $h40
          (block $h39
          (block $h38
          (block $h37
          (block $h36
          (block $h35
          (block $h34
          (block $h33
          (block $h32
          (block $h31
          (block $h30
          (block $h29
          (block $h28
          (block $h27
          (block $h26
          (block $h25
          (block $h24
          (block $h23
          (block $h22
          (block $h21
          (block $h20
          (block $h19
          (block $h18
          (block $h17
          (block $h16
          (block $h15
          (block $h14
          (block $h13
          (block $h12
          (block $h11
          (block $h10
          (block $h9
          (block $h8
          (block $h7
          (block $h6
          (block $h5
          (block $h4
          (block $h3
          (block $h2
          (block $h1
          (block $h0
            (if (i32.gt_u (local.get $op) (i32.const 255))
              (then (return (global.get $ERR_UNSUP)))
            )
            (br_table $h0 $h1 $h2 $h3 $h4 $h5 $h6 $h7 $h8 $h9 $h10 $h11 $h12 $h13 $h14 $h15 $h16 $h17 $h18 $h19 $h20 $h21 $h22 $h23 $h24 $h25 $h26 $h27 $h28 $h29 $h30 $h31 $h32 $h33 $h34 $h35 $h36 $h37 $h38 $h39 $h40 $h41 $h42 $h43 $h44 $h45 $h46 $h47 $h48 $h49 $h50 $h51 $h52 $h53 $h54 $h55 $h56 $h57 $h58 $h59 $h60 $h61 $h62 $h63 $h64 $h65 $h66 $h67 $h68 $h69 $h70 $h71 $h72 $h73 $h74 $h75 $h76 $h77 $h78 $h79 $h80 $h81 $h82 $h83 $h84 $h85 $h86 $h87 $h88 $h89 $h90 $h91 $h92 $h93 $h94 $h95 $h96 $h97 $h98 $h99 $h100 $h101 $h102 $h103 $h104 $h105 $h106 $h107 $h108 $h109 $h110 $h111 $h112 $h113 $h114 $h115 $h116 $h117 $h118 $h119 $h120 $h121 $h122 $h123 $h124 $h125 $h126 $h127 $h128 $h129 $h130 $h131 $h132 $h133 $h134 $h135 $h136 $h137 $h138 $h139 $h140 $h141 $h142 $h143 $h144 $h145 $h146 $h147 $h148 $h149 $h150 $h151 $h152 $h153 $h154 $h155 $h156 $h157 $h158 $h159 $h160 $h161 $h162 $h163 $h164 $h165 $h166 $h167 $h168 $h169 $h170 $h171 $h172 $h173 $h174 $h175 $h176 $h177 $h178 $h179 $h180 $h181 $h182 $h183 $h184 $h185 $h186 $h187 $h188 $h189 $h190 $h191 $h192 $h193 $h194 $h195 $h196 $h197 $h198 $h199 $h200 $h201 $h202 $h203 $h204 $h205 $h206 $h207 $h208 $h209 $h210 $h211 $h212 $h213 $h214 $h215 $h216 $h217 $h218 $h219 $h220 $h221 $h222 $h223 $h224 $h225 $h226 $h227 $h228 $h229 $h230 $h231 $h232 $h233 $h234 $h235 $h236 $h237 $h238 $h239 $h240 $h241 $h242 $h243 $h244 $h245 $h246 $h247 $h248 $h249 $h250 $h251 $h252 $h253 $h254 $h255 $unsup (local.get $op))
          )  ;; close $h0
          )  ;; close $h1
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h2
              (local.set $target (i32.load (i32.add (local.get $op_base) (i32.const 12))))  ;; next_idx
              (local.set $err (call $ctrl_push (i32.const 0x02) (local.get $target)))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h3
              (local.set $target (i32.add (local.get $dec_idx) (i32.const 1)))  ;; loop br target = first op after loop
              (local.set $err (call $ctrl_push (i32.const 0x03) (local.get $target)))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h4
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $val (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $target (i32.load (i32.add (local.get $op_base) (i32.const 12))))  ;; next_idx = else/end+1
              (local.set $err (call $ctrl_push (i32.const 0x04) (local.get $target)))
              (if (local.get $err) (then (return (local.get $err))))
              (if (i32.eqz (local.get $val))
                (then
                  ;; condition is 0: skip to else or end
                  (local.set $dec_idx (local.get $target))
                  (br $dispatch_loop)
                )
              )
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h5
              (local.set $target (i32.load (i32.add (local.get $op_base) (i32.const 12))))  ;; next_idx = end+1
              (local.set $dec_idx (local.get $target))
              (br $dispatch_loop)
          )  ;; close $h6
        (return (global.get $ERR_UNSUP))
          )  ;; close $h7
        (return (global.get $ERR_UNSUP))
          )  ;; close $h8
        (return (global.get $ERR_UNSUP))
          )  ;; close $h9
        (return (global.get $ERR_UNSUP))
          )  ;; close $h10
        (return (global.get $ERR_UNSUP))
          )  ;; close $h11
              (local.set $ctrl_len (call $ctrl_depth))
              ;; If no control frames or this is the last op, it's a function return
              (if (i32.or (i32.eqz (local.get $ctrl_len))
                          (i32.eq (local.get $dec_idx) (i32.sub (i32.add (local.get $dec_start) (local.get $dec_count)) (i32.const 1))))
                (then
                  ;; function-level end: pop result_count values and return
                  (if (local.get $result_count)
                    (then
                      (local.set $i (i32.sub (local.get $result_count) (i32.const 1)))
                      (block $fr_res_lp
                        (loop $fr_res_cont
                          (if (i32.lt_s (local.get $i) (i32.const 0)) (then (br $fr_res_lp)))
                          (local.set $err (call $stack_pop))
                          (if (local.get $err) (then (return (local.get $err))))
                          (i64.store
                            (i32.add (global.get $OFF_EXEC_RESULT) (i32.shl (local.get $i) (i32.const 3)))
                            (i64.extend_i32_s (i32.load (global.get $OFF_SCRATCH0)))
                          )
                          (local.set $i (i32.sub (local.get $i) (i32.const 1)))
                          (br $fr_res_cont)
                        )
                      )
                    )
                  )
                  (i32.store (global.get $OFF_EXEC_RES_COUNT) (local.get $result_count))
                  (i32.store (global.get $OFF_EXEC_CTRL_LEN) (i32.const 0))  ;; reset control stack
                  (i32.store (global.get $OFF_EXEC_STACK_LEN) (global.get $FAST_STACK_LEN))
                  (br $dispatch_end)
                )
                (else
                  ;; block/loop/if end: pop control frame
                  (local.set $err (call $ctrl_pop))
                  (if (local.get $err) (then (return (local.get $err))))
                  (local.set $kind (i32.load (global.get $OFF_SCRATCH0)))
                  (local.set $target (i32.load (global.get $OFF_SCRATCH1)))
                  ;; For loops, jump back to loop start (target is loop's own pos)
                  (if (i32.eq (local.get $kind) (i32.const 0x03))  ;; LOOP
                    (then
                      (local.set $dec_idx (local.get $target))
                      (br $dispatch_loop)
                    )
                  )
                  ;; For block/if/else: fall through to next op
                  (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
                  (br $dispatch_loop)
                )
              )
          )  ;; close $h12
              ;; imm0 = label depth
              (local.set $err (call $ctrl_peek (local.get $imm0)))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $target (i32.load (global.get $OFF_SCRATCH1)))
              (local.set $dec_idx (local.get $target))
              (br $dispatch_loop)
          )  ;; close $h13
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $val (i32.load (global.get $OFF_SCRATCH0)))
              (if (i32.eqz (local.get $val))
                (then
                  ;; condition is 0: fall through
                  (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
                  (br $dispatch_loop)
                )
              )
              ;; condition is true: branch
              (local.set $err (call $ctrl_peek (local.get $imm0)))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $target (i32.load (global.get $OFF_SCRATCH1)))
              (local.set $dec_idx (local.get $target))
              (br $dispatch_loop)
          )  ;; close $h14
              ;; imm0 = number of labels (count)
              ;; imm1 = default label depth (stored by decoder)
              ;; Pop selector value (to keep stack balanced)
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              ;; For MVP: use default label from imm1
              (local.set $err (call $ctrl_peek (local.get $imm1)))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $target (i32.load (global.get $OFF_SCRATCH1)))
              (local.set $dec_idx (local.get $target))
              (br $dispatch_loop)
          )  ;; close $h15
                ;; Pop result_count values into result buffer
                (if (local.get $result_count)
                  (then
                    (local.set $i (i32.sub (local.get $result_count) (i32.const 1)))
                    (block $ret_res_lp
                      (loop $ret_res_cont
                        (if (i32.lt_s (local.get $i) (i32.const 0)) (then (br $ret_res_lp)))
                        (local.set $err (call $stack_pop))
                        (if (local.get $err) (then (return (local.get $err))))
                        (i64.store
                          (i32.add (global.get $OFF_EXEC_RESULT) (i32.shl (local.get $i) (i32.const 3)))
                          (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH0)))
                        )
                        (local.set $i (i32.sub (local.get $i) (i32.const 1)))
                        (br $ret_res_cont)
                      )
                    )
                  )
                )
                (i32.store (global.get $OFF_EXEC_RES_COUNT) (local.get $result_count))
              ;; Reset control stack
              (i32.store (global.get $OFF_EXEC_CTRL_LEN) (i32.const 0))
              (i32.store (global.get $OFF_EXEC_STACK_LEN) (global.get $FAST_STACK_LEN))
              (br $dispatch_end)
          )  ;; close $h16
              ;; imm0 = function index
              ;; Check if it's an imported function
              (if (i32.lt_u (local.get $imm0) (i32.load (global.get $OFF_IMPORT_COUNT)))
                (then (return (global.get $ERR_MISS_IMP)))
              )
              ;; Look up type to get param count (adjust for imports)
              (local.set $val (i32.load (i32.add (global.get $OFF_FUNCTIONS_BUF) (i32.mul (i32.sub (local.get $imm0) (i32.load (global.get $OFF_IMPORT_COUNT))) (global.get $SZ_FUNC)))))
              (local.set $val (i32.add (global.get $OFF_TYPES_BUF) (i32.mul (local.get $val) (global.get $SZ_TYPE))))
              (local.set $val (i32.load16_u (i32.add (local.get $val) (i32.const 128))))  ;; param_count
              ;; Pop params into call args buffer (reversed order - WASM convention)
              (local.set $i (i32.sub (local.get $val) (i32.const 1)))
              (block $pop_params
                (loop $pop_cont
                  (if (i32.lt_s (local.get $i) (i32.const 0)) (then (br $pop_params)))
                  (local.set $err (call $stack_pop))
                  (if (local.get $err) (then (return (local.get $err))))
                  (i32.store
                    (i32.add (global.get $OFF_CALL_ARGS) (i32.shl (local.get $i) (i32.const 2)))
                    (i32.load (global.get $OFF_SCRATCH0))
                  )
                  (local.set $i (i32.sub (local.get $i) (i32.const 1)))
                  (br $pop_cont)
                )
              )
              ;; Flush cached stack length to memory before call
              (i32.store (global.get $OFF_EXEC_STACK_LEN) (global.get $FAST_STACK_LEN))
              ;; Save current state to frame save area
              (local.set $p (i32.load (global.get $OFF_EXEC_CALL_DEPTH)))
              (local.set $op_base (i32.add (global.get $OFF_FRAME_SAVE) (i32.mul (local.get $p) (i32.const 272))))
              (i32.store (local.get $op_base) (i32.add (local.get $dec_idx) (i32.const 1)))  ;; return dec_idx
              (i32.store (i32.add (local.get $op_base) (i32.const 4)) (i32.load (global.get $OFF_EXEC_STACK_LEN)))
              (i32.store (i32.add (local.get $op_base) (i32.const 8)) (i32.load (global.get $OFF_EXEC_CTRL_LEN)))
              (i32.store (i32.add (local.get $op_base) (i32.const 12)) (local.get $local_count))
              ;; Save locals
              (local.set $i (i32.const 0))
              (block $save_locals
                (loop $save_cont
                  (if (i32.ge_u (local.get $i) (local.get $local_count)) (then (br $save_locals)))
                  (i32.store
                    (i32.add (local.get $op_base) (i32.const 16) (i32.shl (local.get $i) (i32.const 2)))
                    (i32.load (i32.add (global.get $OFF_EXEC_LOCALS) (i32.shl (local.get $i) (i32.const 2))))
                  )
                  (local.set $i (i32.add (local.get $i) (i32.const 1)))
                  (br $save_cont)
                )
              )
              ;; Increment call depth
              (i32.store (global.get $OFF_EXEC_CALL_DEPTH) (i32.add (local.get $p) (i32.const 1)))
              ;; Execute target function (recursive call to $exec_fn)
              (local.set $err (call $exec_fn (local.get $imm0) (global.get $OFF_CALL_ARGS) (local.get $val)))
              (if (local.get $err) (then (return (local.get $err))))
              ;; Restore call depth
              (i32.store (global.get $OFF_EXEC_CALL_DEPTH) (local.get $p))
              ;; Restore caller state from frame save
              (local.set $dec_idx (i32.load (local.get $op_base)))  ;; return dec_idx
              (i32.store (global.get $OFF_EXEC_STACK_LEN) (i32.load (i32.add (local.get $op_base) (i32.const 4))))
              (i32.store (global.get $OFF_EXEC_CTRL_LEN) (i32.load (i32.add (local.get $op_base) (i32.const 8))))
              (local.set $local_count (i32.load (i32.add (local.get $op_base) (i32.const 12))))
              ;; Reload cached stack length after restore
              (global.set $FAST_STACK_LEN (i32.load (global.get $OFF_EXEC_STACK_LEN)))
              ;; Restore locals
              (local.set $i (i32.const 0))
              (block $rest_locals
                (loop $rest_cont
                  (if (i32.ge_u (local.get $i) (local.get $local_count)) (then (br $rest_locals)))
                  (i32.store
                    (i32.add (global.get $OFF_EXEC_LOCALS) (i32.shl (local.get $i) (i32.const 2)))
                    (i32.load (i32.add (local.get $op_base) (i32.const 16) (i32.shl (local.get $i) (i32.const 2))))
                  )
                  (local.set $i (i32.add (local.get $i) (i32.const 1)))
                  (br $rest_cont)
                )
              )
              ;; Push results from exec_fn onto value stack
              (local.set $i (i32.const 0))
              (block $call_res_lp
                (loop $call_res_cont
                  (if (i32.ge_u (local.get $i) (i32.load (global.get $OFF_EXEC_RES_COUNT)))
                    (then (br $call_res_lp))
                  )
                  (local.set $err (call $stack_push
                    (i32.wrap_i64 (i64.load (i32.add (global.get $OFF_EXEC_RESULT) (i32.shl (local.get $i) (i32.const 3)))))
                  ))
                  (if (local.get $err) (then (return (local.get $err))))
                  (local.set $i (i32.add (local.get $i) (i32.const 1)))
                  (br $call_res_cont)
                )
              )
              (br $dispatch_loop)
          )  ;; close $h17
              ;; imm0 = expected type_idx, imm1 = table_idx (must be 0)
              (if (i32.ne (local.get $imm1) (i32.const 0)) (then (return (global.get $ERR_UNSUP))))
              ;; Check table exists
              (if (i32.eqz (i32.load8_u (global.get $OFF_TABLE_HAS))) (then (return (global.get $ERR_TRAP))))

              ;; Pop selector (i32) from value stack
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $val (i32.load (global.get $OFF_SCRATCH0)))

              ;; Check selector in table bounds
              (if (i32.ge_u (local.get $val) (i32.load (global.get $OFF_TABLE_MIN)))
                (then (return (global.get $ERR_TRAP)))
              )

              ;; Read function index from table[selector]
              (local.set $type_idx (i32.load (i32.add (global.get $OFF_TABLE_DATA) (i32.shl (local.get $val) (i32.const 2)))))
              (if (i32.eq (local.get $type_idx) (i32.const -1)) (then (return (global.get $ERR_TRAP))))

              ;; Verify the function type matches expected type
              ;; funcs_buf holds type_idx for each function. But functions include imports,
              ;; so we need to get type_idx for the actual function at func_idx.
              (if (i32.lt_u (local.get $type_idx) (i32.load (global.get $OFF_IMPORT_COUNT)))
                (then (return (global.get $ERR_MISS_IMP)))
              )
              (local.set $p (i32.load
                (i32.add (global.get $OFF_FUNCTIONS_BUF)
                  (i32.mul (i32.sub (local.get $type_idx) (i32.load (global.get $OFF_IMPORT_COUNT))) (global.get $SZ_FUNC))
                )
              ))
              ;; $p = actual type_idx of the target function
              (if (i32.ne (local.get $p) (local.get $imm0)) (then (return (global.get $ERR_TRAP))))

              ;; Look up param count from expected type
              (local.set $val (i32.add (global.get $OFF_TYPES_BUF) (i32.mul (local.get $imm0) (global.get $SZ_TYPE))))
              (local.set $val (i32.load16_u (i32.add (local.get $val) (i32.const 128))))

              ;; Pop params into call args buffer
              (local.set $i (i32.sub (local.get $val) (i32.const 1)))
              (block $ci_pop_params
                (loop $ci_pop_cont
                  (if (i32.lt_s (local.get $i) (i32.const 0)) (then (br $ci_pop_params)))
                  (local.set $err (call $stack_pop))
                  (if (local.get $err) (then (return (local.get $err))))
                  (i32.store
                    (i32.add (global.get $OFF_CALL_ARGS) (i32.shl (local.get $i) (i32.const 2)))
                    (i32.load (global.get $OFF_SCRATCH0))
                  )
                  (local.set $i (i32.sub (local.get $i) (i32.const 1)))
                  (br $ci_pop_cont)
                )
              )

              ;; Save current state to frame save area
              (local.set $p (i32.load (global.get $OFF_EXEC_CALL_DEPTH)))
              (local.set $op_base (i32.add (global.get $OFF_FRAME_SAVE) (i32.mul (local.get $p) (i32.const 272))))
              (i32.store (local.get $op_base) (i32.add (local.get $dec_idx) (i32.const 1)))
              (i32.store (i32.add (local.get $op_base) (i32.const 4)) (i32.load (global.get $OFF_EXEC_STACK_LEN)))
              (i32.store (i32.add (local.get $op_base) (i32.const 8)) (i32.load (global.get $OFF_EXEC_CTRL_LEN)))
              (i32.store (i32.add (local.get $op_base) (i32.const 12)) (local.get $local_count))
              ;; Save locals
              (local.set $i (i32.const 0))
              (block $ci_save_locals
                (loop $ci_save_cont
                  (if (i32.ge_u (local.get $i) (local.get $local_count)) (then (br $ci_save_locals)))
                  (i32.store
                    (i32.add (local.get $op_base) (i32.const 16) (i32.shl (local.get $i) (i32.const 2)))
                    (i32.load (i32.add (global.get $OFF_EXEC_LOCALS) (i32.shl (local.get $i) (i32.const 2))))
                  )
                  (local.set $i (i32.add (local.get $i) (i32.const 1)))
                  (br $ci_save_cont)
                )
              )
              ;; Increment call depth and call target function
              (i32.store (global.get $OFF_EXEC_CALL_DEPTH) (i32.add (local.get $p) (i32.const 1)))
              (local.set $err (call $exec_fn (local.get $type_idx) (global.get $OFF_CALL_ARGS) (local.get $val)))
              (if (local.get $err) (then (return (local.get $err))))
              ;; Restore call depth
              (i32.store (global.get $OFF_EXEC_CALL_DEPTH) (local.get $p))
              ;; Restore caller state from frame save
              (local.set $dec_idx (i32.load (local.get $op_base)))
              (i32.store (global.get $OFF_EXEC_STACK_LEN) (i32.load (i32.add (local.get $op_base) (i32.const 4))))
              (i32.store (global.get $OFF_EXEC_CTRL_LEN) (i32.load (i32.add (local.get $op_base) (i32.const 8))))
              (local.set $local_count (i32.load (i32.add (local.get $op_base) (i32.const 12))))
              ;; Restore locals
              (local.set $i (i32.const 0))
              (block $ci_rest_locals
                (loop $ci_rest_cont
                  (if (i32.ge_u (local.get $i) (local.get $local_count)) (then (br $ci_rest_locals)))
                  (i32.store
                    (i32.add (global.get $OFF_EXEC_LOCALS) (i32.shl (local.get $i) (i32.const 2)))
                    (i32.load (i32.add (local.get $op_base) (i32.const 16) (i32.shl (local.get $i) (i32.const 2))))
                  )
                  (local.set $i (i32.add (local.get $i) (i32.const 1)))
                  (br $ci_rest_cont)
                )
              )
              ;; Push results onto value stack
              (local.set $i (i32.const 0))
              (block $ci_res_lp
                (loop $ci_res_cont
                  (if (i32.ge_u (local.get $i) (i32.load (global.get $OFF_EXEC_RES_COUNT)))
                    (then (br $ci_res_lp))
                  )
                  (local.set $err (call $stack_push
                    (i32.wrap_i64 (i64.load (i32.add (global.get $OFF_EXEC_RESULT) (i32.shl (local.get $i) (i32.const 3)))))
                  ))
                  (if (local.get $err) (then (return (local.get $err))))
                  (local.set $i (i32.add (local.get $i) (i32.const 1)))
                  (br $ci_res_cont)
                )
              )
              (br $dispatch_loop)
          )  ;; close $h18
        (return (global.get $ERR_UNSUP))
          )  ;; close $h19
        (return (global.get $ERR_UNSUP))
          )  ;; close $h20
        (return (global.get $ERR_UNSUP))
          )  ;; close $h21
        (return (global.get $ERR_UNSUP))
          )  ;; close $h22
        (return (global.get $ERR_UNSUP))
          )  ;; close $h23
        (return (global.get $ERR_UNSUP))
          )  ;; close $h24
        (return (global.get $ERR_UNSUP))
          )  ;; close $h25
        (return (global.get $ERR_UNSUP))
          )  ;; close $h26
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h27
              ;; Pop condition (i32), then val2, then val1
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $imm0 (i32.load (global.get $OFF_SCRATCH0)))  ;; condition
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $imm1 (i32.load (global.get $OFF_SCRATCH0)))  ;; val2
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $imm2 (i32.load (global.get $OFF_SCRATCH0)))  ;; val1
              (local.set $err (call $stack_push
                (if (result i32) (i32.eqz (local.get $imm0)) (then (local.get $imm1)) (else (local.get $imm2)))
              ))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h28
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $imm0 (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $imm1 (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $imm2 (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $err (call $stack_push
                (if (result i32) (i32.eqz (local.get $imm0)) (then (local.get $imm1)) (else (local.get $imm2)))
              ))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h29
        (return (global.get $ERR_UNSUP))
          )  ;; close $h30
        (return (global.get $ERR_UNSUP))
          )  ;; close $h31
        (return (global.get $ERR_UNSUP))
          )  ;; close $h32
              (local.set $err (call $stack_push
                (i32.load
                  (i32.add (global.get $OFF_EXEC_LOCALS) (i32.shl (local.get $imm0) (i32.const 2)))
                )
              ))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h33
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (i32.store
                (i32.add (global.get $OFF_EXEC_LOCALS) (i32.shl (local.get $imm0) (i32.const 2)))
                (i32.load (global.get $OFF_SCRATCH0))
              )
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h34
              (local.set $err (call $stack_peek))
              (if (local.get $err) (then (return (local.get $err))))
              (i32.store
                (i32.add (global.get $OFF_EXEC_LOCALS) (i32.shl (local.get $imm0) (i32.const 2)))
                (i32.load (global.get $OFF_SCRATCH0))
              )
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h35
              (local.set $err (call $stack_push
                (i32.load (i32.add (global.get $OFF_GLOBALS_BUF) (i32.shl (local.get $imm0) (i32.const 2))))
              ))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h36
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (i32.store
                (i32.add (global.get $OFF_GLOBALS_BUF) (i32.shl (local.get $imm0) (i32.const 2)))
                (i32.load (global.get $OFF_SCRATCH0))
              )
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h37
              (if (i32.eqz (i32.load8_u (global.get $OFF_TABLE_HAS))) (then (return (global.get $ERR_TRAP))))
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $val (i32.load (global.get $OFF_SCRATCH0)))
              (if (i32.ge_u (local.get $val) (i32.load (global.get $OFF_TABLE_MIN)))
                (then (return (global.get $ERR_TRAP)))
              )
              (local.set $err (call $stack_push
                (i32.load (i32.add (global.get $OFF_TABLE_DATA) (i32.shl (local.get $val) (i32.const 2))))
              ))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h38
              (if (i32.eqz (i32.load8_u (global.get $OFF_TABLE_HAS))) (then (return (global.get $ERR_TRAP))))
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $val (i32.load (global.get $OFF_SCRATCH0)))   ;; value
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $p (i32.load (global.get $OFF_SCRATCH0)))     ;; index
              (if (i32.ge_u (local.get $p) (i32.load (global.get $OFF_TABLE_MIN)))
                (then (return (global.get $ERR_TRAP)))
              )
              (i32.store
                (i32.add (global.get $OFF_TABLE_DATA) (i32.shl (local.get $p) (i32.const 2)))
                (local.get $val)
              )
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h39
        (return (global.get $ERR_UNSUP))
          )  ;; close $h40
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $val (i32.load (global.get $OFF_SCRATCH0)))
              ;; imm1 = static offset; addr = val + imm1 + guest_mem_base
              (local.set $err (call $stack_push
                (i32.load (i32.add (i32.add (local.get $val) (local.get $imm1)) (global.get $OFF_GUEST_MEM_BASE)))
              ))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h41
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $val (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $tmp64 (i64.load (i32.add (i32.add (local.get $val) (local.get $imm1)) (global.get $OFF_GUEST_MEM_BASE))))
              (local.set $err (call $stack_push_i64
                (i32.wrap_i64 (local.get $tmp64))
                (i32.wrap_i64 (i64.shr_u (local.get $tmp64) (i64.const 32)))
              ))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h42
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $val (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $err (call $stack_push
                (i32.load (i32.add (i32.add (local.get $val) (local.get $imm1)) (global.get $OFF_GUEST_MEM_BASE)))
              ))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h43
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $val (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $tmp64 (i64.load (i32.add (i32.add (local.get $val) (local.get $imm1)) (global.get $OFF_GUEST_MEM_BASE))))
              (local.set $err (call $stack_push_i64
                (i32.wrap_i64 (local.get $tmp64))
                (i32.wrap_i64 (i64.shr_u (local.get $tmp64) (i64.const 32)))
              ))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h44
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $val (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $err (call $stack_push
                (i32.load8_s (i32.add (i32.add (local.get $val) (local.get $imm1)) (global.get $OFF_GUEST_MEM_BASE)))
              ))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h45
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $val (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $err (call $stack_push
                (i32.load8_u (i32.add (i32.add (local.get $val) (local.get $imm1)) (global.get $OFF_GUEST_MEM_BASE)))
              ))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h46
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $val (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $err (call $stack_push
                (i32.load16_s (i32.add (i32.add (local.get $val) (local.get $imm1)) (global.get $OFF_GUEST_MEM_BASE)))
              ))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h47
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $val (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $err (call $stack_push
                (i32.load16_u (i32.add (i32.add (local.get $val) (local.get $imm1)) (global.get $OFF_GUEST_MEM_BASE)))
              ))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h48
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $val (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $tmp64 (i64.extend_i32_s (i32.load8_s (i32.add (i32.add (local.get $val) (local.get $imm1)) (global.get $OFF_GUEST_MEM_BASE)))))
              (local.set $err (call $stack_push_i64
                (i32.wrap_i64 (local.get $tmp64))
                (i32.wrap_i64 (i64.shr_u (local.get $tmp64) (i64.const 32)))
              ))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h49
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $val (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $tmp64 (i64.extend_i32_u (i32.load8_u (i32.add (i32.add (local.get $val) (local.get $imm1)) (global.get $OFF_GUEST_MEM_BASE)))))
              (local.set $err (call $stack_push_i64
                (i32.wrap_i64 (local.get $tmp64))
                (i32.wrap_i64 (i64.shr_u (local.get $tmp64) (i64.const 32)))
              ))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h50
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $val (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $tmp64 (i64.extend_i32_s (i32.load16_s (i32.add (i32.add (local.get $val) (local.get $imm1)) (global.get $OFF_GUEST_MEM_BASE)))))
              (local.set $err (call $stack_push_i64
                (i32.wrap_i64 (local.get $tmp64))
                (i32.wrap_i64 (i64.shr_u (local.get $tmp64) (i64.const 32)))
              ))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h51
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $val (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $tmp64 (i64.extend_i32_u (i32.load16_u (i32.add (i32.add (local.get $val) (local.get $imm1)) (global.get $OFF_GUEST_MEM_BASE)))))
              (local.set $err (call $stack_push_i64
                (i32.wrap_i64 (local.get $tmp64))
                (i32.wrap_i64 (i64.shr_u (local.get $tmp64) (i64.const 32)))
              ))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h52
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $val (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $tmp64 (i64.extend_i32_s (i32.load (i32.add (i32.add (local.get $val) (local.get $imm1)) (global.get $OFF_GUEST_MEM_BASE)))))
              (local.set $err (call $stack_push_i64
                (i32.wrap_i64 (local.get $tmp64))
                (i32.wrap_i64 (i64.shr_u (local.get $tmp64) (i64.const 32)))
              ))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h53
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $val (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $tmp64 (i64.extend_i32_u (i32.load (i32.add (i32.add (local.get $val) (local.get $imm1)) (global.get $OFF_GUEST_MEM_BASE)))))
              (local.set $err (call $stack_push_i64
                (i32.wrap_i64 (local.get $tmp64))
                (i32.wrap_i64 (i64.shr_u (local.get $tmp64) (i64.const 32)))
              ))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h54
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $val (i32.load (global.get $OFF_SCRATCH0)))   ;; value to store
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $p (i32.load (global.get $OFF_SCRATCH0)))     ;; address
              ;; imm1 = static offset; addr = p + imm1 + guest_mem_base
              (i32.store
                (i32.add (i32.add (local.get $p) (local.get $imm1)) (global.get $OFF_GUEST_MEM_BASE))
                (local.get $val)
              )
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h55
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp64 (i64.or (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH0))) (i64.shl (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH1))) (i64.const 32))))
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $val (i32.load (global.get $OFF_SCRATCH0)))
              (i64.store
                (i32.add (i32.add (local.get $val) (local.get $imm1)) (global.get $OFF_GUEST_MEM_BASE))
                (local.get $tmp64)
              )
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h56
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $val (i32.load (global.get $OFF_SCRATCH0)))   ;; f32 bits to store
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $p (i32.load (global.get $OFF_SCRATCH0)))     ;; address
              (i32.store
                (i32.add (i32.add (local.get $p) (local.get $imm1)) (global.get $OFF_GUEST_MEM_BASE))
                (local.get $val)
              )
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h57
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp64 (i64.or (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH0))) (i64.shl (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH1))) (i64.const 32))))
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $val (i32.load (global.get $OFF_SCRATCH0)))
              (i64.store
                (i32.add (i32.add (local.get $val) (local.get $imm1)) (global.get $OFF_GUEST_MEM_BASE))
                (local.get $tmp64)
              )
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h58
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $val (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $p (i32.load (global.get $OFF_SCRATCH0)))
              (i32.store8
                (i32.add (i32.add (local.get $p) (local.get $imm1)) (global.get $OFF_GUEST_MEM_BASE))
                (local.get $val)
              )
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h59
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $val (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $p (i32.load (global.get $OFF_SCRATCH0)))
              (i32.store16
                (i32.add (i32.add (local.get $p) (local.get $imm1)) (global.get $OFF_GUEST_MEM_BASE))
                (local.get $val)
              )
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h60
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $imm2 (i32.load (global.get $OFF_SCRATCH0)))  ;; lo
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $val (i32.load (global.get $OFF_SCRATCH0)))
              (i32.store8
                (i32.add (i32.add (local.get $val) (local.get $imm1)) (global.get $OFF_GUEST_MEM_BASE))
                (local.get $imm2)
              )
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h61
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $imm2 (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $val (i32.load (global.get $OFF_SCRATCH0)))
              (i32.store16
                (i32.add (i32.add (local.get $val) (local.get $imm1)) (global.get $OFF_GUEST_MEM_BASE))
                (local.get $imm2)
              )
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h62
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $imm2 (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $val (i32.load (global.get $OFF_SCRATCH0)))
              (i32.store
                (i32.add (i32.add (local.get $val) (local.get $imm1)) (global.get $OFF_GUEST_MEM_BASE))
                (local.get $imm2)
              )
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h63
              (local.set $err (call $stack_push (i32.load (global.get $OFF_GUEST_MEM_PAGES))))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h64
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $val (i32.load (global.get $OFF_SCRATCH0)))  ;; requested pages
              (local.set $imm0 (i32.load (global.get $OFF_GUEST_MEM_PAGES)))  ;; current pages
              ;; Compute max available: host_total_pages - guest_base_pages
              (local.set $imm1 (memory.size))  ;; total host pages
              (local.set $imm1 (i32.sub (local.get $imm1) (i32.const 32)))  ;; minus 32 pages for guest base (0x200000)
              (if (i32.le_u (i32.add (local.get $imm0) (local.get $val)) (local.get $imm1))
                (then
                  ;; Growth fits: update current pages, return old size
                  (i32.store (global.get $OFF_GUEST_MEM_PAGES) (i32.add (local.get $imm0) (local.get $val)))
                  (local.set $err (call $stack_push (local.get $imm0)))
                )
                (else
                  ;; Growth doesn't fit: return -1
                  (local.set $err (call $stack_push (i32.const -1)))
                )
              )
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h65
              (local.set $err (call $stack_push (local.get $imm0)))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h66
              (local.set $tmp64 (i64.load (i32.add (local.get $op_base) (i32.const 4))))
              (local.set $err (call $stack_push_i64
                (i32.wrap_i64 (local.get $tmp64))
                (i32.wrap_i64 (i64.shr_u (local.get $tmp64) (i64.const 32)))
              ))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h67
              (local.set $err (call $stack_push
                (i32.load (i32.add (local.get $op_base) (i32.const 4)))
              ))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h68
              (local.set $tmp64 (i64.load (i32.add (local.get $op_base) (i32.const 4))))
              (local.set $err (call $stack_push_i64
                (i32.wrap_i64 (local.get $tmp64))
                (i32.wrap_i64 (i64.shr_u (local.get $tmp64) (i64.const 32)))
              ))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h69
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $val (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $err (call $stack_push
                (if (result i32) (i32.eqz (local.get $val)) (then (i32.const 1)) (else (i32.const 0)))
              ))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h70
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $imm1 (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $imm0 (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $err (call $stack_push
                (if (result i32) (i32.eq (local.get $imm0) (local.get $imm1)) (then (i32.const 1)) (else (i32.const 0)))
              ))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h71
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $imm1 (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $imm0 (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $err (call $stack_push
                (if (result i32) (i32.ne (local.get $imm0) (local.get $imm1)) (then (i32.const 1)) (else (i32.const 0)))
              ))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h72
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $imm1 (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $imm0 (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $err (call $stack_push
                (if (result i32) (i32.lt_s (local.get $imm0) (local.get $imm1)) (then (i32.const 1)) (else (i32.const 0)))
              ))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h73
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $imm1 (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $imm0 (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $err (call $stack_push
                (if (result i32) (i32.lt_u (local.get $imm0) (local.get $imm1)) (then (i32.const 1)) (else (i32.const 0)))
              ))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h74
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $imm1 (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $imm0 (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $err (call $stack_push
                (if (result i32) (i32.gt_s (local.get $imm0) (local.get $imm1)) (then (i32.const 1)) (else (i32.const 0)))
              ))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h75
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $imm1 (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $imm0 (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $err (call $stack_push
                (if (result i32) (i32.gt_u (local.get $imm0) (local.get $imm1)) (then (i32.const 1)) (else (i32.const 0)))
              ))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h76
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $imm1 (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $imm0 (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $err (call $stack_push
                (if (result i32) (i32.le_s (local.get $imm0) (local.get $imm1)) (then (i32.const 1)) (else (i32.const 0)))
              ))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h77
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $imm1 (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $imm0 (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $err (call $stack_push
                (if (result i32) (i32.le_u (local.get $imm0) (local.get $imm1)) (then (i32.const 1)) (else (i32.const 0)))
              ))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h78
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $imm1 (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $imm0 (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $err (call $stack_push
                (if (result i32) (i32.ge_s (local.get $imm0) (local.get $imm1)) (then (i32.const 1)) (else (i32.const 0)))
              ))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h79
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $imm1 (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $imm0 (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $err (call $stack_push
                (if (result i32) (i32.ge_u (local.get $imm0) (local.get $imm1)) (then (i32.const 1)) (else (i32.const 0)))
              ))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h80
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp64 (i64.or (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH0))) (i64.shl (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH1))) (i64.const 32))))
              (local.set $err (call $stack_push (if (result i32) (i64.eqz (local.get $tmp64)) (then (i32.const 1)) (else (i32.const 0)))))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h81
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp64 (i64.or (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH0))) (i64.shl (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH1))) (i64.const 32))))
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp64b (i64.or (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH0))) (i64.shl (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH1))) (i64.const 32))))
              (local.set $err (call $stack_push
                (if (result i32) (i64.eq (local.get $tmp64) (local.get $tmp64b)) (then (i32.const 1)) (else (i32.const 0)))
              ))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h82
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp64 (i64.or (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH0))) (i64.shl (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH1))) (i64.const 32))))
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp64b (i64.or (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH0))) (i64.shl (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH1))) (i64.const 32))))
              (local.set $err (call $stack_push
                (if (result i32) (i64.ne (local.get $tmp64) (local.get $tmp64b)) (then (i32.const 1)) (else (i32.const 0)))
              ))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h83
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp64 (i64.or (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH0))) (i64.shl (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH1))) (i64.const 32))))
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp64b (i64.or (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH0))) (i64.shl (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH1))) (i64.const 32))))
              (local.set $err (call $stack_push
                (if (result i32) (i64.lt_s (local.get $tmp64b) (local.get $tmp64)) (then (i32.const 1)) (else (i32.const 0)))
              ))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h84
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp64 (i64.or (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH0))) (i64.shl (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH1))) (i64.const 32))))
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp64b (i64.or (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH0))) (i64.shl (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH1))) (i64.const 32))))
              (local.set $err (call $stack_push
                (if (result i32) (i64.lt_u (local.get $tmp64b) (local.get $tmp64)) (then (i32.const 1)) (else (i32.const 0)))
              ))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h85
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp64 (i64.or (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH0))) (i64.shl (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH1))) (i64.const 32))))
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp64b (i64.or (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH0))) (i64.shl (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH1))) (i64.const 32))))
              (local.set $err (call $stack_push
                (if (result i32) (i64.gt_s (local.get $tmp64b) (local.get $tmp64)) (then (i32.const 1)) (else (i32.const 0)))
              ))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h86
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp64 (i64.or (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH0))) (i64.shl (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH1))) (i64.const 32))))
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp64b (i64.or (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH0))) (i64.shl (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH1))) (i64.const 32))))
              (local.set $err (call $stack_push
                (if (result i32) (i64.gt_u (local.get $tmp64b) (local.get $tmp64)) (then (i32.const 1)) (else (i32.const 0)))
              ))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h87
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp64 (i64.or (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH0))) (i64.shl (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH1))) (i64.const 32))))
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp64b (i64.or (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH0))) (i64.shl (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH1))) (i64.const 32))))
              (local.set $err (call $stack_push
                (if (result i32) (i64.le_s (local.get $tmp64b) (local.get $tmp64)) (then (i32.const 1)) (else (i32.const 0)))
              ))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h88
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp64 (i64.or (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH0))) (i64.shl (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH1))) (i64.const 32))))
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp64b (i64.or (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH0))) (i64.shl (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH1))) (i64.const 32))))
              (local.set $err (call $stack_push
                (if (result i32) (i64.le_u (local.get $tmp64b) (local.get $tmp64)) (then (i32.const 1)) (else (i32.const 0)))
              ))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h89
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp64 (i64.or (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH0))) (i64.shl (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH1))) (i64.const 32))))
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp64b (i64.or (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH0))) (i64.shl (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH1))) (i64.const 32))))
              (local.set $err (call $stack_push
                (if (result i32) (i64.ge_s (local.get $tmp64b) (local.get $tmp64)) (then (i32.const 1)) (else (i32.const 0)))
              ))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h90
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp64 (i64.or (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH0))) (i64.shl (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH1))) (i64.const 32))))
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp64b (i64.or (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH0))) (i64.shl (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH1))) (i64.const 32))))
              (local.set $err (call $stack_push
                (if (result i32) (i64.ge_u (local.get $tmp64b) (local.get $tmp64)) (then (i32.const 1)) (else (i32.const 0)))
              ))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h91
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (i32.store (global.get $OFF_SCRATCH1) (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $err (call $stack_push
                (if (result i32)
                  (f32.eq (f32.reinterpret_i32 (i32.load (global.get $OFF_SCRATCH0))) (f32.reinterpret_i32 (i32.load (global.get $OFF_SCRATCH1))))
                  (then (i32.const 1))
                  (else (i32.const 0))
                )
              ))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h92
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (i32.store (global.get $OFF_SCRATCH1) (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $err (call $stack_push
                (if (result i32)
                  (f32.ne (f32.reinterpret_i32 (i32.load (global.get $OFF_SCRATCH0))) (f32.reinterpret_i32 (i32.load (global.get $OFF_SCRATCH1))))
                  (then (i32.const 1))
                  (else (i32.const 0))
                )
              ))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h93
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (i32.store (global.get $OFF_SCRATCH1) (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $err (call $stack_push
                (if (result i32)
                  (f32.lt (f32.reinterpret_i32 (i32.load (global.get $OFF_SCRATCH0))) (f32.reinterpret_i32 (i32.load (global.get $OFF_SCRATCH1))))
                  (then (i32.const 1))
                  (else (i32.const 0))
                )
              ))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h94
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (i32.store (global.get $OFF_SCRATCH1) (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $err (call $stack_push
                (if (result i32)
                  (f32.gt (f32.reinterpret_i32 (i32.load (global.get $OFF_SCRATCH0))) (f32.reinterpret_i32 (i32.load (global.get $OFF_SCRATCH1))))
                  (then (i32.const 1))
                  (else (i32.const 0))
                )
              ))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h95
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (i32.store (global.get $OFF_SCRATCH1) (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $err (call $stack_push
                (if (result i32)
                  (f32.le (f32.reinterpret_i32 (i32.load (global.get $OFF_SCRATCH0))) (f32.reinterpret_i32 (i32.load (global.get $OFF_SCRATCH1))))
                  (then (i32.const 1))
                  (else (i32.const 0))
                )
              ))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h96
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (i32.store (global.get $OFF_SCRATCH1) (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $err (call $stack_push
                (if (result i32)
                  (f32.ge (f32.reinterpret_i32 (i32.load (global.get $OFF_SCRATCH0))) (f32.reinterpret_i32 (i32.load (global.get $OFF_SCRATCH1))))
                  (then (i32.const 1))
                  (else (i32.const 0))
                )
              ))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h97
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp64b (i64.or (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH0))) (i64.shl (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH1))) (i64.const 32))))
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp64 (i64.or (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH0))) (i64.shl (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH1))) (i64.const 32))))
              (local.set $err (call $stack_push
                (if (result i32) (f64.eq (f64.reinterpret_i64 (local.get $tmp64b)) (f64.reinterpret_i64 (local.get $tmp64))) (then (i32.const 1)) (else (i32.const 0)))
              ))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h98
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp64b (i64.or (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH0))) (i64.shl (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH1))) (i64.const 32))))
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp64 (i64.or (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH0))) (i64.shl (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH1))) (i64.const 32))))
              (local.set $err (call $stack_push
                (if (result i32) (f64.ne (f64.reinterpret_i64 (local.get $tmp64b)) (f64.reinterpret_i64 (local.get $tmp64))) (then (i32.const 1)) (else (i32.const 0)))
              ))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h99
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp64b (i64.or (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH0))) (i64.shl (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH1))) (i64.const 32))))
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp64 (i64.or (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH0))) (i64.shl (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH1))) (i64.const 32))))
              (local.set $err (call $stack_push
                (if (result i32) (f64.lt (f64.reinterpret_i64 (local.get $tmp64b)) (f64.reinterpret_i64 (local.get $tmp64))) (then (i32.const 1)) (else (i32.const 0)))
              ))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h100
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp64b (i64.or (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH0))) (i64.shl (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH1))) (i64.const 32))))
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp64 (i64.or (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH0))) (i64.shl (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH1))) (i64.const 32))))
              (local.set $err (call $stack_push
                (if (result i32) (f64.gt (f64.reinterpret_i64 (local.get $tmp64b)) (f64.reinterpret_i64 (local.get $tmp64))) (then (i32.const 1)) (else (i32.const 0)))
              ))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h101
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp64b (i64.or (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH0))) (i64.shl (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH1))) (i64.const 32))))
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp64 (i64.or (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH0))) (i64.shl (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH1))) (i64.const 32))))
              (local.set $err (call $stack_push
                (if (result i32) (f64.le (f64.reinterpret_i64 (local.get $tmp64b)) (f64.reinterpret_i64 (local.get $tmp64))) (then (i32.const 1)) (else (i32.const 0)))
              ))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h102
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp64b (i64.or (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH0))) (i64.shl (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH1))) (i64.const 32))))
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp64 (i64.or (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH0))) (i64.shl (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH1))) (i64.const 32))))
              (local.set $err (call $stack_push
                (if (result i32) (f64.ge (f64.reinterpret_i64 (local.get $tmp64b)) (f64.reinterpret_i64 (local.get $tmp64))) (then (i32.const 1)) (else (i32.const 0)))
              ))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h103
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $val (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $err (call $stack_push (i32.clz (local.get $val))))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h104
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $val (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $err (call $stack_push (i32.ctz (local.get $val))))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h105
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $val (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $err (call $stack_push (i32.popcnt (local.get $val))))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h106
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $imm1 (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $imm0 (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $err (call $stack_push (i32.add (local.get $imm0) (local.get $imm1))))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h107
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $imm1 (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $imm0 (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $err (call $stack_push (i32.sub (local.get $imm0) (local.get $imm1))))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h108
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $imm1 (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $imm0 (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $err (call $stack_push (i32.mul (local.get $imm0) (local.get $imm1))))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h109
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $imm1 (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $imm0 (i32.load (global.get $OFF_SCRATCH0)))
              (if (i32.eqz (local.get $imm1))
                (then (return (global.get $ERR_TRAP)))  ;; div by zero
              )
              (if (i32.and (i32.eq (local.get $imm0) (i32.const 0x80000000)) (i32.eq (local.get $imm1) (i32.const -1)))
                (then (return (global.get $ERR_TRAP)))  ;; INT_MIN / -1
              )
              (local.set $err (call $stack_push (i32.div_s (local.get $imm0) (local.get $imm1))))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h110
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $imm1 (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $imm0 (i32.load (global.get $OFF_SCRATCH0)))
              (if (i32.eqz (local.get $imm1))
                (then (return (global.get $ERR_TRAP)))  ;; div by zero
              )
              (local.set $err (call $stack_push (i32.div_u (local.get $imm0) (local.get $imm1))))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h111
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $imm1 (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $imm0 (i32.load (global.get $OFF_SCRATCH0)))
              (if (i32.eqz (local.get $imm1))
                (then (return (global.get $ERR_TRAP)))  ;; rem by zero
              )
              (local.set $err (call $stack_push (i32.rem_s (local.get $imm0) (local.get $imm1))))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h112
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $imm1 (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $imm0 (i32.load (global.get $OFF_SCRATCH0)))
              (if (i32.eqz (local.get $imm1))
                (then (return (global.get $ERR_TRAP)))  ;; rem by zero
              )
              (local.set $err (call $stack_push (i32.rem_u (local.get $imm0) (local.get $imm1))))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h113
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $imm1 (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $imm0 (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $err (call $stack_push (i32.and (local.get $imm0) (local.get $imm1))))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h114
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $imm1 (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $imm0 (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $err (call $stack_push (i32.or (local.get $imm0) (local.get $imm1))))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h115
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $imm1 (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $imm0 (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $err (call $stack_push (i32.xor (local.get $imm0) (local.get $imm1))))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h116
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $imm1 (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $imm0 (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $err (call $stack_push (i32.shl (local.get $imm0) (i32.and (local.get $imm1) (i32.const 31)))))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h117
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $imm1 (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $imm0 (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $err (call $stack_push (i32.shr_s (local.get $imm0) (i32.and (local.get $imm1) (i32.const 31)))))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h118
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $imm1 (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $imm0 (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $err (call $stack_push (i32.shr_u (local.get $imm0) (i32.and (local.get $imm1) (i32.const 31)))))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h119
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $imm1 (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $imm0 (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $err (call $stack_push (i32.rotl (local.get $imm0) (i32.and (local.get $imm1) (i32.const 31)))))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h120
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $imm1 (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $imm0 (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $err (call $stack_push (i32.rotr (local.get $imm0) (i32.and (local.get $imm1) (i32.const 31)))))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h121
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp64 (i64.or (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH0))) (i64.shl (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH1))) (i64.const 32))))
              (local.set $tmp64 (i64.clz (local.get $tmp64)))
              (local.set $err (call $stack_push_i64 (i32.wrap_i64 (local.get $tmp64)) (i32.wrap_i64 (i64.shr_u (local.get $tmp64) (i64.const 32)))))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h122
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp64 (i64.or (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH0))) (i64.shl (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH1))) (i64.const 32))))
              (local.set $tmp64 (i64.ctz (local.get $tmp64)))
              (local.set $err (call $stack_push_i64 (i32.wrap_i64 (local.get $tmp64)) (i32.wrap_i64 (i64.shr_u (local.get $tmp64) (i64.const 32)))))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h123
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp64 (i64.or (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH0))) (i64.shl (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH1))) (i64.const 32))))
              (local.set $tmp64 (i64.popcnt (local.get $tmp64)))
              (local.set $err (call $stack_push_i64 (i32.wrap_i64 (local.get $tmp64)) (i32.wrap_i64 (i64.shr_u (local.get $tmp64) (i64.const 32)))))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h124
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp64 (i64.or (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH0))) (i64.shl (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH1))) (i64.const 32))))
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp64b (i64.or (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH0))) (i64.shl (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH1))) (i64.const 32))))
              (local.set $tmp64 (i64.add (local.get $tmp64) (local.get $tmp64b)))
              (local.set $err (call $stack_push_i64
                (i32.wrap_i64 (local.get $tmp64))
                (i32.wrap_i64 (i64.shr_u (local.get $tmp64) (i64.const 32)))
              ))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h125
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp64 (i64.or (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH0))) (i64.shl (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH1))) (i64.const 32))))
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp64b (i64.or (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH0))) (i64.shl (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH1))) (i64.const 32))))
              (local.set $tmp64 (i64.sub (local.get $tmp64b) (local.get $tmp64)))
              (local.set $err (call $stack_push_i64
                (i32.wrap_i64 (local.get $tmp64))
                (i32.wrap_i64 (i64.shr_u (local.get $tmp64) (i64.const 32)))
              ))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h126
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp64 (i64.or (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH0))) (i64.shl (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH1))) (i64.const 32))))
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp64b (i64.or (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH0))) (i64.shl (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH1))) (i64.const 32))))
              (local.set $tmp64 (i64.mul (local.get $tmp64) (local.get $tmp64b)))
              (local.set $err (call $stack_push_i64
                (i32.wrap_i64 (local.get $tmp64))
                (i32.wrap_i64 (i64.shr_u (local.get $tmp64) (i64.const 32)))
              ))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h127
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $imm2 (i32.load (global.get $OFF_SCRATCH0)))  ;; lo divisor
              (local.set $imm3 (i32.load (global.get $OFF_SCRATCH1)))  ;; hi divisor
              (if (i32.eqz (i32.or (local.get $imm2) (local.get $imm3)))
                (then (return (global.get $ERR_TRAP)))  ;; div by zero
              )
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp64 (i64.or (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH0))) (i64.shl (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH1))) (i64.const 32))))
              (if (i32.and (i32.eq (local.get $imm2) (i32.const 0xFFFFFFFF)) (i32.eq (local.get $imm3) (i32.const 0xFFFFFFFF)))
                (then
                  (if (i64.eq (local.get $tmp64) (i64.const -9223372036854775808))
                    (then (return (global.get $ERR_TRAP)))  ;; INT64_MIN / -1
                  )
                )
              )
              (local.set $tmp64b (i64.or (i64.extend_i32_u (local.get $imm2)) (i64.shl (i64.extend_i32_u (local.get $imm3)) (i64.const 32))))
              (local.set $tmp64 (i64.div_s (local.get $tmp64) (local.get $tmp64b)))
              (local.set $err (call $stack_push_i64
                (i32.wrap_i64 (local.get $tmp64))
                (i32.wrap_i64 (i64.shr_u (local.get $tmp64) (i64.const 32)))
              ))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h128
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $imm2 (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $imm3 (i32.load (global.get $OFF_SCRATCH1)))
              (if (i32.eqz (i32.or (local.get $imm2) (local.get $imm3)))
                (then (return (global.get $ERR_TRAP)))
              )
              (local.set $tmp64b (i64.or (i64.extend_i32_u (local.get $imm2)) (i64.shl (i64.extend_i32_u (local.get $imm3)) (i64.const 32))))
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp64 (i64.or (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH0))) (i64.shl (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH1))) (i64.const 32))))
              (local.set $tmp64 (i64.div_u (local.get $tmp64) (local.get $tmp64b)))
              (local.set $err (call $stack_push_i64
                (i32.wrap_i64 (local.get $tmp64))
                (i32.wrap_i64 (i64.shr_u (local.get $tmp64) (i64.const 32)))
              ))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h129
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $imm2 (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $imm3 (i32.load (global.get $OFF_SCRATCH1)))
              (if (i32.eqz (i32.or (local.get $imm2) (local.get $imm3)))
                (then (return (global.get $ERR_TRAP)))
              )
              (local.set $tmp64b (i64.or (i64.extend_i32_u (local.get $imm2)) (i64.shl (i64.extend_i32_u (local.get $imm3)) (i64.const 32))))
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp64 (i64.or (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH0))) (i64.shl (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH1))) (i64.const 32))))
              (local.set $tmp64 (i64.rem_s (local.get $tmp64) (local.get $tmp64b)))
              (local.set $err (call $stack_push_i64
                (i32.wrap_i64 (local.get $tmp64))
                (i32.wrap_i64 (i64.shr_u (local.get $tmp64) (i64.const 32)))
              ))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h130
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $imm2 (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $imm3 (i32.load (global.get $OFF_SCRATCH1)))
              (if (i32.eqz (i32.or (local.get $imm2) (local.get $imm3)))
                (then (return (global.get $ERR_TRAP)))
              )
              (local.set $tmp64b (i64.or (i64.extend_i32_u (local.get $imm2)) (i64.shl (i64.extend_i32_u (local.get $imm3)) (i64.const 32))))
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp64 (i64.or (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH0))) (i64.shl (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH1))) (i64.const 32))))
              (local.set $tmp64 (i64.rem_u (local.get $tmp64) (local.get $tmp64b)))
              (local.set $err (call $stack_push_i64
                (i32.wrap_i64 (local.get $tmp64))
                (i32.wrap_i64 (i64.shr_u (local.get $tmp64) (i64.const 32)))
              ))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h131
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $imm0 (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $imm1 (i32.load (global.get $OFF_SCRATCH1)))
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $imm2 (i32.and (local.get $imm0) (i32.load (global.get $OFF_SCRATCH0))))
              (local.set $imm3 (i32.and (local.get $imm1) (i32.load (global.get $OFF_SCRATCH1))))
              (local.set $err (call $stack_push_i64 (local.get $imm2) (local.get $imm3)))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h132
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $imm0 (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $imm1 (i32.load (global.get $OFF_SCRATCH1)))
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $imm2 (i32.or (local.get $imm0) (i32.load (global.get $OFF_SCRATCH0))))
              (local.set $imm3 (i32.or (local.get $imm1) (i32.load (global.get $OFF_SCRATCH1))))
              (local.set $err (call $stack_push_i64 (local.get $imm2) (local.get $imm3)))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h133
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $imm0 (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $imm1 (i32.load (global.get $OFF_SCRATCH1)))
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $imm2 (i32.xor (local.get $imm0) (i32.load (global.get $OFF_SCRATCH0))))
              (local.set $imm3 (i32.xor (local.get $imm1) (i32.load (global.get $OFF_SCRATCH1))))
              (local.set $err (call $stack_push_i64 (local.get $imm2) (local.get $imm3)))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h134
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $imm2 (i32.and (i32.load (global.get $OFF_SCRATCH0)) (i32.const 63)))  ;; shift amount (lo i32 & 63)
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp64 (i64.or (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH0))) (i64.shl (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH1))) (i64.const 32))))
              (local.set $tmp64 (i64.shl (local.get $tmp64) (i64.extend_i32_u (local.get $imm2))))
              (local.set $err (call $stack_push_i64
                (i32.wrap_i64 (local.get $tmp64))
                (i32.wrap_i64 (i64.shr_u (local.get $tmp64) (i64.const 32)))
              ))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h135
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $imm2 (i32.and (i32.load (global.get $OFF_SCRATCH0)) (i32.const 63)))
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp64 (i64.or (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH0))) (i64.shl (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH1))) (i64.const 32))))
              (local.set $tmp64 (i64.shr_s (local.get $tmp64) (i64.extend_i32_u (local.get $imm2))))
              (local.set $err (call $stack_push_i64
                (i32.wrap_i64 (local.get $tmp64))
                (i32.wrap_i64 (i64.shr_u (local.get $tmp64) (i64.const 32)))
              ))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h136
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $imm2 (i32.and (i32.load (global.get $OFF_SCRATCH0)) (i32.const 63)))
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp64 (i64.or (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH0))) (i64.shl (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH1))) (i64.const 32))))
              (local.set $tmp64 (i64.shr_u (local.get $tmp64) (i64.extend_i32_u (local.get $imm2))))
              (local.set $err (call $stack_push_i64
                (i32.wrap_i64 (local.get $tmp64))
                (i32.wrap_i64 (i64.shr_u (local.get $tmp64) (i64.const 32)))
              ))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h137
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $imm2 (i32.and (i32.load (global.get $OFF_SCRATCH0)) (i32.const 63)))
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp64 (i64.or (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH0))) (i64.shl (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH1))) (i64.const 32))))
              (local.set $tmp64 (i64.rotl (local.get $tmp64) (i64.extend_i32_u (local.get $imm2))))
              (local.set $err (call $stack_push_i64
                (i32.wrap_i64 (local.get $tmp64))
                (i32.wrap_i64 (i64.shr_u (local.get $tmp64) (i64.const 32)))
              ))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h138
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $imm2 (i32.and (i32.load (global.get $OFF_SCRATCH0)) (i32.const 63)))
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp64 (i64.or (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH0))) (i64.shl (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH1))) (i64.const 32))))
              (local.set $tmp64 (i64.rotr (local.get $tmp64) (i64.extend_i32_u (local.get $imm2))))
              (local.set $err (call $stack_push_i64
                (i32.wrap_i64 (local.get $tmp64))
                (i32.wrap_i64 (i64.shr_u (local.get $tmp64) (i64.const 32)))
              ))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h139
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp_f32 (f32.abs (f32.reinterpret_i32 (i32.load (global.get $OFF_SCRATCH0)))))
              (local.set $err (call $stack_push (i32.reinterpret_f32 (local.get $tmp_f32))))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h140
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp_f32 (f32.neg (f32.reinterpret_i32 (i32.load (global.get $OFF_SCRATCH0)))))
              (local.set $err (call $stack_push (i32.reinterpret_f32 (local.get $tmp_f32))))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h141
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp_f32 (f32.ceil (f32.reinterpret_i32 (i32.load (global.get $OFF_SCRATCH0)))))
              (local.set $err (call $stack_push (i32.reinterpret_f32 (local.get $tmp_f32))))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h142
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp_f32 (f32.floor (f32.reinterpret_i32 (i32.load (global.get $OFF_SCRATCH0)))))
              (local.set $err (call $stack_push (i32.reinterpret_f32 (local.get $tmp_f32))))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h143
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp_f32 (f32.trunc (f32.reinterpret_i32 (i32.load (global.get $OFF_SCRATCH0)))))
              (local.set $err (call $stack_push (i32.reinterpret_f32 (local.get $tmp_f32))))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h144
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp_f32 (f32.nearest (f32.reinterpret_i32 (i32.load (global.get $OFF_SCRATCH0)))))
              (local.set $err (call $stack_push (i32.reinterpret_f32 (local.get $tmp_f32))))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h145
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp_f32 (f32.sqrt (f32.reinterpret_i32 (i32.load (global.get $OFF_SCRATCH0)))))
              (local.set $err (call $stack_push (i32.reinterpret_f32 (local.get $tmp_f32))))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h146
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (i32.store (global.get $OFF_SCRATCH1) (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $err (call $stack_push
                (i32.reinterpret_f32
                  (f32.add
                    (f32.reinterpret_i32 (i32.load (global.get $OFF_SCRATCH0)))
                    (f32.reinterpret_i32 (i32.load (global.get $OFF_SCRATCH1)))
                  )
                )
              ))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h147
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (i32.store (global.get $OFF_SCRATCH1) (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $err (call $stack_push
                (i32.reinterpret_f32
                  (f32.sub
                    (f32.reinterpret_i32 (i32.load (global.get $OFF_SCRATCH0)))
                    (f32.reinterpret_i32 (i32.load (global.get $OFF_SCRATCH1)))
                  )
                )
              ))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h148
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (i32.store (global.get $OFF_SCRATCH1) (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $err (call $stack_push
                (i32.reinterpret_f32
                  (f32.mul
                    (f32.reinterpret_i32 (i32.load (global.get $OFF_SCRATCH0)))
                    (f32.reinterpret_i32 (i32.load (global.get $OFF_SCRATCH1)))
                  )
                )
              ))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h149
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (i32.store (global.get $OFF_SCRATCH1) (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $err (call $stack_push
                (i32.reinterpret_f32
                  (f32.div
                    (f32.reinterpret_i32 (i32.load (global.get $OFF_SCRATCH0)))
                    (f32.reinterpret_i32 (i32.load (global.get $OFF_SCRATCH1)))
                  )
                )
              ))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h150
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (i32.store (global.get $OFF_SCRATCH1) (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $err (call $stack_push
                (i32.reinterpret_f32
                  (f32.min
                    (f32.reinterpret_i32 (i32.load (global.get $OFF_SCRATCH0)))
                    (f32.reinterpret_i32 (i32.load (global.get $OFF_SCRATCH1)))
                  )
                )
              ))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h151
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (i32.store (global.get $OFF_SCRATCH1) (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $err (call $stack_push
                (i32.reinterpret_f32
                  (f32.max
                    (f32.reinterpret_i32 (i32.load (global.get $OFF_SCRATCH0)))
                    (f32.reinterpret_i32 (i32.load (global.get $OFF_SCRATCH1)))
                  )
                )
              ))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h152
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (i32.store (global.get $OFF_SCRATCH1) (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $err (call $stack_push
                (i32.reinterpret_f32
                  (f32.copysign
                    (f32.reinterpret_i32 (i32.load (global.get $OFF_SCRATCH0)))
                    (f32.reinterpret_i32 (i32.load (global.get $OFF_SCRATCH1)))
                  )
                )
              ))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h153
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp64 (i64.or (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH0))) (i64.shl (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH1))) (i64.const 32))))
              (local.set $tmp_f64 (f64.abs (f64.reinterpret_i64 (local.get $tmp64))))
              (local.set $tmp64 (i64.reinterpret_f64 (local.get $tmp_f64)))
              (local.set $err (call $stack_push_i64 (i32.wrap_i64 (local.get $tmp64)) (i32.wrap_i64 (i64.shr_u (local.get $tmp64) (i64.const 32)))))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h154
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp64 (i64.or (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH0))) (i64.shl (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH1))) (i64.const 32))))
              (local.set $tmp_f64 (f64.neg (f64.reinterpret_i64 (local.get $tmp64))))
              (local.set $tmp64 (i64.reinterpret_f64 (local.get $tmp_f64)))
              (local.set $err (call $stack_push_i64 (i32.wrap_i64 (local.get $tmp64)) (i32.wrap_i64 (i64.shr_u (local.get $tmp64) (i64.const 32)))))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h155
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp64 (i64.or (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH0))) (i64.shl (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH1))) (i64.const 32))))
              (local.set $tmp_f64 (f64.ceil (f64.reinterpret_i64 (local.get $tmp64))))
              (local.set $tmp64 (i64.reinterpret_f64 (local.get $tmp_f64)))
              (local.set $err (call $stack_push_i64 (i32.wrap_i64 (local.get $tmp64)) (i32.wrap_i64 (i64.shr_u (local.get $tmp64) (i64.const 32)))))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h156
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp64 (i64.or (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH0))) (i64.shl (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH1))) (i64.const 32))))
              (local.set $tmp_f64 (f64.floor (f64.reinterpret_i64 (local.get $tmp64))))
              (local.set $tmp64 (i64.reinterpret_f64 (local.get $tmp_f64)))
              (local.set $err (call $stack_push_i64 (i32.wrap_i64 (local.get $tmp64)) (i32.wrap_i64 (i64.shr_u (local.get $tmp64) (i64.const 32)))))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h157
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp64 (i64.or (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH0))) (i64.shl (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH1))) (i64.const 32))))
              (local.set $tmp_f64 (f64.trunc (f64.reinterpret_i64 (local.get $tmp64))))
              (local.set $tmp64 (i64.reinterpret_f64 (local.get $tmp_f64)))
              (local.set $err (call $stack_push_i64 (i32.wrap_i64 (local.get $tmp64)) (i32.wrap_i64 (i64.shr_u (local.get $tmp64) (i64.const 32)))))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h158
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp64 (i64.or (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH0))) (i64.shl (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH1))) (i64.const 32))))
              (local.set $tmp_f64 (f64.nearest (f64.reinterpret_i64 (local.get $tmp64))))
              (local.set $tmp64 (i64.reinterpret_f64 (local.get $tmp_f64)))
              (local.set $err (call $stack_push_i64 (i32.wrap_i64 (local.get $tmp64)) (i32.wrap_i64 (i64.shr_u (local.get $tmp64) (i64.const 32)))))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h159
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp64 (i64.or (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH0))) (i64.shl (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH1))) (i64.const 32))))
              (local.set $tmp_f64 (f64.sqrt (f64.reinterpret_i64 (local.get $tmp64))))
              (local.set $tmp64 (i64.reinterpret_f64 (local.get $tmp_f64)))
              (local.set $err (call $stack_push_i64 (i32.wrap_i64 (local.get $tmp64)) (i32.wrap_i64 (i64.shr_u (local.get $tmp64) (i64.const 32)))))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h160
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp64b (i64.or (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH0))) (i64.shl (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH1))) (i64.const 32))))
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp64 (i64.or (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH0))) (i64.shl (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH1))) (i64.const 32))))
              (local.set $tmp_f64 (f64.add (f64.reinterpret_i64 (local.get $tmp64)) (f64.reinterpret_i64 (local.get $tmp64b))))
              (local.set $tmp64 (i64.reinterpret_f64 (local.get $tmp_f64)))
              (local.set $err (call $stack_push_i64 (i32.wrap_i64 (local.get $tmp64)) (i32.wrap_i64 (i64.shr_u (local.get $tmp64) (i64.const 32)))))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h161
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp64b (i64.or (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH0))) (i64.shl (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH1))) (i64.const 32))))
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp64 (i64.or (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH0))) (i64.shl (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH1))) (i64.const 32))))
              (local.set $tmp_f64 (f64.sub (f64.reinterpret_i64 (local.get $tmp64)) (f64.reinterpret_i64 (local.get $tmp64b))))
              (local.set $tmp64 (i64.reinterpret_f64 (local.get $tmp_f64)))
              (local.set $err (call $stack_push_i64 (i32.wrap_i64 (local.get $tmp64)) (i32.wrap_i64 (i64.shr_u (local.get $tmp64) (i64.const 32)))))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h162
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp64b (i64.or (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH0))) (i64.shl (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH1))) (i64.const 32))))
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp64 (i64.or (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH0))) (i64.shl (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH1))) (i64.const 32))))
              (local.set $tmp_f64 (f64.mul (f64.reinterpret_i64 (local.get $tmp64)) (f64.reinterpret_i64 (local.get $tmp64b))))
              (local.set $tmp64 (i64.reinterpret_f64 (local.get $tmp_f64)))
              (local.set $err (call $stack_push_i64 (i32.wrap_i64 (local.get $tmp64)) (i32.wrap_i64 (i64.shr_u (local.get $tmp64) (i64.const 32)))))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h163
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp64b (i64.or (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH0))) (i64.shl (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH1))) (i64.const 32))))
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp64 (i64.or (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH0))) (i64.shl (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH1))) (i64.const 32))))
              (local.set $tmp_f64 (f64.div (f64.reinterpret_i64 (local.get $tmp64)) (f64.reinterpret_i64 (local.get $tmp64b))))
              (local.set $tmp64 (i64.reinterpret_f64 (local.get $tmp_f64)))
              (local.set $err (call $stack_push_i64 (i32.wrap_i64 (local.get $tmp64)) (i32.wrap_i64 (i64.shr_u (local.get $tmp64) (i64.const 32)))))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h164
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp64b (i64.or (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH0))) (i64.shl (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH1))) (i64.const 32))))
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp64 (i64.or (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH0))) (i64.shl (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH1))) (i64.const 32))))
              (local.set $tmp_f64 (f64.min (f64.reinterpret_i64 (local.get $tmp64)) (f64.reinterpret_i64 (local.get $tmp64b))))
              (local.set $tmp64 (i64.reinterpret_f64 (local.get $tmp_f64)))
              (local.set $err (call $stack_push_i64 (i32.wrap_i64 (local.get $tmp64)) (i32.wrap_i64 (i64.shr_u (local.get $tmp64) (i64.const 32)))))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h165
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp64b (i64.or (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH0))) (i64.shl (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH1))) (i64.const 32))))
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp64 (i64.or (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH0))) (i64.shl (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH1))) (i64.const 32))))
              (local.set $tmp_f64 (f64.max (f64.reinterpret_i64 (local.get $tmp64)) (f64.reinterpret_i64 (local.get $tmp64b))))
              (local.set $tmp64 (i64.reinterpret_f64 (local.get $tmp_f64)))
              (local.set $err (call $stack_push_i64 (i32.wrap_i64 (local.get $tmp64)) (i32.wrap_i64 (i64.shr_u (local.get $tmp64) (i64.const 32)))))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h166
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp64b (i64.or (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH0))) (i64.shl (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH1))) (i64.const 32))))
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp64 (i64.or (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH0))) (i64.shl (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH1))) (i64.const 32))))
              (local.set $tmp_f64 (f64.copysign (f64.reinterpret_i64 (local.get $tmp64)) (f64.reinterpret_i64 (local.get $tmp64b))))
              (local.set $tmp64 (i64.reinterpret_f64 (local.get $tmp_f64)))
              (local.set $err (call $stack_push_i64 (i32.wrap_i64 (local.get $tmp64)) (i32.wrap_i64 (i64.shr_u (local.get $tmp64) (i64.const 32)))))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h167
              ;; Pop i64 (2 slots), push lo as i32 (1 slot)
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $err (call $stack_push (i32.load (global.get $OFF_SCRATCH0))))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h168
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $err (call $stack_push (i32.trunc_f32_s (f32.reinterpret_i32 (i32.load (global.get $OFF_SCRATCH0))))))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h169
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $err (call $stack_push (i32.trunc_f32_u (f32.reinterpret_i32 (i32.load (global.get $OFF_SCRATCH0))))))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h170
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp64 (i64.or (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH0))) (i64.shl (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH1))) (i64.const 32))))
              (local.set $err (call $stack_push (i32.trunc_f64_s (f64.reinterpret_i64 (local.get $tmp64)))))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h171
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp64 (i64.or (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH0))) (i64.shl (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH1))) (i64.const 32))))
              (local.set $err (call $stack_push (i32.trunc_f64_u (f64.reinterpret_i64 (local.get $tmp64)))))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h172
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp64 (i64.extend_i32_s (i32.load (global.get $OFF_SCRATCH0))))
              (local.set $err (call $stack_push_i64
                (i32.wrap_i64 (local.get $tmp64))
                (i32.wrap_i64 (i64.shr_u (local.get $tmp64) (i64.const 32)))
              ))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h173
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp64 (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH0))))
              (local.set $err (call $stack_push_i64
                (i32.wrap_i64 (local.get $tmp64))
                (i32.wrap_i64 (i64.shr_u (local.get $tmp64) (i64.const 32)))
              ))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h174
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp64 (i64.trunc_f32_s (f32.reinterpret_i32 (i32.load (global.get $OFF_SCRATCH0)))))
              (local.set $err (call $stack_push_i64 (i32.wrap_i64 (local.get $tmp64)) (i32.wrap_i64 (i64.shr_u (local.get $tmp64) (i64.const 32)))))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h175
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp64 (i64.trunc_f32_u (f32.reinterpret_i32 (i32.load (global.get $OFF_SCRATCH0)))))
              (local.set $err (call $stack_push_i64 (i32.wrap_i64 (local.get $tmp64)) (i32.wrap_i64 (i64.shr_u (local.get $tmp64) (i64.const 32)))))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h176
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp64 (i64.or (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH0))) (i64.shl (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH1))) (i64.const 32))))
              (local.set $tmp64 (i64.trunc_f64_s (f64.reinterpret_i64 (local.get $tmp64))))
              (local.set $err (call $stack_push_i64 (i32.wrap_i64 (local.get $tmp64)) (i32.wrap_i64 (i64.shr_u (local.get $tmp64) (i64.const 32)))))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h177
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp64 (i64.or (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH0))) (i64.shl (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH1))) (i64.const 32))))
              (local.set $tmp64 (i64.trunc_f64_u (f64.reinterpret_i64 (local.get $tmp64))))
              (local.set $err (call $stack_push_i64 (i32.wrap_i64 (local.get $tmp64)) (i32.wrap_i64 (i64.shr_u (local.get $tmp64) (i64.const 32)))))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h178
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $err (call $stack_push (i32.reinterpret_f32 (f32.convert_i32_s (i32.load (global.get $OFF_SCRATCH0))))))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h179
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $err (call $stack_push (i32.reinterpret_f32 (f32.convert_i32_u (i32.load (global.get $OFF_SCRATCH0))))))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h180
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp64 (i64.or (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH0))) (i64.shl (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH1))) (i64.const 32))))
              (local.set $err (call $stack_push (i32.reinterpret_f32 (f32.convert_i64_s (local.get $tmp64)))))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h181
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp64 (i64.or (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH0))) (i64.shl (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH1))) (i64.const 32))))
              (local.set $err (call $stack_push (i32.reinterpret_f32 (f32.convert_i64_u (local.get $tmp64)))))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h182
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp64 (i64.or (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH0))) (i64.shl (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH1))) (i64.const 32))))
              (local.set $err (call $stack_push (i32.reinterpret_f32 (f32.demote_f64 (f64.reinterpret_i64 (local.get $tmp64))))))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h183
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp64 (i64.reinterpret_f64 (f64.convert_i32_s (i32.load (global.get $OFF_SCRATCH0)))))
              (local.set $err (call $stack_push_i64 (i32.wrap_i64 (local.get $tmp64)) (i32.wrap_i64 (i64.shr_u (local.get $tmp64) (i64.const 32)))))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h184
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp64 (i64.reinterpret_f64 (f64.convert_i32_u (i32.load (global.get $OFF_SCRATCH0)))))
              (local.set $err (call $stack_push_i64 (i32.wrap_i64 (local.get $tmp64)) (i32.wrap_i64 (i64.shr_u (local.get $tmp64) (i64.const 32)))))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h185
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp64 (i64.or (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH0))) (i64.shl (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH1))) (i64.const 32))))
              (local.set $tmp64 (i64.reinterpret_f64 (f64.convert_i64_s (local.get $tmp64))))
              (local.set $err (call $stack_push_i64 (i32.wrap_i64 (local.get $tmp64)) (i32.wrap_i64 (i64.shr_u (local.get $tmp64) (i64.const 32)))))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h186
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp64 (i64.or (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH0))) (i64.shl (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH1))) (i64.const 32))))
              (local.set $tmp64 (i64.reinterpret_f64 (f64.convert_i64_u (local.get $tmp64))))
              (local.set $err (call $stack_push_i64 (i32.wrap_i64 (local.get $tmp64)) (i32.wrap_i64 (i64.shr_u (local.get $tmp64) (i64.const 32)))))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h187
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp64 (i64.reinterpret_f64 (f64.promote_f32 (f32.reinterpret_i32 (i32.load (global.get $OFF_SCRATCH0))))))
              (local.set $err (call $stack_push_i64 (i32.wrap_i64 (local.get $tmp64)) (i32.wrap_i64 (i64.shr_u (local.get $tmp64) (i64.const 32)))))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h188
              ;; Value on stack is already i32 bit pattern, no-op
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h189
              ;; Value on stack is already i64 (two slots), no-op
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h190
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h191
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h192
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $val (i32.extend8_s (i32.load (global.get $OFF_SCRATCH0))))
              (local.set $err (call $stack_push (local.get $val)))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h193
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $val (i32.extend16_s (i32.load (global.get $OFF_SCRATCH0))))
              (local.set $err (call $stack_push (local.get $val)))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h194
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp64 (i64.or (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH0))) (i64.shl (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH1))) (i64.const 32))))
              (local.set $tmp64 (i64.extend8_s (local.get $tmp64)))
              (local.set $err (call $stack_push_i64 (i32.wrap_i64 (local.get $tmp64)) (i32.wrap_i64 (i64.shr_u (local.get $tmp64) (i64.const 32)))))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h195
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp64 (i64.or (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH0))) (i64.shl (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH1))) (i64.const 32))))
              (local.set $tmp64 (i64.extend16_s (local.get $tmp64)))
              (local.set $err (call $stack_push_i64 (i32.wrap_i64 (local.get $tmp64)) (i32.wrap_i64 (i64.shr_u (local.get $tmp64) (i64.const 32)))))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h196
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp64 (i64.or (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH0))) (i64.shl (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH1))) (i64.const 32))))
              (local.set $tmp64 (i64.extend32_s (local.get $tmp64)))
              (local.set $err (call $stack_push_i64 (i32.wrap_i64 (local.get $tmp64)) (i32.wrap_i64 (i64.shr_u (local.get $tmp64) (i64.const 32)))))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h197
        (return (global.get $ERR_UNSUP))
          )  ;; close $h198
        (return (global.get $ERR_UNSUP))
          )  ;; close $h199
        (return (global.get $ERR_UNSUP))
          )  ;; close $h200
        (return (global.get $ERR_UNSUP))
          )  ;; close $h201
        (return (global.get $ERR_UNSUP))
          )  ;; close $h202
        (return (global.get $ERR_UNSUP))
          )  ;; close $h203
        (return (global.get $ERR_UNSUP))
          )  ;; close $h204
        (return (global.get $ERR_UNSUP))
          )  ;; close $h205
        (return (global.get $ERR_UNSUP))
          )  ;; close $h206
        (return (global.get $ERR_UNSUP))
          )  ;; close $h207
        (return (global.get $ERR_UNSUP))
          )  ;; close $h208
              (local.set $err (call $stack_push (i32.const -1)))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h209
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $val (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $err (call $stack_push
                (if (result i32) (i32.eq (local.get $val) (i32.const -1)) (then (i32.const 1)) (else (i32.const 0)))
              ))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h210
              ;; imm0 = function index, push it as reference value
              (local.set $err (call $stack_push (local.get $imm0)))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h211
        (return (global.get $ERR_UNSUP))
          )  ;; close $h212
        (return (global.get $ERR_UNSUP))
          )  ;; close $h213
        (return (global.get $ERR_UNSUP))
          )  ;; close $h214
        (return (global.get $ERR_UNSUP))
          )  ;; close $h215
        (return (global.get $ERR_UNSUP))
          )  ;; close $h216
        (return (global.get $ERR_UNSUP))
          )  ;; close $h217
        (return (global.get $ERR_UNSUP))
          )  ;; close $h218
        (return (global.get $ERR_UNSUP))
          )  ;; close $h219
        (return (global.get $ERR_UNSUP))
          )  ;; close $h220
        (return (global.get $ERR_UNSUP))
          )  ;; close $h221
        (return (global.get $ERR_UNSUP))
          )  ;; close $h222
        (return (global.get $ERR_UNSUP))
          )  ;; close $h223
        (return (global.get $ERR_UNSUP))
          )  ;; close $h224
        (return (global.get $ERR_UNSUP))
          )  ;; close $h225
        (return (global.get $ERR_UNSUP))
          )  ;; close $h226
        (return (global.get $ERR_UNSUP))
          )  ;; close $h227
        (return (global.get $ERR_UNSUP))
          )  ;; close $h228
        (return (global.get $ERR_UNSUP))
          )  ;; close $h229
        (return (global.get $ERR_UNSUP))
          )  ;; close $h230
        (return (global.get $ERR_UNSUP))
          )  ;; close $h231
        (return (global.get $ERR_UNSUP))
          )  ;; close $h232
        (return (global.get $ERR_UNSUP))
          )  ;; close $h233
        (return (global.get $ERR_UNSUP))
          )  ;; close $h234
        (return (global.get $ERR_UNSUP))
          )  ;; close $h235
        (return (global.get $ERR_UNSUP))
          )  ;; close $h236
        (return (global.get $ERR_UNSUP))
          )  ;; close $h237
        (return (global.get $ERR_UNSUP))
          )  ;; close $h238
        (return (global.get $ERR_UNSUP))
          )  ;; close $h239
        (return (global.get $ERR_UNSUP))
          )  ;; close $h240
        (return (global.get $ERR_UNSUP))
          )  ;; close $h241
        (return (global.get $ERR_UNSUP))
          )  ;; close $h242
        (return (global.get $ERR_UNSUP))
          )  ;; close $h243
        (return (global.get $ERR_UNSUP))
          )  ;; close $h244
        (return (global.get $ERR_UNSUP))
          )  ;; close $h245
        (return (global.get $ERR_UNSUP))
          )  ;; close $h246
        (return (global.get $ERR_UNSUP))
          )  ;; close $h247
        (return (global.get $ERR_UNSUP))
          )  ;; close $h248
        (return (global.get $ERR_UNSUP))
          )  ;; close $h249
        (return (global.get $ERR_UNSUP))
          )  ;; close $h250
        (return (global.get $ERR_UNSUP))
          )  ;; close $h251
        (return (global.get $ERR_UNSUP))
          )  ;; close $h252
              ;; imm0 = sub-opcode
              ;; ── Saturating truncation ops 0x00-0x07 ──
              (if (i32.eq (local.get $imm0) (i32.const 0x00))     ;; i32.trunc_sat_f32_s
                (then
                  (local.set $err (call $stack_pop))
                  (if (local.get $err) (then (return (local.get $err))))
                  (local.set $tmp_f32 (f32.reinterpret_i32 (i32.load (global.get $OFF_SCRATCH0))))
                  (if (f32.ne (local.get $tmp_f32) (local.get $tmp_f32))
                    (then (local.set $err (call $stack_push (i32.const 0))))
                    (else
                      (if (f32.ge (local.get $tmp_f32) (f32.const 2147483648.0))
                        (then (local.set $err (call $stack_push (i32.const 0x7FFFFFFF))))
                        (else
                          (if (f32.lt (local.get $tmp_f32) (f32.const -2147483648.0))
                            (then (local.set $err (call $stack_push (i32.const 0x80000000))))
                            (else (local.set $err (call $stack_push (i32.trunc_f32_s (local.get $tmp_f32)))))
                          )
                        )
                      )
                    )
                  )
                  (if (local.get $err) (then (return (local.get $err))))
                  (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
                  (br $dispatch_loop)
                )
              )
              (if (i32.eq (local.get $imm0) (i32.const 0x01))     ;; i32.trunc_sat_f32_u
                (then
                  (local.set $err (call $stack_pop))
                  (if (local.get $err) (then (return (local.get $err))))
                  (local.set $tmp_f32 (f32.reinterpret_i32 (i32.load (global.get $OFF_SCRATCH0))))
                  (if (f32.ne (local.get $tmp_f32) (local.get $tmp_f32))
                    (then (local.set $err (call $stack_push (i32.const 0))))
                    (else
                      (if (f32.ge (local.get $tmp_f32) (f32.const 4294967296.0))
                        (then (local.set $err (call $stack_push (i32.const -1))))
                        (else
                          (if (f32.lt (local.get $tmp_f32) (f32.const 0.0))
                            (then (local.set $err (call $stack_push (i32.const 0))))
                            (else (local.set $err (call $stack_push (i32.trunc_f32_u (local.get $tmp_f32)))))
                          )
                        )
                      )
                    )
                  )
                  (if (local.get $err) (then (return (local.get $err))))
                  (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
                  (br $dispatch_loop)
                )
              )
              (if (i32.eq (local.get $imm0) (i32.const 0x02))     ;; i32.trunc_sat_f64_s
                (then
                  (local.set $err (call $stack_pop_i64))
                  (if (local.get $err) (then (return (local.get $err))))
                  (local.set $tmp64 (i64.or (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH0))) (i64.shl (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH1))) (i64.const 32))))
                  (local.set $tmp_f64 (f64.reinterpret_i64 (local.get $tmp64)))
                  (if (f64.ne (local.get $tmp_f64) (local.get $tmp_f64))
                    (then (local.set $err (call $stack_push (i32.const 0))))
                    (else
                      (if (f64.ge (local.get $tmp_f64) (f64.const 2147483648.0))
                        (then (local.set $err (call $stack_push (i32.const 0x7FFFFFFF))))
                        (else
                          (if (f64.lt (local.get $tmp_f64) (f64.const -2147483648.0))
                            (then (local.set $err (call $stack_push (i32.const 0x80000000))))
                            (else (local.set $err (call $stack_push (i32.trunc_f64_s (local.get $tmp_f64)))))
                          )
                        )
                      )
                    )
                  )
                  (if (local.get $err) (then (return (local.get $err))))
                  (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
                  (br $dispatch_loop)
                )
              )
              (if (i32.eq (local.get $imm0) (i32.const 0x03))     ;; i32.trunc_sat_f64_u
                (then
                  (local.set $err (call $stack_pop_i64))
                  (if (local.get $err) (then (return (local.get $err))))
                  (local.set $tmp64 (i64.or (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH0))) (i64.shl (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH1))) (i64.const 32))))
                  (local.set $tmp_f64 (f64.reinterpret_i64 (local.get $tmp64)))
                  (if (f64.ne (local.get $tmp_f64) (local.get $tmp_f64))
                    (then (local.set $err (call $stack_push (i32.const 0))))
                    (else
                      (if (f64.ge (local.get $tmp_f64) (f64.const 4294967296.0))
                        (then (local.set $err (call $stack_push (i32.const -1))))
                        (else
                          (if (f64.lt (local.get $tmp_f64) (f64.const 0.0))
                            (then (local.set $err (call $stack_push (i32.const 0))))
                            (else (local.set $err (call $stack_push (i32.trunc_f64_u (local.get $tmp_f64)))))
                          )
                        )
                      )
                    )
                  )
                  (if (local.get $err) (then (return (local.get $err))))
                  (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
                  (br $dispatch_loop)
                )
              )
              (if (i32.eq (local.get $imm0) (i32.const 0x04))     ;; i64.trunc_sat_f32_s
                (then
                  (local.set $err (call $stack_pop))
                  (if (local.get $err) (then (return (local.get $err))))
                  (local.set $tmp_f32 (f32.reinterpret_i32 (i32.load (global.get $OFF_SCRATCH0))))
                  (if (f32.ne (local.get $tmp_f32) (local.get $tmp_f32))
                    (then
                      (local.set $tmp64 (i64.const 0))
                      (local.set $err (call $stack_push_i64 (i32.const 0) (i32.const 0)))
                    )
                    (else
                      (if (f32.ge (local.get $tmp_f32) (f32.const 9223372036854775808.0))
                        (then
                          (local.set $tmp64 (i64.const 0x7FFFFFFFFFFFFFFF))
                          (local.set $err (call $stack_push_i64 (i32.wrap_i64 (local.get $tmp64)) (i32.wrap_i64 (i64.shr_u (local.get $tmp64) (i64.const 32)))))
                        )
                        (else
                          (if (f32.lt (local.get $tmp_f32) (f32.const -9223372036854775808.0))
                            (then
                              (local.set $tmp64 (i64.const 0x8000000000000000))
                              (local.set $err (call $stack_push_i64 (i32.wrap_i64 (local.get $tmp64)) (i32.wrap_i64 (i64.shr_u (local.get $tmp64) (i64.const 32)))))
                            )
                            (else
                              (local.set $tmp64 (i64.trunc_f32_s (local.get $tmp_f32)))
                              (local.set $err (call $stack_push_i64 (i32.wrap_i64 (local.get $tmp64)) (i32.wrap_i64 (i64.shr_u (local.get $tmp64) (i64.const 32)))))
                            )
                          )
                        )
                      )
                    )
                  )
                  (if (local.get $err) (then (return (local.get $err))))
                  (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
                  (br $dispatch_loop)
                )
              )
              (if (i32.eq (local.get $imm0) (i32.const 0x05))     ;; i64.trunc_sat_f32_u
                (then
                  (local.set $err (call $stack_pop))
                  (if (local.get $err) (then (return (local.get $err))))
                  (local.set $tmp_f32 (f32.reinterpret_i32 (i32.load (global.get $OFF_SCRATCH0))))
                  (if (f32.ne (local.get $tmp_f32) (local.get $tmp_f32))
                    (then
                      (local.set $err (call $stack_push_i64 (i32.const 0) (i32.const 0)))
                    )
                    (else
                      (if (f32.ge (local.get $tmp_f32) (f32.const 18446744073709551616.0))
                        (then
                          (local.set $err (call $stack_push_i64 (i32.const -1) (i32.const -1)))
                        )
                        (else
                          (if (f32.lt (local.get $tmp_f32) (f32.const 0.0))
                            (then
                              (local.set $err (call $stack_push_i64 (i32.const 0) (i32.const 0)))
                            )
                            (else
                              (local.set $tmp64 (i64.trunc_f32_u (local.get $tmp_f32)))
                              (local.set $err (call $stack_push_i64 (i32.wrap_i64 (local.get $tmp64)) (i32.wrap_i64 (i64.shr_u (local.get $tmp64) (i64.const 32)))))
                            )
                          )
                        )
                      )
                    )
                  )
                  (if (local.get $err) (then (return (local.get $err))))
                  (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
                  (br $dispatch_loop)
                )
              )
              (if (i32.eq (local.get $imm0) (i32.const 0x06))     ;; i64.trunc_sat_f64_s
                (then
                  (local.set $err (call $stack_pop_i64))
                  (if (local.get $err) (then (return (local.get $err))))
                  (local.set $tmp64 (i64.or (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH0))) (i64.shl (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH1))) (i64.const 32))))
                  (local.set $tmp_f64 (f64.reinterpret_i64 (local.get $tmp64)))
                  (if (f64.ne (local.get $tmp_f64) (local.get $tmp_f64))
                    (then
                      (local.set $err (call $stack_push_i64 (i32.const 0) (i32.const 0)))
                    )
                    (else
                      (if (f64.ge (local.get $tmp_f64) (f64.const 9223372036854775808.0))
                        (then
                          (local.set $tmp64 (i64.const 0x7FFFFFFFFFFFFFFF))
                          (local.set $err (call $stack_push_i64 (i32.wrap_i64 (local.get $tmp64)) (i32.wrap_i64 (i64.shr_u (local.get $tmp64) (i64.const 32)))))
                        )
                        (else
                          (if (f64.lt (local.get $tmp_f64) (f64.const -9223372036854775808.0))
                            (then
                              (local.set $tmp64 (i64.const 0x8000000000000000))
                              (local.set $err (call $stack_push_i64 (i32.wrap_i64 (local.get $tmp64)) (i32.wrap_i64 (i64.shr_u (local.get $tmp64) (i64.const 32)))))
                            )
                            (else
                              (local.set $tmp64 (i64.trunc_f64_s (local.get $tmp_f64)))
                              (local.set $err (call $stack_push_i64 (i32.wrap_i64 (local.get $tmp64)) (i32.wrap_i64 (i64.shr_u (local.get $tmp64) (i64.const 32)))))
                            )
                          )
                        )
                      )
                    )
                  )
                  (if (local.get $err) (then (return (local.get $err))))
                  (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
                  (br $dispatch_loop)
                )
              )
              (if (i32.eq (local.get $imm0) (i32.const 0x07))     ;; i64.trunc_sat_f64_u
                (then
                  (local.set $err (call $stack_pop_i64))
                  (if (local.get $err) (then (return (local.get $err))))
                  (local.set $tmp64 (i64.or (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH0))) (i64.shl (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH1))) (i64.const 32))))
                  (local.set $tmp_f64 (f64.reinterpret_i64 (local.get $tmp64)))
                  (if (f64.ne (local.get $tmp_f64) (local.get $tmp_f64))
                    (then
                      (local.set $err (call $stack_push_i64 (i32.const 0) (i32.const 0)))
                    )
                    (else
                      (if (f64.ge (local.get $tmp_f64) (f64.const 18446744073709551616.0))
                        (then
                          (local.set $err (call $stack_push_i64 (i32.const -1) (i32.const -1)))
                        )
                        (else
                          (if (f64.lt (local.get $tmp_f64) (f64.const 0.0))
                            (then
                              (local.set $err (call $stack_push_i64 (i32.const 0) (i32.const 0)))
                            )
                            (else
                              (local.set $tmp64 (i64.trunc_f64_u (local.get $tmp_f64)))
                              (local.set $err (call $stack_push_i64 (i32.wrap_i64 (local.get $tmp64)) (i32.wrap_i64 (i64.shr_u (local.get $tmp64) (i64.const 32)))))
                            )
                          )
                        )
                      )
                    )
                  )
                  (if (local.get $err) (then (return (local.get $err))))
                  (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
                  (br $dispatch_loop)
                )
              )
              (if (i32.eq (local.get $imm0) (i32.const 0x0A))     ;; memory.copy
                (then
                  ;; pop n, src, dst
                  (local.set $err (call $stack_pop))
                  (if (local.get $err) (then (return (local.get $err))))
                  (local.set $imm2 (i32.load (global.get $OFF_SCRATCH0)))  ;; n
                  (local.set $err (call $stack_pop))
                  (if (local.get $err) (then (return (local.get $err))))
                  (local.set $imm3 (i32.load (global.get $OFF_SCRATCH0)))  ;; src
                  (local.set $err (call $stack_pop))
                  (if (local.get $err) (then (return (local.get $err))))
                  (local.set $val (i32.load (global.get $OFF_SCRATCH0)))  ;; dst
                  ;; Bounds check
                  (local.set $p (i32.shl (i32.load (global.get $OFF_GUEST_MEM_PAGES)) (i32.const 16)))
                  (if (i32.gt_u (i32.add (local.get $val) (local.get $imm2)) (local.get $p))
                    (then (return (global.get $ERR_TRAP)))
                  )
                  (if (i32.gt_u (i32.add (local.get $imm3) (local.get $imm2)) (local.get $p))
                    (then (return (global.get $ERR_TRAP)))
                  )
                  ;; Copy bytes
                  (local.set $i (i32.const 0))
                  (block $bulk_copy_lp
                    (loop $bulk_copy_cont
                      (if (i32.ge_u (local.get $i) (local.get $imm2)) (then (br $bulk_copy_lp)))
                      (i32.store8
                        (i32.add (global.get $OFF_GUEST_MEM_BASE) (i32.add (local.get $val) (local.get $i)))
                        (i32.load8_u (i32.add (global.get $OFF_GUEST_MEM_BASE) (i32.add (local.get $imm3) (local.get $i))))
                      )
                      (local.set $i (i32.add (local.get $i) (i32.const 1)))
                      (br $bulk_copy_cont)
                    )
                  )
                  (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
                  (br $dispatch_loop)
                )
              )
              (if (i32.eq (local.get $imm0) (i32.const 0x0B))     ;; memory.fill
                (then
                  ;; pop n, val, dst
                  (local.set $err (call $stack_pop))
                  (if (local.get $err) (then (return (local.get $err))))
                  (local.set $imm2 (i32.load (global.get $OFF_SCRATCH0)))  ;; n
                  (local.set $err (call $stack_pop))
                  (if (local.get $err) (then (return (local.get $err))))
                  (local.set $imm3 (i32.load (global.get $OFF_SCRATCH0)))  ;; val
                  (local.set $err (call $stack_pop))
                  (if (local.get $err) (then (return (local.get $err))))
                  (local.set $val (i32.load (global.get $OFF_SCRATCH0)))  ;; dst
                  ;; Bounds check
                  (local.set $p (i32.shl (i32.load (global.get $OFF_GUEST_MEM_PAGES)) (i32.const 16)))
                  (if (i32.gt_u (i32.add (local.get $val) (local.get $imm2)) (local.get $p))
                    (then (return (global.get $ERR_TRAP)))
                  )
                  ;; Fill bytes (val = dst, imm3 = fill byte, imm2 = count)
                  (local.set $i (i32.const 0))
                  (block $bulk_fill_lp
                    (loop $bulk_fill_cont
                      (if (i32.ge_u (local.get $i) (local.get $imm2)) (then (br $bulk_fill_lp)))
                      (i32.store8
                        (i32.add (global.get $OFF_GUEST_MEM_BASE) (i32.add (local.get $val) (local.get $i)))
                        (i32.and (local.get $imm3) (i32.const 0xFF))
                      )
                      (local.set $i (i32.add (local.get $i) (i32.const 1)))
                      (br $bulk_fill_cont)
                    )
                  )
                  (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
                  (br $dispatch_loop)
                )
              )
              (if (i32.eq (local.get $imm0) (i32.const 0x08))     ;; memory.init
                (then
                  ;; imm1 = data_seg_idx
                  ;; pop n, src_offset, dst
                  (local.set $err (call $stack_pop))
                  (if (local.get $err) (then (return (local.get $err))))
                  (local.set $imm2 (i32.load (global.get $OFF_SCRATCH0)))  ;; n
                  (local.set $err (call $stack_pop))
                  (if (local.get $err) (then (return (local.get $err))))
                  (local.set $imm3 (i32.load (global.get $OFF_SCRATCH0)))  ;; src_offset
                  (local.set $err (call $stack_pop))
                  (if (local.get $err) (then (return (local.get $err))))
                  (local.set $val (i32.load (global.get $OFF_SCRATCH0)))  ;; dst
                  ;; Look up data segment info
                  (local.set $p (i32.load (global.get $OFF_DATA_COUNT)))
                  (if (i32.ge_u (local.get $imm1) (local.get $p)) (then (return (global.get $ERR_TRAP))))
                  (local.set $op_base (i32.add (global.get $OFF_DATA_BUF) (i32.mul (local.get $imm1) (global.get $SZ_DATA))))
                  (local.set $p (i32.load (local.get $op_base)))  ;; data_offset in WASM binary
                  (local.set $i (i32.load (i32.add (local.get $op_base) (i32.const 8))))  ;; data_len
                  ;; Check bounds: src_offset + n <= data_len
                  (if (i32.gt_u (i32.add (local.get $imm3) (local.get $imm2)) (local.get $i))
                    (then (return (global.get $ERR_TRAP)))
                  )
                  ;; Check bounds: dst + n <= memory_size
                  (local.set $i (i32.shl (i32.load (global.get $OFF_GUEST_MEM_PAGES)) (i32.const 16)))
                  (if (i32.gt_u (i32.add (local.get $val) (local.get $imm2)) (local.get $i))
                    (then (return (global.get $ERR_TRAP)))
                  )
                  ;; Copy n bytes from wasm_base[data_offset + src_offset] to guest_mem[dst]
                  (local.set $i (i32.const 0))
                  (local.set $p (i32.add (i32.load (global.get $OFF_WASM_PTR)) (local.get $p)))  ;; absolute data ptr
                  (block $mi_copy_lp
                    (loop $mi_copy_cont
                      (if (i32.ge_u (local.get $i) (local.get $imm2)) (then (br $mi_copy_lp)))
                      (i32.store8
                        (i32.add (global.get $OFF_GUEST_MEM_BASE) (i32.add (local.get $val) (local.get $i)))
                        (i32.load8_u (i32.add (local.get $p) (i32.add (local.get $imm3) (local.get $i))))
                      )
                      (local.set $i (i32.add (local.get $i) (i32.const 1)))
                      (br $mi_copy_cont)
                    )
                  )
                  (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
                  (br $dispatch_loop)
                )
              )
              (if (i32.eq (local.get $imm0) (i32.const 0x09))     ;; data.drop
                (then
                  ;; imm1 = data_seg_idx
                  ;; Check segment index in bounds
                  (local.set $p (i32.load (global.get $OFF_DATA_COUNT)))
                  (if (i32.ge_u (local.get $imm1) (local.get $p)) (then (return (global.get $ERR_TRAP))))
                  ;; Mark segment as dropped by setting length to 0
                  (i32.store
                    (i32.add (global.get $OFF_DATA_BUF) (i32.add (i32.mul (local.get $imm1) (global.get $SZ_DATA)) (i32.const 8)))
                    (i32.const 0)
                  )
                  (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
                  (br $dispatch_loop)
                )
              )
              ;; ── table.init (0x0C) ──
              (if (i32.eq (local.get $imm0) (i32.const 0x0C))
                (then
                  ;; imm1 = elem_seg_idx
                  (if (i32.eqz (i32.load8_u (global.get $OFF_TABLE_HAS))) (then (return (global.get $ERR_TRAP))))
                  ;; Pop n, s, d from stack
                  (local.set $err (call $stack_pop))
                  (if (local.get $err) (then (return (local.get $err))))
                  (local.set $imm2 (i32.load (global.get $OFF_SCRATCH0)))  ;; n
                  (local.set $err (call $stack_pop))
                  (if (local.get $err) (then (return (local.get $err))))
                  (local.set $imm3 (i32.load (global.get $OFF_SCRATCH0)))  ;; s (elem src start)
                  (local.set $err (call $stack_pop))
                  (if (local.get $err) (then (return (local.get $err))))
                  (local.set $val (i32.load (global.get $OFF_SCRATCH0)))   ;; d (table dst start)
                  ;; Check elem segment index in bounds
                  (local.set $p (i32.load (global.get $OFF_ELEM_COUNT)))
                  (if (i32.ge_u (local.get $imm1) (local.get $p)) (then (return (global.get $ERR_TRAP))))
                  ;; Get elem segment base
                  (local.set $p (i32.add (global.get $OFF_ELEM_BUF) (i32.mul (local.get $imm1) (global.get $SZ_ELEM))))
                  ;; Check elem segment not dropped
                  (if (i32.load (local.get $p)) (then (return (global.get $ERR_TRAP))))
                  ;; elem_count from buffer  
                  (local.set $i (i32.load (i32.add (local.get $p) (i32.const 8))))  ;; elem_count
                  ;; Check s + n <= elem_count
                  (if (i32.gt_u (i32.add (local.get $imm3) (local.get $imm2)) (local.get $i))
                    (then (return (global.get $ERR_TRAP)))
                  )
                  ;; Check d + n <= table size
                  (local.set $i (i32.load (global.get $OFF_TABLE_MIN)))
                  (if (i32.gt_u (i32.add (local.get $val) (local.get $imm2)) (local.get $i))
                    (then (return (global.get $ERR_TRAP)))
                  )
                  ;; Copy from elem buf to table
                  (local.set $i (i32.const 0))
                  (block $ti_lp
                    (loop $ti_cont
                      (if (i32.ge_u (local.get $i) (local.get $imm2)) (then (br $ti_lp)))
                      (i32.store
                        (i32.add (global.get $OFF_TABLE_DATA) (i32.shl (i32.add (local.get $val) (local.get $i)) (i32.const 2)))
                        (i32.load
                          (i32.add (local.get $p) (i32.add (i32.const 12) (i32.shl (i32.add (local.get $imm3) (local.get $i)) (i32.const 2))))
                        )
                      )
                      (local.set $i (i32.add (local.get $i) (i32.const 1)))
                      (br $ti_cont)
                    )
                  )
                  (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
                  (br $dispatch_loop)
                )
              )
              ;; ── elem.drop (0x0D) ──
              (if (i32.eq (local.get $imm0) (i32.const 0x0D))
                (then
                  ;; imm1 = elem_seg_idx
                  (local.set $p (i32.load (global.get $OFF_ELEM_COUNT)))
                  (if (i32.ge_u (local.get $imm1) (local.get $p)) (then (return (global.get $ERR_TRAP))))
                  ;; Mark as dropped
                  (i32.store
                    (i32.add (global.get $OFF_ELEM_BUF) (i32.mul (local.get $imm1) (global.get $SZ_ELEM)))
                    (i32.const 1)
                  )
                  (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
                  (br $dispatch_loop)
                )
              )
              ;; ── table.copy (0x0E) ──
              (if (i32.eq (local.get $imm0) (i32.const 0x0E))
                (then
                  (if (i32.eqz (i32.load8_u (global.get $OFF_TABLE_HAS))) (then (return (global.get $ERR_TRAP))))
                  ;; Pop n, src_idx, dst_idx
                  (local.set $err (call $stack_pop))
                  (if (local.get $err) (then (return (local.get $err))))
                  (local.set $imm2 (i32.load (global.get $OFF_SCRATCH0)))  ;; n
                  (local.set $err (call $stack_pop))
                  (if (local.get $err) (then (return (local.get $err))))
                  (local.set $imm3 (i32.load (global.get $OFF_SCRATCH0)))  ;; src_idx
                  (local.set $err (call $stack_pop))
                  (if (local.get $err) (then (return (local.get $err))))
                  (local.set $val (i32.load (global.get $OFF_SCRATCH0)))   ;; dst_idx
                  ;; Bounds check
                  (local.set $p (i32.load (global.get $OFF_TABLE_MIN)))
                  (if (i32.or
                    (i32.gt_u (i32.add (local.get $val) (local.get $imm2)) (local.get $p))
                    (i32.gt_u (i32.add (local.get $imm3) (local.get $imm2)) (local.get $p))
                  ) (then (return (global.get $ERR_TRAP))))
                  ;; Copy (with overlap handling: use forward/backward as appropriate)
                  (if (i32.lt_u (local.get $val) (local.get $imm3))
                    (then
                      ;; Forward copy
                      (local.set $i (i32.const 0))
                      (block $tcf_lp
                        (loop $tcf_cont
                          (if (i32.ge_u (local.get $i) (local.get $imm2)) (then (br $tcf_lp)))
                          (i32.store
                            (i32.add (global.get $OFF_TABLE_DATA) (i32.shl (i32.add (local.get $val) (local.get $i)) (i32.const 2)))
                            (i32.load
                              (i32.add (global.get $OFF_TABLE_DATA) (i32.shl (i32.add (local.get $imm3) (local.get $i)) (i32.const 2)))
                            )
                          )
                          (local.set $i (i32.add (local.get $i) (i32.const 1)))
                          (br $tcf_cont)
                        )
                      )
                    )
                    (else
                      ;; Backward copy for overlapping ranges
                      (local.set $i (local.get $imm2))
                      (block $tcb_lp
                        (loop $tcb_cont
                          (if (i32.eqz (local.get $i)) (then (br $tcb_lp)))
                          (local.set $i (i32.sub (local.get $i) (i32.const 1)))
                          (i32.store
                            (i32.add (global.get $OFF_TABLE_DATA) (i32.shl (i32.add (local.get $val) (local.get $i)) (i32.const 2)))
                            (i32.load
                              (i32.add (global.get $OFF_TABLE_DATA) (i32.shl (i32.add (local.get $imm3) (local.get $i)) (i32.const 2)))
                            )
                          )
                          (br $tcb_cont)
                        )
                      )
                    )
                  )
                  (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
                  (br $dispatch_loop)
                )
              )
              ;; ── table.grow (0x0F) ──
              (if (i32.eq (local.get $imm0) (i32.const 0x0F))
                (then
                  (if (i32.eqz (i32.load8_u (global.get $OFF_TABLE_HAS))) (then (return (global.get $ERR_TRAP))))
                  ;; Pop delta (i32), then value (ref) from stack
                  (local.set $err (call $stack_pop))
                  (if (local.get $err) (then (return (local.get $err))))
                  (local.set $p (i32.load (global.get $OFF_SCRATCH0)))    ;; delta
                  (local.set $err (call $stack_pop))
                  (if (local.get $err) (then (return (local.get $err))))
                  (local.set $val (i32.load (global.get $OFF_SCRATCH0)))  ;; value
                  ;; Old size
                  (local.set $imm2 (i32.load (global.get $OFF_TABLE_MIN)))
                  (local.set $imm3 (i32.load (global.get $OFF_TABLE_MAX)))
                  ;; Check growth fits in max
                  (if (i32.gt_u (i32.add (local.get $imm2) (local.get $p)) (local.get $imm3))
                    (then
                      ;; Can't grow: push -1
                      (local.set $err (call $stack_push (i32.const -1)))
                    )
                    (else
                      ;; Grow: update min, init new entries with value
                      (local.set $i (i32.const 0))
                      (block $tg_lp
                        (loop $tg_cont
                          (if (i32.ge_u (local.get $i) (local.get $p)) (then (br $tg_lp)))
                          (i32.store
                            (i32.add (global.get $OFF_TABLE_DATA) (i32.shl (i32.add (local.get $imm2) (local.get $i)) (i32.const 2)))
                            (local.get $val)
                          )
                          (local.set $i (i32.add (local.get $i) (i32.const 1)))
                          (br $tg_cont)
                        )
                      )
                      (i32.store (global.get $OFF_TABLE_MIN) (i32.add (local.get $imm2) (local.get $p)))
                      ;; Push old size
                      (local.set $err (call $stack_push (local.get $imm2)))
                    )
                  )
                  (if (local.get $err) (then (return (local.get $err))))
                  (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
                  (br $dispatch_loop)
                )
              )
              ;; ── table.size (0x10) ──
              (if (i32.eq (local.get $imm0) (i32.const 0x10))
                (then
                  (if (i32.eqz (i32.load8_u (global.get $OFF_TABLE_HAS))) (then (return (global.get $ERR_TRAP))))
                  (local.set $err (call $stack_push (i32.load (global.get $OFF_TABLE_MIN))))
                  (if (local.get $err) (then (return (local.get $err))))
                  (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
                  (br $dispatch_loop)
                )
              )
              ;; ── table.fill (0x11) ──
              (if (i32.eq (local.get $imm0) (i32.const 0x11))
                (then
                  (if (i32.eqz (i32.load8_u (global.get $OFF_TABLE_HAS))) (then (return (global.get $ERR_TRAP))))
                  ;; Pop n, val, idx
                  (local.set $err (call $stack_pop))
                  (if (local.get $err) (then (return (local.get $err))))
                  (local.set $imm2 (i32.load (global.get $OFF_SCRATCH0)))  ;; n
                  (local.set $err (call $stack_pop))
                  (if (local.get $err) (then (return (local.get $err))))
                  (local.set $val (i32.load (global.get $OFF_SCRATCH0)))   ;; val
                  (local.set $err (call $stack_pop))
                  (if (local.get $err) (then (return (local.get $err))))
                  (local.set $p (i32.load (global.get $OFF_SCRATCH0)))     ;; idx
                  ;; Bounds check
                  (if (i32.gt_u (i32.add (local.get $p) (local.get $imm2)) (i32.load (global.get $OFF_TABLE_MIN)))
                    (then (return (global.get $ERR_TRAP)))
                  )
                  ;; Fill
                  (local.set $i (i32.const 0))
                  (block $tfl_lp
                    (loop $tfl_cont
                      (if (i32.ge_u (local.get $i) (local.get $imm2)) (then (br $tfl_lp)))
                      (i32.store
                        (i32.add (global.get $OFF_TABLE_DATA) (i32.shl (i32.add (local.get $p) (local.get $i)) (i32.const 2)))
                        (local.get $val)
                      )
                      (local.set $i (i32.add (local.get $i) (i32.const 1)))
                      (br $tfl_cont)
                    )
                  )
                  (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
                  (br $dispatch_loop)
                )
              )
              (return (global.get $ERR_UNSUP))
          )  ;; close $h253
        (return (global.get $ERR_UNSUP))
          )  ;; close $h254
        (return (global.get $ERR_UNSUP))
          )  ;; close $h255
        (return (global.get $ERR_UNSUP))
        )  ;; close $unsup
        )

        ;; Fallthrough increment for ops that didn't handle dec_idx
        (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
        (br $dispatch_loop)
      )
    )

    (return (global.get $OK))
  )

  ;; ═════════════════════════════════════════════════════════════════════
  ;; call: call an exported function by index
  ;; ═════════════════════════════════════════════════════════════════════
  (func (export "call") (param $func_idx i32) (param $args_ptr i32) (param $args_len i32) (result i32)
    ;; Reset execution state
    (i32.store (global.get $OFF_EXEC_STACK_LEN) (i32.const 0))
    (i32.store (global.get $OFF_EXEC_CTRL_LEN) (i32.const 0))
    (i32.store (global.get $OFF_EXEC_RES_COUNT) (i32.const 0))
    ;; Check module is loaded
    (if (i32.eqz (i32.load8_u (global.get $OFF_EXEC_MOD_VALID)))
      (then (return (global.get $ERR_BAD_ARGUMENT)))
    )
    (return (call $exec_fn (local.get $func_idx) (local.get $args_ptr) (local.get $args_len)))
  )

  ;; ═════════════════════════════════════════════════════════════════════
  ;; load: main entry point
  ;; ═════════════════════════════════════════════════════════════════════
  (func (export "load") (param $wasm_ptr i32) (param $wasm_len i32) (result i32)
    (local $offset i32) (local $err i32) (local $start_func i32) (local $sec_id i32)

    (i32.store (global.get $OFF_WASM_PTR) (local.get $wasm_ptr))
    (i32.store (global.get $OFF_WASM_LEN) (local.get $wasm_len))

    ;; Reset state
    (i32.store (global.get $OFF_TYPE_COUNT) (i32.const 0))
    (i32.store (global.get $OFF_IMPORT_COUNT) (i32.const 0))
    (i32.store (global.get $OFF_FUNCTION_COUNT) (i32.const 0))
    (i32.store (global.get $OFF_CODE_COUNT) (i32.const 0))
    (i32.store (global.get $OFF_EXPORT_COUNT) (i32.const 0))
    (i32.store (global.get $OFF_GLOBAL_COUNT) (i32.const 0))
    (i32.store (global.get $OFF_START_FUNC) (i32.const -1))
    (i32.store (global.get $OFF_DATA_COUNT) (i32.const 0))
    (i32.store (global.get $OFF_ELEM_COUNT) (i32.const 0))
    (i32.store (global.get $OFF_MEM_MIN) (i32.const 0))
    (i32.store (global.get $OFF_MEM_MAX) (i32.const 0))
    (i32.store (global.get $OFF_GUEST_MEM_PAGES) (i32.const 0))
    (i32.store8 (global.get $OFF_TABLE_HAS) (i32.const 0))
    (i32.store (global.get $OFF_NAMES_PTR) (global.get $OFF_NAMES_BUF))
    (i32.store8 (global.get $OFF_EXEC_MOD_VALID) (i32.const 0))
    (i32.store (global.get $OFF_DECODED_COUNT) (i32.const 0))

    (local.set $err (call $parse_header))
    (if (local.get $err) (then (return (local.get $err))))

    (local.set $offset (i32.const 8))

    (block $sec_loop
      (loop $sec_cont
        (if (i32.ge_u (local.get $offset) (local.get $wasm_len)) (then (br $sec_loop)))
        (local.set $err (call $read_section_header (local.get $offset)))
        (if (local.get $err) (then (return (local.get $err))))

        ;; Save section id to protect against scratch0 clobbering by parsers
        (local.set $sec_id (i32.load (global.get $OFF_SCRATCH0)))

        ;; scratch0=sec_id, scratch1=sec_len, scratch2=content_offset, scratch3=end_offset
        (if (i32.eq (local.get $sec_id) (global.get $SEC_TYPE))
          (then
            (local.set $err (call $parse_type_section
              (i32.load (global.get $OFF_SCRATCH2))
              (i32.load (global.get $OFF_SCRATCH1))
            ))
            (if (local.get $err) (then (return (local.get $err))))
            (i32.store (global.get $OFF_SCRATCH0) (local.get $sec_id))
          )
        )
        (if (i32.eq (local.get $sec_id) (global.get $SEC_IMPORT))
          (then
            (local.set $err (call $parse_import_section
              (i32.load (global.get $OFF_SCRATCH2))
              (i32.load (global.get $OFF_SCRATCH1))
            ))
            (if (local.get $err) (then (return (local.get $err))))
            (i32.store (global.get $OFF_SCRATCH0) (local.get $sec_id))
          )
        )
        (if (i32.eq (local.get $sec_id) (global.get $SEC_FUNCTION))
          (then
            (local.set $err (call $parse_function_section
              (i32.load (global.get $OFF_SCRATCH2))
              (i32.load (global.get $OFF_SCRATCH1))
            ))
            (if (local.get $err) (then (return (local.get $err))))
            (i32.store (global.get $OFF_SCRATCH0) (local.get $sec_id))
          )
        )
        (if (i32.eq (local.get $sec_id) (global.get $SEC_TABLE))
          (then
            (local.set $err (call $parse_table_section
              (i32.load (global.get $OFF_SCRATCH2))
              (i32.load (global.get $OFF_SCRATCH1))
            ))
            (if (local.get $err) (then (return (local.get $err))))
            (i32.store (global.get $OFF_SCRATCH0) (local.get $sec_id))
          )
        )
        (if (i32.eq (local.get $sec_id) (global.get $SEC_MEMORY))
          (then
            (local.set $err (call $parse_memory_section
              (i32.load (global.get $OFF_SCRATCH2))
              (i32.load (global.get $OFF_SCRATCH1))
            ))
            (if (local.get $err) (then (return (local.get $err))))
            (i32.store (global.get $OFF_SCRATCH0) (local.get $sec_id))
          )
        )
        (if (i32.eq (local.get $sec_id) (global.get $SEC_GLOBAL))
          (then
            (local.set $err (call $parse_global_section
              (i32.load (global.get $OFF_SCRATCH2))
              (i32.load (global.get $OFF_SCRATCH1))
            ))
            (if (local.get $err) (then (return (local.get $err))))
            (i32.store (global.get $OFF_SCRATCH0) (local.get $sec_id))
          )
        )
        (if (i32.eq (local.get $sec_id) (global.get $SEC_EXPORT))
          (then
            (local.set $err (call $parse_export_section
              (i32.load (global.get $OFF_SCRATCH2))
              (i32.load (global.get $OFF_SCRATCH1))
            ))
            (if (local.get $err) (then (return (local.get $err))))
            (i32.store (global.get $OFF_SCRATCH0) (local.get $sec_id))
          )
        )
        (if (i32.eq (local.get $sec_id) (global.get $SEC_START))
          (then
            (local.set $err (call $parse_start_section
              (i32.load (global.get $OFF_SCRATCH2))
              (i32.load (global.get $OFF_SCRATCH1))
            ))
            (if (local.get $err) (then (return (local.get $err))))
            (i32.store (global.get $OFF_SCRATCH0) (local.get $sec_id))
          )
        )
        (if (i32.eq (local.get $sec_id) (global.get $SEC_ELEMENT))
          (then
            (local.set $err (call $parse_element_section
              (i32.load (global.get $OFF_SCRATCH2))
              (i32.load (global.get $OFF_SCRATCH1))
            ))
            (if (local.get $err) (then (return (local.get $err))))
            (i32.store (global.get $OFF_SCRATCH0) (local.get $sec_id))
          )
        )
        (if (i32.eq (local.get $sec_id) (global.get $SEC_CODE))
          (then
            (local.set $err (call $parse_code_section
              (i32.load (global.get $OFF_SCRATCH2))
              (i32.load (global.get $OFF_SCRATCH1))
            ))
            (if (local.get $err) (then (return (local.get $err))))
            (i32.store (global.get $OFF_SCRATCH0) (local.get $sec_id))
          )
        )
        (if (i32.eq (local.get $sec_id) (global.get $SEC_DATA))
          (then
            (local.set $err (call $parse_data_section
              (i32.load (global.get $OFF_SCRATCH2))
              (i32.load (global.get $OFF_SCRATCH1))
            ))
            (if (local.get $err) (then (return (local.get $err))))
          )
        )
        ;; custom (0) and data_count (12): silently skipped

        ;; Advance to next section
        (local.set $offset (i32.load (global.get $OFF_SCRATCH3)))
        (br $sec_cont)
      )
    )

    ;; Execute start function if present
    (local.set $start_func (i32.load (global.get $OFF_START_FUNC)))
    (if (i32.ne (local.get $start_func) (i32.const -1))
      (then
        (i32.store (global.get $OFF_EXEC_STACK_LEN) (i32.const 0))
        (i32.store (global.get $OFF_EXEC_CTRL_LEN) (i32.const 0))
        (i32.store (global.get $OFF_EXEC_RES_COUNT) (i32.const 0))
        (local.set $err (call $exec_fn (local.get $start_func) (i32.const 0) (i32.const 0)))
        (if (local.get $err) (then (return (local.get $err))))
      )
    )

    (i32.store8 (global.get $OFF_EXEC_MOD_VALID) (i32.const 1))
    (return (global.get $OK))
  )

  (func (export "get_result_value") (param $idx i32) (result i64)
    (i64.store (global.get $OFF_SCRATCH0)
      (i64.load (i32.add (global.get $OFF_EXEC_RESULT) (i32.shl (local.get $idx) (i32.const 3))))
    )
    (return (i64.load (global.get $OFF_SCRATCH0)))
  )

  (func (export "get_result_count") (result i32)
    (return (i32.load (global.get $OFF_EXEC_RES_COUNT)))
  )

  ;; ═════════════════════════════════════════════════════════════════════
  ;; WAT Parser — parse WAT text format directly into state buffers
  ;; ═════════════════════════════════════════════════════════════════════
  ;;
  ;; Memory layout for WAT parser:
  ;;   0x8C000: WAT source pointer
  ;;   0x8C004: WAT source length
  ;;   0x8C008: Current position
  ;;   0x8C00C: Symbol table count
  ;;   0x8C010: Saved wasm_ptr (for restoration)
  ;;   0x8C014: Saved wasm_len
  ;;   0x8C018: Error position
  ;;   0x8C01C: Body offset (temp)
  ;;   0x8B000: Symbol table (256 entries × 16 bytes)
  ;;   0x8D000: WASM bytecode emission buffer (4KB)

  (global $OFF_WAT_PTR  i32 (i32.const 0x8C000))
  (global $OFF_WAT_LEN  i32 (i32.const 0x8C004))
  (global $OFF_WAT_POS  i32 (i32.const 0x8C008))
  (global $OFF_WAT_SYM  i32 (i32.const 0x8C00C))
  (global $OFF_WAT_SAV_PTR i32 (i32.const 0x8C010))
  (global $OFF_WAT_SAV_LEN i32 (i32.const 0x8C014))
  (global $OFF_WAT_ERR    i32 (i32.const 0x8C018))
  (global $OFF_WAT_TMP    i32 (i32.const 0x8C01C))
  (global $OFF_WAT_SYMS   i32 (i32.const 0x8B000))
  (global $WAT_SYM_SZ     i32 (i32.const 16))
  (global $WAT_MAX_SYMS   i32 (i32.const 256))
  (global $OFF_WAT_BODY   i32 (i32.const 0x8D000))
  (global $WAT_BODY_SZ    i32 (i32.const 4096))
  (global $OFF_WAT_TMP0   i32 (i32.const 0x8C040))
  (global $OFF_WAT_TMP1   i32 (i32.const 0x8C044))

  ;; ═════════════════════════════════════════════════════════════════════
  ;; WAT Lexer
  ;; ═════════════════════════════════════════════════════════════════════

  ;; Advance past whitespace and comments starting from pos.
  ;; Returns new position in scratch0, error in return value.
  (func $wat_skip_ws (param $pos i32) (result i32)
    (local $p i32) (local $l i32) (local $b i32) (local $end i32)
    (i32.store (i32.const 0x8C074) (i32.load (global.get $OFF_WAT_PTR)))
    (i32.store (i32.const 0x8C078) (local.get $pos))
    (local.set $p (i32.load (global.get $OFF_WAT_PTR)))
    (local.set $l (i32.load (global.get $OFF_WAT_LEN)))
    (local.set $end (i32.add (local.get $p) (local.get $l)))
    (local.set $p (i32.add (local.get $p) (local.get $pos)))
    (i32.store (i32.const 0x8C07C) (local.get $p))
    (block $done
      (loop $lp
        (if (i32.ge_u (local.get $p) (local.get $end)) (then (br $done)))
        (local.set $b (i32.load8_u (local.get $p)))
        ;; space, tab, lf, cr
        (if (i32.eq (local.get $b) (i32.const 0x20)) (then (local.set $p (i32.add (local.get $p) (i32.const 1))) (br $lp)))
        (if (i32.eq (local.get $b) (i32.const 0x09)) (then (local.set $p (i32.add (local.get $p) (i32.const 1))) (br $lp)))
        (if (i32.eq (local.get $b) (i32.const 0x0A)) (then (local.set $p (i32.add (local.get $p) (i32.const 1))) (br $lp)))
        (if (i32.eq (local.get $b) (i32.const 0x0D)) (then (local.set $p (i32.add (local.get $p) (i32.const 1))) (br $lp)))
        ;; Line comment: ;;
        (if (i32.eq (local.get $b) (i32.const 0x3B))
          (then
            (local.set $p (i32.add (local.get $p) (i32.const 1)))
            (if (i32.lt_u (local.get $p) (local.get $end))
              (then
                (local.set $b (i32.load8_u (local.get $p)))
                (local.set $p (i32.add (local.get $p) (i32.const 1)))
                (if (i32.eq (local.get $b) (i32.const 0x3B))
                  (then
                    ;; Skip to end of line
                    (block $eol
                      (loop $eol_lp
                        (if (i32.ge_u (local.get $p) (local.get $end)) (then (br $eol)))
                        (local.set $b (i32.load8_u (local.get $p)))
                        (local.set $p (i32.add (local.get $p) (i32.const 1)))
                        (if (i32.or (i32.eq (local.get $b) (i32.const 0x0A)) (i32.eq (local.get $b) (i32.const 0x0D)))
                          (then (br $eol))
                          (else (br $eol_lp))
                        )
                      )
                    )
                    (br $lp)
                  )
                )
              )
            )
            (br $done)
          )
        )
        ;; Block comment: (;
        (if (i32.eq (local.get $b) (i32.const 0x28))
          (then
            (local.set $p (i32.add (local.get $p) (i32.const 1)))
            (if (i32.lt_u (local.get $p) (local.get $end))
              (then
                (local.set $b (i32.load8_u (local.get $p)))
                (if (i32.eq (local.get $b) (i32.const 0x3B))
                  (then
                    (local.set $p (i32.add (local.get $p) (i32.const 1)))
                    ;; Skip to ;)
                    (block $bce
                      (loop $bcl
                        (if (i32.ge_u (local.get $p) (local.get $end)) (then (br $bce)))
                        (local.set $b (i32.load8_u (local.get $p)))
                        (local.set $p (i32.add (local.get $p) (i32.const 1)))
                        (if (i32.eq (local.get $b) (i32.const 0x3B))
                          (then
                            (if (i32.lt_u (local.get $p) (local.get $end))
                              (then
                                (local.set $b (i32.load8_u (local.get $p)))
                                (if (i32.eq (local.get $b) (i32.const 0x29))
                                  (then
                                    (local.set $p (i32.add (local.get $p) (i32.const 1)))
                                    (br $bce)
                                  )
                                )
                              )
                            )
                          )
                        )
                        (br $bcl)
                      )
                    )
                    (br $lp)
                  )
                  (else
                    ;; Not a block comment, back up
                    (local.set $p (i32.sub (local.get $p) (i32.const 1)))
                  )
                )
              )
            )
            (br $done)
          )
        )
        (br $done)
      )
    )
    (i32.store (global.get $OFF_SCRATCH0) (i32.sub (local.get $p) (i32.load (global.get $OFF_WAT_PTR))))
    (return (global.get $OK))
  )

  ;; Check if keyword at pos matches given kw_ptr/kw_len (case-insensitive).
  ;; Returns 1 on match, 0 on no match. Does NOT advance position.
  (func $wat_match_kw (param $pos i32) (param $kw_ptr i32) (param $kw_len i32) (result i32)
    (local $i i32) (local $p i32) (local $end i32) (local $b1 i32) (local $b2 i32)
    (local.set $p (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos)))
    (local.set $end (i32.add (local.get $p) (local.get $kw_len)))
    ;; Check bounds
    (if (i32.gt_u (local.get $end) (i32.add (i32.load (global.get $OFF_WAT_PTR)) (i32.load (global.get $OFF_WAT_LEN))))
      (then (return (i32.const 0)))
    )
    (local.set $i (i32.const 0))
    (block $done
      (loop $lp
        (if (i32.ge_u (local.get $i) (local.get $kw_len)) (then (br $done)))
        (local.set $b1 (i32.load8_u (i32.add (local.get $p) (local.get $i))))
        (local.set $b2 (i32.load8_u (i32.add (local.get $kw_ptr) (local.get $i))))
        ;; Case-insensitive compare: convert $b1 to lowercase
        (if (i32.and (i32.ge_u (local.get $b1) (i32.const 0x41)) (i32.le_u (local.get $b1) (i32.const 0x5A)))
          (then (local.set $b1 (i32.or (local.get $b1) (i32.const 32))))
        )
        (if (i32.and (i32.ge_u (local.get $b2) (i32.const 0x41)) (i32.le_u (local.get $b2) (i32.const 0x5A)))
          (then (local.set $b2 (i32.or (local.get $b2) (i32.const 32))))
        )
        (if (i32.ne (local.get $b1) (local.get $b2)) (then (return (i32.const 0))))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $lp)
      )
    )
    (return (i32.const 1))
  )

  ;; Read a keyword at pos (identifier characters: a-z A-Z 0-9 . _ + - * / < > ! ~)
  ;; Stores in WAT source: the keyword starts at pos, with given length.
  ;; Output: scratch0=keyword_offset, scratch1=keyword_len, scratch2=new_pos
  ;; Returns error code.
  (func $wat_read_kw (param $pos i32) (result i32)
    (local $p i32) (local $end i32) (local $b i32) (local $start i32)
    (local.set $p (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos)))
    (local.set $end (i32.add (i32.load (global.get $OFF_WAT_PTR)) (i32.load (global.get $OFF_WAT_LEN))))
    (if (i32.ge_u (local.get $p) (local.get $end))
      (then (return (global.get $ERR_PARSE)))
    )
    (local.set $start (local.get $p))
    (block $done
      (loop $lp
        (if (i32.ge_u (local.get $p) (local.get $end)) (then (br $done)))
        (local.set $b (i32.load8_u (local.get $p)))
        ;; Check if identifier character
        (block $is_id
          (if (i32.and (i32.ge_u (local.get $b) (i32.const 0x61)) (i32.le_u (local.get $b) (i32.const 0x7A))) (then (br $is_id)))
          (if (i32.and (i32.ge_u (local.get $b) (i32.const 0x41)) (i32.le_u (local.get $b) (i32.const 0x5A))) (then (br $is_id)))
          (if (i32.and (i32.ge_u (local.get $b) (i32.const 0x30)) (i32.le_u (local.get $b) (i32.const 0x39))) (then (br $is_id)))
          (if (i32.eq (local.get $b) (i32.const 0x2E)) (then (br $is_id)))  ;; .
          (if (i32.eq (local.get $b) (i32.const 0x5F)) (then (br $is_id)))  ;; _
          (if (i32.eq (local.get $b) (i32.const 0x2B)) (then (br $is_id)))  ;; +
          (if (i32.eq (local.get $b) (i32.const 0x2D)) (then (br $is_id)))  ;; -
          (if (i32.eq (local.get $b) (i32.const 0x2A)) (then (br $is_id)))  ;; *
          (if (i32.eq (local.get $b) (i32.const 0x2F)) (then (br $is_id)))  ;; /
          (if (i32.eq (local.get $b) (i32.const 0x3C)) (then (br $is_id)))  ;; <
          (if (i32.eq (local.get $b) (i32.const 0x3E)) (then (br $is_id)))  ;; >
          (if (i32.eq (local.get $b) (i32.const 0x21)) (then (br $is_id)))  ;; !
          (if (i32.eq (local.get $b) (i32.const 0x7E)) (then (br $is_id)))  ;; ~
          (if (i32.eq (local.get $b) (i32.const 0x27)) (then (br $is_id)))  ;; '
          (br $done)
        )
        (local.set $p (i32.add (local.get $p) (i32.const 1)))
        (br $lp)
      )
    )
    (if (i32.eq (local.get $start) (local.get $p))
      (then (return (global.get $ERR_PARSE)))
    )
    (i32.store (global.get $OFF_SCRATCH0) (local.get $pos))
    (i32.store (global.get $OFF_SCRATCH1) (i32.sub (local.get $p) (local.get $start)))
    (i32.store (global.get $OFF_SCRATCH2) (i32.add (local.get $pos) (i32.sub (local.get $p) (local.get $start))))
    (return (global.get $OK))
  )

  ;; Read a $identifier starting at pos. Store the name (without $) in names buffer.
  ;; Output: scratch0=name_offset (in names buf), scratch1=name_len, scratch2=new_pos
  ;; Returns error code.
  (func $wat_read_id (param $pos i32) (result i32)
    (local $p i32) (local $end i32) (local $b i32) (local $start i32) (local $name_start i32)
    (local $dst i32) (local $i i32) (local $len i32)
    (local.set $p (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos)))
    (local.set $end (i32.add (i32.load (global.get $OFF_WAT_PTR)) (i32.load (global.get $OFF_WAT_LEN))))
    (if (i32.ge_u (local.get $p) (local.get $end))
      (then (return (global.get $ERR_PARSE)))
    )
    (local.set $b (i32.load8_u (local.get $p)))
    (if (i32.ne (local.get $b) (i32.const 0x24))  ;; '$'
      (then (return (global.get $ERR_PARSE)))
    )
    (local.set $p (i32.add (local.get $p) (i32.const 1)))
    (local.set $start (local.get $p))
    (block $done
      (loop $lp
        (if (i32.ge_u (local.get $p) (local.get $end)) (then (br $done)))
        (local.set $b (i32.load8_u (local.get $p)))
        (block $is_idc
          (if (i32.and (i32.ge_u (local.get $b) (i32.const 0x61)) (i32.le_u (local.get $b) (i32.const 0x7A))) (then (br $is_idc)))
          (if (i32.and (i32.ge_u (local.get $b) (i32.const 0x41)) (i32.le_u (local.get $b) (i32.const 0x5A))) (then (br $is_idc)))
          (if (i32.and (i32.ge_u (local.get $b) (i32.const 0x30)) (i32.le_u (local.get $b) (i32.const 0x39))) (then (br $is_idc)))
          (if (i32.eq (local.get $b) (i32.const 0x2E)) (then (br $is_idc)))  ;; .
          (if (i32.eq (local.get $b) (i32.const 0x5F)) (then (br $is_idc)))  ;; _
          (if (i32.eq (local.get $b) (i32.const 0x2D)) (then (br $is_idc)))  ;; -
          (if (i32.eq (local.get $b) (i32.const 0x2B)) (then (br $is_idc)))  ;; +
          (if (i32.eq (local.get $b) (i32.const 0x27)) (then (br $is_idc)))  ;; '
          (br $done)
        )
        (local.set $p (i32.add (local.get $p) (i32.const 1)))
        (br $lp)
      )
    )
    (local.set $len (i32.sub (local.get $p) (local.get $start)))
    (if (i32.eqz (local.get $len)) (then (return (global.get $ERR_PARSE))))
    ;; Copy name to names buffer
    (local.set $dst (i32.load (global.get $OFF_NAMES_PTR)))
    (local.set $i (i32.const 0))
    (block $clp
      (loop $cl
        (if (i32.ge_u (local.get $i) (local.get $len)) (then (br $clp)))
        (i32.store8 (i32.add (local.get $dst) (local.get $i))
          (i32.load8_u (i32.add (local.get $start) (local.get $i)))
        )
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $cl)
      )
    )
    (i32.store (global.get $OFF_NAMES_PTR) (i32.add (local.get $dst) (local.get $len)))
    (i32.store (global.get $OFF_SCRATCH0) (local.get $dst))
    (i32.store (global.get $OFF_SCRATCH1) (local.get $len))
    (i32.store (global.get $OFF_SCRATCH2) (i32.add (local.get $pos) (i32.add (i32.const 1) (local.get $len))))
    (return (global.get $OK))
  )

  ;; Read an unsigned integer (decimal or 0x hex).
  ;; Output: scratch0=value, scratch1=new_pos
  ;; Returns error code.
  (func $wat_read_uint (param $pos i32) (result i32)
    (local $p i32) (local $end i32) (local $b i32) (local $val i32) (local $start i32)
    (local $neg i32)
    (local.set $p (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos)))
    (local.set $end (i32.add (i32.load (global.get $OFF_WAT_PTR)) (i32.load (global.get $OFF_WAT_LEN))))
    (if (i32.ge_u (local.get $p) (local.get $end))
      (then (return (global.get $ERR_PARSE)))
    )
    (local.set $start (local.get $p))
    (local.set $b (i32.load8_u (local.get $p)))
    ;; Optional leading sign for unsigned (will be treated as positive)
    (if (i32.eq (local.get $b) (i32.const 0x2D))
      (then (local.set $p (i32.add (local.get $p) (i32.const 1))) (local.set $neg (i32.const 1)))
    )
    (if (i32.eq (local.get $b) (i32.const 0x2B))
      (then (local.set $p (i32.add (local.get $p) (i32.const 1))))
    )
    ;; Check for hex prefix 0x or 0X
    (local.set $b (i32.load8_u (local.get $p)))
    (if (i32.eq (local.get $b) (i32.const 0x30))
      (then
        (local.set $p (i32.add (local.get $p) (i32.const 1)))
        (if (i32.lt_u (local.get $p) (local.get $end))
          (then
            (local.set $b (i32.load8_u (local.get $p)))
            (if (i32.or (i32.eq (local.get $b) (i32.const 0x78)) (i32.eq (local.get $b) (i32.const 0x58)))
              (then
                ;; Hex number
                (local.set $p (i32.add (local.get $p) (i32.const 1)))
                (local.set $val (i32.const 0))
                (block $hd
                  (loop $hl
                    (if (i32.ge_u (local.get $p) (local.get $end)) (then (br $hd)))
                    (local.set $b (i32.load8_u (local.get $p)))
                    (block $hdig
                      (if (i32.and (i32.ge_u (local.get $b) (i32.const 0x30)) (i32.le_u (local.get $b) (i32.const 0x39)))
                        (then (local.set $b (i32.sub (local.get $b) (i32.const 0x30))) (br $hdig)))
                      (if (i32.and (i32.ge_u (local.get $b) (i32.const 0x61)) (i32.le_u (local.get $b) (i32.const 0x66)))
                        (then (local.set $b (i32.sub (local.get $b) (i32.sub (i32.const 0x61) (i32.const 10)))) (br $hdig)))
                      (if (i32.and (i32.ge_u (local.get $b) (i32.const 0x41)) (i32.le_u (local.get $b) (i32.const 0x46)))
                        (then (local.set $b (i32.sub (local.get $b) (i32.sub (i32.const 0x41) (i32.const 10)))) (br $hdig)))
                      (br $hd)
                    )
                    (local.set $val (i32.add (i32.shl (local.get $val) (i32.const 4)) (local.get $b)))
                    (local.set $p (i32.add (local.get $p) (i32.const 1)))
                    (br $hl)
                  )
                )
                (if (i32.eq (local.get $start) (i32.sub (local.get $p) (i32.const 2)))
                  (then (return (global.get $ERR_PARSE)))  ;; "0x" with no digits
                )
                (i32.store (global.get $OFF_SCRATCH0) (local.get $val))
                (i32.store (global.get $OFF_SCRATCH1) (i32.sub (local.get $p) (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos))))
                (return (global.get $OK))
              )
            )
            ;; Not hex, it's a decimal 0
            (local.set $val (i32.const 0))
            (i32.store (global.get $OFF_SCRATCH0) (local.get $val))
            (i32.store (global.get $OFF_SCRATCH1) (i32.sub (local.get $p) (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos))))
            (return (global.get $OK))
          )
        )
      )
    )
    ;; Decimal
    (local.set $val (i32.const 0))
    (block $dd
      (loop $dl
        (if (i32.ge_u (local.get $p) (local.get $end)) (then (br $dd)))
        (local.set $b (i32.load8_u (local.get $p)))
        (if (i32.or (i32.lt_u (local.get $b) (i32.const 0x30)) (i32.gt_u (local.get $b) (i32.const 0x39)))
          (then (br $dd))
        )
        (local.set $val (i32.add (i32.mul (local.get $val) (i32.const 10)) (i32.sub (local.get $b) (i32.const 0x30))))
        (local.set $p (i32.add (local.get $p) (i32.const 1)))
        (br $dl)
      )
    )
    (i32.store (global.get $OFF_SCRATCH0) (local.get $val))
    (i32.store (global.get $OFF_SCRATCH1) (i32.sub (local.get $p) (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos))))
    (return (global.get $OK))
  )

  ;; Read a signed integer (decimal or 0x hex).
  ;; Output: scratch0=value, scratch1=new_pos
  ;; Returns error code.
  (func $wat_read_sint (param $pos i32) (result i32)
    (local $p i32) (local $end i32) (local $b i32) (local $val i32) (local $neg i32)
    (local.set $p (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos)))
    (local.set $end (i32.add (i32.load (global.get $OFF_WAT_PTR)) (i32.load (global.get $OFF_WAT_LEN))))
    (if (i32.ge_u (local.get $p) (local.get $end))
      (then (return (global.get $ERR_PARSE)))
    )
    (local.set $neg (i32.const 0))
    (local.set $b (i32.load8_u (local.get $p)))
    (if (i32.eq (local.get $b) (i32.const 0x2D))
      (then (local.set $neg (i32.const 1)) (local.set $p (i32.add (local.get $p) (i32.const 1))))
    )
    (if (i32.eq (local.get $b) (i32.const 0x2B))
      (then (local.set $p (i32.add (local.get $p) (i32.const 1))))
    )
    ;; Save relative offset of number start (after sign)
    (local.set $pos (i32.add (local.get $pos) (i32.sub (local.get $p) (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos)))))
    ;; Use unsigned reader on the rest
    (if (call $wat_read_uint (local.get $pos))
      (then (return (global.get $ERR_PARSE)))
    )
    (local.set $val (i32.load (global.get $OFF_SCRATCH0)))
    (if (local.get $neg)
      (then
        (local.set $val (i32.sub (i32.const 0) (local.get $val)))
        (i32.store (global.get $OFF_SCRATCH0) (local.get $val))
      )
    )
    ;; Add sign chars to position advance
    (i32.store (global.get $OFF_SCRATCH1)
      (i32.add
        (i32.sub (local.get $p) (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos)))
        (i32.load (global.get $OFF_SCRATCH1))
      )
    )
    (return (global.get $OK))
  )

  ;; ═════════════════════════════════════════════════════════════════════
  ;; WAT Symbol Table
  ;; ═════════════════════════════════════════════════════════════════════

  ;; Register a name in the symbol table.
  ;; name_off = offset in names buffer, name_len = length, kind = entity kind, index = entity index
  ;; Returns error (ERR_PARSE on overflow).
  (func $wat_sym_register (param $name_off i32) (param $name_len i32) (param $kind i32) (param $index i32) (result i32)
    (local $sym_cnt i32) (local $base i32)
    (local.set $sym_cnt (i32.load (global.get $OFF_WAT_SYM)))
    (if (i32.ge_u (local.get $sym_cnt) (global.get $WAT_MAX_SYMS))
      (then (return (global.get $ERR_PARSE)))
    )
    (local.set $base (i32.add (global.get $OFF_WAT_SYMS) (i32.mul (local.get $sym_cnt) (global.get $WAT_SYM_SZ))))
    (i32.store (i32.add (local.get $base) (i32.const 0)) (local.get $name_off))
    (i32.store (i32.add (local.get $base) (i32.const 4)) (local.get $name_len))
    (i32.store (i32.add (local.get $base) (i32.const 8)) (local.get $kind))
    (i32.store (i32.add (local.get $base) (i32.const 12)) (local.get $index))
    (i32.store (global.get $OFF_WAT_SYM) (i32.add (local.get $sym_cnt) (i32.const 1)))
    (return (global.get $OK))
  )

  ;; Look up a name in the symbol table.
  ;; kind = entity kind to search for (-1 = any kind).
  ;; Output: scratch0=index, scratch1=0 if found, 1 if not found.
  (func $wat_sym_lookup (param $name_off i32) (param $name_len i32) (param $kind i32) (result i32)
    (local $sym_cnt i32) (local $i i32) (local $base i32) (local $n_off i32) (local $n_len i32) (local $k i32)
    (local $j i32) (local $b1 i32) (local $b2 i32)
    (local.set $sym_cnt (i32.load (global.get $OFF_WAT_SYM)))
    (local.set $i (i32.const 0))
    (block $done
      (loop $lp
        (if (i32.ge_u (local.get $i) (local.get $sym_cnt)) (then (br $done)))
        (local.set $base (i32.add (global.get $OFF_WAT_SYMS) (i32.mul (local.get $i) (global.get $WAT_SYM_SZ))))
        (local.set $k (i32.load (i32.add (local.get $base) (i32.const 8))))
        (if (i32.and (i32.ne (local.get $kind) (i32.const -1)) (i32.ne (local.get $k) (local.get $kind)))
          (then (local.set $i (i32.add (local.get $i) (i32.const 1))) (br $lp))
        )
        (local.set $n_len (i32.load (i32.add (local.get $base) (i32.const 4))))
        (if (i32.ne (local.get $n_len) (local.get $name_len))
          (then (local.set $i (i32.add (local.get $i) (i32.const 1))) (br $lp))
        )
        (local.set $n_off (i32.load (i32.add (local.get $base) (i32.const 0))))
        ;; Compare names byte-by-byte
        (local.set $j (i32.const 0))
        (block $cmp_done
          (loop $cmp
            (if (i32.ge_u (local.get $j) (local.get $n_len)) (then (br $cmp_done)))
            (local.set $b1 (i32.load8_u (i32.add (local.get $n_off) (local.get $j))))
            (local.set $b2 (i32.load8_u (i32.add (local.get $name_off) (local.get $j))))
            (if (i32.ne (local.get $b1) (local.get $b2))
              (then (local.set $i (i32.add (local.get $i) (i32.const 1))) (br $lp))
            )
            (local.set $j (i32.add (local.get $j) (i32.const 1)))
            (br $cmp)
          )
        )
        ;; Found!
        (i32.store (global.get $OFF_SCRATCH0) (i32.load (i32.add (local.get $base) (i32.const 12))))
        (i32.store (global.get $OFF_SCRATCH1) (i32.const 0))
        (return (global.get $OK))
      )
    )
    (i32.store (global.get $OFF_SCRATCH1) (i32.const 1))
    (return (global.get $OK))
  )

  ;; Clear the symbol table (for a new function body scope).
  (func $wat_sym_clear (result i32)
    (i32.store (global.get $OFF_WAT_SYM) (i32.const 0))
    (return (global.get $OK))
  )

  ;; ═════════════════════════════════════════════════════════════════════
  ;; Value type helper: convert WAT type keyword → WASM type byte
  ;; Input: kw_offset, kw_len (from wat_read_kw)
  ;; Output: scratch0 = type byte (0x7F, etc.) or -1 if unknown
  ;; ═════════════════════════════════════════════════════════════════════
  (func $wat_valtype (param $kw_off i32) (param $kw_len i32) (result i32)
    (local $p i32) (local $b0 i32) (local $b1 i32) (local $b2 i32) (local $b3 i32)
    (local.set $p (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $kw_off)))
    (local.set $b0 (i32.load8_u (local.get $p)))
    (if (i32.gt_u (local.get $kw_len) (i32.const 1))
      (then (local.set $b1 (i32.load8_u (i32.add (local.get $p) (i32.const 1)))))
    )
    (if (i32.gt_u (local.get $kw_len) (i32.const 2))
      (then (local.set $b2 (i32.load8_u (i32.add (local.get $p) (i32.const 2)))))
    )
    (block $done
      (if (i32.eq (local.get $kw_len) (i32.const 3))
        (then
          (block $try3
            ;; "i32" = 0x69 0x33 0x32
            (if (i32.and (i32.eq (local.get $b0) (i32.const 0x69)) (i32.and (i32.eq (local.get $b1) (i32.const 0x33)) (i32.eq (local.get $b2) (i32.const 0x32))))
              (then (i32.store (global.get $OFF_SCRATCH0) (i32.const 0x7F)) (return (global.get $OK)))
            )
            ;; "i64" = 0x69 0x36 0x34
            (if (i32.and (i32.eq (local.get $b0) (i32.const 0x69)) (i32.and (i32.eq (local.get $b1) (i32.const 0x36)) (i32.eq (local.get $b2) (i32.const 0x34))))
              (then (i32.store (global.get $OFF_SCRATCH0) (i32.const 0x7E)) (return (global.get $OK)))
            )
            ;; "f32" = 0x66 0x33 0x32
            (if (i32.and (i32.eq (local.get $b0) (i32.const 0x66)) (i32.and (i32.eq (local.get $b1) (i32.const 0x33)) (i32.eq (local.get $b2) (i32.const 0x32))))
              (then (i32.store (global.get $OFF_SCRATCH0) (i32.const 0x7D)) (return (global.get $OK)))
            )
            ;; "f64" = 0x66 0x36 0x34
            (if (i32.and (i32.eq (local.get $b0) (i32.const 0x66)) (i32.and (i32.eq (local.get $b1) (i32.const 0x36)) (i32.eq (local.get $b2) (i32.const 0x34))))
              (then (i32.store (global.get $OFF_SCRATCH0) (i32.const 0x7C)) (return (global.get $OK)))
            )
          )
        )
      )
      (if (i32.eq (local.get $kw_len) (i32.const 4))
        (then
          (local.set $b3 (i32.load8_u (i32.add (local.get $p) (i32.const 3))))
          ;; "v128" = 0x76 ('v') 0x31 ('1') 0x32 ('2') 0x38 ('8')
          (if (i32.and (i32.eq (local.get $b0) (i32.const 0x76)) (i32.and (i32.eq (local.get $b1) (i32.const 0x31)) (i32.and (i32.eq (local.get $b2) (i32.const 0x32)) (i32.eq (local.get $b3) (i32.const 0x38)))))
            (then (i32.store (global.get $OFF_SCRATCH0) (i32.const 0x7B)) (return (global.get $OK)))
          )
        )
      )
    )
    (i32.store (global.get $OFF_SCRATCH0) (i32.const -1))
    (return (global.get $OK))
  )

  ;; Parse a (result ...) type annotation.
  ;; Expects: at pos, we've already consumed "(" and "result".
  ;; Reads value types until ")".
  ;; Output: scratch0=first_valtype_byte (0x40 if no results), scratch1=new_pos
  (func $wat_parse_result (param $pos i32) (result i32)
    (local $b i32) (local $vt i32)
    (local.set $vt (i32.const 0x40))  ;; default empty
    (block $lp
      (loop $cont
        (local.set $b (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos))))
        (if (i32.eq (local.get $b) (i32.const 0x29))  ;; )
          (then (local.set $pos (i32.add (local.get $pos) (i32.const 1))) (br $lp))
        )
        ;; Read value type keyword
        (if (call $wat_read_kw (local.get $pos)) (then (return (global.get $ERR_PARSE))))
        (local.set $vt (i32.load (global.get $OFF_SCRATCH0)))
        (if (call $wat_valtype (i32.load (global.get $OFF_SCRATCH0)) (i32.load (global.get $OFF_SCRATCH1)))
          (then (return (global.get $ERR_PARSE)))
        )
        (local.set $vt (i32.load (global.get $OFF_SCRATCH0)))
        (local.set $pos (i32.load (global.get $OFF_SCRATCH2)))
        ;; Only store first result type for now
        (br $lp)  ;; only support single result for now
      )
    )
    (i32.store (global.get $OFF_SCRATCH0) (local.get $vt))
    (i32.store (global.get $OFF_SCRATCH1) (local.get $pos))
    (return (global.get $OK))
  )

  ;; ═════════════════════════════════════════════════════════════════════
  ;; WASM bytecode emission helpers (to scratch buffer)
  ;; ═════════════════════════════════════════════════════════════════════

  ;; Emit one byte to the body buffer at given offset.
  ;; Output: scratch0 = new offset
  (func $wat_emit_byte (param $off i32) (param $b i32) (result i32)
    (if (i32.ge_u (local.get $off) (global.get $WAT_BODY_SZ))
      (then (return (global.get $ERR_NO_MEM)))
    )
    (i32.store8 (i32.add (global.get $OFF_WAT_BODY) (local.get $off)) (local.get $b))
    (i32.store (global.get $OFF_SCRATCH0) (i32.add (local.get $off) (i32.const 1)))
    (return (global.get $OK))
  )

  ;; Emit a LEB128 unsigned integer.
  ;; Output: scratch0 = new offset
  (func $wat_emit_leb_u32 (param $off i32) (param $val i32) (result i32)
    (local $b i32)
    (block $done
      (loop $lp
        (local.set $b (i32.and (local.get $val) (i32.const 0x7F)))
        (local.set $val (i32.shr_u (local.get $val) (i32.const 7)))
        (if (i32.ne (local.get $val) (i32.const 0))
          (then (local.set $b (i32.or (local.get $b) (i32.const 0x80))))
        )
        (if (call $wat_emit_byte (local.get $off) (local.get $b)) (then (return (global.get $ERR_NO_MEM))))
        (local.set $off (i32.load (global.get $OFF_SCRATCH0)))
        (if (i32.eqz (local.get $val)) (then (br $done)))
        (br $lp)
      )
    )
    (i32.store (global.get $OFF_SCRATCH0) (local.get $off))
    (return (global.get $OK))
  )

  ;; Emit a LEB128 signed integer (32-bit).
  ;; Output: scratch0 = new offset
  (func $wat_emit_leb_i32 (param $off i32) (param $val i32) (result i32)
    (local $b i32) (local $more i32) (local $sign i32)
    (local.set $sign (i32.and (local.get $val) (i32.const 0x40)))  ;; bit 6 of original
    (block $done
      (loop $lp
        (local.set $b (i32.and (local.get $val) (i32.const 0x7F)))
        (local.set $val (i32.shr_s (local.get $val) (i32.const 7)))
        ;; Check if more bytes needed
        (block $check_more
          (if (i32.eq (local.get $val) (i32.const 0))
            (then
              (if (i32.eqz (i32.and (local.get $b) (i32.const 0x40))) (then (br $check_more)))
            )
            (else
              (if (i32.eq (local.get $val) (i32.const -1))
                (then
                  (if (i32.and (local.get $b) (i32.const 0x40)) (then (br $check_more)))
                )
                (else (br $check_more))
              )
            )
          )
          (local.set $more (i32.const 0))
          (br $done)
        )
        (local.set $more (i32.const 1))
        (local.set $b (i32.or (local.get $b) (i32.const 0x80)))
        (if (call $wat_emit_byte (local.get $off) (local.get $b)) (then (return (global.get $ERR_NO_MEM))))
        (local.set $off (i32.load (global.get $OFF_SCRATCH0)))
        (br $lp)
      )
    )
    (if (call $wat_emit_byte (local.get $off) (local.get $b)) (then (return (global.get $ERR_NO_MEM))))
    (local.set $off (i32.load (global.get $OFF_SCRATCH0)))
    (i32.store (global.get $OFF_SCRATCH0) (local.get $off))
    (return (global.get $OK))
  )

  ;; ═════════════════════════════════════════════════════════════════════
  ;; Compare rest of keyword bytes (after first byte already matched)
  ;; Input: kw_off, start offset (usually 1), count of bytes to compare,
  ;;        11 expected byte values (unused ones should be 0)
  ;; Returns 1 if mismatch, 0 if match
  ;; ═════════════════════════════════════════════════════════════════════
  (func $wat_kw_match_rest (param $kw_off i32) (param $start i32) (param $count i32)
    (param $p0 i32) (param $p1 i32) (param $p2 i32) (param $p3 i32)
    (param $p4 i32) (param $p5 i32) (param $p6 i32) (param $p7 i32)
    (param $p8 i32) (param $p9 i32)
    (result i32)
    (local $base i32) (local $i i32) (local $b i32)
    (local.set $base (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $kw_off)))
    (local.set $i (i32.const 0))
    (block $done
      (loop $lp
        (if (i32.ge_u (local.get $i) (local.get $count)) (then (br $done)))
        (local.set $b (i32.load8_u (i32.add (local.get $base) (i32.add (local.get $start) (local.get $i)))))
        (if (i32.eq (local.get $i) (i32.const 0))
          (then (if (i32.ne (local.get $b) (local.get $p0)) (then (return (i32.const 1)))))
        )
        (if (i32.eq (local.get $i) (i32.const 1))
          (then (if (i32.ne (local.get $b) (local.get $p1)) (then (return (i32.const 1)))))
        )
        (if (i32.eq (local.get $i) (i32.const 2))
          (then (if (i32.ne (local.get $b) (local.get $p2)) (then (return (i32.const 1)))))
        )
        (if (i32.eq (local.get $i) (i32.const 3))
          (then (if (i32.ne (local.get $b) (local.get $p3)) (then (return (i32.const 1)))))
        )
        (if (i32.eq (local.get $i) (i32.const 4))
          (then (if (i32.ne (local.get $b) (local.get $p4)) (then (return (i32.const 1)))))
        )
        (if (i32.eq (local.get $i) (i32.const 5))
          (then (if (i32.ne (local.get $b) (local.get $p5)) (then (return (i32.const 1)))))
        )
        (if (i32.eq (local.get $i) (i32.const 6))
          (then (if (i32.ne (local.get $b) (local.get $p6)) (then (return (i32.const 1)))))
        )
        (if (i32.eq (local.get $i) (i32.const 7))
          (then (if (i32.ne (local.get $b) (local.get $p7)) (then (return (i32.const 1)))))
        )
        (if (i32.eq (local.get $i) (i32.const 8))
          (then (if (i32.ne (local.get $b) (local.get $p8)) (then (return (i32.const 1)))))
        )
        (if (i32.eq (local.get $i) (i32.const 9))
          (then (if (i32.ne (local.get $b) (local.get $p9)) (then (return (i32.const 1)))))
        )
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $lp)
      )
    )
    (return (i32.const 0))
  )

  ;; ═════════════════════════════════════════════════════════════════════
  ;; Function body opcode parser: reads WAT opcodes, emits WASM bytecodes
  ;; Input: pos = position in WAT source after func header
  ;; Output: scratch0 = decoded_start, scratch1 = decoded_count
  ;; Returns error code.
  ;; ═════════════════════════════════════════════════════════════════════

  ;; Parse one opcode. Returns: 0=continue, -1=done(')'), positive=error
  (func $wat_parse_body (param $pos i32) (param $body_off i32) (result i32)
    (local $err i32) (local $b i32) (local $kw_off i32)
    (local $kw_len i32) (local $b0 i32) (local $b1 i32) (local $b2 i32) (local $b3 i32)
    (local $imm i32)

    (i32.store (i32.const 0x8C060) (local.get $pos))
    (i32.store (i32.const 0x8C080) (local.get $body_off))

    ;; Skip whitespace
    (i32.store (i32.const 0x8C048) (i32.const 0x7001))
    (if (call $wat_skip_ws (local.get $pos)) (then (return (global.get $ERR_PARSE))))
    (i32.store (i32.const 0x8C048) (i32.const 0x7002))
    (i32.store (i32.const 0x8C084) (i32.load (global.get $OFF_SCRATCH0)))
    (local.set $pos (i32.load (global.get $OFF_SCRATCH0)))

    ;; Check for end of function: closing paren at top level
    (i32.store (i32.const 0x8C048) (i32.const 0x7003))
    (local.set $b (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos))))
    (if (i32.eq (local.get $b) (i32.const 0x29))  ;; )
      (then
        (i32.store (i32.const 0x8C048) (i32.const 0x7010))
        ;; Emit end opcode
        (if (call $wat_emit_byte (local.get $body_off) (i32.const 0x0B))
          (then (return (global.get $ERR_NO_MEM)))
        )
        (local.set $body_off (i32.load (global.get $OFF_SCRATCH0)))
        ;; Decode the emitted bytecodes
        (i32.store (global.get $OFF_WAT_TMP) (local.get $body_off))
        (i32.store (global.get $OFF_WAT_SAV_PTR) (i32.load (global.get $OFF_WASM_PTR)))
        (i32.store (global.get $OFF_WAT_SAV_LEN) (i32.load (global.get $OFF_WASM_LEN)))
        (i32.store (global.get $OFF_WASM_PTR) (global.get $OFF_WAT_BODY))
        (i32.store (global.get $OFF_WASM_LEN) (local.get $body_off))
        (local.set $err (call $decode_opcodes (i32.const 0) (local.get $body_off)))
        (if (local.get $err) (then (return (local.get $err))))
        (i32.store (global.get $OFF_WASM_PTR) (i32.load (global.get $OFF_WAT_SAV_PTR)))
        (i32.store (global.get $OFF_WASM_LEN) (i32.load (global.get $OFF_WAT_SAV_LEN)))
        (i32.store (global.get $OFF_SCRATCH0) (local.get $body_off))
        (i32.store (global.get $OFF_SCRATCH1) (i32.load (global.get $OFF_DECODED_COUNT)))
        (local.set $err (call $compute_end_targets
          (local.get $body_off)
          (i32.load (global.get $OFF_DECODED_COUNT))
        ))
        (if (local.get $err) (then (return (local.get $err))))
        (i32.store (global.get $OFF_SCRATCH2) (i32.add (local.get $pos) (i32.const 1)))
        (return (i32.const -1))  ;; DONE
      )
    )

    ;; Check for end of input
    (if (i32.ge_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos))
                  (i32.add (i32.load (global.get $OFF_WAT_PTR)) (i32.load (global.get $OFF_WAT_LEN))))
      (then (return (global.get $ERR_PARSE)))
    )

    ;; Read opcode keyword
    (i32.store (i32.const 0x8C048) (i32.const 0x7040))
    (i32.store (i32.const 0x8C088) (local.get $pos))
    (local.set $err (call $wat_read_kw (local.get $pos)))
    (i32.store (i32.const 0x8C048) (i32.const 0x7041))
    (if (local.get $err) (then (return (global.get $ERR_PARSE))))
    (local.set $kw_off (i32.load (global.get $OFF_SCRATCH0)))
    (local.set $kw_len (i32.load (global.get $OFF_SCRATCH1)))
    (local.set $pos (i32.load (global.get $OFF_SCRATCH2)))

      ;; ─── Opcode dispatch ───
      ;; Inline byte comparison: load first few bytes and compare
      (local.set $b0 (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $kw_off))))
      (i32.store (i32.const 0x8C04C) (local.get $kw_len))
      (i32.store (i32.const 0x8C050) (local.get $b0))
      (i32.store (i32.const 0x8C054) (local.get $kw_off))
      (i32.store (i32.const 0x8C058) (i32.load (global.get $OFF_WAT_PTR)))
        (block $not_unreachable
          (if (i32.ne (local.get $kw_len) (i32.const 11)) (then (br $not_unreachable)))
          (if (i32.ne (local.get $b0) (i32.const 0x75)) (then (br $not_unreachable)))  ;; 'u'
          (if (call $wat_kw_match_rest (local.get $kw_off) (i32.const 1) (i32.const 10)
                (i32.const 0x6e) (i32.const 0x72) (i32.const 0x65) (i32.const 0x61)  ;; nrea
                (i32.const 0x63) (i32.const 0x68) (i32.const 0x61) (i32.const 0x62)  ;; chab
                (i32.const 0x6c) (i32.const 0x65))  ;; le
            (then (br $not_unreachable))
          )
          ;; unreachable
          (local.set $b (i32.const 0x00))
          (if (call $wat_emit_byte (local.get $body_off) (local.get $b)) (then (return (global.get $ERR_NO_MEM))))
          (local.set $body_off (i32.load (global.get $OFF_SCRATCH0)))
          (i32.store (global.get $OFF_SCRATCH0) (local.get $body_off))
          (i32.store (global.get $OFF_SCRATCH1) (local.get $pos))
          (return (i32.const 0))
        )
        (block $not_nop
          (if (i32.ne (local.get $kw_len) (i32.const 3)) (then (br $not_nop)))
          (if (i32.ne (local.get $b0) (i32.const 0x6e)) (then (br $not_nop)))  ;; 'n'
          (local.set $b1 (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (i32.add (local.get $kw_off) (i32.const 1)))))
          (local.set $b2 (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (i32.add (local.get $kw_off) (i32.const 2)))))
          (if (i32.ne (local.get $b1) (i32.const 0x6f)) (then (br $not_nop)))  ;; 'o'
          (if (i32.ne (local.get $b2) (i32.const 0x70)) (then (br $not_nop)))  ;; 'p'
          ;; nop
          (if (call $wat_emit_byte (local.get $body_off) (i32.const 0x01)) (then (return (global.get $ERR_NO_MEM))))
          (local.set $body_off (i32.load (global.get $OFF_SCRATCH0)))
          (i32.store (global.get $OFF_SCRATCH0) (local.get $body_off))
          (i32.store (global.get $OFF_SCRATCH1) (local.get $pos))
          (return (i32.const 0))
        )
        (block $not_i32add
          (if (i32.ne (local.get $kw_len) (i32.const 7)) (then (br $not_i32add)))
          (if (i32.ne (local.get $b0) (i32.const 0x69)) (then (br $not_i32add)))  ;; 'i'
          (if (call $wat_kw_match_rest (local.get $kw_off) (i32.const 1) (i32.const 6)
                (i32.const 0x33) (i32.const 0x32) (i32.const 0x2e) (i32.const 0x61)  ;; 32.a
                (i32.const 0x64) (i32.const 0x64) (i32.const 0) (i32.const 0)  ;; dd
                (i32.const 0) (i32.const 0))
            (then (br $not_i32add))
          )
          ;; i32.add
          (if (call $wat_emit_byte (local.get $body_off) (i32.const 0x6a)) (then (return (global.get $ERR_NO_MEM))))
          (local.set $body_off (i32.load (global.get $OFF_SCRATCH0)))
          (i32.store (global.get $OFF_SCRATCH0) (local.get $body_off))
          (i32.store (global.get $OFF_SCRATCH1) (local.get $pos))
          (return (i32.const 0))
        )
        (block $not_i32sub
          (if (i32.ne (local.get $kw_len) (i32.const 7)) (then (br $not_i32sub)))
          (if (i32.ne (local.get $b0) (i32.const 0x69)) (then (br $not_i32sub)))
          (if (call $wat_kw_match_rest (local.get $kw_off) (i32.const 1) (i32.const 6)
                (i32.const 0x33) (i32.const 0x32) (i32.const 0x2e) (i32.const 0x73)  ;; 32.s
                (i32.const 0x75) (i32.const 0x62) (i32.const 0) (i32.const 0)  ;; ub
                (i32.const 0) (i32.const 0))
            (then (br $not_i32sub))
          )
          ;; i32.sub
          (if (call $wat_emit_byte (local.get $body_off) (i32.const 0x6b)) (then (return (global.get $ERR_NO_MEM))))
          (local.set $body_off (i32.load (global.get $OFF_SCRATCH0)))
          (i32.store (global.get $OFF_SCRATCH0) (local.get $body_off))
          (i32.store (global.get $OFF_SCRATCH1) (local.get $pos))
          (return (i32.const 0))
        )
        (block $not_i32const
          (if (i32.ne (local.get $kw_len) (i32.const 9)) (then (br $not_i32const)))
          (if (i32.ne (local.get $b0) (i32.const 0x69)) (then (br $not_i32const)))
          (if (call $wat_kw_match_rest (local.get $kw_off) (i32.const 1) (i32.const 8)
                (i32.const 0x33) (i32.const 0x32) (i32.const 0x2e) (i32.const 0x63)  ;; 32.c
                (i32.const 0x6f) (i32.const 0x6e) (i32.const 0x73) (i32.const 0x74)  ;; onst
                (i32.const 0) (i32.const 0))
            (then (br $not_i32const))
          )
          ;; i32.const — read immediate value
          (if (call $wat_skip_ws (local.get $pos)) (then (return (global.get $ERR_PARSE))))
          (local.set $pos (i32.load (global.get $OFF_SCRATCH0)))
          (if (call $wat_read_sint (local.get $pos)) (then (return (global.get $ERR_PARSE))))
          (local.set $imm (i32.load (global.get $OFF_SCRATCH0)))
          (local.set $pos (i32.load (global.get $OFF_SCRATCH1)))
          (if (call $wat_emit_byte (local.get $body_off) (i32.const 0x41)) (then (return (global.get $ERR_NO_MEM))))
          (local.set $body_off (i32.load (global.get $OFF_SCRATCH0)))
          (if (call $wat_emit_leb_i32 (local.get $body_off) (local.get $imm)) (then (return (global.get $ERR_NO_MEM))))
          (local.set $body_off (i32.load (global.get $OFF_SCRATCH0)))
          (i32.store (global.get $OFF_SCRATCH0) (local.get $body_off))
          (i32.store (global.get $OFF_SCRATCH1) (local.get $pos))
          (return (i32.const 0))
        )
        (block $not_localget
          (if (i32.ne (local.get $kw_len) (i32.const 9)) (then (br $not_localget)))
          (if (i32.ne (local.get $b0) (i32.const 0x6c)) (then (br $not_localget)))  ;; 'l'
          (if (call $wat_kw_match_rest (local.get $kw_off) (i32.const 1) (i32.const 8)
                (i32.const 0x6f) (i32.const 0x63) (i32.const 0x61) (i32.const 0x6c)  ;; ocal
                (i32.const 0x2e) (i32.const 0x67) (i32.const 0x65) (i32.const 0x74)  ;; .get
                (i32.const 0) (i32.const 0))
            (then (br $not_localget))
          )
          ;; local.get — read immediate local index
          (if (call $wat_skip_ws (local.get $pos)) (then (return (global.get $ERR_PARSE))))
          (local.set $pos (i32.load (global.get $OFF_SCRATCH0)))
          (if (call $wat_read_uint (local.get $pos)) (then (return (global.get $ERR_PARSE))))
          (local.set $imm (i32.load (global.get $OFF_SCRATCH0)))
          (local.set $pos (i32.load (global.get $OFF_SCRATCH1)))
          (if (call $wat_emit_byte (local.get $body_off) (i32.const 0x20)) (then (return (global.get $ERR_NO_MEM))))
          (local.set $body_off (i32.load (global.get $OFF_SCRATCH0)))
          (if (call $wat_emit_leb_u32 (local.get $body_off) (local.get $imm)) (then (return (global.get $ERR_NO_MEM))))
          (local.set $body_off (i32.load (global.get $OFF_SCRATCH0)))
          (i32.store (global.get $OFF_SCRATCH0) (local.get $body_off))
          (i32.store (global.get $OFF_SCRATCH1) (local.get $pos))
          (return (i32.const 0))
        )
        (block $not_localset
          (if (i32.ne (local.get $kw_len) (i32.const 9)) (then (br $not_localset)))
          (if (i32.ne (local.get $b0) (i32.const 0x6c)) (then (br $not_localset)))
          (if (call $wat_kw_match_rest (local.get $kw_off) (i32.const 1) (i32.const 8)
                (i32.const 0x6f) (i32.const 0x63) (i32.const 0x61) (i32.const 0x6c)  ;; ocal
                (i32.const 0x2e) (i32.const 0x73) (i32.const 0x65) (i32.const 0x74)  ;; .set
                (i32.const 0) (i32.const 0))
            (then (br $not_localset))
          )
          ;; local.set — read immediate local index
          (if (call $wat_skip_ws (local.get $pos)) (then (return (global.get $ERR_PARSE))))
          (local.set $pos (i32.load (global.get $OFF_SCRATCH0)))
          (if (call $wat_read_uint (local.get $pos)) (then (return (global.get $ERR_PARSE))))
          (local.set $imm (i32.load (global.get $OFF_SCRATCH0)))
          (local.set $pos (i32.load (global.get $OFF_SCRATCH1)))
          (if (call $wat_emit_byte (local.get $body_off) (i32.const 0x21)) (then (return (global.get $ERR_NO_MEM))))
          (local.set $body_off (i32.load (global.get $OFF_SCRATCH0)))
          (if (call $wat_emit_leb_u32 (local.get $body_off) (local.get $imm)) (then (return (global.get $ERR_NO_MEM))))
          (local.set $body_off (i32.load (global.get $OFF_SCRATCH0)))
          (i32.store (global.get $OFF_SCRATCH0) (local.get $body_off))
          (i32.store (global.get $OFF_SCRATCH1) (local.get $pos))
          (return (i32.const 0))
        )
        (block $not_localtee
          (if (i32.ne (local.get $kw_len) (i32.const 9)) (then (br $not_localtee)))
          (if (i32.ne (local.get $b0) (i32.const 0x6c)) (then (br $not_localtee)))
          (if (call $wat_kw_match_rest (local.get $kw_off) (i32.const 1) (i32.const 8)
                (i32.const 0x6f) (i32.const 0x63) (i32.const 0x61) (i32.const 0x6c)  ;; ocal
                (i32.const 0x2e) (i32.const 0x74) (i32.const 0x65) (i32.const 0x65)  ;; .tee
                (i32.const 0) (i32.const 0))
            (then (br $not_localtee))
          )
          ;; local.tee — read immediate local index
          (if (call $wat_skip_ws (local.get $pos)) (then (return (global.get $ERR_PARSE))))
          (local.set $pos (i32.load (global.get $OFF_SCRATCH0)))
          (if (call $wat_read_uint (local.get $pos)) (then (return (global.get $ERR_PARSE))))
          (local.set $imm (i32.load (global.get $OFF_SCRATCH0)))
          (local.set $pos (i32.load (global.get $OFF_SCRATCH1)))
          (if (call $wat_emit_byte (local.get $body_off) (i32.const 0x22)) (then (return (global.get $ERR_NO_MEM))))
          (local.set $body_off (i32.load (global.get $OFF_SCRATCH0)))
          (if (call $wat_emit_leb_u32 (local.get $body_off) (local.get $imm)) (then (return (global.get $ERR_NO_MEM))))
          (local.set $body_off (i32.load (global.get $OFF_SCRATCH0)))
          (i32.store (global.get $OFF_SCRATCH0) (local.get $body_off))
          (i32.store (global.get $OFF_SCRATCH1) (local.get $pos))
          (return (i32.const 0))
        )
        (block $not_return
          (if (i32.ne (local.get $kw_len) (i32.const 6)) (then (br $not_return)))
          (if (i32.ne (local.get $b0) (i32.const 0x72)) (then (br $not_return)))  ;; 'r'
          (if (call $wat_kw_match_rest (local.get $kw_off) (i32.const 1) (i32.const 5)
                (i32.const 0x65) (i32.const 0x74) (i32.const 0x75) (i32.const 0x72)  ;; etur
                (i32.const 0x6e) (i32.const 0) (i32.const 0) (i32.const 0)  ;; n
                (i32.const 0) (i32.const 0))
            (then (br $not_return))
          )
          ;; return
          (if (call $wat_emit_byte (local.get $body_off) (i32.const 0x0F)) (then (return (global.get $ERR_NO_MEM))))
          (local.set $body_off (i32.load (global.get $OFF_SCRATCH0)))
          (i32.store (global.get $OFF_SCRATCH0) (local.get $body_off))
          (i32.store (global.get $OFF_SCRATCH1) (local.get $pos))
          (return (i32.const 0))
        )
        (block $not_drop
          (if (i32.ne (local.get $kw_len) (i32.const 4)) (then (br $not_drop)))
          (if (i32.ne (local.get $b0) (i32.const 0x64)) (then (br $not_drop)))  ;; 'd'
          (local.set $b1 (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (i32.add (local.get $kw_off) (i32.const 1)))))
          (local.set $b2 (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (i32.add (local.get $kw_off) (i32.const 2)))))
          (local.set $b3 (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (i32.add (local.get $kw_off) (i32.const 3)))))
          (if (i32.ne (local.get $b1) (i32.const 0x72)) (then (br $not_drop)))  ;; 'r'
          (if (i32.ne (local.get $b2) (i32.const 0x6f)) (then (br $not_drop)))  ;; 'o'
          (if (i32.ne (local.get $b3) (i32.const 0x70)) (then (br $not_drop)))  ;; 'p'
          ;; drop
          (if (call $wat_emit_byte (local.get $body_off) (i32.const 0x1A)) (then (return (global.get $ERR_NO_MEM))))
          (local.set $body_off (i32.load (global.get $OFF_SCRATCH0)))
          (i32.store (global.get $OFF_SCRATCH0) (local.get $body_off))
          (i32.store (global.get $OFF_SCRATCH1) (local.get $pos))
          (return (i32.const 0))
        )
        (block $not_end
          (if (i32.ne (local.get $kw_len) (i32.const 3)) (then (br $not_end)))
          (if (i32.ne (local.get $b0) (i32.const 0x65)) (then (br $not_end)))  ;; 'e'
          (local.set $b1 (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (i32.add (local.get $kw_off) (i32.const 1)))))
          (local.set $b2 (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (i32.add (local.get $kw_off) (i32.const 2)))))
          (if (i32.ne (local.get $b1) (i32.const 0x6e)) (then (br $not_end)))  ;; 'n'
          (if (i32.ne (local.get $b2) (i32.const 0x64)) (then (br $not_end)))  ;; 'd'
          ;; end — emit end opcode
          (if (call $wat_emit_byte (local.get $body_off) (i32.const 0x0B)) (then (return (global.get $ERR_NO_MEM))))
          (local.set $body_off (i32.load (global.get $OFF_SCRATCH0)))
          (i32.store (global.get $OFF_SCRATCH0) (local.get $body_off))
          (i32.store (global.get $OFF_SCRATCH1) (local.get $pos))
          (return (i32.const 0))
        )

        ;; Unknown opcode
        (i32.store (i32.const 0x8C048) (i32.const 0x3100))
        (return (global.get $ERR_PARSE))
      )

  ;; ═════════════════════════════════════════════════════════════════════
  ;; Top-level declaration parsers
  ;; ═════════════════════════════════════════════════════════════════════

  ;; Parse (type ...) declaration.
  ;; Input: pos after "(type"
  ;; Output: scratch0 = new_pos, scratch1 = type_index
  (func $wat_parse_type_decl (param $pos i32) (result i32)
    (local $err i32) (local $tc i32) (local $base i32) (local $rc i32)
    (local $i i32) (local $vt i32) (local $b i32) (local $kw_off i32) (local $kw_len i32)
    (local $p0 i32) (local $p1 i32) (local $p2 i32) (local $p3 i32)

    ;; Skip optional $name identifier
    (if (call $wat_skip_ws (local.get $pos)) (then (return (global.get $ERR_PARSE))))
    (local.set $pos (i32.load (global.get $OFF_SCRATCH0)))
    (local.set $b (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos))))
    (if (i32.eq (local.get $b) (i32.const 0x24))  ;; '$'
      (then
        (if (call $wat_read_id (local.get $pos)) (then (return (global.get $ERR_PARSE))))
        (local.set $pos (i32.load (global.get $OFF_SCRATCH2)))
      )
    )

    ;; Expect "("
    (if (call $wat_skip_ws (local.get $pos)) (then (return (global.get $ERR_PARSE))))
    (local.set $pos (i32.load (global.get $OFF_SCRATCH0)))
    (local.set $b (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos))))
    (if (i32.ne (local.get $b) (i32.const 0x28))  ;; '('
      (then (return (global.get $ERR_PARSE)))
    )
    (local.set $pos (i32.add (local.get $pos) (i32.const 1)))

    ;; Expect "func"
    (if (call $wat_skip_ws (local.get $pos)) (then (return (global.get $ERR_PARSE))))
    (local.set $pos (i32.load (global.get $OFF_SCRATCH0)))
    (if (call $wat_read_kw (local.get $pos)) (then (return (global.get $ERR_PARSE))))
    (local.set $kw_off (i32.load (global.get $OFF_SCRATCH0)))
    (local.set $kw_len (i32.load (global.get $OFF_SCRATCH1)))
    (local.set $p0 (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $kw_off))))
    (if (i32.ne (local.get $kw_len) (i32.const 4))
      (then (return (global.get $ERR_PARSE)))
    )
    (if (i32.ne (local.get $p0) (i32.const 0x66))  ;; 'f'
      (then (return (global.get $ERR_PARSE)))
    )
    (local.set $p1 (i32.load8_u (i32.add (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $kw_off)) (i32.const 1))))
    (local.set $p2 (i32.load8_u (i32.add (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $kw_off)) (i32.const 2))))
    (local.set $p3 (i32.load8_u (i32.add (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $kw_off)) (i32.const 3))))
    (if (i32.ne (local.get $p1) (i32.const 0x75))  ;; 'u'
      (then (return (global.get $ERR_PARSE)))
    )
    (if (i32.ne (local.get $p2) (i32.const 0x6e))  ;; 'n'
      (then (return (global.get $ERR_PARSE)))
    )
    (if (i32.ne (local.get $p3) (i32.const 0x63))  ;; 'c'
      (then (return (global.get $ERR_PARSE)))
    )
    (local.set $pos (i32.load (global.get $OFF_SCRATCH2)))

    ;; Get type count
    (local.set $tc (i32.load (global.get $OFF_TYPE_COUNT)))
    (if (i32.ge_u (local.get $tc) (global.get $MAX_TYPES)) (then (return (global.get $ERR_PARSE))))
    (local.set $base (i32.add (global.get $OFF_TYPES_BUF) (i32.mul (local.get $tc) (global.get $SZ_TYPE))))

    ;; Store functype marker at offset 0 (not strictly needed but follows binary pattern)
    ;; The interpreter uses SZ_TYPE = 256 with structured fields, not this marker.

    ;; Parse (param ...) and (result ...)
    (local.set $rc (i32.const 0))
    (block $tlp
      (loop $tcont
        (if (call $wat_skip_ws (local.get $pos)) (then (return (global.get $ERR_PARSE))))
        (local.set $pos (i32.load (global.get $OFF_SCRATCH0)))
        (local.set $b (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos))))
        (if (i32.eq (local.get $b) (i32.const 0x29))  ;; ')'
          (then (local.set $pos (i32.add (local.get $pos) (i32.const 1))) (br $tlp))
        )
        ;; Expect "("
        (if (i32.ne (local.get $b) (i32.const 0x28))  ;; '('
          (then (return (global.get $ERR_PARSE)))
        )
        (local.set $pos (i32.add (local.get $pos) (i32.const 1)))
        ;; Read keyword: "param" or "result"
        (if (call $wat_skip_ws (local.get $pos)) (then (return (global.get $ERR_PARSE))))
        (local.set $pos (i32.load (global.get $OFF_SCRATCH0)))
        (if (call $wat_read_kw (local.get $pos)) (then (return (global.get $ERR_PARSE))))
        (local.set $kw_off (i32.load (global.get $OFF_SCRATCH0)))
        (local.set $kw_len (i32.load (global.get $OFF_SCRATCH1)))
        (local.set $pos (i32.load (global.get $OFF_SCRATCH2)))

        ;; Check if keyword is "param" (5 bytes)
        (local.set $p0 (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $kw_off))))
        (block $not_param
          (if (i32.ne (local.get $kw_len) (i32.const 5)) (then (br $not_param)))
          (if (i32.ne (local.get $p0) (i32.const 0x70)) (then (br $not_param)))  ;; 'p'
          (if (call $wat_kw_match_rest (local.get $kw_off) (i32.const 1) (i32.const 4)
                (i32.const 0x61) (i32.const 0x72) (i32.const 0x61) (i32.const 0x6d)  ;; aram
                (i32.const 0) (i32.const 0) (i32.const 0) (i32.const 0)
                (i32.const 0) (i32.const 0))
            (then (br $not_param))
          )
          ;; Read param types until ')'
          (block $plp
              (loop $pcont
                (if (call $wat_skip_ws (local.get $pos)) (then (return (global.get $ERR_PARSE))))
                (local.set $pos (i32.load (global.get $OFF_SCRATCH0)))
                (local.set $b (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos))))
                (if (i32.eq (local.get $b) (i32.const 0x29))  ;; ')'
                  (then (local.set $pos (i32.add (local.get $pos) (i32.const 1))) (br $plp))
                )
                ;; Skip optional $name
                (if (i32.eq (local.get $b) (i32.const 0x24))
                  (then
                    (if (call $wat_read_id (local.get $pos)) (then (return (global.get $ERR_PARSE))))
                    (local.set $pos (i32.load (global.get $OFF_SCRATCH2)))
                    (if (call $wat_skip_ws (local.get $pos)) (then (return (global.get $ERR_PARSE))))
                    (local.set $pos (i32.load (global.get $OFF_SCRATCH0)))
                  )
                )
                ;; Read value type
                (if (call $wat_read_kw (local.get $pos)) (then (return (global.get $ERR_PARSE))))
                (if (call $wat_valtype (i32.load (global.get $OFF_SCRATCH0)) (i32.load (global.get $OFF_SCRATCH1)))
                  (then (return (global.get $ERR_PARSE)))
                )
                (local.set $vt (i32.load (global.get $OFF_SCRATCH0)))
                ;; Store param type
                (if (i32.gt_u (local.get $rc) (i32.const 31)) (then (return (global.get $ERR_PARSE))))
                (i32.store8 (i32.add (local.get $base) (local.get $rc)) (local.get $vt))
                (local.set $rc (i32.add (local.get $rc) (i32.const 1)))
                (local.set $pos (i32.load (global.get $OFF_SCRATCH2)))
                (br $pcont)
              )
            )
            (br $tcont)
          )

        ;; Check if keyword is "result" (6 bytes)
        (block $not_result
          (if (i32.ne (local.get $kw_len) (i32.const 6)) (then (br $not_result)))
          (if (i32.ne (local.get $p0) (i32.const 0x72)) (then (br $not_result)))  ;; 'r'
          (if (call $wat_kw_match_rest (local.get $kw_off) (i32.const 1) (i32.const 5)
                (i32.const 0x65) (i32.const 0x73) (i32.const 0x75) (i32.const 0x6c)  ;; esul
                (i32.const 0x74) (i32.const 0) (i32.const 0) (i32.const 0)  ;; t
                (i32.const 0) (i32.const 0))
            (then (br $not_result))
          )
          ;; Read result types until ')'
          (local.set $i (i32.const 0))
            (block $rlp
              (loop $rcont
                (if (call $wat_skip_ws (local.get $pos)) (then (return (global.get $ERR_PARSE))))
                (local.set $pos (i32.load (global.get $OFF_SCRATCH0)))
                (local.set $b (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos))))
                (if (i32.eq (local.get $b) (i32.const 0x29))  ;; ')'
                  (then (local.set $pos (i32.add (local.get $pos) (i32.const 1))) (br $rlp))
                )
                ;; Read value type
                (if (call $wat_read_kw (local.get $pos)) (then (return (global.get $ERR_PARSE))))
                (if (call $wat_valtype (i32.load (global.get $OFF_SCRATCH0)) (i32.load (global.get $OFF_SCRATCH1)))
                  (then (return (global.get $ERR_PARSE)))
                )
                (local.set $vt (i32.load (global.get $OFF_SCRATCH0)))
                ;; Store result type
                (if (i32.gt_u (local.get $i) (i32.const 3)) (then (return (global.get $ERR_PARSE))))
                (i32.store8 (i32.add (local.get $base) (i32.const 132)) (local.get $vt))  ;; result_types at +132
                (local.set $i (i32.add (local.get $i) (i32.const 1)))
                (local.set $pos (i32.load (global.get $OFF_SCRATCH2)))
                (br $rcont)
              )
            )
            ;; Store result_count at +136
            (i32.store16 (i32.add (local.get $base) (i32.const 136)) (local.get $i))
            ;; Expect closing ')'
            (if (call $wat_skip_ws (local.get $pos)) (then (return (global.get $ERR_PARSE))))
            (local.set $pos (i32.load (global.get $OFF_SCRATCH0)))
            (local.set $b (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos))))
            (if (i32.ne (local.get $b) (i32.const 0x29)) (then (return (global.get $ERR_PARSE))))
            (local.set $pos (i32.add (local.get $pos) (i32.const 1)))
            (br $tlp)
        )
        (return (global.get $ERR_PARSE))
      )
    )

    ;; Store param_count at +128
    (i32.store16 (i32.add (local.get $base) (i32.const 128)) (local.get $rc))

    ;; Increment type count
    (i32.store (global.get $OFF_TYPE_COUNT) (i32.add (local.get $tc) (i32.const 1)))

    (i32.store (global.get $OFF_SCRATCH0) (local.get $pos))
    (i32.store (global.get $OFF_SCRATCH1) (local.get $tc))
    (return (global.get $OK))
  )

  ;; ═════════════════════════════════════════════════════════════════════
  ;; Parse (func ...) declaration (inline)
  ;; Input: pos after "(func"
  ;; Output: scratch0 = new_pos
  ;; ═════════════════════════════════════════════════════════════════════
  (func $wat_parse_func_decl (param $pos i32) (result i32)
    (local $err i32) (local $b i32) (local $kw_off i32) (local $kw_len i32)
    (local $tc i32) (local $base i32) (local $rc i32) (local $i i32) (local $vt i32)
    (local $fc i32) (local $code_i i32)
    (local $export_name_ptr i32) (local $export_name_len i32)
    (local $dst i32) (local $body_len i32)
    (local $has_export i32)
    (local $p0 i32) (local $p1 i32) (local $p2 i32) (local $p3 i32)
    (local $result_count i32)
    (local $decoded_start i32) (local $decoded_count i32)
    (local $body_off i32)

    ;; ── Skip optional $name ──
    (if (call $wat_skip_ws (local.get $pos)) (then (return (global.get $ERR_PARSE))))
    (local.set $pos (i32.load (global.get $OFF_SCRATCH0)))
    (local.set $b (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos))))
    (if (i32.eq (local.get $b) (i32.const 0x24))  ;; '$'
      (then
        (if (call $wat_read_id (local.get $pos)) (then (return (global.get $ERR_PARSE))))
        (local.set $pos (i32.load (global.get $OFF_SCRATCH2)))
      )
    )

    ;; ── Check for (export "name") ──
    (local.set $has_export (i32.const 0))
    (block $export_check_done
      ;; Look for '('
      (if (call $wat_skip_ws (local.get $pos)) (then (return (global.get $ERR_PARSE))))
      (local.set $pos (i32.load (global.get $OFF_SCRATCH0)))
      (local.set $b (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos))))
      (if (i32.ne (local.get $b) (i32.const 0x28)) (then (br $export_check_done)))  ;; not '('
      (local.set $pos (i32.add (local.get $pos) (i32.const 1)))
      ;; Read keyword
      (if (call $wat_skip_ws (local.get $pos)) (then (return (global.get $ERR_PARSE))))
      (local.set $pos (i32.load (global.get $OFF_SCRATCH0)))
      (if (call $wat_read_kw (local.get $pos)) (then (return (global.get $ERR_PARSE))))
      (local.set $kw_off (i32.load (global.get $OFF_SCRATCH0)))
      (local.set $kw_len (i32.load (global.get $OFF_SCRATCH1)))
      (local.set $p0 (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $kw_off))))
      ;; Check "export" (6 bytes)
      (if (i32.ne (local.get $kw_len) (i32.const 6)) (then (return (global.get $ERR_PARSE))))
      (if (i32.ne (local.get $p0) (i32.const 0x65)) (then (return (global.get $ERR_PARSE))))
      (if (call $wat_kw_match_rest (local.get $kw_off) (i32.const 1) (i32.const 5)
            (i32.const 0x78) (i32.const 0x70) (i32.const 0x6f) (i32.const 0x72)
            (i32.const 0x74) (i32.const 0) (i32.const 0) (i32.const 0)
            (i32.const 0) (i32.const 0))
        (then (return (global.get $ERR_PARSE)))
      )
      (local.set $pos (i32.load (global.get $OFF_SCRATCH2)))

      ;; Parse export name string: "name"
      (if (call $wat_skip_ws (local.get $pos)) (then (return (global.get $ERR_PARSE))))
      (local.set $pos (i32.load (global.get $OFF_SCRATCH0)))
      (local.set $b (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos))))
      (if (i32.ne (local.get $b) (i32.const 0x22)) (then (return (global.get $ERR_PARSE))))  ;; '"'
      (local.set $pos (i32.add (local.get $pos) (i32.const 1)))
      ;; Read name bytes into names buffer
      (local.set $export_name_ptr (i32.load (global.get $OFF_NAMES_PTR)))
      (local.set $export_name_len (i32.const 0))
      (block $ename_done
        (loop $ename_lp
          (local.set $b (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos))))
          (if (i32.eq (local.get $b) (i32.const 0x22))  ;; closing '"'
            (then
              (local.set $pos (i32.add (local.get $pos) (i32.const 1)))
              (br $ename_done)
            )
          )
          (i32.store8 (i32.add (local.get $export_name_ptr) (local.get $export_name_len)) (local.get $b))
          (local.set $export_name_len (i32.add (local.get $export_name_len) (i32.const 1)))
          (local.set $pos (i32.add (local.get $pos) (i32.const 1)))
          (br $ename_lp)
        )
      )
      ;; Advance names buffer pointer
      (i32.store (global.get $OFF_NAMES_PTR) (i32.add (local.get $export_name_ptr) (local.get $export_name_len)))
      ;; Expect ')'
      (if (call $wat_skip_ws (local.get $pos)) (then (return (global.get $ERR_PARSE))))
      (local.set $pos (i32.load (global.get $OFF_SCRATCH0)))
      (local.set $b (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos))))
      (if (i32.ne (local.get $b) (i32.const 0x29)) (then (return (global.get $ERR_PARSE))))  ;; ')'
      (local.set $pos (i32.add (local.get $pos) (i32.const 1)))
      (local.set $has_export (i32.const 1))
    )

    ;; ── Parse inline type: (param ...) and (result ...) ──
    (local.set $tc (i32.load (global.get $OFF_TYPE_COUNT)))
    (if (i32.ge_u (local.get $tc) (global.get $MAX_TYPES)) (then (return (global.get $ERR_PARSE))))
    (local.set $base (i32.add (global.get $OFF_TYPES_BUF) (i32.mul (local.get $tc) (global.get $SZ_TYPE))))
    (local.set $rc (i32.const 0))

    (block $type_done
      (loop $type_lp
        (if (call $wat_skip_ws (local.get $pos)) (then (return (global.get $ERR_PARSE))))
        (local.set $pos (i32.load (global.get $OFF_SCRATCH0)))
        (local.set $b (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos))))
        ;; If we see `)` we're done with type declarations, go to body
        (if (i32.eq (local.get $b) (i32.const 0x29)) (then (local.set $pos (i32.add (local.get $pos) (i32.const 1))) (br $type_done)))
        ;; Must be '('
        (if (i32.ne (local.get $b) (i32.const 0x28)) (then (br $type_done)))  ;; not '(', assume body starts
        (local.set $pos (i32.add (local.get $pos) (i32.const 1)))
        ;; Read keyword
        (if (call $wat_skip_ws (local.get $pos)) (then (return (global.get $ERR_PARSE))))
        (local.set $pos (i32.load (global.get $OFF_SCRATCH0)))
        (if (call $wat_read_kw (local.get $pos)) (then (return (global.get $ERR_PARSE))))
        (local.set $kw_off (i32.load (global.get $OFF_SCRATCH0)))
        (local.set $kw_len (i32.load (global.get $OFF_SCRATCH1)))
        (local.set $pos (i32.load (global.get $OFF_SCRATCH2)))
        (local.set $p0 (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $kw_off))))

        ;; "param" (5 bytes)
        (block $not_p
          (if (i32.ne (local.get $kw_len) (i32.const 5)) (then (br $not_p)))
          (if (i32.ne (local.get $p0) (i32.const 0x70)) (then (br $not_p)))
          (if (call $wat_kw_match_rest (local.get $kw_off) (i32.const 1) (i32.const 4)
                (i32.const 0x61) (i32.const 0x72) (i32.const 0x61) (i32.const 0x6d)
                (i32.const 0) (i32.const 0) (i32.const 0) (i32.const 0)
                (i32.const 0) (i32.const 0))
            (then (br $not_p))
          )
          ;; Param types until ')'
          (block $param_done
            (loop $param_lp
              (if (call $wat_skip_ws (local.get $pos)) (then (return (global.get $ERR_PARSE))))
              (local.set $pos (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $b (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos))))
              (if (i32.eq (local.get $b) (i32.const 0x29)) (then (local.set $pos (i32.add (local.get $pos) (i32.const 1))) (br $param_done)))  ;; ')'
              ;; Read value type
              (if (call $wat_read_kw (local.get $pos)) (then (return (global.get $ERR_PARSE))))
              (if (call $wat_valtype (i32.load (global.get $OFF_SCRATCH0)) (i32.load (global.get $OFF_SCRATCH1)))
                (then (return (global.get $ERR_PARSE)))
              )
              (local.set $vt (i32.load (global.get $OFF_SCRATCH0)))
              (i32.store8 (i32.add (local.get $base) (local.get $rc)) (local.get $vt))
              (local.set $rc (i32.add (local.get $rc) (i32.const 1)))
              (local.set $pos (i32.load (global.get $OFF_SCRATCH2)))
              (br $param_lp)
            )
          )
          (br $type_lp)
        )

        ;; "result" (6 bytes)
        (block $not_r
          (if (i32.ne (local.get $kw_len) (i32.const 6)) (then (br $not_r)))
          (if (i32.ne (local.get $p0) (i32.const 0x72)) (then (br $not_r)))
          (if (call $wat_kw_match_rest (local.get $kw_off) (i32.const 1) (i32.const 5)
                (i32.const 0x65) (i32.const 0x73) (i32.const 0x75) (i32.const 0x6c)
                (i32.const 0x74) (i32.const 0) (i32.const 0) (i32.const 0)
                (i32.const 0) (i32.const 0))
            (then (br $not_r))
          )
          ;; Result types until ')'
          (local.set $result_count (i32.const 0))
          (block $res_done
            (loop $res_lp
              (if (call $wat_skip_ws (local.get $pos)) (then (return (global.get $ERR_PARSE))))
              (local.set $pos (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $b (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos))))
              (if (i32.eq (local.get $b) (i32.const 0x29)) (then (local.set $pos (i32.add (local.get $pos) (i32.const 1))) (br $res_done)))
              ;; Read value type
              (if (call $wat_read_kw (local.get $pos)) (then (return (global.get $ERR_PARSE))))
              (if (call $wat_valtype (i32.load (global.get $OFF_SCRATCH0)) (i32.load (global.get $OFF_SCRATCH1)))
                (then (return (global.get $ERR_PARSE)))
              )
              (local.set $vt (i32.load (global.get $OFF_SCRATCH0)))
              (i32.store8 (i32.add (local.get $base) (i32.const 132)) (local.get $vt))
              (local.set $result_count (i32.add (local.get $result_count) (i32.const 1)))
              (local.set $pos (i32.load (global.get $OFF_SCRATCH2)))
              (br $res_lp)
            )
          )
          (i32.store16 (i32.add (local.get $base) (i32.const 136)) (local.get $result_count))
          (br $type_lp)
        )

        ;; Not param or result — unread the open paren and go to body
        (local.set $pos (i32.sub (local.get $pos) (i32.const 1)))
        (br $type_done)
      )
    )

    ;; Store param_count at +128
    (i32.store16 (i32.add (local.get $base) (i32.const 128)) (local.get $rc))
    ;; Increment type count
    (i32.store (global.get $OFF_TYPE_COUNT) (i32.add (local.get $tc) (i32.const 1)))

    ;; ── Store function entry ──
    (local.set $fc (i32.load (global.get $OFF_FUNCTION_COUNT)))
    (i32.store (i32.add (global.get $OFF_FUNCTIONS_BUF) (i32.mul (local.get $fc) (global.get $SZ_FUNC))) (local.get $tc))

    ;; ── Parse body ──
    (local.set $body_off (i32.const 0))
    (block $body_done
      (loop $body_loop
        (local.set $err (call $wat_parse_body (local.get $pos) (local.get $body_off)))
        (if (i32.lt_s (local.get $err) (i32.const 0)) (then (br $body_done)))  ;; DONE
        (if (local.get $err)
          (then
            (i32.store (global.get $OFF_WAT_DBG) (i32.const 0x2001))
            (i32.store (global.get $OFF_WAT_TMP) (local.get $err))
            (return (global.get $ERR_PARSE))
          )
        )
        ;; CONTINUE
        (local.set $body_off (i32.load (global.get $OFF_SCRATCH0)))
        (local.set $pos (i32.load (global.get $OFF_SCRATCH1)))
        (br $body_loop)
      )
    )
    ;; After $wat_parse_body: scratch0=decoded_start, scratch1=decoded_count
    (local.set $decoded_start (i32.load (global.get $OFF_SCRATCH0)))
    (local.set $decoded_count (i32.load (global.get $OFF_SCRATCH1)))
    (local.set $body_len (i32.load (global.get $OFF_WAT_TMP)))
    (local.set $pos (i32.load (global.get $OFF_SCRATCH2)))

    ;; ── Store code entry ──
    (local.set $code_i (i32.load (global.get $OFF_CODE_COUNT)))
    (local.set $base (i32.add (global.get $OFF_CODE_BUF) (i32.mul (local.get $code_i) (global.get $SZ_CODE))))
    (i32.store (i32.add (local.get $base) (i32.const 0)) (i32.const 0))  ;; body_offset = 0
    (i32.store (i32.add (local.get $base) (i32.const 8)) (local.get $body_len))  ;; body_len
    (i32.store (i32.add (local.get $base) (i32.const 16)) (i32.const 0))  ;; local_count = 0
    (i32.store (i32.add (local.get $base) (i32.const 24)) (local.get $decoded_start))  ;; decoded_start
    (i32.store (i32.add (local.get $base) (i32.const 32)) (local.get $decoded_count))  ;; decoded_count

    ;; ── Store export entry (if export clause present) ──
    (if (local.get $has_export)
      (then
        (local.set $i (i32.load (global.get $OFF_EXPORT_COUNT)))
        (local.set $base (i32.add (global.get $OFF_EXPORTS_BUF) (i32.mul (local.get $i) (global.get $SZ_EXPORT))))
        (i32.store (i32.add (local.get $base) (i32.const 0)) (local.get $export_name_ptr))  ;; name_ptr
        (i32.store (i32.add (local.get $base) (i32.const 8)) (local.get $export_name_len))  ;; name_len
        (i32.store8 (i32.add (local.get $base) (i32.const 16)) (i32.const 0x00))  ;; kind = func
        (i32.store (i32.add (local.get $base) (i32.const 24)) (local.get $fc))  ;; index
        (i32.store (global.get $OFF_EXPORT_COUNT) (i32.add (local.get $i) (i32.const 1)))
      )
    )

    ;; ── Update counters ──
    (i32.store (global.get $OFF_FUNCTION_COUNT) (i32.add (local.get $fc) (i32.const 1)))
    (i32.store (global.get $OFF_CODE_COUNT) (i32.add (local.get $code_i) (i32.const 1)))

    ;; ── Skip closing ')' of func — $wat_parse_body already consumed it ──
    (i32.store (global.get $OFF_SCRATCH0) (local.get $pos))
    (return (global.get $OK))
  )

  (global $OFF_WAT_DBG i32 (i32.const 0x8C020))

  ;; ═════════════════════════════════════════════════════════════════════
  ;; Main WAT module parser
  ;; Input: WAT source at wasm_ptr (we'll save it), length = wasm_len
  ;; ═════════════════════════════════════════════════════════════════════

  (func $wat_parse_module (param $wat_ptr i32) (param $wat_len i32) (result i32)
    (local $pos i32) (local $err i32) (local $b i32)
    (local $func_i i32) (local $func_cnt i32) (local $type_idx i32)
    (local $code_i i32) (local $code_cnt i32)
    (local $export_i i32) (local $export_cnt i32)
    (local $import_i i32) (local $import_cnt i32)
    (local $global_i i32) (local $global_cnt i32)
    (local $start_func i32)
    (local $kw_off i32) (local $kw_len i32) (local $open_parens i32)
    (local $b0 i32) (local $b1 i32) (local $b2 i32) (local $b3 i32)

    (i32.store (global.get $OFF_WAT_DBG) (i32.const 1))  ;; debug: entered

    ;; Save original wasm_ptr/len
    (i32.store (global.get $OFF_WAT_SAV_PTR) (i32.load (global.get $OFF_WASM_PTR)))
    (i32.store (global.get $OFF_WAT_SAV_LEN) (i32.load (global.get $OFF_WASM_LEN)))

    ;; Store WAT source as the "wasm" pointer temporarily for lexer access
    (i32.store (global.get $OFF_WAT_PTR) (local.get $wat_ptr))
    (i32.store (global.get $OFF_WAT_LEN) (local.get $wat_len))
    (local.set $pos (i32.const 0))

    ;; Clear symbol table
    (i32.store (global.get $OFF_WAT_SYM) (i32.const 0))

    ;; Clear state counters (reset module state fully)
    (i32.store (global.get $OFF_TYPE_COUNT) (i32.const 0))
    (i32.store (global.get $OFF_IMPORT_COUNT) (i32.const 0))
    (i32.store (global.get $OFF_FUNCTION_COUNT) (i32.const 0))
    (i32.store (global.get $OFF_CODE_COUNT) (i32.const 0))
    (i32.store (global.get $OFF_EXPORT_COUNT) (i32.const 0))
    (i32.store (global.get $OFF_GLOBAL_COUNT) (i32.const 0))
    (i32.store (global.get $OFF_TABLE_HAS) (i32.const 0))
    (i32.store (global.get $OFF_MEM_MIN) (i32.const 0))
    (i32.store (global.get $OFF_START_FUNC) (i32.const -1))
    (i32.store (global.get $OFF_ELEM_COUNT) (i32.const 0))
    (i32.store (global.get $OFF_DATA_COUNT) (i32.const 0))
    (i32.store (global.get $OFF_DECODED_COUNT) (i32.const 0))
    (i32.store8 (global.get $OFF_EXEC_MOD_VALID) (i32.const 0))

    ;; Reset names pointer
    (i32.store (global.get $OFF_NAMES_PTR) (global.get $OFF_NAMES_BUF))

    (i32.store (global.get $OFF_WAT_DBG) (i32.const 2))

    ;; Skip whitespace
    (if (call $wat_skip_ws (local.get $pos)) (then (i32.store (global.get $OFF_WAT_DBG) (i32.const 10)) (return (global.get $ERR_PARSE))))
    (local.set $pos (i32.load (global.get $OFF_SCRATCH0)))

    (i32.store (global.get $OFF_WAT_DBG) (i32.const 3))

    ;; Expect "("
    (i32.store (global.get $OFF_WAT_TMP0) (i32.load (global.get $OFF_WAT_PTR)))
    (i32.store (global.get $OFF_WAT_TMP1) (local.get $pos))
    (i32.store (global.get $OFF_WAT_TMP1) (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos)))
    (local.set $b (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos))))
    (i32.store8 (global.get $OFF_WAT_DBG) (local.get $b))
    (i32.store8 (global.get $OFF_WAT_TMP0) (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos))))
    (if (i32.ne (local.get $b) (i32.const 0x28))
      (then (i32.store (global.get $OFF_WAT_DBG) (i32.const 0x1100)) (return (global.get $ERR_PARSE)))
    )
    (local.set $pos (i32.add (local.get $pos) (i32.const 1)))

    (i32.store (global.get $OFF_WAT_DBG) (i32.const 4))

    ;; Expect "module"
    (if (call $wat_skip_ws (local.get $pos)) (then (i32.store (global.get $OFF_WAT_DBG) (i32.const 12)) (return (global.get $ERR_PARSE))))
    (local.set $pos (i32.load (global.get $OFF_SCRATCH0)))

    (i32.store (global.get $OFF_WAT_DBG) (i32.const 5))

    (if (call $wat_read_kw (local.get $pos)) (then (i32.store (global.get $OFF_WAT_DBG) (i32.const 13)) (return (global.get $ERR_PARSE))))

    (i32.store (global.get $OFF_WAT_DBG) (i32.const 6))

    (local.set $kw_off (i32.load (global.get $OFF_SCRATCH0)))
    (local.set $kw_len (i32.load (global.get $OFF_SCRATCH1)))
    (if (i32.ne (local.get $kw_len) (i32.const 6))
      (then (i32.store (global.get $OFF_WAT_DBG) (i32.const 14)) (return (global.get $ERR_PARSE)))
    )

    (i32.store (global.get $OFF_WAT_DBG) (i32.const 7))

    (if (i32.ne (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $kw_off))) (i32.const 0x6D))
      (then (i32.store (global.get $OFF_WAT_DBG) (i32.const 15)) (return (global.get $ERR_PARSE)))  ;; 'm'
    )

    (i32.store (global.get $OFF_WAT_DBG) (i32.const 8))

    (if (call $wat_kw_match_rest (local.get $kw_off) (i32.const 1) (i32.const 5)
          (i32.const 0x6F) (i32.const 0x64) (i32.const 0x75) (i32.const 0x6C)
          (i32.const 0x65) (i32.const 0) (i32.const 0) (i32.const 0)
          (i32.const 0) (i32.const 0))
      (then (i32.store (global.get $OFF_WAT_DBG) (i32.const 16)) (return (global.get $ERR_PARSE)))
    )

    (i32.store (global.get $OFF_WAT_DBG) (i32.const 9))

    (local.set $pos (i32.load (global.get $OFF_SCRATCH2)))

    (i32.store (global.get $OFF_WAT_DBG) (i32.const 20))

    ;; ═══ Pass 1: Scan all declarations, register names, count entities ═══
    (block $pass1_done
      (loop $pass1
        (if (call $wat_skip_ws (local.get $pos)) (then (i32.store (global.get $OFF_WAT_DBG) (i32.const 21)) (return (global.get $ERR_PARSE))))
        (local.set $pos (i32.load (global.get $OFF_SCRATCH0)))
        (local.set $b (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos))))
        ;; Closing paren = end of module
        (if (i32.eq (local.get $b) (i32.const 0x29))  ;; ')'
          (then
            (local.set $pos (i32.add (local.get $pos) (i32.const 1)))
            (br $pass1_done)
          )
        )

        ;; Must be "("
        (if (i32.ne (local.get $b) (i32.const 0x28))  ;; '('
          (then (i32.store (global.get $OFF_WAT_DBG) (i32.const 22)) (return (global.get $ERR_PARSE)))
        )
        (local.set $pos (i32.add (local.get $pos) (i32.const 1)))

        ;; Read keyword
        (if (call $wat_skip_ws (local.get $pos)) (then (i32.store (global.get $OFF_WAT_DBG) (i32.const 23)) (return (global.get $ERR_PARSE))))
        (local.set $pos (i32.load (global.get $OFF_SCRATCH0)))
        (if (call $wat_read_kw (local.get $pos)) (then (i32.store (global.get $OFF_WAT_DBG) (i32.const 24)) (return (global.get $ERR_PARSE))))
        (local.set $kw_off (i32.load (global.get $OFF_SCRATCH0)))
        (local.set $kw_len (i32.load (global.get $OFF_SCRATCH1)))
        (local.set $pos (i32.load (global.get $OFF_SCRATCH2)))
        (local.set $b0 (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $kw_off))))

        ;; ── Count declarations by keyword ──
        (block $kw_matched
          (block $not_type
            (if (i32.ne (local.get $kw_len) (i32.const 4)) (then (br $not_type)))
            (if (i32.ne (local.get $b0) (i32.const 0x74)) (then (br $not_type)))  ;; 't'
            (if (call $wat_kw_match_rest (local.get $kw_off) (i32.const 1) (i32.const 3)
                  (i32.const 0x79) (i32.const 0x70) (i32.const 0x65) (i32.const 0)
                  (i32.const 0) (i32.const 0) (i32.const 0) (i32.const 0)
                  (i32.const 0) (i32.const 0))
              (then (br $not_type))
            )
            (i32.store (global.get $OFF_TYPE_COUNT) (i32.add (i32.load (global.get $OFF_TYPE_COUNT)) (i32.const 1)))
            (br $kw_matched)
          )
          (block $not_func
            (if (i32.ne (local.get $kw_len) (i32.const 4)) (then (br $not_func)))
            (if (i32.ne (local.get $b0) (i32.const 0x66)) (then (br $not_func)))  ;; 'f'
            (if (call $wat_kw_match_rest (local.get $kw_off) (i32.const 1) (i32.const 3)
                  (i32.const 0x75) (i32.const 0x6e) (i32.const 0x63) (i32.const 0)
                  (i32.const 0) (i32.const 0) (i32.const 0) (i32.const 0)
                  (i32.const 0) (i32.const 0))
              (then (br $not_func))
            )
            (i32.store (global.get $OFF_FUNCTION_COUNT) (i32.add (i32.load (global.get $OFF_FUNCTION_COUNT)) (i32.const 1)))
            (i32.store (global.get $OFF_CODE_COUNT) (i32.add (i32.load (global.get $OFF_CODE_COUNT)) (i32.const 1)))
            (br $kw_matched)
          )
          (block $not_export
            (if (i32.ne (local.get $kw_len) (i32.const 6)) (then (br $not_export)))
            (if (i32.ne (local.get $b0) (i32.const 0x65)) (then (br $not_export)))  ;; 'e'
            (if (call $wat_kw_match_rest (local.get $kw_off) (i32.const 1) (i32.const 5)
                  (i32.const 0x78) (i32.const 0x70) (i32.const 0x6f) (i32.const 0x72)  ;; xpor
                  (i32.const 0x74) (i32.const 0) (i32.const 0) (i32.const 0)
                  (i32.const 0) (i32.const 0))
              (then (br $not_export))
            )
            (i32.store (global.get $OFF_EXPORT_COUNT) (i32.add (i32.load (global.get $OFF_EXPORT_COUNT)) (i32.const 1)))
            (br $kw_matched)
          )
          (block $not_memory
            (if (i32.ne (local.get $kw_len) (i32.const 6)) (then (br $not_memory)))
            (if (i32.ne (local.get $b0) (i32.const 0x6d)) (then (br $not_memory)))  ;; 'm'
            (if (call $wat_kw_match_rest (local.get $kw_off) (i32.const 1) (i32.const 5)
                  (i32.const 0x65) (i32.const 0x6d) (i32.const 0x6f) (i32.const 0x72)  ;; emor
                  (i32.const 0x79) (i32.const 0) (i32.const 0) (i32.const 0)
                  (i32.const 0) (i32.const 0))
              (then (br $not_memory))
            )
            (i32.store (global.get $OFF_MEM_MIN) (i32.const 1))
            (br $kw_matched)
          )
          (block $not_start
            (if (i32.ne (local.get $kw_len) (i32.const 5)) (then (br $not_start)))
            (if (i32.ne (local.get $b0) (i32.const 0x73)) (then (br $not_start)))  ;; 's'
            (if (call $wat_kw_match_rest (local.get $kw_off) (i32.const 1) (i32.const 4)
                  (i32.const 0x74) (i32.const 0x61) (i32.const 0x72) (i32.const 0x74)  ;; tart
                  (i32.const 0) (i32.const 0) (i32.const 0) (i32.const 0)
                  (i32.const 0) (i32.const 0))
              (then (br $not_start))
            )
            (br $kw_matched)
          )
          (block $not_data
            (if (i32.ne (local.get $kw_len) (i32.const 4)) (then (br $not_data)))
            (if (i32.ne (local.get $b0) (i32.const 0x64)) (then (br $not_data)))  ;; 'd'
            (if (call $wat_kw_match_rest (local.get $kw_off) (i32.const 1) (i32.const 3)
                  (i32.const 0x61) (i32.const 0x74) (i32.const 0x61) (i32.const 0)
                  (i32.const 0) (i32.const 0) (i32.const 0) (i32.const 0)
                  (i32.const 0) (i32.const 0))
              (then (br $not_data))
            )
            (i32.store (global.get $OFF_DATA_COUNT) (i32.add (i32.load (global.get $OFF_DATA_COUNT)) (i32.const 1)))
            (br $kw_matched)
          )
        )

        ;; Find matching closing paren and skip entire declaration
        (local.set $open_parens (i32.const 1))
        (block $skip_decl
          (loop $skip_lp
            (if (i32.ge_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos))
                          (i32.add (i32.load (global.get $OFF_WAT_PTR)) (i32.load (global.get $OFF_WAT_LEN))))
              (then (i32.store (global.get $OFF_WAT_DBG) (i32.const 25)) (return (global.get $ERR_PARSE)))
            )
            (local.set $b (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos))))
            (if (i32.eq (local.get $b) (i32.const 0x28))  ;; '('
              (then (local.set $open_parens (i32.add (local.get $open_parens) (i32.const 1))))
            )
            (if (i32.eq (local.get $b) (i32.const 0x29))  ;; ')'
              (then
                (local.set $open_parens (i32.sub (local.get $open_parens) (i32.const 1)))
                (if (i32.eqz (local.get $open_parens))
                  (then
                    (local.set $pos (i32.add (local.get $pos) (i32.const 1)))
                    (br $skip_decl)
                  )
                )
              )
            )
            (local.set $pos (i32.add (local.get $pos) (i32.const 1)))
            (br $skip_lp)
          )
        )
        (br $pass1)
      )
    )

    (i32.store (global.get $OFF_WAT_DBG) (i32.const 30))

    ;; ═══ Pass 2: Parse declarations into state buffers ═══
    (local.set $pos (i32.const 0))
    (if (call $wat_skip_ws (local.get $pos)) (then (i32.store (global.get $OFF_WAT_DBG) (i32.const 31)) (return (global.get $ERR_PARSE))))
    (local.set $pos (i32.load (global.get $OFF_SCRATCH0)))

    ;; skip "(module"
    (local.set $b (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos))))
    (if (i32.ne (local.get $b) (i32.const 0x28)) (then (i32.store (global.get $OFF_WAT_DBG) (i32.const 32)) (return (global.get $ERR_PARSE))))
    (local.set $pos (i32.add (local.get $pos) (i32.const 1)))

    (if (call $wat_skip_ws (local.get $pos)) (then (i32.store (global.get $OFF_WAT_DBG) (i32.const 33)) (return (global.get $ERR_PARSE))))
    (local.set $pos (i32.load (global.get $OFF_SCRATCH0)))
    (if (call $wat_read_kw (local.get $pos)) (then (i32.store (global.get $OFF_WAT_DBG) (i32.const 34)) (return (global.get $ERR_PARSE))))
    (local.set $pos (i32.load (global.get $OFF_SCRATCH2)))

    ;; Reset decl counters for pass 2
    (i32.store (global.get $OFF_TYPE_COUNT) (i32.const 0))
    (i32.store (global.get $OFF_FUNCTION_COUNT) (i32.const 0))
    (i32.store (global.get $OFF_CODE_COUNT) (i32.const 0))
    (i32.store (global.get $OFF_EXPORT_COUNT) (i32.const 0))

    (block $pass2_done
      (loop $pass2
        (if (call $wat_skip_ws (local.get $pos)) (then (i32.store (global.get $OFF_WAT_DBG) (i32.const 35)) (return (global.get $ERR_PARSE))))
        (local.set $pos (i32.load (global.get $OFF_SCRATCH0)))
        (local.set $b (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos))))
        (if (i32.eq (local.get $b) (i32.const 0x29))  ;; ')'
          (then (br $pass2_done))
        )
        ;; Must be "("
        (if (i32.ne (local.get $b) (i32.const 0x28))  ;; '('
          (then (i32.store (global.get $OFF_WAT_DBG) (i32.const 36)) (return (global.get $ERR_PARSE)))
        )
        (local.set $pos (i32.add (local.get $pos) (i32.const 1)))
        ;; Read keyword
        (if (call $wat_skip_ws (local.get $pos)) (then (i32.store (global.get $OFF_WAT_DBG) (i32.const 37)) (return (global.get $ERR_PARSE))))
        (local.set $pos (i32.load (global.get $OFF_SCRATCH0)))
        (if (call $wat_read_kw (local.get $pos)) (then (i32.store (global.get $OFF_WAT_DBG) (i32.const 38)) (return (global.get $ERR_PARSE))))
        (local.set $kw_off (i32.load (global.get $OFF_SCRATCH0)))
        (local.set $kw_len (i32.load (global.get $OFF_SCRATCH1)))
        (local.set $pos (i32.load (global.get $OFF_SCRATCH2)))
        (local.set $b0 (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $kw_off))))

        (block $kw2_fallthrough
          (if (i32.eq (local.get $kw_len) (i32.const 4))
            (then
              (if (i32.eq (local.get $b0) (i32.const 0x74))  ;; 't'
                (then
                  (if (call $wat_kw_match_rest (local.get $kw_off) (i32.const 1) (i32.const 3)
                        (i32.const 0x79) (i32.const 0x70) (i32.const 0x65) (i32.const 0)
                        (i32.const 0) (i32.const 0) (i32.const 0) (i32.const 0)
                        (i32.const 0) (i32.const 0))
                    (then (br $kw2_fallthrough))
                  )
                  (if (call $wat_parse_type_decl (local.get $pos))
                    (then (return (global.get $ERR_PARSE)))
                  )
                  (local.set $pos (i32.load (global.get $OFF_SCRATCH0)))
                  (br $pass2)
                )
              )
              (if (i32.eq (local.get $b0) (i32.const 0x66))  ;; 'f'
                (then
                  (if (call $wat_kw_match_rest (local.get $kw_off) (i32.const 1) (i32.const 3)
                        (i32.const 0x75) (i32.const 0x6e) (i32.const 0x63) (i32.const 0)
                        (i32.const 0) (i32.const 0) (i32.const 0) (i32.const 0)
                        (i32.const 0) (i32.const 0))
                    (then (br $kw2_fallthrough))
                  )
                  ;; Parse function decl inline
                  (if (call $wat_parse_func_decl (local.get $pos))
                    (then (return (global.get $ERR_PARSE)))
                  )
                  (local.set $pos (i32.load (global.get $OFF_SCRATCH0)))
                  (br $pass2)
                )
              )
            )
          )
        )

        ;; Unknown declaration — skip it
        (local.set $open_parens (i32.const 1))
        (block $skip2
          (loop $skip2_lp
            (if (i32.ge_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos))
                          (i32.add (i32.load (global.get $OFF_WAT_PTR)) (i32.load (global.get $OFF_WAT_LEN))))
              (then (return (global.get $ERR_PARSE)))
            )
            (local.set $b (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos))))
            (if (i32.eq (local.get $b) (i32.const 0x28)) (then (local.set $open_parens (i32.add (local.get $open_parens) (i32.const 1)))))
            (if (i32.eq (local.get $b) (i32.const 0x29))
              (then
                (local.set $open_parens (i32.sub (local.get $open_parens) (i32.const 1)))
                (if (i32.eqz (local.get $open_parens))
                  (then (local.set $pos (i32.add (local.get $pos) (i32.const 1))) (br $skip2))
                )
              )
            )
            (local.set $pos (i32.add (local.get $pos) (i32.const 1)))
            (br $skip2_lp)
          )
        )
        (br $pass2)
      )
    )

    ;; Restore wasm ptr/len
    (i32.store (global.get $OFF_WASM_PTR) (i32.load (global.get $OFF_WAT_SAV_PTR)))
    (i32.store (global.get $OFF_WASM_LEN) (i32.load (global.get $OFF_WAT_SAV_LEN)))

    (i32.store (global.get $OFF_WAT_DBG) (i32.const 99))
    (return (global.get $OK))
  )

  ;; ═════════════════════════════════════════════════════════════════════
  ;; Exported WAT loader: load_wat(wat_ptr, wat_len) -> error_code
  ;; ═════════════════════════════════════════════════════════════════════

  (func (export "load_wat") (param $wat_ptr i32) (param $wat_len i32) (result i32)
    (return (call $wat_parse_module (local.get $wat_ptr) (local.get $wat_len)))
  )
)
