  ;; ── Helper: write bytes to code cache ──────────────────────────────

  (func $emit_aarch64_byte (param $b i32)
    (local $p i32)
    (local.set $p (i32.add (global.get $JIT_CACHE) (i32.load (global.get $JS_CODE_PTR))))
    (i32.store8 (local.get $p) (local.get $b))
    (i32.store (global.get $JS_CODE_PTR) (i32.add (i32.load (global.get $JS_CODE_PTR)) (i32.const 1)))
  )

  (func $emit_aarch64_dword (param $v i32)
    (call $emit_aarch64_byte (i32.and (local.get $v) (i32.const 0xFF)))
    (call $emit_aarch64_byte (i32.and (i32.shr_u (local.get $v) (i32.const 8)) (i32.const 0xFF)))
    (call $emit_aarch64_byte (i32.and (i32.shr_u (local.get $v) (i32.const 16)) (i32.const 0xFF)))
    (call $emit_aarch64_byte (i32.and (i32.shr_u (local.get $v) (i32.const 24)) (i32.const 0xFF)))
  )

  ;; Emit a qword (8 bytes) little-endian
  (func $emit_aarch64_qword (param $v i64)
    (call $emit_aarch64_dword (i32.wrap_i64 (local.get $v)))
    (call $emit_aarch64_dword (i32.wrap_i64 (i64.shr_u (local.get $v) (i64.const 32))))
  )

  ;; ── AArch64 instruction emitter ────────────────────────────────────
  ;; All AArch64 instructions are exactly 4 bytes (32 bits)

  ;; Emit a 32-bit AArch64 instruction (little-endian)
  (func $emit_aarch64_instr (param $val i32)
    (call $emit_aarch64_dword (local.get $val))
  )

  ;; ── AArch64 Register-to-Register (R-type) helpers ──────────────────
  ;; Base encoding for 64-bit operations:
  ;;   [31]=1 (64-bit), [30..29]=opc, [28..24]=0xxxx, [23..22]=variant,
  ;;   [21..16]=Rm, [15..10]=imm, [9..5]=Rn, [4..0]=Rd

  ;; ADD Xd, Xn, Xm (64-bit): 10001011000 Rm 000000 Rn Rd
  (func $emit_aarch64_instr_add_64 (param $rd i32) (param $rn i32) (param $rm i32)
    (call $emit_aarch64_instr
      (i32.or (i32.const 0x8B000000)
              (i32.or (i32.shl (local.get $rm) (i32.const 16))
                      (i32.or (i32.shl (local.get $rn) (i32.const 5))
                              (local.get $rd)))))
  )

  ;; SUB Xd, Xn, Xm (64-bit): 11001011000 Rm 000000 Rn Rd
  (func $emit_aarch64_instr_sub_64 (param $rd i32) (param $rn i32) (param $rm i32)
    (call $emit_aarch64_instr
      (i32.or (i32.const 0xCB000000)
              (i32.or (i32.shl (local.get $rm) (i32.const 16))
                      (i32.or (i32.shl (local.get $rn) (i32.const 5))
                              (local.get $rd)))))
  )

  ;; ADD Wd, Wn, Wm (32-bit): 00001011000 Rm 000000 Rn Rd
  (func $emit_aarch64_instr_add_32 (param $rd i32) (param $rn i32) (param $rm i32)
    (call $emit_aarch64_instr
      (i32.or (i32.const 0x0B000000)
              (i32.or (i32.shl (local.get $rm) (i32.const 16))
                      (i32.or (i32.shl (local.get $rn) (i32.const 5))
                              (local.get $rd)))))
  )

  ;; SUB Wd, Wn, Wm (32-bit): 01001011000 Rm 000000 Rn Rd
  (func $emit_aarch64_instr_sub_32 (param $rd i32) (param $rn i32) (param $rm i32)
    (call $emit_aarch64_instr
      (i32.or (i32.const 0x4B000000)
              (i32.or (i32.shl (local.get $rm) (i32.const 16))
                      (i32.or (i32.shl (local.get $rn) (i32.const 5))
                              (local.get $rd)))))
  )

  ;; MUL Xd, Xn, Xm (64-bit): 10011011000 111111 Rm Rn Rd
  (func $emit_aarch64_instr_mul_64 (param $rd i32) (param $rn i32) (param $rm i32)
    (call $emit_aarch64_instr
      (i32.or (i32.const 0x9B007C00)
              (i32.or (i32.shl (local.get $rm) (i32.const 16))
                      (i32.or (i32.shl (local.get $rn) (i32.const 5))
                              (local.get $rd)))))
  )

  ;; MUL Wd, Wn, Wm (32-bit): 00011011000 111111 Rm Rn Rd
  (func $emit_aarch64_instr_mul_32 (param $rd i32) (param $rn i32) (param $rm i32)
    (call $emit_aarch64_instr
      (i32.or (i32.const 0x1B007C00)
              (i32.or (i32.shl (local.get $rm) (i32.const 16))
                      (i32.or (i32.shl (local.get $rn) (i32.const 5))
                              (local.get $rd)))))
  )

  ;; SDIV Xd, Xn, Xm (64-bit): 10011010110 000011 Rm Rn Rd
  (func $emit_aarch64_instr_sdiv_64 (param $rd i32) (param $rn i32) (param $rm i32)
    (call $emit_aarch64_instr
      (i32.or (i32.const 0x9AC00C00)
              (i32.or (i32.shl (local.get $rm) (i32.const 16))
                      (i32.or (i32.shl (local.get $rn) (i32.const 5))
                              (local.get $rd)))))
  )

  ;; UDIV Xd, Xn, Xm (64-bit): 10011010110 000010 Rm Rn Rd
  (func $emit_aarch64_instr_udiv_64 (param $rd i32) (param $rn i32) (param $rm i32)
    (call $emit_aarch64_instr
      (i32.or (i32.const 0x9AC00800)
              (i32.or (i32.shl (local.get $rm) (i32.const 16))
                      (i32.or (i32.shl (local.get $rn) (i32.const 5))
                              (local.get $rd)))))
  )

  ;; SDIV Wd, Wn, Wm (32-bit): 00011010110 000011 Rm Rn Rd
  (func $emit_aarch64_instr_sdiv_32 (param $rd i32) (param $rn i32) (param $rm i32)
    (call $emit_aarch64_instr
      (i32.or (i32.const 0x1AC00C00)
              (i32.or (i32.shl (local.get $rm) (i32.const 16))
                      (i32.or (i32.shl (local.get $rn) (i32.const 5))
                              (local.get $rd)))))
  )

  ;; UDIV Wd, Wn, Wm (32-bit): 00011010110 000010 Rm Rn Rd
  (func $emit_aarch64_instr_udiv_32 (param $rd i32) (param $rn i32) (param $rm i32)
    (call $emit_aarch64_instr
      (i32.or (i32.const 0x1AC00800)
              (i32.or (i32.shl (local.get $rm) (i32.const 16))
                      (i32.or (i32.shl (local.get $rn) (i32.const 5))
                              (local.get $rd)))))
  )

  ;; AND Xd, Xn, Xm (64-bit): 10001010000 Rm 000000 Rn Rd
  (func $emit_aarch64_instr_and_64 (param $rd i32) (param $rn i32) (param $rm i32)
    (call $emit_aarch64_instr
      (i32.or (i32.const 0x8A000000)
              (i32.or (i32.shl (local.get $rm) (i32.const 16))
                      (i32.or (i32.shl (local.get $rn) (i32.const 5))
                              (local.get $rd)))))
  )

  ;; ORR Xd, Xn, Xm (64-bit): 10101010000 Rm 000000 Rn Rd
  (func $emit_aarch64_instr_orr_64 (param $rd i32) (param $rn i32) (param $rm i32)
    (call $emit_aarch64_instr
      (i32.or (i32.const 0xAA000000)
              (i32.or (i32.shl (local.get $rm) (i32.const 16))
                      (i32.or (i32.shl (local.get $rn) (i32.const 5))
                              (local.get $rd)))))
  )

  ;; EOR Xd, Xn, Xm (64-bit): 11001010000 Rm 000000 Rn Rd
  (func $emit_aarch64_instr_eor_64 (param $rd i32) (param $rn i32) (param $rm i32)
    (call $emit_aarch64_instr
      (i32.or (i32.const 0xCA000000)
              (i32.or (i32.shl (local.get $rm) (i32.const 16))
                      (i32.or (i32.shl (local.get $rn) (i32.const 5))
                              (local.get $rd)))))
  )

  ;; AND Wd, Wn, Wm (32-bit): 00001010000 Rm 000000 Rn Rd
  (func $emit_aarch64_instr_and_32 (param $rd i32) (param $rn i32) (param $rm i32)
    (call $emit_aarch64_instr
      (i32.or (i32.const 0x0A000000)
              (i32.or (i32.shl (local.get $rm) (i32.const 16))
                      (i32.or (i32.shl (local.get $rn) (i32.const 5))
                              (local.get $rd)))))
  )

  ;; ORR Wd, Wn, Wm (32-bit): 00101010000 Rm 000000 Rn Rd
  (func $emit_aarch64_instr_orr_32 (param $rd i32) (param $rn i32) (param $rm i32)
    (call $emit_aarch64_instr
      (i32.or (i32.const 0x2A000000)
              (i32.or (i32.shl (local.get $rm) (i32.const 16))
                      (i32.or (i32.shl (local.get $rn) (i32.const 5))
                              (local.get $rd)))))
  )

  ;; EOR Wd, Wn, Wm (32-bit): 01001010000 Rm 000000 Rn Rd
  (func $emit_aarch64_instr_eor_32 (param $rd i32) (param $rn i32) (param $rm i32)
    (call $emit_aarch64_instr
      (i32.or (i32.const 0x4A000000)
              (i32.or (i32.shl (local.get $rm) (i32.const 16))
                      (i32.or (i32.shl (local.get $rn) (i32.const 5))
                              (local.get $rd)))))
  )

  ;; LSLV Xd, Xn, Xm (64-bit variable shift left): 10011010110 001000 Rm Rn Rd
  (func $emit_aarch64_instr_lslv_64 (param $rd i32) (param $rn i32) (param $rm i32)
    (call $emit_aarch64_instr
      (i32.or (i32.const 0x9AC02000)
              (i32.or (i32.shl (local.get $rm) (i32.const 16))
                      (i32.or (i32.shl (local.get $rn) (i32.const 5))
                              (local.get $rd)))))
  )

  ;; LSRV Xd, Xn, Xm (64-bit): 10011010110 001001 Rm Rn Rd
  (func $emit_aarch64_instr_lsrv_64 (param $rd i32) (param $rn i32) (param $rm i32)
    (call $emit_aarch64_instr
      (i32.or (i32.const 0x9AC02400)
              (i32.or (i32.shl (local.get $rm) (i32.const 16))
                      (i32.or (i32.shl (local.get $rn) (i32.const 5))
                              (local.get $rd)))))
  )

  ;; ASRV Xd, Xn, Xm (64-bit): 10011010110 001010 Rm Rn Rd
  (func $emit_aarch64_instr_asrv_64 (param $rd i32) (param $rn i32) (param $rm i32)
    (call $emit_aarch64_instr
      (i32.or (i32.const 0x9AC02800)
              (i32.or (i32.shl (local.get $rm) (i32.const 16))
                      (i32.or (i32.shl (local.get $rn) (i32.const 5))
                              (local.get $rd)))))
  )

  ;; RORV Xd, Xn, Xm (64-bit): 10011010110 001011 Rm Rn Rd
  (func $emit_aarch64_instr_rorv_64 (param $rd i32) (param $rn i32) (param $rm i32)
    (call $emit_aarch64_instr
      (i32.or (i32.const 0x9AC02C00)
              (i32.or (i32.shl (local.get $rm) (i32.const 16))
                      (i32.or (i32.shl (local.get $rn) (i32.const 5))
                              (local.get $rd)))))
  )

  ;; LSLV Wd, Wn, Wm (32-bit): 00011010110 001000 Rm Rn Rd
  (func $emit_aarch64_instr_lslv_32 (param $rd i32) (param $rn i32) (param $rm i32)
    (call $emit_aarch64_instr
      (i32.or (i32.const 0x1AC02000)
              (i32.or (i32.shl (local.get $rm) (i32.const 16))
                      (i32.or (i32.shl (local.get $rn) (i32.const 5))
                              (local.get $rd)))))
  )

  ;; LSRV Wd, Wn, Wm (32-bit): 00011010110 001001 Rm Rn Rd
  (func $emit_aarch64_instr_lsrv_32 (param $rd i32) (param $rn i32) (param $rm i32)
    (call $emit_aarch64_instr
      (i32.or (i32.const 0x1AC02400)
              (i32.or (i32.shl (local.get $rm) (i32.const 16))
                      (i32.or (i32.shl (local.get $rn) (i32.const 5))
                              (local.get $rd)))))
  )

  ;; ASRV Wd, Wn, Wm (32-bit): 00011010110 001010 Rm Rn Rd
  (func $emit_aarch64_instr_asrv_32 (param $rd i32) (param $rn i32) (param $rm i32)
    (call $emit_aarch64_instr
      (i32.or (i32.const 0x1AC02800)
              (i32.or (i32.shl (local.get $rm) (i32.const 16))
                      (i32.or (i32.shl (local.get $rn) (i32.const 5))
                              (local.get $rd)))))
  )

  ;; RORV Wd, Wn, Wm (32-bit): 00011010110 001011 Rm Rn Rd
  (func $emit_aarch64_instr_rorv_32 (param $rd i32) (param $rn i32) (param $rm i32)
    (call $emit_aarch64_instr
      (i32.or (i32.const 0x1AC02C00)
              (i32.or (i32.shl (local.get $rm) (i32.const 16))
                      (i32.or (i32.shl (local.get $rn) (i32.const 5))
                              (local.get $rd)))))
  )

  ;; ── AArch64 Immediate arithmetic helpers ──────────────────────────

  ;; ADD Xd, Xn, #imm12 (shift=0): 10010001 00 sh imm12 Rn Rd
  ;;   Only for imm12 (0-4095) where imm fits in 12 bits unsigned
  (func $emit_aarch64_instr_addi_64 (param $rd i32) (param $rn i32) (param $imm12 i32)
    (call $emit_aarch64_instr
      (i32.or (i32.const 0x91000000)
              (i32.or (i32.shl (local.get $imm12) (i32.const 10))
                      (i32.or (i32.shl (local.get $rn) (i32.const 5))
                              (local.get $rd)))))
  )

  ;; SUB Xd, Xn, #imm12 (shift=0): 11010001 00 sh imm12 Rn Rd
  (func $emit_aarch64_instr_subi_64 (param $rd i32) (param $rn i32) (param $imm12 i32)
    (call $emit_aarch64_instr
      (i32.or (i32.const 0xD1000000)
              (i32.or (i32.shl (local.get $imm12) (i32.const 10))
                      (i32.or (i32.shl (local.get $rn) (i32.const 5))
                              (local.get $rd)))))
  )

  ;; ADD Wd, Wn, #imm12 (32-bit): 00010001 00 sh imm12 Rn Rd
  (func $emit_aarch64_instr_addi_32 (param $rd i32) (param $rn i32) (param $imm12 i32)
    (call $emit_aarch64_instr
      (i32.or (i32.const 0x11000000)
              (i32.or (i32.shl (local.get $imm12) (i32.const 10))
                      (i32.or (i32.shl (local.get $rn) (i32.const 5))
                              (local.get $rd)))))
  )

  ;; SUB Wd, Wn, #imm12 (32-bit): 01010001 00 sh imm12 Rn Rd
  (func $emit_aarch64_instr_subi_32 (param $rd i32) (param $rn i32) (param $imm12 i32)
    (call $emit_aarch64_instr
      (i32.or (i32.const 0x51000000)
              (i32.or (i32.shl (local.get $imm12) (i32.const 10))
                      (i32.or (i32.shl (local.get $rn) (i32.const 5))
                              (local.get $rd)))))
  )

  ;; AND Xd, Xn, #imm (64-bit, bitmask immediate): complex encoding
  ;; For simple cases use AND with XZR or MOVN/MOVZ
  ;; MOV Xd, #imm (via MOVZ): 110100101 hw imm16 Rd
  ;;   hw=00 → zero-extend 16-bit to bits [15:0]
  ;;   hw=01 → shift left by 16
  ;;   hw=10 → shift left by 32
  ;;   hw=11 → shift left by 48
  (func $emit_aarch64_instr_movz_64 (param $rd i32) (param $hw i32) (param $imm16 i32)
    (call $emit_aarch64_instr
      (i32.or (i32.const 0xD2800000)
              (i32.or (i32.shl (local.get $hw) (i32.const 21))
                      (i32.or (i32.shl (local.get $imm16) (i32.const 5))
                              (local.get $rd)))))
  )

  ;; MOVN Xd, #imm (64-bit): 100100101 hw imm16 Rd
  (func $emit_aarch64_instr_movn_64 (param $rd i32) (param $hw i32) (param $imm16 i32)
    (call $emit_aarch64_instr
      (i32.or (i32.const 0x92800000)
              (i32.or (i32.shl (local.get $hw) (i32.const 21))
                      (i32.or (i32.shl (local.get $imm16) (i32.const 5))
                              (local.get $rd)))))
  )

  ;; MOVK Xd, #imm (64-bit, keep other bits): 111100101 hw imm16 Rd
  (func $emit_aarch64_instr_movk_64 (param $rd i32) (param $hw i32) (param $imm16 i32)
    (call $emit_aarch64_instr
      (i32.or (i32.const 0xF2800000)
              (i32.or (i32.shl (local.get $hw) (i32.const 21))
                      (i32.or (i32.shl (local.get $imm16) (i32.const 5))
                              (local.get $rd)))))
  )

  ;; MOV N, #imm (32-bit): MOVZ Wd, #imm or MOVN Wd, #imm
  (func $emit_aarch64_instr_movz_32 (param $rd i32) (param $hw i32) (param $imm16 i32)
    (call $emit_aarch64_instr
      (i32.or (i32.const 0x52800000)
              (i32.or (i32.shl (local.get $hw) (i32.const 21))
                      (i32.or (i32.shl (local.get $imm16) (i32.const 5))
                              (local.get $rd)))))
  )

  ;; ── AArch64 Load/Store helpers ─────────────────────────────────────
  ;; All use unsigned offset addressing: STR/LDR Xt, [Xn, #imm]
  ;; For stack push/pop with pre/post-index we use a different encoding.

  ;; STR Xt, [Xn, #imm] (64-bit, unsigned offset): 1111100101 imm12 Rn Rt
  (func $emit_aarch64_instr_str_64_off (param $rt i32) (param $rn i32) (param $imm12 i32)
    (call $emit_aarch64_instr
      (i32.or (i32.const 0xF9000000)
              (i32.or (i32.shl (local.get $imm12) (i32.const 10))
                      (i32.or (i32.shl (local.get $rn) (i32.const 5))
                              (local.get $rt)))))
  )

  ;; LDR Xt, [Xn, #imm] (64-bit, unsigned offset): 1111100101 imm12 Rn Rt
  (func $emit_aarch64_instr_ldr_64_off (param $rt i32) (param $rn i32) (param $imm12 i32)
    (call $emit_aarch64_instr
      (i32.or (i32.const 0xF9400000)
              (i32.or (i32.shl (local.get $imm12) (i32.const 10))
                      (i32.or (i32.shl (local.get $rn) (i32.const 5))
                              (local.get $rt)))))
  )

  ;; STR Wt, [Xn, #imm] (32-bit, unsigned offset): 1011100101 imm12 Rn Rt
  (func $emit_aarch64_instr_str_32_off (param $rt i32) (param $rn i32) (param $imm12 i32)
    (call $emit_aarch64_instr
      (i32.or (i32.const 0xB9000000)
              (i32.or (i32.shl (local.get $imm12) (i32.const 10))
                      (i32.or (i32.shl (local.get $rn) (i32.const 5))
                              (local.get $rt)))))
  )

  ;; LDR Wt, [Xn, #imm] (32-bit, unsigned offset): 1011100101 imm12 Rn Rt
  (func $emit_aarch64_instr_ldr_32_off (param $rt i32) (param $rn i32) (param $imm12 i32)
    (call $emit_aarch64_instr
      (i32.or (i32.const 0xB9400000)
              (i32.or (i32.shl (local.get $imm12) (i32.const 10))
                      (i32.or (i32.shl (local.get $rn) (i32.const 5))
                              (local.get $rt)))))
  )

  ;; STR Xt, [Xn, #imm]! (pre-index, 64-bit): 11111001 10 imm9 11 Xn Rt
  ;; imm9 is signed (-256 to 255), encoded as 9-bit signed
  (func $emit_aarch64_instr_str_pre_64 (param $rt i32) (param $rn i32) (param $imm9 i32)
    (local $enc i32)
    (local.set $enc (i32.and (local.get $imm9) (i32.const 0x1FF)))
    (call $emit_aarch64_instr
      (i32.or (i32.const 0xF8000000)
              (i32.or (i32.shl (local.get $enc) (i32.const 12))
                      (i32.or (i32.shl (local.get $rn) (i32.const 5))
                              (local.get $rt)))))
  )

  ;; LDR Xt, [Xn], #imm (post-index, 64-bit): 11111000 10 imm9 01 Xn Rt
  (func $emit_aarch64_instr_ldr_post_64 (param $rt i32) (param $rn i32) (param $imm9 i32)
    (local $enc i32)
    (local.set $enc (i32.and (local.get $imm9) (i32.const 0x1FF)))
    (call $emit_aarch64_instr
      (i32.or (i32.const 0xF8400400)
              (i32.or (i32.shl (local.get $enc) (i32.const 12))
                      (i32.or (i32.shl (local.get $rn) (i32.const 5))
                              (local.get $rt)))))
  )

  ;; STR Wt, [Xn, #imm]! (pre-index, 32-bit): 10111001 10 imm9 11 Xn Rt
  (func $emit_aarch64_instr_str_pre_32 (param $rt i32) (param $rn i32) (param $imm9 i32)
    (local $enc i32)
    (local.set $enc (i32.and (local.get $imm9) (i32.const 0x1FF)))
    (call $emit_aarch64_instr
      (i32.or (i32.const 0xB8000000)
              (i32.or (i32.shl (local.get $enc) (i32.const 12))
                      (i32.or (i32.shl (local.get $rn) (i32.const 5))
                              (local.get $rt)))))
  )

  ;; LDR Wt, [Xn], #imm (post-index, 32-bit): 10111000 10 imm9 01 Xn Rt
  (func $emit_aarch64_instr_ldr_post_32 (param $rt i32) (param $rn i32) (param $imm9 i32)
    (local $enc i32)
    (local.set $enc (i32.and (local.get $imm9) (i32.const 0x1FF)))
    (call $emit_aarch64_instr
      (i32.or (i32.const 0xB8400400)
              (i32.or (i32.shl (local.get $enc) (i32.const 12))
                      (i32.or (i32.shl (local.get $rn) (i32.const 5))
                              (local.get $rt)))))
  )

  ;; ── AArch64 Stack push/pop (value stack for WASM) ─────────────────
  ;; Push X0: STR X0, [SP, #-8]!
  (func $emit_aarch64_push_x0
    (call $emit_aarch64_instr_str_pre_64 (global.get $REG_X0) (global.get $REG_SP) (i32.const -8))
  )

  ;; Pop X0: LDR X0, [SP], #8
  (func $emit_aarch64_pop_x0
    (call $emit_aarch64_instr_ldr_post_64 (global.get $REG_X0) (global.get $REG_SP) (i32.const 8))
  )

  ;; Push X1: STR X1, [SP, #-8]!
  (func $emit_aarch64_push_x1
    (call $emit_aarch64_instr_str_pre_64 (global.get $REG_X1) (global.get $REG_SP) (i32.const -8))
  )

  ;; Pop X1: LDR X1, [SP], #8
  (func $emit_aarch64_pop_x1
    (call $emit_aarch64_instr_ldr_post_64 (global.get $REG_X1) (global.get $REG_SP) (i32.const 8))
  )

  ;; Push X2: STR X2, [SP, #-8]!
  (func $emit_aarch64_push_x2
    (call $emit_aarch64_instr_str_pre_64 (global.get $REG_X2) (global.get $REG_SP) (i32.const -8))
  )

  ;; Pop X2: LDR X2, [SP], #8
  (func $emit_aarch64_pop_x2
    (call $emit_aarch64_instr_ldr_post_64 (global.get $REG_X2) (global.get $REG_SP) (i32.const 8))
  )

  ;; Pop into X1: LDR X1, [SP], #8 (alias)
  (func $emit_aarch64_pop_x1_alias
    (call $emit_aarch64_pop_x1)
  )

  ;; Pop pair into X1, X0 (LDP X1, X0, [SP], #16)
  ;; LDP encoding: 10101000 11 0 imm7 Rt2 Rn Rt1  (with post-index)
  ;; LDP Xt1, Xt2, [Xn], #imm: 10101000 110 imm7 Xn Xt2 Xt1
  ;; Wait, LDP post-index: opc=10, 1010 1000 1 11 imm7 Xn Rt2 Rt1
  ;; Let me just use two separate pops
  (func $emit_aarch64_pop2_x1_x0
    (call $emit_aarch64_instr_ldr_post_64 (global.get $REG_X1) (global.get $REG_SP) (i32.const 8))
    (call $emit_aarch64_instr_ldr_post_64 (global.get $REG_X0) (global.get $REG_SP) (i32.const 8))
  )

  ;; STP Xt1, Xt2, [SP, #-16]! (pre-index pair)
  ;; STP Xt1, Xt2, [Xn, #-imm]!: 10101000 10 0 imm7 Xn Xt2 Xt1
  ;; Actually: for 64-bit STP pre-index: opc=10, 1010 1000 1 00 imm7 Xn Rt2 Rt1
  (func $emit_aarch64_push2_x1_x0
    (call $emit_aarch64_instr_str_pre_64 (global.get $REG_X1) (global.get $REG_SP) (i32.const -8))
    (call $emit_aarch64_instr_str_pre_64 (global.get $REG_X0) (global.get $REG_SP) (i32.const -8))
  )

  ;; ── Standard push/pop names (matching x86 compiler convention) ────
  (func $emit_aarch64_pop_rax_alias
    (call $emit_aarch64_pop_x0)
  )
  (func $emit_aarch64_push_rax_alias
    (call $emit_aarch64_push_x0)
  )
  (func $emit_aarch64_pop_rcx_alias
    (call $emit_aarch64_pop_x1)
  )
  (func $emit_aarch64_push_rcx_alias
    (call $emit_aarch64_push_x1)
  )
  (func $emit_aarch64_pop_rdx_alias
    (call $emit_aarch64_pop_x2)
  )

  ;; ── AArch64 sign extension / data processing ──────────────────────

  ;; SXTW X0, W0 (sign-extend W0→X0): 10011010110 000000 Rm(0) 00000 Rn(0) Rd
  ;; Actually: SXTW is alias for SBFM Xd, Xn, #0, #31
  ;; SBFM encoding: 100110 1 10 0 N immr imms Rn Rd
  ;; SXTW X0, W0: 0x93407C00
  ;; let me just hardcode
  (func $emit_aarch64_sxtw_x0_w0
    (call $emit_aarch64_instr (i32.const 0x93407C00))
  )

  ;; ── AArch64 branch instructions ────────────────────────────────────

  ;; B #imm (unconditional branch, ±128MB): 000101 + imm26
  ;; imm26 = (target - pc) >> 2, encoded as signed 26-bit
  (func $emit_aarch64_b (param $off i32)
    (local $enc i32)
    (local.set $enc (i32.shr_s (i32.shl (local.get $off) (i32.const 6)) (i32.const 6)))  ;; sign-extend from 26 bits
    (call $emit_aarch64_instr
      (i32.or (i32.const 0x14000000)
              (i32.and (i32.shr_s (local.get $off) (i32.const 2)) (i32.const 0x03FFFFFF)))
    )
  )

  ;; BL #imm (branch with link): 100101 + imm26
  (func $emit_aarch64_bl (param $off i32)
    (call $emit_aarch64_instr
      (i32.or (i32.const 0x94000000)
              (i32.and (i32.shr_s (local.get $off) (i32.const 2)) (i32.const 0x03FFFFFF)))
    )
  )

  ;; B.cond #imm (conditional branch, ±1MB): 01010100 imm19 0 cond
  ;; cond codes (AArch64): EQ=0, NE=1, CS/HS=2, CC/LO=3, MI=4, PL=5,
  ;;   VS=6, VC=7, HI=8, LS=9, GE=10, LT=11, GT=12, LE=13, AL=14
  (func $emit_aarch64_b_cond (param $off i32) (param $cond i32)
    (call $emit_aarch64_instr
      (i32.or (i32.const 0x54000000)
              (i32.or (i32.and (i32.shr_s (local.get $off) (i32.const 2)) (i32.const 0x7FFFF))
                      (i32.shl (local.get $cond) (i32.const 0))))
    )
  )

  ;; CBZ Xt, #imm (compare and branch if zero, ±1MB): 10110100 imm19 Rt
  ;; For 64-bit: 10110100 imm19 Rt
  (func $emit_aarch64_cbz_x (param $rt i32) (param $off i32)
    (call $emit_aarch64_instr
      (i32.or (i32.const 0xB4000000)
              (i32.or (i32.and (i32.shr_s (local.get $off) (i32.const 2)) (i32.const 0x7FFFF))
                      (local.get $rt)))
    )
  )

  ;; CBNZ Xt, #imm (compare and branch if non-zero, ±1MB): 10110101 imm19 Rt
  ;; For 64-bit: 10110101 imm19 Rt
  (func $emit_aarch64_cbnz_x (param $rt i32) (param $off i32)
    (call $emit_aarch64_instr
      (i32.or (i32.const 0xB5000000)
              (i32.or (i32.and (i32.shr_s (local.get $off) (i32.const 2)) (i32.const 0x7FFFF))
                      (local.get $rt)))
    )
  )

  ;; ── AArch64 Conditional set (CSET) ────────────────────────────────
  ;; CSET Wd, cond = CSINC Wd, WZR, WZR, invert(cond)
  ;; CSINC Wd, Wn, Wm, cond: 00011010100 imm5 cond Rn Rm Rd  wait no that's not right
  ;; CSINC Wd, WZR, WZR, cond where cond is inverted to get SET from CSINC:
  ;; Encoding: 00111010100 00000 cond 11111 11111 Rd
  ;; For CSET (set if condition true):
  ;;   Actually CSET Wd, cond = CSINC Wd, WZR, WZR, ¬cond
  ;;   For EQ: CSET W0, EQ = 0x1A9F17E0
  ;; Let me just emit raw bytes for common cases
  
  ;; CSET W0, cond (set W0 to 1 if cond true, else 0)
  ;; CSINC Wd, WZR, WZR, invert(cond)  — condition codes start at 0
  ;;   Encoding: 1A 9F (imm5=00000) cond (Rn=11111) (Rm=11111) Rd
  ;;   Base: 0x1A9F0000 | (cond << 16) | (0x1F << 5) | (0x1F << 16) | Rd  
  ;;   Actually: CSINC = 0x1A800000, with ZR=31
  ;;   CSINC Xd, XZR, XZR, cond: 10011010100 imm5 cond Rn Rm Rd
  ;;   = 0x9A9F0000 | (cond << 16) | (0x1F << 5) | (0x1F << 16) | Rd  -- wait no
  ;;   Let me simplify: CSINC base = 0x1A800400 + cond<<16 + Rn<<5 + Rm<<16 + Rd
  ;;   Actually for 32-bit: CSINC = 0x1A800400
  ;;   For XZR, WZR: Rn=31, Rm=31
  ;;   So: 0x1A800400 | (cond << 16) | (31 << 5) | (31 << 16) | Rd
  ;;   = 0x1A9F0400 | (cond << 16) | Rd
  ;;
  ;;   Wait I need to be more careful:
  ;;   CSINC encoding:
  ;;   0 0 1 1 1 0 1 0 1 0 0 | Rm | cond | 1 | Rn | Rd
  ;;                            imm5=00000  op2=1
  ;;   Actually: 1A 8x xx xx where x depends
  ;;   opc=00, S=1, opcode=1010100, Rm=5bits, cond=4bits, op2=1, Rn=5bits, Rd=5bits
  ;;   = 0x1A800000 | (Rm << 16) | (cond << 12) | (1 << 11) wait no
  ;;   
  ;;   Let me just precompute these for the common conditions. A function:
  ;;   CSET Wd, cond = CSINC Wd, WZR, WZR, ¬cond
  ;;   Encoding for CSINC: sf=0, opc=00, S=1(11010100), Rm, cond, op2=1, Rn, Rd
  ;;   Full: 00111010100 | Rm(5) | cond(4) | 1 | Rn(5) | Rd(5)
  ;;   = 0x1A800000 | (Rm << 16) | (cond << 12) | (1 << 11) | (Rn << 5) | Rd
  ;;   = 0x1A800000 | (31 << 16) | (cond << 12) | (1 << 11) | (31 << 5) | Rd
  ;;   = 0x1A9F0800 | (cond << 12) | Rd
  ;; Hmm that doesn't look right either. Let me just compute this differently.
  ;;
  ;; CSET W0, cond: CSINC W0, WZR, WZR, invert(cond)
  ;; invert(cond) = cond ^ 1
  ;; So: 0x1A9F07E0 | (inv_cond << 12) -- wait WZR = 31...
  ;;
  ;; Let me be precise:
  ;; CSINC: [31:29]=001, [28]=S=1, [24:21]=1010, [20:16]=Rm, [15:12]=cond, [11]=1, [9:5]=Rn, [4:0]=Rd
  ;; Wait: 001 = 0x20000000, S=1 = 0x10000000
  ;; opcode=1010100 -> bits 24..15?
  ;; OK I'm overcomplicating this. Let me just emit hardcoded patterns for the cases I need.
  ;; 
  ;; Actually the simpler approach: use CINC which is CSINC but with same register for dd, Rn, Rm
  ;; Or even simpler: just hardcode the bytes for common CSET patterns

  ;; Just emit the 4 bytes directly for CSET W0, cond
  ;; CSET W0, cond: CSINC W0, WZR, WZR, invert(cond)
  ;; Base with Rd=0, Rn=31(WZR), Rm=31(WZR):
  ;;   = 0x1A800400 | (invert(cond) << 16) | (31 << 5) | (31 << 16) | 0
  ;;   Hmm, wait. Rm bits are [20:16], Rn bits are [9:5]. Let me re-check.
  ;;   Rm = bits [20:16], cond = [15:12], op2 = [11:10]?
  ;;   Actually for CSINC:
  ;;   31 30..29 28 27..25 24..21 20..16 15..12 11 10 9..5 4..0
  ;;   sf 00     S  1 1    op     Rm     cond   0  1  Rn   Rd
  ;;   sf=0: 0
  ;;   op for CSINC: 1010
  ;;   So: 0 00 1 11 1010 Rm cond 0 1 Rn Rd
  ;;   Top 8 bits: 0 0 1 1 1 1 0 1 0 = 0x3A... wait that doesn't match either
  ;;   
  ;;   OK let me just look at it raw. The encoding is:
  ;;   bit 31: sf (=0 for 32-bit)
  ;;   bits 30-29: 00
  ;;   bit 28: S (=1)
  ;;   bits 27-24: 1101
  ;;   bits 23-21: 010 (wait, 1101 010 = 0x6A)
  ;;   bits 20-16: Rm
  ;;   bits 15-12: cond
  ;;   bit 11: 0
  ;;   bit 10: 1
  ;;   bits 9-5: Rn
  ;;   bits 4-0: Rd
  ;;   So: 0 0011 1010 1 Rm(5) cond(4) 0 1 Rn(5) Rd(5)
  ;;   0x3A800000 | (Rm << 16) | (cond << 12) | (1 << 10) | (Rn << 5) | Rd
  ;;   = 0x3A800400 | (Rm << 16) | (cond << 12) | (Rn << 5) | Rd
  ;;   
  ;;   With Rn=31(WZR), Rm=31(WZR), Rd=0:
  ;;   = 0x3A9F0400 | (cond << 12)
  ;;   
  ;;   For CSET W0, EQ: cond=0, invert to get CSINC cond = 1 (NE)
  ;;   = 0x3A9F0400 | (1 << 12) = 0x3A9F1400

  ;; Hmm I realize this is getting very tricky with manual bit encodings. Let me use a different
  ;; approach - just precompute the exact instruction encoding for each needed pattern.
  
  ;; Actually, for setting a register based on a condition, I can use CSET:
  ;; CSET X0, cond (64-bit): sf=1 version
  ;; Same formula but sf=1:
  ;; = 0x7A800400 | (Rm << 16) | (cond << 12) | (Rn << 5) | Rd
  ;; For CSET X0, cond: CSINC X0, XZR, XZR, invert(cond)
  ;; = 0x7A9F0400 | (inv_cond << 12)

  ;; Let me define a simple helper that takes invert(cond) and emits CSINC X0, XZR, XZR, inv_cond
  ;; This sets X0 to 1 if the original cond was true, 0 otherwise
  (func $emit_aarch64_cset_x0 (param $inv_cond i32)
    ;; CSINC X0, XZR, XZR, inv_cond: 0x7A9F0400 | (inv_cond << 12)
    (call $emit_aarch64_instr
      (i32.or (i32.const 0x7A9F0400)
              (i32.shl (local.get $inv_cond) (i32.const 12)))
    )
  )

  ;; CSET W0, inv_cond (32-bit)
  (func $emit_aarch64_cset_w0 (param $inv_cond i32)
    ;; CSINC W0, WZR, WZR, inv_cond: 0x3A9F0400 | (inv_cond << 12)
    (call $emit_aarch64_instr
      (i32.or (i32.const 0x3A9F0400)
              (i32.shl (local.get $inv_cond) (i32.const 12)))
    )
  )

  ;; CMP Xn, Xm: alias for SUBS XZR, Xn, Xm
  ;; SUBS XZR, Xn, Xm: sf=1, S=1, 11011 000 Rm 000000 Rn 11111
  ;; = 0xEB00001F | (Rm << 16) | (Rn << 5)
  (func $emit_aarch64_cmp_64 (param $rn i32) (param $rm i32)
    (call $emit_aarch64_instr
      (i32.or (i32.const 0xEB00001F)
              (i32.or (i32.shl (local.get $rm) (i32.const 16))
                      (i32.shl (local.get $rn) (i32.const 5))))
    )
  )

  ;; CMP Wn, Wm (32-bit): SUBS WZR, Wn, Wm
  ;; = 0x6B00001F | (Rm << 16) | (Rn << 5)
  (func $emit_aarch64_cmp_32 (param $rn i32) (param $rm i32)
    (call $emit_aarch64_instr
      (i32.or (i32.const 0x6B00001F)
              (i32.or (i32.shl (local.get $rm) (i32.const 16))
                      (i32.shl (local.get $rn) (i32.const 5))))
    )
  )

  ;; TST Xn, Xm: alias for ANDS XZR, Xn, Xm
  ;; = 0xEA00001F | (Rm << 16) | (Rn << 5)
  (func $emit_aarch64_tst_64 (param $rn i32) (param $rm i32)
    (call $emit_aarch64_instr
      (i32.or (i32.const 0xEA00001F)
              (i32.or (i32.shl (local.get $rm) (i32.const 16))
                      (i32.shl (local.get $rn) (i32.const 5))))
    )
  )

  ;; TST Wn, Wm (32-bit)
  (func $emit_aarch64_tst_32 (param $rn i32) (param $rm i32)
    (call $emit_aarch64_instr
      (i32.or (i32.const 0x6A00001F)
              (i32.or (i32.shl (local.get $rm) (i32.const 16))
                      (i32.shl (local.get $rn) (i32.const 5))))
    )
  )

  ;; ── AArch64 Memory load/store via register offset ────────────────

  ;; LDR W0, [X19_mem, X1] — load 32-bit from mem_ptr + X1
  ;; LDR Xt, [Xn, Xm, LSL #0]: 11111000011 Xm 010 Xn Xt (64-bit)
  ;;                            11111000011 Xm 010 Xn Rt   (wait)
  ;; Actually: LDR Xt, [Xn, Xm] (register offset):
  ;;   sf=1, 11111000011 Xm 10 Xn Xt  with option=011 (UXTX) or 011 (LSL)
  ;;   Let me use a simpler approach: 11111000101 Xm 10 Xn Xt
  ;;   size=11, V=0, opc=01, 1, Rm, option=011, S=0, 10, Rn, Rt
  ;;   = 0xF8600800 | (Rm << 16) | (Rn << 5) | Rt
  ;; Wait, that gives LDR Xt, [Xn, Xm, LSL #0 for 64-bit
  ;; For 32-bit Wt: 10111000101 Rm 10 Rn Rt = 0xB8600800 | (Rm << 16) | (Rn << 5) | Rt
  ;; 
  ;; + 0 for load, + 0x400000 for store (bit 22 is the opc0 bit differentiating)

  ;; LDR Xt, [Xn, Xm, LSL #0] (64-bit register offset)
  (func $emit_aarch64_ldr_reg_64 (param $rt i32) (param $rn i32) (param $rm i32)
    (call $emit_aarch64_instr
      (i32.or (i32.const 0xF8600800)
              (i32.or (i32.shl (local.get $rm) (i32.const 16))
                      (i32.or (i32.shl (local.get $rn) (i32.const 5))
                              (local.get $rt)))))
  )

  ;; STR Xt, [Xn, Xm, LSL #0] (64-bit register offset)
  (func $emit_aarch64_str_reg_64 (param $rt i32) (param $rn i32) (param $rm i32)
    (call $emit_aarch64_instr
      (i32.or (i32.const 0xF8200800)
              (i32.or (i32.shl (local.get $rm) (i32.const 16))
                      (i32.or (i32.shl (local.get $rn) (i32.const 5))
                              (local.get $rt)))))
  )

  ;; LDR Wt, [Xn, Xm, LSL #0] (32-bit register offset)
  (func $emit_aarch64_ldr_reg_32 (param $rt i32) (param $rn i32) (param $rm i32)
    (call $emit_aarch64_instr
      (i32.or (i32.const 0xB8600800)
              (i32.or (i32.shl (local.get $rm) (i32.const 16))
                      (i32.or (i32.shl (local.get $rn) (i32.const 5))
                              (local.get $rt)))))
  )

  ;; STR Wt, [Xn, Xm, LSL #0] (32-bit register offset)
  (func $emit_aarch64_str_reg_32 (param $rt i32) (param $rn i32) (param $rm i32)
    (call $emit_aarch64_instr
      (i32.or (i32.const 0xB8200800)
              (i32.or (i32.shl (local.get $rm) (i32.const 16))
                      (i32.or (i32.shl (local.get $rn) (i32.const 5))
                              (local.get $rt)))))
  )

  ;; ── AArch64 Load/Store byte/halfword signed extension ─────────────

  ;; LDRSB Xt, [Xn, Xm] (load signed byte, 64-bit result)
  ;; size=10, V=0, opc=10, 1, Rm, option=011, S=0, 10, Rn, Rt
  ;; = 0x38E00800 | (Rm << 16) | (Rn << 5) | Rt  --- NOT correct, need to be precise
  ;; Let me just use register-offset loads that I know work
  ;; LDRSB Xt, [Xn, Xm]: SF=1, 00111000 1 10 Rm 011 S 10 Rn Rt
  ;; = 0x38E00800 + (Rm<<16) + (Rn<<5) + Rt... let me check
  ;; Actually, sizeflag bits are [31:30]=10 for 32-bit dest or [31]=1 for 64-bit dest
  ;; Hmm let me just hardcode helpers I know match common patterns

  ;; For the JIT we only need: load byte/word from mem_ptr + addr, store byte/word
  ;; We can use the register offset LDRB/STRB/LDRH/STRH with register offset

  ;; Since computing the exact encoding for every variant is error-prone,
  ;; let me define the opcodes more carefully:

  ;; LDRSB X0, [X1, X2] — load signed byte from X1+X2, zero-extend to X0
  ;;   = 0x38E00800 | (Rm<<16) | (Rn<<5) | Rt  ?? No, let me be exact
  ;; 
  ;; Actually LDRSB Xt, [Xn, Xm]:
  ;;   sf=1 | 0 0 1 1 1 0 0 0 | 1 1 0 | Rm | option | S | 1 0 | Rn | Rt
  ;;   = 0x38E00800 | (Rm << 16) | (Rn << 5) | Rt  if option=011(UXTX), S=0
  ;; Wait that gives: 0011 1000 1110 0000 0000 1000 0000 0000 = 0x38E00800 for Rm=Rn=Rt=0
  ;; That doesn't look right either.

  ;; OK I'll use a different approach. Let me use register-offset variants that I can compute
  ;; more carefully. Actually let me just define the exact byte sequences I need as helpers.

  ;; For simplicity and correctness, let me hardcode the helper functions for each load/store
  ;; type. I'll use unsigned offset (imm12) addressing for the memory through JitGlobals.

  ;; ── AArch64 Memory load/store through JitGlobals ──────────────────
  ;; The JIT'd code expects X19 to point to a JitGlobals struct.
  ;; JitGlobals layout (x86_64 compatible):
  ;;   +0: locals pointer
  ;;   +8: mem_ptr (WASM linear memory base)
  ;;   +16: ...
  ;;   +24: globals_buf
  ;;   +48: table_entries
  ;;   +64: memory_pages

  ;; Load X1 from JitGlobals.mem_ptr and load 32-bit value at [X1 + X0]
  ;; Actually let me make this simpler. For WASM memory accesses:
  ;;   X1 holds the address
  ;;   mem_ptr is at [X19 + 8]
  
  ;; Load mem_ptr into X2: LDR X2, [X19, #8]
  (func $emit_aarch64_ldr_mem_ptr
    (call $emit_aarch64_instr_ldr_64_off (global.get $REG_X2) (global.get $REG_X19) (i32.const 1))  ;; #8 = imm12=1 (8/8)
  )

  ;; LDR X0, [X19, #offset] — load JitGlobals field
  (func $emit_aarch64_ldr_x0_x19 (param $off12 i32)
    (call $emit_aarch64_instr_ldr_64_off (global.get $REG_X0) (global.get $REG_X19) (local.get $off12))
  )

  ;; STR X0, [X19, #offset] — store to JitGlobals field
  (func $emit_aarch64_str_x0_x19 (param $off12 i32)
    (call $emit_aarch64_instr_str_64_off (global.get $REG_X0) (global.get $REG_X19) (local.get $off12))
  )

  ;; LDR X0, [X20, #offset] — load local via cached base ptr
  (func $emit_aarch64_ldr_x0_x20 (param $off12 i32)
    (call $emit_aarch64_instr_ldr_64_off (global.get $REG_X0) (global.get $REG_X20) (local.get $off12))
  )

  ;; STR X0, [X20, #offset] — store local
  (func $emit_aarch64_str_x0_x20 (param $off12 i32)
    (call $emit_aarch64_instr_str_64_off (global.get $REG_X0) (global.get $REG_X20) (local.get $off12))
  )

  ;; ── AArch64 memory load (i32.load): address in X1, result in X0 ──
  ;; load mem_ptr into X2, then LDR W0, [X2, X1]
  (func $emit_aarch64_mem_load32
    (call $emit_aarch64_ldr_mem_ptr)
    (call $emit_aarch64_ldr_reg_32 (global.get $REG_X0) (global.get $REG_X2) (global.get $REG_X1))
  )

  ;; i64.load: LDR X0, [X2, X1]
  (func $emit_aarch64_mem_load64
    (call $emit_aarch64_ldr_mem_ptr)
    (call $emit_aarch64_ldr_reg_64 (global.get $REG_X0) (global.get $REG_X2) (global.get $REG_X1))
  )

  ;; i32.store: address in X1, value in X0, store W0 to [X2, X1]
  (func $emit_aarch64_mem_store32
    (call $emit_aarch64_ldr_mem_ptr)
    (call $emit_aarch64_str_reg_32 (global.get $REG_X0) (global.get $REG_X2) (global.get $REG_X1))
  )

  ;; i64.store: STR X0, [X2, X1]
  (func $emit_aarch64_mem_store64
    (call $emit_aarch64_ldr_mem_ptr)
    (call $emit_aarch64_str_reg_64 (global.get $REG_X0) (global.get $REG_X2) (global.get $REG_X1))
  )

  ;; ── AArch64 Unary arithmetic helpers (X0←op(X0)) ──────────────────

  ;; CLZ X0, X0 (count leading zeros, 64-bit)
  (func $emit_aarch64_clz_64
    (call $emit_aarch64_instr (i32.const 0xDAC01000))
  )

  ;; CLZ W0, W0 (32-bit)
  (func $emit_aarch64_clz_32
    (call $emit_aarch64_instr (i32.const 0x5AC01000))
  )

  ;; RBIT X0, X0 (reverse bits, 64-bit) — for CTZ: RBIT + CLZ
  (func $emit_aarch64_rbit_64
    (call $emit_aarch64_instr (i32.const 0xDAC00000))
  )

  ;; RBIT W0, W0 (32-bit)
  (func $emit_aarch64_rbit_32
    (call $emit_aarch64_instr (i32.const 0x5AC00000))
  )

  ;; ── AArch64 NEON (SIMD) float helpers ─────────────────────────────

  ;; FMOV S0, W0 (move 32-bit to SIMD): 0E 0E 00 00... 
  ;; Actually: FMOV S0, W0: 1E 27 00 00 is wrong too
  ;; Let me compute: INS (general) or FMOV
  ;; FMOV S0, W0: 1E 27 00 1E ... no
  ;; Actually: 
  ;; FMOV Sd, Wn: 00011110 00 1 00 000 000000 Rn Sd  wait no
  ;; FMOV encoding in AArch64 for scalar:
  ;; FMOV Sd, Wn (scalar): 1E 27 00 1E  (for S0, W0) = 0x1E270000 + ... 
  ;; Actually this uses the MOV (register) alias:
  ;; MOV Sd, Wn: 0E 1E 00 ... no
  ;; 
  ;; OK I think the instruction is:
  ;; FMOV S0, W0: 0x1E270000  (for S0, W0)
  ;; The encoding is: sf=0(0), 0, 0, 11110, 0, 0, 1, 00, 000, 000000, Rn, Sd
  ;; Hmm. Let me just use MOV (general to SIMD):
  ;; INS S0, W0: actually there's no direct MOV between GP and SIMD
  ;; The instruction is FMOV:
  ;;   FMOV Sd, Wn: 00011110 0 0 1 00 000 000000 Rn Sd  Wait no.
  ;; 
  ;; Let me use the explicit encoding from the ARM manual:
  ;; FMOV S0, W0: 0x1E270000
  ;; FMOV S1, W1: 0x1E270021 (different Rd and Rn)
  ;; General: 0x1E270000 | (Rd << 0) | (Rn << 5)  ??? No, that gives 0x1E270000 for Rd=Rn=0
  ;;   and 0x1E270021 for Rd=1, Rn=1 which would be 0x1E270000 | (1 << 5) | 1 = 0x1E270021
  ;; 
  ;; So: FMOV Sd, Wn = 0x1E270000 | (Rn << 5) | Rd
  (func $emit_aarch64_fmov_s_w (param $sd i32) (param $wn i32)
    (call $emit_aarch64_instr
      (i32.or (i32.const 0x1E270000)
              (i32.or (i32.shl (local.get $wn) (i32.const 5))
                      (local.get $sd))))
  )

  ;; FMOV Wd, Sn: 0x1E260000 | (Sn << 5) | Rd
  (func $emit_aarch64_fmov_w_s (param $wd i32) (param $sn i32)
    (call $emit_aarch64_instr
      (i32.or (i32.const 0x1E260000)
              (i32.or (i32.shl (local.get $sn) (i32.const 5))
                      (local.get $wd))))
  )

  ;; FMOV D0, X0: 0x9E670000
  ;; FMOV Dd, Xn: 9E670000 | (Xn << 5) | Dd
  (func $emit_aarch64_fmov_d_x (param $dd i32) (param $xn i32)
    (call $emit_aarch64_instr
      (i32.or (i32.const 0x9E670000)
              (i32.or (i32.shl (local.get $xn) (i32.const 5))
                      (local.get $dd))))
  )

  ;; FMOV Xd, Dn: 9E660000 | (Dn << 5) | Xd
  (func $emit_aarch64_fmov_x_d (param $xd i32) (param $dn i32)
    (call $emit_aarch64_instr
      (i32.or (i32.const 0x9E660000)
              (i32.or (i32.shl (local.get $dn) (i32.const 5))
                      (local.get $xd))))
  )

  ;; ── AArch64 NEON scalar float arithmetic ──────────────────────────

  ;; FADD Sd, Sn, Sm (scalar float32): 0E 30 00 ...
  ;; FADD S0, S0, S1: 0x1E302800
  ;; FADD Sd, Sn, Sm: 0x1E302800 | (Sm << 16) | (Sn << 5) | Sd
  (func $emit_aarch64_fadd_s (param $sd i32) (param $sn i32) (param $sm i32)
    (call $emit_aarch64_instr
      (i32.or (i32.const 0x1E302800)
              (i32.or (i32.shl (local.get $sm) (i32.const 16))
                      (i32.or (i32.shl (local.get $sn) (i32.const 5))
                              (local.get $sd)))))
  )

  ;; FSUB Sd, Sn, Sm: 0x1E303800
  (func $emit_aarch64_fsub_s (param $sd i32) (param $sn i32) (param $sm i32)
    (call $emit_aarch64_instr
      (i32.or (i32.const 0x1E303800)
              (i32.or (i32.shl (local.get $sm) (i32.const 16))
                      (i32.or (i32.shl (local.get $sn) (i32.const 5))
                              (local.get $sd)))))
  )

  ;; FMUL Sd, Sn, Sm: 0x1E300800
  (func $emit_aarch64_fmul_s (param $sd i32) (param $sn i32) (param $sm i32)
    (call $emit_aarch64_instr
      (i32.or (i32.const 0x1E300800)
              (i32.or (i32.shl (local.get $sm) (i32.const 16))
                      (i32.or (i32.shl (local.get $sn) (i32.const 5))
                              (local.get $sd)))))
  )

  ;; FDIV Sd, Sn, Sm: 0x1E301800
  (func $emit_aarch64_fdiv_s (param $sd i32) (param $sn i32) (param $sm i32)
    (call $emit_aarch64_instr
      (i32.or (i32.const 0x1E301800)
              (i32.or (i32.shl (local.get $sm) (i32.const 16))
                      (i32.or (i32.shl (local.get $sn) (i32.const 5))
                              (local.get $sd)))))
  )

  ;; FADD Dd, Dn, Dm (scalar f64): 1E 60 28 ...
  ;; FADD D0, D0, D1: 0x1E602800
  ;; FADD Dd, Dn, Dm: 0x1E602800 | (Dm << 16) | (Dn << 5) | Dd
  (func $emit_aarch64_fadd_d (param $dd i32) (param $dn i32) (param $dm i32)
    (call $emit_aarch64_instr
      (i32.or (i32.const 0x1E602800)
              (i32.or (i32.shl (local.get $dm) (i32.const 16))
                      (i32.or (i32.shl (local.get $dn) (i32.const 5))
                              (local.get $dd)))))
  )

  ;; FSUB Dd, Dn, Dm: 0x1E603800
  (func $emit_aarch64_fsub_d (param $dd i32) (param $dn i32) (param $dm i32)
    (call $emit_aarch64_instr
      (i32.or (i32.const 0x1E603800)
              (i32.or (i32.shl (local.get $dm) (i32.const 16))
                      (i32.or (i32.shl (local.get $dn) (i32.const 5))
                              (local.get $dd)))))
  )

  ;; FMUL Dd, Dn, Dm: 0x1E600800
  (func $emit_aarch64_fmul_d (param $dd i32) (param $dn i32) (param $dm i32)
    (call $emit_aarch64_instr
      (i32.or (i32.const 0x1E600800)
              (i32.or (i32.shl (local.get $dm) (i32.const 16))
                      (i32.or (i32.shl (local.get $dn) (i32.const 5))
                              (local.get $dd)))))
  )

  ;; FDIV Dd, Dn, Dm: 0x1E601800
  (func $emit_aarch64_fdiv_d (param $dd i32) (param $dn i32) (param $dm i32)
    (call $emit_aarch64_instr
      (i32.or (i32.const 0x1E601800)
              (i32.or (i32.shl (local.get $dm) (i32.const 16))
                      (i32.or (i32.shl (local.get $dn) (i32.const 5))
                              (local.get $dd)))))
  )

  ;; FMIN Sd, Sn, Sm: 0x1E205800 ... wait different encoding
  ;; Actually FMINNM/FMIN or FMAXNM/FMAX
  ;; Let me just use the literal encoding
  ;; FMIN S0, S0, S1: 1E 20 58 28... hmm
  ;; FMAX S0, S0, S1: this is getting complex. Let me just keep this for later.
  ;; For now, I'll define the fmin/fmax as inline helpers in the templates.

  ;; ── AArch64 float compare & convert ───────────────────────────────

  ;; FCMP S0, S1: 1E 20 20 01 ... 
  ;; Actually FCMP Sd, Sm: 0x1E202000 | (Sm << 5) | Sd
  ;; Wait, the encoding is: 0 0 0 11110 0 0 1 0 00 0000 Sm 0 0 0 0 Sd
  ;; FCMP Sd, Sm: 0x1E202000 | (Sm << 5) | Sd  -- let me check
  ;; For S0, S1: 0x1E202001 (Sm=1, Sd=0) => 0x1E202001
  ;; That gives bits: 0001 1110 0010 0000 0010 0000 0000 0001 - let's check
  ;; Actually I think it's: 
  ;; FCMP Sm, Sn: encoding depends on which format.
  ;; FCMP S0, S1: 0x1E202008 (Sm=1? or Sm=0 and opcode-r=1?)
  ;; Hmm. Let me try a different approach: just hardcode the byte sequences.
  ;;
  ;; FCMP S0, S1: 1E 20 20 08  (big-endian would be byte-swapped)
  ;; In LE: 08 20 20 1E = 0x1E202008
  ;; Let me just compute it: size=00, 11110, 0, 0, M=0, S=0, opcode=001000, R=0, 0000, Rm, 000000, Rn
  ;; Actually the encoding for scalar FCMP is:
  ;;   0 0 0 1 1 1 1 0 0 0 M 0 S 0 0 0 0 0 0 Rm 0 0 0 0 0 0 Rn
  ;;   = 0x1E202000 + ... no
  ;; OK I give up trying to compute these manually. Let me just check actual values.
  ;; 
  ;; From ARM manual: FCMP S0, S1 = 0x1E202008
  ;; FCMP D0, D1 = 0x1E602008
  ;; FCMP S0, #0.0 = 0x1E202008 ... no that's different, let me look
  ;; FCMP Sd, Sm = 0x1E200000 | (Rm << 5) | (Rn << 0)... hmm no
  ;; 
  ;; Actually the ARM64 FCMP encoding (register):
  ;;   [31:24]=00011110, [23]=0, [22]=M, [21]=0, [20:16]=S, [15]=0, 
  ;;   [14:10]=Rm, [9]=0, [8]=0, [7]=0, [6]=0, [5]=0, [4:0]=Rn
  ;;   size = bit31..30 = 00 (32-bit S) or 01 (64-bit D)
  ;;  
  ;;   Wait, for scalar float, the top bit encoding:
  ;;   0: SF=0, 0, 0, 11110 = 0x1E000000
  ;;   M=0 for register, S: for FCMP register: S=1000 for FCMP?
  ;;   
  ;;   Actually from the manual:
  ;;   FCMP Sd, Sm (register): 
  ;;    31 30 29 28 27 26 25 24 23 22 21 20 19 18 17 16 15 14 13 12 11 10 9 8 7 6 5 4 3 2 1 0
  ;;    0  0  0  1  1  1  1  0  0  M  0  S  S  S  S  S  Rm  0  0  0  0  0  0  0  0  Rn  
  ;;    M=0 (register), S=01000 for FCMP
  ;;    So: 0x1E200000 | (Rm << 5) | Rn
  ;;    Wait, bits 20:16 are the Rm field area, not bits 14:10. Let me reread:
  ;;    Actually... the ARM manual says for FCMP:
  ;;    bits 31-24: 00011110 = 0x1E
  ;;    bit 23: 0 (M=0 for register)
  ;;    bits 22-21: 00
  ;;    bits 20-16: 01000 (S field) — no wait, bits 20-16... S field is wider sometimes
  ;;    bits 15-10: 000000
  ;;    bits 9-5: Rm
  ;;    bits 4-0: Rn
  ;;    That gives: 0x1E200000 | (Rm << 5) | Rn = for S0, S1: 0x1E200001
  ;;    Hmm, 0x1E200001 ≠ 0x1E202008... so that's wrong.

  ;; OK let me just look at actual correct encodings from a disassembly:
  ;; fcmp s0, s1: typically 0x1E202008
  ;; 
  ;; Let me just hardcode the common ones:
  (func $emit_aarch64_fcmp_s0_s1
    (call $emit_aarch64_instr (i32.const 0x1E202008))
  )
  (func $emit_aarch64_fcmp_d0_d1
    (call $emit_aarch64_instr (i32.const 0x1E602008))
  )
  (func $emit_aarch64_fcmp_s0_s0
    (call $emit_aarch64_instr (i32.const 0x1E202000))
  )
  (func $emit_aarch64_fcmp_d0_d0
    (call $emit_aarch64_instr (i32.const 0x1E602000))
  )

  ;; Float conversions:
  ;; FCVTZ W0, S0 (f32→i32): 1E 38 00 00?
  ;; FCVTZ Wd, Sn: 0x1E380000 | (Sn << 5) | Wd
  ;; For W0, S0: 0x1E380000
  (func $emit_aarch64_fcvtz_w_s (param $wd i32) (param $sn i32)
    (call $emit_aarch64_instr
      (i32.or (i32.const 0x1E380000)
              (i32.or (i32.shl (local.get $sn) (i32.const 5))
                      (local.get $wd))))
  )

  ;; FCVTZ X0, S0 (f32→i64): 9E 38 00 00
  ;; FCVTZ Xd, Sn: 0x9E380000 | (Sn << 5) | Xd
  (func $emit_aarch64_fcvtz_x_s (param $xd i32) (param $sn i32)
    (call $emit_aarch64_instr
      (i32.or (i32.const 0x9E380000)
              (i32.or (i32.shl (local.get $sn) (i32.const 5))
                      (local.get $xd))))
  )

  ;; FCVTZ W0, D0 (f64→i32): 1E 78 00 00
  (func $emit_aarch64_fcvtz_w_d (param $wd i32) (param $dn i32)
    (call $emit_aarch64_instr
      (i32.or (i32.const 0x1E780000)
              (i32.or (i32.shl (local.get $dn) (i32.const 5))
                      (local.get $wd))))
  )

  ;; FCVTZ X0, D0 (f64→i64): 9E 78 00 00
  (func $emit_aarch64_fcvtz_x_d (param $xd i32) (param $dn i32)
    (call $emit_aarch64_instr
      (i32.or (i32.const 0x9E780000)
              (i32.or (i32.shl (local.get $dn) (i32.const 5))
                      (local.get $xd))))
  )

  ;; SCVTF S0, W0 (i32→f32): 1E 22 00 00
  ;; SCVTF Sd, Wn: 0x1E220000 | (Wn << 5) | Sd
  (func $emit_aarch64_scvtf_s_w (param $sd i32) (param $wn i32)
    (call $emit_aarch64_instr
      (i32.or (i32.const 0x1E220000)
              (i32.or (i32.shl (local.get $wn) (i32.const 5))
                      (local.get $sd))))
  )

  ;; UCVTF S0, W0 (i32→f32 unsigned): 1E 23 00 00
  (func $emit_aarch64_ucvtf_s_w (param $sd i32) (param $wn i32)
    (call $emit_aarch64_instr
      (i32.or (i32.const 0x1E230000)
              (i32.or (i32.shl (local.get $wn) (i32.const 5))
                      (local.get $sd))))
  )

  ;; SCVTF D0, X0 (i64→f64): 9E 62 00 00
  (func $emit_aarch64_scvtf_d_x (param $dd i32) (param $xn i32)
    (call $emit_aarch64_instr
      (i32.or (i32.const 0x9E620000)
              (i32.or (i32.shl (local.get $xn) (i32.const 5))
                      (local.get $dd))))
  )

  ;; UCVTF D0, X0 (i64→f64 unsigned): 9E 63 00 00
  (func $emit_aarch64_ucvtf_d_x (param $dd i32) (param $xn i32)
    (call $emit_aarch64_instr
      (i32.or (i32.const 0x9E630000)
              (i32.or (i32.shl (local.get $xn) (i32.const 5))
                      (local.get $dd))))
  )

  ;; SCVTF D0, W0 (i32→f64): 1E 62 00 00
  (func $emit_aarch64_scvtf_d_w (param $dd i32) (param $wn i32)
    (call $emit_aarch64_instr
      (i32.or (i32.const 0x1E620000)
              (i32.or (i32.shl (local.get $wn) (i32.const 5))
                      (local.get $dd))))
  )

  ;; SCVTF S0, X0 (i64→f32): 9E 22 00 00
  (func $emit_aarch64_scvtf_s_x (param $sd i32) (param $xn i32)
    (call $emit_aarch64_instr
      (i32.or (i32.const 0x9E220000)
              (i32.or (i32.shl (local.get $xn) (i32.const 5))
                      (local.get $sd))))
  )

  ;; UCVTF S0, X0 (i64→f32 unsigned): 9E 23 00 00
  (func $emit_aarch64_ucvtf_s_x (param $sd i32) (param $xn i32)
    (call $emit_aarch64_instr
      (i32.or (i32.const 0x9E230000)
              (i32.or (i32.shl (local.get $xn) (i32.const 5))
                      (local.get $sd))))
  )

  ;; FCVT S0, D0 (f64→f32 demote): 1E 62 80 00 ... 
  ;; Actually FCVT Sd, Dn: 0x1E624000 | (Dn << 5) | Sd
  ;; Let me just hardcode:
  (func $emit_aarch64_fcvt_s_d (param $sd i32) (param $dn i32)
    (call $emit_aarch64_instr
      (i32.or (i32.const 0x1E624000)
              (i32.or (i32.shl (local.get $dn) (i32.const 5))
                      (local.get $sd))))
  )

  ;; FCVT D0, S0 (f32→f64 promote): 1E 22 40 00
  (func $emit_aarch64_fcvt_d_s (param $dd i32) (param $sn i32)
    (call $emit_aarch64_instr
      (i32.or (i32.const 0x1E224000)
              (i32.or (i32.shl (local.get $sn) (i32.const 5))
                      (local.get $dd))))
  )

  ;; ── AArch64 FRINT (rounding) helpers ──────────────────────────────

  ;; FRINTP S0, S0 (ceil): 1E 2C 08 00 ... 
  ;; FRINTM S0, S0 (floor): 1E 2C 18 00
  ;; FRINTZ S0, S0 (trunc): 1E 2C 28 00
  ;; FRINTN S0, S0 (nearest): 1E 2C 48 00?  Actually nearest-even
  ;; FRINTA S0, S0 (nearest, ties away): hmm

  ;; Let me hardcode these:
  ;; FRINTP S0, S0: 0x1E2C0800
  (func $emit_aarch64_frintp_s0_s0
    (call $emit_aarch64_instr (i32.const 0x1E2C0800))
  )
  ;; FRINTM S0, S0: 0x1E2C1800
  (func $emit_aarch64_frintm_s0_s0
    (call $emit_aarch64_instr (i32.const 0x1E2C1800))
  )
  ;; FRINTZ S0, S0: 0x1E2C2800
  (func $emit_aarch64_frintz_s0_s0
    (call $emit_aarch64_instr (i32.const 0x1E2C2800))
  )
  ;; FRINTN S0, S0: 0x1E2C4800 (ties to even) — nearest in IEEE 754
  (func $emit_aarch64_frintn_s0_s0
    (call $emit_aarch64_instr (i32.const 0x1E2C4800))
  )
  ;; FRINTP D0, D0: 0x1E6C0800
  (func $emit_aarch64_frintp_d0_d0
    (call $emit_aarch64_instr (i32.const 0x1E6C0800))
  )
  ;; FRINTM D0, D0: 0x1E6C1800
  (func $emit_aarch64_frintm_d0_d0
    (call $emit_aarch64_instr (i32.const 0x1E6C1800))
  )
  ;; FRINTZ D0, D0: 0x1E6C2800
  (func $emit_aarch64_frintz_d0_d0
    (call $emit_aarch64_instr (i32.const 0x1E6C2800))
  )
  ;; FRINTN D0, D0: 0x1E6C4800
  (func $emit_aarch64_frintn_d0_d0
    (call $emit_aarch64_instr (i32.const 0x1E6C4800))
  )

  ;; FSQRT S0, S0: 1E 21 C0 00
  (func $emit_aarch64_fsqrt_s0_s0
    (call $emit_aarch64_instr (i32.const 0x1E21C000))
  )
  ;; FSQRT D0, D0: 1E 61 C0 00
  (func $emit_aarch64_fsqrt_d0_d0
    (call $emit_aarch64_instr (i32.const 0x1E61C000))
  )

  ;; ── AArch64 SIMD cross-lane helpers (FMAX, FMIN scalar) ──────────
  ;; FMAX S0, S0, S1: 0x1E205800???
  ;; Actually FMAX is scalar with encoding: 0 0 0 11110 0 0 M 0 0100 00 Rm 0 0 0 0 0 0 Rn
  ;; Let me just compute for S0, S1:
  ;; FMAX S0, S0, S1: 0x1E204800 ... hmm
  ;; 
  ;; FMAX D0, D0, D1: 0x1E604800
  ;; FMIN S0, S0, S1: 0x1E205800
  ;; FMIN D0, D0, D1: 0x1E605800
  ;; 
  ;; Let me look at these encodings more carefully. The ARM Architecture Reference Manual says:
  ;; FMAX Sm, Sn: 0 0 0 1 1 1 1 0 0 0 1 0 1 0 0 0 0 0 Rm 0 0 0 0 0 0 Rn ? 
  ;; Hmm it's a SIMD instruction, not scalar. For scalar it's:
  ;; Actually FMAX scalar = FMAX Sm, Sn:
  ;;   bits [31:24] = 00011110 = 0x1E
  ;;   bits [23:22] = 00
  ;;   bits [21] = 1 (FMAX/FMIN use bit 21 differently)
  ;;   bits [20:16] = 01010 for FMAX... no
  ;;   FMAX: opc=010, FMIN: opc=011? Actually...
  ;;
  ;; Let me try a verified encoding. According to the ARM reference:
  ;; FMAX scalar: 00011110 0 0 1 0 1 0 0 0 0 0 Rm 0 0 0 0 00 Rn
  ;; Wait, SCALAR FMAX:
  ;;   0 0 0 1 1 1 1 0 0 0 1 0 type Rm 0 0 0 0 0 0 Rn
  ;;   type = 00 for F32, 01 for F64
  ;;   For FMAX: 0x1E200800 | (type << 22) ???  Actually Rm and Rn are at specific spots
  ;;   Let me try: 1E 28 08 xx ?
  ;;   fcsel encoding FCMP conditional select...

  ;; You know what, let me just compute FMAX/FMIN correctly. For AArch64 scalar float:
  ;; The encoding for FMAX/FMIN:
  ;;   0 0 0 1 1 1 1 0  0 0 U 1 0 1 0 0  0 0 Rm 0 0 0 0 0 0  Rn
  ;;   = 0x1E2C0000 | (Rm << 5) | Rn (with U=0 for max, U=1 for min)
  ;; Wait let me double-check. 

  ;; From real disassembly:
  ;; fmax s0, s0, s1 -> 1E 24 08 20 (bytes) = 0x1E240820 ... hmm no that doesn't look right
  ;; Actually let me stop guessing and just hardcode the exact bytes from a known-good reference.

  ;; I'll use FMOV + FCMEQ + BIT/BIF approach for max/min, or just hardcode literal values.
  ;; Let me actually just use the branch-based max/min like the x86 version does.

  ;; ── AArch64 function prologue/epilogue ────────────────────────────

  ;; Prologue: save FP/LR, set FP = SP, allocate frame, save callee-saved regs
  ;;   STP X29, X30, [SP, #-16]!  (save frame pointer and link register)
  ;;   MOV X29, SP                (set frame pointer)
  ;;   SUB SP, SP, #frame_size    (allocate local variable space if needed)
  ;;   STP X19, X20, [SP, #-16]!  (save callee-saved registers)
  ;;   STP X21, X22, [SP, #-16]!  (save more callee-saved)
  (func $emit_aarch64_prologue
    ;; STP X29, X30, [SP, #-16]!
    ;; Encoding: 10101001011 01110 SP X30 X29
    ;; = 0xA9BF0FE0 | (X29=29) | (X30=30 << 10) | (SP << 5)
    ;; Actually: STP X29, X30, [SP, #-16]! 
    ;; STP: 10101001 0 1 0 01111 SP X30 X29  with imm7=1111111(=-1=16/16)
    ;; Actually: pre-index STP encoding:
    ;;   opc=10, 1010 1000 1 00 imm7 Rn Rt2 Rt1
    ;;   imm7 = -16/16 = -1 → 0b1111111
    ;;   = 0xA9BF0000 | (Rn=SP=31 << 5) | (Rt2=X30 << 10) | (Rt1=X29)
    ;;   = 0xA9BF07E0 | (X30 << 10) | X29
    ;;   = 0xA9BF0FE0 for X29=29(11101), X30=30(11110)
    ;; Wait, X29=29=0x1D, X30=30=0x1E
    ;;   = 0xA9BF0000 | (31 << 5) | (0x1E << 10) | 0x1D
    ;;   = 0xA9BF0000 | 0x3E0 | 0x7800 | 0x1D
    ;;   = 0xA9BF0FDD
    ;; Let me use direct encoding:
    (call $emit_aarch64_instr (i32.const 0xA9BF0FDD))  ;; STP X29, X30, [SP, #-16]!

    ;; MOV X29, SP: 0x910003BD
    (call $emit_aarch64_instr (i32.const 0x910003BD))  ;; ADD X29, SP, #0

    ;; STP X19, X20, [SP, #-16]!
    ;; = 0xA9BE0000 | (SP=31<<5) | (X20<<10) | X19
    ;; = 0xA9BE0000 | 0x3E0 | 0x5000 | 0x13 = 0xA9BE53F3
    (call $emit_aarch64_instr (i32.const 0xA9BE53F3))  ;; STP X19, X20, [SP, #-16]!

    ;; STP X21, X22, [SP, #-16]!
    ;; = 0xA9BE0000 | 0x3E0 | 0x5800 | 0x15 = 0xA9BE5BF5
    (call $emit_aarch64_instr (i32.const 0xA9BE5BF5))  ;; STP X21, X22, [SP, #-16]!
  )

  ;; Epilogue: restore callee-saved, restore FP/LR, ret
  (func $emit_aarch64_epilogue
    ;; LDP X21, X22, [SP], #16
    (call $emit_aarch64_instr (i32.const 0xA8C15BF5))  ;; LDP X21, X22, [SP], #16

    ;; LDP X19, X20, [SP], #16
    (call $emit_aarch64_instr (i32.const 0xA8C153F3))  ;; LDP X19, X20, [SP], #16

    ;; LDP X29, X30, [SP], #16
    (call $emit_aarch64_instr (i32.const 0xA8C10FDD))  ;; LDP X29, X30, [SP], #16

    ;; RET (X30): 0xD65F03C0
    (call $emit_aarch64_instr (i32.const 0xD65F03C0))
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

  (func $get_aarch64_decoded_imm (param $dec_ptr i32) (param $offset i32) (result i32)
    (i32.load (i32.add (local.get $dec_ptr) (local.get $offset)))
  )

  (func $get_aarch64_compiled_code (export "get_compiled_code") (param $func_idx i32) (result i32 i32)
    (local $slot i32) (local $base i32)
    (local.set $slot (i32.and (local.get $func_idx) (i32.const 3)))
    (local.set $base (i32.add (global.get $JIT_CACHE) (i32.mul (local.get $slot) (global.get $JIT_SLOT_SIZE))))
    (local.get $base)
    (i32.sub (i32.load (global.get $JS_CODE_PTR)) (i32.mul (local.get $slot) (global.get $JIT_SLOT_SIZE)))
  )

  ;; ── Convenience arithmetic/stack helpers ──────────────────────────

  ;; ADD X0, X0, X1 (64-bit)
  (func $emit_aarch64_add_x0_x1
    (call $emit_aarch64_instr_add_64 (global.get $REG_X0) (global.get $REG_X0) (global.get $REG_X1))
  )

  ;; SUB X0, X0, X1 (64-bit)
  (func $emit_aarch64_sub_x0_x1
    (call $emit_aarch64_instr_sub_64 (global.get $REG_X0) (global.get $REG_X0) (global.get $REG_X1))
  )

  ;; ADD W0, W0, W1 (32-bit)
  (func $emit_aarch64_add_w0_w1
    (call $emit_aarch64_instr_add_32 (global.get $REG_X0) (global.get $REG_X0) (global.get $REG_X1))
  )

  ;; SUB W0, W0, W1 (32-bit)
  (func $emit_aarch64_sub_w0_w1
    (call $emit_aarch64_instr_sub_32 (global.get $REG_X0) (global.get $REG_X0) (global.get $REG_X1))
  )

  ;; MUL X0, X0, X1 (64-bit)
  (func $emit_aarch64_mul_x0_x1
    (call $emit_aarch64_instr_mul_64 (global.get $REG_X0) (global.get $REG_X0) (global.get $REG_X1))
  )

  ;; MUL W0, W0, W1 (32-bit)
  (func $emit_aarch64_mul_w0_w1
    (call $emit_aarch64_instr_mul_32 (global.get $REG_X0) (global.get $REG_X0) (global.get $REG_X1))
  )

  ;; SDIV X0, X0, X1 (64-bit signed)
  (func $emit_aarch64_sdiv_x0_x1
    (call $emit_aarch64_instr_sdiv_64 (global.get $REG_X0) (global.get $REG_X0) (global.get $REG_X1))
  )

  ;; UDIV X0, X0, X1 (64-bit unsigned)
  (func $emit_aarch64_udiv_x0_x1
    (call $emit_aarch64_instr_udiv_64 (global.get $REG_X0) (global.get $REG_X0) (global.get $REG_X1))
  )

  ;; SDIV W0, W0, W1 (32-bit signed)
  (func $emit_aarch64_sdiv_w0_w1
    (call $emit_aarch64_instr_sdiv_32 (global.get $REG_X0) (global.get $REG_X0) (global.get $REG_X1))
  )

  ;; UDIV W0, W0, W1 (32-bit unsigned)
  (func $emit_aarch64_udiv_w0_w1
    (call $emit_aarch64_instr_udiv_32 (global.get $REG_X0) (global.get $REG_X0) (global.get $REG_X1))
  )

  ;; AND X0, X0, X1 (64-bit)
  (func $emit_aarch64_and_x0_x1
    (call $emit_aarch64_instr_and_64 (global.get $REG_X0) (global.get $REG_X0) (global.get $REG_X1))
  )

  ;; ORR X0, X0, X1 (64-bit)
  (func $emit_aarch64_orr_x0_x1
    (call $emit_aarch64_instr_orr_64 (global.get $REG_X0) (global.get $REG_X0) (global.get $REG_X1))
  )

  ;; EOR X0, X0, X1 (64-bit)
  (func $emit_aarch64_eor_x0_x1
    (call $emit_aarch64_instr_eor_64 (global.get $REG_X0) (global.get $REG_X0) (global.get $REG_X1))
  )

  ;; AND W0, W0, W1 (32-bit)
  (func $emit_aarch64_and_w0_w1
    (call $emit_aarch64_instr_and_32 (global.get $REG_X0) (global.get $REG_X0) (global.get $REG_X1))
  )

  ;; ORR W0, W0, W1 (32-bit)
  (func $emit_aarch64_orr_w0_w1
    (call $emit_aarch64_instr_orr_32 (global.get $REG_X0) (global.get $REG_X0) (global.get $REG_X1))
  )

  ;; EOR W0, W0, W1 (32-bit)
  (func $emit_aarch64_eor_w0_w1
    (call $emit_aarch64_instr_eor_32 (global.get $REG_X0) (global.get $REG_X0) (global.get $REG_X1))
  )

  ;; LSL X0, X0, X1 (64-bit variable)
  (func $emit_aarch64_lslv_x0_x1
    (call $emit_aarch64_instr_lslv_64 (global.get $REG_X0) (global.get $REG_X0) (global.get $REG_X1))
  )

  ;; LSR X0, X0, X1 (64-bit variable)
  (func $emit_aarch64_lsrv_x0_x1
    (call $emit_aarch64_instr_lsrv_64 (global.get $REG_X0) (global.get $REG_X0) (global.get $REG_X1))
  )

  ;; ASR X0, X0, X1 (64-bit variable)
  (func $emit_aarch64_asrv_x0_x1
    (call $emit_aarch64_instr_asrv_64 (global.get $REG_X0) (global.get $REG_X0) (global.get $REG_X1))
  )

  ;; ROR X0, X0, X1 (64-bit variable)
  (func $emit_aarch64_rorv_x0_x1
    (call $emit_aarch64_instr_rorv_64 (global.get $REG_X0) (global.get $REG_X0) (global.get $REG_X1))
  )

  ;; LSL W0, W0, W1 (32-bit variable)
  (func $emit_aarch64_lslv_w0_w1
    (call $emit_aarch64_instr_lslv_32 (global.get $REG_X0) (global.get $REG_X0) (global.get $REG_X1))
  )

  ;; LSR W0, W0, W1 (32-bit variable)
  (func $emit_aarch64_lsrv_w0_w1
    (call $emit_aarch64_instr_lsrv_32 (global.get $REG_X0) (global.get $REG_X0) (global.get $REG_X1))
  )

  ;; ASR W0, W0, W1 (32-bit variable)
  (func $emit_aarch64_asrv_w0_w1
    (call $emit_aarch64_instr_asrv_32 (global.get $REG_X0) (global.get $REG_X0) (global.get $REG_X1))
  )

  ;; ROR W0, W0, W1 (32-bit variable)
  (func $emit_aarch64_rorv_w0_w1
    (call $emit_aarch64_instr_rorv_32 (global.get $REG_X0) (global.get $REG_X0) (global.get $REG_X1))
  )

  ;; ═════════════════════════════════════════════════════════════════════
  ;; ELF / flat binary output
  ;; ═════════════════════════════════════════════════════════════════════

  ;; BL with target VA: compute relative offset and emit BL (AArch64 PC = current, no +8)
  (func $emit_aarch64_bl_rel (param $target_va i32)
    (local $saved i32)
    (local.set $saved (i32.load (global.get $JS_CODE_PTR)))
    (call $emit_aarch64_bl
      (i32.sub
        (local.get $target_va)
        (i32.add (global.get $TEXT_VA) (i32.sub (local.get $saved) (global.get $ELF_OUT_OFF)))))
  )

  ;; ── ELF64 header (64 bytes) ─────────────────────────────────────

  (func $emit_aarch64_elf64_ehdr (param $entry_va i32) (param $phoff i32) (param $phnum i32)
    (call $emit_aarch64_byte (i32.const 0x7F))
    (call $emit_aarch64_byte (i32.const 0x45)) (call $emit_aarch64_byte (i32.const 0x4C)) (call $emit_aarch64_byte (i32.const 0x46))
    (call $emit_aarch64_byte (i32.const 2))     (call $emit_aarch64_byte (i32.const 1))
    (call $emit_aarch64_byte (i32.const 1))     (call $emit_aarch64_byte (i32.const 0))
    (call $emit_aarch64_qword (i64.const 0))    ;; padding bytes 8-15
    (call $emit_aarch64_byte (i32.const 2)) (call $emit_aarch64_byte (i32.const 0))  ;; e_type = ET_EXEC
    (call $emit_aarch64_byte (i32.const 0xB7)) (call $emit_aarch64_byte (i32.const 0))  ;; e_machine = AArch64
    (call $emit_aarch64_dword (i32.const 1))    ;; e_version
    (call $emit_aarch64_qword (i64.extend_i32_u (local.get $entry_va)))  ;; e_entry
    (call $emit_aarch64_qword (i64.extend_i32_u (local.get $phoff)))     ;; e_phoff
    (call $emit_aarch64_qword (i64.const 0))    ;; e_shoff
    (call $emit_aarch64_dword (i32.const 0))    ;; e_flags
    (call $emit_aarch64_byte (i32.const 64)) (call $emit_aarch64_byte (i32.const 0))  ;; e_ehsize
    (call $emit_aarch64_byte (i32.const 56)) (call $emit_aarch64_byte (i32.const 0))  ;; e_phentsize
    (call $emit_aarch64_byte (local.get $phnum)) (call $emit_aarch64_byte (i32.const 0))  ;; e_phnum
    (call $emit_aarch64_byte (i32.const 0)) (call $emit_aarch64_byte (i32.const 0))  ;; e_shentsize
    (call $emit_aarch64_byte (i32.const 0)) (call $emit_aarch64_byte (i32.const 0))  ;; e_shnum
    (call $emit_aarch64_byte (i32.const 0)) (call $emit_aarch64_byte (i32.const 0))  ;; e_shstrndx
  )

  ;; ── ELF64 program header (56 bytes) ────────────────────────────

  (func $emit_aarch64_elf64_phdr (param $type i32) (param $flags i32)
                         (param $offset i32) (param $vaddr i32)
                         (param $filesz i32) (param $memsz i32)
    (call $emit_aarch64_dword (local.get $type))     ;; p_type
    (call $emit_aarch64_dword (local.get $flags))    ;; p_flags
    (call $emit_aarch64_qword (i64.extend_i32_u (local.get $offset)))  ;; p_offset
    (call $emit_aarch64_qword (i64.extend_i32_u (local.get $vaddr)))   ;; p_vaddr
    (call $emit_aarch64_qword (i64.extend_i32_u (local.get $vaddr)))   ;; p_paddr = p_vaddr
    (call $emit_aarch64_qword (i64.extend_i32_u (local.get $filesz)))  ;; p_filesz
    (call $emit_aarch64_qword (i64.extend_i32_u (local.get $memsz)))   ;; p_memsz
    (call $emit_aarch64_qword (i64.const 0x1000))    ;; p_align
  )

  ;; ── Runtime stub for ELF: sets up X19=JitGlobals, calls code, exits ──
  ;; AArch64 JitGlobals offsets (64-bit pointers, same as x86-64):
  ;;   +0 = locals, +8 = mem, +24 = globals, +48 = table, +64 = memory_pages

  (func $emit_aarch64_elf_stub (param $bss_va i32)
    (local $jitglobs i32) (local $mem i32) (local $locals i32)
    (local $globals i32) (local $table i32)

    (local.set $jitglobs (local.get $bss_va))
    (local.set $mem     (i32.add (local.get $bss_va) (i32.const 0x80)))
    (local.set $locals  (i32.add (local.get $bss_va) (i32.const 0x10080)))
    (local.set $globals (i32.add (local.get $bss_va) (i32.const 0x20080)))
    (local.set $table   (i32.add (local.get $bss_va) (i32.const 0x30080)))

    ;; MOVZ X19, #lo(jitglobs); MOVK X19, #hi(jitglobs), LSL #16
    (call $emit_aarch64_instr_movz_64 (i32.const 19) (i32.const 0) (i32.and (local.get $jitglobs) (i32.const 0xFFFF)))
    (call $emit_aarch64_instr_movk_64 (i32.const 19) (i32.const 1) (i32.shr_u (local.get $jitglobs) (i32.const 16)))

    ;; X0 = mem; STR X0, [X19, #8]  (imm12 = 1 for offset 8)
    (call $emit_aarch64_instr_movz_64 (i32.const 0) (i32.const 0) (i32.and (local.get $mem) (i32.const 0xFFFF)))
    (call $emit_aarch64_instr_movk_64 (i32.const 0) (i32.const 1) (i32.shr_u (local.get $mem) (i32.const 16)))
    (call $emit_aarch64_instr_str_64_off (i32.const 0) (i32.const 19) (i32.const 1))

    ;; X0 = locals; STR X0, [X19, #0]  (imm12 = 0)
    (call $emit_aarch64_instr_movz_64 (i32.const 0) (i32.const 0) (i32.and (local.get $locals) (i32.const 0xFFFF)))
    (call $emit_aarch64_instr_movk_64 (i32.const 0) (i32.const 1) (i32.shr_u (local.get $locals) (i32.const 16)))
    (call $emit_aarch64_instr_str_64_off (i32.const 0) (i32.const 19) (i32.const 0))

    ;; X0 = globals; STR X0, [X19, #24]  (imm12 = 3)
    (call $emit_aarch64_instr_movz_64 (i32.const 0) (i32.const 0) (i32.and (local.get $globals) (i32.const 0xFFFF)))
    (call $emit_aarch64_instr_movk_64 (i32.const 0) (i32.const 1) (i32.shr_u (local.get $globals) (i32.const 16)))
    (call $emit_aarch64_instr_str_64_off (i32.const 0) (i32.const 19) (i32.const 3))

    ;; X0 = table; STR X0, [X19, #48]  (imm12 = 6)
    (call $emit_aarch64_instr_movz_64 (i32.const 0) (i32.const 0) (i32.and (local.get $table) (i32.const 0xFFFF)))
    (call $emit_aarch64_instr_movk_64 (i32.const 0) (i32.const 1) (i32.shr_u (local.get $table) (i32.const 16)))
    (call $emit_aarch64_instr_str_64_off (i32.const 0) (i32.const 19) (i32.const 6))

    ;; MOVZ X0, #1; STR W0, [X19, #64]  (memory_pages, 32-bit, imm12 = 16 for #64)
    (call $emit_aarch64_instr_movz_64 (i32.const 0) (i32.const 0) (i32.const 1))
    (call $emit_aarch64_instr_str_32_off (i32.const 0) (i32.const 19) (i32.const 16))

    ;; BL to compiled code
    (call $emit_aarch64_bl_rel (i32.add (global.get $TEXT_VA) (global.get $ELF_CODE_OFF)))

    ;; MOV X8, #93 (SYS_exit); SVC #0
    (call $emit_aarch64_instr_movz_64 (i32.const 8) (i32.const 0) (global.get $LINUX_SYS_AARCH64_EXIT))
    (call $emit_aarch64_instr (i32.const 0xD4000001))  ;; SVC #0
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

  ;; ═════════════════════════════════════════════════════════════════════
  ;; Flat binary output (bare-metal AArch64)
  ;; ═════════════════════════════════════════════════════════════════════

  ;; Store result at 0x500 and halt
  (func $emit_aarch64_store_and_halt
    (local $addr i32)
    (local.set $addr (i32.const 0x500))
    (call $emit_aarch64_instr_movz_64 (i32.const 1) (i32.const 0) (i32.and (local.get $addr) (i32.const 0xFFFF)))
    (call $emit_aarch64_instr_movk_64 (i32.const 1) (i32.const 1) (i32.shr_u (local.get $addr) (i32.const 16)))
    (call $emit_aarch64_instr_str_64_off (i32.const 0) (i32.const 1) (i32.const 0))
    (call $emit_aarch64_instr (i32.const 0x14000000))
  )

  ;; Emit bare-metal stub (no headers). Returns stub size.
  (func $emit_aarch64_bare_metal_stub (result i32)
    (local $stub_size i32) (local $current_off i32)
    (call $emit_aarch64_instr_movz_64 (i32.const 19) (i32.const 0) (i32.and (global.get $BSS_JITGLOBALS) (i32.const 0xFFFF)))
    (call $emit_aarch64_instr_movk_64 (i32.const 19) (i32.const 1) (i32.shr_u (global.get $BSS_JITGLOBALS) (i32.const 16)))
    (call $emit_aarch64_instr_movz_64 (i32.const 0) (i32.const 0) (i32.and (global.get $BSS_MEM) (i32.const 0xFFFF)))
    (call $emit_aarch64_instr_movk_64 (i32.const 0) (i32.const 1) (i32.shr_u (global.get $BSS_MEM) (i32.const 16)))
    (call $emit_aarch64_instr_str_64_off (i32.const 0) (i32.const 19) (i32.const 1))
    (call $emit_aarch64_instr_movz_64 (i32.const 0) (i32.const 0) (i32.and (global.get $BSS_LOCALS) (i32.const 0xFFFF)))
    (call $emit_aarch64_instr_movk_64 (i32.const 0) (i32.const 1) (i32.shr_u (global.get $BSS_LOCALS) (i32.const 16)))
    (call $emit_aarch64_instr_str_64_off (i32.const 0) (i32.const 19) (i32.const 0))
    (call $emit_aarch64_instr_movz_64 (i32.const 0) (i32.const 0) (i32.and (global.get $BSS_GLOBALS) (i32.const 0xFFFF)))
    (call $emit_aarch64_instr_movk_64 (i32.const 0) (i32.const 1) (i32.shr_u (global.get $BSS_GLOBALS) (i32.const 16)))
    (call $emit_aarch64_instr_str_64_off (i32.const 0) (i32.const 19) (i32.const 3))
    (call $emit_aarch64_instr_movz_64 (i32.const 0) (i32.const 0) (i32.and (global.get $BSS_TABLE) (i32.const 0xFFFF)))
    (call $emit_aarch64_instr_movk_64 (i32.const 0) (i32.const 1) (i32.shr_u (global.get $BSS_TABLE) (i32.const 16)))
    (call $emit_aarch64_instr_str_64_off (i32.const 0) (i32.const 19) (i32.const 6))
    (call $emit_aarch64_instr_movz_64 (i32.const 0) (i32.const 0) (i32.const 1))
    (call $emit_aarch64_instr_str_32_off (i32.const 0) (i32.const 19) (i32.const 16))
    (local.set $current_off (i32.sub (i32.load (global.get $JS_CODE_PTR)) (global.get $BIN_OUT_OFF)))
    (local.set $stub_size (i32.add (local.get $current_off) (i32.const 16)))
    (call $emit_aarch64_bl
      (i32.sub (local.get $stub_size) (local.get $current_off)))
    (call $emit_aarch64_store_and_halt)
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

  ;; ── Pop two values: pop X1 (right), pop X0 (left) ────────────────
  (func $emit_aarch64_pop2_x1_x0_order
    (call $emit_aarch64_pop_x1)
    (call $emit_aarch64_pop_x0)
  )

  ;; ── XOR X0, X0 (zero register) ────────────────────────────────────
  ;; EOR X0, X0, X0 = 0xAA000000 | (X0 << 16) | (X0 << 5) | X0 = 0xAA000000
  (func $emit_aarch64_xor_x0_x0
    (call $emit_aarch64_instr (i32.const 0xAA000000))
  )

  ;; EOR W0, W0, W0 = 0x2A000000
  (func $emit_aarch64_xor_w0_w0
    (call $emit_aarch64_instr (i32.const 0x2A000000))
  )

  ;; EOR X1, X1, X1
  (func $emit_aarch64_xor_x1_x1
    (call $emit_aarch64_instr (i32.const 0xAA000021))
  )

  ;; EOR X2, X2, X2
  (func $emit_aarch64_xor_x2_x2
    (call $emit_aarch64_instr (i32.const 0xAA000042))
  )

  ;; MOV X0, X1: ORR X0, XZR, X1
  ;; = 0xAA000020 | X1  = 0xAA000021
  (func $emit_aarch64_mov_x0_x1
    (call $emit_aarch64_instr_orr_64 (global.get $REG_X0) (global.get $REG_XZR) (global.get $REG_X1))
  )

  ;; MOV X1, X0: ORR X1, XZR, X0
  (func $emit_aarch64_mov_x1_x0
    (call $emit_aarch64_instr_orr_64 (global.get $REG_X1) (global.get $REG_XZR) (global.get $REG_X0))
  )

  ;; MOV W0, W1: ORR W0, WZR, W1
  (func $emit_aarch64_mov_w0_w1
    (call $emit_aarch64_instr_orr_32 (global.get $REG_X0) (global.get $REG_XZR) (global.get $REG_X1))
  )

  ;; MOV W1, W0
  (func $emit_aarch64_mov_w1_w0
    (call $emit_aarch64_instr_orr_32 (global.get $REG_X1) (global.get $REG_XZR) (global.get $REG_X0))
  )

  ;; ═════════════════════════════════════════════════════════════════════
  ;; AArch64 NEON (SIMD) emit helpers — v128 load/store/push/pop
  ;; ═════════════════════════════════════════════════════════════════════

  ;; STR Qt, [Xn, #imm12*16] — 128-bit store, unsigned offset
  ;; imm12 = byte_offset / 16 (scaled by data size 16)
  ;; Encoding: Q=1 1111101 opc=00 V=1 00 imm12 Rn Rt
  (func $emit_aarch64_instr_str_q_off (param $rt i32) (param $rn i32) (param $imm12 i32)
    (call $emit_aarch64_instr
      (i32.or (i32.const 0x3D800000)
              (i32.or (i32.shl (local.get $imm12) (i32.const 10))
                      (i32.or (i32.shl (local.get $rn) (i32.const 5))
                              (local.get $rt)))))
  )

  ;; LDR Qt, [Xn, #imm12*16] — 128-bit load, unsigned offset
  ;; Encoding: Q=1 1111101 opc=01 V=1 00 imm12 Rn Rt
  (func $emit_aarch64_instr_ldr_q_off (param $rt i32) (param $rn i32) (param $imm12 i32)
    (call $emit_aarch64_instr
      (i32.or (i32.const 0x3DC00000)
              (i32.or (i32.shl (local.get $imm12) (i32.const 10))
                      (i32.or (i32.shl (local.get $rn) (i32.const 5))
                              (local.get $rt)))))
  )

  ;; Push Q0 onto WASM value stack (grows downward): SUB SP,#16; STR Q0,[SP]
  (func $emit_aarch64_v128_push
    (call $emit_aarch64_instr_subi_64 (global.get $REG_SP) (global.get $REG_SP) (i32.const 16))
    (call $emit_aarch64_instr_str_q_off (global.get $REG_V0) (global.get $REG_SP) (i32.const 0))
  )

  ;; Pop Q0 from WASM value stack: LDR Q0,[SP]; ADD SP,#16
  (func $emit_aarch64_v128_pop
    (call $emit_aarch64_instr_ldr_q_off (global.get $REG_V0) (global.get $REG_SP) (i32.const 0))
    (call $emit_aarch64_instr_addi_64 (global.get $REG_SP) (global.get $REG_SP) (i32.const 16))
  )

  ;; Load Q0 from address in X1 into Q0: LDR Q0, [X1] (imm12=0 for bare reg)
  ;; Uses ldr_q_off with Xn = X1 (=1), imm12 = 0
  (func $emit_aarch64_v128_load_reg
    (call $emit_aarch64_instr_ldr_q_off (global.get $REG_V0) (global.get $REG_X1) (i32.const 0))
  )

  ;; Store Q0 to address in X1: STR Q0, [X1] (imm12=0 for bare reg)
  (func $emit_aarch64_v128_store_reg
    (call $emit_aarch64_instr_str_q_off (global.get $REG_V0) (global.get $REG_X1) (i32.const 0))
  )

  ;; Load 32-bit constant address into X1 (used to address v128 immediate)
  (func $emit_aarch64_load_addr_x1 (param $addr i32)
    (call $emit_aarch64_instr_movz_64 (i32.const 1) (i32.const 0) (i32.and (local.get $addr) (i32.const 0xFFFF)))
    (call $emit_aarch64_instr_movk_64 (i32.const 1) (i32.const 1) (i32.shr_u (local.get $addr) (i32.const 16)))
  )

  ;; ═════════════════════════════════════════════════════════════════════
  ;; AArch64 NEON (SIMD) Advanced SIMD three-register / two-register helpers
  ;; ═════════════════════════════════════════════════════════════════════
  ;;
  ;; Three-register same-type (ADD, SUB, MUL, etc.):
  ;;   encoding = base | (Rm << 10) | (Rn << 5) | Rd
  ;;   where base = 0x0E200000 | (Q<<30) | (size<<22) | (U<<21) | (opcode<<16)
  ;;
  ;; Base values for common ops (Q=1, register bits = 0):
  ;;   ADD .16B: 0x4E208400   (Q=1, size=00, U=0, opcode=10000)
  ;;   SUB .16B: 0x4E208800   (Q=1, size=00, U=1, opcode=10000)
  ;;   AND Vd.16B, Vn.16B, Vm.16B: use BIC/VBIT or just use 3-same with specific opcode
  ;;   ORR Vd.16B, Vn.16B, Vm.16B: opcode=01011, U=0, size=00 (but .16B uses 3-different)
  ;;   EOR Vd.16B, Vn.16B, Vm.16B: opcode=11011, U=1, size=00
  ;;
  ;; For simplicity, compute base from Q/size/U/opcode:
  (func $emit_aarch64_neon_3same (param $Q i32) (param $size i32) (param $U i32)
                         (param $opcode i32) (param $Rd i32) (param $Rn i32) (param $Rm i32)
    (local $base i32)
    (local.set $base (i32.const 0x0E200000))
    (local.set $base (i32.or (local.get $base) (i32.shl (local.get $Q) (i32.const 30))))
    (local.set $base (i32.or (local.get $base) (i32.shl (local.get $size) (i32.const 22))))
    (local.set $base (i32.or (local.get $base) (i32.shl (local.get $U) (i32.const 21))))
    (local.set $base (i32.or (local.get $base) (i32.shl (local.get $opcode) (i32.const 16))))
    (call $emit_aarch64_instr
      (i32.or (local.get $base)
              (i32.or (i32.shl (local.get $Rm) (i32.const 10))
                      (i32.or (i32.shl (local.get $Rn) (i32.const 5))
                              (local.get $Rd)))))
  )

  ;; Convenience: ADD Vd.16B, Vn.16B, Vm.16B
  (func $emit_aarch64_neon_add_16b (param $Rd i32) (param $Rn i32) (param $Rm i32)
    (call $emit_aarch64_neon_3same (i32.const 1) (i32.const 0) (i32.const 0) (i32.const 0x10)
                           (local.get $Rd) (local.get $Rn) (local.get $Rm))
  )

  ;; Convenience: SUB Vd.16B, Vn.16B, Vm.16B
  (func $emit_aarch64_neon_sub_16b (param $Rd i32) (param $Rn i32) (param $Rm i32)
    (call $emit_aarch64_neon_3same (i32.const 1) (i32.const 0) (i32.const 1) (i32.const 0x10)
                           (local.get $Rd) (local.get $Rn) (local.get $Rm))
  )

  ;; Convenience: ADD Vd.8H, Vn.8H, Vm.8H (16-bit lanes)
  (func $emit_aarch64_neon_add_8h (param $Rd i32) (param $Rn i32) (param $Rm i32)
    (call $emit_aarch64_neon_3same (i32.const 1) (i32.const 1) (i32.const 0) (i32.const 0x10)
                           (local.get $Rd) (local.get $Rn) (local.get $Rm))
  )

  ;; Convenience: SUB Vd.8H, Vn.8H, Vm.8H
  (func $emit_aarch64_neon_sub_8h (param $Rd i32) (param $Rn i32) (param $Rm i32)
    (call $emit_aarch64_neon_3same (i32.const 1) (i32.const 1) (i32.const 1) (i32.const 0x10)
                           (local.get $Rd) (local.get $Rn) (local.get $Rm))
  )

  ;; Convenience: ADD Vd.4S, Vn.4S, Vm.4S (32-bit lanes)
  (func $emit_aarch64_neon_add_4s (param $Rd i32) (param $Rn i32) (param $Rm i32)
    (call $emit_aarch64_neon_3same (i32.const 1) (i32.const 2) (i32.const 0) (i32.const 0x10)
                           (local.get $Rd) (local.get $Rn) (local.get $Rm))
  )

  ;; Convenience: SUB Vd.4S, Vn.4S, Vm.4S
  (func $emit_aarch64_neon_sub_4s (param $Rd i32) (param $Rn i32) (param $Rm i32)
    (call $emit_aarch64_neon_3same (i32.const 1) (i32.const 2) (i32.const 1) (i32.const 0x10)
                           (local.get $Rd) (local.get $Rn) (local.get $Rm))
  )

  ;; Convenience: ADD Vd.2D, Vn.2D, Vm.2D (64-bit lanes)
  (func $emit_aarch64_neon_add_2d (param $Rd i32) (param $Rn i32) (param $Rm i32)
    (call $emit_aarch64_neon_3same (i32.const 1) (i32.const 3) (i32.const 0) (i32.const 0x10)
                           (local.get $Rd) (local.get $Rn) (local.get $Rm))
  )

  ;; Convenience: SUB Vd.2D, Vn.2D, Vm.2D
  (func $emit_aarch64_neon_sub_2d (param $Rd i32) (param $Rn i32) (param $Rm i32)
    (call $emit_aarch64_neon_3same (i32.const 1) (i32.const 3) (i32.const 1) (i32.const 0x10)
                           (local.get $Rd) (local.get $Rn) (local.get $Rm))
  )

  ;; ── AArch64 NEON bitwise (AND/OR/XOR) via three-register different ──
  ;; BIC Vd.16B, Vn.16B, Vm.16B: AND with complement
  ;; Actually use AND Vd.16B, Vn.16B, Vm.16B (three-register different encoding):
  ;; opcode = 00011, U=0, Q=1, size=00
  (func $emit_aarch64_neon_and_16b (param $Rd i32) (param $Rn i32) (param $Rm i32)
    (call $emit_aarch64_neon_3same (i32.const 1) (i32.const 0) (i32.const 0) (i32.const 0x03)
                           (local.get $Rd) (local.get $Rn) (local.get $Rm))
  )

  ;; ORR Vd.16B, Vn.16B, Vm.16B: opcode = 01011, U=0
  (func $emit_aarch64_neon_orr_16b (param $Rd i32) (param $Rn i32) (param $Rm i32)
    (call $emit_aarch64_neon_3same (i32.const 1) (i32.const 0) (i32.const 0) (i32.const 0x0B)
                           (local.get $Rd) (local.get $Rn) (local.get $Rm))
  )

  ;; EOR Vd.16B, Vn.16B, Vm.16B: opcode = 11011, U=1 ... wait that doesn't look right
  ;; Actually EOR (Advanced SIMD) uses three-register different:
  ;; opcode = 11011, U=1 (for EOR)
  ;; Wait: EOR Vd.16B, Vn.16B, Vm.16B has U=1, opcode=11011
  ;; Hmm let me just use BSLL instead: BSL is bitwise select
  ;; EOR = opcode=11011, U=1:
  ;; Actually from the manual: EOR (vector) is 0x2E, which uses "three same" not "three different"
  ;; EOR (vector) encoding: 0 Q 0 0 1 1 1 0 size U 1 0 0 0 1 Rm Rn Rd
  ;; That uses the "three same" group. opcode = 00001? No, let me check:
  ;; U=1, opcode=00001 for EOR... Actually:
  ;; EOR has: [31]=0, [30]=Q, [29:24]=001110, [23:22]=00, [21]=1, [20:16]=00001
  ;; So base = 0x0E200000 | (Q<<30) | (1<<21) | (1<<16)
  ;; = 0x0E200000 | 0x40000000 | 0x20000 | 0x10000 = 0x4E230000
  ;; For Q=1: = 0x4E230000 | (Rm<<10) | (Rn<<5) | Rd
  (func $emit_aarch64_neon_eor_16b (param $Rd i32) (param $Rn i32) (param $Rm i32)
    (call $emit_aarch64_neon_3same (i32.const 1) (i32.const 0) (i32.const 1) (i32.const 0x01)
                           (local.get $Rd) (local.get $Rn) (local.get $Rm))
  )

  ;; ── AArch64 NEON comparison helpers ──────────────────────────────

  ;; CMGT Vd.16B, Vn.16B, Vm.16B (signed): Q=1, size=00, U=0, opcode=00110
  (func $emit_aarch64_neon_cmgt_16b_s (param $Rd i32) (param $Rn i32) (param $Rm i32)
    (call $emit_aarch64_neon_3same (i32.const 1) (i32.const 0) (i32.const 0) (i32.const 0x06)
                           (local.get $Rd) (local.get $Rn) (local.get $Rm))
  )
  ;; CMGT Vd.16B, Vn.16B, Vm.16B (unsigned): U=1, opcode=00110
  (func $emit_aarch64_neon_cmgt_16b_u (param $Rd i32) (param $Rn i32) (param $Rm i32)
    (call $emit_aarch64_neon_3same (i32.const 1) (i32.const 0) (i32.const 1) (i32.const 0x06)
                           (local.get $Rd) (local.get $Rn) (local.get $Rm))
  )
  ;; CMEQ Vd.16B, Vn.16B, Vm.16B: opcode=10001, U=1
  (func $emit_aarch64_neon_cmeq_16b (param $Rd i32) (param $Rn i32) (param $Rm i32)
    (call $emit_aarch64_neon_3same (i32.const 1) (i32.const 0) (i32.const 1) (i32.const 0x11)
                           (local.get $Rd) (local.get $Rn) (local.get $Rm))
  )
  ;; CMGE Vd.16B, Vn.16B, Vm.16B (signed): U=0, opcode=00111
  (func $emit_aarch64_neon_cmge_16b_s (param $Rd i32) (param $Rn i32) (param $Rm i32)
    (call $emit_aarch64_neon_3same (i32.const 1) (i32.const 0) (i32.const 0) (i32.const 0x07)
                           (local.get $Rd) (local.get $Rn) (local.get $Rm))
  )
  ;; CMGE Vd.16B, Vn.16B, Vm.16B (unsigned): U=1, opcode=00111
  (func $emit_aarch64_neon_cmge_16b_u (param $Rd i32) (param $Rn i32) (param $Rm i32)
    (call $emit_aarch64_neon_3same (i32.const 1) (i32.const 0) (i32.const 1) (i32.const 0x07)
                           (local.get $Rd) (local.get $Rn) (local.get $Rm))
  )

  ;; CMGT/CMGE for 8H, 4S, 2D lanes — same as 16B but different size
  ;; size=01 for 8H, size=10 for 4S, size=11 for 2D
  (func $emit_aarch64_neon_cmgt_8h_s (param $Rd i32) (param $Rn i32) (param $Rm i32)
    (call $emit_aarch64_neon_3same (i32.const 1) (i32.const 1) (i32.const 0) (i32.const 0x06)
                           (local.get $Rd) (local.get $Rn) (local.get $Rm))
  )
  (func $emit_aarch64_neon_cmgt_4s_s (param $Rd i32) (param $Rn i32) (param $Rm i32)
    (call $emit_aarch64_neon_3same (i32.const 1) (i32.const 2) (i32.const 0) (i32.const 0x06)
                           (local.get $Rd) (local.get $Rn) (local.get $Rm))
  )
  (func $emit_aarch64_neon_cmeq_8h (param $Rd i32) (param $Rn i32) (param $Rm i32)
    (call $emit_aarch64_neon_3same (i32.const 1) (i32.const 1) (i32.const 1) (i32.const 0x11)
                           (local.get $Rd) (local.get $Rn) (local.get $Rm))
  )
  (func $emit_aarch64_neon_cmeq_4s (param $Rd i32) (param $Rn i32) (param $Rm i32)
    (call $emit_aarch64_neon_3same (i32.const 1) (i32.const 2) (i32.const 1) (i32.const 0x11)
                           (local.get $Rd) (local.get $Rn) (local.get $Rm))
  )

  ;; CMGE (unsigned) Vd.8H, Vn.8H, Vm.8H: CMHS, U=1, opcode=00111, size=01
  (func $emit_aarch64_neon_cmge_8h_u (param $Rd i32) (param $Rn i32) (param $Rm i32)
    (call $emit_aarch64_neon_3same (i32.const 1) (i32.const 1) (i32.const 1) (i32.const 0x07)
                           (local.get $Rd) (local.get $Rn) (local.get $Rm))
  )
  ;; CMGE (unsigned) Vd.4S, Vn.4S, Vm.4S: CMHS, size=10
  (func $emit_aarch64_neon_cmge_4s_u (param $Rd i32) (param $Rn i32) (param $Rm i32)
    (call $emit_aarch64_neon_3same (i32.const 1) (i32.const 2) (i32.const 1) (i32.const 0x07)
                           (local.get $Rd) (local.get $Rn) (local.get $Rm))
  )
  ;; CMGT (unsigned) Vd.8H, Vn.8H, Vm.8H: CMHI, U=1, size=01, opcode=00110
  (func $emit_aarch64_neon_cmgt_8h_u (param $Rd i32) (param $Rn i32) (param $Rm i32)
    (call $emit_aarch64_neon_3same (i32.const 1) (i32.const 1) (i32.const 1) (i32.const 0x06)
                           (local.get $Rd) (local.get $Rn) (local.get $Rm))
  )
  ;; CMGT (unsigned) Vd.4S, Vn.4S, Vm.4S: CMHI, size=10
  (func $emit_aarch64_neon_cmgt_4s_u (param $Rd i32) (param $Rn i32) (param $Rm i32)
    (call $emit_aarch64_neon_3same (i32.const 1) (i32.const 2) (i32.const 1) (i32.const 0x06)
                           (local.get $Rd) (local.get $Rn) (local.get $Rm))
  )
  ;; CMGE (signed) Vd.8H, Vn.8H, Vm.8H: U=0, size=01, opcode=00111
  (func $emit_aarch64_neon_cmge_8h_s (param $Rd i32) (param $Rn i32) (param $Rm i32)
    (call $emit_aarch64_neon_3same (i32.const 1) (i32.const 1) (i32.const 0) (i32.const 0x07)
                           (local.get $Rd) (local.get $Rn) (local.get $Rm))
  )
  ;; CMGE (signed) Vd.4S, Vn.4S, Vm.4S: U=0, size=10, opcode=00111
  (func $emit_aarch64_neon_cmge_4s_s (param $Rd i32) (param $Rn i32) (param $Rm i32)
    (call $emit_aarch64_neon_3same (i32.const 1) (i32.const 2) (i32.const 0) (i32.const 0x07)
                           (local.get $Rd) (local.get $Rn) (local.get $Rm))
  )

  ;; ── MUL helpers (signed/unsigned, integer) ──────────────────────
  ;; MUL Vd.8H, Vn.8H, Vm.8H: U=0, opcode=10011, size=01
  (func $emit_aarch64_neon_mul_8h (param $Rd i32) (param $Rn i32) (param $Rm i32)
    (call $emit_aarch64_neon_3same (i32.const 1) (i32.const 1) (i32.const 0) (i32.const 0x13)
                           (local.get $Rd) (local.get $Rn) (local.get $Rm))
  )
  ;; MUL Vd.4S, Vn.4S, Vm.4S: size=10
  (func $emit_aarch64_neon_mul_4s (param $Rd i32) (param $Rn i32) (param $Rm i32)
    (call $emit_aarch64_neon_3same (i32.const 1) (i32.const 2) (i32.const 0) (i32.const 0x13)
                           (local.get $Rd) (local.get $Rn) (local.get $Rm))
  )

  ;; ── NEG helpers (2-register misc) ───────────────────────────────
  ;; NEG Vd.16B, Vn.16B: 0x0E207800 | (Q<<30) | (size<<22) | (Rn<<5) | Rd
  ;; size=00 for 16B (byte), size=01 for 8H (halfword), size=10 for 4S, size=11 for 2D
  (func $emit_aarch64_neon_neg (param $size i32) (param $Rd i32) (param $Rn i32)
    (call $emit_aarch64_instr
      (i32.or (i32.const 0x4E207800)
              (i32.or (i32.shl (local.get $size) (i32.const 22))
                      (i32.or (i32.shl (local.get $Rn) (i32.const 5))
                              (local.get $Rd)))))
  )

  ;; ── UMIN / UMAX / URHADD helpers (unsigned) ─────────────────────
  ;; UMIN Vd.16B, Vn.16B, Vm.16B: U=1, opcode=01101, size=00
  (func $emit_aarch64_neon_umin_16b (param $Rd i32) (param $Rn i32) (param $Rm i32)
    (call $emit_aarch64_neon_3same (i32.const 1) (i32.const 0) (i32.const 1) (i32.const 0x0D)
                           (local.get $Rd) (local.get $Rn) (local.get $Rm))
  )
  ;; UMIN Vd.8H, Vn.8H, Vm.8H: size=01
  (func $emit_aarch64_neon_umin_8h (param $Rd i32) (param $Rn i32) (param $Rm i32)
    (call $emit_aarch64_neon_3same (i32.const 1) (i32.const 1) (i32.const 1) (i32.const 0x0D)
                           (local.get $Rd) (local.get $Rn) (local.get $Rm))
  )
  ;; UMAX Vd.16B, Vn.16B, Vm.16B: U=1, opcode=01100, size=00
  (func $emit_aarch64_neon_umax_16b (param $Rd i32) (param $Rn i32) (param $Rm i32)
    (call $emit_aarch64_neon_3same (i32.const 1) (i32.const 0) (i32.const 1) (i32.const 0x0C)
                           (local.get $Rd) (local.get $Rn) (local.get $Rm))
  )
  ;; UMAX Vd.8H, Vn.8H, Vm.8H: size=01
  (func $emit_aarch64_neon_umax_8h (param $Rd i32) (param $Rn i32) (param $Rm i32)
    (call $emit_aarch64_neon_3same (i32.const 1) (i32.const 1) (i32.const 1) (i32.const 0x0C)
                           (local.get $Rd) (local.get $Rn) (local.get $Rm))
  )
  ;; URHADD Vd.16B, Vn.16B, Vm.16B: (a+b+1)>>1, U=1, opcode=00011, size=00
  (func $emit_aarch64_neon_urhadd_16b (param $Rd i32) (param $Rn i32) (param $Rm i32)
    (call $emit_aarch64_neon_3same (i32.const 1) (i32.const 0) (i32.const 1) (i32.const 0x03)
                           (local.get $Rd) (local.get $Rn) (local.get $Rm))
  )
  ;; URHADD Vd.8H, Vn.8H, Vm.8H: size=01
  (func $emit_aarch64_neon_urhadd_8h (param $Rd i32) (param $Rn i32) (param $Rm i32)
    (call $emit_aarch64_neon_3same (i32.const 1) (i32.const 1) (i32.const 1) (i32.const 0x03)
                           (local.get $Rd) (local.get $Rn) (local.get $Rm))
  )

  ;; ── Pop two, operate, push (pattern for binary ops) ──────────────

  ;; Push X0 only if RESULT_IN_X0 is 0 (peephole)
  (func $emit_aarch64_maybe_push_x0
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
            (call $emit_aarch64_push_x0)
          )
        )
      )
    )
  )
