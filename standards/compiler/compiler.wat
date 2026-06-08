;; ═════════════════════════════════════════════════════════════════════
  ;; EdgeRun WASM → Native JIT Compiler — unified core
  ;;
  ;; GENERATOR TEMPLATE — per-arch KV templates expand into dispatch tables.
  ;;
  ;; Usage:
  ;;   python tools/gen_compiler.py templates/x86_64.json > gen/jit-dispatch-x86-64.wat
  ;; ═════════════════════════════════════════════════════════════════════


  ;; ═════════════════════════════════════════════════════════════════════
  ;; JIT State — per-architecture globals (suffixed by arch)
  ;; ═════════════════════════════════════════════════════════════════════

  (global $JIT_SLOT_SIZE{SUFFIX}  i32 (i32.const 0x40000))  ;; 256KB per function
  (global $JIT_STATE{SUFFIX}      i32 (i32.const 0x300000))

  ;; JIT state field offsets (relative to JIT_STATE)
  (global $JS_CODE_PTR{SUFFIX}       i32 (i32.const 0))
  (global $JS_CACHE_BASE{SUFFIX}     i32 (i32.const 4))
  (global $JS_CACHE_END{SUFFIX}      i32 (i32.const 8))
  (global $JS_FUNC_IDX{SUFFIX}       i32 (i32.const 12))
  (global $JS_RESULT_COUNT{SUFFIX}   i32 (i32.const 16))
  (global $JS_STACK_DEPTH{SUFFIX}    i32 (i32.const 20))
  (global $JS_MAX_STACK{SUFFIX}      i32 (i32.const 24))
  (global $JS_LABEL_DEPTH{SUFFIX}    i32 (i32.const 28))
  (global $JS_RETURN_EMITTED{SUFFIX} i32 (i32.const 32))
  (global $JS_LABEL_OFFSETS{SUFFIX}  i32 (i32.const 64))    ;; 256*i32 = 1024 bytes
  (global $JS_LABEL_KINDS{SUFFIX}    i32 (i32.const 1088))   ;; 256*byte = 256 bytes
  (global $JS_LABEL_IF_JZ{SUFFIX}    i32 (i32.const 1344))   ;; 256*i32 = 1024 bytes
  (global $JS_FIXUP_COUNT{SUFFIX}    i32 (i32.const 2368))
  (global $JS_FIXUP_LABEL{SUFFIX}    i32 (i32.const 2372))   ;; 256*i32 = 1024 bytes
  (global $JS_FIXUP_OFFSET{SUFFIX}   i32 (i32.const 3396))   ;; 256*i32 = 1024 bytes
  (global $JS_INITIALIZED{SUFFIX}    i32 (i32.const 4420))
  (global $JS_CALL_FIXUP_COUNT{SUFFIX} i32 (i32.const 4424))

  ;; Label kinds
  (global $JIT_LABEL_BLOCK{SUFFIX}   i32 (i32.const 0))
  (global $JIT_LABEL_LOOP{SUFFIX}    i32 (i32.const 1))
  (global $JIT_LABEL_IF{SUFFIX}      i32 (i32.const 2))
  (global $JIT_LABEL_ELSE    i32 (i32.const 3))

  ;; Peephole optimization flag
  (global ${RESULT_GLOBAL} (mut i32) (i32.const 0))

  ;; Current decoded op pointer
  (global $CURRENT_DEC_PTR{SUFFIX}   (mut i32) (i32.const 0))

  ;; Peephole lookahead
  (global ${NEXT_OP_GLOBAL} (mut i32) (i32.const 0))

  ;; JIT error code
  (global ${JIT_ERROR_GLOBAL} (mut i32) (i32.const 0))

  ;; ELF/binary buffer globals are defined in per-arch emit-*.wat files


  ;; ═════════════════════════════════════════════════════════════════════
  ;; Label Management — shared across all architectures
  ;; ═════════════════════════════════════════════════════════════════════

  ;; Push a label kind onto the label stack
  (func $push_label{SUFFIX} (param $kind i32) (result i32)
    (local $depth i32)
    (local.set $depth (i32.load (global.get $JS_LABEL_DEPTH{SUFFIX})))
    (i32.store8
      (i32.add (global.get $JS_LABEL_KINDS{SUFFIX}) (local.get $depth))
      (local.get $kind))
    (i32.store (global.get $JS_LABEL_DEPTH{SUFFIX}) (i32.add (local.get $depth) (i32.const 1)))
    (local.get $depth)
  )

  ;; Pop a label kind from the label stack
  (func $pop_label{SUFFIX} (result i32)
    (local $depth i32)
    (local.set $depth (i32.sub (i32.load (global.get $JS_LABEL_DEPTH{SUFFIX})) (i32.const 1)))
    (i32.store (global.get $JS_LABEL_DEPTH{SUFFIX}) (local.get $depth))
    (local.get $depth)
  )

  ;; Get label kind at depth
  (func $get_label_kind{SUFFIX} (param $depth i32) (result i32)
    (i32.load8_u (i32.add (global.get $JS_LABEL_KINDS{SUFFIX}) (local.get $depth)))
  )

  ;; Set label offset
  (func $set_label_offset{SUFFIX} (param $depth i32) (param $offset i32)
    (i32.store
      (i32.add (global.get $JS_LABEL_OFFSETS{SUFFIX}) (i32.shl (local.get $depth) (i32.const 2)))
      (local.get $offset))
  )

  ;; Get label offset
  (func $get_label_offset{SUFFIX} (param $depth i32) (result i32)
    (i32.load
      (i32.add (global.get $JS_LABEL_OFFSETS{SUFFIX}) (i32.shl (local.get $depth) (i32.const 2))))
  )

  ;; Set label if-jz offset
  (func $set_label_if_jz{SUFFIX} (param $depth i32) (param $offset i32)
    (i32.store
      (i32.add (global.get $JS_LABEL_IF_JZ{SUFFIX}) (i32.shl (local.get $depth) (i32.const 2)))
      (local.get $offset))
  )

  ;; Get label if-jz offset
  (func $get_label_if_jz{SUFFIX} (param $depth i32) (result i32)
    (i32.load
      (i32.add (global.get $JS_LABEL_IF_JZ{SUFFIX}) (i32.shl (local.get $depth) (i32.const 2))))
  )

  ;; Push a fixup
  (func $push_fixup{SUFFIX} (param $target_depth i32)
    (local $count i32)
    (local.set $count (i32.load (global.get $JS_FIXUP_COUNT{SUFFIX})))
    (i32.store
      (i32.add (global.get $JS_FIXUP_LABEL{SUFFIX}) (i32.shl (local.get $count) (i32.const 2)))
      (local.get $target_depth))
    (i32.store
      (i32.add (global.get $JS_FIXUP_OFFSET{SUFFIX}) (i32.shl (local.get $count) (i32.const 2)))
      (i32.load (global.get $JS_CODE_PTR{SUFFIX})))
    (i32.store (global.get $JS_FIXUP_COUNT{SUFFIX}) (i32.add (local.get $count) (i32.const 1)))
  )

  ;; Patch fixups for a given depth
  (func $patch_fixups{SUFFIX} (param $depth i32)
    (local $count i32) (local $i i32) (local $target i32) (local $off i32)
    (local $code_ptr i32) (local $disp i32) (local $label_off i32)
    (local.set $count (i32.load (global.get $JS_FIXUP_COUNT{SUFFIX})))
    (local.set $code_ptr (i32.load (global.get $JS_CODE_PTR{SUFFIX})))
    (local.set $i (i32.const 0))
    (block $done
      (loop $lp
        (if (i32.ge_u (local.get $i) (local.get $count)) (then (br $done)))
        (local.set $target (i32.load
          (i32.add (global.get $JS_FIXUP_LABEL{SUFFIX}) (i32.shl (local.get $i) (i32.const 2)))))
        (if (i32.eq (local.get $target) (local.get $depth))
          (then
            (local.set $off (i32.load
              (i32.add (global.get $JS_FIXUP_OFFSET{SUFFIX}) (i32.shl (local.get $i) (i32.const 2)))))
            (local.set $label_off (call $get_label_offset{SUFFIX} (local.get $depth)))
            (local.set $disp (i32.sub (local.get $label_off) (i32.add (local.get $off) (i32.const 4))))
            (i32.store (i32.add (global.get $JIT_CACHE) (local.get $off)) (local.get $disp))
          )
        )
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $lp)
      )
    )
  )


  ;; ═════════════════════════════════════════════════════════════════════
  ;; JIT Compile Dispatch — opcode dispatch tables expanded from per-arch KV template.
  ;; ═════════════════════════════════════════════════════════════════════

  (func $jit_compile{SUFFIX} (export "jit_compile{SUFFIX}") (param $func_idx i32) (result i32)
    (local $dec_ptr i32) (local $opcode i32) (local $imm0 i32)
    (local $code_start i32) (local $label_level i32)
    (local $loop_top_offset i32)
    (local $decoded_ops_base i32) (local $decoded_ops_count i32)

    ;; Reset label depth and push implicit function block
    (i32.store (global.get $JS_LABEL_DEPTH{SUFFIX}) (i32.const 0))
    (call $push_label{SUFFIX} (global.get $JIT_LABEL_BLOCK{SUFFIX}))

    ;; Get decoded_ops base from imported global
    (local.set $decoded_ops_base (global.get $OFF_DECODED_OPS))

    ;; ── Record this function's offset in func_off_table ────────────
    (i32.store
      (i32.add (global.get ${FUNC_OFF_TABLE}) (i32.shl (local.get $func_idx) (i32.const 2)))
      (i32.load (global.get $JS_CODE_PTR{SUFFIX}))
    )

    ;; ── Emit prologue ─────────────────────────────────────────────
    (call ${PROLOGUE})

    ;; ── Main compile loop ─────────────────────────────────────────
    ;; Point dec_ptr to this function's decoded ops:
    ;;   code_entry_index = func_idx - import_count
    ;;   decoded_start = code_entry[code_entry_index].decoded_start
    ;;   dec_ptr = decoded_ops_base + decoded_start * DEC_SZ
    (local.set $dec_ptr
      (i32.add
        (local.get $decoded_ops_base)
        (i32.mul
          (i32.load
            (i32.add
              (i32.add
                (global.get $OFF_CODE_BUF)
                (i32.mul
                  (i32.sub (local.get $func_idx) (i32.load (global.get $OFF_IMPORT_COUNT)))
                  (global.get $SZ_CODE)))
              (i32.const 24)))
          (global.get $DEC_SZ))))

    ;; ── Read next op for peephole ─────────────────────────────────
    (if (i32.load (i32.add (local.get $dec_ptr) (global.get $DEC_SZ)))
      (then (global.set ${NEXT_OP_GLOBAL} (i32.load (i32.add (local.get $dec_ptr) (global.get $DEC_SZ)))))
      (else (global.set ${NEXT_OP_GLOBAL} (i32.const 0)))
    )
    (block $compile_done
      (loop $compile_loop
        (local.set $opcode (i32.load (local.get $dec_ptr)))

        ;; Read next opcode for peephole (at dec_ptr + DEC_SZ)
        (if (i32.load (i32.add (local.get $dec_ptr) (global.get $DEC_SZ)))
          (then (global.set ${NEXT_OP_GLOBAL} (i32.load (i32.add (local.get $dec_ptr) (global.get $DEC_SZ)))))
          (else (global.set ${NEXT_OP_GLOBAL} (i32.const 0)))
        )

        ;; ── Dispatch by opcode ─────────────────────────────────────
        (block $dispatch_done
{OP_TABLE}
          ;; 0xFC prefix — dispatch on imm0
          (if (i32.eq (local.get $opcode) (i32.const 0xFC))
            (then
              (local.set $imm0 (i32.load (i32.add (local.get $dec_ptr) (i32.const 4))))
              (block $fc_done
{FC_TABLE}
                ;; Unknown 0xFC sub-opcode
                (global.set ${JIT_ERROR_GLOBAL} (i32.const -3))
              )
              (br $dispatch_done)
            )
          )
          ;; 0xFD prefix (SIMD) — dispatch on imm0
          (if (i32.eq (local.get $opcode) (i32.const 0xFD))
            (then
              (local.set $imm0 (i32.load (i32.add (local.get $dec_ptr) (i32.const 4))))
              (block $fd_done
{FD_TABLE}
                ;; Unknown 0xFD sub-opcode
                (global.set ${JIT_ERROR_GLOBAL} (i32.const -4))
              )
              (br $dispatch_done)
            )
          )
          ;; Unknown opcode
          (global.set ${JIT_ERROR_GLOBAL} (i32.const -2))
        )
        ;; Advance to next decoded op
        (local.set $dec_ptr (i32.add (local.get $dec_ptr) (global.get $DEC_SZ)))
        (br_if $compile_done (i32.lt_s (i32.load (global.get $JS_LABEL_DEPTH{SUFFIX})) (i32.const 0)))
        (br_if $compile_done (i32.eqz (i32.load (global.get $JS_LABEL_DEPTH{SUFFIX}))))
        (br $compile_loop)
      )
    )

    ;; ── Pop return value, then epilogue ──────────────────────────────
    (call ${EPILOGUE})

    ;; ── Return code size ──────────────────────────────────────────
    (return (i32.load (global.get $JS_CODE_PTR{SUFFIX})))
  )


  ;; ═════════════════════════════════════════════════════════════════════
  ;; ELF Output — compile all functions to ELF64 binary
  ;; ═════════════════════════════════════════════════════════════════════

  (func $compile_all_to_elf{SUFFIX} (export "compile_all_to_elf{SUFFIX}") (result i32 i32)
    (local $import_count i32) (local $func_count i32) (local $i i32)
    (local $code_size i32) (local $total_size i32) (local $syscall_data_size i32)
    (local $saved i32) (local $bss_va i32) (local $data_va i32)
    (local $start_func_idx i32) (local $start_off i32) (local $start_va i32)

    (local.set $import_count (i32.load (i32.const 0x4108)))
    (local.set $func_count (i32.load (i32.const 0x4510)))

    (i32.store (global.get $JS_CODE_PTR{SUFFIX}) (i32.const 0))
    (i32.store (global.get $JS_FIXUP_COUNT{SUFFIX}) (i32.const 0))
    (i32.store (global.get $JS_CALL_FIXUP_COUNT{SUFFIX}) (i32.const 0))

    (local.set $i (local.get $import_count))
    (block $compile_done
      (loop $compile_loop
        (if (i32.ge_u (local.get $i) (local.get $func_count)) (then (br $compile_done)))
        (drop (call $jit_compile{SUFFIX} (local.get $i)))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $compile_loop)
      )
    )

    (call ${FIXUP_CALLS})

    ;; ── Look up start function (main) code offset ──────────────────
    (local.set $start_func_idx (i32.load (global.get $OFF_START_FUNC)))
    (local.set $start_off
      (i32.load
        (i32.add (global.get ${FUNC_OFF_TABLE}) (i32.shl (local.get $start_func_idx) (i32.const 2)))))
    (local.set $start_va
      (i32.add
        (i32.add (global.get $TEXT_VA{SUFFIX}) (global.get $ELF_CODE_OFF{SUFFIX}))
        (local.get $start_off)))

    (local.set $code_size (i32.load (global.get $JS_CODE_PTR{SUFFIX})))
    (local.set $syscall_data_size (i32.shl (local.get $import_count) (i32.const 2)))
    (local.set $total_size
      (i32.add
        (i32.add (global.get $ELF_CODE_OFF{SUFFIX}) (local.get $code_size))
        (local.get $syscall_data_size)))

    (local.set $bss_va
      (i32.add
        (i32.and (i32.add (local.get $total_size) (i32.const 0xFFF)) (i32.const -0x1000))
        (global.get $TEXT_VA{SUFFIX})))

    (local.set $data_va
      (i32.add
        (global.get $TEXT_VA{SUFFIX})
        (i32.add (global.get $ELF_CODE_OFF{SUFFIX}) (local.get $code_size))))

    (local.set $saved (i32.load (global.get $JS_CODE_PTR{SUFFIX})))
    (i32.store (global.get $JS_CODE_PTR{SUFFIX}) (global.get $ELF_OUT_OFF{SUFFIX}))

    ;; ── Emit ELF64 header ──
    (call ${EMIT_ELF64_EHDR}
      (i32.add (global.get $TEXT_VA{SUFFIX}) (global.get $ELF_STUB_OFF{SUFFIX}))
      (i32.const 64)
      (i32.const 1))

    ;; ── Emit program header ──
    (call ${EMIT_ELF64_PHDR}
      (i32.const 1) (i32.const 7) (i32.const 0)
      (global.get $TEXT_VA{SUFFIX}) (local.get $total_size)
      (i32.add
        (i32.and (i32.add (local.get $total_size) (i32.const 0xFFF)) (i32.const -0x1000))
        (global.get $BSS_SIZE{SUFFIX})))

    ;; ── Emit runtime stub (calls start function = main) ──
    (call ${EMIT_ELF_STUB} (local.get $bss_va) (local.get $data_va) (local.get $import_count) (local.get $start_va))

    ;; ── Pad to ELF_CODE_OFF ──
    (block $pad_done
      (loop $pad_loop
        (if (i32.ge_u (i32.load (global.get $JS_CODE_PTR{SUFFIX}))
                       (i32.add (global.get $ELF_OUT_OFF{SUFFIX}) (global.get $ELF_CODE_OFF{SUFFIX})))
          (then (br $pad_done)))
        (call ${EMIT_BYTE} (i32.const 0))
        (br $pad_loop)))

    (call ${COPY_COMPILED_CODE} (global.get $JIT_CACHE) (local.get $code_size))

    (i32.store (global.get $JS_CODE_PTR{SUFFIX})
      (i32.add (i32.load (global.get $JS_CODE_PTR{SUFFIX})) (local.get $code_size)))

    (local.set $i (i32.const 0))
    (block $data_loop_done
      (loop $data_loop
        (if (i32.ge_u (local.get $i) (local.get $import_count)) (then (br $data_loop_done)))
        (call ${EMIT_DWORD}
          (i32.load (i32.add (i32.const 0x90000) (i32.shl (local.get $i) (i32.const 2)))))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $data_loop)))

    (i32.store (global.get $JS_CODE_PTR{SUFFIX}) (local.get $saved))
    (return (global.get $ELF_OUT_BUF{SUFFIX}) (local.get $total_size))
  )


  ;; ═════════════════════════════════════════════════════════════════════
  ;; Runtime Backend Dispatch — selects arch at runtime
  ;; ═════════════════════════════════════════════════════════════════════

  (global $JIT_BACKEND (export "JIT_BACKEND") (mut i32) (i32.const 0))

  (func $jit_compile (export "jit_compile") (param $func_idx i32) (param $backend i32) (result i32)
    (if (i32.eq (local.get $backend) (i32.const 0))
      (then (return (call $jit_compile_x86_64 (local.get $func_idx)))))
    (if (i32.eq (local.get $backend) (i32.const 1))
      (then (return (call $jit_compile_arm32 (local.get $func_idx)))))
    (if (i32.eq (local.get $backend) (i32.const 2))
      (then (return (call $jit_compile_aarch64 (local.get $func_idx)))))
    (i32.const -1)
  )

  (func $compile_to_elf (export "compile_to_elf") (param $func_idx i32) (param $backend i32) (result i32 i32)
    (if (i32.eq (local.get $backend) (i32.const 0))
      (then (return (call $compile_to_elf_x86_64 (local.get $func_idx)))))
    (if (i32.eq (local.get $backend) (i32.const 1))
      (then (return (call $compile_to_elf_arm32 (local.get $func_idx)))))
    (if (i32.eq (local.get $backend) (i32.const 2))
      (then (return (call $compile_to_elf_aarch64 (local.get $func_idx)))))
    (i32.const 0) (i32.const 0)
  )
