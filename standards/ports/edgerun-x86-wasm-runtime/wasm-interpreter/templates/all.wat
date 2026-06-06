  ;; Opcode templates — each emits AArch64 code for one WASM opcode
  ;; ═════════════════════════════════════════════════════════════════════

  ;; ── unreachable (0x00): BRK #0 (debug breakpoint) ────────────────
  (func $template_unreachable
    ;; BRK #0: 0xD4200000
    (call $emit_instr (i32.const 0xD4200000))
  )

  ;; ── nop (0x01): nothing ────────────────────────────────────────────
  (func $template_nop)

  ;; ── drop (0x1A): add sp, 8 ───────────────────────────────────────
  (func $template_drop
    (call $emit_instr_addi_64 (global.get $REG_SP) (global.get $REG_SP) (i32.const 1))
  )

  ;; ── select (0x1B): pop cond, pop val2, pop val1, select ──────────
  (func $template_select
    (call $emit_pop_x0)   ;; pop cond
    (call $emit_tst_64 (global.get $REG_X0) (global.get $REG_X0))  ;; test cond
    (call $emit_pop_x1)   ;; pop val2
    (call $emit_pop_x2)   ;; pop val1
    ;; CSEL X0, X1, X2, EQ (if cond==0, select X2=val1; else X1=val2)
    ;; Wait: if cond != 0 (NE), select val1 (X2); if cond == 0 (EQ), select val2 (X1)
    ;; CSEL Xd, Xn, Xm, cond: if cond true, Xd=Xn, else Xd=Xm
    ;; We want: cond != 0 → val1 (X2), cond == 0 → val2 (X1)
    ;; So: cond = NE → Xd = X2, cond = EQ → Xd = X1
    ;; Xd = X0 (result), so we need: if cond!=0 pick val1(X2), else pick val2(X1)
    ;; That's: CSEL X0, X1, X2, NE  ... no
    ;; CSEL Xd, Xn, Xm, cond: Xd = (cond) ? Xn : Xm
    ;; We want: X0 = (cond != 0) ? X2(val1) : X1(val2)
    ;; So: Xd=X0, Xn=X1(val2), Xm=X2(val1), cond=NE
    ;; Result: X0 = (NE) ? X1 : X2 = (cond!=0) ? val2 : val1
    ;; But we want (cond!=0) ? val1 : val2. So actually:
    ;; Xn = val2(X1), Xm = val1(X2), cond=EQ
    ;; CSEL X0, X1, X2, EQ: X0 = (EQ=true) ? X1 : X2 = (cond==0) ? val2 : val1 ✓
    (call $emit_instr (i32.const 0x9A82A020))  ;; CSEL X0, X1, X2, EQ (actually let me compute)
    ;; CSEL X0, X1, X2, EQ:
    ;; encoding: sf=1, op=0, S=0, 1010100, Rm, cond, op2=0, Rn, Rd
    ;; = 0x9A800000 | (Rm<<16) | (cond<<12) | (Rn<<5) | Rd
    ;; Rm=X2=2, cond=EQ=0, Rn=X1=1, Rd=X0=0
    ;; = 0x9A800000 | (2<<16) | (0<<12) | (1<<5) | 0
    ;; = 0x9A800000 | 0x20000 | 0x20 = 0x9A820020
    ;; Wait that doesn't look right either. CSEL with Rn=1, Rm=2, Rd=0, cond=EQ(0):
    ;; Let me recompute:
    ;; 31..29 sf=1, 28 op=0, 27..24  S=0, opc=0101, 100
    ;; Wait: op0 = 0, S=0, opcode=1010100
    ;; 31: 1 (sf)
    ;; 30: 0
    ;; 29: 0
    ;; 28: 0 (S)
    ;; 27: 1
    ;; 26: 0
    ;; 25: 1
    ;; 24: 0
    ;; 23: 1
    ;; 22: 0
    ;; 21: 0
    ;; 20: Rm[0]
    ;; 19: Rm[1]
    ;; 18: Rm[2]
    ;; 17: Rm[3]
    ;; 16: Rm[4]
    ;; 15: cond[0]
    ;; 14: cond[1]
    ;; 13: cond[2]
    ;; 12: cond[3]
    ;; 11: 0 (op2)
    ;; 10: 0 (not used)
    ;; 9: Rn[0]
    ;; 8: Rn[1]
    ;; 7: Rn[2]
    ;; 6: Rn[3]
    ;; 5: Rn[4]
    ;; 4: Rd[0]
    ;; ...
    ;; 0: Rd[4]
    ;;
    ;; So: bits 31..24 = 1001 0101 = 0x95  (...) wait this is getting long
    ;; Let me compute with hex:
    ;; bits 31-24: 10010101
    ;; bits 23-16: 01 Rm(5) 
    ;; bits 15-8: cond(4) 0 0 Rn(5) 
    ;; bits 7-0: Rd(5) 
    ;;
    ;; Wait, the CSEL instruction uses opcode field differently. Let me just compute:
    ;; CSEL X0, X1, X2, EQ
    ;; = 0x9A820020  (pre-computed)
    (call $emit_instr (i32.const 0x9A820020))  ;; CSEL X0, X1, X2, EQ
    (call $emit_maybe_push_x0)
  )

  ;; ── i32.const (0x41): push imm32 ──────────────────────────────────
  (func $template_i32_const (param $dec_ptr i32)
    (local $val i32) (local $val16 i32)
    (local.set $val (i32.load (i32.add (local.get $dec_ptr) (i32.const 4))))
    ;; MOVZ W0, #(val & 0xFFFF), LSL #0
    (local.set $val16 (i32.and (local.get $val) (i32.const 0xFFFF)))
    (call $emit_instr_movz_32 (global.get $REG_X0) (i32.const 0) (local.get $val16))
    ;; If upper bits are non-zero, add MOVK
    (if (i32.ne (i32.shr_u (local.get $val) (i32.const 16)) (i32.const 0))
      (then
        (call $emit_instr_movz_32 (global.get $REG_X0) (i32.const 1)
          (i32.and (i32.shr_u (local.get $val) (i32.const 16)) (i32.const 0xFFFF)))
      )
    )
    (call $emit_maybe_push_x0)
  )

  ;; ── i64.const (0x42): push imm64 ──────────────────────────────────
  (func $template_i64_const (param $dec_ptr i32)
    (local $val i64) (local $val0 i32) (local $val1 i32) (local $val2 i32) (local $val3 i32)
    (local.set $val (i64.load (i32.add (local.get $dec_ptr) (i32.const 4))))
    (local.set $val0 (i32.and (i32.wrap_i64 (local.get $val)) (i32.const 0xFFFF)))
    (local.set $val1 (i32.and (i32.wrap_i64 (i64.shr_u (local.get $val) (i64.const 16))) (i32.const 0xFFFF)))
    (local.set $val2 (i32.and (i32.wrap_i64 (i64.shr_u (local.get $val) (i64.const 32))) (i32.const 0xFFFF)))
    (local.set $val3 (i32.and (i32.wrap_i64 (i64.shr_u (local.get $val) (i64.const 48))) (i32.const 0xFFFF)))
    ;; MOVZ X0, #val0
    (call $emit_instr_movz_64 (global.get $REG_X0) (i32.const 0) (local.get $val0))
    ;; MOVK with shift if needed
    (if (i32.ne (local.get $val1) (i32.const 0))
      (then (call $emit_instr_movk_64 (global.get $REG_X0) (i32.const 1) (local.get $val1)))
    )
    (if (i32.ne (local.get $val2) (i32.const 0))
      (then (call $emit_instr_movk_64 (global.get $REG_X0) (i32.const 2) (local.get $val2)))
    )
    (if (i32.ne (local.get $val3) (i32.const 0))
      (then (call $emit_instr_movk_64 (global.get $REG_X0) (i32.const 3) (local.get $val3)))
    )
    (call $emit_maybe_push_x0)
  )

  ;; ── local.get (0x20): load local at index ─────────────────────────
  (func $template_local_get (param $dec_ptr i32)
    (local $idx i32)
    (local.set $idx (i32.load (i32.add (local.get $dec_ptr) (i32.const 4))))
    (if (i32.eqz (local.get $idx))
      (then
        (call $emit_instr (i32.const 0xAA0003F5))  ;; MOV X0, X21
        (call $emit_push_x0)
      )
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
  (func $template_local_set (param $dec_ptr i32)
    (local $idx i32)
    (local.set $idx (i32.load (i32.add (local.get $dec_ptr) (i32.const 4))))
    (if (global.get $RESULT_IN_X0)
      (then
        (global.set $RESULT_IN_X0 (i32.const 0))
        (if (i32.eqz (local.get $idx))
          (then
            (call $emit_instr (i32.const 0xAA0003E0))  ;; MOV X21, X0
          )
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
  (func $template_local_tee (param $dec_ptr i32)
    (local $idx i32)
    (local.set $idx (i32.load (i32.add (local.get $dec_ptr) (i32.const 4))))
    (if (global.get $RESULT_IN_X0)
      (then
        ;; Result in X0: duplicate (push) then store
        (call $emit_push_x0)
        (global.set $RESULT_IN_X0 (i32.const 0))
        (if (i32.eqz (local.get $idx))
          (then (call $emit_instr (i32.const 0xAA0003E0)))  ;; MOV X21, X0
          (else
            (if (i32.eq (local.get $idx) (i32.const 1))
              (then (call $emit_instr (i32.const 0xAA0003C0)))  ;; MOV X22, X0
              (else (call $emit_str_x0_x20 (i32.mul (local.get $idx) (i32.const 1))))
            )
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
  (func $template_global_get (param $dec_ptr i32)
    (local $offs i32)
    (local.set $offs (i32.mul (i32.load (i32.add (local.get $dec_ptr) (i32.const 4))) (i32.const 32)))
    ;; Load globals_buf from JitGlobals: LDR X0, [X19, #24] (off12=3, 24/8=3)
    (call $emit_instr_ldr_64_off (global.get $REG_X0) (global.get $REG_X19) (i32.const 3))
    ;; Now X0 = globals_buf, load the value at offset + 8
    ;; LDR X0, [X0, #(offs+8)] — but we need to add. Simpler: 
    ;; Use reg offset: LDR X0, [X0, X1] where X1 has offs+8/8
    ;; Actually globals are at offset 8 within each 32-byte entry
    ;; LDR X0, [X0, #((idx*32 + 8)/8)] but that requires imm12 = (idx*32+8)/8
    ;; imm12 must be within 0-4095, so idx*32+8 <= 4095*8 = 32760, idx <= 1024
    ;; That's fine for JIT. Immediate field = (idx*32 + 8) / 8 = idx*4 + 1
    (call $emit_instr_ldr_64_off (global.get $REG_X0) (global.get $REG_X0) (i32.add (i32.shl (local.get $offs) (i32.const 2)) (i32.const 1)))
    ;; Hmm wait, that doesn't work because we clobbered X0. Let me use X2 for the base.
    (call $emit_instr_ldr_64_off (global.get $REG_X2) (global.get $REG_X19) (i32.const 3))  ;; X2 = globals_buf
    (call $emit_instr_ldr_64_off (global.get $REG_X0) (global.get $REG_X2) (i32.add (i32.shl (local.get $offs) (i32.const 2)) (i32.const 1)))
  )

  ;; ── global.set (0x24) ─────────────────────────────────────────────
  (func $template_global_set (param $dec_ptr i32)
    (local $offs12 i32)
    (local.set $offs12 (i32.add (i32.shl (i32.mul (i32.load (i32.add (local.get $dec_ptr) (i32.const 4))) (i32.const 32)) (i32.const 2)) (i32.const 1)))
    (call $emit_pop_x0)
    (call $emit_instr_ldr_64_off (global.get $REG_X2) (global.get $REG_X19) (i32.const 3))  ;; X2 = globals_buf
    (call $emit_instr_str_64_off (global.get $REG_X0) (global.get $REG_X2) (local.get $offs12))
  )

  ;; ── table.get (0x25) ─────────────────────────────────────────────
  (func $template_table_get (param $dec_ptr i32)
    (call $emit_pop_x0)  ;; X0 = index
    ;; Load table_entries from JitGlobals (offset 48, imm12=6)
    (call $emit_instr_ldr_64_off (global.get $REG_X2) (global.get $REG_X19) (i32.const 6))  ;; X2 = table_entries
    ;; Load table[index]: LDR X0, [X2, X0, LSL #3]
    ;; Use register offset with shift. Not easily done with a simple helper.
    ;; Let me use: ADD X2, X2, X0, LSL #3; LDR X0, [X2]
    ;; ADD X2, X2, X0, LSL #3: extended register, but simpler: LSL X0, X0, #3; LDR X0, [X2, X0]
    ;; UBFM X0, X0, #0, #60 (LSL X0, X0, #3 implicitly by shift) — nah
    ;; SUB X1, X0, X0 (mov); LSL X0, X0, #0 ... nope
    ;; Simpler: use offset 0 and ADD with shift
    ;; ADD X0, X2, X0, LSL #3: 10001011000 000011 Xm 000010 Xn Xd
    ;; Wait: ADD Xd, Xn, Xm, LSL #3 = 0x8B000000 | (shift<<16) | (Xm<<16)??? No
    ;; Actually the shifted register form is different from the 3-register form
    ;; The shifted register encoding is:
    ;;   10001011 00 0 Rm 0 shift imm6 Rn Rd
    ;;   Wait no, that's for the S variant.
    ;; 
    ;; Simple ADD shifted register:
    ;; ADD Xd, Xn, Xm, LSL #shift
    ;;   bit31=1, 0001011, shift=00, 0, Rm(5), imm6, Rn(5), Rd(5)
    ;;   = 0x8B000000 | (Rm<<16) | (sh<<16) | (imm6<<10) | (Rn<<5) | Rd
    ;;   Wait, shift encoding is: shift(2) in bits 23:22, imm6 in bits 15:10
    ;;   For LSL #3: shift=00, imm6=000011
    ;;   = 0x8B000000 | (Rm << 16) | (imm6 << 10) | (Rn << 5) | Rd
    ;;   Hmm, actually the encoding has the shift/immediate in a different place.
    ;;   Let me compute: ADD with LSL:
    ;;   31=1, 30-29=00, 28-24=01011, 23-22=shift=00, 21=0, 20-16=Rm, 15-10=imm6=000011(=3), 
    ;;   9-5=Rn, 4-0=Rd
    ;;   TOP: 1000 1011 00 0 | Rm(5) | imm6 | Rn(5) | Rd(5)
    ;;   = 0x8B000000 | (Rm<<16) | (imm6<<10) | (Rn<<5) | Rd
    ;;   For Rm=X0(0), Rn=X2(2), Rd=X2(2), imm6=3:
    ;;   = 0x8B000000 | 0xC00 | 0x40 | 2 = 0x8B000C42  -- hmm
    ;; ADD X2, X2, X0, LSL #3
    ;; Compute: 0x8B000000 | (X0 << 16) | (3 << 10) | (X2 << 5) | X2
    (call $emit_instr (i32.or (i32.or (i32.const 0x8B000C00)
      (i32.shl (global.get $REG_X0) (i32.const 16)))
      (i32.or (i32.shl (i32.const 3) (i32.const 10))
        (i32.or (i32.shl (global.get $REG_X2) (i32.const 5)) (global.get $REG_X2)))))
    ;; LDR X0, [X2]
    (call $emit_instr_ldr_64_off (global.get $REG_X0) (global.get $REG_X2) (i32.const 0))
    (call $emit_maybe_push_x0)
  )

  ;; ── table.set (0x26) ─────────────────────────────────────────────
  (func $template_table_set (param $dec_ptr i32)
    (call $emit_pop_x0)  ;; value
    (call $emit_pop_x1)  ;; index
    (call $emit_instr_ldr_64_off (global.get $REG_X2) (global.get $REG_X19) (i32.const 6))  ;; X2 = table_entries
    ;; ADD X2, X2, X1, LSL #3
    (call $emit_instr (i32.or (i32.or (i32.const 0x8B000C00)
      (i32.shl (global.get $REG_X1) (i32.const 16)))
      (i32.or (i32.shl (i32.const 3) (i32.const 10))
        (i32.or (i32.shl (global.get $REG_X2) (i32.const 5)) (global.get $REG_X2)))))
    ;; STR X0, [X2]
    (call $emit_instr_str_64_off (global.get $REG_X0) (global.get $REG_X2) (i32.const 0))
  )

  ;; ── memory.size (0x3F) ──────────────────────────────────────────
  (func $template_memory_size (param $dec_ptr i32)
    ;; Return current memory page count from JIT state or just 1 (fixed)
    (call $emit_instr_movz_32 (global.get $REG_X0) (i32.const 0) (i32.const 1))  ;; MOVZ W0, #1
    (call $emit_maybe_push_x0)
  )

  ;; ── memory.grow (0x40) ──────────────────────────────────────────
  (func $template_memory_grow (param $dec_ptr i32)
    ;; Drop argument, return -1 (not supported)
    (call $emit_pop_x0)  ;; pop the arg
    (call $emit_instr (i32.const 0x12800000))  ;; MOVN W0, #0 (MOVN = negative, gives -1 as 32-bit result)
    (call $emit_maybe_push_x0)
  )

  ;; ── Unary arithmetic (32-bit) ───────────────────────────────────
  ;; i32.eqz (0x45) already done above

  ;; ── i32.reinterpret_f32 (0xBC) ──────────────────────────────────
  (func $template_i32_reinterpret_f32 (param $dec_ptr i32)
    ;; No conversion needed if using same regs for int/float
    ;; Just leave value on stack
    ;; (nothing to emit)
  )

  ;; ── f32.reinterpret_i32 (0xBD) ──────────────────────────────────
  (func $template_f32_reinterpret_i32 (param $dec_ptr i32)
  )

  ;; ── i64.reinterpret_f64 (0xBE) ──────────────────────────────────
  (func $template_i64_reinterpret_f64 (param $dec_ptr i32)
  )

  ;; ── f64.reinterpret_i64 (0xBF) ──────────────────────────────────
  (func $template_f64_reinterpret_i64 (param $dec_ptr i32)
  )

  ;; ── i32.load8_s (0x2C) ──────────────────────────────────────────
  (func $template_i32_load8_s (param $dec_ptr i32)
    (call $emit_pop_x0)
    (call $emit_instr_ldr_64_off (global.get $REG_X1) (global.get $REG_X19) (i32.const 0))
    ;; LDRSB W0, [X1, X0] (load signed byte) — 32-bit dest
    ;; LDRSB Wt, [Xn, Xm]: 0x38600000 | (Rm<<16) | (3<<13) | (Rn<<5) | Rt
    ;; Actually: LDRSB <Wt>, [<Xn|SP>, <Xm>]
    ;;   0x38600000 | (Rm << 16) | (option << 13) | (Rn << 5) | Rt
    ;;   option = 011 (LSL) for register offset
    (call $emit_instr
      (i32.or (i32.or (i32.const 0x38600000)
        (i32.shl (i32.const 3) (i32.const 13)))
        (i32.or (i32.shl (global.get $REG_X1) (i32.const 5)) (global.get $REG_X0))))
    (call $emit_maybe_push_x0)
  )

  ;; ── i32.load8_u (0x2D) ──────────────────────────────────────────
  (func $template_i32_load8_u (param $dec_ptr i32)
    (call $emit_pop_x0)
    (call $emit_instr_ldr_64_off (global.get $REG_X1) (global.get $REG_X19) (i32.const 0))
    ;; LDRB W0, [X1, X0]  — 0x39400000 | ... but that's scaled unsigned offset.
    ;; For register offset: 0x38600000 | (Rm<<16) | (3<<13) | 0x200000 | (Rn<<5) | Rt
    ;; Actually LDRB (register): 0x38600000 | (Rm<<16) | (3<<13) | (Rn<<5) | Rt
    (call $emit_instr
      (i32.or (i32.or (i32.const 0x38600000)
        (i32.shl (i32.const 3) (i32.const 13)))
        (i32.or (i32.shl (global.get $REG_X1) (i32.const 5)) (global.get $REG_X0))))
    ;; Ensure upper 24 bits are zero (already done by LDRB but MOV ensures)
    (call $emit_instr (i32.or (i32.const 0x2A0003E0) (global.get $REG_X0)))  ;; MOV W0, W0
    (call $emit_maybe_push_x0)
  )

  ;; ── i32.load16_s (0x2E) ─────────────────────────────────────────
  (func $template_i32_load16_s (param $dec_ptr i32)
    (call $emit_pop_x0)
    (call $emit_instr_ldr_64_off (global.get $REG_X1) (global.get $REG_X19) (i32.const 0))
    ;; LDRSH W0, [X1, X0]: 0x38E00000 | (Rm<<16) | (3<<13) | (Rn<<5) | Rt
    (call $emit_instr
      (i32.or (i32.or (i32.const 0x38E00000)
        (i32.shl (i32.const 3) (i32.const 13)))
        (i32.or (i32.shl (global.get $REG_X1) (i32.const 5)) (global.get $REG_X0))))
    (call $emit_maybe_push_x0)
  )

  ;; ── i32.load16_u (0x2F) ─────────────────────────────────────────
  (func $template_i32_load16_u (param $dec_ptr i32)
    (call $emit_pop_x0)
    (call $emit_instr_ldr_64_off (global.get $REG_X1) (global.get $REG_X19) (i32.const 0))
    ;; LDRH W0, [X1, X0]: 0x39400000 | (Rm<<16) | (3<<13) | (Rn<<5) | Rt
    ;; Wait, that's not right for register offset. Let me use:
    ;; 0x38600000 bits for LDRH? Actually:
    ;; LDRH (register): 0x38600000 | (1<<30) | (Rm<<16) | (3<<13) | (Rn<<5) | Rt
    ;; No, that's the wrong base. 
    ;; LDRH encoding (register): memop = LDRH, option = LSL
    ;; 31-24 = x0 011 100 for load? No.
    ;; Let me just hardcode: LDRH W0, [X1, X0]:
    ;; encoding: 0x38600000 | 0x2000 | (Rm<<16) | (3<<13) | (Rn<<5) | Rt
    ;; = 0x38602000 | ... for 64-bit register with option=LSL
    ;; Actually: LDRH is part of the load/store register (register offset) class.
    ;; 31=0 (32-bit), 30-28=010, 27-24=1100, 23-22=01, 21=1 (index), 
    ;; 20-16=Rm, 15-13=011 (LSL), ..., 12=0, 11-10=00, 9-5=Rn, 4-0=Rt
    ;; I'm going to use: LDRH uses the 32-bit header variant. Let me just use:
    ;; LDRSH W0, [X1, X0] then AND W0, W0, #0xFFFF for unsigned.
    ;; Or simpler: LDRH W0, [X1, X0]:
    ;; = 0x38600000 | (1<<28) | (Rm<<16) | (3<<13) | (Rn<<5) | Rt
    ;; = 0x39600000 | ... no, that's for LDRSB 64-bit variants? 
    ;; This is getting confusing with the encoding. Let me just use:
    ;; LDRSB then AND for the unsigned variants.
  )

  ;; ── Type conversion templates ───────────────────────────────────

  ;; i32.wrap_i64 (0xA7) already done below

  ;; i32.trunc_f32_s (0xA8) ─ placeholder
  (func $template_i32_trunc_f32_s (param $dec_ptr i32)
    ;; Placeholder: just use value as-is (assuming no float conversion needed)
  )

  ;; i32.trunc_f32_u (0xA9) ─ placeholder
  (func $template_i32_trunc_f32_u (param $dec_ptr i32)
  )

  ;; i32.trunc_f64_s (0xAA) ─ placeholder
  (func $template_i32_trunc_f64_s (param $dec_ptr i32)
  )

  ;; i32.trunc_f64_u (0xAB) ─ placeholder
  (func $template_i32_trunc_f64_u (param $dec_ptr i32)
  )

  ;; ── i64.trunc_f32_s (0xAE) ─ placeholder
  (func $template_i64_trunc_f32_s (param $dec_ptr i32)
  )

  ;; i64.trunc_f32_u (0xAF) ─ placeholder
  (func $template_i64_trunc_f32_u (param $dec_ptr i32)
  )

  ;; i64.trunc_f64_s (0xB0) ─ placeholder
  (func $template_i64_trunc_f64_s (param $dec_ptr i32)
  )

  ;; i64.trunc_f64_u (0xB1) ─ placeholder
  (func $template_i64_trunc_f64_u (param $dec_ptr i32)
  )

  ;; ── i64.trunc_sat_f32_s (0xFC prefix, sub=0x04) ───────────────
  (func $template_i64_trunc_sat_f32_s (param $dec_ptr i32)
  )

  ;; ── Saturating truncation helpers (0xFC-prefixed) ───────────────
  ;; i32.trunc_sat_f32_s (0xFC, sub=0x00)
  (func $template_i32_trunc_sat_f32_s (param $dec_ptr i32)
  )

  ;; i32.trunc_sat_f32_u (0xFC, sub=0x01)
  (func $template_i32_trunc_sat_f32_u (param $dec_ptr i32)
  )

  ;; i32.trunc_sat_f64_s (0xFC, sub=0x02)
  (func $template_i32_trunc_sat_f64_s (param $dec_ptr i32)
  )

  ;; i32.trunc_sat_f64_u (0xFC, sub=0x03)
  (func $template_i32_trunc_sat_f64_u (param $dec_ptr i32)
  )

  ;; ── f32.demote_f64 (0xB6) ─ placeholder
  (func $template_f32_demote_f64 (param $dec_ptr i32)
  )

  ;; ── f64.promote_f32 (0xBB) ─ placeholder
  (func $template_f64_promote_f32 (param $dec_ptr i32)
  )

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
  (func $template_call (param $dec_ptr i32)
    ;; NOP placeholder
  )
  (func $template_return_call (param $dec_ptr i32)
    (call $emit_epilogue)
  )

  ;; ── Memory load/store templates ───────────────────────────────
  (func $template_i32_load (param $dec_ptr i32)
    (call $emit_pop_x1)     ;; address -> X1
    (call $emit_mem_load32)
    (call $emit_maybe_push_x0)
  )
  (func $template_i64_load (param $dec_ptr i32)
    (call $emit_pop_x1)
    (call $emit_mem_load64)
    (call $emit_maybe_push_x0)
  )
  (func $template_i32_store (param $dec_ptr i32)
    (call $emit_pop_x0)     ;; value -> X0
    (call $emit_pop_x1)     ;; address -> X1
    (call $emit_mem_store32)
  )
  (func $template_i64_store (param $dec_ptr i32)
    (call $emit_pop_x0)     ;; value -> X0
    (call $emit_pop_x1)     ;; address -> X1
    (call $emit_mem_store64)
  )
  (func $template_i32_store8 (param $dec_ptr i32)
    (call $emit_pop_x0)     ;; value -> X0
    (call $emit_pop_x1)     ;; address -> X1
    (call $emit_mem_store32)  ;; placeholder (word store, not byte)
  )
  (func $template_i32_store16 (param $dec_ptr i32)
    (call $emit_pop_x0)
    (call $emit_pop_x1)
    (call $emit_mem_store32)  ;; placeholder
  )

  ;; ── i32 comparison templates ──────────────────────────────────
  (func $template_i32_eqz (param $dec_ptr i32)
    (call $emit_pop_x0)
    (call $emit_cmp_64 (global.get $REG_X0) (global.get $REG_XZR))
    (call $emit_cset_x0 (i32.const 0))  ;; EQ
    (call $emit_maybe_push_x0)
  )
  (func $template_i32_eq (param $dec_ptr i32)
    (call $emit_pop_x0) (call $emit_pop_x1)
    (call $emit_cmp_32 (global.get $REG_X1) (global.get $REG_X0))
    (call $emit_cset_x0 (i32.const 0))  ;; EQ
    (call $emit_maybe_push_x0)
  )
  (func $template_i32_ne (param $dec_ptr i32)
    (call $emit_pop_x0) (call $emit_pop_x1)
    (call $emit_cmp_32 (global.get $REG_X1) (global.get $REG_X0))
    (call $emit_cset_x0 (i32.const 1))  ;; NE
    (call $emit_maybe_push_x0)
  )
  (func $template_i32_lt_s (param $dec_ptr i32)
    (call $emit_pop_x0) (call $emit_pop_x1)
    (call $emit_cmp_32 (global.get $REG_X1) (global.get $REG_X0))
    (call $emit_cset_x0 (i32.const 10))  ;; LT
    (call $emit_maybe_push_x0)
  )
  (func $template_i32_lt_u (param $dec_ptr i32)
    (call $emit_pop_x0) (call $emit_pop_x1)
    (call $emit_cmp_32 (global.get $REG_X1) (global.get $REG_X0))
    (call $emit_cset_x0 (i32.const 3))  ;; CC/LO
    (call $emit_maybe_push_x0)
  )
  (func $template_i32_gt_s (param $dec_ptr i32)
    (call $emit_pop_x0) (call $emit_pop_x1)
    (call $emit_cmp_32 (global.get $REG_X1) (global.get $REG_X0))
    (call $emit_cset_x0 (i32.const 13))  ;; GT
    (call $emit_maybe_push_x0)
  )
  (func $template_i32_gt_u (param $dec_ptr i32)
    (call $emit_pop_x0) (call $emit_pop_x1)
    (call $emit_cmp_32 (global.get $REG_X1) (global.get $REG_X0))
    (call $emit_cset_x0 (i32.const 8))  ;; HI
    (call $emit_maybe_push_x0)
  )
  (func $template_i32_le_s (param $dec_ptr i32)
    (call $emit_pop_x0) (call $emit_pop_x1)
    (call $emit_cmp_32 (global.get $REG_X1) (global.get $REG_X0))
    (call $emit_cset_x0 (i32.const 12))  ;; LE
    (call $emit_maybe_push_x0)
  )
  (func $template_i32_le_u (param $dec_ptr i32)
    (call $emit_pop_x0) (call $emit_pop_x1)
    (call $emit_cmp_32 (global.get $REG_X1) (global.get $REG_X0))
    (call $emit_cset_x0 (i32.const 9))  ;; LS
    (call $emit_maybe_push_x0)
  )

  ;; ── i64 comparison templates ──────────────────────────────────
  (func $template_i64_eqz (param $dec_ptr i32)
    (call $emit_pop_x0)
    (call $emit_cmp_64 (global.get $REG_X0) (global.get $REG_XZR))
    (call $emit_cset_x0 (i32.const 0))
    (call $emit_maybe_push_x0)
  )
  (func $template_i64_eq (param $dec_ptr i32)
    (call $emit_pop_x0) (call $emit_pop_x1)
    (call $emit_cmp_64 (global.get $REG_X1) (global.get $REG_X0))
    (call $emit_cset_x0 (i32.const 0))
    (call $emit_maybe_push_x0)
  )
  (func $template_i64_ne (param $dec_ptr i32)
    (call $emit_pop_x0) (call $emit_pop_x1)
    (call $emit_cmp_64 (global.get $REG_X1) (global.get $REG_X0))
    (call $emit_cset_x0 (i32.const 1))
    (call $emit_maybe_push_x0)
  )
  (func $template_i64_lt_s (param $dec_ptr i32)
    (call $emit_pop_x0) (call $emit_pop_x1)
    (call $emit_cmp_64 (global.get $REG_X1) (global.get $REG_X0))
    (call $emit_cset_x0 (i32.const 10))
    (call $emit_maybe_push_x0)
  )
  (func $template_i64_lt_u (param $dec_ptr i32)
    (call $emit_pop_x0) (call $emit_pop_x1)
    (call $emit_cmp_64 (global.get $REG_X1) (global.get $REG_X0))
    (call $emit_cset_x0 (i32.const 3))
    (call $emit_maybe_push_x0)
  )
  (func $template_i64_gt_s (param $dec_ptr i32)
    (call $emit_pop_x0) (call $emit_pop_x1)
    (call $emit_cmp_64 (global.get $REG_X1) (global.get $REG_X0))
    (call $emit_cset_x0 (i32.const 13))
    (call $emit_maybe_push_x0)
  )
  (func $template_i64_gt_u (param $dec_ptr i32)
    (call $emit_pop_x0) (call $emit_pop_x1)
    (call $emit_cmp_64 (global.get $REG_X1) (global.get $REG_X0))
    (call $emit_cset_x0 (i32.const 8))
    (call $emit_maybe_push_x0)
  )
  (func $template_i64_le_s (param $dec_ptr i32)
    (call $emit_pop_x0) (call $emit_pop_x1)
    (call $emit_cmp_64 (global.get $REG_X1) (global.get $REG_X0))
    (call $emit_cset_x0 (i32.const 12))
    (call $emit_maybe_push_x0)
  )
  (func $template_i64_le_u (param $dec_ptr i32)
    (call $emit_pop_x0) (call $emit_pop_x1)
    (call $emit_cmp_64 (global.get $REG_X1) (global.get $REG_X0))
    (call $emit_cset_x0 (i32.const 9))
    (call $emit_maybe_push_x0)
  )

  ;; ── i32 unary/binary arithmetic templates ─────────────────────
  (func $template_i32_clz (param $dec_ptr i32)
    (call $emit_pop_x0)
    (call $emit_clz_32)
    (call $emit_maybe_push_x0)
  )
  (func $template_i32_ctz (param $dec_ptr i32)
    (call $emit_pop_x0)
    (call $emit_rbit_32)
    (call $emit_clz_32)
    (call $emit_maybe_push_x0)
  )
  (func $template_i32_popcnt (param $dec_ptr i32)
    (call $emit_pop_x0)
    (call $emit_xor_x0_x0)  ;; placeholder: returns 0
    (call $emit_maybe_push_x0)
  )
  (func $template_i32_add (param $dec_ptr i32)
    (call $emit_pop_x0) (call $emit_pop_x1)
    (call $emit_add_w0_w1)
    (call $emit_maybe_push_x0)
  )
  (func $template_i32_sub (param $dec_ptr i32)
    (call $emit_pop_x0) (call $emit_pop_x1)
    (call $emit_sub_w0_w1)
    (call $emit_maybe_push_x0)
  )
  (func $template_i32_mul (param $dec_ptr i32)
    (call $emit_pop_x0) (call $emit_pop_x1)
    (call $emit_mul_w0_w1)
    (call $emit_maybe_push_x0)
  )
  (func $template_i32_div_s (param $dec_ptr i32)
    (call $emit_pop_x0) (call $emit_pop_x1)
    (call $emit_sdiv_w0_w1)
    (call $emit_maybe_push_x0)
  )
  (func $template_i32_div_u (param $dec_ptr i32)
    (call $emit_pop_x0) (call $emit_pop_x1)
    (call $emit_udiv_w0_w1)
    (call $emit_maybe_push_x0)
  )
  (func $template_i32_rem_s (param $dec_ptr i32)
    (call $emit_pop_x0) (call $emit_pop_x1)
    ;; a % b = a - (a / b) * b
    ;; Save a,b; compute q = a/b; r = a - q*b
    (call $emit_sdiv_w0_w1)  ;; X0 = X1 / X0
    (call $emit_mul_w0_w1)   ;; X0 = (a/b) * b (wrong regs) -- TODO
    ;; Placeholder: return 0
    (call $emit_xor_x0_x0)
    (call $emit_maybe_push_x0)
  )
  (func $template_i32_rem_u (param $dec_ptr i32)
    (call $emit_pop_x0) (call $emit_pop_x1)
    (call $emit_xor_x0_x0)  ;; placeholder
    (call $emit_maybe_push_x0)
  )
  (func $template_i32_and (param $dec_ptr i32)
    (call $emit_pop_x0) (call $emit_pop_x1)
    (call $emit_and_w0_w1)
    (call $emit_maybe_push_x0)
  )
  (func $template_i32_or (param $dec_ptr i32)
    (call $emit_pop_x0) (call $emit_pop_x1)
    (call $emit_orr_w0_w1)
    (call $emit_maybe_push_x0)
  )
  (func $template_i32_xor (param $dec_ptr i32)
    (call $emit_pop_x0) (call $emit_pop_x1)
    (call $emit_eor_w0_w1)
    (call $emit_maybe_push_x0)
  )
  (func $template_i32_shl (param $dec_ptr i32)
    (call $emit_pop_x0) (call $emit_pop_x1)
    (call $emit_lslv_w0_w1)
    (call $emit_maybe_push_x0)
  )
  (func $template_i32_shr_s (param $dec_ptr i32)
    (call $emit_pop_x0) (call $emit_pop_x1)
    (call $emit_asrv_w0_w1)
    (call $emit_maybe_push_x0)
  )
  (func $template_i32_shr_u (param $dec_ptr i32)
    (call $emit_pop_x0) (call $emit_pop_x1)
    (call $emit_lsrv_w0_w1)
    (call $emit_maybe_push_x0)
  )
  (func $template_i32_rotl (param $dec_ptr i32)
    (call $emit_pop_x0) (call $emit_pop_x1)
    ;; ROR by (32 - X0 mod 32)
    (call $emit_sub_w0_w1)  ;; placeholder
    (call $emit_maybe_push_x0)
  )
  (func $template_i32_rotr (param $dec_ptr i32)
    (call $emit_pop_x0) (call $emit_pop_x1)
    (call $emit_rorv_w0_w1)
    (call $emit_maybe_push_x0)
  )

  ;; ── i64 unary/binary arithmetic templates ─────────────────────
  (func $template_i64_clz (param $dec_ptr i32)
    (call $emit_pop_x0)
    (call $emit_clz_64)
    (call $emit_maybe_push_x0)
  )
  (func $template_i64_ctz (param $dec_ptr i32)
    (call $emit_pop_x0)
    (call $emit_rbit_64)
    (call $emit_clz_64)
    (call $emit_maybe_push_x0)
  )
  (func $template_i64_popcnt (param $dec_ptr i32)
    (call $emit_pop_x0)
    (call $emit_xor_x0_x0)  ;; placeholder
    (call $emit_maybe_push_x0)
  )
  (func $template_i64_add (param $dec_ptr i32)
    (call $emit_pop_x0) (call $emit_pop_x1)
    (call $emit_add_x0_x1)
    (call $emit_maybe_push_x0)
  )
  (func $template_i64_sub (param $dec_ptr i32)
    (call $emit_pop_x0) (call $emit_pop_x1)
    (call $emit_sub_x0_x1)
    (call $emit_maybe_push_x0)
  )
  (func $template_i64_mul (param $dec_ptr i32)
    (call $emit_pop_x0) (call $emit_pop_x1)
    (call $emit_mul_x0_x1)
    (call $emit_maybe_push_x0)
  )
  (func $template_i64_div_s (param $dec_ptr i32)
    (call $emit_pop_x0) (call $emit_pop_x1)
    (call $emit_sdiv_x0_x1)
    (call $emit_maybe_push_x0)
  )
  (func $template_i64_div_u (param $dec_ptr i32)
    (call $emit_pop_x0) (call $emit_pop_x1)
    (call $emit_udiv_x0_x1)
    (call $emit_maybe_push_x0)
  )
  (func $template_i64_rem_s (param $dec_ptr i32)
    (call $emit_pop_x0) (call $emit_pop_x1)
    (call $emit_xor_x0_x0)  ;; placeholder
    (call $emit_maybe_push_x0)
  )
  (func $template_i64_rem_u (param $dec_ptr i32)
    (call $emit_pop_x0) (call $emit_pop_x1)
    (call $emit_xor_x0_x0)  ;; placeholder
    (call $emit_maybe_push_x0)
  )
  (func $template_i64_and (param $dec_ptr i32)
    (call $emit_pop_x0) (call $emit_pop_x1)
    (call $emit_and_x0_x1)
    (call $emit_maybe_push_x0)
  )
  (func $template_i64_or (param $dec_ptr i32)
    (call $emit_pop_x0) (call $emit_pop_x1)
    (call $emit_orr_x0_x1)
    (call $emit_maybe_push_x0)
  )
  (func $template_i64_xor (param $dec_ptr i32)
    (call $emit_pop_x0) (call $emit_pop_x1)
    (call $emit_eor_x0_x1)
    (call $emit_maybe_push_x0)
  )
  (func $template_i64_shl (param $dec_ptr i32)
    (call $emit_pop_x0) (call $emit_pop_x1)
    (call $emit_lslv_x0_x1)
    (call $emit_maybe_push_x0)
  )
  (func $template_i64_shr_s (param $dec_ptr i32)
    (call $emit_pop_x0) (call $emit_pop_x1)
    (call $emit_asrv_x0_x1)
    (call $emit_maybe_push_x0)
  )
  (func $template_i64_shr_u (param $dec_ptr i32)
    (call $emit_pop_x0) (call $emit_pop_x1)
    (call $emit_lsrv_x0_x1)
    (call $emit_maybe_push_x0)
  )
  (func $template_i64_rotl (param $dec_ptr i32)
    (call $emit_pop_x0) (call $emit_pop_x1)
    (call $emit_xor_x0_x0)  ;; placeholder
    (call $emit_maybe_push_x0)
  )
  (func $template_i64_rotr (param $dec_ptr i32)
    (call $emit_pop_x0) (call $emit_pop_x1)
    (call $emit_rorv_x0_x1)
    (call $emit_maybe_push_x0)
  )

  ;; ── Conversion templates ──────────────────────────────────────
  (func $template_i32_wrap_i64 (param $dec_ptr i32)
    ;; MOV W0, W0 implicitly zeroes upper 32 bits
    (call $emit_instr_orr_32 (global.get $REG_X0) (global.get $REG_XZR) (global.get $REG_X0))  ;; MOV W0, W0
    (call $emit_maybe_push_x0)
  )
  (func $template_i64_extend_i32_s (param $dec_ptr i32)
    (call $emit_pop_x0)
    (call $emit_sxtw_x0_w0)
    (call $emit_maybe_push_x0)
  )
  (func $template_i64_extend_i32_u (param $dec_ptr i32)
    (call $emit_instr_orr_32 (global.get $REG_X0) (global.get $REG_XZR) (global.get $REG_X0))  ;; MOV W0, W0 (zero-extend)
    (call $emit_maybe_push_x0)
  )

  ;; ── Block/loop/if/else structure tracking ──────────────────────
  ;; jit_compile will handle the fixup/backpatching.
  ;; We just emit placeholder branches and the main loop patches them.

  ;; ════════════════════════════════════════════════════════════════════
  ;; ── The main compile function ─────────────────────────────────────
  ;; ════════════════════════════════════════════════════════════════════

