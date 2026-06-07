(module
  ;; Register number globals


;; ═════════════════════════════════════════════════════════════════════
  ;; Opcode templates — each emits code for one WASM opcode
  ;; Caller sets: decoded_op_ptr before calling template
  ;; Auto-extracted from old compiler files.
  ;; ═════════════════════════════════════════════════════════════════════


  (func $template_unreachable_aarch64
    (call $emit_aarch64_instr (i32.const 0x00000000))
  )

  (func $template_nop_aarch64 (param $dec_ptr i32))

  (func $template_block_aarch64 (param $dec_ptr i32))

  (func $template_loop_aarch64 (param $dec_ptr i32))

  (func $template_if_aarch64 (param $dec_ptr i32)
    (call $emit_aarch64_pop_x0)
    (call $emit_aarch64_cbz_x (global.get $REG_X0_aarch64) (i32.const 0))  ;; placeholder offset
  )

  (func $template_else_aarch64 (param $dec_ptr i32)
    (call $emit_aarch64_b (i32.const 0))  ;; placeholder branch
  )

  (func $template_end_aarch64 (param $dec_ptr i32))

  (func $template_br_aarch64 (param $dec_ptr i32)
    (call $emit_aarch64_b (i32.const 0))  ;; placeholder
  )

  (func $template_br_if_aarch64 (param $dec_ptr i32)
    (call $emit_aarch64_pop_x0)
    (call $emit_aarch64_cbnz_x (global.get $REG_X0_aarch64) (i32.const 0))  ;; placeholder
  )

  (func $template_br_table_aarch64 (param $dec_ptr i32)
    (call $emit_aarch64_pop_x0)
    (call $emit_aarch64_b (i32.const 0))  ;; placeholder (jump to default)
  )

  (func $template_return_aarch64 (param $dec_ptr i32)
    (call $emit_aarch64_epilogue)
  )

  (func $template_return_call_aarch64 (param $dec_ptr i32)
    (call $emit_aarch64_epilogue)
  )

  (func $template_i32_load_aarch64 (param $dec_ptr i32)
    (call $emit_aarch64_pop_x0)
    (call $emit_aarch64_mov_x0_x1)
    (call $emit_aarch64_mem_load32)
    (call $emit_aarch64_push_x0)
  )

  (func $template_i32_load8_s_aarch64 (param $dec_ptr i32)
    (call $emit_aarch64_pop_x0)
    (call $emit_aarch64_mov_x0_x1)
    (call $emit_aarch64_mem_load32)
    (call $emit_aarch64_instr (i32.const 0x13001C00))
    (call $emit_aarch64_push_x0)
  )

  (func $template_i32_load8_u_aarch64 (param $dec_ptr i32)
    (call $emit_aarch64_pop_x0)
    (call $emit_aarch64_mov_x0_x1)
    (call $emit_aarch64_mem_load32)
    (call $emit_aarch64_instr_movz_32 (global.get $REG_X1_aarch64) (i32.const 0) (i32.const 0xFF))
    (call $emit_aarch64_and_w0_w1)
    (call $emit_aarch64_push_x0)
  )

  (func $template_i32_load16_s_aarch64 (param $dec_ptr i32)
    (call $emit_aarch64_pop_x0)
    (call $emit_aarch64_mov_x0_x1)
    (call $emit_aarch64_mem_load32)
    (call $emit_aarch64_instr (i32.const 0x13003C00))
    (call $emit_aarch64_push_x0)
  )

  (func $template_i32_load16_u_aarch64 (param $dec_ptr i32)
    (call $emit_aarch64_pop_x0)
    (call $emit_aarch64_mov_x0_x1)
    (call $emit_aarch64_mem_load32)
    (call $emit_aarch64_instr_movz_32 (global.get $REG_X1_aarch64) (i32.const 0) (i32.const 0xFFFF))
    (call $emit_aarch64_and_w0_w1)
    (call $emit_aarch64_push_x0)
  )

  (func $template_i64_load_aarch64 (param $dec_ptr i32)
    (call $emit_aarch64_pop_x0)
    (call $emit_aarch64_mov_x0_x1)
    (call $emit_aarch64_mem_load64)
    (call $emit_aarch64_push_x0)
  )

  (func $template_i32_store_aarch64 (param $dec_ptr i32)
    (call $emit_aarch64_pop_x0)
    (call $emit_aarch64_pop_x1)
    (call $emit_aarch64_mem_store32)
  )

  (func $template_i32_store8_aarch64 (param $dec_ptr i32)
    (call $emit_aarch64_pop_x0)
    (call $emit_aarch64_pop_x1)
    (call $emit_aarch64_mem_store32)
  )

  (func $template_i32_store16_aarch64 (param $dec_ptr i32)
    (call $emit_aarch64_pop_x0)
    (call $emit_aarch64_pop_x1)
    (call $emit_aarch64_mem_store32)
  )

  (func $template_i64_store_aarch64 (param $dec_ptr i32)
    (call $emit_aarch64_pop_x0)
    (call $emit_aarch64_pop_x1)
    (call $emit_aarch64_mem_store64)
  )

  (func $template_i32_eq_aarch64 (param $dec_ptr i32)
    (call $emit_aarch64_pop_x1)
    (call $emit_aarch64_pop_x0)
    (call $emit_aarch64_cmp_32 (global.get $REG_X0_aarch64) (global.get $REG_X1_aarch64))
    (call $emit_aarch64_cset_w0 (i32.const 1))
    (call $emit_aarch64_push_x0)
  )

  (func $template_i32_ne_aarch64 (param $dec_ptr i32)
    (call $emit_aarch64_pop_x1)
    (call $emit_aarch64_pop_x0)
    (call $emit_aarch64_cmp_32 (global.get $REG_X0_aarch64) (global.get $REG_X1_aarch64))
    (call $emit_aarch64_cset_w0 (i32.const 0))
    (call $emit_aarch64_push_x0)
  )

  (func $template_i32_lt_s_aarch64 (param $dec_ptr i32)
    (call $emit_aarch64_pop_x1)
    (call $emit_aarch64_pop_x0)
    (call $emit_aarch64_cmp_32 (global.get $REG_X0_aarch64) (global.get $REG_X1_aarch64))
    (call $emit_aarch64_cset_w0 (i32.const 10))
    (call $emit_aarch64_push_x0)
  )

  (func $template_i32_lt_u_aarch64 (param $dec_ptr i32)
    (call $emit_aarch64_pop_x1)
    (call $emit_aarch64_pop_x0)
    (call $emit_aarch64_cmp_32 (global.get $REG_X0_aarch64) (global.get $REG_X1_aarch64))
    (call $emit_aarch64_cset_w0 (i32.const 2))
    (call $emit_aarch64_push_x0)
  )

  (func $template_i32_gt_s_aarch64 (param $dec_ptr i32)
    (call $emit_aarch64_pop_x1)
    (call $emit_aarch64_pop_x0)
    (call $emit_aarch64_cmp_32 (global.get $REG_X0_aarch64) (global.get $REG_X1_aarch64))
    (call $emit_aarch64_cset_w0 (i32.const 13))
    (call $emit_aarch64_push_x0)
  )

  (func $template_i32_gt_u_aarch64 (param $dec_ptr i32)
    (call $emit_aarch64_pop_x1)
    (call $emit_aarch64_pop_x0)
    (call $emit_aarch64_cmp_32 (global.get $REG_X0_aarch64) (global.get $REG_X1_aarch64))
    (call $emit_aarch64_cset_w0 (i32.const 9))
    (call $emit_aarch64_push_x0)
  )

  (func $template_i32_le_s_aarch64 (param $dec_ptr i32)
    (call $emit_aarch64_pop_x1)
    (call $emit_aarch64_pop_x0)
    (call $emit_aarch64_cmp_32 (global.get $REG_X0_aarch64) (global.get $REG_X1_aarch64))
    (call $emit_aarch64_cset_w0 (i32.const 12))
    (call $emit_aarch64_push_x0)
  )

  (func $template_i32_le_u_aarch64 (param $dec_ptr i32)
    (call $emit_aarch64_pop_x1)
    (call $emit_aarch64_pop_x0)
    (call $emit_aarch64_cmp_32 (global.get $REG_X0_aarch64) (global.get $REG_X1_aarch64))
    (call $emit_aarch64_cset_w0 (i32.const 8))
    (call $emit_aarch64_push_x0)
  )

  (func $template_i32_ge_s_aarch64 (param $dec_ptr i32)
    (call $emit_aarch64_pop_x1)
    (call $emit_aarch64_pop_x0)
    (call $emit_aarch64_cmp_32 (global.get $REG_X0_aarch64) (global.get $REG_X1_aarch64))
    (call $emit_aarch64_cset_w0 (i32.const 11))
    (call $emit_aarch64_push_x0)
  )

  (func $template_i32_ge_u_aarch64 (param $dec_ptr i32)
    (call $emit_aarch64_pop_x1)
    (call $emit_aarch64_pop_x0)
    (call $emit_aarch64_cmp_32 (global.get $REG_X0_aarch64) (global.get $REG_X1_aarch64))
    (call $emit_aarch64_cset_w0 (i32.const 3))
    (call $emit_aarch64_push_x0)
  )

  (func $template_i64_eq_aarch64 (param $dec_ptr i32)
    (call $emit_aarch64_pop_x1)
    (call $emit_aarch64_pop_x0)
    (call $emit_aarch64_cmp_64 (global.get $REG_X0_aarch64) (global.get $REG_X1_aarch64))
    (call $emit_aarch64_cset_x0 (i32.const 1))
    (call $emit_aarch64_push_x0)
  )

  (func $template_i64_ne_aarch64 (param $dec_ptr i32)
    (call $emit_aarch64_pop_x1)
    (call $emit_aarch64_pop_x0)
    (call $emit_aarch64_cmp_64 (global.get $REG_X0_aarch64) (global.get $REG_X1_aarch64))
    (call $emit_aarch64_cset_x0 (i32.const 0))
    (call $emit_aarch64_push_x0)
  )

  (func $template_i64_lt_s_aarch64 (param $dec_ptr i32)
    (call $emit_aarch64_pop_x1)
    (call $emit_aarch64_pop_x0)
    (call $emit_aarch64_cmp_64 (global.get $REG_X0_aarch64) (global.get $REG_X1_aarch64))
    (call $emit_aarch64_cset_x0 (i32.const 10))
    (call $emit_aarch64_push_x0)
  )

  (func $template_i64_lt_u_aarch64 (param $dec_ptr i32)
    (call $emit_aarch64_pop_x1)
    (call $emit_aarch64_pop_x0)
    (call $emit_aarch64_cmp_64 (global.get $REG_X0_aarch64) (global.get $REG_X1_aarch64))
    (call $emit_aarch64_cset_x0 (i32.const 2))
    (call $emit_aarch64_push_x0)
  )

  (func $template_i64_gt_s_aarch64 (param $dec_ptr i32)
    (call $emit_aarch64_pop_x1)
    (call $emit_aarch64_pop_x0)
    (call $emit_aarch64_cmp_64 (global.get $REG_X0_aarch64) (global.get $REG_X1_aarch64))
    (call $emit_aarch64_cset_x0 (i32.const 13))
    (call $emit_aarch64_push_x0)
  )

  (func $template_i64_gt_u_aarch64 (param $dec_ptr i32)
    (call $emit_aarch64_pop_x1)
    (call $emit_aarch64_pop_x0)
    (call $emit_aarch64_cmp_64 (global.get $REG_X0_aarch64) (global.get $REG_X1_aarch64))
    (call $emit_aarch64_cset_x0 (i32.const 9))
    (call $emit_aarch64_push_x0)
  )

  (func $template_i64_le_s_aarch64 (param $dec_ptr i32)
    (call $emit_aarch64_pop_x1)
    (call $emit_aarch64_pop_x0)
    (call $emit_aarch64_cmp_64 (global.get $REG_X0_aarch64) (global.get $REG_X1_aarch64))
    (call $emit_aarch64_cset_x0 (i32.const 12))
    (call $emit_aarch64_push_x0)
  )

  (func $template_i64_le_u_aarch64 (param $dec_ptr i32)
    (call $emit_aarch64_pop_x1)
    (call $emit_aarch64_pop_x0)
    (call $emit_aarch64_cmp_64 (global.get $REG_X0_aarch64) (global.get $REG_X1_aarch64))
    (call $emit_aarch64_cset_x0 (i32.const 8))
    (call $emit_aarch64_push_x0)
  )

  (func $template_i64_ge_s_aarch64 (param $dec_ptr i32)
    (call $emit_aarch64_pop_x1)
    (call $emit_aarch64_pop_x0)
    (call $emit_aarch64_cmp_64 (global.get $REG_X0_aarch64) (global.get $REG_X1_aarch64))
    (call $emit_aarch64_cset_x0 (i32.const 11))
    (call $emit_aarch64_push_x0)
  )

  (func $template_i64_ge_u_aarch64 (param $dec_ptr i32)
    (call $emit_aarch64_pop_x1)
    (call $emit_aarch64_pop_x0)
    (call $emit_aarch64_cmp_64 (global.get $REG_X0_aarch64) (global.get $REG_X1_aarch64))
    (call $emit_aarch64_cset_x0 (i32.const 3))
    (call $emit_aarch64_push_x0)
  )

  (func $template_i32_eqz_aarch64 (param $dec_ptr i32)
    (call $emit_aarch64_pop_x0)
    (call $emit_aarch64_cmp_32 (global.get $REG_X0_aarch64) (global.get $REG_XZR_aarch64))
    (call $emit_aarch64_cset_w0 (i32.const 1))
    (call $emit_aarch64_push_x0)
  )

  (func $template_i32_clz_aarch64 (param $dec_ptr i32)
    (call $emit_aarch64_pop_x0)
    (call $emit_aarch64_clz_32)
    (call $emit_aarch64_push_x0)
  )

  (func $template_i32_ctz_aarch64 (param $dec_ptr i32)
    (call $emit_aarch64_pop_x0)
    (call $emit_aarch64_rbit_32)
    (call $emit_aarch64_clz_32)
    (call $emit_aarch64_push_x0)
  )

  (func $template_i32_popcnt_aarch64 (param $dec_ptr i32) (global.set $JIT_ERROR_aarch64 (i32.const -4)))

  (func $template_i32_const_aarch64 (param $dec_ptr i32)
    (call $emit_aarch64_instr_movz_32 (global.get $REG_X0_aarch64) (i32.const 0) (i32.and (i32.load (i32.add (local.get $dec_ptr) (i32.const 4))) (i32.const 0xFFFF)))
    (call $emit_aarch64_instr_movk_64 (global.get $REG_X0_aarch64) (i32.const 1) (i32.shr_u (i32.load (i32.add (local.get $dec_ptr) (i32.const 4))) (i32.const 16)))
    (call $emit_aarch64_push_x0)
  )

  (func $template_i32_add_aarch64 (param $dec_ptr i32)
    (call $emit_aarch64_pop_x1)
    (call $emit_aarch64_pop_x0)
    (call $emit_aarch64_add_w0_w1)
    (call $emit_aarch64_push_x0)
  )

  (func $template_i32_sub_aarch64 (param $dec_ptr i32)
    (call $emit_aarch64_pop_x1)
    (call $emit_aarch64_pop_x0)
    (call $emit_aarch64_sub_w0_w1)
    (call $emit_aarch64_push_x0)
  )

  (func $template_i32_mul_aarch64 (param $dec_ptr i32)
    (call $emit_aarch64_pop_x1)
    (call $emit_aarch64_pop_x0)
    (call $emit_aarch64_mul_w0_w1)
    (call $emit_aarch64_push_x0)
  )

  (func $template_i32_div_s_aarch64 (param $dec_ptr i32)
    (call $emit_aarch64_pop_x1)
    (call $emit_aarch64_pop_x0)
    (call $emit_aarch64_sdiv_w0_w1)
    (call $emit_aarch64_push_x0)
  )

  (func $template_i32_div_u_aarch64 (param $dec_ptr i32)
    (call $emit_aarch64_pop_x1)
    (call $emit_aarch64_pop_x0)
    (call $emit_aarch64_udiv_w0_w1)
    (call $emit_aarch64_push_x0)
  )

  (func $template_i32_rem_s_aarch64 (param $dec_ptr i32)
    (call $emit_aarch64_pop_x1)
    (call $emit_aarch64_pop_x0)
    (call $emit_aarch64_push_x0)
    (call $emit_aarch64_push_x1)
    (call $emit_aarch64_pop_x1)
    (call $emit_aarch64_pop_x0)
    (call $emit_aarch64_sdiv_w0_w1)
    (call $emit_aarch64_mul_w0_w1)
    (call $emit_aarch64_mov_w1_w0)
    (call $emit_aarch64_pop_x0)
    (call $emit_aarch64_sub_w0_w1)
    (call $emit_aarch64_push_x0)
  )

  (func $template_i32_rem_u_aarch64 (param $dec_ptr i32)
    (call $emit_aarch64_pop_x1)
    (call $emit_aarch64_pop_x0)
    (call $emit_aarch64_push_x0)
    (call $emit_aarch64_push_x1)
    (call $emit_aarch64_pop_x1)
    (call $emit_aarch64_pop_x0)
    (call $emit_aarch64_udiv_w0_w1)
    (call $emit_aarch64_mul_w0_w1)
    (call $emit_aarch64_mov_w1_w0)
    (call $emit_aarch64_pop_x0)
    (call $emit_aarch64_sub_w0_w1)
    (call $emit_aarch64_push_x0)
  )

  (func $template_i32_and_aarch64 (param $dec_ptr i32)
    (call $emit_aarch64_pop_x1)
    (call $emit_aarch64_pop_x0)
    (call $emit_aarch64_and_w0_w1)
    (call $emit_aarch64_push_x0)
  )

  (func $template_i32_or_aarch64 (param $dec_ptr i32)
    (call $emit_aarch64_pop_x1)
    (call $emit_aarch64_pop_x0)
    (call $emit_aarch64_orr_w0_w1)
    (call $emit_aarch64_push_x0)
  )

  (func $template_i32_xor_aarch64 (param $dec_ptr i32)
    (call $emit_aarch64_pop_x1)
    (call $emit_aarch64_pop_x0)
    (call $emit_aarch64_eor_w0_w1)
    (call $emit_aarch64_push_x0)
  )

  (func $template_i32_shl_aarch64 (param $dec_ptr i32)
    (call $emit_aarch64_pop_x1)
    (call $emit_aarch64_pop_x0)
    (call $emit_aarch64_lslv_w0_w1)
    (call $emit_aarch64_push_x0)
  )

  (func $template_i32_shr_s_aarch64 (param $dec_ptr i32)
    (call $emit_aarch64_pop_x1)
    (call $emit_aarch64_pop_x0)
    (call $emit_aarch64_asrv_w0_w1)
    (call $emit_aarch64_push_x0)
  )

  (func $template_i32_shr_u_aarch64 (param $dec_ptr i32)
    (call $emit_aarch64_pop_x1)
    (call $emit_aarch64_pop_x0)
    (call $emit_aarch64_lsrv_w0_w1)
    (call $emit_aarch64_push_x0)
  )

  (func $template_i32_rotl_aarch64 (param $dec_ptr i32)
    (call $emit_aarch64_pop_x1)
    (call $emit_aarch64_pop_x0)
    (call $emit_aarch64_instr_sub_32 (global.get $REG_X1_aarch64) (global.get $REG_XZR_aarch64) (global.get $REG_X1_aarch64))
    (call $emit_aarch64_rorv_w0_w1)
    (call $emit_aarch64_push_x0)
  )

  (func $template_i32_rotr_aarch64 (param $dec_ptr i32)
    (call $emit_aarch64_pop_x1)
    (call $emit_aarch64_pop_x0)
    (call $emit_aarch64_rorv_w0_w1)
    (call $emit_aarch64_push_x0)
  )

  (func $template_i64_eqz_aarch64 (param $dec_ptr i32)
    (call $emit_aarch64_pop_x0)
    (call $emit_aarch64_cmp_64 (global.get $REG_X0_aarch64) (global.get $REG_XZR_aarch64))
    (call $emit_aarch64_cset_x0 (i32.const 1))
    (call $emit_aarch64_push_x0)
  )

  (func $template_i64_clz_aarch64 (param $dec_ptr i32)
    (call $emit_aarch64_pop_x0)
    (call $emit_aarch64_clz_64)
    (call $emit_aarch64_push_x0)
  )

  (func $template_i64_ctz_aarch64 (param $dec_ptr i32)
    (call $emit_aarch64_pop_x0)
    (call $emit_aarch64_rbit_64)
    (call $emit_aarch64_clz_64)
    (call $emit_aarch64_push_x0)
  )

  (func $template_i64_popcnt_aarch64 (param $dec_ptr i32) (global.set $JIT_ERROR_aarch64 (i32.const -4)))

  (func $template_i64_const_aarch64 (param $dec_ptr i32)
    (call $emit_aarch64_instr_movz_32 (global.get $REG_X0_aarch64) (i32.const 0) (i32.and (i32.load (i32.add (local.get $dec_ptr) (i32.const 4))) (i32.const 0xFFFF)))
    (call $emit_aarch64_instr_movk_64 (global.get $REG_X0_aarch64) (i32.const 1) (i32.shr_u (i32.load (i32.add (local.get $dec_ptr) (i32.const 4))) (i32.const 16)))
    (call $emit_aarch64_instr_movk_64 (global.get $REG_X0_aarch64) (i32.const 2) (i32.shr_u (i32.load (i32.add (local.get $dec_ptr) (i32.const 8))) (i32.const 0)))
    (call $emit_aarch64_instr_movk_64 (global.get $REG_X0_aarch64) (i32.const 3) (i32.shr_u (i32.load (i32.add (local.get $dec_ptr) (i32.const 8))) (i32.const 16)))
    (call $emit_aarch64_push_x0)
  )

  (func $template_i64_add_aarch64 (param $dec_ptr i32)
    (call $emit_aarch64_pop_x1)
    (call $emit_aarch64_pop_x0)
    (call $emit_aarch64_add_x0_x1)
    (call $emit_aarch64_push_x0)
  )

  (func $template_i64_sub_aarch64 (param $dec_ptr i32)
    (call $emit_aarch64_pop_x1)
    (call $emit_aarch64_pop_x0)
    (call $emit_aarch64_sub_x0_x1)
    (call $emit_aarch64_push_x0)
  )

  (func $template_i64_mul_aarch64 (param $dec_ptr i32)
    (call $emit_aarch64_pop_x1)
    (call $emit_aarch64_pop_x0)
    (call $emit_aarch64_mul_x0_x1)
    (call $emit_aarch64_push_x0)
  )

  (func $template_i64_div_s_aarch64 (param $dec_ptr i32)
    (call $emit_aarch64_pop_x1)
    (call $emit_aarch64_pop_x0)
    (call $emit_aarch64_sdiv_x0_x1)
    (call $emit_aarch64_push_x0)
  )

  (func $template_i64_div_u_aarch64 (param $dec_ptr i32)
    (call $emit_aarch64_pop_x1)
    (call $emit_aarch64_pop_x0)
    (call $emit_aarch64_udiv_x0_x1)
    (call $emit_aarch64_push_x0)
  )

  (func $template_i64_rem_s_aarch64 (param $dec_ptr i32)
    (call $emit_aarch64_pop_x1)
    (call $emit_aarch64_pop_x0)
    (call $emit_aarch64_push_x0)
    (call $emit_aarch64_push_x1)
    (call $emit_aarch64_pop_x1)
    (call $emit_aarch64_pop_x0)
    (call $emit_aarch64_sdiv_x0_x1)
    (call $emit_aarch64_mul_x0_x1)
    (call $emit_aarch64_mov_x1_x0)
    (call $emit_aarch64_pop_x0)
    (call $emit_aarch64_sub_x0_x1)
    (call $emit_aarch64_push_x0)
  )

  (func $template_i64_rem_u_aarch64 (param $dec_ptr i32)
    (call $emit_aarch64_pop_x1)
    (call $emit_aarch64_pop_x0)
    (call $emit_aarch64_push_x0)
    (call $emit_aarch64_push_x1)
    (call $emit_aarch64_pop_x1)
    (call $emit_aarch64_pop_x0)
    (call $emit_aarch64_udiv_x0_x1)
    (call $emit_aarch64_mul_x0_x1)
    (call $emit_aarch64_mov_x1_x0)
    (call $emit_aarch64_pop_x0)
    (call $emit_aarch64_sub_x0_x1)
    (call $emit_aarch64_push_x0)
  )

  (func $template_i64_and_aarch64 (param $dec_ptr i32)
    (call $emit_aarch64_pop_x1)
    (call $emit_aarch64_pop_x0)
    (call $emit_aarch64_and_x0_x1)
    (call $emit_aarch64_push_x0)
  )

  (func $template_i64_or_aarch64 (param $dec_ptr i32)
    (call $emit_aarch64_pop_x1)
    (call $emit_aarch64_pop_x0)
    (call $emit_aarch64_orr_x0_x1)
    (call $emit_aarch64_push_x0)
  )

  (func $template_i64_xor_aarch64 (param $dec_ptr i32)
    (call $emit_aarch64_pop_x1)
    (call $emit_aarch64_pop_x0)
    (call $emit_aarch64_eor_x0_x1)
    (call $emit_aarch64_push_x0)
  )

  (func $template_i64_shl_aarch64 (param $dec_ptr i32)
    (call $emit_aarch64_pop_x1)
    (call $emit_aarch64_pop_x0)
    (call $emit_aarch64_lslv_x0_x1)
    (call $emit_aarch64_push_x0)
  )

  (func $template_i64_shr_s_aarch64 (param $dec_ptr i32)
    (call $emit_aarch64_pop_x1)
    (call $emit_aarch64_pop_x0)
    (call $emit_aarch64_asrv_x0_x1)
    (call $emit_aarch64_push_x0)
  )

  (func $template_i64_shr_u_aarch64 (param $dec_ptr i32)
    (call $emit_aarch64_pop_x1)
    (call $emit_aarch64_pop_x0)
    (call $emit_aarch64_lsrv_x0_x1)
    (call $emit_aarch64_push_x0)
  )

  (func $template_i64_rotl_aarch64 (param $dec_ptr i32)
    (call $emit_aarch64_pop_x1)
    (call $emit_aarch64_pop_x0)
    (call $emit_aarch64_instr_sub_64 (global.get $REG_X1_aarch64) (global.get $REG_XZR_aarch64) (global.get $REG_X1_aarch64))
    (call $emit_aarch64_rorv_x0_x1)
    (call $emit_aarch64_push_x0)
  )

  (func $template_i64_rotr_aarch64 (param $dec_ptr i32)
    (call $emit_aarch64_pop_x1)
    (call $emit_aarch64_pop_x0)
    (call $emit_aarch64_rorv_x0_x1)
    (call $emit_aarch64_push_x0)
  )

  (func $template_i32_wrap_i64_aarch64 (param $dec_ptr i32))

  (func $template_i32_reinterpret_f32_aarch64 (param $dec_ptr i32)
    (call $emit_aarch64_pop_x0)
    (call $emit_aarch64_push_x0)
  )

  (func $template_i64_extend_i32_s_aarch64 (param $dec_ptr i32)
    (call $emit_aarch64_pop_x0)
    (call $emit_aarch64_sxtw_x0_w0)
    (call $emit_aarch64_push_x0)
  )

  (func $template_i64_extend_i32_u_aarch64 (param $dec_ptr i32))

  (func $template_i64_reinterpret_f64_aarch64 (param $dec_ptr i32)
    (call $emit_aarch64_pop_x0)
    (call $emit_aarch64_push_x0)
  )

  (func $template_f32_reinterpret_i32_aarch64 (param $dec_ptr i32)
    (call $emit_aarch64_pop_x0)
    (call $emit_aarch64_push_x0)
  )

  (func $template_f64_reinterpret_i64_aarch64 (param $dec_ptr i32)
    (call $emit_aarch64_pop_x0)
    (call $emit_aarch64_push_x0)
  )

  (func $template_i32_trunc_f32_s_aarch64 (param $dec_ptr i32)
    (call $emit_aarch64_pop_x0)
    (call $emit_aarch64_fmov_s_w (i32.const 0) (global.get $REG_X0_aarch64))
    (call $emit_aarch64_fcvtz_w_s (global.get $REG_X0_aarch64) (i32.const 0))
    (call $emit_aarch64_push_x0)
  )

  (func $template_i32_trunc_f32_u_aarch64 (param $dec_ptr i32)
    (call $emit_aarch64_pop_x0)
    (call $emit_aarch64_fmov_s_w (i32.const 0) (global.get $REG_X0_aarch64))
    (call $emit_aarch64_instr (i32.const 0x1E180000))
    (call $emit_aarch64_push_x0)
  )

  (func $template_i32_trunc_f64_s_aarch64 (param $dec_ptr i32)
    (call $emit_aarch64_pop_x0)
    (call $emit_aarch64_fmov_d_x (i32.const 0) (global.get $REG_X0_aarch64))
    (call $emit_aarch64_fcvtz_w_d (global.get $REG_X0_aarch64) (i32.const 0))
    (call $emit_aarch64_push_x0)
  )

  (func $template_i32_trunc_f64_u_aarch64 (param $dec_ptr i32)
    (call $emit_aarch64_pop_x0)
    (call $emit_aarch64_fmov_d_x (i32.const 0) (global.get $REG_X0_aarch64))
    (call $emit_aarch64_instr (i32.const 0x1E580000))
    (call $emit_aarch64_push_x0)
  )

  (func $template_i32_trunc_sat_f32_s_aarch64 (param $dec_ptr i32)
    (call $emit_aarch64_pop_x0)
    (call $emit_aarch64_fmov_s_w (i32.const 0) (global.get $REG_X0_aarch64))
    (call $emit_aarch64_fcvtz_w_s (global.get $REG_X0_aarch64) (i32.const 0))
    (call $emit_aarch64_push_x0)
  )

  (func $template_i32_trunc_sat_f32_u_aarch64 (param $dec_ptr i32)
    (call $emit_aarch64_pop_x0)
    (call $emit_aarch64_fmov_s_w (i32.const 0) (global.get $REG_X0_aarch64))
    (call $emit_aarch64_instr (i32.const 0x1E180000))
    (call $emit_aarch64_push_x0)
  )

  (func $template_i32_trunc_sat_f64_s_aarch64 (param $dec_ptr i32)
    (call $emit_aarch64_pop_x0)
    (call $emit_aarch64_fmov_d_x (i32.const 0) (global.get $REG_X0_aarch64))
    (call $emit_aarch64_fcvtz_w_d (global.get $REG_X0_aarch64) (i32.const 0))
    (call $emit_aarch64_push_x0)
  )

  (func $template_i32_trunc_sat_f64_u_aarch64 (param $dec_ptr i32)
    (call $emit_aarch64_pop_x0)
    (call $emit_aarch64_fmov_d_x (i32.const 0) (global.get $REG_X0_aarch64))
    (call $emit_aarch64_instr (i32.const 0x1E580000))
    (call $emit_aarch64_push_x0)
  )

  (func $template_i64_trunc_f32_s_aarch64 (param $dec_ptr i32)
    (call $emit_aarch64_pop_x0)
    (call $emit_aarch64_fmov_s_w (i32.const 0) (global.get $REG_X0_aarch64))
    (call $emit_aarch64_fcvtz_x_s (global.get $REG_X0_aarch64) (i32.const 0))
    (call $emit_aarch64_push_x0)
  )

  (func $template_i64_trunc_f32_u_aarch64 (param $dec_ptr i32)
    (call $emit_aarch64_pop_x0)
    (call $emit_aarch64_fmov_s_w (i32.const 0) (global.get $REG_X0_aarch64))
    (call $emit_aarch64_instr (i32.const 0x9E180000))
    (call $emit_aarch64_push_x0)
  )

  (func $template_i64_trunc_f64_s_aarch64 (param $dec_ptr i32)
    (call $emit_aarch64_pop_x0)
    (call $emit_aarch64_fmov_d_x (i32.const 0) (global.get $REG_X0_aarch64))
    (call $emit_aarch64_fcvtz_x_d (global.get $REG_X0_aarch64) (i32.const 0))
    (call $emit_aarch64_push_x0)
  )

  (func $template_i64_trunc_f64_u_aarch64 (param $dec_ptr i32)
    (call $emit_aarch64_pop_x0)
    (call $emit_aarch64_fmov_d_x (i32.const 0) (global.get $REG_X0_aarch64))
    (call $emit_aarch64_instr (i32.const 0x9E580000))
    (call $emit_aarch64_push_x0)
  )

  (func $template_i64_trunc_sat_f32_s_aarch64 (param $dec_ptr i32)
    (call $emit_aarch64_pop_x0)
    (call $emit_aarch64_fmov_s_w (i32.const 0) (global.get $REG_X0_aarch64))
    (call $emit_aarch64_fcvtz_x_s (global.get $REG_X0_aarch64) (i32.const 0))
    (call $emit_aarch64_push_x0)
  )

  (func $template_drop_aarch64 (param $dec_ptr i32)
    (call $emit_aarch64_pop_x0)
  )

  (func $template_select_aarch64 (param $dec_ptr i32) (global.set $JIT_ERROR_aarch64 (i32.const -4)))

  (func $template_local_get_aarch64 (param $dec_ptr i32)
    (local $idx i32) (local $disp i32)
    (local.set $idx (i32.load (i32.add (local.get $dec_ptr) (i32.const 4))))
    (local.set $disp (i32.shl (local.get $idx) (i32.const 3)))
    (call $emit_aarch64_instr_ldr_64_off (global.get $REG_X0_aarch64) (global.get $REG_X19_aarch64) (local.get $disp))
    (call $emit_aarch64_push_x0)
  )

  (func $template_local_set_aarch64 (param $dec_ptr i32)
    (local $idx i32) (local $disp i32)
    (local.set $idx (i32.load (i32.add (local.get $dec_ptr) (i32.const 4))))
    (call $emit_aarch64_pop_x0)
    (local.set $disp (i32.shl (local.get $idx) (i32.const 3)))
    (call $emit_aarch64_instr_str_64_off (global.get $REG_X0_aarch64) (global.get $REG_X19_aarch64) (local.get $disp))
  )

  (func $template_local_tee_aarch64 (param $dec_ptr i32) (global.set $JIT_ERROR_aarch64 (i32.const -4)))

  (func $template_global_get_aarch64 (param $dec_ptr i32) (global.set $JIT_ERROR_aarch64 (i32.const -4)))

  (func $template_global_set_aarch64 (param $dec_ptr i32) (global.set $JIT_ERROR_aarch64 (i32.const -4)))

  (func $template_call_aarch64 (param $dec_ptr i32) (global.set $JIT_ERROR_aarch64 (i32.const -4)))

  (func $template_memory_size_aarch64 (param $dec_ptr i32) (global.set $JIT_ERROR_aarch64 (i32.const -4)))

  (func $template_memory_grow_aarch64 (param $dec_ptr i32) (global.set $JIT_ERROR_aarch64 (i32.const -4)))

  (func $template_table_get_aarch64 (param $dec_ptr i32) (global.set $JIT_ERROR_aarch64 (i32.const -4)))

  (func $template_table_set_aarch64 (param $dec_ptr i32) (global.set $JIT_ERROR_aarch64 (i32.const -4)))

  ;; ── Float op stubs ────────────────────────────────────────────
  (func $template_f32_load_aarch64 (param $dec_ptr i32) (global.set $JIT_ERROR_aarch64 (i32.const -4)))
  (func $template_f64_load_aarch64 (param $dec_ptr i32) (global.set $JIT_ERROR_aarch64 (i32.const -4)))
  (func $template_f32_store_aarch64 (param $dec_ptr i32) (global.set $JIT_ERROR_aarch64 (i32.const -4)))
  (func $template_f64_store_aarch64 (param $dec_ptr i32) (global.set $JIT_ERROR_aarch64 (i32.const -4)))
  (func $template_f32_convert_i32_s_aarch64 (param $dec_ptr i32) (global.set $JIT_ERROR_aarch64 (i32.const -4)))
  (func $template_f32_convert_i32_u_aarch64 (param $dec_ptr i32) (global.set $JIT_ERROR_aarch64 (i32.const -4)))
  (func $template_f32_convert_i64_s_aarch64 (param $dec_ptr i32) (global.set $JIT_ERROR_aarch64 (i32.const -4)))
  (func $template_f32_convert_i64_u_aarch64 (param $dec_ptr i32) (global.set $JIT_ERROR_aarch64 (i32.const -4)))
  (func $template_f32_demote_f64_aarch64 (param $dec_ptr i32) (global.set $JIT_ERROR_aarch64 (i32.const -4)))
  (func $template_f64_convert_i32_s_aarch64 (param $dec_ptr i32) (global.set $JIT_ERROR_aarch64 (i32.const -4)))
  (func $template_f64_convert_i32_u_aarch64 (param $dec_ptr i32) (global.set $JIT_ERROR_aarch64 (i32.const -4)))
  (func $template_f64_convert_i64_s_aarch64 (param $dec_ptr i32) (global.set $JIT_ERROR_aarch64 (i32.const -4)))
  (func $template_f64_convert_i64_u_aarch64 (param $dec_ptr i32) (global.set $JIT_ERROR_aarch64 (i32.const -4)))
  (func $template_f64_promote_f32_aarch64 (param $dec_ptr i32) (global.set $JIT_ERROR_aarch64 (i32.const -4)))































































































)