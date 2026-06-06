  ;; ═════════════════════════════════════════════════════════════════════
  ;; ARM32 (ARMv7-A) Register constants
  ;; ═════════════════════════════════════════════════════════════════════
  ;; R0 = TOS/result (like x86 rax)
  ;; R1 = scratch (like rcx)
  ;; R2 = scratch (like rdx)
  ;; R8 = JitGlobals (like r15/r19)
  ;; R9 = locals cache (like rbx/r20)
  ;; R10 = register-allocated local 0 (like r12/r21)
  ;; R11 = register-allocated local 1 (like r13/r22)
  ;; R12 = intra-call scratch (ip)
  ;; R13 = SP, R14 = LR, R15 = PC

  (global $REG_R0  i32 (i32.const 0))
  (global $REG_R1  i32 (i32.const 1))
  (global $REG_R2  i32 (i32.const 2))
  (global $REG_R8  i32 (i32.const 8))
  (global $REG_R9  i32 (i32.const 9))
  (global $REG_R10 i32 (i32.const 10))
  (global $REG_R11 i32 (i32.const 11))
  (global $REG_R12 i32 (i32.const 12))
  (global $REG_LR  i32 (i32.const 14))
  (global $REG_PC  i32 (i32.const 15))

  ;; ── Helper: write bytes to code cache ──────────────────────────────

  (func $emit_byte (param $b i32)
    (local $p i32)
    (local.set $p (i32.add (global.get $JIT_CACHE) (i32.load (global.get $JS_CODE_PTR))))
    (i32.store8 (local.get $p) (local.get $b))
    (i32.store (global.get $JS_CODE_PTR) (i32.add (i32.load (global.get $JS_CODE_PTR)) (i32.const 1)))
  )

  (func $emit_dword (param $v i32)
    (call $emit_byte (i32.and (local.get $v) (i32.const 0xFF)))
    (call $emit_byte (i32.and (i32.shr_u (local.get $v) (i32.const 8)) (i32.const 0xFF)))
    (call $emit_byte (i32.and (i32.shr_u (local.get $v) (i32.const 16)) (i32.const 0xFF)))
    (call $emit_byte (i32.and (i32.shr_u (local.get $v) (i32.const 24)) (i32.const 0xFF)))
  )

  (func $emit_qword (param $v i64)
    (call $emit_dword (i32.wrap_i64 (local.get $v)))
    (call $emit_dword (i32.wrap_i64 (i64.shr_u (local.get $v) (i64.const 32))))
  )

  ;; ── ARM32 instruction emitter ─────────────────────────────────────
  ;; All ARM instructions are exactly 4 bytes (32 bits)

  (func $emit_instr (param $val i32)
    (call $emit_dword (local.get $val))
  )

  ;; ── ARM32 Data Processing (ALU) helpers ───────────────────────────
  ;; Base: cond=AL(1110), opcode(4), S=0, Rn(4), Rd(4), operand2
  ;; For register operand2: imm5(5) shift(2) shift_type(2) Rm(4)

  ;; MOV Rd, Rm: 0xE1A00000 | (Rd << 12) | Rm
  (func $emit_instr_mov (param $rd i32) (param $rm i32)
    (call $emit_instr
      (i32.or (i32.const 0xE1A00000)
              (i32.or (i32.shl (local.get $rd) (i32.const 12))
                      (local.get $rm))))
  )

  ;; MOV Rd, #imm (8-bit, zero-extended)
  ;; ARM immediate encoding: 0xE3A00000 | (Rd << 12) | imm8
  ;; Only handles 0-255 directly
  (func $emit_instr_movi (param $rd i32) (param $imm8 i32)
    (call $emit_instr
      (i32.or (i32.const 0xE3A00000)
              (i32.or (i32.shl (local.get $rd) (i32.const 12))
                      (i32.and (local.get $imm8) (i32.const 0xFF)))))
  )

  ;; ADD Rd, Rn, Rm: 0xE0800000 | (Rn << 16) | (Rd << 12) | Rm
  (func $emit_instr_add (param $rd i32) (param $rn i32) (param $rm i32)
    (call $emit_instr
      (i32.or (i32.const 0xE0800000)
              (i32.or (i32.shl (local.get $rn) (i32.const 16))
                      (i32.or (i32.shl (local.get $rd) (i32.const 12))
                              (local.get $rm)))))
  )

  ;; SUB Rd, Rn, Rm: 0xE0400000 | (Rn << 16) | (Rd << 12) | Rm
  (func $emit_instr_sub (param $rd i32) (param $rn i32) (param $rm i32)
    (call $emit_instr
      (i32.or (i32.const 0xE0400000)
              (i32.or (i32.shl (local.get $rn) (i32.const 16))
                      (i32.or (i32.shl (local.get $rd) (i32.const 12))
                              (local.get $rm)))))
  )

  ;; RSB Rd, Rn, Rm (reverse sub: Rd = Rm - Rn): 0xE0600000 | ...
  (func $emit_instr_rsb (param $rd i32) (param $rn i32) (param $rm i32)
    (call $emit_instr
      (i32.or (i32.const 0xE0600000)
              (i32.or (i32.shl (local.get $rn) (i32.const 16))
                      (i32.or (i32.shl (local.get $rd) (i32.const 12))
                              (local.get $rm)))))
  )

  ;; AND Rd, Rn, Rm: 0xE0000000 | (Rn << 16) | (Rd << 12) | Rm
  (func $emit_instr_and (param $rd i32) (param $rn i32) (param $rm i32)
    (call $emit_instr
      (i32.or (i32.const 0xE0000000)
              (i32.or (i32.shl (local.get $rn) (i32.const 16))
                      (i32.or (i32.shl (local.get $rd) (i32.const 12))
                              (local.get $rm)))))
  )

  ;; ORR Rd, Rn, Rm: 0xE1800000 | (Rn << 16) | (Rd << 12) | Rm
  (func $emit_instr_orr (param $rd i32) (param $rn i32) (param $rm i32)
    (call $emit_instr
      (i32.or (i32.const 0xE1800000)
              (i32.or (i32.shl (local.get $rn) (i32.const 16))
                      (i32.or (i32.shl (local.get $rd) (i32.const 12))
                              (local.get $rm)))))
  )

  ;; EOR Rd, Rn, Rm: 0xE0200000 | (Rn << 16) | (Rd << 12) | Rm
  (func $emit_instr_eor (param $rd i32) (param $rn i32) (param $rm i32)
    (call $emit_instr
      (i32.or (i32.const 0xE0200000)
              (i32.or (i32.shl (local.get $rn) (i32.const 16))
                      (i32.or (i32.shl (local.get $rd) (i32.const 12))
                              (local.get $rm)))))
  )

  ;; MUL Rd, Rn, Rm: 0xE0000900 | (Rd << 16) | (Rn << 12) | Rm
  (func $emit_instr_mul (param $rd i32) (param $rn i32) (param $rm i32)
    (call $emit_instr
      (i32.or (i32.const 0xE0000900)
              (i32.or (i32.shl (local.get $rd) (i32.const 16))
                      (i32.or (i32.shl (local.get $rn) (i32.const 12))
                              (local.get $rm)))))
  )

  ;; SDIV Rd, Rn, Rm (ARMv7): 0xE710F010 | (Rn << 16) | (Rd << 12) | Rm
  (func $emit_instr_sdiv (param $rd i32) (param $rn i32) (param $rm i32)
    (call $emit_instr
      (i32.or (i32.const 0xE710F010)
              (i32.or (i32.shl (local.get $rn) (i32.const 16))
                      (i32.or (i32.shl (local.get $rd) (i32.const 12))
                              (local.get $rm)))))
  )

  ;; UDIV Rd, Rn, Rm (ARMv7): 0xE730F010 | (Rn << 16) | (Rd << 12) | Rm
  (func $emit_instr_udiv (param $rd i32) (param $rn i32) (param $rm i32)
    (call $emit_instr
      (i32.or (i32.const 0xE730F010)
              (i32.or (i32.shl (local.get $rn) (i32.const 16))
                      (i32.or (i32.shl (local.get $rd) (i32.const 12))
                              (local.get $rm)))))
  )

  ;; LSL Rd, Rn, Rm (variable shift): 0xE1A00010 | (Rn << 16) | (Rd << 12) | Rm
  ;; Actually for variable LSL: MOV Rd, Rn, LSL Rm
  ;; Encoding: cond 0001 1010 S Rn Rd 0001 0 Rm  -- with shift
  ;; Actually: MOV Rd, Rn, LSL Rm = 0xE1A00010 | (Rn << 16) | (Rd << 12) | Rm
  ;; Wait, that doesn't look right. MOV Rd, Rn, LSL Rm:
  ;;   cond=1110, 000 1101 0, S=0, Rn, Rd, 0001 0, Rm
  ;; Hmm, the barrel shifter encoding for LSL with register is:
  ;;   shift_imm=0000, shift=00, 1, Rm (for LSL by register)
  ;; This gives: 0xE1A00010 | (Rn << 16) | (Rd << 12) | Rm
  (func $emit_instr_lsl (param $rd i32) (param $rn i32) (param $rm i32)
    (call $emit_instr
      (i32.or (i32.const 0xE1A00010)
              (i32.or (i32.shl (local.get $rn) (i32.const 16))
                      (i32.or (i32.shl (local.get $rd) (i32.const 12))
                              (local.get $rm)))))
  )

  ;; LSR Rd, Rn, Rm: MOV Rd, Rn, LSR Rm = 0xE1A00030 | ...
  (func $emit_instr_lsr (param $rd i32) (param $rn i32) (param $rm i32)
    (call $emit_instr
      (i32.or (i32.const 0xE1A00030)
              (i32.or (i32.shl (local.get $rn) (i32.const 16))
                      (i32.or (i32.shl (local.get $rd) (i32.const 12))
                              (local.get $rm)))))
  )

  ;; ASR Rd, Rn, Rm: MOV Rd, Rn, ASR Rm = 0xE1A00050 | ...
  (func $emit_instr_asr (param $rd i32) (param $rn i32) (param $rm i32)
    (call $emit_instr
      (i32.or (i32.const 0xE1A00050)
              (i32.or (i32.shl (local.get $rn) (i32.const 16))
                      (i32.or (i32.shl (local.get $rd) (i32.const 12))
                              (local.get $rm)))))
  )

  ;; ROR Rd, Rn, Rm: MOV Rd, Rn, ROR Rm = 0xE1A00070 | ...
  (func $emit_instr_ror (param $rd i32) (param $rn i32) (param $rm i32)
    (call $emit_instr
      (i32.or (i32.const 0xE1A00070)
              (i32.or (i32.shl (local.get $rn) (i32.const 16))
                      (i32.or (i32.shl (local.get $rd) (i32.const 12))
                              (local.get $rm)))))
  )

  ;; ── ARM32 ADD/SUB with immediate ─────────────────────────────────

  ;; ADD Rd, Rn, #imm8 (with rotate): 0xE2800000 | (Rn << 16) | (Rd << 12) | imm12
  ;; imm12 = [rotate:imm8] where rotate is 4-bit (even rotate amount)
  ;; For small immediates (0-255): rotate=0, so imm12 = imm8
  (func $emit_instr_addi (param $rd i32) (param $rn i32) (param $imm8 i32)
    (call $emit_instr
      (i32.or (i32.const 0xE2800000)
              (i32.or (i32.shl (local.get $rn) (i32.const 16))
                      (i32.or (i32.shl (local.get $rd) (i32.const 12))
                              (i32.and (local.get $imm8) (i32.const 0xFF))))))
  )

  ;; SUB Rd, Rn, #imm8: 0xE2400000 | (Rn << 16) | (Rd << 12) | imm8
  (func $emit_instr_subi (param $rd i32) (param $rn i32) (param $imm8 i32)
    (call $emit_instr
      (i32.or (i32.const 0xE2400000)
              (i32.or (i32.shl (local.get $rn) (i32.const 16))
                      (i32.or (i32.shl (local.get $rd) (i32.const 12))
                              (i32.and (local.get $imm8) (i32.const 0xFF))))))
  )

  ;; ── ARM32 CLZ (count leading zeros) ──────────────────────────────
  ;; CLZ Rd, Rm: 0xE16F0F10 | (Rd << 12) | Rm
  (func $emit_instr_clz (param $rd i32) (param $rm i32)
    (call $emit_instr
      (i32.or (i32.const 0xE16F0F10)
              (i32.or (i32.shl (local.get $rd) (i32.const 12))
                      (local.get $rm))))
  )

  ;; ── ARM32 RBIT (reverse bits) ────────────────────────────────────
  ;; RBIT Rd, Rm: 0xE6FF0F30 | (Rd << 12) | Rm
  (func $emit_instr_rbit (param $rd i32) (param $rm i32)
    (call $emit_instr
      (i32.or (i32.const 0xE6FF0F30)
              (i32.or (i32.shl (local.get $rd) (i32.const 12))
                      (local.get $rm))))
  )

  ;; ── ARM32 Load/Store helpers ─────────────────────────────────────
  ;; LDR Rt, [Rn, Rm]: 0xE7900000 | (Rn << 16) | (Rt << 12) | Rm
  (func $emit_instr_ldr_reg (param $rt i32) (param $rn i32) (param $rm i32)
    (call $emit_instr
      (i32.or (i32.const 0xE7900000)
              (i32.or (i32.shl (local.get $rn) (i32.const 16))
                      (i32.or (i32.shl (local.get $rt) (i32.const 12))
                              (local.get $rm)))))
  )

  ;; STR Rt, [Rn, Rm]: 0xE7800000 | (Rn << 16) | (Rt << 12) | Rm
  (func $emit_instr_str_reg (param $rt i32) (param $rn i32) (param $rm i32)
    (call $emit_instr
      (i32.or (i32.const 0xE7800000)
              (i32.or (i32.shl (local.get $rn) (i32.const 16))
                      (i32.or (i32.shl (local.get $rt) (i32.const 12))
                              (local.get $rm)))))
  )

  ;; LDR Rt, [Rn, #imm12]: 0xE5900000 | (Rn << 16) | (Rt << 12) | imm12
  (func $emit_instr_ldr_off (param $rt i32) (param $rn i32) (param $imm12 i32)
    (call $emit_instr
      (i32.or (i32.const 0xE5900000)
              (i32.or (i32.shl (local.get $rn) (i32.const 16))
                      (i32.or (i32.shl (local.get $rt) (i32.const 12))
                              (i32.and (local.get $imm12) (i32.const 0xFFF))))))
  )

  ;; STR Rt, [Rn, #imm12]: 0xE5800000 | (Rn << 16) | (Rt << 12) | imm12
  (func $emit_instr_str_off (param $rt i32) (param $rn i32) (param $imm12 i32)
    (call $emit_instr
      (i32.or (i32.const 0xE5800000)
              (i32.or (i32.shl (local.get $rn) (i32.const 16))
                      (i32.or (i32.shl (local.get $rt) (i32.const 12))
                              (i32.and (local.get $imm12) (i32.const 0xFFF))))))
  )

  ;; LDR pre/post-index helpers for stack

  ;; STR Rt, [SP, #-4]! (pre-index, push): 0xE52D0004 | (Rt << 12)
  ;; Actually STMDB SP!, {Rt} would be better
  ;; But for single reg: STR Rt, [SP, -#imm12]! = 0xE52D0000 | (Rt << 12) | imm12
  ;; For imm12=4: 0xE52D0004 | (Rt << 12)
  (func $emit_instr_str_pre_sp (param $rt i32)
    (call $emit_instr
      (i32.or (i32.const 0xE52D0004)
              (i32.shl (local.get $rt) (i32.const 12))))
  )

  ;; LDR Rt, [SP], #4 (post-index, pop): 0xE49D0004 | (Rt << 12)
  (func $emit_instr_ldr_post_sp (param $rt i32)
    (call $emit_instr
      (i32.or (i32.const 0xE49D0004)
              (i32.shl (local.get $rt) (i32.const 12))))
  )

  ;; ── ARM32 Stack push/pop (using real ARM stack) ──────────────────

  ;; Push R0: STR R0, [SP, #-4]!
  (func $emit_push_r0
    (call $emit_instr_str_pre_sp (global.get $REG_R0))
  )

  ;; Pop R0: LDR R0, [SP], #4
  (func $emit_pop_r0
    (call $emit_instr_ldr_post_sp (global.get $REG_R0))
  )

  ;; Push R1
  (func $emit_push_r1
    (call $emit_instr_str_pre_sp (global.get $REG_R1))
  )

  ;; Pop R1
  (func $emit_pop_r1
    (call $emit_instr_ldr_post_sp (global.get $REG_R1))
  )

  ;; Push R2
  (func $emit_push_r2
    (call $emit_instr_str_pre_sp (global.get $REG_R2))
  )

  ;; Pop R2
  (func $emit_pop_r2
    (call $emit_instr_ldr_post_sp (global.get $REG_R2))
  )

  ;; Pop pair: pop R1 then R0
  (func $emit_pop2_r1_r0
    (call $emit_pop_r1)
    (call $emit_pop_r0)
  )

  ;; Push pair: push R1 then R0 (actually reverse: push R0 then R1)
  (func $emit_push2_r1_r0
    (call $emit_push_r1)
    (call $emit_push_r0)
  )

  ;; ── Standard push/pop names ─────────────────────────────────────
  (func $emit_pop_rax_alias
    (call $emit_pop_r0)
  )
  (func $emit_push_rax_alias
    (call $emit_push_r0)
  )
  (func $emit_pop_rcx_alias
    (call $emit_pop_r1)
  )
  (func $emit_push_rcx_alias
    (call $emit_push_r1)
  )
  (func $emit_pop_rdx_alias
    (call $emit_pop_r2)
  )

  ;; ── ARM32 branch instructions ────────────────────────────────────

  ;; B #imm (unconditional branch, ±32MB): 0xEA000000 | imm24
  ;; imm24 = (target - pc - 8) >> 2, signed 24-bit
  (func $emit_b (param $off i32)
    (call $emit_instr
      (i32.or (i32.const 0xEA000000)
              (i32.and (i32.shr_s (local.get $off) (i32.const 2)) (i32.const 0x00FFFFFF)))
    )
  )

  ;; BL #imm (branch with link): 0xEB000000 | imm24
  (func $emit_bl (param $off i32)
    (call $emit_instr
      (i32.or (i32.const 0xEB000000)
              (i32.and (i32.shr_s (local.get $off) (i32.const 2)) (i32.const 0x00FFFFFF)))
    )
  )

  ;; B.cond #imm (conditional branch, ±32MB): cond 1010 imm24
  ;; cond codes: EQ=0, NE=1, CS=2, CC=3, MI=4, PL=5, VS=6, VC=7
  ;; HI=8, LS=9, GE=10, LT=11, GT=12, LE=13, AL=14
  (func $emit_b_cond (param $off i32) (param $cond i32)
    (call $emit_instr
      (i32.or (i32.shl (local.get $cond) (i32.const 28))
              (i32.or (i32.const 0x0A000000)
                      (i32.and (i32.shr_s (local.get $off) (i32.const 2)) (i32.const 0x00FFFFFF))))
    )
  )

  ;; CBZ/CBNZ not available in ARM32 (it's a Thumb-2 instruction)
  ;; Use CMP + B cond instead

  ;; ── ARM32 CMP/TST helpers ────────────────────────────────────────

  ;; CMP Rn, Rm: 0xE1500000 | (Rn << 16) | Rm
  (func $emit_cmp (param $rn i32) (param $rm i32)
    (call $emit_instr
      (i32.or (i32.const 0xE1500000)
              (i32.or (i32.shl (local.get $rn) (i32.const 16))
                      (local.get $rm))))
  )

  ;; CMP Rn, #imm8: 0xE3500000 | (Rn << 16) | imm8
  (func $emit_cmpi (param $rn i32) (param $imm8 i32)
    (call $emit_instr
      (i32.or (i32.const 0xE3500000)
              (i32.or (i32.shl (local.get $rn) (i32.const 16))
                      (i32.and (local.get $imm8) (i32.const 0xFF)))))
  )

  ;; TST Rn, Rm: 0xE1100000 | (Rn << 16) | Rm
  (func $emit_tst (param $rn i32) (param $rm i32)
    (call $emit_instr
      (i32.or (i32.const 0xE1100000)
              (i32.or (i32.shl (local.get $rn) (i32.const 16))
                      (local.get $rm))))
  )

  ;; ── Conditional moves (for CSET) ──────────────────────────────────
  ;; MOVNE R0, #1 (move if not equal): 0x13A00001 | ...
  ;; Actually: MOVcc Rd, #imm8 where cc is the condition
  ;; MOVNE R0, #1: 0x13A00001
  (func $emit_mov_cond (param $rd i32) (param $imm8 i32) (param $cond i32)
    (call $emit_instr
      (i32.or (i32.shl (local.get $cond) (i32.const 28))
              (i32.or (i32.const 0x03A00000)
                      (i32.or (i32.shl (local.get $rd) (i32.const 12))
                              (i32.and (local.get $imm8) (i32.const 0xFF))))))
  )

  ;; ── ARM32 sign extension ─────────────────────────────────────────
  ;; SXTB Rd, Rm (sign-extend byte to word): 0xE6AF0070 | (Rd << 12) | Rm
  (func $emit_instr_sxtb (param $rd i32) (param $rm i32)
    (call $emit_instr
      (i32.or (i32.const 0xE6AF0070)
              (i32.or (i32.shl (local.get $rd) (i32.const 12))
                      (local.get $rm))))
  )

  ;; SXTH Rd, Rm (sign-extend halfword to word): 0xE6BF0070 | (Rd << 12) | Rm
  (func $emit_instr_sxth (param $rd i32) (param $rm i32)
    (call $emit_instr
      (i32.or (i32.const 0xE6BF0070)
              (i32.or (i32.shl (local.get $rd) (i32.const 12))
                      (local.get $rm))))
  )

  ;; ── ARM32 Memory load/store through JitGlobals ──────────────────
  ;; JitGlobals layout matches x86-64/AArch64:
  ;;   +0: locals pointer
  ;;   +4: mem_ptr (WASM linear memory base)
  ;;   +8: ...
  ;;   +24: globals_buf
  ;;   +48: table_entries
  ;;   +64: memory_pages

  ;; Load mem_ptr into R2: LDR R2, [R8, #4]
  (func $emit_ldr_mem_ptr
    (call $emit_instr_ldr_off (global.get $REG_R2) (global.get $REG_R8) (i32.const 4))
  )

  ;; LDR R0, [R8, #offset]
  (func $emit_ldr_r0_r8 (param $off12 i32)
    (call $emit_instr_ldr_off (global.get $REG_R0) (global.get $REG_R8) (local.get $off12))
  )

  ;; STR R0, [R8, #offset]
  (func $emit_str_r0_r8 (param $off12 i32)
    (call $emit_instr_str_off (global.get $REG_R0) (global.get $REG_R8) (local.get $off12))
  )

  ;; LDR R0, [R9, #offset]
  (func $emit_ldr_r0_r9 (param $off12 i32)
    (call $emit_instr_ldr_off (global.get $REG_R0) (global.get $REG_R9) (local.get $off12))
  )

  ;; STR R0, [R9, #offset]
  (func $emit_str_r0_r9 (param $off12 i32)
    (call $emit_instr_str_off (global.get $REG_R0) (global.get $REG_R9) (local.get $off12))
  )

  ;; ── ARM32 memory load/store via register offset ─────────────────

  ;; i32.load: address in R1, result in R0
  ;; LDR R0, [R2, R1] where R2 = mem_ptr
  (func $emit_mem_load32
    (call $emit_ldr_mem_ptr)
    (call $emit_instr_ldr_reg (global.get $REG_R0) (global.get $REG_R2) (global.get $REG_R1))
  )

  ;; i64.load: same as i32.load (ARM32 is 32-bit, handles 32-bit only)
  ;; LDR R0, [R2, R1] - only loads lower 32 bits
  (func $emit_mem_load64
    (call $emit_ldr_mem_ptr)
    (call $emit_instr_ldr_reg (global.get $REG_R0) (global.get $REG_R2) (global.get $REG_R1))
  )

  ;; i32.store: address in R1, value in R0
  (func $emit_mem_store32
    (call $emit_ldr_mem_ptr)
    (call $emit_instr_str_reg (global.get $REG_R0) (global.get $REG_R2) (global.get $REG_R1))
  )

  ;; i64.store: same as i32.store (32-bit only)
  (func $emit_mem_store64
    (call $emit_ldr_mem_ptr)
    (call $emit_instr_str_reg (global.get $REG_R0) (global.get $REG_R2) (global.get $REG_R1))
  )

  ;; ── ARM32 Unary arithmetic helpers (R0←op(R0)) ──────────────────

  ;; CLZ R0, R0
  (func $emit_clz_32
    (call $emit_instr_clz (global.get $REG_R0) (global.get $REG_R0))
  )

  ;; RBIT R0, R0
  (func $emit_rbit_32
    (call $emit_instr_rbit (global.get $REG_R0) (global.get $REG_R0))
  )

  ;; ── ARM32 VFP (float) register encoding helpers ────────────────
  ;;
  ;; VFP single register number (5-bit, S0-S31) is split into:
  ;;   D/N/M bit + 4-bit V field
  ;; Two conventions:
  ;;   Data-processing (VADD, VCVT, etc): D = reg[4], V = reg[3:0]
  ;;   VMOV (ARM↔VFP): Vn:N = reg[4:1]:reg[0] → N = reg[0], Vn = reg[4:1]

  ;; ── VFP 3-operand single (VADD/VSUB/VMUL/VDIV.F32) ────────────
  ;; Encoding: cond(4) 1110(4) D(1) opc(3) Sn[3:0](4) Sd[3:0](4) 1010(4) Sn[4](1) sbz(1) Sm[4](1) 0(1) Sm[3:0](4)
  ;; Register split: D=Sd[4], Vd=Sd[3:0], N=Sn[4], Vn=Sn[3:0], M=Sm[4], Vm=Sm[3:0]
  (func $emit_vfp_3op_s (param $base i32) (param $sd i32) (param $sn i32) (param $sm i32)
    (local $insn i32)
    (local.set $insn (i32.const 0xE0000000))
    (local.set $insn (i32.or (local.get $insn) (local.get $base)))
    (local.set $insn (i32.or (local.get $insn) (i32.shl (i32.shr_u (local.get $sd) (i32.const 4)) (i32.const 23))))
    (local.set $insn (i32.or (local.get $insn) (i32.shl (i32.and (local.get $sn) (i32.const 0xF)) (i32.const 16))))
    (local.set $insn (i32.or (local.get $insn) (i32.shl (i32.and (local.get $sd) (i32.const 0xF)) (i32.const 12))))
    (local.set $insn (i32.or (local.get $insn) (i32.shl (i32.shr_u (local.get $sn) (i32.const 4)) (i32.const 7))))
    (local.set $insn (i32.or (local.get $insn) (i32.shl (i32.shr_u (local.get $sm) (i32.const 4)) (i32.const 5))))
    (local.set $insn (i32.or (local.get $insn) (i32.and (local.get $sm) (i32.const 0xF))))
    (call $emit_instr (local.get $insn))
  )

  ;; VFP 3-operand double: same as single with Dreg = Dnum << 1 (into S encoding)
  (func $emit_vfp_3op_d (param $base i32) (param $dd i32) (param $dn i32) (param $dm i32)
    (call $emit_vfp_3op_s
      (local.get $base)
      (i32.shl (local.get $dd) (i32.const 1))
      (i32.shl (local.get $dn) (i32.const 1))
      (i32.shl (local.get $dm) (i32.const 1)))
  )

  ;; ── VFP 2-operand monadic single (VABS/VNEG/VSQRT/FRINT.F32) ──
  ;; Encoding: cond 1110 D 1 op2 Vd Vn 1011 N 0 M 0 Vm
  ;; For monadic: Vd[3:0]=Sd[3:0]+bit23=Sd[4], Vm[3:0]=Sm[3:0]+bit5=Sm[4]
  (func $emit_vfp_2op_s (param $base i32) (param $sd i32) (param $sm i32)
    (local $insn i32)
    (local.set $insn (i32.const 0xE0000000))
    (local.set $insn (i32.or (local.get $insn) (local.get $base)))
    (local.set $insn (i32.or (local.get $insn) (i32.shl (i32.shr_u (local.get $sd) (i32.const 4)) (i32.const 23))))
    (local.set $insn (i32.or (local.get $insn) (i32.shl (i32.and (local.get $sd) (i32.const 0xF)) (i32.const 12))))
    (local.set $insn (i32.or (local.get $insn) (i32.shl (i32.shr_u (local.get $sm) (i32.const 4)) (i32.const 5))))
    (local.set $insn (i32.or (local.get $insn) (i32.and (local.get $sm) (i32.const 0xF))))
    (call $emit_instr (local.get $insn))
  )

  (func $emit_vfp_2op_d (param $base i32) (param $dd i32) (param $dm i32)
    (call $emit_vfp_2op_s
      (local.get $base)
      (i32.shl (local.get $dd) (i32.const 1))
      (i32.shl (local.get $dm) (i32.const 1)))
  )

  ;; ── VFP compare single ──────────────────────────────────────────
  ;; FCMP Sd, Sm: cond 1110 D 1 0 1 Vd Vn 1011 N 0 M 0 Vm  (bits 20:16 = 10100?)
  ;; Using kernel defines: FEXT_FCMP = 0x00040000 (bit 18), bit7=0
  ;; Base with Sd=0, Sm=0: 0xEEB40A40
  (func $emit_vfp_cmp_s (param $base i32) (param $sd i32) (param $sm i32)
    (local $insn i32)
    (local.set $insn (i32.const 0xE0000000))
    (local.set $insn (i32.or (local.get $insn) (local.get $base)))
    (local.set $insn (i32.or (local.get $insn) (i32.shl (i32.shr_u (local.get $sd) (i32.const 4)) (i32.const 23))))
    (local.set $insn (i32.or (local.get $insn) (i32.shl (i32.and (local.get $sd) (i32.const 0xF)) (i32.const 12))))
    (local.set $insn (i32.or (local.get $insn) (i32.shl (i32.shr_u (local.get $sm) (i32.const 4)) (i32.const 5))))
    (local.set $insn (i32.or (local.get $insn) (i32.and (local.get $sm) (i32.const 0xF))))
    (call $emit_instr (local.get $insn))
  )

  (func $emit_vfp_cmp_d (param $base i32) (param $dd i32) (param $dm i32)
    (call $emit_vfp_cmp_s
      (local.get $base)
      (i32.shl (local.get $dd) (i32.const 1))
      (i32.shl (local.get $dm) (i32.const 1)))
  )

  ;; ── Move ARM ↔ VFP (VMOV) ─────────────────────────────────────
  ;; VMOV Sd, Rt (ARM→VFP 32-bit):
  ;;   cond 1110 00 0 0 Vn Rt 1010 N 001 0000
  ;;   Where Vn:N = (Sd>>1):(Sd&1)
  ;;   = 0xEE000A10 | ((Sd>>1)<<16) | ((Sd&1)<<7) | Rt
  (func $emit_fmov_s_w (param $sd i32) (param $wn i32)
    (call $emit_instr
      (i32.or (i32.const 0xEE000A10)
              (i32.or (i32.shl (i32.shr_u (local.get $sd) (i32.const 1)) (i32.const 16))
                      (i32.or (i32.shl (i32.and (local.get $sd) (i32.const 1)) (i32.const 7))
                              (local.get $wn)))))
  )

  ;; VMOV Rd, Sn (VFP→ARM 32-bit):
  ;;   cond 1110 00 0 1 Vn Rd 1010 N 001 0000
  ;;   = 0xEE100A10 | ((Sn>>1)<<16) | ((Sn&1)<<7) | Rd
  (func $emit_fmov_w_s (param $wd i32) (param $sn i32)
    (call $emit_instr
      (i32.or (i32.const 0xEE100A10)
              (i32.or (i32.shl (i32.shr_u (local.get $sn) (i32.const 1)) (i32.const 16))
                      (i32.or (i32.shl (i32.and (local.get $sn) (i32.const 1)) (i32.const 7))
                              (local.get $wd)))))
  )

  ;; VMOV Dd, Xn (ARM→VFP 64-bit) not needed for ARM32 (no 64-bit ARM)
  (func $emit_fmov_d_x (param $dd i32) (param $xn i32)
    ;; Placeholder: not implemented for ARM32
  )

  ;; VMOV Xd, Dn (VFP→ARM 64-bit) not needed for ARM32
  (func $emit_fmov_x_d (param $xd i32) (param $dn i32)
    ;; Placeholder: not implemented for ARM32
  )

  ;; ── VFP 3-operand bases (F32) ──────────────────────────────────
  ;; VADD.F32: base = 0x0E300A00, VSUB.F32: base = 0x0E300A40
  ;; VMUL.F32: base = 0x0E200A00, VDIV.F32: base = 0x0E800A00
  ;; VADD.F64: base = 0x0E300B00 (bit8=1), VSUB.F64: base = 0x0E300B40
  ;; VMUL.F64: base = 0x0E200B00, VDIV.F64: base = 0x0E800B00

  (func $emit_fadd_s (param $sd i32) (param $sn i32) (param $sm i32)
    (call $emit_vfp_3op_s (i32.const 0x0E300A00) (local.get $sd) (local.get $sn) (local.get $sm)))
  (func $emit_fsub_s (param $sd i32) (param $sn i32) (param $sm i32)
    (call $emit_vfp_3op_s (i32.const 0x0E300A40) (local.get $sd) (local.get $sn) (local.get $sm)))
  (func $emit_fmul_s (param $sd i32) (param $sn i32) (param $sm i32)
    (call $emit_vfp_3op_s (i32.const 0x0E200A00) (local.get $sd) (local.get $sn) (local.get $sm)))
  (func $emit_fdiv_s (param $sd i32) (param $sn i32) (param $sm i32)
    (call $emit_vfp_3op_s (i32.const 0x0E800A00) (local.get $sd) (local.get $sn) (local.get $sm)))
  (func $emit_fadd_d (param $dd i32) (param $dn i32) (param $dm i32)
    (call $emit_vfp_3op_d (i32.const 0x0E300B00) (local.get $dd) (local.get $dn) (local.get $dm)))
  (func $emit_fsub_d (param $dd i32) (param $dn i32) (param $dm i32)
    (call $emit_vfp_3op_d (i32.const 0x0E300B40) (local.get $dd) (local.get $dn) (local.get $dm)))
  (func $emit_fmul_d (param $dd i32) (param $dn i32) (param $dm i32)
    (call $emit_vfp_3op_d (i32.const 0x0E200B00) (local.get $dd) (local.get $dn) (local.get $dm)))
  (func $emit_fdiv_d (param $dd i32) (param $dn i32) (param $dm i32)
    (call $emit_vfp_3op_d (i32.const 0x0E800B00) (local.get $dd) (local.get $dn) (local.get $dm)))

  ;; ── VFP compare ────────────────────────────────────────────────
  ;; FCMPE.F32 Sd, Sm: use base 0x0EB40A40
  ;; FCMPE.F64 Dd, Dm: use base 0x0EB40B40

  (func $emit_fcmp_s0_s1
    (call $emit_vfp_cmp_s (i32.const 0x0EB40A40) (i32.const 0) (i32.const 1)))
  (func $emit_fcmp_d0_d1
    (call $emit_vfp_cmp_d (i32.const 0x0EB40B40) (i32.const 0) (i32.const 1)))
  (func $emit_fcmp_s0_s0
    (call $emit_vfp_cmp_s (i32.const 0x0EB40A40) (i32.const 0) (i32.const 0)))
  (func $emit_fcmp_d0_d0
    (call $emit_vfp_cmp_d (i32.const 0x0EB40B40) (i32.const 0) (i32.const 0)))

  ;; ── Float-to-int conversions (FCVTZ) ──────────────────────────
  ;; VCVT.S32.F32 Sd, Sm: base = 0x0EBD0A40
  ;; VCVT.S32.F64 Sd, Dm: base = 0x0EBD0B40

  (func $emit_fcvtz_w_s (param $wd i32) (param $sn i32)
    (call $emit_vfp_2op_s (i32.const 0x0EBD0A40) (local.get $wd) (local.get $sn))
  )
  (func $emit_fcvtz_x_s (param $xd i32) (param $sn i32)
    (call $emit_fcvtz_w_s (local.get $xd) (local.get $sn))
  )
  (func $emit_fcvtz_w_d (param $wd i32) (param $dn i32)
    (call $emit_vfp_2op_d (i32.const 0x0EBD0B40) (local.get $wd) (local.get $dn))
  )
  (func $emit_fcvtz_x_d (param $xd i32) (param $dn i32)
    (call $emit_fcvtz_w_d (local.get $xd) (local.get $dn))
  )

  ;; ── Int-to-float conversions (SCVTF/UCVTF) ────────────────────
  ;; VCVT.F32.S32 Sd, Sm: base = 0x0EB80A40
  ;; VCVT.F32.U32 Sd, Sm: base = 0x0EB80A40 (bit for signed/unsigned may differ)

  (func $emit_scvtf_s_w (param $sd i32) (param $wn i32)
    (call $emit_fmov_s_w (local.get $sd) (local.get $wn))
    (call $emit_vfp_2op_s (i32.const 0x0EB80A40) (local.get $sd) (local.get $sd))
  )
  (func $emit_ucvtf_s_w (param $sd i32) (param $wn i32)
    (call $emit_fmov_s_w (local.get $sd) (local.get $wn))
    (call $emit_vfp_2op_s (i32.const 0x0EB80A40) (local.get $sd) (local.get $sd))
  )
  (func $emit_scvtf_d_x (param $dd i32) (param $xn i32)
    (call $emit_scvtf_s_w (local.get $dd) (local.get $xn))
  )
  (func $emit_ucvtf_d_x (param $dd i32) (param $xn i32)
    (call $emit_ucvtf_s_w (local.get $dd) (local.get $xn))
  )
  (func $emit_scvtf_d_w (param $dd i32) (param $wn i32)
    (call $emit_scvtf_s_w (local.get $dd) (local.get $wn))
  )
  (func $emit_scvtf_s_x (param $sd i32) (param $xn i32)
    (call $emit_scvtf_s_w (local.get $sd) (local.get $xn))
  )
  (func $emit_ucvtf_s_x (param $sd i32) (param $xn i32)
    (call $emit_ucvtf_s_w (local.get $sd) (local.get $xn))
  )

  ;; ── Floating precision conversion (FCVT) ──────────────────────
  ;; VCVT.F32.F64 Sd, Dm: base = 0x0EB60B40
  ;; VCVT.F64.F32 Dd, Sm: base = 0x0EB70A40

  (func $emit_fcvt_s_d (param $sd i32) (param $dn i32)
    (call $emit_vfp_2op_s (i32.const 0x0EB60B40) (local.get $sd) (local.get $dn))
  )
  (func $emit_fcvt_d_s (param $dd i32) (param $sn i32)
    (call $emit_vfp_2op_d (i32.const 0x0EB70A40) (local.get $dd) (local.get $sn))
  )

  ;; ── FRINT (rounding) ──────────────────────────────────────────
  ;; ARM32 VFPv3 doesn't have dedicated FRINT; uses VCVT round-trip
  ;; For now, emit NOP-like instruction to keep the value

  (func $emit_frintp_s0_s0
    (call $emit_vfp_2op_s (i32.const 0x0EB80A40) (i32.const 0) (i32.const 0)))
  (func $emit_frintm_s0_s0
    (call $emit_vfp_2op_s (i32.const 0x0EB80A40) (i32.const 0) (i32.const 0)))
  (func $emit_frintz_s0_s0
    (call $emit_vfp_2op_s (i32.const 0x0EB80A40) (i32.const 0) (i32.const 0)))
  (func $emit_frintn_s0_s0
    (call $emit_vfp_2op_s (i32.const 0x0EB80A40) (i32.const 0) (i32.const 0)))
  (func $emit_frintp_d0_d0
    (call $emit_vfp_2op_d (i32.const 0x0EB80B40) (i32.const 0) (i32.const 0)))
  (func $emit_frintm_d0_d0
    (call $emit_vfp_2op_d (i32.const 0x0EB80B40) (i32.const 0) (i32.const 0)))
  (func $emit_frintz_d0_d0
    (call $emit_vfp_2op_d (i32.const 0x0EB80B40) (i32.const 0) (i32.const 0)))
  (func $emit_frintn_d0_d0
    (call $emit_vfp_2op_d (i32.const 0x0EB80B40) (i32.const 0) (i32.const 0)))

  ;; ── FSQRT ─────────────────────────────────────────────────────
  ;; VSQRT.F32 Sd, Sm: base = 0x0EB10AC0 (kernel: FEXT_FSQRT = 0x00010080)
  ;; VSQRT.F64 Dd, Dm: base = 0x0EB10BC0

  (func $emit_fsqrt_s0_s0
    (call $emit_vfp_2op_s (i32.const 0x0EB10AC0) (i32.const 0) (i32.const 0)))
  (func $emit_fsqrt_d0_d0
    (call $emit_vfp_2op_d (i32.const 0x0EB10BC0) (i32.const 0) (i32.const 0)))

  ;; ═════════════════════════════════════════════════════════════════════
  ;; ELF / flat binary output
  ;; ═════════════════════════════════════════════════════════════════════

  ;; BL with target VA: compute relative offset and emit BL
  (func $emit_bl_rel (param $target_va i32)
    (local $saved i32)
    (local.set $saved (i32.load (global.get $JS_CODE_PTR)))
    (call $emit_bl
      (i32.sub
        (local.get $target_va)
        (i32.add
          (i32.add (global.get $TEXT_VA) (i32.sub (local.get $saved) (global.get $ELF_OUT_OFF)))
          (i32.const 8))))
  )

  ;; ── ELF32 header (52 bytes) ─────────────────────────────────────

  (func $emit_elf32_ehdr (param $entry_va i32) (param $phoff i32) (param $phnum i32)
    (call $emit_byte (i32.const 0x7F))
    (call $emit_byte (i32.const 0x45)) (call $emit_byte (i32.const 0x4C)) (call $emit_byte (i32.const 0x46))
    (call $emit_byte (i32.const 1))     (call $emit_byte (i32.const 1))
    (call $emit_byte (i32.const 1))     (call $emit_byte (i32.const 0))
    (call $emit_dword (i32.const 0))    ;; padding bytes 8-11
    (call $emit_dword (i32.const 0))    ;; padding bytes 12-15
    (call $emit_byte (i32.const 2)) (call $emit_byte (i32.const 0))  ;; e_type
    (call $emit_byte (i32.const 0x28)) (call $emit_byte (i32.const 0))  ;; e_machine=ARM
    (call $emit_dword (i32.const 1))    ;; e_version
    (call $emit_dword (local.get $entry_va))  ;; e_entry
    (call $emit_dword (local.get $phoff))     ;; e_phoff
    (call $emit_dword (i32.const 0))    ;; e_shoff
    (call $emit_dword (i32.const 0x05000000))  ;; e_flags (EABI v5)
    (call $emit_byte (i32.const 52)) (call $emit_byte (i32.const 0))  ;; e_ehsize
    (call $emit_byte (i32.const 32)) (call $emit_byte (i32.const 0))  ;; e_phentsize
    (call $emit_byte (local.get $phnum)) (call $emit_byte (i32.const 0))  ;; e_phnum
    (call $emit_byte (i32.const 0)) (call $emit_byte (i32.const 0))  ;; e_shentsize
    (call $emit_byte (i32.const 0)) (call $emit_byte (i32.const 0))  ;; e_shnum
    (call $emit_byte (i32.const 0)) (call $emit_byte (i32.const 0))  ;; e_shstrndx
  )

  ;; ── ELF32 program header (32 bytes) ────────────────────────────

  (func $emit_elf32_phdr (param $type i32) (param $flags i32)
                         (param $offset i32) (param $vaddr i32)
                         (param $filesz i32) (param $memsz i32)
    (call $emit_dword (local.get $type))
    (call $emit_dword (local.get $offset))
    (call $emit_dword (local.get $vaddr))
    (call $emit_dword (local.get $vaddr))  ;; p_paddr = p_vaddr
    (call $emit_dword (local.get $filesz))
    (call $emit_dword (local.get $memsz))
    (call $emit_dword (local.get $flags))
    (call $emit_dword (i32.const 0x1000))
  )

  ;; ── Runtime stub for ELF: sets up R8=JitGlobals, calls code, exits ──

  (func $emit_elf_stub (param $bss_va i32)
    (local $jitglobs i32) (local $mem i32) (local $locals i32)
    (local $globals i32) (local $table i32)

    (local.set $jitglobs (local.get $bss_va))
    (local.set $mem     (i32.add (local.get $bss_va) (i32.const 0x80)))
    (local.set $locals  (i32.add (local.get $bss_va) (i32.const 0x10080)))
    (local.set $globals (i32.add (local.get $bss_va) (i32.const 0x20080)))
    (local.set $table   (i32.add (local.get $bss_va) (i32.const 0x30080)))

    (call $emit_instr_movz_64 (i32.const 8) (i32.const 0) (i32.and (local.get $jitglobs) (i32.const 0xFFFF)))
    (call $emit_instr_movk_64 (i32.const 8) (i32.const 0) (i32.shr_u (local.get $jitglobs) (i32.const 16)))

    (call $emit_instr_movz_64 (i32.const 0) (i32.const 0) (i32.and (local.get $mem) (i32.const 0xFFFF)))
    (call $emit_instr_movk_64 (i32.const 0) (i32.const 0) (i32.shr_u (local.get $mem) (i32.const 16)))
    (call $emit_instr_str_off (global.get $REG_R0) (global.get $REG_R8) (i32.const 4))

    (call $emit_instr_movz_64 (i32.const 0) (i32.const 0) (i32.and (local.get $locals) (i32.const 0xFFFF)))
    (call $emit_instr_movk_64 (i32.const 0) (i32.const 0) (i32.shr_u (local.get $locals) (i32.const 16)))
    (call $emit_instr_str_off (global.get $REG_R0) (global.get $REG_R8) (i32.const 0))

    (call $emit_instr_movz_64 (i32.const 0) (i32.const 0) (i32.and (local.get $globals) (i32.const 0xFFFF)))
    (call $emit_instr_movk_64 (i32.const 0) (i32.const 0) (i32.shr_u (local.get $globals) (i32.const 16)))
    (call $emit_instr_str_off (global.get $REG_R0) (global.get $REG_R8) (i32.const 8))

    (call $emit_instr_movz_64 (i32.const 0) (i32.const 0) (i32.and (local.get $table) (i32.const 0xFFFF)))
    (call $emit_instr_movk_64 (i32.const 0) (i32.const 0) (i32.shr_u (local.get $table) (i32.const 16)))
    (call $emit_instr_str_off (global.get $REG_R0) (global.get $REG_R8) (i32.const 12))

    (call $emit_instr_movi (i32.const 0) (i32.const 1))
    (call $emit_instr_str_off (global.get $REG_R0) (global.get $REG_R8) (i32.const 16))

    (call $emit_bl_rel (i32.add (global.get $TEXT_VA) (global.get $ELF_CODE_OFF)))

    (call $emit_instr_movi (i32.const 7) (global.get $LINUX_SYS_ARM32_EXIT))  ;; MOV R7, #1 (SYS_exit)
    (call $emit_instr (i32.const 0xEF000000))            ;; SVC #0
  )

  ;; ── Copy compiled code from JIT cache to output ────────────────

  (func $copy_compiled_code (param $src i32) (param $size i32) (param $dst_off i32)
    (local $i i32) (local $dst i32)
    (local.set $dst (i32.add (global.get $ELF_OUT_BUF) (local.get $dst_off)))
    (local.set $i (i32.const 0))
    (block $done
      (loop $copy
        (if (i32.ge_u (local.get $i) (local.get $size)) (then (br $done)))
        (i32.store8
          (i32.add (local.get $dst) (local.get $i))
          (i32.load8_u (i32.add (local.get $src) (local.get $i))))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $copy)
      )
    )
  )

  ;; ── compile_to_elf: compile WASM, emit ELF32 executable ─────────

  (func (export "compile_to_elf") (param $func_idx i32) (result i32 i32)
    (local $code_size i32) (local $total_size i32)
    (local $saved i32) (local $bss_va i32)

    (local.set $code_size (call $jit_compile (local.get $func_idx)))
    (local.set $total_size (i32.add (global.get $ELF_CODE_OFF) (local.get $code_size)))
    (local.set $bss_va
      (i32.add
        (i32.and (i32.add (local.get $total_size) (i32.const 0xFFF)) (i32.const -0x1000))
        (global.get $TEXT_VA)))

    (local.set $saved (i32.load (global.get $JS_CODE_PTR)))
    (i32.store (global.get $JS_CODE_PTR) (global.get $ELF_OUT_OFF))

    (call $emit_elf32_ehdr
      (i32.add (global.get $TEXT_VA) (global.get $ELF_STUB_OFF))
      (i32.const 52)
      (i32.const 1))

    (call $emit_elf32_phdr
      (i32.const 1)         ;; PT_LOAD
      (i32.const 7)         ;; PF_R | PF_W | PF_X
      (i32.const 0)         ;; p_offset
      (global.get $TEXT_VA) ;; p_vaddr
      (local.get $total_size) ;; p_filesz
      (i32.add (i32.and (i32.add (local.get $total_size) (i32.const 0xFFF)) (i32.const -0x1000)) (global.get $BSS_SIZE))) ;; p_memsz

    (call $emit_elf_stub (local.get $bss_va))

    (block $pad_done
      (loop $pad_loop
        (if (i32.ge_u (i32.load (global.get $JS_CODE_PTR))
                       (i32.add (global.get $ELF_OUT_OFF) (global.get $ELF_CODE_OFF)))
          (then (br $pad_done)))
        (call $emit_byte (i32.const 0))
        (br $pad_loop)
      )
    )

    (call $copy_compiled_code (global.get $JIT_CACHE) (local.get $code_size) (global.get $ELF_CODE_OFF))
    (i32.store (global.get $JS_CODE_PTR) (local.get $saved))

    (return (global.get $ELF_OUT_BUF) (local.get $total_size))
  )

  ;; ═════════════════════════════════════════════════════════════════════
  ;; Flat binary output (bare-metal ARM32)
  ;; ═════════════════════════════════════════════════════════════════════

  ;; Store result and halt helper: STR R0, [R3]; B .
  (func $emit_store_and_halt
    (local $addr i32)
    (local.set $addr (i32.const 0x500))
    (call $emit_instr_movz_64 (i32.const 3) (i32.const 0) (i32.and (local.get $addr) (i32.const 0xFFFF)))
    (call $emit_instr_movk_64 (i32.const 3) (i32.const 0) (i32.shr_u (local.get $addr) (i32.const 16)))
    (call $emit_instr_str_off (global.get $REG_R0) (i32.const 3) (i32.const 0))
    (call $emit_instr (i32.const 0xEAFFFFFE))  ;; B . (infinite loop: 0xEAFFFFFF + 1 = 0xEA000000 | (-1))
  )

  ;; Emit bare-metal stub (no headers). Returns stub size.
  ;; Compiled code placed right after stub.
  (func $emit_bare_metal_stub (result i32)
    (local $stub_size i32) (local $current_off i32)
    ;; R8 = JitGlobals
    (call $emit_instr_movz_64 (i32.const 8) (i32.const 0) (i32.and (global.get $BSS_JITGLOBALS) (i32.const 0xFFFF)))
    (call $emit_instr_movk_64 (i32.const 8) (i32.const 0) (i32.shr_u (global.get $BSS_JITGLOBALS) (i32.const 16)))
    ;; R0 = mem; STR R0, [R8, #4]
    (call $emit_instr_movz_64 (i32.const 0) (i32.const 0) (i32.and (global.get $BSS_MEM) (i32.const 0xFFFF)))
    (call $emit_instr_movk_64 (i32.const 0) (i32.const 0) (i32.shr_u (global.get $BSS_MEM) (i32.const 16)))
    (call $emit_instr_str_off (global.get $REG_R0) (global.get $REG_R8) (i32.const 4))
    ;; R0 = locals; STR R0, [R8, #0]
    (call $emit_instr_movz_64 (i32.const 0) (i32.const 0) (i32.and (global.get $BSS_LOCALS) (i32.const 0xFFFF)))
    (call $emit_instr_movk_64 (i32.const 0) (i32.const 0) (i32.shr_u (global.get $BSS_LOCALS) (i32.const 16)))
    (call $emit_instr_str_off (global.get $REG_R0) (global.get $REG_R8) (i32.const 0))
    ;; R0 = globals; STR R0, [R8, #8]
    (call $emit_instr_movz_64 (i32.const 0) (i32.const 0) (i32.and (global.get $BSS_GLOBALS) (i32.const 0xFFFF)))
    (call $emit_instr_movk_64 (i32.const 0) (i32.const 0) (i32.shr_u (global.get $BSS_GLOBALS) (i32.const 16)))
    (call $emit_instr_str_off (global.get $REG_R0) (global.get $REG_R8) (i32.const 8))
    ;; R0 = table; STR R0, [R8, #12]
    (call $emit_instr_movz_64 (i32.const 0) (i32.const 0) (i32.and (global.get $BSS_TABLE) (i32.const 0xFFFF)))
    (call $emit_instr_movk_64 (i32.const 0) (i32.const 0) (i32.shr_u (global.get $BSS_TABLE) (i32.const 16)))
    (call $emit_instr_str_off (global.get $REG_R0) (global.get $REG_R8) (i32.const 12))
    ;; MOV R0, #1; STR R0, [R8, #16] (memory_pages = 1)
    (call $emit_instr_movi (i32.const 0) (i32.const 1))
    (call $emit_instr_str_off (global.get $REG_R0) (global.get $REG_R8) (i32.const 16))

    ;; Compute stub size: current offset + BL(4) + store_and_halt(16) = 20
    (local.set $current_off (i32.sub (i32.load (global.get $JS_CODE_PTR)) (global.get $BIN_OUT_OFF)))
    (local.set $stub_size (i32.add (local.get $current_off) (i32.const 20)))

    ;; BL to compiled code at offset stub_size (ARM32 PC = current + 8)
    (call $emit_bl
      (i32.sub (local.get $stub_size) (i32.add (local.get $current_off) (i32.const 8))))

    ;; Store result at 0x500 and halt
    (call $emit_store_and_halt)

    (return (local.get $stub_size))
  )

  ;; ── Copy code to binary output buffer ──────────────────────────

  (func $copy_code_to (param $dst_buf i32) (param $code_size i32) (param $code_off i32)
    (local $i i32) (local $dst i32)
    (local.set $dst (i32.add (local.get $dst_buf) (local.get $code_off)))
    (local.set $i (i32.const 0))
    (block $done
      (loop $copy
        (if (i32.ge_u (local.get $i) (local.get $code_size)) (then (br $done)))
        (i32.store8
          (i32.add (local.get $dst) (local.get $i))
          (i32.load8_u (i32.add (global.get $JIT_CACHE) (local.get $i))))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $copy)
      )
    )
  )

  ;; ── compile_to_bin: emit flat binary ───────────────────────────

  (func (export "compile_to_bin") (param $func_idx i32) (result i32 i32)
    (local $code_size i32) (local $stub_size i32) (local $total_size i32)
    (local $saved i32)

    (local.set $code_size (call $jit_compile (local.get $func_idx)))
    (local.set $saved (i32.load (global.get $JS_CODE_PTR)))
    (i32.store (global.get $JS_CODE_PTR) (global.get $BIN_OUT_OFF))

    (local.set $stub_size (call $emit_bare_metal_stub))
    (call $copy_code_to (global.get $BIN_OUT_BUF) (local.get $code_size) (local.get $stub_size))
    (local.set $total_size (i32.add (local.get $stub_size) (local.get $code_size)))

    (i32.store (global.get $JS_CODE_PTR) (local.get $saved))
    (return (global.get $BIN_OUT_BUF) (local.get $total_size))
  )

  ;; ── ARM32 function prologue/epilogue ──────────────────────────────

  ;; Prologue: save LR, save callee-saved regs, setup frame, zero temp regs
  (func $emit_prologue
    ;; PUSH {R8-R11, LR} - STMDB SP!, {R8-R11, LR}
    ;; register mask: bits 8-11 + 14 = 0x1E00 | 0x4000 = 0x5E00
    ;; STMDB SP!, {R8-R11, LR}: 0xE92D0000 | mask
    (call $emit_instr (i32.const 0xE92D5E00))  ;; STMDB SP!, {R8-R11, LR}

    ;; MOV R11, SP (set frame pointer)
    (call $emit_instr (i32.const 0xE1A0B00D))  ;; MOV R11, SP

    ;; Zero R12 (used as XZR / zero register for CMP operations)
    (call $emit_instr (i32.const 0xE02CC00C))  ;; EOR R12, R12, R12
  )

  ;; Epilogue: restore callee-saved, return
  (func $emit_epilogue
    ;; MOV SP, R11 (restore SP)
    (call $emit_instr (i32.const 0xE1A0D00B))  ;; MOV SP, R11

    ;; LDMIA SP!, {R8-R11, PC} - 0xE8BD0000 | mask
    (call $emit_instr (i32.const 0xE8BD5E00))  ;; LDMIA SP!, {R8-R11, PC}
  )

  ;; ── JIT state helpers ─────────────────────────────────────────────

  (func $jit_reset_state
    (i32.store (global.get $JS_CODE_PTR) (i32.const 0))
    (i32.store (global.get $JS_LABEL_DEPTH) (i32.const 0))
    (i32.store (global.get $JS_FIXUP_COUNT) (i32.const 0))
    (i32.store (global.get $JS_RETURN_EMITTED) (i32.const 0))
    (i32.store (global.get $JS_STACK_DEPTH) (i32.const 0))
    (i32.store (global.get $JS_MAX_STACK) (i32.const 0))
  )

  (func $get_decoded_imm (param $dec_ptr i32) (param $offset i32) (result i32)
    (i32.load (i32.add (local.get $dec_ptr) (local.get $offset)))
  )

  (func $get_compiled_code (export "get_compiled_code") (param $func_idx i32) (result i32 i32)
    (local $slot i32) (local $base i32)
    (local.set $slot (i32.and (local.get $func_idx) (i32.const 3)))
    (local.set $base (i32.add (global.get $JIT_CACHE) (i32.mul (local.get $slot) (global.get $JIT_SLOT_SIZE))))
    (local.get $base)
    (i32.sub (i32.load (global.get $JS_CODE_PTR)) (i32.mul (local.get $slot) (global.get $JIT_SLOT_SIZE)))
  )

  ;; ── Convenience arithmetic/stack helpers ──────────────────────────

  ;; ADD R0, R0, R1
  (func $emit_add_r0_r1
    (call $emit_instr_add (global.get $REG_R0) (global.get $REG_R0) (global.get $REG_R1))
  )

  ;; SUB R0, R0, R1
  (func $emit_sub_r0_r1
    (call $emit_instr_sub (global.get $REG_R0) (global.get $REG_R0) (global.get $REG_R1))
  )

  ;; MUL R0, R0, R1
  (func $emit_mul_r0_r1
    (call $emit_instr_mul (global.get $REG_R0) (global.get $REG_R0) (global.get $REG_R1))
  )

  ;; SDIV R0, R0, R1
  (func $emit_sdiv_r0_r1
    (call $emit_instr_sdiv (global.get $REG_R0) (global.get $REG_R0) (global.get $REG_R1))
  )

  ;; UDIV R0, R0, R1
  (func $emit_udiv_r0_r1
    (call $emit_instr_udiv (global.get $REG_R0) (global.get $REG_R0) (global.get $REG_R1))
  )

  ;; AND R0, R0, R1
  (func $emit_and_r0_r1
    (call $emit_instr_and (global.get $REG_R0) (global.get $REG_R0) (global.get $REG_R1))
  )

  ;; ORR R0, R0, R1
  (func $emit_orr_r0_r1
    (call $emit_instr_orr (global.get $REG_R0) (global.get $REG_R0) (global.get $REG_R1))
  )

  ;; EOR R0, R0, R1
  (func $emit_eor_r0_r1
    (call $emit_instr_eor (global.get $REG_R0) (global.get $REG_R0) (global.get $REG_R1))
  )

  ;; LSL R0, R0, R1 (variable shift)
  (func $emit_lslv_r0_r1
    (call $emit_instr_lsl (global.get $REG_R0) (global.get $REG_R0) (global.get $REG_R1))
  )

  ;; LSR R0, R0, R1
  (func $emit_lsrv_r0_r1
    (call $emit_instr_lsr (global.get $REG_R0) (global.get $REG_R0) (global.get $REG_R1))
  )

  ;; ASR R0, R0, R1
  (func $emit_asrv_r0_r1
    (call $emit_instr_asr (global.get $REG_R0) (global.get $REG_R0) (global.get $REG_R1))
  )

  ;; ROR R0, R0, R1
  (func $emit_rorv_r0_r1
    (call $emit_instr_ror (global.get $REG_R0) (global.get $REG_R0) (global.get $REG_R1))
  )

  ;; ── Pop two values: pop R1 (right), pop R0 (left) ────────────────
  (func $emit_pop2_r1_r0_order
    (call $emit_pop_r1)
    (call $emit_pop_r0)
  )

  ;; ── XOR R0, R0 (zero register) ────────────────────────────────────
  ;; EOR R0, R0, R0 = 0xE0200000
  (func $emit_xor_r0_r0
    (call $emit_instr (i32.const 0xE0200000))
  )

  ;; EOR R1, R1, R1
  (func $emit_xor_r1_r1
    (call $emit_instr (i32.const 0xE0211001))
  )

  ;; EOR R2, R2, R2
  (func $emit_xor_r2_r2
    (call $emit_instr (i32.const 0xE0222002))
  )

  ;; MOV R0, R1
  (func $emit_mov_r0_r1
    (call $emit_instr_mov (global.get $REG_R0) (global.get $REG_R1))
  )

  ;; MOV R1, R0
  (func $emit_mov_r1_r0
    (call $emit_instr_mov (global.get $REG_R1) (global.get $REG_R0))
  )

  ;; ── Alias: "x0" names for templates expecting x86-like names ──────
  (func $emit_add_x0_x1 (call $emit_add_r0_r1))
  (func $emit_sub_x0_x1 (call $emit_sub_r0_r1))
  (func $emit_mul_x0_x1 (call $emit_mul_r0_r1))
  (func $emit_sdiv_x0_x1 (call $emit_sdiv_r0_r1))
  (func $emit_udiv_x0_x1 (call $emit_udiv_r0_r1))
  (func $emit_and_x0_x1 (call $emit_and_r0_r1))
  (func $emit_orr_x0_x1 (call $emit_orr_r0_r1))
  (func $emit_eor_x0_x1 (call $emit_eor_r0_r1))
  (func $emit_lslv_x0_x1 (call $emit_lslv_r0_r1))
  (func $emit_lsrv_x0_x1 (call $emit_lsrv_r0_r1))
  (func $emit_asrv_x0_x1 (call $emit_asrv_r0_r1))
  (func $emit_rorv_x0_x1 (call $emit_rorv_r0_r1))
  ;; 32-bit aliases (same on ARM32)
  (func $emit_add_w0_w1 (call $emit_add_r0_r1))
  (func $emit_sub_w0_w1 (call $emit_sub_r0_r1))
  (func $emit_mul_w0_w1 (call $emit_mul_r0_r1))
  (func $emit_sdiv_w0_w1 (call $emit_sdiv_r0_r1))
  (func $emit_udiv_w0_w1 (call $emit_udiv_r0_r1))
  (func $emit_and_w0_w1 (call $emit_and_r0_r1))
  (func $emit_orr_w0_w1 (call $emit_orr_r0_r1))
  (func $emit_eor_w0_w1 (call $emit_eor_r0_r1))
  (func $emit_lslv_w0_w1 (call $emit_lslv_r0_r1))
  (func $emit_lsrv_w0_w1 (call $emit_lsrv_r0_r1))
  (func $emit_asrv_w0_w1 (call $emit_asrv_r0_r1))
  (func $emit_rorv_w0_w1 (call $emit_rorv_r0_r1))

  ;; Push/pop aliases
  (func $emit_push_x0 (call $emit_push_r0))
  (func $emit_pop_x0 (call $emit_pop_r0))
  (func $emit_push_x1 (call $emit_push_r1))
  (func $emit_pop_x1 (call $emit_pop_r1))
  (func $emit_push_x2 (call $emit_push_r2))
  (func $emit_pop_x2 (call $emit_pop_r2))
  (func $emit_pop_x1_alias (call $emit_pop_r1))
  (func $emit_pop2_x1_x0 (call $emit_pop2_r1_r0))
  (func $emit_push2_x1_x0 (call $emit_push2_r1_r0))

  ;; Zero register aliases
  (func $emit_xor_x0_x0 (call $emit_xor_r0_r0))
  (func $emit_xor_w0_w0 (call $emit_xor_r0_r0))
  (func $emit_xor_x1_x1 (call $emit_xor_r1_r1))
  (func $emit_xor_x2_x2 (call $emit_xor_r2_r2))

  ;; Move aliases
  (func $emit_mov_x0_x1 (call $emit_mov_r0_r1))
  (func $emit_mov_x1_x0 (call $emit_mov_r1_r0))
  (func $emit_mov_w0_w1 (call $emit_mov_r0_r1))
  (func $emit_mov_w1_w0 (call $emit_mov_r1_r0))

  ;; Sign extension alias (no-op on ARM32 since we only have 32-bit)
  (func $emit_sxtw_x0_w0)

  ;; Compare aliases
  (func $emit_cmp_64 (param $rn i32) (param $rm i32)
    (call $emit_cmp (local.get $rn) (local.get $rm))
  )
  (func $emit_cmp_32 (param $rn i32) (param $rm i32)
    (call $emit_cmp (local.get $rn) (local.get $rm))
  )
  (func $emit_tst_64 (param $rn i32) (param $rm i32)
    (call $emit_tst (local.get $rn) (local.get $rm))
  )
  (func $emit_tst_32 (param $rn i32) (param $rm i32)
    (call $emit_tst (local.get $rn) (local.get $rm))
  )

  ;; CSET X0, cond: set R0 to 1 if cond true, else 0
  ;; Input: inv_cond = inverted condition (since templates pass inverted)
  ;; original cond = inv_cond ^ 1
  ;; Strategy:
  ;;   MOVcc R0, #1   (where cc = original cond)
  ;;   MOV!cc R0, #0  (where !cc = original cond ^ 1)
  (func $emit_cset_x0 (param $inv_cond i32)
    (local $cond i32)
    (local.set $cond (i32.xor (local.get $inv_cond) (i32.const 1)))
    ;; MOVcc R0, #1: 0xE3A00001 | (cond << 28)
    (call $emit_instr
      (i32.or (i32.const 0xE3A00001)
              (i32.shl (local.get $cond) (i32.const 28))))
    ;; MOV!cc R0, #0: 0xE3A00000 | ((cond ^ 1) << 28)
    (call $emit_instr
      (i32.or (i32.const 0xE3A00000)
              (i32.shl (i32.xor (local.get $cond) (i32.const 1)) (i32.const 28))))
  )
  (func $emit_cset_w0 (param $inv_cond i32)
    (call $emit_cset_x0 (local.get $inv_cond))
  )

  ;; Immediate loading helpers (ARM32 version)
  ;; For MOV/MOVT on ARMv7+: use MOVW/MOVT
  ;; But simpler: use MOV + MOVT
  ;; Actually MOVW Rd, #imm16: 0xE3000000 | ... complex encoding
  ;; Let me use a multi-instruction approach

  ;; For loading 32-bit constant into R0:
  ;; Just emit multiple MOV/ORR instructions as needed
  ;; But for now, let me define the same-named functions as empty stubs
  ;; that the templates call, and implement the simple cases

  ;; MOVZ R0, #imm16 (zero-extend 16-bit)
  ;; ARM32: MOVW Rd, #imm16: 0xE3000000 | (Rd << 12) | imm12
  ;;   where imm12 = imm16[11:0] | (imm16[15:12] << 16)
  ;; Actually: MOVW encoding: cond 0011 0000 imm4 Rd imm12 
  ;;   0xE3000000 | (Rd << 12) | (imm16 & 0xFFF) | ((imm16 >> 12) << 16)
  (func $emit_instr_movz_64 (param $rd i32) (param $hw i32) (param $imm16 i32)
    (call $emit_instr
      (i32.or (i32.const 0xE3000000)
              (i32.or (i32.shl (local.get $rd) (i32.const 12))
                      (i32.or (i32.and (local.get $imm16) (i32.const 0xFFF))
                              (i32.shl (i32.shr_u (local.get $imm16) (i32.const 12)) (i32.const 16))))))
  )

  (func $emit_instr_movz_32 (param $rd i32) (param $hw i32) (param $imm16 i32)
    (call $emit_instr_movz_64 (local.get $rd) (local.get $hw) (local.get $imm16))
  )

  ;; MOVK (move keep): MOVT Rd, #imm16 (sets bits 31:16)
  ;; MOVT Rd, #imm16: 0xE3400000 | (Rd << 12) | imm12 | ((imm16>>12) << 16)
  (func $emit_instr_movk_64 (param $rd i32) (param $hw i32) (param $imm16 i32)
    (call $emit_instr
      (i32.or (i32.const 0xE3400000)
              (i32.or (i32.shl (local.get $rd) (i32.const 12))
                      (i32.or (i32.and (local.get $imm16) (i32.const 0xFFF))
                              (i32.shl (i32.shr_u (local.get $imm16) (i32.const 12)) (i32.const 16))))))
  )

  ;; MOVN (move not): MVN Rd, #imm
  (func $emit_instr_movn_64 (param $rd i32) (param $hw i32) (param $imm16 i32)
    ;; MVN Rd, #imm16: compute the immediate, then MVN
    ;; For MVN we need to encode the value as an 8-bit rotated immediate
    ;; Simple case: MVN Rd, #0 = 0xE3E00000 | (Rd << 12)  -- gives -1
    (call $emit_instr
      (i32.or (i32.const 0xE3E00000)
              (i32.or (i32.shl (local.get $rd) (i32.const 12))
                      (i32.and (local.get $imm16) (i32.const 0xFF)))))
  )

  ;; ADD immediate (aliases)
  (func $emit_instr_addi_64 (param $rd i32) (param $rn i32) (param $imm12 i32)
    (call $emit_instr_addi (local.get $rd) (local.get $rn) (local.get $imm12))
  )
  (func $emit_instr_addi_32 (param $rd i32) (param $rn i32) (param $imm12 i32)
    (call $emit_instr_addi (local.get $rd) (local.get $rn) (local.get $imm12))
  )
  (func $emit_instr_subi_64 (param $rd i32) (param $rn i32) (param $imm12 i32)
    (call $emit_instr_subi (local.get $rd) (local.get $rn) (local.get $imm12))
  )
  (func $emit_instr_subi_32 (param $rd i32) (param $rn i32) (param $imm12 i32)
    (call $emit_instr_subi (local.get $rd) (local.get $rn) (local.get $imm12))
  )

  ;; LDR immediate (aliases)
  (func $emit_instr_ldr_64_off (param $rt i32) (param $rn i32) (param $imm12 i32)
    (call $emit_instr_ldr_off (local.get $rt) (local.get $rn) (local.get $imm12))
  )
  (func $emit_instr_ldr_32_off (param $rt i32) (param $rn i32) (param $imm12 i32)
    (call $emit_instr_ldr_off (local.get $rt) (local.get $rn) (local.get $imm12))
  )
  (func $emit_instr_str_64_off (param $rt i32) (param $rn i32) (param $imm12 i32)
    (call $emit_instr_str_off (local.get $rt) (local.get $rn) (local.get $imm12))
  )
  (func $emit_instr_str_32_off (param $rt i32) (param $rn i32) (param $imm12 i32)
    (call $emit_instr_str_off (local.get $rt) (local.get $rn) (local.get $imm12))
  )

  ;; Pre/post index stack helpers (aliases)
  (func $emit_instr_str_pre_64 (param $rt i32) (param $rn i32) (param $imm9 i32)
    (call $emit_instr_str_pre_sp (local.get $rt))
  )
  (func $emit_instr_ldr_post_64 (param $rt i32) (param $rn i32) (param $imm9 i32)
    (call $emit_instr_ldr_post_sp (local.get $rt))
  )
  (func $emit_instr_str_pre_32 (param $rt i32) (param $rn i32) (param $imm9 i32)
    (call $emit_instr_str_pre_sp (local.get $rt))
  )
  (func $emit_instr_ldr_post_32 (param $rt i32) (param $rn i32) (param $imm9 i32)
    (call $emit_instr_ldr_post_sp (local.get $rt))
  )

  ;; Register offset load/store (aliases for 'x' naming)
  (func $emit_ldr_reg_64 (param $rt i32) (param $rn i32) (param $rm i32)
    (call $emit_instr_ldr_reg (local.get $rt) (local.get $rn) (local.get $rm))
  )
  (func $emit_ldr_reg_32 (param $rt i32) (param $rn i32) (param $rm i32)
    (call $emit_instr_ldr_reg (local.get $rt) (local.get $rn) (local.get $rm))
  )
  (func $emit_str_reg_64 (param $rt i32) (param $rn i32) (param $rm i32)
    (call $emit_instr_str_reg (local.get $rt) (local.get $rn) (local.get $rm))
  )
  (func $emit_str_reg_32 (param $rt i32) (param $rn i32) (param $rm i32)
    (call $emit_instr_str_reg (local.get $rt) (local.get $rn) (local.get $rm))
  )

  ;; LDR x0 from x19/r8 aliases
  (func $emit_ldr_x0_x19 (param $off12 i32)
    (call $emit_instr_ldr_off (global.get $REG_R0) (global.get $REG_R8) (local.get $off12))
  )
  (func $emit_str_x0_x19 (param $off12 i32)
    (call $emit_instr_str_off (global.get $REG_R0) (global.get $REG_R8) (local.get $off12))
  )
  (func $emit_ldr_x0_x20 (param $off12 i32)
    (call $emit_instr_ldr_off (global.get $REG_R0) (global.get $REG_R9) (local.get $off12))
  )
  (func $emit_str_x0_x20 (param $off12 i32)
    (call $emit_instr_str_off (global.get $REG_R0) (global.get $REG_R9) (local.get $off12))
  )

  ;; CBZ/CBNZ equivalents using CMP + B.cond
  (func $emit_cbz_x (param $rt i32) (param $off i32)
    (call $emit_cmp (local.get $rt) (global.get $REG_R0))  ;; wait, we need temp zero
    (call $emit_cmpi (local.get $rt) (i32.const 0))  ;; CMP Rt, #0
    (call $emit_b_cond (local.get $off) (i32.const 0))  ;; BEQ
  )
  (func $emit_cbnz_x (param $rt i32) (param $off i32)
    (call $emit_cmpi (local.get $rt) (i32.const 0))  ;; CMP Rt, #0
    (call $emit_b_cond (local.get $off) (i32.const 1))  ;; BNE
  )

  ;; ── Missing aliases for template compatibility ────────────────────

  ;; 64-bit CLZ/RBIT (same as 32-bit on ARM32)
  (func $emit_clz_64
    (call $emit_clz_32)
  )
  (func $emit_rbit_64
    (call $emit_rbit_32)
  )

  ;; Raw instruction helpers called directly from templates
  (func $emit_instr_orr_32 (param $rd i32) (param $rn i32) (param $rm i32)
    (call $emit_instr_orr (local.get $rd) (local.get $rn) (local.get $rm))
  )
  (func $emit_instr_and_32 (param $rd i32) (param $rn i32) (param $rm i32)
    (call $emit_instr_and (local.get $rd) (local.get $rn) (local.get $rm))
  )
  (func $emit_instr_eor_32 (param $rd i32) (param $rn i32) (param $rm i32)
    (call $emit_instr_eor (local.get $rd) (local.get $rn) (local.get $rm))
  )
  (func $emit_instr_add_32 (param $rd i32) (param $rn i32) (param $rm i32)
    (call $emit_instr_add (local.get $rd) (local.get $rn) (local.get $rm))
  )
  (func $emit_instr_sub_32 (param $rd i32) (param $rn i32) (param $rm i32)
    (call $emit_instr_sub (local.get $rd) (local.get $rn) (local.get $rm))
  )
  (func $emit_instr_mul_32 (param $rd i32) (param $rn i32) (param $rm i32)
    (call $emit_instr_mul (local.get $rd) (local.get $rn) (local.get $rm))
  )
  (func $emit_instr_sdiv_32 (param $rd i32) (param $rn i32) (param $rm i32)
    (call $emit_instr_sdiv (local.get $rd) (local.get $rn) (local.get $rm))
  )
  (func $emit_instr_udiv_32 (param $rd i32) (param $rn i32) (param $rm i32)
    (call $emit_instr_udiv (local.get $rd) (local.get $rn) (local.get $rm))
  )

  ;; ═════════════════════════════════════════════════════════════════════
  ;; ARM32 NEON (SIMD) emit helpers — v128 load/store/push/pop
  ;; ═════════════════════════════════════════════════════════════════════

  ;; Push Q0 onto WASM value stack: VSTMDB SP!, {D0-D1}
  ;; Stores D0 and D1 (16 bytes = 128 bits), decrements SP by 16.
  ;; Encoding: cond=1110, 110, P=1, U=0, D=0, W=1, L=0, Rn=SP=13, Vd=0, 1011, imm8=2
  (func $emit_v128_push
    (call $emit_instr (i32.const 0xED2D0B02))
  )

  ;; Pop Q0 from WASM value stack: VLDMIA SP!, {D0-D1}
  ;; Loads D0 and D1 (16 bytes = 128 bits), increments SP by 16.
  ;; Encoding: cond=1110, 110, P=0, U=1, D=0, W=1, L=1, Rn=SP=13, Vd=0, 1011, imm8=2
  (func $emit_v128_pop
    (call $emit_instr (i32.const 0xECBD0B02))
  )

  ;; Load Q0 from address in R1: VLDMIA R1, {D0-D1}
  ;; Loads 16 bytes from [R1] into D0,D1.
  ;; Encoding: cond=1110, 110, P=0, U=1, D=0, W=0, L=1, Rn=1, Vd=0, 1011, imm8=2
  (func $emit_v128_load_reg
    (call $emit_instr (i32.const 0xECB10B02))
  )

  ;; Store Q0 to address in R1: VSTMIA R1, {D0-D1}
  ;; Stores 16 bytes from D0,D1 into [R1].
  ;; Encoding: cond=1110, 110, P=0, U=1, D=0, W=0, L=0, Rn=1, Vd=0, 1011, imm8=2
  (func $emit_v128_store_reg
    (call $emit_instr (i32.const 0xECA10B02))
  )

  ;; Load 32-bit constant address into R1 (MOVW/MOVT)
  (func $emit_load_addr_r1 (param $addr i32)
    (call $emit_instr_movz_32 (i32.const 1) (i32.const 0) (i32.and (local.get $addr) (i32.const 0xFFFF)))
    (call $emit_instr_movk_64 (i32.const 1) (i32.const 1) (i32.shr_u (local.get $addr) (i32.const 16)))
  )

  ;; ── Pop two, operate, push (pattern for binary ops) ──────────────

  ;; Push R0 only if RESULT_IN_X0 is 0 (peephole)
  (func $emit_maybe_push_x0
    (local $next_op i32)
    (local $next_imm0 i32)
    (if (i32.eqz (global.get $RESULT_IN_X0))
      (then
        (local.set $next_op (i32.load8_u (i32.add (global.get $CURRENT_DEC_PTR) (global.get $DEC_SZ))))
        (local.set $next_imm0 (i32.load (i32.add (i32.add (global.get $CURRENT_DEC_PTR) (global.get $DEC_SZ)) (i32.const 4))))
        (if (i32.and (i32.or (i32.eq (local.get $next_op) (i32.const 0x21)) (i32.eq (local.get $next_op) (i32.const 0x22)))
                      (i32.le_u (local.get $next_imm0) (i32.const 1)))
          (then
            (global.set $RESULT_IN_X0 (i32.const 1))
          )
          (else
            (call $emit_push_r0)
          )
        )
      )
    )
  )
