;; ═════════════════════════════════════════════════════════════════════
  ;; Opcode templates — each emits x86_64 code for one WASM opcode
  ;; Caller sets: rdi = decoded_op_ptr before calling template
  ;; ═════════════════════════════════════════════════════════════════════

  ;; ── unreachable (0x00): ud2 ────────────────────────────────────────
  (func $template_unreachable_x86_64
    (call $emit_x86_ud2)
  )

  ;; ── nop (0x01): nothing ────────────────────────────────────────────
  (func $template_nop_x86_64 (param $dec_ptr i32))

  ;; ── drop (0x1A): add rsp, 8 ───────────────────────────────────────
  (func $template_drop_x86_64
    (call $emit_x86_add_rsp_imm (i32.const 8))
  )

  ;; ── select (0x1B): pop cond, pop val2, pop val1; push val1 if cond!=0 else val2 ──
  (func $template_select_x86_64
    (call $emit_x86_pop_rax)          ;; cond
    (call $emit_x86_test_eax)         ;; set ZF
    (call $emit_x86_pop_reg (i32.const 1))  ;; pop rcx = val2
    (call $emit_x86_pop_reg (i32.const 2))  ;; pop rdx = val1
    (call $emit_x86_cmove_rdx_rcx)    ;; if ZF (cond==0), rdx = rcx (val2)
    (call $emit_x86_push_reg (i32.const 2)) ;; push rdx
  )

  ;; ── i32.const (0x41): push imm32 ──────────────────────────────────
  (func $template_i32_const_x86_64 (param $dec_ptr i32)
    (call $emit_x86_push_imm32 (i32.load (i32.add (local.get $dec_ptr) (i32.const 4))))
  )

  ;; ── i64.const (0x42): push imm64 ──────────────────────────────────
  (func $template_i64_const_x86_64 (param $dec_ptr i32)
    (local $val i64)
    (local.set $val (i64.load (i32.add (local.get $dec_ptr) (i32.const 4))))
    ;; mov rax, imm64 (REX.W B8 + qword); push rax
    (call $emit_x86_rex_w)
    (call $emit_x86_byte (i32.const 0xB8))
    (call $emit_x86_qword (local.get $val))
    (call $emit_x86_maybe_push_rax)
  )

  ;; ── local.get (0x20): mov rax, [rdx + index*8]; push rax ──────────
  (func $template_local_get_x86_64 (param $dec_ptr i32)
    (local $idx i32) (local $disp i32)
    (local.set $idx (i32.load (i32.add (local.get $dec_ptr) (i32.const 4))))
    (if (i32.eqz (local.get $idx))
      (then (call $emit_x86_push_r12))
      (else
        (if (i32.eq (local.get $idx) (i32.const 1))
          (then (call $emit_x86_push_r13))
          (else
            (local.set $disp (i32.shl (local.get $idx) (i32.const 3)))
            (call $emit_x86_load_rax_rbx_disp (local.get $disp))
            (call $emit_x86_maybe_push_rax)
          )
        )
      )
    )
  )

  ;; ── local.set (0x21): pop rax/reg; store to locals ──────────────────
  (func $template_local_set_x86_64 (param $dec_ptr i32)
    (local $idx i32) (local $disp i32)
    (local.set $idx (i32.load (i32.add (local.get $dec_ptr) (i32.const 4))))
    (if (global.get $RESULT_IN_EAX)
      (then
        ;; Result already in eax — directly store (skip pop)
        (global.set $RESULT_IN_EAX (i32.const 0))
        (if (i32.eqz (local.get $idx))
          (then (call $emit_x86_store_rax_r12))
          (else
            (if (i32.eq (local.get $idx) (i32.const 1))
              (then (call $emit_x86_store_rax_r13))
              (else
                (local.set $disp (i32.shl (local.get $idx) (i32.const 3)))
                (call $emit_x86_store_rax_rbx_disp (local.get $disp))
              )
            )
          )
        )
      )
      (else
        ;; Normal path: pop from stack
        (if (i32.eqz (local.get $idx))
          (then (call $emit_x86_pop_r12))
          (else
            (if (i32.eq (local.get $idx) (i32.const 1))
              (then (call $emit_x86_pop_r13))
              (else
                (local.set $disp (i32.shl (local.get $idx) (i32.const 3)))
                (call $emit_x86_pop_rax)
                (call $emit_x86_store_rax_rbx_disp (local.get $disp))
              )
            )
          )
        )
      )
    )
  )

  ;; ── local.tee (0x22): same as local.set but keep value on stack ───
  (func $template_local_tee_x86_64 (param $dec_ptr i32)
    (local $idx i32) (local $disp i32)
    (local.set $idx (i32.load (i32.add (local.get $dec_ptr) (i32.const 4))))
    (if (global.get $RESULT_IN_EAX)
      (then
        ;; Result in eax: duplicate it (push copy) then store
        (call $emit_x86_byte (i32.const 0x50))
        (global.set $RESULT_IN_EAX (i32.const 0))
        (if (i32.eqz (local.get $idx))
          (then (call $emit_x86_store_rax_r12))
          (else
            (if (i32.eq (local.get $idx) (i32.const 1))
              (then (call $emit_x86_store_rax_r13))
              (else
                (local.set $disp (i32.shl (local.get $idx) (i32.const 3)))
                (call $emit_x86_store_rax_rbx_disp (local.get $disp))
              )
            )
          )
        )
      )
      (else
        ;; Normal path: peek from stack, duplicate, store
        (call $emit_x86_rex_w)
        (call $emit_x86_byte (i32.const 0x8B))
        (call $emit_x86_byte (i32.const 0x04))
        (call $emit_x86_sib (i32.const 0) (i32.const 4) (i32.const 4))  ;; [rsp]
        (call $emit_x86_byte (i32.const 0x50))
        (if (i32.eqz (local.get $idx))
          (then (call $emit_x86_store_r12_rbx))
          (else
            (if (i32.eq (local.get $idx) (i32.const 1))
              (then (call $emit_x86_store_r13_rbx8))
              (else
                (local.set $disp (i32.shl (local.get $idx) (i32.const 3)))
                (call $emit_x86_store_rax_rbx_disp (local.get $disp))
              )
            )
          )
        )
      )
    )
  )

  ;; ── global.get (0x23) ─────────────────────────────────────────────
  (func $template_global_get_x86_64 (param $dec_ptr i32)
    (local $offs i32)
    (local.set $offs (i32.add (i32.mul (i32.load (i32.add (local.get $dec_ptr) (i32.const 4))) (i32.const 32)) (i32.const 8)))
    (call $emit_x86_load_r15_to_rdx (i32.const 24))   ;; JitGlobals.globals_buf
    (call $emit_x86_load_rax_rdx_disp (local.get $offs))
    (call $emit_x86_maybe_push_rax)
  )

  ;; ── global.set (0x24) ─────────────────────────────────────────────
  (func $template_global_set_x86_64 (param $dec_ptr i32)
    (local $offs i32)
    (local.set $offs (i32.add (i32.mul (i32.load (i32.add (local.get $dec_ptr) (i32.const 4))) (i32.const 32)) (i32.const 8)))
    (call $emit_x86_pop_rax)
    (call $emit_x86_maybe_push_rax)
    (call $emit_x86_load_r15_to_rdx (i32.const 24))
    (call $emit_x86_pop_rax)
    (call $emit_x86_store_rax_rdx_disp (local.get $offs))
  )

  ;; ── table.get (0x25) ─────────────────────────────────────────────
  (func $template_table_get_x86_64 (param $dec_ptr i32)
    (call $emit_x86_pop_reg (i32.const 0))      ;; pop rax = index
    ;; load table_entries → rdx
    (call $emit_x86_load_r15_to_rdx (i32.const 48))  ;; JitGlobals.table_entries
    ;; mov rax, [rdx + rax*8] — use SIB with scale=3 (8x)
    (call $emit_x86_rex_w)
    (call $emit_x86_byte (i32.const 0x8B))
    (call $emit_x86_byte (i32.const 0x04))      ;; mod=00 reg=0 rm=SIB
    (call $emit_x86_sib (i32.const 3) (i32.const 0) (i32.const 2))  ;; scale=8, index=rax, base=rdx
    (call $emit_x86_maybe_push_rax)
  )

  ;; ── table.set (0x26) ─────────────────────────────────────────────
  (func $template_table_set_x86_64 (param $dec_ptr i32)
    ;; Stack: ... value index
    (call $emit_x86_pop_reg (i32.const 2))      ;; pop rdx = index
    (call $emit_x86_pop_reg (i32.const 0))      ;; pop rax = value
    (call $emit_x86_push_reg (i32.const 0))     ;; save value on stack
    (call $emit_x86_push_reg (i32.const 2))     ;; save index on stack
    (call $emit_x86_load_r15_to_rdx (i32.const 48))  ;; rdx = table_entries
    (call $emit_x86_pop_reg (i32.const 0))      ;; pop rax = index
    (call $emit_x86_pop_reg (i32.const 1))      ;; pop rcx = value
    ;; mov [rdx + rax*8], rcx
    (call $emit_x86_rex_w)
    (call $emit_x86_byte (i32.const 0x89))
    (call $emit_x86_modrm (i32.const 0) (i32.const 1) (i32.const 4))
    (call $emit_x86_sib (i32.const 3) (i32.const 0) (i32.const 2))
  )

  ;; ── i32 comparison templates ──────────────────────────────────────

  ;; i32.eqz (0x45) — pop rax, test eax,eax, sete al, movzx, push rax
  (func $template_i32_eqz_x86_64
    (call $emit_x86_pop_rax)
    (call $emit_x86_test_eax)
    (call $emit_x86_setcc (i32.const 0x94))    ;; sete al
    (call $emit_x86_movzx_eax_al)
    (call $emit_x86_maybe_push_rax)
  )

  ;; i32.eq (0x46) — pop rcx, pop rax, cmp, sete, push
  (func $template_i32_eq_x86_64
    (call $emit_x86_pop2_rcx_rax)
    (call $emit_x86_cmp32)
    (call $emit_x86_setcc (i32.const 0x94))
    (call $emit_x86_movzx_eax_al)
    (call $emit_x86_maybe_push_rax)
  )

  ;; i32.ne (0x47)
  (func $template_i32_ne_x86_64
    (call $emit_x86_pop2_rcx_rax)
    (call $emit_x86_cmp32)
    (call $emit_x86_setcc (i32.const 0x95))    ;; setne
    (call $emit_x86_movzx_eax_al)
    (call $emit_x86_maybe_push_rax)
  )

  ;; i32.lt_s (0x48)
  (func $template_i32_lt_s_x86_64
    (call $emit_x86_pop2_rcx_rax)
    (call $emit_x86_cmp32)
    (call $emit_x86_setcc (i32.const 0x9C))    ;; setl
    (call $emit_x86_movzx_eax_al)
    (call $emit_x86_maybe_push_rax)
  )

  ;; i32.lt_u (0x49)
  (func $template_i32_lt_u_x86_64
    (call $emit_x86_pop2_rcx_rax)
    (call $emit_x86_cmp32)
    (call $emit_x86_setcc (i32.const 0x92))    ;; setb
    (call $emit_x86_movzx_eax_al)
    (call $emit_x86_maybe_push_rax)
  )

  ;; i32.gt_s (0x4A)
  (func $template_i32_gt_s_x86_64
    (call $emit_x86_pop2_rcx_rax)
    (call $emit_x86_cmp32)
    (call $emit_x86_setcc (i32.const 0x9F))    ;; setg
    (call $emit_x86_movzx_eax_al)
    (call $emit_x86_maybe_push_rax)
  )

  ;; i32.gt_u (0x4B)
  (func $template_i32_gt_u_x86_64
    (call $emit_x86_pop2_rcx_rax)
    (call $emit_x86_cmp32)
    (call $emit_x86_setcc (i32.const 0x97))    ;; seta
    (call $emit_x86_movzx_eax_al)
    (call $emit_x86_maybe_push_rax)
  )

  ;; i32.le_s (0x4C)
  (func $template_i32_le_s_x86_64
    (call $emit_x86_pop2_rcx_rax)
    (call $emit_x86_cmp32)
    (call $emit_x86_setcc (i32.const 0x9E))    ;; setle
    (call $emit_x86_movzx_eax_al)
    (call $emit_x86_maybe_push_rax)
  )

  ;; i32.le_u (0x4D)
  (func $template_i32_le_u_x86_64
    (call $emit_x86_pop2_rcx_rax)
    (call $emit_x86_cmp32)
    (call $emit_x86_setcc (i32.const 0x96))    ;; setbe
    (call $emit_x86_movzx_eax_al)
    (call $emit_x86_maybe_push_rax)
  )

  ;; i32.ge_s (0x4E)
  (func $template_i32_ge_s_x86_64
    (call $emit_x86_pop2_rcx_rax)
    (call $emit_x86_cmp32)
    (call $emit_x86_setcc (i32.const 0x9D))    ;; setge
    (call $emit_x86_movzx_eax_al)
    (call $emit_x86_maybe_push_rax)
  )

  ;; i32.ge_u (0x4F)
  (func $template_i32_ge_u_x86_64
    (call $emit_x86_pop2_rcx_rax)
    (call $emit_x86_cmp32)
    (call $emit_x86_setcc (i32.const 0x93))    ;; setae
    (call $emit_x86_movzx_eax_al)
    (call $emit_x86_maybe_push_rax)
  )

  ;; ── i32 binary arithmetic templates ───────────────────────────────

  (func $template_i32_add_x86_64
    (call $emit_x86_pop2_rcx_rax) (call $emit_x86_add32) (call $emit_x86_maybe_push_rax)
  )

  (func $template_i32_sub_x86_64
    (call $emit_x86_pop2_rcx_rax) (call $emit_x86_sub32) (call $emit_x86_maybe_push_rax)
  )

  (func $template_i32_mul_x86_64
    (call $emit_x86_pop2_rcx_rax) (call $emit_x86_imul32) (call $emit_x86_maybe_push_rax)
  )

  (func $template_i32_div_s_x86_64
    (call $emit_x86_pop2_rcx_rax) (call $emit_x86_cdq) (call $emit_x86_idiv32) (call $emit_x86_maybe_push_rax)
  )

  (func $template_i32_div_u_x86_64
    (call $emit_x86_pop2_rcx_rax) (call $emit_x86_xor_edx_edx) (call $emit_x86_idiv32) (call $emit_x86_maybe_push_rax)
  )

  (func $template_i32_rem_s_x86_64
    (call $emit_x86_pop2_rcx_rax) (call $emit_x86_cdq) (call $emit_x86_idiv32)
    (call $emit_x86_mov_eax_edx) (call $emit_x86_maybe_push_rax)
  )

  (func $template_i32_rem_u_x86_64
    (call $emit_x86_pop2_rcx_rax) (call $emit_x86_xor_edx_edx) (call $emit_x86_idiv32)
    (call $emit_x86_mov_eax_edx) (call $emit_x86_maybe_push_rax)
  )

  (func $template_i32_and_x86_64
    (call $emit_x86_pop2_rcx_rax) (call $emit_x86_and32) (call $emit_x86_maybe_push_rax)
  )

  (func $template_i32_or_x86_64
    (call $emit_x86_pop2_rcx_rax) (call $emit_x86_ror32) (call $emit_x86_maybe_push_rax)
  )

  (func $template_i32_xor_x86_64
    (call $emit_x86_pop2_rcx_rax) (call $emit_x86_xor32) (call $emit_x86_maybe_push_rax)
  )

  (func $template_i32_shl_x86_64
    (call $emit_x86_pop2_rcx_rax) (call $emit_x86_shl32) (call $emit_x86_maybe_push_rax)
  )

  (func $template_i32_shr_s_x86_64
    (call $emit_x86_pop2_rcx_rax) (call $emit_x86_sar32) (call $emit_x86_maybe_push_rax)
  )

  (func $template_i32_shr_u_x86_64
    (call $emit_x86_pop2_rcx_rax) (call $emit_x86_shr32) (call $emit_x86_maybe_push_rax)
  )

  (func $template_i32_rotl_x86_64
    (call $emit_x86_pop2_rcx_rax) (call $emit_x86_rol32) (call $emit_x86_maybe_push_rax)
  )

  (func $template_i32_rotr_x86_64
    (call $emit_x86_pop2_rcx_rax) (call $emit_x86_ror32) (call $emit_x86_maybe_push_rax)
  )

  ;; ── i64 binary arithmetic templates ───────────────────────────────

  (func $template_i64_add_x86_64
    (call $emit_x86_pop2_rcx_rax) (call $emit_x86_add64) (call $emit_x86_maybe_push_rax)
  )

  (func $template_i64_sub_x86_64
    (call $emit_x86_pop2_rcx_rax) (call $emit_x86_sub64) (call $emit_x86_maybe_push_rax)
  )

  (func $template_i64_mul_x86_64
    (call $emit_x86_pop2_rcx_rax) (call $emit_x86_imul64) (call $emit_x86_maybe_push_rax)
  )

  ;; i64.div_s (0x7F) — pop rcx, pop rax, cqo, idiv rcx
  (func $template_i64_div_s_x86_64
    (call $emit_x86_pop2_rcx_rax) (call $emit_x86_cqo) (call $emit_x86_idiv64) (call $emit_x86_maybe_push_rax)
  )

  ;; i64.div_u (0x80)
  (func $template_i64_div_u_x86_64
    (call $emit_x86_pop2_rcx_rax) (call $emit_x86_xor_rdx_rdx) (call $emit_x86_idiv64) (call $emit_x86_maybe_push_rax)
  )

  ;; i64.rem_s (0x81)
  (func $template_i64_rem_s_x86_64
    (call $emit_x86_pop2_rcx_rax) (call $emit_x86_cqo) (call $emit_x86_idiv64)
    (call $emit_x86_mov_rax_rdx) (call $emit_x86_maybe_push_rax)
  )

  ;; i64.rem_u (0x82)
  (func $template_i64_rem_u_x86_64
    (call $emit_x86_pop2_rcx_rax) (call $emit_x86_xor_rdx_rdx) (call $emit_x86_idiv64)
    (call $emit_x86_mov_rax_rdx) (call $emit_x86_maybe_push_rax)
  )

  ;; i64.and (0x83), or (0x84), xor (0x85)
  (func $template_i64_and_x86_64
    (call $emit_x86_pop2_rcx_rax) (call $emit_x86_and64) (call $emit_x86_maybe_push_rax)
  )

  (func $template_i64_or_x86_64
    (call $emit_x86_pop2_rcx_rax) (call $emit_x86_xor64) (call $emit_x86_maybe_push_rax)
  )

  (func $template_i64_xor_x86_64
    (call $emit_x86_pop2_rcx_rax) (call $emit_x86_xor64) (call $emit_x86_maybe_push_rax)
  )

  ;; i64.shl (0x86), shr_s (0x87), shr_u (0x88), rotl (0x89), rotr (0x8A)
  (func $template_i64_shl_x86_64
    (call $emit_x86_pop2_rcx_rax) (call $emit_x86_shl64) (call $emit_x86_maybe_push_rax)
  )

  (func $template_i64_shr_s_x86_64
    (call $emit_x86_pop2_rcx_rax) (call $emit_x86_sar64) (call $emit_x86_maybe_push_rax)
  )

  (func $template_i64_shr_u_x86_64
    (call $emit_x86_pop2_rcx_rax) (call $emit_x86_shr64) (call $emit_x86_maybe_push_rax)
  )

  (func $template_i64_rotl_x86_64
    (call $emit_x86_pop2_rcx_rax) (call $emit_x86_rol64) (call $emit_x86_maybe_push_rax)
  )

  (func $template_i64_rotr_x86_64
    (call $emit_x86_pop2_rcx_rax) (call $emit_x86_ror64) (call $emit_x86_maybe_push_rax)
  )

  ;; ── i64 unary templates ──────────────────────────────────────────

  ;; i64.clz (0x79)
  (func $template_i64_clz_x86_64
    (call $emit_x86_pop_rax) (call $emit_x86_lzcnt64) (call $emit_x86_maybe_push_rax)
  )

  ;; i64.ctz (0x7A)
  (func $template_i64_ctz_x86_64
    (call $emit_x86_pop_rax) (call $emit_x86_tzcnt64) (call $emit_x86_maybe_push_rax)
  )

  ;; i64.popcnt (0x7B)
  (func $template_i64_popcnt_x86_64
    (call $emit_x86_pop_rax) (call $emit_x86_popcnt64) (call $emit_x86_maybe_push_rax)
  )

  ;; ── i64 comparison templates ─────────────────────────────────────

  ;; i64.eqz (0x50) — test rax, rax; sete; movzx
  (func $template_i64_eqz_x86_64
    (call $emit_x86_pop_rax)
    (call $emit_x86_rex_w)
    (call $emit_x86_test_eax)   ;; becomes test rax, rax with REX.W
    (call $emit_x86_setcc (i32.const 0x94))    ;; sete al
    (call $emit_x86_movzx_eax_al)
    (call $emit_x86_maybe_push_rax)
  )

  ;; i64.eq (0x51), ne (0x52), lt_s (0x53), lt_u (0x54)
  (func $template_i64_eq_x86_64
    (call $emit_x86_pop2_rcx_rax) (call $emit_x86_cmp64)
    (call $emit_x86_setcc (i32.const 0x94)) (call $emit_x86_movzx_eax_al) (call $emit_x86_maybe_push_rax)
  )

  (func $template_i64_ne_x86_64
    (call $emit_x86_pop2_rcx_rax) (call $emit_x86_cmp64)
    (call $emit_x86_setcc (i32.const 0x95)) (call $emit_x86_movzx_eax_al) (call $emit_x86_maybe_push_rax)
  )

  (func $template_i64_lt_s_x86_64
    (call $emit_x86_pop2_rcx_rax) (call $emit_x86_cmp64)
    (call $emit_x86_setcc (i32.const 0x9C)) (call $emit_x86_movzx_eax_al) (call $emit_x86_maybe_push_rax)
  )

  (func $template_i64_lt_u_x86_64
    (call $emit_x86_pop2_rcx_rax) (call $emit_x86_cmp64)
    (call $emit_x86_setcc (i32.const 0x92)) (call $emit_x86_movzx_eax_al) (call $emit_x86_maybe_push_rax)
  )

  ;; i64.gt_s (0x55), gt_u (0x56), le_s (0x57), le_u (0x58)
  (func $template_i64_gt_s_x86_64
    (call $emit_x86_pop2_rcx_rax) (call $emit_x86_cmp64)
    (call $emit_x86_setcc (i32.const 0x9F)) (call $emit_x86_movzx_eax_al) (call $emit_x86_maybe_push_rax)
  )

  (func $template_i64_gt_u_x86_64
    (call $emit_x86_pop2_rcx_rax) (call $emit_x86_cmp64)
    (call $emit_x86_setcc (i32.const 0x97)) (call $emit_x86_movzx_eax_al) (call $emit_x86_maybe_push_rax)
  )

  (func $template_i64_le_s_x86_64
    (call $emit_x86_pop2_rcx_rax) (call $emit_x86_cmp64)
    (call $emit_x86_setcc (i32.const 0x9E)) (call $emit_x86_movzx_eax_al) (call $emit_x86_maybe_push_rax)
  )

  (func $template_i64_le_u_x86_64
    (call $emit_x86_pop2_rcx_rax) (call $emit_x86_cmp64)
    (call $emit_x86_setcc (i32.const 0x96)) (call $emit_x86_movzx_eax_al) (call $emit_x86_maybe_push_rax)
  )

  ;; i64.ge_s (0x59), ge_u (0x5A)
  (func $template_i64_ge_s_x86_64
    (call $emit_x86_pop2_rcx_rax) (call $emit_x86_cmp64)
    (call $emit_x86_setcc (i32.const 0x9D)) (call $emit_x86_movzx_eax_al) (call $emit_x86_maybe_push_rax)
  )

  (func $template_i64_ge_u_x86_64
    (call $emit_x86_pop2_rcx_rax) (call $emit_x86_cmp64)
    (call $emit_x86_setcc (i32.const 0x93)) (call $emit_x86_movzx_eax_al) (call $emit_x86_maybe_push_rax)
  )

  ;; ── i32 unary templates ──────────────────────────────────────────

  (func $template_i32_clz_x86_64
    (call $emit_x86_pop_rax) (call $emit_x86_lzcnt32) (call $emit_x86_maybe_push_rax)
  )

  (func $template_i32_ctz_x86_64
    (call $emit_x86_pop_rax) (call $emit_x86_tzcnt32) (call $emit_x86_maybe_push_rax)
  )

  (func $template_i32_popcnt_x86_64
    (call $emit_x86_pop_rax) (call $emit_x86_popcnt32) (call $emit_x86_maybe_push_rax)
  )

  ;; ── Sign extension templates ──────────────────────────────────────

  (func $template_i32_extend8_s_x86_64
    (call $emit_x86_pop_rax)
    (call $emit_x86_byte (i32.const 0x0F)) (call $emit_x86_byte (i32.const 0xBE))
    (call $emit_x86_modrm (i32.const 3) (i32.const 0) (i32.const 0))  ;; movsx eax, al
    (call $emit_x86_maybe_push_rax)
  )

  (func $template_i32_extend16_s_x86_64
    (call $emit_x86_pop_rax)
    (call $emit_x86_byte (i32.const 0x0F)) (call $emit_x86_byte (i32.const 0xBF))
    (call $emit_x86_modrm (i32.const 3) (i32.const 0) (i32.const 0))  ;; movsx eax, ax
    (call $emit_x86_maybe_push_rax)
  )

  ;; i64.extend8_s (0xC2): movsx rax, al (REX.W + 0F BE C0)
  (func $template_i64_extend8_s_x86_64
    (call $emit_x86_pop_rax)
    (call $emit_x86_rex_w)
    (call $emit_x86_byte (i32.const 0x0F)) (call $emit_x86_byte (i32.const 0xBE))
    (call $emit_x86_modrm (i32.const 3) (i32.const 0) (i32.const 0))
    (call $emit_x86_maybe_push_rax)
  )

  ;; i64.extend16_s (0xC3): movsx rax, ax (REX.W + 0F BF C0)
  (func $template_i64_extend16_s_x86_64
    (call $emit_x86_pop_rax)
    (call $emit_x86_rex_w)
    (call $emit_x86_byte (i32.const 0x0F)) (call $emit_x86_byte (i32.const 0xBF))
    (call $emit_x86_modrm (i32.const 3) (i32.const 0) (i32.const 0))
    (call $emit_x86_maybe_push_rax)
  )

  ;; i64.extend32_s (0xC4): movsxd rax, eax (48 63 C0)
  (func $template_i64_extend32_s_x86_64
    (call $emit_x86_pop_rax)
    (call $emit_x86_movsxd_rax_eax)
    (call $emit_x86_maybe_push_rax)
  )

  ;; ── Conversion templates ─────────────────────────────────────────

  (func $template_i32_wrap_i64_x86_64
    (call $emit_x86_pop_rax)
    (call $emit_x86_mov_eax_eax)  ;; zero-extend
    (call $emit_x86_maybe_push_rax)
  )

  (func $template_i64_extend_i32_s_x86_64
    (call $emit_x86_pop_rax)
    (call $emit_x86_movsxd_rax_eax)
    (call $emit_x86_maybe_push_rax)
  )

  (func $template_i64_extend_i32_u_x86_64
    (call $emit_x86_pop_rax)
    (call $emit_x86_mov_eax_eax)
    (call $emit_x86_maybe_push_rax)
  )

  ;; ── Float conversion templates ─────────────────────────────────────

  ;; f32.convert_i32_s (0xB2)
  (func $template_f32_convert_i32_s_x86_64
    (call $emit_x86_pop_rax)
    (call $emit_x86_cvtsi2ss_xmm0_eax)
    (call $emit_x86_movd_eax_xmm0)
    (call $emit_x86_maybe_push_rax)
  )

  ;; f32.convert_i64_s (0xB4)
  (func $template_f32_convert_i64_s_x86_64
    (call $emit_x86_pop_rax)
    (call $emit_x86_cvtsi2ss_xmm0_rax)
    (call $emit_x86_movd_eax_xmm0)
    (call $emit_x86_maybe_push_rax)
  )

  ;; f32.convert_i32_u (0xB3) — branch-free unsigned i32→f32
  (func $template_f32_convert_i32_u_x86_64
    (call $emit_x86_pop_rax)
    (call $emit_x86_mov_ecx_eax)
    (call $emit_x86_sar_ecx_31)
    (call $emit_x86_and_ecx_imm32 (i32.const 0x4F800000))  ;; 2^32 as f32
    (call $emit_x86_cvtsi2ss_xmm0_eax)
    (call $emit_x86_byte (i32.const 0x66))
    (call $emit_x86_byte (i32.const 0x0F))
    (call $emit_x86_byte (i32.const 0x6E))
    (call $emit_x86_modrm (i32.const 3) (i32.const 1) (i32.const 0))  ;; movd xmm1, ecx
    (call $emit_x86_byte (i32.const 0xF3))
    (call $emit_x86_byte (i32.const 0x0F))
    (call $emit_x86_byte (i32.const 0x58))
    (call $emit_x86_modrm (i32.const 3) (i32.const 0) (i32.const 1))  ;; addss xmm0, xmm1
    (call $emit_x86_movd_eax_xmm0)
    (call $emit_x86_maybe_push_rax)
  )

  ;; f32.convert_i64_u (0xB5) — branch-free unsigned i64→f32
  (func $template_f32_convert_i64_u_x86_64
    (call $emit_x86_pop_rax)
    (call $emit_x86_mov_ecx_eax)  ;; rcx = i64 (lower 32 bits)
    (call $emit_x86_sar_rcx_63)
    (call $emit_x86_and_ecx_imm32 (i32.const 0x5F800000))  ;; 2^64 as f32
    (call $emit_x86_cvtsi2ss_xmm0_rax)
    (call $emit_x86_byte (i32.const 0x66))
    (call $emit_x86_byte (i32.const 0x0F))
    (call $emit_x86_byte (i32.const 0x6E))
    (call $emit_x86_modrm (i32.const 3) (i32.const 1) (i32.const 0))  ;; movd xmm1, ecx
    (call $emit_x86_byte (i32.const 0xF3))
    (call $emit_x86_byte (i32.const 0x0F))
    (call $emit_x86_byte (i32.const 0x58))
    (call $emit_x86_modrm (i32.const 3) (i32.const 0) (i32.const 1))  ;; addss xmm0, xmm1
    (call $emit_x86_movd_eax_xmm0)
    (call $emit_x86_maybe_push_rax)
  )

  ;; f64.convert_i32_s (0xB7)
  (func $template_f64_convert_i32_s_x86_64
    (call $emit_x86_pop_rax)
    (call $emit_x86_cvtsi2sd_xmm0_eax)
    (call $emit_x86_movq_rax_xmm0)
    (call $emit_x86_maybe_push_rax)
  )

  ;; f64.convert_i64_s (0xB9)
  (func $template_f64_convert_i64_s_x86_64
    (call $emit_x86_pop_rax)
    (call $emit_x86_cvtsi2sd_xmm0_rax)
    (call $emit_x86_movq_rax_xmm0)
    (call $emit_x86_maybe_push_rax)
  )

  ;; f64.convert_i32_u (0xB8) — branch-free unsigned i32→f64
  (func $template_f64_convert_i32_u_x86_64
    (call $emit_x86_pop_rax)
    (call $emit_x86_mov_ecx_eax)
    (call $emit_x86_sar_ecx_31)
    (call $emit_x86_and_ecx_imm32 (i32.const 0x41F00000))  ;; high dword of 2^32 as f64
    (call $emit_x86_cvtsi2sd_xmm0_eax)
    (call $emit_x86_mov_rax_rcx)
    (call $emit_x86_shl_rax_imm8 (i32.const 32))            ;; rax = 0x41F0000000000000 or 0
    (call $emit_x86_movq_xmm1_rax)
    (call $emit_x86_byte (i32.const 0xF2))
    (call $emit_x86_byte (i32.const 0x0F))
    (call $emit_x86_byte (i32.const 0x58))
    (call $emit_x86_modrm (i32.const 3) (i32.const 0) (i32.const 1))  ;; addsd xmm0, xmm1
    (call $emit_x86_movq_rax_xmm0)
    (call $emit_x86_maybe_push_rax)
  )

  ;; f64.convert_i64_u (0xBA) — branch-free unsigned i64→f64
  (func $template_f64_convert_i64_u_x86_64
    (call $emit_x86_pop_rax)
    (call $emit_x86_mov_ecx_eax)
    (call $emit_x86_sar_rcx_63)
    (call $emit_x86_and_ecx_imm32 (i32.const 0x43F00000))  ;; high dword of 2^64 as f64
    (call $emit_x86_cvtsi2sd_xmm0_rax)
    (call $emit_x86_mov_rax_rcx)
    (call $emit_x86_shl_rax_imm8 (i32.const 32))            ;; rax = 0x43F0000000000000 or 0
    (call $emit_x86_movq_xmm1_rax)
    (call $emit_x86_byte (i32.const 0xF2))
    (call $emit_x86_byte (i32.const 0x0F))
    (call $emit_x86_byte (i32.const 0x58))
    (call $emit_x86_modrm (i32.const 3) (i32.const 0) (i32.const 1))  ;; addsd xmm0, xmm1
    (call $emit_x86_movq_rax_xmm0)
    (call $emit_x86_maybe_push_rax)
  )

  ;; i32.trunc_f32_s (0xA8)
  (func $template_i32_trunc_f32_s_x86_64
    (call $emit_x86_pop_rax)
    (call $emit_x86_movd_xmm0_eax)
    (call $emit_x86_cvttss2si_eax_xmm0)
    (call $emit_x86_maybe_push_rax)
  )

  ;; i32.trunc_f64_s (0xAA)
  (func $template_i32_trunc_f64_s_x86_64
    (call $emit_x86_pop_rax)
    (call $emit_x86_movq_xmm0_rax)
    (call $emit_x86_cvttsd2si_eax_xmm0)
    (call $emit_x86_maybe_push_rax)
  )

  ;; i32.trunc_f32_u (0xA9) — branch-free with cmovns
  (func $template_i32_trunc_f32_u_x86_64
    (call $emit_x86_pop_rax)
    (call $emit_x86_movd_xmm0_eax)
    ;; Path A: direct signed truncation, save in edx
    (call $emit_x86_cvttss2si_eax_xmm0)
    (call $emit_x86_mov_edx_eax)
    ;; Path B: unsigned adjustment (f32 - 2^31, truncate, add 2^31)
    (call $emit_x86_push_imm32 (i32.const 0x4F000000))  ;; 2^31 as f32
    (call $emit_x86_pop_rax)
    (call $emit_x86_byte (i32.const 0x66))
    (call $emit_x86_byte (i32.const 0x0F))
    (call $emit_x86_byte (i32.const 0x6E))
    (call $emit_x86_modrm (i32.const 3) (i32.const 1) (i32.const 0))  ;; movd xmm1, eax
    (call $emit_x86_byte (i32.const 0xF3))
    (call $emit_x86_byte (i32.const 0x0F))
    (call $emit_x86_byte (i32.const 0x5C))
    (call $emit_x86_modrm (i32.const 3) (i32.const 0) (i32.const 1))  ;; subss xmm0, xmm1
    (call $emit_x86_cvttss2si_eax_xmm0)
    (call $emit_x86_add_eax_imm32 (i32.const 0x80000000))
    ;; Select: if edx >= 0, use edx; else use eax
    (call $emit_x86_test_edx)
    (call $emit_x86_cmovns_eax_edx)
    (call $emit_x86_maybe_push_rax)
  )

  ;; i32.trunc_f64_u (0xAB) — branch-free with cmovns
  (func $template_i32_trunc_f64_u_x86_64
    (call $emit_x86_pop_rax)
    (call $emit_x86_movq_xmm0_rax)
    ;; Path A
    (call $emit_x86_cvttsd2si_eax_xmm0)
    (call $emit_x86_mov_edx_eax)
    ;; Path B: f64 - 2^31, truncate, add 2^31
    (call $emit_x86_rex_w)
    (call $emit_x86_byte (i32.const 0xB8))
    (call $emit_x86_qword (i64.const 0x41E0000000000000))  ;; 2^31 as f64
    (call $emit_x86_movq_xmm1_rax)
    (call $emit_x86_byte (i32.const 0xF2))
    (call $emit_x86_byte (i32.const 0x0F))
    (call $emit_x86_byte (i32.const 0x5C))
    (call $emit_x86_modrm (i32.const 3) (i32.const 0) (i32.const 1))  ;; subsd xmm0, xmm1
    (call $emit_x86_cvttsd2si_eax_xmm0)
    (call $emit_x86_add_eax_imm32 (i32.const 0x80000000))
    ;; Select
    (call $emit_x86_test_edx)
    (call $emit_x86_cmovns_eax_edx)
    (call $emit_x86_maybe_push_rax)
  )

  ;; i64.trunc_f32_s (0xAE)
  (func $template_i64_trunc_f32_s_x86_64
    (call $emit_x86_pop_rax)
    (call $emit_x86_movd_xmm0_eax)
    (call $emit_x86_cvttss2si_rax_xmm0)
    (call $emit_x86_maybe_push_rax)
  )

  ;; i64.trunc_f64_s (0xB0)
  (func $template_i64_trunc_f64_s_x86_64
    (call $emit_x86_pop_rax)
    (call $emit_x86_movq_xmm0_rax)
    (call $emit_x86_cvttsd2si_rax_xmm0)
    (call $emit_x86_maybe_push_rax)
  )

  ;; i64.trunc_f32_u (0xAF) — branch-free with cmovns
  (func $template_i64_trunc_f32_u_x86_64
    (call $emit_x86_pop_rax)
    (call $emit_x86_movd_xmm0_eax)
    ;; Path A: direct signed 64-bit truncation, save in rdx
    (call $emit_x86_cvttss2si_rax_xmm0)
    (call $emit_x86_mov_rdx_rax)
    ;; Path B: f32 - 2^63, truncate, add 2^63 via bts
    (call $emit_x86_push_imm32 (i32.const 0x5F000000))  ;; 2^63 as f32
    (call $emit_x86_pop_rax)
    (call $emit_x86_byte (i32.const 0x66))
    (call $emit_x86_byte (i32.const 0x0F))
    (call $emit_x86_byte (i32.const 0x6E))
    (call $emit_x86_modrm (i32.const 3) (i32.const 1) (i32.const 0))  ;; movd xmm1, eax
    (call $emit_x86_byte (i32.const 0xF3))
    (call $emit_x86_byte (i32.const 0x0F))
    (call $emit_x86_byte (i32.const 0x5C))
    (call $emit_x86_modrm (i32.const 3) (i32.const 0) (i32.const 1))  ;; subss xmm0, xmm1
    (call $emit_x86_cvttss2si_rax_xmm0)
    ;; bts rax, 63: set bit 63 (= add 2^63)
    (call $emit_x86_rex_w)
    (call $emit_x86_byte (i32.const 0x0F))
    (call $emit_x86_byte (i32.const 0xBA))
    (call $emit_x86_modrm (i32.const 3) (i32.const 5) (i32.const 0))
    (call $emit_x86_byte (i32.const 0x3F))
    ;; Select: if rdx >= 0, use rdx; else use rax
    (call $emit_x86_rex_w)
    (call $emit_x86_byte (i32.const 0x85))
    (call $emit_x86_modrm (i32.const 3) (i32.const 2) (i32.const 2))  ;; test rdx, rdx
    (call $emit_x86_cmovns_rax_rdx)
    (call $emit_x86_maybe_push_rax)
  )

  ;; i64.trunc_f64_u (0xB1) — branch-free with cmovns
  (func $template_i64_trunc_f64_u_x86_64
    (call $emit_x86_pop_rax)
    (call $emit_x86_movq_xmm0_rax)
    ;; Path A
    (call $emit_x86_cvttsd2si_rax_xmm0)
    (call $emit_x86_mov_rdx_rax)
    ;; Path B: f64 - 2^63, truncate, add 2^63 via bts
    (call $emit_x86_rex_w)
    (call $emit_x86_byte (i32.const 0xB8))
    (call $emit_x86_qword (i64.const 0x43E0000000000000))  ;; 2^63 as f64
    (call $emit_x86_movq_xmm1_rax)
    (call $emit_x86_byte (i32.const 0xF2))
    (call $emit_x86_byte (i32.const 0x0F))
    (call $emit_x86_byte (i32.const 0x5C))
    (call $emit_x86_modrm (i32.const 3) (i32.const 0) (i32.const 1))  ;; subsd xmm0, xmm1
    (call $emit_x86_cvttsd2si_rax_xmm0)
    ;; bts rax, 63
    (call $emit_x86_rex_w)
    (call $emit_x86_byte (i32.const 0x0F))
    (call $emit_x86_byte (i32.const 0xBA))
    (call $emit_x86_modrm (i32.const 3) (i32.const 5) (i32.const 0))
    (call $emit_x86_byte (i32.const 0x3F))
    ;; Select
    (call $emit_x86_rex_w)
    (call $emit_x86_byte (i32.const 0x85))
    (call $emit_x86_modrm (i32.const 3) (i32.const 2) (i32.const 2))  ;; test rdx, rdx
    (call $emit_x86_cmovns_rax_rdx)
    (call $emit_x86_maybe_push_rax)
  )

  ;; f32.demote_f64 (0xB6)
  (func $template_f32_demote_f64_x86_64
    (call $emit_x86_pop_rax)
    (call $emit_x86_movq_xmm0_rax)
    (call $emit_x86_cvtsd2ss_xmm0_xmm0)
    (call $emit_x86_movd_eax_xmm0)
    (call $emit_x86_maybe_push_rax)
  )

  ;; f64.promote_f32 (0xBB)
  (func $template_f64_promote_f32_x86_64
    (call $emit_x86_pop_rax)
    (call $emit_x86_movd_xmm0_eax)
    (call $emit_x86_cvtss2sd_xmm0_xmm0)
    (call $emit_x86_movq_rax_xmm0)
    (call $emit_x86_maybe_push_rax)
  )

  ;; ── Saturating truncation templates (0xFC prefix) ─────────────────

  ;; Helper: patch a rel8 displacement at position (addr-1)
  (func $patch_rel8 (param $after_jmp i32)
    (i32.store8
      (i32.sub (local.get $after_jmp) (i32.const 1))
      (i32.sub (call $get_x86_code_ptr) (local.get $after_jmp))
    )
  )

  ;; Helper: load 4-byte constant into xmm1 (push_imm32 + pop + movd xmm1)
  (func $load_imm32_xmm1 (param $val i32)
    (call $emit_x86_push_imm32 (local.get $val))
    (call $emit_x86_pop_rax)
    (call $emit_x86_byte (i32.const 0x66))
    (call $emit_x86_byte (i32.const 0x0F))
    (call $emit_x86_byte (i32.const 0x6E))
    (call $emit_x86_modrm (i32.const 3) (i32.const 1) (i32.const 0))  ;; movd xmm1, eax
  )

  ;; Helper: load 8-byte constant into xmm1 (mov rax, imm64 + movq xmm1)
  (func $load_imm64_xmm1 (param $hi i32) (param $lo i32)
    (call $emit_x86_rex_w)
    (call $emit_x86_byte (i32.const 0xB8))
    (call $emit_x86_dword (local.get $lo))
    (call $emit_x86_dword (local.get $hi))
    (call $emit_x86_movq_xmm1_rax)
  )

  ;; i32.trunc_sat_f32_s (0xFC, 0x00)
  (func $template_i32_trunc_sat_f32_s_x86_64
    (local $nan_p i32) (local $ovf_p i32) (local $udf_p i32) (local $done_p i32)
    (call $emit_x86_pop_rax)
    (call $emit_x86_movd_xmm0_eax)
    ;; NaN check
    (call $emit_x86_ucomiss_xmm0)
    (call $emit_x86_jp_rel8 (i32.const 0))
    (local.set $nan_p (call $get_x86_code_ptr))
    ;; Overflow: f32 >= 2147483648.0 → INT_MAX
    (call $load_imm32_xmm1 (i32.const 0x4F000000))
    (call $emit_x86_ucomiss)
    (call $emit_x86_jae_rel8 (i32.const 0))
    (local.set $ovf_p (call $get_x86_code_ptr))
    ;; Underflow: f32 < -2147483648.0 → INT_MIN
    (call $load_imm32_xmm1 (i32.const 0xCF000000))
    (call $emit_x86_ucomiss)
    (call $emit_x86_jb_rel8 (i32.const 0))
    (local.set $udf_p (call $get_x86_code_ptr))
    ;; Normal
    (call $emit_x86_cvttss2si_eax_xmm0)
    (call $emit_x86_jmp_rel8 (i32.const 0))
    (local.set $done_p (call $get_x86_code_ptr))
    ;; .nan
    (call $patch_rel8 (local.get $nan_p))
    (call $emit_x86_xor_eax_eax)
    (call $emit_x86_jmp_rel8 (i32.const 0))
    (local.set $done_p (call $get_x86_code_ptr))
    ;; .max (overflow)
    (call $patch_rel8 (local.get $ovf_p))
    (call $emit_x86_mov_eax_imm32 (i32.const 0x7FFFFFFF))
    (call $emit_x86_jmp_rel8 (i32.const 0))
    (local.set $done_p (call $get_x86_code_ptr))
    ;; .min (underflow)
    (call $patch_rel8 (local.get $udf_p))
    (call $emit_x86_mov_eax_imm32 (i32.const 0x80000000))
    ;; .done
    (call $patch_rel8 (local.get $done_p))
    (call $emit_x86_maybe_push_rax)
  )

  ;; i32.trunc_sat_f32_u (0xFC, 0x01)
  (func $template_i32_trunc_sat_f32_u_x86_64
    (local $nan_p i32) (local $ovf_p i32) (local $udf_p i32) (local $done_p i32)
    (call $emit_x86_pop_rax)
    (call $emit_x86_movd_xmm0_eax)
    ;; NaN → 0
    (call $emit_x86_ucomiss_xmm0)
    (call $emit_x86_jp_rel8 (i32.const 0))
    (local.set $nan_p (call $get_x86_code_ptr))
    ;; Overflow: f32 >= 4294967296.0 → UINT_MAX (0xFFFFFFFF)
    (call $load_imm32_xmm1 (i32.const 0x4F800000))
    (call $emit_x86_ucomiss)
    (call $emit_x86_jae_rel8 (i32.const 0))
    (local.set $ovf_p (call $get_x86_code_ptr))
    ;; Underflow: f32 < 0.0 → 0
    (call $load_imm32_xmm1 (i32.const 0x00000000))  ;; 0.0f
    (call $emit_x86_ucomiss)
    (call $emit_x86_jb_rel8 (i32.const 0))
    (local.set $udf_p (call $get_x86_code_ptr))
    ;; Normal: use unsigned truncation
    (call $template_i32_trunc_f32_u_x86_64)
    (call $emit_x86_jmp_rel8 (i32.const 0))
    (local.set $done_p (call $get_x86_code_ptr))
    ;; .nan: eax = 0
    (call $patch_rel8 (local.get $nan_p))
    (call $emit_x86_xor_eax_eax)
    (call $emit_x86_jmp_rel8 (i32.const 0))
    (local.set $done_p (call $get_x86_code_ptr))
    ;; .overflow: eax = -1 (UINT_MAX)
    (call $patch_rel8 (local.get $ovf_p))
    (call $emit_x86_mov_eax_imm32 (i32.const 0xFFFFFFFF))
    (call $emit_x86_jmp_rel8 (i32.const 0))
    (local.set $done_p (call $get_x86_code_ptr))
    ;; .underflow: eax = 0
    (call $patch_rel8 (local.get $udf_p))
    (call $emit_x86_xor_eax_eax)
    ;; .done
    (call $patch_rel8 (local.get $done_p))
    (call $emit_x86_maybe_push_rax)
  )

  ;; i32.trunc_sat_f64_s (0xFC, 0x02)
  (func $template_i32_trunc_sat_f64_s_x86_64
    (local $nan_p i32) (local $ovf_p i32) (local $udf_p i32) (local $done_p i32)
    (call $emit_x86_pop_rax)
    (call $emit_x86_movq_xmm0_rax)
    ;; NaN → 0
    (call $emit_x86_ucomisd_xmm0)
    (call $emit_x86_jp_rel8 (i32.const 0))
    (local.set $nan_p (call $get_x86_code_ptr))
    ;; Overflow: f64 >= 2147483648.0 → INT_MAX
    (call $load_imm64_xmm1 (i32.const 0x41E00000) (i32.const 0x00000000))
    (call $emit_x86_ucomisd)
    (call $emit_x86_jae_rel8 (i32.const 0))
    (local.set $ovf_p (call $get_x86_code_ptr))
    ;; Underflow: f64 < -2147483648.0 → INT_MIN
    (call $load_imm64_xmm1 (i32.const 0xC1E00000) (i32.const 0x00000000))
    (call $emit_x86_ucomisd)
    (call $emit_x86_jb_rel8 (i32.const 0))
    (local.set $udf_p (call $get_x86_code_ptr))
    ;; Normal
    (call $emit_x86_cvttsd2si_eax_xmm0)
    (call $emit_x86_jmp_rel8 (i32.const 0))
    (local.set $done_p (call $get_x86_code_ptr))
    ;; .nan
    (call $patch_rel8 (local.get $nan_p))
    (call $emit_x86_xor_eax_eax)
    (call $emit_x86_jmp_rel8 (i32.const 0))
    (local.set $done_p (call $get_x86_code_ptr))
    ;; .max
    (call $patch_rel8 (local.get $ovf_p))
    (call $emit_x86_mov_eax_imm32 (i32.const 0x7FFFFFFF))
    (call $emit_x86_jmp_rel8 (i32.const 0))
    (local.set $done_p (call $get_x86_code_ptr))
    ;; .min
    (call $patch_rel8 (local.get $udf_p))
    (call $emit_x86_mov_eax_imm32 (i32.const 0x80000000))
    ;; .done
    (call $patch_rel8 (local.get $done_p))
    (call $emit_x86_maybe_push_rax)
  )

  ;; i32.trunc_sat_f64_u (0xFC, 0x03)
  (func $template_i32_trunc_sat_f64_u_x86_64
    (local $nan_p i32) (local $ovf_p i32) (local $udf_p i32) (local $done_p i32)
    (call $emit_x86_pop_rax)
    (call $emit_x86_movq_xmm0_rax)
    ;; NaN → 0
    (call $emit_x86_ucomisd_xmm0)
    (call $emit_x86_jp_rel8 (i32.const 0))
    (local.set $nan_p (call $get_x86_code_ptr))
    ;; Overflow: f64 >= 4294967296.0 → UINT_MAX
    (call $load_imm64_xmm1 (i32.const 0x41F00000) (i32.const 0x00000000))
    (call $emit_x86_ucomisd)
    (call $emit_x86_jae_rel8 (i32.const 0))
    (local.set $ovf_p (call $get_x86_code_ptr))
    ;; Underflow: f64 < 0.0 → 0
    (call $load_imm64_xmm1 (i32.const 0x00000000) (i32.const 0x00000000))
    (call $emit_x86_ucomisd)
    (call $emit_x86_jb_rel8 (i32.const 0))
    (local.set $udf_p (call $get_x86_code_ptr))
    ;; Normal
    (call $template_i32_trunc_f64_u_x86_64)
    (call $emit_x86_jmp_rel8 (i32.const 0))
    (local.set $done_p (call $get_x86_code_ptr))
    ;; .nan
    (call $patch_rel8 (local.get $nan_p))
    (call $emit_x86_xor_eax_eax)
    (call $emit_x86_jmp_rel8 (i32.const 0))
    (local.set $done_p (call $get_x86_code_ptr))
    ;; .overflow
    (call $patch_rel8 (local.get $ovf_p))
    (call $emit_x86_mov_eax_imm32 (i32.const 0xFFFFFFFF))
    (call $emit_x86_jmp_rel8 (i32.const 0))
    (local.set $done_p (call $get_x86_code_ptr))
    ;; .underflow
    (call $patch_rel8 (local.get $udf_p))
    (call $emit_x86_xor_eax_eax)
    ;; .done
    (call $patch_rel8 (local.get $done_p))
    (call $emit_x86_maybe_push_rax)
  )

  ;; i64.trunc_sat_f32_s (0xFC, 0x04)
  (func $template_i64_trunc_sat_f32_s_x86_64
    (local $nan_p i32) (local $ovf_p i32) (local $udf_p i32) (local $done_p i32)
    (call $emit_x86_pop_rax)
    (call $emit_x86_movd_xmm0_eax)
    ;; NaN → 0
    (call $emit_x86_ucomiss_xmm0)
    (call $emit_x86_jp_rel8 (i32.const 0))
    (local.set $nan_p (call $get_x86_code_ptr))
    ;; Overflow: f32 >= 9223372036854775808.0 → INT64_MAX
    (call $load_imm32_xmm1 (i32.const 0x5F000000))
    (call $emit_x86_ucomiss)
    (call $emit_x86_jae_rel8 (i32.const 0))
    (local.set $ovf_p (call $get_x86_code_ptr))
    ;; Underflow: f32 < -9223372036854775808.0 → INT64_MIN
    (call $load_imm32_xmm1 (i32.const 0xDF000000))
    (call $emit_x86_ucomiss)
    (call $emit_x86_jb_rel8 (i32.const 0))
    (local.set $udf_p (call $get_x86_code_ptr))
    ;; Normal
    (call $emit_x86_cvttss2si_rax_xmm0)
    (call $emit_x86_jmp_rel8 (i32.const 0))
    (local.set $done_p (call $get_x86_code_ptr))
    ;; .nan: rax = 0
    (call $patch_rel8 (local.get $nan_p))
    (call $emit_x86_rex_w)
    (call $emit_x86_byte (i32.const 0x31))  ;; xor r/m64, r64
    (call $emit_x86_modrm (i32.const 3) (i32.const 0) (i32.const 0))  ;; xor rax, rax
    (call $emit_x86_jmp_rel8 (i32.const 0))
    (local.set $done_p (call $get_x86_code_ptr))
    ;; .max: rax = INT64_MAX
    (call $patch_rel8 (local.get $ovf_p))
    (call $emit_x86_rex_w)
    (call $emit_x86_byte (i32.const 0xB8))
    (call $emit_x86_dword (i32.const 0xFFFFFFFF))
    (call $emit_x86_dword (i32.const 0x7FFFFFFF))  ;; mov rax, 0x7FFFFFFFFFFFFFFF
    (call $emit_x86_jmp_rel8 (i32.const 0))
    (local.set $done_p (call $get_x86_code_ptr))
    ;; .min: rax = INT64_MIN
    (call $patch_rel8 (local.get $udf_p))
    (call $emit_x86_rex_w)
    (call $emit_x86_byte (i32.const 0xB8))
    (call $emit_x86_dword (i32.const 0x00000000))
    (call $emit_x86_dword (i32.const 0x80000000))  ;; mov rax, 0x8000000000000000
    ;; .done
    (call $patch_rel8 (local.get $done_p))
    (call $emit_x86_maybe_push_rax)
  )

  ;; i64.trunc_sat_f32_u (0xFC, 0x05)
  (func $template_i64_trunc_sat_f32_u_x86_64
    (local $nan_p i32) (local $ovf_p i32) (local $udf_p i32) (local $done_p i32)
    (call $emit_x86_pop_rax)
    (call $emit_x86_movd_xmm0_eax)
    ;; NaN → 0
    (call $emit_x86_ucomiss_xmm0)
    (call $emit_x86_jp_rel8 (i32.const 0))
    (local.set $nan_p (call $get_x86_code_ptr))
    ;; Overflow: f32 >= 18446744073709551616.0 → UINT64_MAX
    (call $load_imm32_xmm1 (i32.const 0x5F800000))
    (call $emit_x86_ucomiss)
    (call $emit_x86_jae_rel8 (i32.const 0))
    (local.set $ovf_p (call $get_x86_code_ptr))
    ;; Underflow: f32 < 0.0 → 0
    (call $load_imm32_xmm1 (i32.const 0x00000000))
    (call $emit_x86_ucomiss)
    (call $emit_x86_jb_rel8 (i32.const 0))
    (local.set $udf_p (call $get_x86_code_ptr))
    ;; Normal
    (call $template_i64_trunc_f32_u_x86_64)
    (call $emit_x86_jmp_rel8 (i32.const 0))
    (local.set $done_p (call $get_x86_code_ptr))
    ;; .nan: rax = 0
    (call $patch_rel8 (local.get $nan_p))
    (call $emit_x86_rex_w)
    (call $emit_x86_byte (i32.const 0x31))  ;; xor r/m64, r64
    (call $emit_x86_modrm (i32.const 3) (i32.const 0) (i32.const 0))  ;; xor rax, rax
    (call $emit_x86_jmp_rel8 (i32.const 0))
    (local.set $done_p (call $get_x86_code_ptr))
    ;; .overflow: rax = -1 (UINT64_MAX)
    (call $patch_rel8 (local.get $ovf_p))
    (call $emit_x86_rex_w)
    (call $emit_x86_byte (i32.const 0xB8))
    (call $emit_x86_dword (i32.const 0xFFFFFFFF))
    (call $emit_x86_dword (i32.const 0xFFFFFFFF))  ;; mov rax, -1
    (call $emit_x86_jmp_rel8 (i32.const 0))
    (local.set $done_p (call $get_x86_code_ptr))
    ;; .underflow: rax = 0
    (call $patch_rel8 (local.get $udf_p))
    (call $emit_x86_rex_w)
    (call $emit_x86_byte (i32.const 0x31))
    (call $emit_x86_modrm (i32.const 3) (i32.const 0) (i32.const 0))  ;; xor rax, rax
    ;; .done
    (call $patch_rel8 (local.get $done_p))
    (call $emit_x86_maybe_push_rax)
  )

  ;; i64.trunc_sat_f64_s (0xFC, 0x06)
  (func $template_i64_trunc_sat_f64_s_x86_64
    (local $nan_p i32) (local $ovf_p i32) (local $udf_p i32) (local $done_p i32)
    (call $emit_x86_pop_rax)
    (call $emit_x86_movq_xmm0_rax)
    ;; NaN → 0
    (call $emit_x86_ucomisd_xmm0)
    (call $emit_x86_jp_rel8 (i32.const 0))
    (local.set $nan_p (call $get_x86_code_ptr))
    ;; Overflow: f64 >= 9223372036854775808.0 → INT64_MAX
    (call $load_imm64_xmm1 (i32.const 0x43E00000) (i32.const 0x00000000))
    (call $emit_x86_ucomisd)
    (call $emit_x86_jae_rel8 (i32.const 0))
    (local.set $ovf_p (call $get_x86_code_ptr))
    ;; Underflow: f64 < -9223372036854775808.0 → INT64_MIN
    (call $load_imm64_xmm1 (i32.const 0xC3E00000) (i32.const 0x00000000))
    (call $emit_x86_ucomisd)
    (call $emit_x86_jb_rel8 (i32.const 0))
    (local.set $udf_p (call $get_x86_code_ptr))
    ;; Normal
    (call $emit_x86_cvttsd2si_rax_xmm0)
    (call $emit_x86_jmp_rel8 (i32.const 0))
    (local.set $done_p (call $get_x86_code_ptr))
    ;; .nan
    (call $patch_rel8 (local.get $nan_p))
    (call $emit_x86_rex_w)
    (call $emit_x86_byte (i32.const 0x31))
    (call $emit_x86_modrm (i32.const 3) (i32.const 0) (i32.const 0))  ;; xor rax, rax
    (call $emit_x86_jmp_rel8 (i32.const 0))
    (local.set $done_p (call $get_x86_code_ptr))
    ;; .max
    (call $patch_rel8 (local.get $ovf_p))
    (call $emit_x86_rex_w)
    (call $emit_x86_byte (i32.const 0xB8))
    (call $emit_x86_dword (i32.const 0xFFFFFFFF))
    (call $emit_x86_dword (i32.const 0x7FFFFFFF))
    (call $emit_x86_jmp_rel8 (i32.const 0))
    (local.set $done_p (call $get_x86_code_ptr))
    ;; .min
    (call $patch_rel8 (local.get $udf_p))
    (call $emit_x86_rex_w)
    (call $emit_x86_byte (i32.const 0xB8))
    (call $emit_x86_dword (i32.const 0x00000000))
    (call $emit_x86_dword (i32.const 0x80000000))
    ;; .done
    (call $patch_rel8 (local.get $done_p))
    (call $emit_x86_maybe_push_rax)
  )

  ;; i64.trunc_sat_f64_u (0xFC, 0x07)
  (func $template_i64_trunc_sat_f64_u_x86_64
    (local $nan_p i32) (local $ovf_p i32) (local $udf_p i32) (local $done_p i32)
    (call $emit_x86_pop_rax)
    (call $emit_x86_movq_xmm0_rax)
    ;; NaN → 0
    (call $emit_x86_ucomisd_xmm0)
    (call $emit_x86_jp_rel8 (i32.const 0))
    (local.set $nan_p (call $get_x86_code_ptr))
    ;; Overflow: f64 >= 18446744073709551616.0 → UINT64_MAX
    (call $load_imm64_xmm1 (i32.const 0x43F00000) (i32.const 0x00000000))
    (call $emit_x86_ucomisd)
    (call $emit_x86_jae_rel8 (i32.const 0))
    (local.set $ovf_p (call $get_x86_code_ptr))
    ;; Underflow: f64 < 0.0 → 0
    (call $load_imm64_xmm1 (i32.const 0x00000000) (i32.const 0x00000000))
    (call $emit_x86_ucomisd)
    (call $emit_x86_jb_rel8 (i32.const 0))
    (local.set $udf_p (call $get_x86_code_ptr))
    ;; Normal
    (call $template_i64_trunc_f64_u_x86_64)
    (call $emit_x86_jmp_rel8 (i32.const 0))
    (local.set $done_p (call $get_x86_code_ptr))
    ;; .nan
    (call $patch_rel8 (local.get $nan_p))
    (call $emit_x86_rex_w)
    (call $emit_x86_byte (i32.const 0x31))
    (call $emit_x86_modrm (i32.const 3) (i32.const 0) (i32.const 0))
    (call $emit_x86_jmp_rel8 (i32.const 0))
    (local.set $done_p (call $get_x86_code_ptr))
    ;; .overflow
    (call $patch_rel8 (local.get $ovf_p))
    (call $emit_x86_rex_w)
    (call $emit_x86_byte (i32.const 0xB8))
    (call $emit_x86_dword (i32.const 0xFFFFFFFF))
    (call $emit_x86_dword (i32.const 0xFFFFFFFF))
    (call $emit_x86_jmp_rel8 (i32.const 0))
    (local.set $done_p (call $get_x86_code_ptr))
    ;; .underflow
    (call $patch_rel8 (local.get $udf_p))
    (call $emit_x86_rex_w)
    (call $emit_x86_byte (i32.const 0x31))
    (call $emit_x86_modrm (i32.const 3) (i32.const 0) (i32.const 0))
    ;; .done
    (call $patch_rel8 (local.get $done_p))
    (call $emit_x86_maybe_push_rax)
  )

  ;; Reinterpret ops (0xBC-0xBF) — no-ops on the JIT stack (same bits)
  (func $template_i32_reinterpret_f32)
  (func $template_i64_reinterpret_f64)
  (func $template_f32_reinterpret_i32)
  (func $template_f64_reinterpret_i64)

  ;; ── Float load/store templates ────────────────────────────────────

  ;; f32.load (0x2A)
  (func $template_f32_load_x86_64
    (call $emit_x86_pop_reg (i32.const 1))    ;; pop rcx = addr
    (call $emit_x86_mem_load32)
    (call $emit_x86_maybe_push_rax)
  )

  ;; f64.load (0x2B)
  (func $template_f64_load_x86_64
    (call $emit_x86_pop_reg (i32.const 1))    ;; pop rcx = addr
    (call $emit_x86_mem_load64)
    (call $emit_x86_maybe_push_rax)
  )

  ;; f32.store (0x38)
  (func $template_f32_store_x86_64
    (call $emit_x86_pop_reg (i32.const 1))    ;; pop rcx = addr
    (call $emit_x86_pop_rax)                   ;; pop rax = value
    (call $emit_x86_mem_store32)
  )

  ;; f64.store (0x39)
  (func $template_f64_store_x86_64
    (call $emit_x86_pop_reg (i32.const 1))    ;; pop rcx = addr
    (call $emit_x86_pop_rax)                   ;; pop rax = value
    (call $emit_x86_mem_store64)
  )

  ;; ── Memory load/store templates ──────────────────────────────────

  (func $template_i32_load_x86_64
    (call $emit_x86_pop_reg (i32.const 1))    ;; pop rcx
    (call $emit_x86_mem_load32)
    (call $emit_x86_maybe_push_rax)
  )

  (func $template_i64_load_x86_64
    (call $emit_x86_pop_reg (i32.const 1))
    (call $emit_x86_mem_load64)
    (call $emit_x86_maybe_push_rax)
  )

  (func $template_i32_load8_s_x86_64
    (call $emit_x86_pop_reg (i32.const 1))
    (call $emit_x86_mem_load8_s)
    (call $emit_x86_maybe_push_rax)
  )

  (func $template_i32_load8_u_x86_64
    (call $emit_x86_pop_reg (i32.const 1))
    (call $emit_x86_mem_load8_u)
    (call $emit_x86_maybe_push_rax)
  )

  (func $template_i32_load16_s_x86_64
    (call $emit_x86_pop_reg (i32.const 1))
    (call $emit_x86_mem_load16_s)
    (call $emit_x86_maybe_push_rax)
  )

  (func $template_i32_load16_u_x86_64
    (call $emit_x86_pop_reg (i32.const 1))
    (call $emit_x86_mem_load16_u)
    (call $emit_x86_maybe_push_rax)
  )

  (func $template_i32_store_x86_64
    (call $emit_x86_pop_reg (i32.const 1))    ;; pop rcx = addr
    (call $emit_x86_pop_rax)                   ;; pop rax = value
    (call $emit_x86_mem_store32)
  )

  (func $template_i64_store_x86_64
    (call $emit_x86_pop_reg (i32.const 1))
    (call $emit_x86_pop_rax)
    (call $emit_x86_mem_store64)
  )

  (func $template_i32_store8_x86_64
    (call $emit_x86_pop_reg (i32.const 1))
    (call $emit_x86_pop_rax)
    (call $emit_x86_mem_store8)
  )

  (func $template_i32_store16_x86_64
    (call $emit_x86_pop_reg (i32.const 1))
    (call $emit_x86_pop_rax)
    (call $emit_x86_mem_store16)
  )

  ;; i64.load8_s (0x30)
  (func $template_i64_load8_s_x86_64
    (call $emit_x86_pop_reg (i32.const 1))
    (call $emit_x86_mem_load8_s_64)
    (call $emit_x86_maybe_push_rax)
  )

  ;; i64.load8_u (0x31)
  (func $template_i64_load8_u_x86_64
    (call $emit_x86_pop_reg (i32.const 1))
    (call $emit_x86_mem_load8_u)
    (call $emit_x86_maybe_push_rax)
  )

  ;; i64.load16_s (0x32)
  (func $template_i64_load16_s_x86_64
    (call $emit_x86_pop_reg (i32.const 1))
    (call $emit_x86_mem_load16_s_64)
    (call $emit_x86_maybe_push_rax)
  )

  ;; i64.load16_u (0x33)
  (func $template_i64_load16_u_x86_64
    (call $emit_x86_pop_reg (i32.const 1))
    (call $emit_x86_mem_load16_u)
    (call $emit_x86_maybe_push_rax)
  )

  ;; i64.load32_s (0x34)
  (func $template_i64_load32_s_x86_64
    (call $emit_x86_pop_reg (i32.const 1))
    (call $emit_x86_mem_load32_s)
    (call $emit_x86_maybe_push_rax)
  )

  ;; i64.load32_u (0x35)
  (func $template_i64_load32_u_x86_64
    (call $emit_x86_pop_reg (i32.const 1))
    (call $emit_x86_mem_load32)
    (call $emit_x86_maybe_push_rax)
  )

  ;; i64.store8 (0x3C)
  (func $template_i64_store8_x86_64
    (call $emit_x86_pop_reg (i32.const 1))
    (call $emit_x86_pop_rax)
    (call $emit_x86_mem_store8)
  )

  ;; i64.store16 (0x3D)
  (func $template_i64_store16_x86_64
    (call $emit_x86_pop_reg (i32.const 1))
    (call $emit_x86_pop_rax)
    (call $emit_x86_mem_store16)
  )

  ;; i64.store32 (0x3E)
  (func $template_i64_store32_x86_64
    (call $emit_x86_pop_reg (i32.const 1))
    (call $emit_x86_pop_rax)
    (call $emit_x86_mem_store32)
  )

  ;; ── memory.size (0x3F) ──────────────────────────────────────────
  (func $template_memory_size_x86_64
    (call $emit_x86_load_r15_to_rdx (i32.const 64))   ;; JitGlobals.memory_pages
    (call $emit_x86_load_rax_rdx_disp (i32.const 0))
    (call $emit_x86_maybe_push_rax)
  )

  ;; memory.grow (0x40)
  (func $template_memory_grow_x86_64
    (call $emit_x86_pop_rax)
    ;; push result = memory.grow - not implementable in JIT without host calls
    (call $emit_x86_xor_eax_eax)  ;; return 0 (success, no grow)
    (call $emit_x86_maybe_push_rax)
  )

  ;; ref.null (0xD0): push 0 (null reference)
  (func $template_ref_null_x86_64
    (call $emit_x86_xor_eax_eax)
    (call $emit_x86_maybe_push_rax)
  )

  ;; ref.is_null (0xD1): pop, test if zero, push i32 result
  (func $template_ref_is_null_x86_64
    (call $emit_x86_pop_rax)
    (call $emit_x86_test_eax)
    (call $emit_x86_setcc_and_push (i32.const 0x94))  ;; sete
  )

  ;; ref.func (0xD2): push function reference (from imm0)
  (func $template_ref_func_x86_64 (param $dec_ptr i32)
    (call $emit_x86_mov_eax_imm32 (i32.load (i32.add (local.get $dec_ptr) (i32.const 4))))
    (call $emit_x86_maybe_push_rax)
  )

  (func $template_unsupported_x86_64 (param $dec_ptr i32)
    ;; just emit ud2 and nop
    (call $emit_x86_ud2)
  )

  ;; ── Call / import syscall template ─────────────────────────────────

  ;; call (0x10): if func_idx < import_count, emit inline syscall.
  ;; Otherwise emit unsupported (no multi-function JIT yet).
  (func $template_call_x86_64 (param $dec_ptr i32)
    (local $func_idx i32) (local $import_count i32) (local $type_off i32)
    (local $param_count i32) (local $sysno i32) (local $i i32)

    (local.set $func_idx (i32.load (i32.add (local.get $dec_ptr) (i32.const 4))))
    (local.set $import_count (i32.load (global.get $OFF_IMPORT_COUNT)))

    (if (i32.lt_u (local.get $func_idx) (local.get $import_count))
      (then
        ;; ── Imported function → emit syscall inline ────────────────
        ;; Read type_index from functions buffer (SZ_FUNC=16 per entry)
        (local.set $type_off
          (i32.shl
            (i32.load (i32.add (global.get $OFF_FUNCTIONS_BUF) (i32.shl (local.get $func_idx) (i32.const 4))))
            (i32.const 8)
          )
        )
        ;; Read param_count from type entry (16-bit at offset 128, SZ_TYPE=256)
        (local.set $param_count
          (i32.load16_u
            (i32.add (i32.add (global.get $OFF_TYPES_BUF) (local.get $type_off)) (i32.const 128))
          )
        )
        ;; Read syscall number from map table (default: sysno = import index)
        (local.set $sysno
          (i32.load (i32.add (global.get $OFF_SYSCALL_MAP) (i32.shl (local.get $func_idx) (i32.const 2))))
        )

        ;; Emit: allocate 64-byte struct (48 for params, 16 for host iovec)
        (call $emit_x86_sub_rsp_imm (i32.const 64))

        ;; Zero-fill all 8 slots: xor eax,eax; store at each offset
        (call $emit_x86_xor_eax_eax)
        (local.set $i (i32.const 0))
        (block $zfill
          (loop $zloop
            (if (i32.eq (local.get $i) (i32.const 8)) (then (br $zfill)))
            (call $emit_x86_store_rax_rsp_disp (i32.shl (local.get $i) (i32.const 3)))
            (local.set $i (i32.add (local.get $i) (i32.const 1)))
            (br $zloop)
          )
        )

        ;; Pop param_count args and store at offsets 0, 8, 16, 24, 32, 40
        ;; First arg popped goes to highest offset, last arg to offset 0
        (local.set $i (local.get $param_count))
        (block $pop_args
          (loop $pop_loop
            (if (i32.eqz (local.get $i)) (then (br $pop_args)))
            (local.set $i (i32.sub (local.get $i) (i32.const 1)))
            (call $emit_x86_pop_rax)
            (call $emit_x86_store_rax_rsp_disp (i32.shl (local.get $i) (i32.const 3)))
            (br $pop_loop)
          )
        )

        ;; Load syscall regs from struct
        (call $emit_x86_load_rdi_rsp_disp (i32.const 0))
        (call $emit_x86_load_rsi_rsp_disp (i32.const 8))
        (call $emit_x86_load_rdx_rsp_disp (i32.const 16))
        (call $emit_x86_load_r10_rsp_disp (i32.const 24))
        (call $emit_x86_load_r8_rsp_disp (i32.const 32))
        (call $emit_x86_load_r9_rsp_disp (i32.const 40))

        ;; Check for special WASI syscalls (negative sysno)
        (if (i32.lt_s (local.get $sysno) (i32.const 0))
          (then
            ;; ── args_sizes_get (-1) ─────────────────────────────────
            ;; Handle inline: read argc from JitGlobals, write to output
            ;; rdi = argc_out (WASM relative), rsi = buf_size_out (WASM relative)
            (if (i32.eq (local.get $sysno) (i32.const -1))
              (then
                ;; mov rax, [r15+8]    (load mem_ptr)
                (call $emit_x86_byte (i32.const 0x49))
                (call $emit_x86_byte (i32.const 0x8B))
                (call $emit_x86_byte (i32.const 0x47))
                (call $emit_x86_byte (i32.const 0x08))
                ;; mov r9d, [r15+72]   (argc)
                (call $emit_x86_byte (i32.const 0x45))
                (call $emit_x86_byte (i32.const 0x8B))
                (call $emit_x86_byte (i32.const 0x4F))
                (call $emit_x86_byte (i32.const 72))
                ;; mov [rax+rdi], r9d  (write argc to *argc_out)
                (call $emit_x86_byte (i32.const 0x44))
                (call $emit_x86_byte (i32.const 0x89))
                (call $emit_x86_byte (i32.const 0x0C))
                (call $emit_x86_byte (i32.const 0x38))
                ;; imul ecx, r9d, 1024 (total_size = argc * 1024)
                (call $emit_x86_byte (i32.const 0x41))
                (call $emit_x86_byte (i32.const 0x69))
                (call $emit_x86_byte (i32.const 0xC9))
                (call $emit_x86_byte (i32.const 0x00))
                (call $emit_x86_byte (i32.const 0x04))
                (call $emit_x86_byte (i32.const 0x00))
                (call $emit_x86_byte (i32.const 0x00))
                ;; mov [rax+rsi], ecx  (write total_size to *buf_size_out)
                (call $emit_x86_byte (i32.const 0x40))
                (call $emit_x86_byte (i32.const 0x89))
                (call $emit_x86_byte (i32.const 0x0C))
                (call $emit_x86_byte (i32.const 0x30))
                ;; xor eax, eax (return 0)
                (call $emit_x86_byte (i32.const 0x31))
                (call $emit_x86_byte (i32.const 0xC0))
              )
            )
            ;; ── args_get (-2) ───────────────────────────────────────
            ;; For now: just write argc to *argv_buf, return 0
            (if (i32.eq (local.get $sysno) (i32.const -2))
              (then
                ;; mov rax, [r15+8]    (load mem_ptr)
                (call $emit_x86_byte (i32.const 0x49))
                (call $emit_x86_byte (i32.const 0x8B))
                (call $emit_x86_byte (i32.const 0x47))
                (call $emit_x86_byte (i32.const 0x08))
                ;; mov r9d, [r15+72]   (argc)
                (call $emit_x86_byte (i32.const 0x45))
                (call $emit_x86_byte (i32.const 0x8B))
                (call $emit_x86_byte (i32.const 0x4F))
                (call $emit_x86_byte (i32.const 72))
                ;; mov [rax+rdi], r9d  (write argc to *argv_buf)
                (call $emit_x86_byte (i32.const 0x44))
                (call $emit_x86_byte (i32.const 0x89))
                (call $emit_x86_byte (i32.const 0x0C))
                (call $emit_x86_byte (i32.const 0x38))
                ;; xor eax, eax (return 0)
                (call $emit_x86_byte (i32.const 0x31))
                (call $emit_x86_byte (i32.const 0xC0))
              )
            )
          )
          (else
            ;; ── fd_write (SYS_writev=20) inline wrapper ──────────
            (if (i32.eq (local.get $sysno) (i32.const 20))
              (then
                ;; After template's load_regs:
                ;;   rdi = fd (rsp+0), rsi = iovs WASM offset (rsp+8)
                ;;   rdx = iovs_len (rsp+16), r10 = nwritten WASM offset (rsp+24)
                ;;   r14 = mem_ptr (set by stub at _start)
                ;;
                ;; Convert WASM iovec[0] to host iovec at rsp+48:
                (call $emit_x86_add_rsi_r14)         ;; rsi += r14 (host addr of WASM iovec)
                (call $emit_x86_mov_eax_ind_rsi)     ;; eax = [rsi] (buf_offset)
                (call $emit_x86_add_rax_r14)         ;; rax += r14 (host_buf = mem_ptr + offset)
                (call $emit_x86_store_rax_rsp_disp (i32.const 48))  ;; [rsp+48] = host iov_base
                (call $emit_x86_mov_eax_ind_rsi_4)   ;; eax = [rsi+4] (buf_len)
                (call $emit_x86_store_rax_rsp_disp (i32.const 56))  ;; [rsp+56] = host iov_len
                (call $emit_x86_lea_rsi_rsp_disp (i32.const 48))    ;; rsi = &host iovec
                ;; rdi already has fd (loaded by template)
                (call $emit_x86_mov_edx_imm32 (i32.const 1))         ;; edx = 1 (iovcnt)
                (call $emit_x86_mov_eax_imm (i32.const 20))          ;; eax = SYS_writev
                (call $emit_x86_syscall)
                ;; Write result to *nwritten (WASM *nwritten = (r14 + [rsp+24]))
                (call $emit_x86_add_r10_r14)         ;; r10 += r14 (host addr of nwritten)
                (call $emit_x86_mov_dword_r10)       ;; [r10] = eax (*nwritten = result)
              )
              (else
                ;; Normal syscall: set eax = sysno, execute syscall
                (call $emit_x86_mov_eax_imm (local.get $sysno))
                (call $emit_x86_syscall)
              )
            )
          )
        )

        ;; Deallocate struct
        (call $emit_x86_add_rsp_imm (i32.const 64))

        ;; Push return value onto WASM stack
        (call $emit_x86_byte (i32.const 0x50))  ;; push rax
      )
      (else
        ;; ── Local function → emit call rel32 with fixup ─────────────
        ;; Calling convention: r12 = param0, r13 = param1
        ;; Caller saves/restores rbx, r12, r13 on x86-64 stack
        (call $emit_x86_byte (i32.const 0x53))       ;; push rbx
        (call $emit_x86_byte (i32.const 0x41))       ;; REX.B
        (call $emit_x86_byte (i32.const 0x54))       ;; push r12
        (call $emit_x86_byte (i32.const 0x41))       ;; REX.B
        (call $emit_x86_byte (i32.const 0x55))       ;; push r13

        ;; Read param count from type entry
        (local.set $type_off
          (i32.shl
            (i32.load (i32.add (global.get $OFF_FUNCTIONS_BUF) (i32.shl (local.get $func_idx) (i32.const 4))))
            (i32.const 8)
          )
        )
        (local.set $param_count
          (i32.load16_u
            (i32.add (i32.add (global.get $OFF_TYPES_BUF) (local.get $type_off)) (i32.const 128))
          )
        )

        ;; Pop params from WASM stack into r12/r13
        (if (i32.eqz (local.get $param_count))
          (then
            (call $emit_x86_xor_eax_eax)   ;; xor eax,eax
            (call $emit_x86_mov_r12_rax)    ;; r12 = 0
            (call $emit_x86_mov_r13_rax)    ;; r13 = 0
          )
        )
        (if (i32.eq (local.get $param_count) (i32.const 1))
          (then
            (call $emit_x86_pop_rax)        ;; pop param0
            (call $emit_x86_mov_r12_rax)    ;; r12 = param0
            (call $emit_x86_xor_eax_eax)
            (call $emit_x86_mov_r13_rax)    ;; r13 = 0
          )
        )
        (if (i32.eq (local.get $param_count) (i32.const 2))
          (then
            (call $emit_x86_pop_rax)        ;; pop param1
            (call $emit_x86_mov_r13_rax)    ;; r13 = param1
            (call $emit_x86_pop_rax)        ;; pop param0
            (call $emit_x86_mov_r12_rax)    ;; r12 = param0
          )
        )

        ;; Emit call rel32 placeholder with fixup record
        (call $emit_call_rel32_fixup (local.get $func_idx))

        ;; Push return value onto WASM stack (rax from callee)
        (call $emit_x86_byte (i32.const 0x50))  ;; push rax

        ;; Restore caller's r13, r12, rbx
        (call $emit_x86_byte (i32.const 0x41))  ;; REX.B
        (call $emit_x86_byte (i32.const 0x5D))  ;; pop r13
        (call $emit_x86_byte (i32.const 0x41))  ;; REX.B
        (call $emit_x86_byte (i32.const 0x5C))  ;; pop r12
        (call $emit_x86_byte (i32.const 0x5B))  ;; pop rbx
      )
    )
  )

  ;; ── Float arithmetic templates ───────────────────────────────────

  ;; f32.add: pop rcx, pop rax, movd xmm1,ecx, movd xmm0,eax, addss, movd, push
  (func $template_f32_add_x86_64
    (call $emit_x86_pop2_rcx_rax)
    (call $emit_x86_movd_xmm1_ecx)
    (call $emit_x86_movd_xmm0_eax)
    (call $emit_x86_sse_op (i32.const 0xF3) (i32.const 0x58))  ;; addss
    (call $emit_x86_movd_eax_xmm0)
    (call $emit_x86_maybe_push_rax)
  )

  (func $template_f32_sub_x86_64
    (call $emit_x86_pop2_rcx_rax)
    (call $emit_x86_movd_xmm1_ecx)
    (call $emit_x86_movd_xmm0_eax)
    (call $emit_x86_sse_op (i32.const 0xF3) (i32.const 0x5C))  ;; subss
    (call $emit_x86_movd_eax_xmm0)
    (call $emit_x86_maybe_push_rax)
  )

  (func $template_f32_mul_x86_64
    (call $emit_x86_pop2_rcx_rax)
    (call $emit_x86_movd_xmm1_ecx)
    (call $emit_x86_movd_xmm0_eax)
    (call $emit_x86_sse_op (i32.const 0xF3) (i32.const 0x59))  ;; mulss
    (call $emit_x86_movd_eax_xmm0)
    (call $emit_x86_maybe_push_rax)
  )

  (func $template_f32_div_x86_64
    (call $emit_x86_pop2_rcx_rax)
    (call $emit_x86_movd_xmm1_ecx)
    (call $emit_x86_movd_xmm0_eax)
    (call $emit_x86_sse_op (i32.const 0xF3) (i32.const 0x5E))  ;; divss
    (call $emit_x86_movd_eax_xmm0)
    (call $emit_x86_maybe_push_rax)
  )

  ;; f32.min (0x96): minss + NaN fixup
  (func $template_f32_min_x86_64
    (local $nan_p i32) (local $done_p i32)
    (call $emit_x86_pop2_rcx_rax)
    (call $emit_x86_movd_xmm1_ecx)
    (call $emit_x86_movd_xmm0_eax)
    (call $emit_x86_sse_op (i32.const 0xF3) (i32.const 0x5D))  ;; minss
    ;; NaN fixup: ucomiss xmm0, xmm0 → JP → canonical NaN
    (call $emit_x86_ucomiss_xmm0)
    (call $emit_x86_jnp_rel8 (i32.const 0))
    (local.set $done_p (call $get_x86_code_ptr))
    ;; is NaN: push canonical NaN, pop to xmm0
    (call $emit_x86_push_imm32 (i32.const 0x7FC00000))
    (call $emit_x86_pop_rax)
    (call $emit_x86_movd_xmm0_eax)
    (call $patch_rel8 (local.get $done_p))
    (call $emit_x86_movd_eax_xmm0)
    (call $emit_x86_maybe_push_rax)
  )

  ;; f32.max (0x97): maxss + NaN fixup
  (func $template_f32_max_x86_64
    (local $nan_p i32) (local $done_p i32)
    (call $emit_x86_pop2_rcx_rax)
    (call $emit_x86_movd_xmm1_ecx)
    (call $emit_x86_movd_xmm0_eax)
    (call $emit_x86_sse_op (i32.const 0xF3) (i32.const 0x5F))  ;; maxss
    (call $emit_x86_ucomiss_xmm0)
    (call $emit_x86_jnp_rel8 (i32.const 0))
    (local.set $done_p (call $get_x86_code_ptr))
    (call $emit_x86_push_imm32 (i32.const 0x7FC00000))
    (call $emit_x86_pop_rax)
    (call $emit_x86_movd_xmm0_eax)
    (call $patch_rel8 (local.get $done_p))
    (call $emit_x86_movd_eax_xmm0)
    (call $emit_x86_maybe_push_rax)
  )

  ;; f32.copysign (0x98): result = abs(left) | signbit(right)
  (func $template_f32_copysign_x86_64
    (call $emit_x86_pop_reg (i32.const 1))  ;; pop rcx = right
    (call $emit_x86_pop_rax)                 ;; pop rax = left
    (call $emit_x86_btr_eax_imm8 (i32.const 31))  ;; clear sign bit of left
    (call $emit_x86_and_ecx_imm32 (i32.const 0x80000000))  ;; isolate sign of right
    (call $emit_x86_ror32)  ;; or eax, ecx
    (call $emit_x86_maybe_push_rax)
  )

  ;; ── f64 binary templates ──────────────────────────────────────────

  (func $template_f64_add_x86_64
    (call $emit_x86_pop2_rcx_rax)
    (call $emit_x86_movq_xmm1_rcx)
    (call $emit_x86_movq_xmm0_rax)
    (call $emit_x86_sse_op (i32.const 0xF2) (i32.const 0x58))  ;; addsd
    (call $emit_x86_movq_rax_xmm0)
    (call $emit_x86_maybe_push_rax)
  )

  (func $template_f64_sub_x86_64
    (call $emit_x86_pop2_rcx_rax)
    (call $emit_x86_movq_xmm1_rcx)
    (call $emit_x86_movq_xmm0_rax)
    (call $emit_x86_sse_op (i32.const 0xF2) (i32.const 0x5C))
    (call $emit_x86_movq_rax_xmm0)
    (call $emit_x86_maybe_push_rax)
  )

  (func $template_f64_mul_x86_64
    (call $emit_x86_pop2_rcx_rax)
    (call $emit_x86_movq_xmm1_rcx)
    (call $emit_x86_movq_xmm0_rax)
    (call $emit_x86_sse_op (i32.const 0xF2) (i32.const 0x59))
    (call $emit_x86_movq_rax_xmm0)
    (call $emit_x86_maybe_push_rax)
  )

  (func $template_f64_div_x86_64
    (call $emit_x86_pop2_rcx_rax)
    (call $emit_x86_movq_xmm1_rcx)
    (call $emit_x86_movq_xmm0_rax)
    (call $emit_x86_sse_op (i32.const 0xF2) (i32.const 0x5E))
    (call $emit_x86_movq_rax_xmm0)
    (call $emit_x86_maybe_push_rax)
  )

  ;; f64.min (0xA4): minsd + NaN fixup
  (func $template_f64_min_x86_64
    (local $nan_p i32) (local $done_p i32)
    (call $emit_x86_pop2_rcx_rax)
    (call $emit_x86_movq_xmm1_rcx)
    (call $emit_x86_movq_xmm0_rax)
    (call $emit_x86_sse_op (i32.const 0xF2) (i32.const 0x5D))  ;; minsd
    (call $emit_x86_ucomisd_xmm0)
    (call $emit_x86_jnp_rel8 (i32.const 0))
    (local.set $done_p (call $get_x86_code_ptr))
    (call $emit_x86_rex_w)
    (call $emit_x86_byte (i32.const 0xB8))
    (call $emit_x86_dword (i32.const 0x00000000))
    (call $emit_x86_dword (i32.const 0x7FF80000))  ;; mov rax, 0x7FF8000000000000 (canonical NaN f64)
    (call $emit_x86_movq_xmm0_rax)
    (call $patch_rel8 (local.get $done_p))
    (call $emit_x86_movq_rax_xmm0)
    (call $emit_x86_maybe_push_rax)
  )

  ;; f64.max (0xA5): maxsd + NaN fixup
  (func $template_f64_max_x86_64
    (local $nan_p i32) (local $done_p i32)
    (call $emit_x86_pop2_rcx_rax)
    (call $emit_x86_movq_xmm1_rcx)
    (call $emit_x86_movq_xmm0_rax)
    (call $emit_x86_sse_op (i32.const 0xF2) (i32.const 0x5F))  ;; maxsd
    (call $emit_x86_ucomisd_xmm0)
    (call $emit_x86_jnp_rel8 (i32.const 0))
    (local.set $done_p (call $get_x86_code_ptr))
    (call $emit_x86_rex_w)
    (call $emit_x86_byte (i32.const 0xB8))
    (call $emit_x86_dword (i32.const 0x00000000))
    (call $emit_x86_dword (i32.const 0x7FF80000))
    (call $emit_x86_movq_xmm0_rax)
    (call $patch_rel8 (local.get $done_p))
    (call $emit_x86_movq_rax_xmm0)
    (call $emit_x86_maybe_push_rax)
  )

  ;; f64.copysign (0xA6): result = abs(left) | signbit(right)
  (func $template_f64_copysign_x86_64
    (call $emit_x86_pop_reg (i32.const 1))  ;; pop rcx = right
    (call $emit_x86_pop_rax)                 ;; pop rax = left
    (call $emit_x86_btr_rax_imm8 (i32.const 63))  ;; clear sign of left for magnitude
    ;; isolate sign bit of right: shl rcx, 1; rcr rcx, 1
    (call $emit_x86_rex_w)
    (call $emit_x86_byte (i32.const 0xD1))
    (call $emit_x86_modrm (i32.const 3) (i32.const 4) (i32.const 1))  ;; shl rcx, 1
    (call $emit_x86_rex_w)
    (call $emit_x86_byte (i32.const 0xD1))
    (call $emit_x86_modrm (i32.const 3) (i32.const 3) (i32.const 1))  ;; rcr rcx, 1
    (call $emit_x86_xor64)  ;; or rax, rcx
    (call $emit_x86_maybe_push_rax)
  )

  ;; ── f32 comparison templates ──────────────────────────────────────

  (func $template_f32_eq_x86_64
    (call $emit_x86_f32_cmp_prologue)
    (call $emit_x86_setnp_save_ah)
    (call $emit_x86_setcc_and_push (i32.const 0x94))   ;; sete
  )

  (func $template_f32_ne_x86_64
    (call $emit_x86_f32_cmp_prologue)
    (call $emit_x86_setp_save_ah)
    (call $emit_x86_setcc_or_push (i32.const 0x95))    ;; setne
  )

  (func $template_f32_lt_x86_64
    (call $emit_x86_f32_cmp_prologue)
    (call $emit_x86_setnp_save_ah)
    (call $emit_x86_setcc_and_push (i32.const 0x92))   ;; setb
  )

  (func $template_f32_gt_x86_64
    (call $emit_x86_f32_cmp_prologue)
    (call $emit_x86_setcc (i32.const 0x97))            ;; seta (NaN-safe: CF=1 → seta=0)
    (call $emit_x86_movzx_eax_al)
    (call $emit_x86_maybe_push_rax)
  )

  (func $template_f32_le_x86_64
    (call $emit_x86_f32_cmp_prologue)
    (call $emit_x86_setnp_save_ah)
    (call $emit_x86_setcc_and_push (i32.const 0x96))   ;; setbe
  )

  (func $template_f32_ge_x86_64
    (call $emit_x86_f32_cmp_prologue)
    (call $emit_x86_setcc (i32.const 0x93))            ;; setae
    (call $emit_x86_movzx_eax_al)
    (call $emit_x86_maybe_push_rax)
  )

  ;; ── f64 comparison templates ──────────────────────────────────────

  (func $template_f64_eq_x86_64
    (call $emit_x86_f64_cmp_prologue)
    (call $emit_x86_setnp_save_ah)
    (call $emit_x86_setcc_and_push (i32.const 0x94))
  )

  (func $template_f64_ne_x86_64
    (call $emit_x86_f64_cmp_prologue)
    (call $emit_x86_setp_save_ah)
    (call $emit_x86_setcc_or_push (i32.const 0x95))
  )

  (func $template_f64_lt_x86_64
    (call $emit_x86_f64_cmp_prologue)
    (call $emit_x86_setnp_save_ah)
    (call $emit_x86_setcc_and_push (i32.const 0x92))
  )

  (func $template_f64_gt_x86_64
    (call $emit_x86_f64_cmp_prologue)
    (call $emit_x86_setcc (i32.const 0x97))
    (call $emit_x86_movzx_eax_al)
    (call $emit_x86_maybe_push_rax)
  )

  (func $template_f64_le_x86_64
    (call $emit_x86_f64_cmp_prologue)
    (call $emit_x86_setnp_save_ah)
    (call $emit_x86_setcc_and_push (i32.const 0x96))
  )

  (func $template_f64_ge_x86_64
    (call $emit_x86_f64_cmp_prologue)
    (call $emit_x86_setcc (i32.const 0x93))
    (call $emit_x86_movzx_eax_al)
    (call $emit_x86_maybe_push_rax)
  )

  ;; ── Float unary templates ────────────────────────────────────────

  (func $template_f32_abs_x86_64
    ;; and eax, 0x7FFFFFFF  (bits: clear sign bit)
    (call $emit_x86_pop_rax)
    (call $emit_x86_and_eax_imm32 (i32.const 0x7FFFFFFF))
    (call $emit_x86_maybe_push_rax)
  )

  (func $template_f32_neg_x86_64
    ;; xor eax, 0x80000000  (bits: flip sign bit)
    (call $emit_x86_pop_rax)
    (call $emit_x86_xor_eax_imm32 (i32.const 0x80000000))
    (call $emit_x86_maybe_push_rax)
  )

  (func $template_f32_sqrt_x86_64
    (call $emit_x86_f32_unop_prologue)
    (call $emit_x86_sse_op (i32.const 0xF3) (i32.const 0x51))  ;; sqrtss
    (call $emit_x86_f32_unop_epilogue)
  )

  (func $template_f32_ceil_x86_64
    (call $emit_x86_f32_unop_prologue)
    (call $emit_x86_sse3a_op (i32.const 0x0A) (i32.const 2))   ;; roundss, ceil
    (call $emit_x86_f32_unop_epilogue)
  )

  (func $template_f32_floor_x86_64
    (call $emit_x86_f32_unop_prologue)
    (call $emit_x86_sse3a_op (i32.const 0x0A) (i32.const 1))   ;; roundss, floor
    (call $emit_x86_f32_unop_epilogue)
  )

  (func $template_f32_trunc_x86_64
    (call $emit_x86_f32_unop_prologue)
    (call $emit_x86_sse3a_op (i32.const 0x0A) (i32.const 3))   ;; roundss, trunc
    (call $emit_x86_f32_unop_epilogue)
  )

  (func $template_f32_nearest_x86_64
    (call $emit_x86_f32_unop_prologue)
    (call $emit_x86_sse3a_op (i32.const 0x0A) (i32.const 0))   ;; roundss, nearest
    (call $emit_x86_f32_unop_epilogue)
  )

  ;; ── f32.const (0x43): push 4-byte constant ──────────────────────────
  (func $template_f32_const_x86_64 (param $dec_ptr i32)
    (call $emit_x86_push_imm32 (i32.load (i32.add (local.get $dec_ptr) (i32.const 4))))
  )

  ;; ── f64.const (0x44): push 8-byte constant ──────────────────────────
  (func $template_f64_const_x86_64 (param $dec_ptr i32)
    (local $val i64)
    (local.set $val (i64.load (i32.add (local.get $dec_ptr) (i32.const 4))))
    (call $emit_x86_rex_w)
    (call $emit_x86_byte (i32.const 0xB8))
    (call $emit_x86_qword (local.get $val))
    (call $emit_x86_maybe_push_rax)
  )

  (func $template_f64_abs_x86_64
    ;; btr rax, 63: clear the sign bit (5 bytes: 48 0F BA F0 3F)
    ;; ModRM /6 = BTR (bit test and reset)
    (call $emit_x86_pop_rax)
    (call $emit_x86_rex_w)
    (call $emit_x86_byte (i32.const 0x0F))
    (call $emit_x86_byte (i32.const 0xBA))
    (call $emit_x86_modrm (i32.const 3) (i32.const 6) (i32.const 0))
    (call $emit_x86_byte (i32.const 0x3F))
    (call $emit_x86_maybe_push_rax)
  )

  (func $template_f64_neg_x86_64
    ;; bts rax, 63: flip the sign bit (5 bytes: 48 0F BA F8 3F)
    ;; ModRM /5 = BTS (bit test and set)
    (call $emit_x86_pop_rax)
    (call $emit_x86_rex_w)
    (call $emit_x86_byte (i32.const 0x0F))
    (call $emit_x86_byte (i32.const 0xBA))
    (call $emit_x86_modrm (i32.const 3) (i32.const 5) (i32.const 0))
    (call $emit_x86_byte (i32.const 0x3F))
    (call $emit_x86_maybe_push_rax)
  )

  (func $template_f64_sqrt_x86_64
    (call $emit_x86_f64_unop_prologue)
    (call $emit_x86_sse_op (i32.const 0xF2) (i32.const 0x51))  ;; sqrtsd
    (call $emit_x86_f64_unop_epilogue)
  )

  (func $template_f64_ceil_x86_64
    (call $emit_x86_f64_unop_prologue)
    (call $emit_x86_sse3a_op (i32.const 0x0B) (i32.const 2))   ;; roundsd, ceil
    (call $emit_x86_f64_unop_epilogue)
  )

  (func $template_f64_floor_x86_64
    (call $emit_x86_f64_unop_prologue)
    (call $emit_x86_sse3a_op (i32.const 0x0B) (i32.const 1))   ;; roundsd, floor
    (call $emit_x86_f64_unop_epilogue)
  )

  (func $template_f64_trunc_x86_64
    (call $emit_x86_f64_unop_prologue)
    (call $emit_x86_sse3a_op (i32.const 0x0B) (i32.const 3))   ;; roundsd, trunc
    (call $emit_x86_f64_unop_epilogue)
  )

  (func $template_f64_nearest_x86_64
    (call $emit_x86_f64_unop_prologue)
    (call $emit_x86_sse3a_op (i32.const 0x0B) (i32.const 0))   ;; roundsd, nearest
    (call $emit_x86_f64_unop_epilogue)
  )

  ;; ═════════════════════════════════════════════════════════════════════
  ;; Label / fixup helpers (for control flow compilation)
  ;; ═════════════════════════════════════════════════════════════════════

  (func $push_label (param $kind i32)
    (local $depth i32)
    (local.set $depth (i32.load (global.get $JS_LABEL_DEPTH_x86_64)))
    (i32.store8 (i32.add (global.get $JS_LABEL_KINDS_x86_64) (local.get $depth)) (local.get $kind))
    (i32.store (i32.add (global.get $JS_LABEL_IF_JZ_x86_64) (i32.shl (local.get $depth) (i32.const 2))) (i32.const 0))
    (i32.store (global.get $JS_LABEL_DEPTH_x86_64) (i32.add (local.get $depth) (i32.const 1)))
  )

  (func $pop_label (result i32)
    (local $depth i32)
    (local.set $depth (i32.sub (i32.load (global.get $JS_LABEL_DEPTH_x86_64)) (i32.const 1)))
    (i32.store (global.get $JS_LABEL_DEPTH_x86_64) (local.get $depth))
    (local.get $depth)
  )

  (func $get_label_offset (param $depth i32) (result i32)
    (i32.load (i32.add (global.get $JS_LABEL_OFFSETS_x86_64) (i32.shl (local.get $depth) (i32.const 2))))
  )

  (func $set_label_offset (param $depth i32) (param $off i32)
    (i32.store (i32.add (global.get $JS_LABEL_OFFSETS_x86_64) (i32.shl (local.get $depth) (i32.const 2))) (local.get $off))
  )

  (func $get_label_kind (param $depth i32) (result i32)
    (i32.load8_u (i32.add (global.get $JS_LABEL_KINDS_x86_64) (local.get $depth)))
  )

  (func $get_label_if_jz (param $depth i32) (result i32)
    (i32.load (i32.add (global.get $JS_LABEL_IF_JZ_x86_64) (i32.shl (local.get $depth) (i32.const 2))))
  )

  (func $set_label_if_jz (param $depth i32) (param $off i32)
    (i32.store (i32.add (global.get $JS_LABEL_IF_JZ_x86_64) (i32.shl (local.get $depth) (i32.const 2))) (local.get $off))
  )

  ;; ── Fixup helpers ─────────────────────────────────────────────────

  (func $push_fixup (param $label_depth i32)
    (local $count i32) (local $off i32)
    (local.set $count (i32.load (global.get $JS_FIXUP_COUNT_x86_64)))
    (local.set $off (i32.load (global.get $JS_CODE_PTR_x86_64)))
    (i32.store (i32.add (global.get $JS_FIXUP_LABEL_x86_64) (i32.shl (local.get $count) (i32.const 2))) (local.get $label_depth))
    (i32.store (i32.add (global.get $JS_FIXUP_OFFSET_x86_64) (i32.shl (local.get $count) (i32.const 2))) (local.get $off))
    (i32.store (global.get $JS_FIXUP_COUNT_x86_64) (i32.add (local.get $count) (i32.const 1)))
  )

  ;; Patch fixups at given label depth: compute forward jump displacement
  ;; and write it at each fixup offset
  (func $patch_fixups (param $depth i32)
    (local $count i32) (local $i i32) (local $cur i32) (local $fix_off i32)
    (local.set $count (i32.load (global.get $JS_FIXUP_COUNT_x86_64)))
    (local.set $cur (i32.load (global.get $JS_CODE_PTR_x86_64)))
    (local.set $i (i32.const 0))
    (block $pfx_end
      (loop $pfx_loop
        (if (i32.ge_u (local.get $i) (local.get $count)) (then (br $pfx_end)))
        (if (i32.eq (i32.load (i32.add (global.get $JS_FIXUP_LABEL_x86_64) (i32.shl (local.get $i) (i32.const 2)))) (local.get $depth))
          (then
            (local.set $fix_off (i32.load (i32.add (global.get $JS_FIXUP_OFFSET_x86_64) (i32.shl (local.get $i) (i32.const 2)))))
            ;; displacement = cur - (fix_off + 4)
            ;; fix_off is an offset from JIT_CACHE, so write to JIT_CACHE + fix_off
            (i32.store
              (i32.add (global.get $JIT_CACHE) (local.get $fix_off))
              (i32.sub (local.get $cur) (i32.add (local.get $fix_off) (i32.const 4)))
            )
            ;; Remove fixup by swapping with last
            (local.set $count (i32.sub (local.get $count) (i32.const 1)))
            (i32.store (i32.add (global.get $JS_FIXUP_LABEL_x86_64) (i32.shl (local.get $i) (i32.const 2)))
              (i32.load (i32.add (global.get $JS_FIXUP_LABEL_x86_64) (i32.shl (local.get $count) (i32.const 2)))))
            (i32.store (i32.add (global.get $JS_FIXUP_OFFSET_x86_64) (i32.shl (local.get $i) (i32.const 2)))
              (i32.load (i32.add (global.get $JS_FIXUP_OFFSET_x86_64) (i32.shl (local.get $count) (i32.const 2)))))
            (i32.store (global.get $JS_FIXUP_COUNT_x86_64) (local.get $count))
          )
        )
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $pfx_loop)
      )
    )
  )

  ;; ═════════════════════════════════════════════════════════════════════
  ;; SSE 128-bit (SIMD) helpers
  ;; ═════════════════════════════════════════════════════════════════════

  ;; mov rax, imm64 (10 bytes) — with parameter

  ;; ── Unsupported opcode handler ─────────────────────────────────────
  (func $template_x86_unsupported (param $dec_ptr i32)
    (global.set $JIT_ERROR_x86_64 (i32.const -4))
  )

  ;; ── Control flow templates ─────────────────────────────────────────
  (func $template_block_x86_64 (param $dec_ptr i32)
    (call $push_label (global.get $JIT_LABEL_BLOCK_x86_64))
  )
  (func $template_loop_x86_64 (param $dec_ptr i32)
    (local $depth i32)
    (call $push_label (global.get $JIT_LABEL_LOOP_x86_64))
    (local.set $depth (i32.load (global.get $JS_LABEL_DEPTH_x86_64)))
    (call $set_label_offset (local.get $depth) (i32.load (global.get $JS_CODE_PTR_x86_64)))
  )
  (func $template_if_x86_64 (param $dec_ptr i32)
    (local $depth i32)
    (call $emit_x86_pop_rax)
    (call $emit_x86_test_eax)
    (call $push_label (global.get $JIT_LABEL_IF_x86_64))
    (call $emit_x86_byte (i32.const 0x0F))
    (call $emit_x86_byte (i32.const 0x84))
    (local.set $depth (i32.load (global.get $JS_LABEL_DEPTH_x86_64)))
    (i32.store
      (i32.add (global.get $JS_LABEL_IF_JZ_x86_64) (i32.shl (i32.sub (local.get $depth) (i32.const 1)) (i32.const 2)))
      (i32.load (global.get $JS_CODE_PTR_x86_64)))
    (call $emit_x86_dword (i32.const 0))
  )
  (func $template_else_x86_64 (param $dec_ptr i32)
    (local $depth i32) (local $idx i32) (local $fix_off i32) (local $cur i32) (local $disp i32)
    (local.set $depth (i32.load (global.get $JS_LABEL_DEPTH_x86_64)))
    (local.set $idx (i32.sub (local.get $depth) (i32.const 1)))
    ;; Patch the IF's jz to jump to current position (start of else)
    (local.set $fix_off
      (i32.load (i32.add (global.get $JS_LABEL_IF_JZ_x86_64) (i32.shl (local.get $idx) (i32.const 2)))))
    (if (i32.gt_u (local.get $fix_off) (i32.const 0))
      (then
        (local.set $cur (i32.load (global.get $JS_CODE_PTR_x86_64)))
        (local.set $disp (i32.sub (local.get $cur) (i32.add (local.get $fix_off) (i32.const 4))))
        (i32.store (i32.add (global.get $JIT_CACHE) (local.get $fix_off)) (local.get $disp))
      )
    )
    ;; Emit jmp placeholder for jump to end of if/else
    (call $emit_x86_byte (i32.const 0xE9))
    (i32.store
      (i32.add (global.get $JS_LABEL_IF_JZ_x86_64) (i32.shl (local.get $idx) (i32.const 2)))
      (i32.load (global.get $JS_CODE_PTR_x86_64)))
    (call $emit_x86_dword (i32.const 0))
  )
  (func $template_end_x86_64 (param $dec_ptr i32)
    (local $depth i32) (local $idx i32) (local $kind i32)
    (local $fix_off i32) (local $cur i32) (local $disp i32)
    (local.set $depth (i32.load (global.get $JS_LABEL_DEPTH_x86_64)))
    (if (i32.gt_u (local.get $depth) (i32.const 0))
      (then
        (local.set $idx (i32.sub (local.get $depth) (i32.const 1)))
        (local.set $kind (call $get_label_kind (local.get $idx)))
        ;; Patch pending fixup (if's jz or else's jmp) stored in JS_LABEL_IF_JZ
        (local.set $fix_off
          (i32.load (i32.add (global.get $JS_LABEL_IF_JZ_x86_64) (i32.shl (local.get $idx) (i32.const 2)))))
        (if (i32.gt_u (local.get $fix_off) (i32.const 0))
          (then
            (local.set $cur (i32.load (global.get $JS_CODE_PTR_x86_64)))
            (local.set $disp (i32.sub (local.get $cur) (i32.add (local.get $fix_off) (i32.const 4))))
            (i32.store (i32.add (global.get $JIT_CACHE) (local.get $fix_off)) (local.get $disp))
          )
        )
        ;; Patch BR fixups at this depth (block/if only, not loop)
        (if (i32.ne (local.get $kind) (global.get $JIT_LABEL_LOOP_x86_64))
          (then (call $patch_fixups (local.get $depth)))
        )
        (drop (call $pop_label))
      )
    )
  )
  (func $template_br_x86_64 (param $dec_ptr i32)
    (local $target_rel i32) (local $target_abs i32) (local $cur_depth i32) (local $target_off i32)
    (local.set $target_rel (i32.load (i32.add (local.get $dec_ptr) (i32.const 4))))
    (local.set $cur_depth (i32.load (global.get $JS_LABEL_DEPTH_x86_64)))
    (local.set $target_abs (i32.sub (local.get $cur_depth) (local.get $target_rel)))
    (local.set $target_off (call $get_label_offset (local.get $target_abs)))
    (if (i32.eqz (local.get $target_off))
      (then
        ;; Forward jump — use fixup
        (call $emit_x86_byte (i32.const 0xE9))
        (call $push_fixup (local.get $target_abs))
        (call $emit_x86_dword (i32.const 0))
      )
      (else
        ;; Backward jump — compute displacement directly
        (call $emit_x86_jmp_rel32
          (i32.sub (local.get $target_off)
            (i32.add (i32.load (global.get $JS_CODE_PTR_x86_64)) (i32.const 5))))
      )
    )
  )
  (func $template_br_if_x86_64 (param $dec_ptr i32)
    (local $target_rel i32) (local $target_abs i32) (local $cur_depth i32) (local $target_off i32)
    (local.set $target_rel (i32.load (i32.add (local.get $dec_ptr) (i32.const 4))))
    (local.set $cur_depth (i32.load (global.get $JS_LABEL_DEPTH_x86_64)))
    (local.set $target_abs (i32.sub (local.get $cur_depth) (local.get $target_rel)))
    (local.set $target_off (call $get_label_offset (local.get $target_abs)))
    (call $emit_x86_pop_rax)
    (call $emit_x86_test_eax)
    (if (i32.eqz (local.get $target_off))
      (then
        (call $emit_x86_byte (i32.const 0x0F))
        (call $emit_x86_byte (i32.const 0x85))
        (call $push_fixup (local.get $target_abs))
        (call $emit_x86_dword (i32.const 0))
      )
      (else
        (call $emit_x86_jne_rel32
          (i32.sub (local.get $target_off)
            (i32.add (i32.load (global.get $JS_CODE_PTR_x86_64)) (i32.const 6))))
      )
    )
  )
  (func $template_br_table_x86_64 (param $dec_ptr i32)
    (call $emit_x86_pop_rax)
    (call $emit_x86_jmp_rel32 (i32.const 0))
  )

  ;; ── Return templates ──────────────────────────────────────────────
  (func $template_return_x86_64 (param $dec_ptr i32)
    (call $emit_x86_epilogue)
  )
  (func $template_return_call_x86_64 (param $dec_ptr i32)
    (global.set $JIT_ERROR_x86_64 (i32.const -4))
  )

  ;; ── Reinterpret templates (bitcasts — no code needed) ────────────
  (func $template_i32_reinterpret_f32_x86_64 (param $dec_ptr i32))
  (func $template_f32_reinterpret_i32_x86_64 (param $dec_ptr i32))
  (func $template_i64_reinterpret_f64_x86_64 (param $dec_ptr i32))
  (func $template_f64_reinterpret_i64_x86_64 (param $dec_ptr i32))

