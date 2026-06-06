  ;; Opcode templates — each emits AArch64 code for one WASM opcode
  ;; ═════════════════════════════════════════════════════════════════════

  ;; ── unreachable (0x00): BRK #0 (debug breakpoint) ────────────────

  ;; ── nop (0x01): nothing ────────────────────────────────────────────

  ;; ── select (0x1B): pop cond, pop val2, pop val1, select ──────────

  ;; ── i32.const (0x41): push imm32 ──────────────────────────────────
    )
    (call $emit_maybe_push_x0)
  )

  ;; ── i64.const (0x42): push imm64 ──────────────────────────────────
    (if (i32.ne (local.get $val2) (i32.const 0))
      (then (call $emit_instr_movk_64 (global.get $REG_X0) (i32.const 2) (local.get $val2)))
    )
    (if (i32.ne (local.get $val3) (i32.const 0))
      (then (call $emit_instr_movk_64 (global.get $REG_X0) (i32.const 3) (local.get $val3)))
    )
    (call $emit_maybe_push_x0)
  )

  ;; ── local.get (0x20): load local at index ─────────────────────────
      (else
        (if (i32.eq (local.get $idx) (i32.const 1))
          (then
            (call $emit_instr (i32.const 0xAA0003F6))  ;; MOV X0, X22
            (call $emit_push_x0)
          )
          (else
            ;; Load from memory via X20 (locals base)
            (call $emit_ldr_x0_x20 (local.get $idx))  ;; actual offset = idx * 8 / 8 = idx
            (call $emit_maybe_push_x0)
          )
        )
      )
    )
  )

  ;; ── local.set (0x21): pop/store to local ──────────────────────────
          (else
            (if (i32.eq (local.get $idx) (i32.const 1))
              (then (call $emit_instr (i32.const 0xAA0003C0)))  ;; MOV X22, X0
              (else (call $emit_str_x0_x20 (i32.mul (local.get $idx) (i32.const 1))))  ;; offset = idx*8/8 = idx
            )
          )
        )
      )
      (else
        (if (i32.eqz (local.get $idx))
          (then
            (call $emit_pop_x0)
            (call $emit_instr (i32.const 0xAA0003E0))  ;; MOV X21, X0
          )
          (else
            (if (i32.eq (local.get $idx) (i32.const 1))
              (then
                (call $emit_pop_x0)
                (call $emit_instr (i32.const 0xAA0003C0))  ;; MOV X22, X0
              )
              (else
                (call $emit_pop_x0)
                (call $emit_str_x0_x20 (i32.mul (local.get $idx) (i32.const 1)))
              )
            )
          )
        )
      )
    )
  )

  ;; ── local.tee (0x22): like local.set but keep value on stack ──────
          )
        )
      )
      (else
        ;; Load TOS, duplicate, store
        ;; LDR X0, [SP] — unsigned offset 0
        (call $emit_instr_ldr_64_off (global.get $REG_X0) (global.get $REG_SP) (i32.const 0))
        (call $emit_push_x0)
        (if (i32.eqz (local.get $idx))
          (then
            (call $emit_pop_x0)
            (call $emit_instr (i32.const 0xAA0003E0))  ;; MOV X21, X0
          )
          (else
            (if (i32.eq (local.get $idx) (i32.const 1))
              (then
                (call $emit_pop_x0)
                (call $emit_instr (i32.const 0xAA0003C0))  ;; MOV X22, X0
              )
              (else
                (call $emit_pop_x0)
                (call $emit_str_x0_x20 (i32.mul (local.get $idx) (i32.const 1)))
              )
            )
          )
        )
      )
    )
  )

  ;; ── global.get (0x23) ─────────────────────────────────────────────

  ;; ── global.set (0x24) ─────────────────────────────────────────────

  ;; ── table.get (0x25) ─────────────────────────────────────────────

  ;; ── table.set (0x26) ─────────────────────────────────────────────

  ;; ── memory.size (0x3F) ──────────────────────────────────────────

  ;; ── memory.grow (0x40) ──────────────────────────────────────────

  ;; ── Unary arithmetic (32-bit) ───────────────────────────────────
  ;; i32.eqz (0x45) already done above

  ;; ── i32.reinterpret_f32 (0xBC) ──────────────────────────────────

  ;; ── f32.reinterpret_i32 (0xBD) ──────────────────────────────────

  ;; ── i64.reinterpret_f64 (0xBE) ──────────────────────────────────

  ;; ── f64.reinterpret_i64 (0xBF) ──────────────────────────────────

  ;; ── i32.load8_s (0x2C) ──────────────────────────────────────────

  ;; ── i32.load8_u (0x2D) ──────────────────────────────────────────

  ;; ── i32.load16_s (0x2E) ─────────────────────────────────────────

  ;; ── i32.load16_u (0x2F) ─────────────────────────────────────────

  ;; ── Type conversion templates ───────────────────────────────────

  ;; i32.wrap_i64 (0xA7) already done below

  ;; i32.trunc_f32_s (0xA8) ─ placeholder

  ;; i32.trunc_f32_u (0xA9) ─ placeholder

  ;; i32.trunc_f64_s (0xAA) ─ placeholder

  ;; i32.trunc_f64_u (0xAB) ─ placeholder

  ;; ── i64.trunc_f32_s (0xAE) ─ placeholder

  ;; i64.trunc_f32_u (0xAF) ─ placeholder

  ;; i64.trunc_f64_s (0xB0) ─ placeholder

  ;; i64.trunc_f64_u (0xB1) ─ placeholder

  ;; ── i64.trunc_sat_f32_s (0xFC prefix, sub=0x04) ───────────────

  ;; ── Saturating truncation helpers (0xFC-prefixed) ───────────────
  ;; i32.trunc_sat_f32_s (0xFC, sub=0x00)

  ;; i32.trunc_sat_f32_u (0xFC, sub=0x01)

  ;; i32.trunc_sat_f64_s (0xFC, sub=0x02)

  ;; i32.trunc_sat_f64_u (0xFC, sub=0x03)

  ;; ── f32.demote_f64 (0xB6) ─ placeholder

  ;; ── f64.promote_f32 (0xBB) ─ placeholder

  ;; ── Control flow templates ────────────────────────────────────
  (func $template_block (param $dec_ptr i32))
  (func $template_loop (param $dec_ptr i32))
  (func $template_if (param $dec_ptr i32)
    (call $emit_pop_x0)
    (call $emit_cbz_x (global.get $REG_X0) (i32.const 0))  ;; placeholder offset
  )
  (func $template_else (param $dec_ptr i32)
    (call $emit_b (i32.const 0))  ;; placeholder branch
  )
  (func $template_end (param $dec_ptr i32))
  (func $template_br (param $dec_ptr i32)
    (call $emit_b (i32.const 0))  ;; placeholder
  )
  (func $template_br_if (param $dec_ptr i32)
    (call $emit_pop_x0)
    (call $emit_cbnz_x (global.get $REG_X0) (i32.const 0))  ;; placeholder
  )
  (func $template_br_table (param $dec_ptr i32)
    (call $emit_pop_x0)
    (call $emit_b (i32.const 0))  ;; placeholder (jump to default)
  )
  (func $template_return (param $dec_ptr i32)
    (call $emit_epilogue)
  )
  (func $template_return_call (param $dec_ptr i32)
    (call $emit_epilogue)
  )

  ;; ── Memory load/store templates ───────────────────────────────

  ;; ── i32 comparison templates ──────────────────────────────────

  ;; ── i64 comparison templates ──────────────────────────────────

  ;; ── i32 unary/binary arithmetic templates ─────────────────────

  ;; ── i64 unary/binary arithmetic templates ─────────────────────

  ;; ── Conversion templates ──────────────────────────────────────

  ;; ── Block/loop/if/else structure tracking ──────────────────────
  ;; jit_compile will handle the fixup/backpatching.
  ;; We just emit placeholder branches and the main loop patches them.

  ;; ════════════════════════════════════════════════════════════════════
  ;; ── The main compile function ─────────────────────────────────────
  ;; ════════════════════════════════════════════════════════════════════

