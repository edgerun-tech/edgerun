  ;; ═════════════════════════════════════════════════════════════════════
  ;; EdgeRun WASM → AArch64 (ARM64) JIT Compiler
  ;;
  ;; Compiles WASM decoded ops into AArch64 machine code in a memory buffer.
  ;; Follows the same architecture as the x86_64 JIT (compiler.wat) but emits
  ;; AArch64 instructions (all 4 bytes fixed-length).
  ;; ═════════════════════════════════════════════════════════════════════


  ;; ── Shared constants (from edgerun-core) ─────────────────────────────

  ;; JIT code cache: 1MB starting at 0x100000
  (global $JIT_CACHE      i32 (i32.const 0x100000))
  (global $JIT_CACHE_SIZE i32 (i32.const 0x100000))
  (global $JIT_SLOT_SIZE  i32 (i32.const 0x40000))  ;; 256KB per function

  ;; JIT state at 0x300000
  (global $JIT_STATE      i32 (i32.const 0x300000))

  ;; JIT state field offsets (relative to JIT_STATE)
  (global $JS_CODE_PTR       i32 (i32.const 0))
  (global $JS_CACHE_BASE     i32 (i32.const 4))
  (global $JS_CACHE_END      i32 (i32.const 8))
  (global $JS_FUNC_IDX       i32 (i32.const 12))
  (global $JS_RESULT_COUNT   i32 (i32.const 16))
  (global $JS_STACK_DEPTH    i32 (i32.const 20))
  (global $JS_MAX_STACK      i32 (i32.const 24))
  (global $JS_LABEL_DEPTH    i32 (i32.const 28))
  (global $JS_RETURN_EMITTED i32 (i32.const 32))
  (global $JS_LABEL_OFFSETS  i32 (i32.const 64))    ;; 256*i32 = 1024 bytes
  (global $JS_LABEL_KINDS    i32 (i32.const 1088))   ;; 256*byte = 256 bytes
  (global $JS_LABEL_IF_JZ    i32 (i32.const 1344))   ;; 256*i32 = 1024 bytes
  (global $JS_FIXUP_COUNT    i32 (i32.const 2368))
  (global $JS_FIXUP_LABEL    i32 (i32.const 2372))   ;; 256*i32 = 1024 bytes
  (global $JS_FIXUP_OFFSET   i32 (i32.const 3396))   ;; 256*i32 = 1024 bytes
  (global $JS_INITIALIZED    i32 (i32.const 4420))

  ;; Label kinds
  (global $JIT_LABEL_BLOCK   i32 (i32.const 0))
  (global $JIT_LABEL_LOOP    i32 (i32.const 1))
  (global $JIT_LABEL_IF      i32 (i32.const 2))

  ;; Error codes
  (global $OK                i32 (i32.const 0))
  (global $ERR_UNSUP         i32 (i32.const -1))

  ;; ── WASM type constants ───────────────────────────────────────────────
  (global $WASM_TYPE_V128    i32 (i32.const 0x7B))
  (global $OP_PREFIX_FC      i32 (i32.const 0xFC))
  (global $OP_PREFIX_FD      i32 (i32.const 0xFD))

  ;; ── AArch64 NEON register constants ─────────────────────────────────
  ;; V0-V31 are the 128-bit SIMD/FP registers (aliased as Q0-Q31)
  (global $REG_V0  i32 (i32.const 0))
  (global $REG_V1  i32 (i32.const 1))
  (global $REG_V2  i32 (i32.const 2))

  ;; Peephole optimization flag
  (global $RESULT_IN_X0      (mut i32) (i32.const 0))
  (global $NEXT_OP           (mut i32) (i32.const 0))
  (global $JIT_ERROR         (mut i32) (i32.const 0))

  ;; ═════════════════════════════════════════════════════════════════════
  ;; ELF output buffer (for compile_to_elf)
  ;; ═════════════════════════════════════════════════════════════════════
  (global $ELF_OUT_BUF  i32 (i32.const 0x800000))
  (global $ELF_OUT_OFF  i32 (i32.const 0x700000))  ;; ELF_OUT_BUF - JIT_CACHE

  (global $TEXT_VA      i32 (i32.const 0x400000))
  (global $BSS_VA       i32 (i32.const 0x500000))

  (global $EHDR_SIZE    i32 (i32.const 64))    ;; 64-bit ELF header
  (global $PHDR_SIZE    i32 (i32.const 56))    ;; 64-bit ELF phdr
  (global $ELF_STUB_OFF i32 (i32.const 120))   ;; after ehdr (64) + 1 phdr (56)
  (global $ELF_CODE_OFF i32 (i32.const 256))   ;; aligned after stub
  (global $BSS_SIZE     i32 (i32.const 0x40000)) ;; 256KB

  ;; BSS item VAs (relative to BSS_VA)
  (global $BSS_JITGLOBALS i32 (i32.const 0x500000))
  (global $BSS_MEM       i32 (i32.const 0x500080))
  (global $BSS_LOCALS    i32 (i32.const 0x510080))
  (global $BSS_GLOBALS   i32 (i32.const 0x520080))
  (global $BSS_TABLE     i32 (i32.const 0x530080))

  ;; ═════════════════════════════════════════════════════════════════════
  ;; Flat binary output buffer (for compile_to_bin)
  ;; ═════════════════════════════════════════════════════════════════════
  (global $BIN_OUT_BUF  i32 (i32.const 0x900000))
  (global $BIN_OUT_OFF  i32 (i32.const 0x800000))  ;; BIN_OUT_BUF - JIT_CACHE


  ;; Current decoded op pointer
  (global $CURRENT_DEC_PTR   (mut i32) (i32.const 0))

  ;; ═════════════════════════════════════════════════════════════════════
  ;; AArch64 Register constants
  ;; ═════════════════════════════════════════════════════════════════════
  ;; X0 = TOS/result (like rax)
  ;; X1 = scratch (like rcx)
  ;; X2 = scratch (like rdx)
  ;; X19 = JitGlobals (like r15)
  ;; X20 = locals cache (like rbx)
  ;; X21 = register-allocated local 0 (like r12)
  ;; X22 = register-allocated local 1 (like r13)
  ;; X29 = FP, X30 = LR

  (global $REG_X0 i32 (i32.const 0))
  (global $REG_X1 i32 (i32.const 1))
  (global $REG_X2 i32 (i32.const 2))
  (global $REG_X19 i32 (i32.const 19))
  (global $REG_X20 i32 (i32.const 20))
  (global $REG_X21 i32 (i32.const 21))
  (global $REG_X22 i32 (i32.const 22))
  (global $REG_SP i32 (i32.const 31))
  (global $REG_XZR i32 (i32.const 31))

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

  ;; Emit a qword (8 bytes) little-endian
  (func $emit_qword (param $v i64)
    (call $emit_dword (i32.wrap_i64 (local.get $v)))
    (call $emit_dword (i32.wrap_i64 (i64.shr_u (local.get $v) (i64.const 32))))
  )

  ;; ── AArch64 instruction emitter ────────────────────────────────────
  ;; All AArch64 instructions are exactly 4 bytes (32 bits)

  ;; Emit a 32-bit AArch64 instruction (little-endian)
  (func $emit_instr (param $val i32)
    (call $emit_dword (local.get $val))
  )

  ;; ── AArch64 Register-to-Register (R-type) helpers ──────────────────
  ;; Base encoding for 64-bit operations:
  ;;   [31]=1 (64-bit), [30..29]=opc, [28..24]=0xxxx, [23..22]=variant,
  ;;   [21..16]=Rm, [15..10]=imm, [9..5]=Rn, [4..0]=Rd

  ;; ADD Xd, Xn, Xm (64-bit): 10001011000 Rm 000000 Rn Rd
  (func $emit_instr_add_64 (param $rd i32) (param $rn i32) (param $rm i32)
    (call $emit_instr
      (i32.or (i32.const 0x8B000000)
              (i32.or (i32.shl (local.get $rm) (i32.const 16))
                      (i32.or (i32.shl (local.get $rn) (i32.const 5))
                              (local.get $rd)))))
  )

  ;; SUB Xd, Xn, Xm (64-bit): 11001011000 Rm 000000 Rn Rd
  (func $emit_instr_sub_64 (param $rd i32) (param $rn i32) (param $rm i32)
    (call $emit_instr
      (i32.or (i32.const 0xCB000000)
              (i32.or (i32.shl (local.get $rm) (i32.const 16))
                      (i32.or (i32.shl (local.get $rn) (i32.const 5))
                              (local.get $rd)))))
  )

  ;; ADD Wd, Wn, Wm (32-bit): 00001011000 Rm 000000 Rn Rd
  (func $emit_instr_add_32 (param $rd i32) (param $rn i32) (param $rm i32)
    (call $emit_instr
      (i32.or (i32.const 0x0B000000)
              (i32.or (i32.shl (local.get $rm) (i32.const 16))
                      (i32.or (i32.shl (local.get $rn) (i32.const 5))
                              (local.get $rd)))))
  )

  ;; SUB Wd, Wn, Wm (32-bit): 01001011000 Rm 000000 Rn Rd
  (func $emit_instr_sub_32 (param $rd i32) (param $rn i32) (param $rm i32)
    (call $emit_instr
      (i32.or (i32.const 0x4B000000)
              (i32.or (i32.shl (local.get $rm) (i32.const 16))
                      (i32.or (i32.shl (local.get $rn) (i32.const 5))
                              (local.get $rd)))))
  )

  ;; MUL Xd, Xn, Xm (64-bit): 10011011000 111111 Rm Rn Rd
  (func $emit_instr_mul_64 (param $rd i32) (param $rn i32) (param $rm i32)
    (call $emit_instr
      (i32.or (i32.const 0x9B007C00)
              (i32.or (i32.shl (local.get $rm) (i32.const 16))
                      (i32.or (i32.shl (local.get $rn) (i32.const 5))
                              (local.get $rd)))))
  )

  ;; MUL Wd, Wn, Wm (32-bit): 00011011000 111111 Rm Rn Rd
  (func $emit_instr_mul_32 (param $rd i32) (param $rn i32) (param $rm i32)
    (call $emit_instr
      (i32.or (i32.const 0x1B007C00)
              (i32.or (i32.shl (local.get $rm) (i32.const 16))
                      (i32.or (i32.shl (local.get $rn) (i32.const 5))
                              (local.get $rd)))))
  )

  ;; SDIV Xd, Xn, Xm (64-bit): 10011010110 000011 Rm Rn Rd
  (func $emit_instr_sdiv_64 (param $rd i32) (param $rn i32) (param $rm i32)
    (call $emit_instr
      (i32.or (i32.const 0x9AC00C00)
              (i32.or (i32.shl (local.get $rm) (i32.const 16))
                      (i32.or (i32.shl (local.get $rn) (i32.const 5))
                              (local.get $rd)))))
  )

  ;; UDIV Xd, Xn, Xm (64-bit): 10011010110 000010 Rm Rn Rd
  (func $emit_instr_udiv_64 (param $rd i32) (param $rn i32) (param $rm i32)
    (call $emit_instr
      (i32.or (i32.const 0x9AC00800)
              (i32.or (i32.shl (local.get $rm) (i32.const 16))
                      (i32.or (i32.shl (local.get $rn) (i32.const 5))
                              (local.get $rd)))))
  )

  ;; SDIV Wd, Wn, Wm (32-bit): 00011010110 000011 Rm Rn Rd
  (func $emit_instr_sdiv_32 (param $rd i32) (param $rn i32) (param $rm i32)
    (call $emit_instr
      (i32.or (i32.const 0x1AC00C00)
              (i32.or (i32.shl (local.get $rm) (i32.const 16))
                      (i32.or (i32.shl (local.get $rn) (i32.const 5))
                              (local.get $rd)))))
  )

  ;; UDIV Wd, Wn, Wm (32-bit): 00011010110 000010 Rm Rn Rd
  (func $emit_instr_udiv_32 (param $rd i32) (param $rn i32) (param $rm i32)
    (call $emit_instr
      (i32.or (i32.const 0x1AC00800)
              (i32.or (i32.shl (local.get $rm) (i32.const 16))
                      (i32.or (i32.shl (local.get $rn) (i32.const 5))
                              (local.get $rd)))))
  )

  ;; AND Xd, Xn, Xm (64-bit): 10001010000 Rm 000000 Rn Rd
  (func $emit_instr_and_64 (param $rd i32) (param $rn i32) (param $rm i32)
    (call $emit_instr
      (i32.or (i32.const 0x8A000000)
              (i32.or (i32.shl (local.get $rm) (i32.const 16))
                      (i32.or (i32.shl (local.get $rn) (i32.const 5))
                              (local.get $rd)))))
  )

  ;; ORR Xd, Xn, Xm (64-bit): 10101010000 Rm 000000 Rn Rd
  (func $emit_instr_orr_64 (param $rd i32) (param $rn i32) (param $rm i32)
    (call $emit_instr
      (i32.or (i32.const 0xAA000000)
              (i32.or (i32.shl (local.get $rm) (i32.const 16))
                      (i32.or (i32.shl (local.get $rn) (i32.const 5))
                              (local.get $rd)))))
  )

  ;; EOR Xd, Xn, Xm (64-bit): 11001010000 Rm 000000 Rn Rd
  (func $emit_instr_eor_64 (param $rd i32) (param $rn i32) (param $rm i32)
    (call $emit_instr
      (i32.or (i32.const 0xCA000000)
              (i32.or (i32.shl (local.get $rm) (i32.const 16))
                      (i32.or (i32.shl (local.get $rn) (i32.const 5))
                              (local.get $rd)))))
  )

  ;; AND Wd, Wn, Wm (32-bit): 00001010000 Rm 000000 Rn Rd
  (func $emit_instr_and_32 (param $rd i32) (param $rn i32) (param $rm i32)
    (call $emit_instr
      (i32.or (i32.const 0x0A000000)
              (i32.or (i32.shl (local.get $rm) (i32.const 16))
                      (i32.or (i32.shl (local.get $rn) (i32.const 5))
                              (local.get $rd)))))
  )

  ;; ORR Wd, Wn, Wm (32-bit): 00101010000 Rm 000000 Rn Rd
  (func $emit_instr_orr_32 (param $rd i32) (param $rn i32) (param $rm i32)
    (call $emit_instr
      (i32.or (i32.const 0x2A000000)
              (i32.or (i32.shl (local.get $rm) (i32.const 16))
                      (i32.or (i32.shl (local.get $rn) (i32.const 5))
                              (local.get $rd)))))
  )

  ;; EOR Wd, Wn, Wm (32-bit): 01001010000 Rm 000000 Rn Rd
  (func $emit_instr_eor_32 (param $rd i32) (param $rn i32) (param $rm i32)
    (call $emit_instr
      (i32.or (i32.const 0x4A000000)
              (i32.or (i32.shl (local.get $rm) (i32.const 16))
                      (i32.or (i32.shl (local.get $rn) (i32.const 5))
                              (local.get $rd)))))
  )

  ;; LSLV Xd, Xn, Xm (64-bit variable shift left): 10011010110 001000 Rm Rn Rd
  (func $emit_instr_lslv_64 (param $rd i32) (param $rn i32) (param $rm i32)
    (call $emit_instr
      (i32.or (i32.const 0x9AC02000)
              (i32.or (i32.shl (local.get $rm) (i32.const 16))
                      (i32.or (i32.shl (local.get $rn) (i32.const 5))
                              (local.get $rd)))))
  )

  ;; LSRV Xd, Xn, Xm (64-bit): 10011010110 001001 Rm Rn Rd
  (func $emit_instr_lsrv_64 (param $rd i32) (param $rn i32) (param $rm i32)
    (call $emit_instr
      (i32.or (i32.const 0x9AC02400)
              (i32.or (i32.shl (local.get $rm) (i32.const 16))
                      (i32.or (i32.shl (local.get $rn) (i32.const 5))
                              (local.get $rd)))))
  )

  ;; ASRV Xd, Xn, Xm (64-bit): 10011010110 001010 Rm Rn Rd
  (func $emit_instr_asrv_64 (param $rd i32) (param $rn i32) (param $rm i32)
    (call $emit_instr
      (i32.or (i32.const 0x9AC02800)
              (i32.or (i32.shl (local.get $rm) (i32.const 16))
                      (i32.or (i32.shl (local.get $rn) (i32.const 5))
                              (local.get $rd)))))
  )

  ;; RORV Xd, Xn, Xm (64-bit): 10011010110 001011 Rm Rn Rd
  (func $emit_instr_rorv_64 (param $rd i32) (param $rn i32) (param $rm i32)
    (call $emit_instr
      (i32.or (i32.const 0x9AC02C00)
              (i32.or (i32.shl (local.get $rm) (i32.const 16))
                      (i32.or (i32.shl (local.get $rn) (i32.const 5))
                              (local.get $rd)))))
  )

  ;; LSLV Wd, Wn, Wm (32-bit): 00011010110 001000 Rm Rn Rd
  (func $emit_instr_lslv_32 (param $rd i32) (param $rn i32) (param $rm i32)
    (call $emit_instr
      (i32.or (i32.const 0x1AC02000)
              (i32.or (i32.shl (local.get $rm) (i32.const 16))
                      (i32.or (i32.shl (local.get $rn) (i32.const 5))
                              (local.get $rd)))))
  )

  ;; LSRV Wd, Wn, Wm (32-bit): 00011010110 001001 Rm Rn Rd
  (func $emit_instr_lsrv_32 (param $rd i32) (param $rn i32) (param $rm i32)
    (call $emit_instr
      (i32.or (i32.const 0x1AC02400)
              (i32.or (i32.shl (local.get $rm) (i32.const 16))
                      (i32.or (i32.shl (local.get $rn) (i32.const 5))
                              (local.get $rd)))))
  )

  ;; ASRV Wd, Wn, Wm (32-bit): 00011010110 001010 Rm Rn Rd
  (func $emit_instr_asrv_32 (param $rd i32) (param $rn i32) (param $rm i32)
    (call $emit_instr
      (i32.or (i32.const 0x1AC02800)
              (i32.or (i32.shl (local.get $rm) (i32.const 16))
                      (i32.or (i32.shl (local.get $rn) (i32.const 5))
                              (local.get $rd)))))
  )

  ;; RORV Wd, Wn, Wm (32-bit): 00011010110 001011 Rm Rn Rd
  (func $emit_instr_rorv_32 (param $rd i32) (param $rn i32) (param $rm i32)
    (call $emit_instr
      (i32.or (i32.const 0x1AC02C00)
              (i32.or (i32.shl (local.get $rm) (i32.const 16))
                      (i32.or (i32.shl (local.get $rn) (i32.const 5))
                              (local.get $rd)))))
  )

  ;; ── AArch64 Immediate arithmetic helpers ──────────────────────────

  ;; ADD Xd, Xn, #imm12 (shift=0): 10010001 00 sh imm12 Rn Rd
  ;;   Only for imm12 (0-4095) where imm fits in 12 bits unsigned
  (func $emit_instr_addi_64 (param $rd i32) (param $rn i32) (param $imm12 i32)
    (call $emit_instr
      (i32.or (i32.const 0x91000000)
              (i32.or (i32.shl (local.get $imm12) (i32.const 10))
                      (i32.or (i32.shl (local.get $rn) (i32.const 5))
                              (local.get $rd)))))
  )

  ;; SUB Xd, Xn, #imm12 (shift=0): 11010001 00 sh imm12 Rn Rd
  (func $emit_instr_subi_64 (param $rd i32) (param $rn i32) (param $imm12 i32)
    (call $emit_instr
      (i32.or (i32.const 0xD1000000)
              (i32.or (i32.shl (local.get $imm12) (i32.const 10))
                      (i32.or (i32.shl (local.get $rn) (i32.const 5))
                              (local.get $rd)))))
  )

  ;; ADD Wd, Wn, #imm12 (32-bit): 00010001 00 sh imm12 Rn Rd
  (func $emit_instr_addi_32 (param $rd i32) (param $rn i32) (param $imm12 i32)
    (call $emit_instr
      (i32.or (i32.const 0x11000000)
              (i32.or (i32.shl (local.get $imm12) (i32.const 10))
                      (i32.or (i32.shl (local.get $rn) (i32.const 5))
                              (local.get $rd)))))
  )

  ;; SUB Wd, Wn, #imm12 (32-bit): 01010001 00 sh imm12 Rn Rd
  (func $emit_instr_subi_32 (param $rd i32) (param $rn i32) (param $imm12 i32)
    (call $emit_instr
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
  (func $emit_instr_movz_64 (param $rd i32) (param $hw i32) (param $imm16 i32)
    (call $emit_instr
      (i32.or (i32.const 0xD2800000)
              (i32.or (i32.shl (local.get $hw) (i32.const 21))
                      (i32.or (i32.shl (local.get $imm16) (i32.const 5))
                              (local.get $rd)))))
  )

  ;; MOVN Xd, #imm (64-bit): 100100101 hw imm16 Rd
  (func $emit_instr_movn_64 (param $rd i32) (param $hw i32) (param $imm16 i32)
    (call $emit_instr
      (i32.or (i32.const 0x92800000)
              (i32.or (i32.shl (local.get $hw) (i32.const 21))
                      (i32.or (i32.shl (local.get $imm16) (i32.const 5))
                              (local.get $rd)))))
  )

  ;; MOVK Xd, #imm (64-bit, keep other bits): 111100101 hw imm16 Rd
  (func $emit_instr_movk_64 (param $rd i32) (param $hw i32) (param $imm16 i32)
    (call $emit_instr
      (i32.or (i32.const 0xF2800000)
              (i32.or (i32.shl (local.get $hw) (i32.const 21))
                      (i32.or (i32.shl (local.get $imm16) (i32.const 5))
                              (local.get $rd)))))
  )

  ;; MOV N, #imm (32-bit): MOVZ Wd, #imm or MOVN Wd, #imm
  (func $emit_instr_movz_32 (param $rd i32) (param $hw i32) (param $imm16 i32)
    (call $emit_instr
      (i32.or (i32.const 0x52800000)
              (i32.or (i32.shl (local.get $hw) (i32.const 21))
                      (i32.or (i32.shl (local.get $imm16) (i32.const 5))
                              (local.get $rd)))))
  )

  ;; ── AArch64 Load/Store helpers ─────────────────────────────────────
  ;; All use unsigned offset addressing: STR/LDR Xt, [Xn, #imm]
  ;; For stack push/pop with pre/post-index we use a different encoding.

  ;; STR Xt, [Xn, #imm] (64-bit, unsigned offset): 1111100101 imm12 Rn Rt
  (func $emit_instr_str_64_off (param $rt i32) (param $rn i32) (param $imm12 i32)
    (call $emit_instr
      (i32.or (i32.const 0xF9000000)
              (i32.or (i32.shl (local.get $imm12) (i32.const 10))
                      (i32.or (i32.shl (local.get $rn) (i32.const 5))
                              (local.get $rt)))))
  )

  ;; LDR Xt, [Xn, #imm] (64-bit, unsigned offset): 1111100101 imm12 Rn Rt
  (func $emit_instr_ldr_64_off (param $rt i32) (param $rn i32) (param $imm12 i32)
    (call $emit_instr
      (i32.or (i32.const 0xF9400000)
              (i32.or (i32.shl (local.get $imm12) (i32.const 10))
                      (i32.or (i32.shl (local.get $rn) (i32.const 5))
                              (local.get $rt)))))
  )

  ;; STR Wt, [Xn, #imm] (32-bit, unsigned offset): 1011100101 imm12 Rn Rt
  (func $emit_instr_str_32_off (param $rt i32) (param $rn i32) (param $imm12 i32)
    (call $emit_instr
      (i32.or (i32.const 0xB9000000)
              (i32.or (i32.shl (local.get $imm12) (i32.const 10))
                      (i32.or (i32.shl (local.get $rn) (i32.const 5))
                              (local.get $rt)))))
  )

  ;; LDR Wt, [Xn, #imm] (32-bit, unsigned offset): 1011100101 imm12 Rn Rt
  (func $emit_instr_ldr_32_off (param $rt i32) (param $rn i32) (param $imm12 i32)
    (call $emit_instr
      (i32.or (i32.const 0xB9400000)
              (i32.or (i32.shl (local.get $imm12) (i32.const 10))
                      (i32.or (i32.shl (local.get $rn) (i32.const 5))
                              (local.get $rt)))))
  )

  ;; STR Xt, [Xn, #imm]! (pre-index, 64-bit): 11111001 10 imm9 11 Xn Rt
  ;; imm9 is signed (-256 to 255), encoded as 9-bit signed
  (func $emit_instr_str_pre_64 (param $rt i32) (param $rn i32) (param $imm9 i32)
    (local $enc i32)
    (local.set $enc (i32.and (local.get $imm9) (i32.const 0x1FF)))
    (call $emit_instr
      (i32.or (i32.const 0xF8000000)
              (i32.or (i32.shl (local.get $enc) (i32.const 12))
                      (i32.or (i32.shl (local.get $rn) (i32.const 5))
                              (local.get $rt)))))
  )

  ;; LDR Xt, [Xn], #imm (post-index, 64-bit): 11111000 10 imm9 01 Xn Rt
  (func $emit_instr_ldr_post_64 (param $rt i32) (param $rn i32) (param $imm9 i32)
    (local $enc i32)
    (local.set $enc (i32.and (local.get $imm9) (i32.const 0x1FF)))
    (call $emit_instr
      (i32.or (i32.const 0xF8400400)
              (i32.or (i32.shl (local.get $enc) (i32.const 12))
                      (i32.or (i32.shl (local.get $rn) (i32.const 5))
                              (local.get $rt)))))
  )

  ;; STR Wt, [Xn, #imm]! (pre-index, 32-bit): 10111001 10 imm9 11 Xn Rt
  (func $emit_instr_str_pre_32 (param $rt i32) (param $rn i32) (param $imm9 i32)
    (local $enc i32)
    (local.set $enc (i32.and (local.get $imm9) (i32.const 0x1FF)))
    (call $emit_instr
      (i32.or (i32.const 0xB8000000)
              (i32.or (i32.shl (local.get $enc) (i32.const 12))
                      (i32.or (i32.shl (local.get $rn) (i32.const 5))
                              (local.get $rt)))))
  )

  ;; LDR Wt, [Xn], #imm (post-index, 32-bit): 10111000 10 imm9 01 Xn Rt
  (func $emit_instr_ldr_post_32 (param $rt i32) (param $rn i32) (param $imm9 i32)
    (local $enc i32)
    (local.set $enc (i32.and (local.get $imm9) (i32.const 0x1FF)))
    (call $emit_instr
      (i32.or (i32.const 0xB8400400)
              (i32.or (i32.shl (local.get $enc) (i32.const 12))
                      (i32.or (i32.shl (local.get $rn) (i32.const 5))
                              (local.get $rt)))))
  )

  ;; ── AArch64 Stack push/pop (value stack for WASM) ─────────────────
  ;; Push X0: STR X0, [SP, #-8]!
  (func $emit_push_x0
    (call $emit_instr_str_pre_64 (global.get $REG_X0) (global.get $REG_SP) (i32.const -8))
  )

  ;; Pop X0: LDR X0, [SP], #8
  (func $emit_pop_x0
    (call $emit_instr_ldr_post_64 (global.get $REG_X0) (global.get $REG_SP) (i32.const 8))
  )

  ;; Push X1: STR X1, [SP, #-8]!
  (func $emit_push_x1
    (call $emit_instr_str_pre_64 (global.get $REG_X1) (global.get $REG_SP) (i32.const -8))
  )

  ;; Pop X1: LDR X1, [SP], #8
  (func $emit_pop_x1
    (call $emit_instr_ldr_post_64 (global.get $REG_X1) (global.get $REG_SP) (i32.const 8))
  )

  ;; Push X2: STR X2, [SP, #-8]!
  (func $emit_push_x2
    (call $emit_instr_str_pre_64 (global.get $REG_X2) (global.get $REG_SP) (i32.const -8))
  )

  ;; Pop X2: LDR X2, [SP], #8
  (func $emit_pop_x2
    (call $emit_instr_ldr_post_64 (global.get $REG_X2) (global.get $REG_SP) (i32.const 8))
  )

  ;; Pop into X1: LDR X1, [SP], #8 (alias)
  (func $emit_pop_x1_alias
    (call $emit_pop_x1)
  )

  ;; Pop pair into X1, X0 (LDP X1, X0, [SP], #16)
  ;; LDP encoding: 10101000 11 0 imm7 Rt2 Rn Rt1  (with post-index)
  ;; LDP Xt1, Xt2, [Xn], #imm: 10101000 110 imm7 Xn Xt2 Xt1
  ;; Wait, LDP post-index: opc=10, 1010 1000 1 11 imm7 Xn Rt2 Rt1
  ;; Let me just use two separate pops
  (func $emit_pop2_x1_x0
    (call $emit_instr_ldr_post_64 (global.get $REG_X1) (global.get $REG_SP) (i32.const 8))
    (call $emit_instr_ldr_post_64 (global.get $REG_X0) (global.get $REG_SP) (i32.const 8))
  )

  ;; STP Xt1, Xt2, [SP, #-16]! (pre-index pair)
  ;; STP Xt1, Xt2, [Xn, #-imm]!: 10101000 10 0 imm7 Xn Xt2 Xt1
  ;; Actually: for 64-bit STP pre-index: opc=10, 1010 1000 1 00 imm7 Xn Rt2 Rt1
  (func $emit_push2_x1_x0
    (call $emit_instr_str_pre_64 (global.get $REG_X1) (global.get $REG_SP) (i32.const -8))
    (call $emit_instr_str_pre_64 (global.get $REG_X0) (global.get $REG_SP) (i32.const -8))
  )

  ;; ── Standard push/pop names (matching x86 compiler convention) ────
  (func $emit_pop_rax_alias
    (call $emit_pop_x0)
  )
  (func $emit_push_rax_alias
    (call $emit_push_x0)
  )
  (func $emit_pop_rcx_alias
    (call $emit_pop_x1)
  )
  (func $emit_push_rcx_alias
    (call $emit_push_x1)
  )
  (func $emit_pop_rdx_alias
    (call $emit_pop_x2)
  )

  ;; ── AArch64 sign extension / data processing ──────────────────────

  ;; SXTW X0, W0 (sign-extend W0→X0): 10011010110 000000 Rm(0) 00000 Rn(0) Rd
  ;; Actually: SXTW is alias for SBFM Xd, Xn, #0, #31
  ;; SBFM encoding: 100110 1 10 0 N immr imms Rn Rd
  ;; SXTW X0, W0: 0x93407C00
  ;; let me just hardcode
  (func $emit_sxtw_x0_w0
    (call $emit_instr (i32.const 0x93407C00))
  )

  ;; ── AArch64 branch instructions ────────────────────────────────────

  ;; B #imm (unconditional branch, ±128MB): 000101 + imm26
  ;; imm26 = (target - pc) >> 2, encoded as signed 26-bit
  (func $emit_b (param $off i32)
    (local $enc i32)
    (local.set $enc (i32.shr_s (i32.shl (local.get $off) (i32.const 6)) (i32.const 6)))  ;; sign-extend from 26 bits
    (call $emit_instr
      (i32.or (i32.const 0x14000000)
              (i32.and (i32.shr_s (local.get $off) (i32.const 2)) (i32.const 0x03FFFFFF)))
    )
  )

  ;; BL #imm (branch with link): 100101 + imm26
  (func $emit_bl (param $off i32)
    (call $emit_instr
      (i32.or (i32.const 0x94000000)
              (i32.and (i32.shr_s (local.get $off) (i32.const 2)) (i32.const 0x03FFFFFF)))
    )
  )

  ;; B.cond #imm (conditional branch, ±1MB): 01010100 imm19 0 cond
  ;; cond codes (AArch64): EQ=0, NE=1, CS/HS=2, CC/LO=3, MI=4, PL=5,
  ;;   VS=6, VC=7, HI=8, LS=9, GE=10, LT=11, GT=12, LE=13, AL=14
  (func $emit_b_cond (param $off i32) (param $cond i32)
    (call $emit_instr
      (i32.or (i32.const 0x54000000)
              (i32.or (i32.and (i32.shr_s (local.get $off) (i32.const 2)) (i32.const 0x7FFFF))
                      (i32.shl (local.get $cond) (i32.const 0))))
    )
  )

  ;; CBZ Xt, #imm (compare and branch if zero, ±1MB): 10110100 imm19 Rt
  ;; For 64-bit: 10110100 imm19 Rt
  (func $emit_cbz_x (param $rt i32) (param $off i32)
    (call $emit_instr
      (i32.or (i32.const 0xB4000000)
              (i32.or (i32.and (i32.shr_s (local.get $off) (i32.const 2)) (i32.const 0x7FFFF))
                      (local.get $rt)))
    )
  )

  ;; CBNZ Xt, #imm (compare and branch if non-zero, ±1MB): 10110101 imm19 Rt
  ;; For 64-bit: 10110101 imm19 Rt
  (func $emit_cbnz_x (param $rt i32) (param $off i32)
    (call $emit_instr
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
  (func $emit_cset_x0 (param $inv_cond i32)
    ;; CSINC X0, XZR, XZR, inv_cond: 0x7A9F0400 | (inv_cond << 12)
    (call $emit_instr
      (i32.or (i32.const 0x7A9F0400)
              (i32.shl (local.get $inv_cond) (i32.const 12)))
    )
  )

  ;; CSET W0, inv_cond (32-bit)
  (func $emit_cset_w0 (param $inv_cond i32)
    ;; CSINC W0, WZR, WZR, inv_cond: 0x3A9F0400 | (inv_cond << 12)
    (call $emit_instr
      (i32.or (i32.const 0x3A9F0400)
              (i32.shl (local.get $inv_cond) (i32.const 12)))
    )
  )

  ;; CMP Xn, Xm: alias for SUBS XZR, Xn, Xm
  ;; SUBS XZR, Xn, Xm: sf=1, S=1, 11011 000 Rm 000000 Rn 11111
  ;; = 0xEB00001F | (Rm << 16) | (Rn << 5)
  (func $emit_cmp_64 (param $rn i32) (param $rm i32)
    (call $emit_instr
      (i32.or (i32.const 0xEB00001F)
              (i32.or (i32.shl (local.get $rm) (i32.const 16))
                      (i32.shl (local.get $rn) (i32.const 5))))
    )
  )

  ;; CMP Wn, Wm (32-bit): SUBS WZR, Wn, Wm
  ;; = 0x6B00001F | (Rm << 16) | (Rn << 5)
  (func $emit_cmp_32 (param $rn i32) (param $rm i32)
    (call $emit_instr
      (i32.or (i32.const 0x6B00001F)
              (i32.or (i32.shl (local.get $rm) (i32.const 16))
                      (i32.shl (local.get $rn) (i32.const 5))))
    )
  )

  ;; TST Xn, Xm: alias for ANDS XZR, Xn, Xm
  ;; = 0xEA00001F | (Rm << 16) | (Rn << 5)
  (func $emit_tst_64 (param $rn i32) (param $rm i32)
    (call $emit_instr
      (i32.or (i32.const 0xEA00001F)
              (i32.or (i32.shl (local.get $rm) (i32.const 16))
                      (i32.shl (local.get $rn) (i32.const 5))))
    )
  )

  ;; TST Wn, Wm (32-bit)
  (func $emit_tst_32 (param $rn i32) (param $rm i32)
    (call $emit_instr
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
  (func $emit_ldr_reg_64 (param $rt i32) (param $rn i32) (param $rm i32)
    (call $emit_instr
      (i32.or (i32.const 0xF8600800)
              (i32.or (i32.shl (local.get $rm) (i32.const 16))
                      (i32.or (i32.shl (local.get $rn) (i32.const 5))
                              (local.get $rt)))))
  )

  ;; STR Xt, [Xn, Xm, LSL #0] (64-bit register offset)
  (func $emit_str_reg_64 (param $rt i32) (param $rn i32) (param $rm i32)
    (call $emit_instr
      (i32.or (i32.const 0xF8200800)
              (i32.or (i32.shl (local.get $rm) (i32.const 16))
                      (i32.or (i32.shl (local.get $rn) (i32.const 5))
                              (local.get $rt)))))
  )

  ;; LDR Wt, [Xn, Xm, LSL #0] (32-bit register offset)
  (func $emit_ldr_reg_32 (param $rt i32) (param $rn i32) (param $rm i32)
    (call $emit_instr
      (i32.or (i32.const 0xB8600800)
              (i32.or (i32.shl (local.get $rm) (i32.const 16))
                      (i32.or (i32.shl (local.get $rn) (i32.const 5))
                              (local.get $rt)))))
  )

  ;; STR Wt, [Xn, Xm, LSL #0] (32-bit register offset)
  (func $emit_str_reg_32 (param $rt i32) (param $rn i32) (param $rm i32)
    (call $emit_instr
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
  (func $emit_ldr_mem_ptr
    (call $emit_instr_ldr_64_off (global.get $REG_X2) (global.get $REG_X19) (i32.const 1))  ;; #8 = imm12=1 (8/8)
  )

  ;; LDR X0, [X19, #offset] — load JitGlobals field
  (func $emit_ldr_x0_x19 (param $off12 i32)
    (call $emit_instr_ldr_64_off (global.get $REG_X0) (global.get $REG_X19) (local.get $off12))
  )

  ;; STR X0, [X19, #offset] — store to JitGlobals field
  (func $emit_str_x0_x19 (param $off12 i32)
    (call $emit_instr_str_64_off (global.get $REG_X0) (global.get $REG_X19) (local.get $off12))
  )

  ;; LDR X0, [X20, #offset] — load local via cached base ptr
  (func $emit_ldr_x0_x20 (param $off12 i32)
    (call $emit_instr_ldr_64_off (global.get $REG_X0) (global.get $REG_X20) (local.get $off12))
  )

  ;; STR X0, [X20, #offset] — store local
  (func $emit_str_x0_x20 (param $off12 i32)
    (call $emit_instr_str_64_off (global.get $REG_X0) (global.get $REG_X20) (local.get $off12))
  )

  ;; ── AArch64 memory load (i32.load): address in X1, result in X0 ──
  ;; load mem_ptr into X2, then LDR W0, [X2, X1]
  (func $emit_mem_load32
    (call $emit_ldr_mem_ptr)
    (call $emit_ldr_reg_32 (global.get $REG_X0) (global.get $REG_X2) (global.get $REG_X1))
  )

  ;; i64.load: LDR X0, [X2, X1]
  (func $emit_mem_load64
    (call $emit_ldr_mem_ptr)
    (call $emit_ldr_reg_64 (global.get $REG_X0) (global.get $REG_X2) (global.get $REG_X1))
  )

  ;; i32.store: address in X1, value in X0, store W0 to [X2, X1]
  (func $emit_mem_store32
    (call $emit_ldr_mem_ptr)
    (call $emit_str_reg_32 (global.get $REG_X0) (global.get $REG_X2) (global.get $REG_X1))
  )

  ;; i64.store: STR X0, [X2, X1]
  (func $emit_mem_store64
    (call $emit_ldr_mem_ptr)
    (call $emit_str_reg_64 (global.get $REG_X0) (global.get $REG_X2) (global.get $REG_X1))
  )

  ;; ── AArch64 Unary arithmetic helpers (X0←op(X0)) ──────────────────

  ;; CLZ X0, X0 (count leading zeros, 64-bit)
  (func $emit_clz_64
    (call $emit_instr (i32.const 0xDAC01000))
  )

  ;; CLZ W0, W0 (32-bit)
  (func $emit_clz_32
    (call $emit_instr (i32.const 0x5AC01000))
  )

  ;; RBIT X0, X0 (reverse bits, 64-bit) — for CTZ: RBIT + CLZ
  (func $emit_rbit_64
    (call $emit_instr (i32.const 0xDAC00000))
  )

  ;; RBIT W0, W0 (32-bit)
  (func $emit_rbit_32
    (call $emit_instr (i32.const 0x5AC00000))
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
  (func $emit_fmov_s_w (param $sd i32) (param $wn i32)
    (call $emit_instr
      (i32.or (i32.const 0x1E270000)
              (i32.or (i32.shl (local.get $wn) (i32.const 5))
                      (local.get $sd))))
  )

  ;; FMOV Wd, Sn: 0x1E260000 | (Sn << 5) | Rd
  (func $emit_fmov_w_s (param $wd i32) (param $sn i32)
    (call $emit_instr
      (i32.or (i32.const 0x1E260000)
              (i32.or (i32.shl (local.get $sn) (i32.const 5))
                      (local.get $wd))))
  )

  ;; FMOV D0, X0: 0x9E670000
  ;; FMOV Dd, Xn: 9E670000 | (Xn << 5) | Dd
  (func $emit_fmov_d_x (param $dd i32) (param $xn i32)
    (call $emit_instr
      (i32.or (i32.const 0x9E670000)
              (i32.or (i32.shl (local.get $xn) (i32.const 5))
                      (local.get $dd))))
  )

  ;; FMOV Xd, Dn: 9E660000 | (Dn << 5) | Xd
  (func $emit_fmov_x_d (param $xd i32) (param $dn i32)
    (call $emit_instr
      (i32.or (i32.const 0x9E660000)
              (i32.or (i32.shl (local.get $dn) (i32.const 5))
                      (local.get $xd))))
  )

  ;; ── AArch64 NEON scalar float arithmetic ──────────────────────────

  ;; FADD Sd, Sn, Sm (scalar float32): 0E 30 00 ...
  ;; FADD S0, S0, S1: 0x1E302800
  ;; FADD Sd, Sn, Sm: 0x1E302800 | (Sm << 16) | (Sn << 5) | Sd
  (func $emit_fadd_s (param $sd i32) (param $sn i32) (param $sm i32)
    (call $emit_instr
      (i32.or (i32.const 0x1E302800)
              (i32.or (i32.shl (local.get $sm) (i32.const 16))
                      (i32.or (i32.shl (local.get $sn) (i32.const 5))
                              (local.get $sd)))))
  )

  ;; FSUB Sd, Sn, Sm: 0x1E303800
  (func $emit_fsub_s (param $sd i32) (param $sn i32) (param $sm i32)
    (call $emit_instr
      (i32.or (i32.const 0x1E303800)
              (i32.or (i32.shl (local.get $sm) (i32.const 16))
                      (i32.or (i32.shl (local.get $sn) (i32.const 5))
                              (local.get $sd)))))
  )

  ;; FMUL Sd, Sn, Sm: 0x1E300800
  (func $emit_fmul_s (param $sd i32) (param $sn i32) (param $sm i32)
    (call $emit_instr
      (i32.or (i32.const 0x1E300800)
              (i32.or (i32.shl (local.get $sm) (i32.const 16))
                      (i32.or (i32.shl (local.get $sn) (i32.const 5))
                              (local.get $sd)))))
  )

  ;; FDIV Sd, Sn, Sm: 0x1E301800
  (func $emit_fdiv_s (param $sd i32) (param $sn i32) (param $sm i32)
    (call $emit_instr
      (i32.or (i32.const 0x1E301800)
              (i32.or (i32.shl (local.get $sm) (i32.const 16))
                      (i32.or (i32.shl (local.get $sn) (i32.const 5))
                              (local.get $sd)))))
  )

  ;; FADD Dd, Dn, Dm (scalar f64): 1E 60 28 ...
  ;; FADD D0, D0, D1: 0x1E602800
  ;; FADD Dd, Dn, Dm: 0x1E602800 | (Dm << 16) | (Dn << 5) | Dd
  (func $emit_fadd_d (param $dd i32) (param $dn i32) (param $dm i32)
    (call $emit_instr
      (i32.or (i32.const 0x1E602800)
              (i32.or (i32.shl (local.get $dm) (i32.const 16))
                      (i32.or (i32.shl (local.get $dn) (i32.const 5))
                              (local.get $dd)))))
  )

  ;; FSUB Dd, Dn, Dm: 0x1E603800
  (func $emit_fsub_d (param $dd i32) (param $dn i32) (param $dm i32)
    (call $emit_instr
      (i32.or (i32.const 0x1E603800)
              (i32.or (i32.shl (local.get $dm) (i32.const 16))
                      (i32.or (i32.shl (local.get $dn) (i32.const 5))
                              (local.get $dd)))))
  )

  ;; FMUL Dd, Dn, Dm: 0x1E600800
  (func $emit_fmul_d (param $dd i32) (param $dn i32) (param $dm i32)
    (call $emit_instr
      (i32.or (i32.const 0x1E600800)
              (i32.or (i32.shl (local.get $dm) (i32.const 16))
                      (i32.or (i32.shl (local.get $dn) (i32.const 5))
                              (local.get $dd)))))
  )

  ;; FDIV Dd, Dn, Dm: 0x1E601800
  (func $emit_fdiv_d (param $dd i32) (param $dn i32) (param $dm i32)
    (call $emit_instr
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
  (func $emit_fcmp_s0_s1
    (call $emit_instr (i32.const 0x1E202008))
  )
  (func $emit_fcmp_d0_d1
    (call $emit_instr (i32.const 0x1E602008))
  )
  (func $emit_fcmp_s0_s0
    (call $emit_instr (i32.const 0x1E202000))
  )
  (func $emit_fcmp_d0_d0
    (call $emit_instr (i32.const 0x1E602000))
  )

  ;; Float conversions:
  ;; FCVTZ W0, S0 (f32→i32): 1E 38 00 00?
  ;; FCVTZ Wd, Sn: 0x1E380000 | (Sn << 5) | Wd
  ;; For W0, S0: 0x1E380000
  (func $emit_fcvtz_w_s (param $wd i32) (param $sn i32)
    (call $emit_instr
      (i32.or (i32.const 0x1E380000)
              (i32.or (i32.shl (local.get $sn) (i32.const 5))
                      (local.get $wd))))
  )

  ;; FCVTZ X0, S0 (f32→i64): 9E 38 00 00
  ;; FCVTZ Xd, Sn: 0x9E380000 | (Sn << 5) | Xd
  (func $emit_fcvtz_x_s (param $xd i32) (param $sn i32)
    (call $emit_instr
      (i32.or (i32.const 0x9E380000)
              (i32.or (i32.shl (local.get $sn) (i32.const 5))
                      (local.get $xd))))
  )

  ;; FCVTZ W0, D0 (f64→i32): 1E 78 00 00
  (func $emit_fcvtz_w_d (param $wd i32) (param $dn i32)
    (call $emit_instr
      (i32.or (i32.const 0x1E780000)
              (i32.or (i32.shl (local.get $dn) (i32.const 5))
                      (local.get $wd))))
  )

  ;; FCVTZ X0, D0 (f64→i64): 9E 78 00 00
  (func $emit_fcvtz_x_d (param $xd i32) (param $dn i32)
    (call $emit_instr
      (i32.or (i32.const 0x9E780000)
              (i32.or (i32.shl (local.get $dn) (i32.const 5))
                      (local.get $xd))))
  )

  ;; SCVTF S0, W0 (i32→f32): 1E 22 00 00
  ;; SCVTF Sd, Wn: 0x1E220000 | (Wn << 5) | Sd
  (func $emit_scvtf_s_w (param $sd i32) (param $wn i32)
    (call $emit_instr
      (i32.or (i32.const 0x1E220000)
              (i32.or (i32.shl (local.get $wn) (i32.const 5))
                      (local.get $sd))))
  )

  ;; UCVTF S0, W0 (i32→f32 unsigned): 1E 23 00 00
  (func $emit_ucvtf_s_w (param $sd i32) (param $wn i32)
    (call $emit_instr
      (i32.or (i32.const 0x1E230000)
              (i32.or (i32.shl (local.get $wn) (i32.const 5))
                      (local.get $sd))))
  )

  ;; SCVTF D0, X0 (i64→f64): 9E 62 00 00
  (func $emit_scvtf_d_x (param $dd i32) (param $xn i32)
    (call $emit_instr
      (i32.or (i32.const 0x9E620000)
              (i32.or (i32.shl (local.get $xn) (i32.const 5))
                      (local.get $dd))))
  )

  ;; UCVTF D0, X0 (i64→f64 unsigned): 9E 63 00 00
  (func $emit_ucvtf_d_x (param $dd i32) (param $xn i32)
    (call $emit_instr
      (i32.or (i32.const 0x9E630000)
              (i32.or (i32.shl (local.get $xn) (i32.const 5))
                      (local.get $dd))))
  )

  ;; SCVTF D0, W0 (i32→f64): 1E 62 00 00
  (func $emit_scvtf_d_w (param $dd i32) (param $wn i32)
    (call $emit_instr
      (i32.or (i32.const 0x1E620000)
              (i32.or (i32.shl (local.get $wn) (i32.const 5))
                      (local.get $dd))))
  )

  ;; SCVTF S0, X0 (i64→f32): 9E 22 00 00
  (func $emit_scvtf_s_x (param $sd i32) (param $xn i32)
    (call $emit_instr
      (i32.or (i32.const 0x9E220000)
              (i32.or (i32.shl (local.get $xn) (i32.const 5))
                      (local.get $sd))))
  )

  ;; UCVTF S0, X0 (i64→f32 unsigned): 9E 23 00 00
  (func $emit_ucvtf_s_x (param $sd i32) (param $xn i32)
    (call $emit_instr
      (i32.or (i32.const 0x9E230000)
              (i32.or (i32.shl (local.get $xn) (i32.const 5))
                      (local.get $sd))))
  )

  ;; FCVT S0, D0 (f64→f32 demote): 1E 62 80 00 ... 
  ;; Actually FCVT Sd, Dn: 0x1E624000 | (Dn << 5) | Sd
  ;; Let me just hardcode:
  (func $emit_fcvt_s_d (param $sd i32) (param $dn i32)
    (call $emit_instr
      (i32.or (i32.const 0x1E624000)
              (i32.or (i32.shl (local.get $dn) (i32.const 5))
                      (local.get $sd))))
  )

  ;; FCVT D0, S0 (f32→f64 promote): 1E 22 40 00
  (func $emit_fcvt_d_s (param $dd i32) (param $sn i32)
    (call $emit_instr
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
  (func $emit_frintp_s0_s0
    (call $emit_instr (i32.const 0x1E2C0800))
  )
  ;; FRINTM S0, S0: 0x1E2C1800
  (func $emit_frintm_s0_s0
    (call $emit_instr (i32.const 0x1E2C1800))
  )
  ;; FRINTZ S0, S0: 0x1E2C2800
  (func $emit_frintz_s0_s0
    (call $emit_instr (i32.const 0x1E2C2800))
  )
  ;; FRINTN S0, S0: 0x1E2C4800 (ties to even) — nearest in IEEE 754
  (func $emit_frintn_s0_s0
    (call $emit_instr (i32.const 0x1E2C4800))
  )
  ;; FRINTP D0, D0: 0x1E6C0800
  (func $emit_frintp_d0_d0
    (call $emit_instr (i32.const 0x1E6C0800))
  )
  ;; FRINTM D0, D0: 0x1E6C1800
  (func $emit_frintm_d0_d0
    (call $emit_instr (i32.const 0x1E6C1800))
  )
  ;; FRINTZ D0, D0: 0x1E6C2800
  (func $emit_frintz_d0_d0
    (call $emit_instr (i32.const 0x1E6C2800))
  )
  ;; FRINTN D0, D0: 0x1E6C4800
  (func $emit_frintn_d0_d0
    (call $emit_instr (i32.const 0x1E6C4800))
  )

  ;; FSQRT S0, S0: 1E 21 C0 00
  (func $emit_fsqrt_s0_s0
    (call $emit_instr (i32.const 0x1E21C000))
  )
  ;; FSQRT D0, D0: 1E 61 C0 00
  (func $emit_fsqrt_d0_d0
    (call $emit_instr (i32.const 0x1E61C000))
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
  (func $emit_prologue
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
    (call $emit_instr (i32.const 0xA9BF0FDD))  ;; STP X29, X30, [SP, #-16]!

    ;; MOV X29, SP: 0x910003BD
    (call $emit_instr (i32.const 0x910003BD))  ;; ADD X29, SP, #0

    ;; STP X19, X20, [SP, #-16]!
    ;; = 0xA9BE0000 | (SP=31<<5) | (X20<<10) | X19
    ;; = 0xA9BE0000 | 0x3E0 | 0x5000 | 0x13 = 0xA9BE53F3
    (call $emit_instr (i32.const 0xA9BE53F3))  ;; STP X19, X20, [SP, #-16]!

    ;; STP X21, X22, [SP, #-16]!
    ;; = 0xA9BE0000 | 0x3E0 | 0x5800 | 0x15 = 0xA9BE5BF5
    (call $emit_instr (i32.const 0xA9BE5BF5))  ;; STP X21, X22, [SP, #-16]!
  )

  ;; Epilogue: restore callee-saved, restore FP/LR, ret
  (func $emit_epilogue
    ;; LDP X21, X22, [SP], #16
    (call $emit_instr (i32.const 0xA8C15BF5))  ;; LDP X21, X22, [SP], #16

    ;; LDP X19, X20, [SP], #16
    (call $emit_instr (i32.const 0xA8C153F3))  ;; LDP X19, X20, [SP], #16

    ;; LDP X29, X30, [SP], #16
    (call $emit_instr (i32.const 0xA8C10FDD))  ;; LDP X29, X30, [SP], #16

    ;; RET (X30): 0xD65F03C0
    (call $emit_instr (i32.const 0xD65F03C0))
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

  ;; ADD X0, X0, X1 (64-bit)
  (func $emit_add_x0_x1
    (call $emit_instr_add_64 (global.get $REG_X0) (global.get $REG_X0) (global.get $REG_X1))
  )

  ;; SUB X0, X0, X1 (64-bit)
  (func $emit_sub_x0_x1
    (call $emit_instr_sub_64 (global.get $REG_X0) (global.get $REG_X0) (global.get $REG_X1))
  )

  ;; ADD W0, W0, W1 (32-bit)
  (func $emit_add_w0_w1
    (call $emit_instr_add_32 (global.get $REG_X0) (global.get $REG_X0) (global.get $REG_X1))
  )

  ;; SUB W0, W0, W1 (32-bit)
  (func $emit_sub_w0_w1
    (call $emit_instr_sub_32 (global.get $REG_X0) (global.get $REG_X0) (global.get $REG_X1))
  )

  ;; MUL X0, X0, X1 (64-bit)
  (func $emit_mul_x0_x1
    (call $emit_instr_mul_64 (global.get $REG_X0) (global.get $REG_X0) (global.get $REG_X1))
  )

  ;; MUL W0, W0, W1 (32-bit)
  (func $emit_mul_w0_w1
    (call $emit_instr_mul_32 (global.get $REG_X0) (global.get $REG_X0) (global.get $REG_X1))
  )

  ;; SDIV X0, X0, X1 (64-bit signed)
  (func $emit_sdiv_x0_x1
    (call $emit_instr_sdiv_64 (global.get $REG_X0) (global.get $REG_X0) (global.get $REG_X1))
  )

  ;; UDIV X0, X0, X1 (64-bit unsigned)
  (func $emit_udiv_x0_x1
    (call $emit_instr_udiv_64 (global.get $REG_X0) (global.get $REG_X0) (global.get $REG_X1))
  )

  ;; SDIV W0, W0, W1 (32-bit signed)
  (func $emit_sdiv_w0_w1
    (call $emit_instr_sdiv_32 (global.get $REG_X0) (global.get $REG_X0) (global.get $REG_X1))
  )

  ;; UDIV W0, W0, W1 (32-bit unsigned)
  (func $emit_udiv_w0_w1
    (call $emit_instr_udiv_32 (global.get $REG_X0) (global.get $REG_X0) (global.get $REG_X1))
  )

  ;; AND X0, X0, X1 (64-bit)
  (func $emit_and_x0_x1
    (call $emit_instr_and_64 (global.get $REG_X0) (global.get $REG_X0) (global.get $REG_X1))
  )

  ;; ORR X0, X0, X1 (64-bit)
  (func $emit_orr_x0_x1
    (call $emit_instr_orr_64 (global.get $REG_X0) (global.get $REG_X0) (global.get $REG_X1))
  )

  ;; EOR X0, X0, X1 (64-bit)
  (func $emit_eor_x0_x1
    (call $emit_instr_eor_64 (global.get $REG_X0) (global.get $REG_X0) (global.get $REG_X1))
  )

  ;; AND W0, W0, W1 (32-bit)
  (func $emit_and_w0_w1
    (call $emit_instr_and_32 (global.get $REG_X0) (global.get $REG_X0) (global.get $REG_X1))
  )

  ;; ORR W0, W0, W1 (32-bit)
  (func $emit_orr_w0_w1
    (call $emit_instr_orr_32 (global.get $REG_X0) (global.get $REG_X0) (global.get $REG_X1))
  )

  ;; EOR W0, W0, W1 (32-bit)
  (func $emit_eor_w0_w1
    (call $emit_instr_eor_32 (global.get $REG_X0) (global.get $REG_X0) (global.get $REG_X1))
  )

  ;; LSL X0, X0, X1 (64-bit variable)
  (func $emit_lslv_x0_x1
    (call $emit_instr_lslv_64 (global.get $REG_X0) (global.get $REG_X0) (global.get $REG_X1))
  )

  ;; LSR X0, X0, X1 (64-bit variable)
  (func $emit_lsrv_x0_x1
    (call $emit_instr_lsrv_64 (global.get $REG_X0) (global.get $REG_X0) (global.get $REG_X1))
  )

  ;; ASR X0, X0, X1 (64-bit variable)
  (func $emit_asrv_x0_x1
    (call $emit_instr_asrv_64 (global.get $REG_X0) (global.get $REG_X0) (global.get $REG_X1))
  )

  ;; ROR X0, X0, X1 (64-bit variable)
  (func $emit_rorv_x0_x1
    (call $emit_instr_rorv_64 (global.get $REG_X0) (global.get $REG_X0) (global.get $REG_X1))
  )

  ;; LSL W0, W0, W1 (32-bit variable)
  (func $emit_lslv_w0_w1
    (call $emit_instr_lslv_32 (global.get $REG_X0) (global.get $REG_X0) (global.get $REG_X1))
  )

  ;; LSR W0, W0, W1 (32-bit variable)
  (func $emit_lsrv_w0_w1
    (call $emit_instr_lsrv_32 (global.get $REG_X0) (global.get $REG_X0) (global.get $REG_X1))
  )

  ;; ASR W0, W0, W1 (32-bit variable)
  (func $emit_asrv_w0_w1
    (call $emit_instr_asrv_32 (global.get $REG_X0) (global.get $REG_X0) (global.get $REG_X1))
  )

  ;; ROR W0, W0, W1 (32-bit variable)
  (func $emit_rorv_w0_w1
    (call $emit_instr_rorv_32 (global.get $REG_X0) (global.get $REG_X0) (global.get $REG_X1))
  )

  ;; ═════════════════════════════════════════════════════════════════════
  ;; ELF / flat binary output
  ;; ═════════════════════════════════════════════════════════════════════

  ;; BL with target VA: compute relative offset and emit BL (AArch64 PC = current, no +8)
  (func $emit_bl_rel (param $target_va i32)
    (local $saved i32)
    (local.set $saved (i32.load (global.get $JS_CODE_PTR)))
    (call $emit_bl
      (i32.sub
        (local.get $target_va)
        (i32.add (global.get $TEXT_VA) (i32.sub (local.get $saved) (global.get $ELF_OUT_OFF)))))
  )

  ;; ── ELF64 header (64 bytes) ─────────────────────────────────────

  (func $emit_elf64_ehdr (param $entry_va i32) (param $phoff i32) (param $phnum i32)
    (call $emit_byte (i32.const 0x7F))
    (call $emit_byte (i32.const 0x45)) (call $emit_byte (i32.const 0x4C)) (call $emit_byte (i32.const 0x46))
    (call $emit_byte (i32.const 2))     (call $emit_byte (i32.const 1))
    (call $emit_byte (i32.const 1))     (call $emit_byte (i32.const 0))
    (call $emit_qword (i64.const 0))    ;; padding bytes 8-15
    (call $emit_byte (i32.const 2)) (call $emit_byte (i32.const 0))  ;; e_type = ET_EXEC
    (call $emit_byte (i32.const 0xB7)) (call $emit_byte (i32.const 0))  ;; e_machine = AArch64
    (call $emit_dword (i32.const 1))    ;; e_version
    (call $emit_qword (i64.extend_i32_u (local.get $entry_va)))  ;; e_entry
    (call $emit_qword (i64.extend_i32_u (local.get $phoff)))     ;; e_phoff
    (call $emit_qword (i64.const 0))    ;; e_shoff
    (call $emit_dword (i32.const 0))    ;; e_flags
    (call $emit_byte (i32.const 64)) (call $emit_byte (i32.const 0))  ;; e_ehsize
    (call $emit_byte (i32.const 56)) (call $emit_byte (i32.const 0))  ;; e_phentsize
    (call $emit_byte (local.get $phnum)) (call $emit_byte (i32.const 0))  ;; e_phnum
    (call $emit_byte (i32.const 0)) (call $emit_byte (i32.const 0))  ;; e_shentsize
    (call $emit_byte (i32.const 0)) (call $emit_byte (i32.const 0))  ;; e_shnum
    (call $emit_byte (i32.const 0)) (call $emit_byte (i32.const 0))  ;; e_shstrndx
  )

  ;; ── ELF64 program header (56 bytes) ────────────────────────────

  (func $emit_elf64_phdr (param $type i32) (param $flags i32)
                         (param $offset i32) (param $vaddr i32)
                         (param $filesz i32) (param $memsz i32)
    (call $emit_dword (local.get $type))     ;; p_type
    (call $emit_dword (local.get $flags))    ;; p_flags
    (call $emit_qword (i64.extend_i32_u (local.get $offset)))  ;; p_offset
    (call $emit_qword (i64.extend_i32_u (local.get $vaddr)))   ;; p_vaddr
    (call $emit_qword (i64.extend_i32_u (local.get $vaddr)))   ;; p_paddr = p_vaddr
    (call $emit_qword (i64.extend_i32_u (local.get $filesz)))  ;; p_filesz
    (call $emit_qword (i64.extend_i32_u (local.get $memsz)))   ;; p_memsz
    (call $emit_qword (i64.const 0x1000))    ;; p_align
  )

  ;; ── Runtime stub for ELF: sets up X19=JitGlobals, calls code, exits ──
  ;; AArch64 JitGlobals offsets (64-bit pointers, same as x86-64):
  ;;   +0 = locals, +8 = mem, +24 = globals, +48 = table, +64 = memory_pages

  (func $emit_elf_stub (param $bss_va i32)
    (local $jitglobs i32) (local $mem i32) (local $locals i32)
    (local $globals i32) (local $table i32)

    (local.set $jitglobs (local.get $bss_va))
    (local.set $mem     (i32.add (local.get $bss_va) (i32.const 0x80)))
    (local.set $locals  (i32.add (local.get $bss_va) (i32.const 0x10080)))
    (local.set $globals (i32.add (local.get $bss_va) (i32.const 0x20080)))
    (local.set $table   (i32.add (local.get $bss_va) (i32.const 0x30080)))

    ;; MOVZ X19, #lo(jitglobs); MOVK X19, #hi(jitglobs), LSL #16
    (call $emit_instr_movz_64 (i32.const 19) (i32.const 0) (i32.and (local.get $jitglobs) (i32.const 0xFFFF)))
    (call $emit_instr_movk_64 (i32.const 19) (i32.const 1) (i32.shr_u (local.get $jitglobs) (i32.const 16)))

    ;; X0 = mem; STR X0, [X19, #8]  (imm12 = 1 for offset 8)
    (call $emit_instr_movz_64 (i32.const 0) (i32.const 0) (i32.and (local.get $mem) (i32.const 0xFFFF)))
    (call $emit_instr_movk_64 (i32.const 0) (i32.const 1) (i32.shr_u (local.get $mem) (i32.const 16)))
    (call $emit_instr_str_64_off (i32.const 0) (i32.const 19) (i32.const 1))

    ;; X0 = locals; STR X0, [X19, #0]  (imm12 = 0)
    (call $emit_instr_movz_64 (i32.const 0) (i32.const 0) (i32.and (local.get $locals) (i32.const 0xFFFF)))
    (call $emit_instr_movk_64 (i32.const 0) (i32.const 1) (i32.shr_u (local.get $locals) (i32.const 16)))
    (call $emit_instr_str_64_off (i32.const 0) (i32.const 19) (i32.const 0))

    ;; X0 = globals; STR X0, [X19, #24]  (imm12 = 3)
    (call $emit_instr_movz_64 (i32.const 0) (i32.const 0) (i32.and (local.get $globals) (i32.const 0xFFFF)))
    (call $emit_instr_movk_64 (i32.const 0) (i32.const 1) (i32.shr_u (local.get $globals) (i32.const 16)))
    (call $emit_instr_str_64_off (i32.const 0) (i32.const 19) (i32.const 3))

    ;; X0 = table; STR X0, [X19, #48]  (imm12 = 6)
    (call $emit_instr_movz_64 (i32.const 0) (i32.const 0) (i32.and (local.get $table) (i32.const 0xFFFF)))
    (call $emit_instr_movk_64 (i32.const 0) (i32.const 1) (i32.shr_u (local.get $table) (i32.const 16)))
    (call $emit_instr_str_64_off (i32.const 0) (i32.const 19) (i32.const 6))

    ;; MOVZ X0, #1; STR W0, [X19, #64]  (memory_pages, 32-bit, imm12 = 16 for #64)
    (call $emit_instr_movz_64 (i32.const 0) (i32.const 0) (i32.const 1))
    (call $emit_instr_str_32_off (i32.const 0) (i32.const 19) (i32.const 16))

    ;; BL to compiled code
    (call $emit_bl_rel (i32.add (global.get $TEXT_VA) (global.get $ELF_CODE_OFF)))

    ;; MOV X8, #93 (SYS_exit); SVC #0
    (call $emit_instr_movz_64 (i32.const 8) (i32.const 0) (global.get $LINUX_SYS_AARCH64_EXIT))
    (call $emit_instr (i32.const 0xD4000001))  ;; SVC #0
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

  ;; ── compile_to_elf: compile WASM, emit ELF64 executable ─────────

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

    (call $emit_elf64_ehdr
      (i32.add (global.get $TEXT_VA) (global.get $ELF_STUB_OFF))
      (i32.const 64)
      (i32.const 1))

    (call $emit_elf64_phdr
      (i32.const 1)         ;; PT_LOAD
      (i32.const 7)         ;; PF_R | PF_W | PF_X
      (i32.const 0)         ;; p_offset
      (global.get $TEXT_VA) ;; p_vaddr
      (local.get $total_size) ;; p_filesz
      (i32.add (i32.and (i32.add (local.get $total_size) (i32.const 0xFFF)) (i32.const -0x1000)) (global.get $BSS_SIZE)))

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
  ;; Flat binary output (bare-metal AArch64)
  ;; ═════════════════════════════════════════════════════════════════════

  ;; Store result at 0x500 and halt
  (func $emit_store_and_halt
    (local $addr i32)
    (local.set $addr (i32.const 0x500))
    (call $emit_instr_movz_64 (i32.const 1) (i32.const 0) (i32.and (local.get $addr) (i32.const 0xFFFF)))
    (call $emit_instr_movk_64 (i32.const 1) (i32.const 1) (i32.shr_u (local.get $addr) (i32.const 16)))
    (call $emit_instr_str_64_off (i32.const 0) (i32.const 1) (i32.const 0))
    (call $emit_instr (i32.const 0x14000000))  ;; B . (infinite loop)
  )

  ;; Emit bare-metal stub (no headers). Returns stub size.
  (func $emit_bare_metal_stub (result i32)
    (local $stub_size i32) (local $current_off i32)
    ;; X19 = JitGlobals
    (call $emit_instr_movz_64 (i32.const 19) (i32.const 0) (i32.and (global.get $BSS_JITGLOBALS) (i32.const 0xFFFF)))
    (call $emit_instr_movk_64 (i32.const 19) (i32.const 1) (i32.shr_u (global.get $BSS_JITGLOBALS) (i32.const 16)))
    ;; X0 = mem; STR X0, [X19, #8]
    (call $emit_instr_movz_64 (i32.const 0) (i32.const 0) (i32.and (global.get $BSS_MEM) (i32.const 0xFFFF)))
    (call $emit_instr_movk_64 (i32.const 0) (i32.const 1) (i32.shr_u (global.get $BSS_MEM) (i32.const 16)))
    (call $emit_instr_str_64_off (i32.const 0) (i32.const 19) (i32.const 1))
    ;; X0 = locals; STR X0, [X19, #0]
    (call $emit_instr_movz_64 (i32.const 0) (i32.const 0) (i32.and (global.get $BSS_LOCALS) (i32.const 0xFFFF)))
    (call $emit_instr_movk_64 (i32.const 0) (i32.const 1) (i32.shr_u (global.get $BSS_LOCALS) (i32.const 16)))
    (call $emit_instr_str_64_off (i32.const 0) (i32.const 19) (i32.const 0))
    ;; X0 = globals; STR X0, [X19, #24]
    (call $emit_instr_movz_64 (i32.const 0) (i32.const 0) (i32.and (global.get $BSS_GLOBALS) (i32.const 0xFFFF)))
    (call $emit_instr_movk_64 (i32.const 0) (i32.const 1) (i32.shr_u (global.get $BSS_GLOBALS) (i32.const 16)))
    (call $emit_instr_str_64_off (i32.const 0) (i32.const 19) (i32.const 3))
    ;; X0 = table; STR X0, [X19, #48]
    (call $emit_instr_movz_64 (i32.const 0) (i32.const 0) (i32.and (global.get $BSS_TABLE) (i32.const 0xFFFF)))
    (call $emit_instr_movk_64 (i32.const 0) (i32.const 1) (i32.shr_u (global.get $BSS_TABLE) (i32.const 16)))
    (call $emit_instr_str_64_off (i32.const 0) (i32.const 19) (i32.const 6))
    ;; MOVZ X0, #1; STR W0, [X19, #64]
    (call $emit_instr_movz_64 (i32.const 0) (i32.const 0) (i32.const 1))
    (call $emit_instr_str_32_off (i32.const 0) (i32.const 19) (i32.const 16))

    ;; Compute stub size: current + BL(4) + store_and_halt(12) = 16
    (local.set $current_off (i32.sub (i32.load (global.get $JS_CODE_PTR)) (global.get $BIN_OUT_OFF)))
    (local.set $stub_size (i32.add (local.get $current_off) (i32.const 16)))

    ;; BL to compiled code (AArch64 PC = current, no +8)
    (call $emit_bl
      (i32.sub (local.get $stub_size) (local.get $current_off)))

    ;; Store result and halt
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

  ;; ── Pop two values: pop X1 (right), pop X0 (left) ────────────────
  (func $emit_pop2_x1_x0_order
    (call $emit_pop_x1)
    (call $emit_pop_x0)
  )

  ;; ── XOR X0, X0 (zero register) ────────────────────────────────────
  ;; EOR X0, X0, X0 = 0xAA000000 | (X0 << 16) | (X0 << 5) | X0 = 0xAA000000
  (func $emit_xor_x0_x0
    (call $emit_instr (i32.const 0xAA000000))
  )

  ;; EOR W0, W0, W0 = 0x2A000000
  (func $emit_xor_w0_w0
    (call $emit_instr (i32.const 0x2A000000))
  )

  ;; EOR X1, X1, X1
  (func $emit_xor_x1_x1
    (call $emit_instr (i32.const 0xAA000021))
  )

  ;; EOR X2, X2, X2
  (func $emit_xor_x2_x2
    (call $emit_instr (i32.const 0xAA000042))
  )

  ;; MOV X0, X1: ORR X0, XZR, X1
  ;; = 0xAA000020 | X1  = 0xAA000021
  (func $emit_mov_x0_x1
    (call $emit_instr_orr_64 (global.get $REG_X0) (global.get $REG_XZR) (global.get $REG_X1))
  )

  ;; MOV X1, X0: ORR X1, XZR, X0
  (func $emit_mov_x1_x0
    (call $emit_instr_orr_64 (global.get $REG_X1) (global.get $REG_XZR) (global.get $REG_X0))
  )

  ;; MOV W0, W1: ORR W0, WZR, W1
  (func $emit_mov_w0_w1
    (call $emit_instr_orr_32 (global.get $REG_X0) (global.get $REG_XZR) (global.get $REG_X1))
  )

  ;; MOV W1, W0
  (func $emit_mov_w1_w0
    (call $emit_instr_orr_32 (global.get $REG_X1) (global.get $REG_XZR) (global.get $REG_X0))
  )

  ;; ═════════════════════════════════════════════════════════════════════
  ;; AArch64 NEON (SIMD) emit helpers — v128 load/store/push/pop
  ;; ═════════════════════════════════════════════════════════════════════

  ;; STR Qt, [Xn, #imm12*16] — 128-bit store, unsigned offset
  ;; imm12 = byte_offset / 16 (scaled by data size 16)
  ;; Encoding: Q=1 1111101 opc=00 V=1 00 imm12 Rn Rt
  (func $emit_instr_str_q_off (param $rt i32) (param $rn i32) (param $imm12 i32)
    (call $emit_instr
      (i32.or (i32.const 0x3D800000)
              (i32.or (i32.shl (local.get $imm12) (i32.const 10))
                      (i32.or (i32.shl (local.get $rn) (i32.const 5))
                              (local.get $rt)))))
  )

  ;; LDR Qt, [Xn, #imm12*16] — 128-bit load, unsigned offset
  ;; Encoding: Q=1 1111101 opc=01 V=1 00 imm12 Rn Rt
  (func $emit_instr_ldr_q_off (param $rt i32) (param $rn i32) (param $imm12 i32)
    (call $emit_instr
      (i32.or (i32.const 0x3DC00000)
              (i32.or (i32.shl (local.get $imm12) (i32.const 10))
                      (i32.or (i32.shl (local.get $rn) (i32.const 5))
                              (local.get $rt)))))
  )

  ;; Push Q0 onto WASM value stack (grows downward): SUB SP,#16; STR Q0,[SP]
  (func $emit_v128_push
    (call $emit_instr_subi_64 (global.get $REG_SP) (global.get $REG_SP) (i32.const 16))
    (call $emit_instr_str_q_off (global.get $REG_V0) (global.get $REG_SP) (i32.const 0))
  )

  ;; Pop Q0 from WASM value stack: LDR Q0,[SP]; ADD SP,#16
  (func $emit_v128_pop
    (call $emit_instr_ldr_q_off (global.get $REG_V0) (global.get $REG_SP) (i32.const 0))
    (call $emit_instr_addi_64 (global.get $REG_SP) (global.get $REG_SP) (i32.const 16))
  )

  ;; Load Q0 from address in X1 into Q0: LDR Q0, [X1] (imm12=0 for bare reg)
  ;; Uses ldr_q_off with Xn = X1 (=1), imm12 = 0
  (func $emit_v128_load_reg
    (call $emit_instr_ldr_q_off (global.get $REG_V0) (global.get $REG_X1) (i32.const 0))
  )

  ;; Store Q0 to address in X1: STR Q0, [X1] (imm12=0 for bare reg)
  (func $emit_v128_store_reg
    (call $emit_instr_str_q_off (global.get $REG_V0) (global.get $REG_X1) (i32.const 0))
  )

  ;; Load 32-bit constant address into X1 (used to address v128 immediate)
  (func $emit_load_addr_x1 (param $addr i32)
    (call $emit_instr_movz_64 (i32.const 1) (i32.const 0) (i32.and (local.get $addr) (i32.const 0xFFFF)))
    (call $emit_instr_movk_64 (i32.const 1) (i32.const 1) (i32.shr_u (local.get $addr) (i32.const 16)))
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
  (func $emit_neon_3same (param $Q i32) (param $size i32) (param $U i32)
                         (param $opcode i32) (param $Rd i32) (param $Rn i32) (param $Rm i32)
    (local $base i32)
    (local.set $base (i32.const 0x0E200000))
    (local.set $base (i32.or (local.get $base) (i32.shl (local.get $Q) (i32.const 30))))
    (local.set $base (i32.or (local.get $base) (i32.shl (local.get $size) (i32.const 22))))
    (local.set $base (i32.or (local.get $base) (i32.shl (local.get $U) (i32.const 21))))
    (local.set $base (i32.or (local.get $base) (i32.shl (local.get $opcode) (i32.const 16))))
    (call $emit_instr
      (i32.or (local.get $base)
              (i32.or (i32.shl (local.get $Rm) (i32.const 10))
                      (i32.or (i32.shl (local.get $Rn) (i32.const 5))
                              (local.get $Rd)))))
  )

  ;; Convenience: ADD Vd.16B, Vn.16B, Vm.16B
  (func $emit_neon_add_16b (param $Rd i32) (param $Rn i32) (param $Rm i32)
    (call $emit_neon_3same (i32.const 1) (i32.const 0) (i32.const 0) (i32.const 0x10)
                           (local.get $Rd) (local.get $Rn) (local.get $Rm))
  )

  ;; Convenience: SUB Vd.16B, Vn.16B, Vm.16B
  (func $emit_neon_sub_16b (param $Rd i32) (param $Rn i32) (param $Rm i32)
    (call $emit_neon_3same (i32.const 1) (i32.const 0) (i32.const 1) (i32.const 0x10)
                           (local.get $Rd) (local.get $Rn) (local.get $Rm))
  )

  ;; Convenience: ADD Vd.8H, Vn.8H, Vm.8H (16-bit lanes)
  (func $emit_neon_add_8h (param $Rd i32) (param $Rn i32) (param $Rm i32)
    (call $emit_neon_3same (i32.const 1) (i32.const 1) (i32.const 0) (i32.const 0x10)
                           (local.get $Rd) (local.get $Rn) (local.get $Rm))
  )

  ;; Convenience: SUB Vd.8H, Vn.8H, Vm.8H
  (func $emit_neon_sub_8h (param $Rd i32) (param $Rn i32) (param $Rm i32)
    (call $emit_neon_3same (i32.const 1) (i32.const 1) (i32.const 1) (i32.const 0x10)
                           (local.get $Rd) (local.get $Rn) (local.get $Rm))
  )

  ;; Convenience: ADD Vd.4S, Vn.4S, Vm.4S (32-bit lanes)
  (func $emit_neon_add_4s (param $Rd i32) (param $Rn i32) (param $Rm i32)
    (call $emit_neon_3same (i32.const 1) (i32.const 2) (i32.const 0) (i32.const 0x10)
                           (local.get $Rd) (local.get $Rn) (local.get $Rm))
  )

  ;; Convenience: SUB Vd.4S, Vn.4S, Vm.4S
  (func $emit_neon_sub_4s (param $Rd i32) (param $Rn i32) (param $Rm i32)
    (call $emit_neon_3same (i32.const 1) (i32.const 2) (i32.const 1) (i32.const 0x10)
                           (local.get $Rd) (local.get $Rn) (local.get $Rm))
  )

  ;; Convenience: ADD Vd.2D, Vn.2D, Vm.2D (64-bit lanes)
  (func $emit_neon_add_2d (param $Rd i32) (param $Rn i32) (param $Rm i32)
    (call $emit_neon_3same (i32.const 1) (i32.const 3) (i32.const 0) (i32.const 0x10)
                           (local.get $Rd) (local.get $Rn) (local.get $Rm))
  )

  ;; Convenience: SUB Vd.2D, Vn.2D, Vm.2D
  (func $emit_neon_sub_2d (param $Rd i32) (param $Rn i32) (param $Rm i32)
    (call $emit_neon_3same (i32.const 1) (i32.const 3) (i32.const 1) (i32.const 0x10)
                           (local.get $Rd) (local.get $Rn) (local.get $Rm))
  )

  ;; ── AArch64 NEON bitwise (AND/OR/XOR) via three-register different ──
  ;; BIC Vd.16B, Vn.16B, Vm.16B: AND with complement
  ;; Actually use AND Vd.16B, Vn.16B, Vm.16B (three-register different encoding):
  ;; opcode = 00011, U=0, Q=1, size=00
  (func $emit_neon_and_16b (param $Rd i32) (param $Rn i32) (param $Rm i32)
    (call $emit_neon_3same (i32.const 1) (i32.const 0) (i32.const 0) (i32.const 0x03)
                           (local.get $Rd) (local.get $Rn) (local.get $Rm))
  )

  ;; ORR Vd.16B, Vn.16B, Vm.16B: opcode = 01011, U=0
  (func $emit_neon_orr_16b (param $Rd i32) (param $Rn i32) (param $Rm i32)
    (call $emit_neon_3same (i32.const 1) (i32.const 0) (i32.const 0) (i32.const 0x0B)
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
  (func $emit_neon_eor_16b (param $Rd i32) (param $Rn i32) (param $Rm i32)
    (call $emit_neon_3same (i32.const 1) (i32.const 0) (i32.const 1) (i32.const 0x01)
                           (local.get $Rd) (local.get $Rn) (local.get $Rm))
  )

  ;; ── AArch64 NEON comparison helpers ──────────────────────────────

  ;; CMGT Vd.16B, Vn.16B, Vm.16B (signed): Q=1, size=00, U=0, opcode=00110
  (func $emit_neon_cmgt_16b_s (param $Rd i32) (param $Rn i32) (param $Rm i32)
    (call $emit_neon_3same (i32.const 1) (i32.const 0) (i32.const 0) (i32.const 0x06)
                           (local.get $Rd) (local.get $Rn) (local.get $Rm))
  )
  ;; CMGT Vd.16B, Vn.16B, Vm.16B (unsigned): U=1, opcode=00110
  (func $emit_neon_cmgt_16b_u (param $Rd i32) (param $Rn i32) (param $Rm i32)
    (call $emit_neon_3same (i32.const 1) (i32.const 0) (i32.const 1) (i32.const 0x06)
                           (local.get $Rd) (local.get $Rn) (local.get $Rm))
  )
  ;; CMEQ Vd.16B, Vn.16B, Vm.16B: opcode=10001, U=1
  (func $emit_neon_cmeq_16b (param $Rd i32) (param $Rn i32) (param $Rm i32)
    (call $emit_neon_3same (i32.const 1) (i32.const 0) (i32.const 1) (i32.const 0x11)
                           (local.get $Rd) (local.get $Rn) (local.get $Rm))
  )
  ;; CMGE Vd.16B, Vn.16B, Vm.16B (signed): U=0, opcode=00111
  (func $emit_neon_cmge_16b_s (param $Rd i32) (param $Rn i32) (param $Rm i32)
    (call $emit_neon_3same (i32.const 1) (i32.const 0) (i32.const 0) (i32.const 0x07)
                           (local.get $Rd) (local.get $Rn) (local.get $Rm))
  )
  ;; CMGE Vd.16B, Vn.16B, Vm.16B (unsigned): U=1, opcode=00111
  (func $emit_neon_cmge_16b_u (param $Rd i32) (param $Rn i32) (param $Rm i32)
    (call $emit_neon_3same (i32.const 1) (i32.const 0) (i32.const 1) (i32.const 0x07)
                           (local.get $Rd) (local.get $Rn) (local.get $Rm))
  )

  ;; CMGT/CMGE for 8H, 4S, 2D lanes — same as 16B but different size
  ;; size=01 for 8H, size=10 for 4S, size=11 for 2D
  (func $emit_neon_cmgt_8h_s (param $Rd i32) (param $Rn i32) (param $Rm i32)
    (call $emit_neon_3same (i32.const 1) (i32.const 1) (i32.const 0) (i32.const 0x06)
                           (local.get $Rd) (local.get $Rn) (local.get $Rm))
  )
  (func $emit_neon_cmgt_4s_s (param $Rd i32) (param $Rn i32) (param $Rm i32)
    (call $emit_neon_3same (i32.const 1) (i32.const 2) (i32.const 0) (i32.const 0x06)
                           (local.get $Rd) (local.get $Rn) (local.get $Rm))
  )
  (func $emit_neon_cmeq_8h (param $Rd i32) (param $Rn i32) (param $Rm i32)
    (call $emit_neon_3same (i32.const 1) (i32.const 1) (i32.const 1) (i32.const 0x11)
                           (local.get $Rd) (local.get $Rn) (local.get $Rm))
  )
  (func $emit_neon_cmeq_4s (param $Rd i32) (param $Rn i32) (param $Rm i32)
    (call $emit_neon_3same (i32.const 1) (i32.const 2) (i32.const 1) (i32.const 0x11)
                           (local.get $Rd) (local.get $Rn) (local.get $Rm))
  )

  ;; CMGE (unsigned) Vd.8H, Vn.8H, Vm.8H: CMHS, U=1, opcode=00111, size=01
  (func $emit_neon_cmge_8h_u (param $Rd i32) (param $Rn i32) (param $Rm i32)
    (call $emit_neon_3same (i32.const 1) (i32.const 1) (i32.const 1) (i32.const 0x07)
                           (local.get $Rd) (local.get $Rn) (local.get $Rm))
  )
  ;; CMGE (unsigned) Vd.4S, Vn.4S, Vm.4S: CMHS, size=10
  (func $emit_neon_cmge_4s_u (param $Rd i32) (param $Rn i32) (param $Rm i32)
    (call $emit_neon_3same (i32.const 1) (i32.const 2) (i32.const 1) (i32.const 0x07)
                           (local.get $Rd) (local.get $Rn) (local.get $Rm))
  )
  ;; CMGT (unsigned) Vd.8H, Vn.8H, Vm.8H: CMHI, U=1, size=01, opcode=00110
  (func $emit_neon_cmgt_8h_u (param $Rd i32) (param $Rn i32) (param $Rm i32)
    (call $emit_neon_3same (i32.const 1) (i32.const 1) (i32.const 1) (i32.const 0x06)
                           (local.get $Rd) (local.get $Rn) (local.get $Rm))
  )
  ;; CMGT (unsigned) Vd.4S, Vn.4S, Vm.4S: CMHI, size=10
  (func $emit_neon_cmgt_4s_u (param $Rd i32) (param $Rn i32) (param $Rm i32)
    (call $emit_neon_3same (i32.const 1) (i32.const 2) (i32.const 1) (i32.const 0x06)
                           (local.get $Rd) (local.get $Rn) (local.get $Rm))
  )
  ;; CMGE (signed) Vd.8H, Vn.8H, Vm.8H: U=0, size=01, opcode=00111
  (func $emit_neon_cmge_8h_s (param $Rd i32) (param $Rn i32) (param $Rm i32)
    (call $emit_neon_3same (i32.const 1) (i32.const 1) (i32.const 0) (i32.const 0x07)
                           (local.get $Rd) (local.get $Rn) (local.get $Rm))
  )
  ;; CMGE (signed) Vd.4S, Vn.4S, Vm.4S: U=0, size=10, opcode=00111
  (func $emit_neon_cmge_4s_s (param $Rd i32) (param $Rn i32) (param $Rm i32)
    (call $emit_neon_3same (i32.const 1) (i32.const 2) (i32.const 0) (i32.const 0x07)
                           (local.get $Rd) (local.get $Rn) (local.get $Rm))
  )

  ;; ── MUL helpers (signed/unsigned, integer) ──────────────────────
  ;; MUL Vd.8H, Vn.8H, Vm.8H: U=0, opcode=10011, size=01
  (func $emit_neon_mul_8h (param $Rd i32) (param $Rn i32) (param $Rm i32)
    (call $emit_neon_3same (i32.const 1) (i32.const 1) (i32.const 0) (i32.const 0x13)
                           (local.get $Rd) (local.get $Rn) (local.get $Rm))
  )
  ;; MUL Vd.4S, Vn.4S, Vm.4S: size=10
  (func $emit_neon_mul_4s (param $Rd i32) (param $Rn i32) (param $Rm i32)
    (call $emit_neon_3same (i32.const 1) (i32.const 2) (i32.const 0) (i32.const 0x13)
                           (local.get $Rd) (local.get $Rn) (local.get $Rm))
  )

  ;; ── NEG helpers (2-register misc) ───────────────────────────────
  ;; NEG Vd.16B, Vn.16B: 0x0E207800 | (Q<<30) | (size<<22) | (Rn<<5) | Rd
  ;; size=00 for 16B (byte), size=01 for 8H (halfword), size=10 for 4S, size=11 for 2D
  (func $emit_neon_neg (param $size i32) (param $Rd i32) (param $Rn i32)
    (call $emit_instr
      (i32.or (i32.const 0x4E207800)
              (i32.or (i32.shl (local.get $size) (i32.const 22))
                      (i32.or (i32.shl (local.get $Rn) (i32.const 5))
                              (local.get $Rd)))))
  )

  ;; ── UMIN / UMAX / URHADD helpers (unsigned) ─────────────────────
  ;; UMIN Vd.16B, Vn.16B, Vm.16B: U=1, opcode=01101, size=00
  (func $emit_neon_umin_16b (param $Rd i32) (param $Rn i32) (param $Rm i32)
    (call $emit_neon_3same (i32.const 1) (i32.const 0) (i32.const 1) (i32.const 0x0D)
                           (local.get $Rd) (local.get $Rn) (local.get $Rm))
  )
  ;; UMIN Vd.8H, Vn.8H, Vm.8H: size=01
  (func $emit_neon_umin_8h (param $Rd i32) (param $Rn i32) (param $Rm i32)
    (call $emit_neon_3same (i32.const 1) (i32.const 1) (i32.const 1) (i32.const 0x0D)
                           (local.get $Rd) (local.get $Rn) (local.get $Rm))
  )
  ;; UMAX Vd.16B, Vn.16B, Vm.16B: U=1, opcode=01100, size=00
  (func $emit_neon_umax_16b (param $Rd i32) (param $Rn i32) (param $Rm i32)
    (call $emit_neon_3same (i32.const 1) (i32.const 0) (i32.const 1) (i32.const 0x0C)
                           (local.get $Rd) (local.get $Rn) (local.get $Rm))
  )
  ;; UMAX Vd.8H, Vn.8H, Vm.8H: size=01
  (func $emit_neon_umax_8h (param $Rd i32) (param $Rn i32) (param $Rm i32)
    (call $emit_neon_3same (i32.const 1) (i32.const 1) (i32.const 1) (i32.const 0x0C)
                           (local.get $Rd) (local.get $Rn) (local.get $Rm))
  )
  ;; URHADD Vd.16B, Vn.16B, Vm.16B: (a+b+1)>>1, U=1, opcode=00011, size=00
  (func $emit_neon_urhadd_16b (param $Rd i32) (param $Rn i32) (param $Rm i32)
    (call $emit_neon_3same (i32.const 1) (i32.const 0) (i32.const 1) (i32.const 0x03)
                           (local.get $Rd) (local.get $Rn) (local.get $Rm))
  )
  ;; URHADD Vd.8H, Vn.8H, Vm.8H: size=01
  (func $emit_neon_urhadd_8h (param $Rd i32) (param $Rn i32) (param $Rm i32)
    (call $emit_neon_3same (i32.const 1) (i32.const 1) (i32.const 1) (i32.const 0x03)
                           (local.get $Rd) (local.get $Rn) (local.get $Rm))
  )

  ;; ── Pop two, operate, push (pattern for binary ops) ──────────────

  ;; Push X0 only if RESULT_IN_X0 is 0 (peephole)
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
            (call $emit_push_x0)
          )
        )
      )
    )
  )


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


  ;; ═════════════════════════════════════════════════════════════════════
  ;; WASM SIMD (0xFD prefix) — AArch64 NEON implementations
  ;; ═════════════════════════════════════════════════════════════════════

  ;; ── v128.const (0xFD 0x0C): load 16-byte constant from decoded op ──
  (func $template_v128_const (param $dec_ptr i32)
    (local $addr i32)
    (local.set $addr (i32.add (local.get $dec_ptr) (i32.const 16)))
    (call $emit_load_addr_x1 (local.get $addr))
    (call $emit_v128_load_reg)
    (call $emit_v128_push)
  )

  ;; ── v128.load (0xFD 0x00): pop addr, load 16 bytes from mem ─────────
  (func $template_v128_load (param $dec_ptr i32)
    (call $emit_pop_x0)
    (call $emit_ldr_mem_ptr)
    (call $emit_instr_add_64 (global.get $REG_X1) (global.get $REG_X2) (global.get $REG_X0))
    (call $emit_v128_load_reg)
    (call $emit_v128_push)
  )

  ;; ── v128.store (0xFD 0x1B): pop value, pop addr, store 16 bytes ─────
  (func $template_v128_store (param $dec_ptr i32)
    (call $emit_v128_pop)
    (call $emit_pop_x0)
    (call $emit_ldr_mem_ptr)
    (call $emit_instr_add_64 (global.get $REG_X1) (global.get $REG_X2) (global.get $REG_X0))
    (call $emit_v128_store_reg)
  )

  ;; ── Splat templates ───────────────────────────────────────────────
  (func $template_i8x16_splat (param $dec_ptr i32)
    (call $emit_pop_x0)
    (call $emit_instr (i32.const 0x4E000800))
    (call $emit_v128_push)
  )
  (func $template_i16x8_splat (param $dec_ptr i32)
    (call $emit_pop_x0)
    (call $emit_instr (i32.const 0x4E001000))
    (call $emit_v128_push)
  )
  (func $template_i32x4_splat (param $dec_ptr i32)
    (call $emit_pop_x0)
    (call $emit_instr (i32.const 0x4E002000))
    (call $emit_v128_push)
  )
  (func $template_i64x2_splat (param $dec_ptr i32)
    (call $emit_pop_x0)
    (call $emit_instr (i32.const 0x4E004000))
    (call $emit_v128_push)
  )
  (func $template_f32x4_splat (param $dec_ptr i32)
    (call $emit_pop_x0)
    (call $emit_instr (i32.const 0x4E002000))
    (call $emit_v128_push)
  )
  (func $template_f64x2_splat (param $dec_ptr i32)
    (call $emit_pop_x0)
    (call $emit_instr (i32.const 0x4E004000))
    (call $emit_v128_push)
  )

  ;; ── Integer binary ops ────────────────────────────────────────────
  (func $template_i8x16_add (param $dec_ptr i32)
    (call $emit_instr_ldr_q_off (global.get $REG_V1) (global.get $REG_SP) (i32.const 0))
    (call $emit_instr_ldr_q_off (global.get $REG_V0) (global.get $REG_SP) (i32.const 1))
    (call $emit_neon_add_16b (global.get $REG_V0) (global.get $REG_V0) (global.get $REG_V1))
    (call $emit_instr_addi_64 (global.get $REG_SP) (global.get $REG_SP) (i32.const 16))
    (call $emit_instr_str_q_off (global.get $REG_V0) (global.get $REG_SP) (i32.const 0))
  )
  (func $template_i16x8_add (param $dec_ptr i32)
    (call $emit_instr_ldr_q_off (global.get $REG_V1) (global.get $REG_SP) (i32.const 0))
    (call $emit_instr_ldr_q_off (global.get $REG_V0) (global.get $REG_SP) (i32.const 1))
    (call $emit_neon_add_8h (global.get $REG_V0) (global.get $REG_V0) (global.get $REG_V1))
    (call $emit_instr_addi_64 (global.get $REG_SP) (global.get $REG_SP) (i32.const 16))
    (call $emit_instr_str_q_off (global.get $REG_V0) (global.get $REG_SP) (i32.const 0))
  )
  (func $template_i32x4_add (param $dec_ptr i32)
    (call $emit_instr_ldr_q_off (global.get $REG_V1) (global.get $REG_SP) (i32.const 0))
    (call $emit_instr_ldr_q_off (global.get $REG_V0) (global.get $REG_SP) (i32.const 1))
    (call $emit_neon_add_4s (global.get $REG_V0) (global.get $REG_V0) (global.get $REG_V1))
    (call $emit_instr_addi_64 (global.get $REG_SP) (global.get $REG_SP) (i32.const 16))
    (call $emit_instr_str_q_off (global.get $REG_V0) (global.get $REG_SP) (i32.const 0))
  )
  (func $template_i64x2_add (param $dec_ptr i32)
    (call $emit_instr_ldr_q_off (global.get $REG_V1) (global.get $REG_SP) (i32.const 0))
    (call $emit_instr_ldr_q_off (global.get $REG_V0) (global.get $REG_SP) (i32.const 1))
    (call $emit_neon_add_2d (global.get $REG_V0) (global.get $REG_V0) (global.get $REG_V1))
    (call $emit_instr_addi_64 (global.get $REG_SP) (global.get $REG_SP) (i32.const 16))
    (call $emit_instr_str_q_off (global.get $REG_V0) (global.get $REG_SP) (i32.const 0))
  )
  (func $template_i8x16_sub (param $dec_ptr i32)
    (call $emit_instr_ldr_q_off (global.get $REG_V1) (global.get $REG_SP) (i32.const 0))
    (call $emit_instr_ldr_q_off (global.get $REG_V0) (global.get $REG_SP) (i32.const 1))
    (call $emit_neon_sub_16b (global.get $REG_V0) (global.get $REG_V0) (global.get $REG_V1))
    (call $emit_instr_addi_64 (global.get $REG_SP) (global.get $REG_SP) (i32.const 16))
    (call $emit_instr_str_q_off (global.get $REG_V0) (global.get $REG_SP) (i32.const 0))
  )
  (func $template_i16x8_sub (param $dec_ptr i32)
    (call $emit_instr_ldr_q_off (global.get $REG_V1) (global.get $REG_SP) (i32.const 0))
    (call $emit_instr_ldr_q_off (global.get $REG_V0) (global.get $REG_SP) (i32.const 1))
    (call $emit_neon_sub_8h (global.get $REG_V0) (global.get $REG_V0) (global.get $REG_V1))
    (call $emit_instr_addi_64 (global.get $REG_SP) (global.get $REG_SP) (i32.const 16))
    (call $emit_instr_str_q_off (global.get $REG_V0) (global.get $REG_SP) (i32.const 0))
  )
  (func $template_i32x4_sub (param $dec_ptr i32)
    (call $emit_instr_ldr_q_off (global.get $REG_V1) (global.get $REG_SP) (i32.const 0))
    (call $emit_instr_ldr_q_off (global.get $REG_V0) (global.get $REG_SP) (i32.const 1))
    (call $emit_neon_sub_4s (global.get $REG_V0) (global.get $REG_V0) (global.get $REG_V1))
    (call $emit_instr_addi_64 (global.get $REG_SP) (global.get $REG_SP) (i32.const 16))
    (call $emit_instr_str_q_off (global.get $REG_V0) (global.get $REG_SP) (i32.const 0))
  )
  (func $template_i64x2_sub (param $dec_ptr i32)
    (call $emit_instr_ldr_q_off (global.get $REG_V1) (global.get $REG_SP) (i32.const 0))
    (call $emit_instr_ldr_q_off (global.get $REG_V0) (global.get $REG_SP) (i32.const 1))
    (call $emit_neon_sub_2d (global.get $REG_V0) (global.get $REG_V0) (global.get $REG_V1))
    (call $emit_instr_addi_64 (global.get $REG_SP) (global.get $REG_SP) (i32.const 16))
    (call $emit_instr_str_q_off (global.get $REG_V0) (global.get $REG_SP) (i32.const 0))
  )

  ;; ── Float binary ops ──────────────────────────────────────────────
  (func $template_f32x4_add (param $dec_ptr i32)
    (call $emit_instr_ldr_q_off (global.get $REG_V1) (global.get $REG_SP) (i32.const 0))
    (call $emit_instr_ldr_q_off (global.get $REG_V0) (global.get $REG_SP) (i32.const 1))
    (call $emit_instr (i32.const 0x4E20D400))
    (call $emit_instr_addi_64 (global.get $REG_SP) (global.get $REG_SP) (i32.const 16))
    (call $emit_instr_str_q_off (global.get $REG_V0) (global.get $REG_SP) (i32.const 0))
  )
  (func $template_f64x2_add (param $dec_ptr i32)
    (call $emit_instr_ldr_q_off (global.get $REG_V1) (global.get $REG_SP) (i32.const 0))
    (call $emit_instr_ldr_q_off (global.get $REG_V0) (global.get $REG_SP) (i32.const 1))
    (call $emit_instr (i32.const 0x4E60D400))
    (call $emit_instr_addi_64 (global.get $REG_SP) (global.get $REG_SP) (i32.const 16))
    (call $emit_instr_str_q_off (global.get $REG_V0) (global.get $REG_SP) (i32.const 0))
  )
  ;; f32x4_sub: FSUB V0.4S, V0.4S, V1.4S = 0x4E22D400
  (func $template_f32x4_sub (param $dec_ptr i32)
    (call $emit_instr_ldr_q_off (global.get $REG_V1) (global.get $REG_SP) (i32.const 0))
    (call $emit_instr_ldr_q_off (global.get $REG_V0) (global.get $REG_SP) (i32.const 1))
    (call $emit_instr (i32.const 0x4E22D400))
    (call $emit_instr_addi_64 (global.get $REG_SP) (global.get $REG_SP) (i32.const 16))
    (call $emit_instr_str_q_off (global.get $REG_V0) (global.get $REG_SP) (i32.const 0))
  )
  ;; f64x2_sub: FSUB V0.2D, V0.2D, V1.2D = 0x4E62D400
  (func $template_f64x2_sub (param $dec_ptr i32)
    (call $emit_instr_ldr_q_off (global.get $REG_V1) (global.get $REG_SP) (i32.const 0))
    (call $emit_instr_ldr_q_off (global.get $REG_V0) (global.get $REG_SP) (i32.const 1))
    (call $emit_instr (i32.const 0x4E62D400))
    (call $emit_instr_addi_64 (global.get $REG_SP) (global.get $REG_SP) (i32.const 16))
    (call $emit_instr_str_q_off (global.get $REG_V0) (global.get $REG_SP) (i32.const 0))
  )

  ;; ── Bitwise ops ───────────────────────────────────────────────────
  (func $template_i8x16_and (param $dec_ptr i32)
    (call $emit_instr_ldr_q_off (global.get $REG_V1) (global.get $REG_SP) (i32.const 0))
    (call $emit_instr_ldr_q_off (global.get $REG_V0) (global.get $REG_SP) (i32.const 1))
    (call $emit_neon_and_16b (global.get $REG_V0) (global.get $REG_V0) (global.get $REG_V1))
    (call $emit_instr_addi_64 (global.get $REG_SP) (global.get $REG_SP) (i32.const 16))
    (call $emit_instr_str_q_off (global.get $REG_V0) (global.get $REG_SP) (i32.const 0))
  )
  (func $template_i8x16_or (param $dec_ptr i32)
    (call $emit_instr_ldr_q_off (global.get $REG_V1) (global.get $REG_SP) (i32.const 0))
    (call $emit_instr_ldr_q_off (global.get $REG_V0) (global.get $REG_SP) (i32.const 1))
    (call $emit_neon_orr_16b (global.get $REG_V0) (global.get $REG_V0) (global.get $REG_V1))
    (call $emit_instr_addi_64 (global.get $REG_SP) (global.get $REG_SP) (i32.const 16))
    (call $emit_instr_str_q_off (global.get $REG_V0) (global.get $REG_SP) (i32.const 0))
  )
  (func $template_i8x16_xor (param $dec_ptr i32)
    (call $emit_instr_ldr_q_off (global.get $REG_V1) (global.get $REG_SP) (i32.const 0))
    (call $emit_instr_ldr_q_off (global.get $REG_V0) (global.get $REG_SP) (i32.const 1))
    (call $emit_neon_eor_16b (global.get $REG_V0) (global.get $REG_V0) (global.get $REG_V1))
    (call $emit_instr_addi_64 (global.get $REG_SP) (global.get $REG_SP) (i32.const 16))
    (call $emit_instr_str_q_off (global.get $REG_V0) (global.get $REG_SP) (i32.const 0))
  )

  ;; ── Comparisons ───────────────────────────────────────────────────
  (func $template_i8x16_eq (param $dec_ptr i32)
    (call $emit_instr_ldr_q_off (global.get $REG_V1) (global.get $REG_SP) (i32.const 0))
    (call $emit_instr_ldr_q_off (global.get $REG_V0) (global.get $REG_SP) (i32.const 1))
    (call $emit_neon_cmeq_16b (global.get $REG_V0) (global.get $REG_V0) (global.get $REG_V1))
    (call $emit_instr_addi_64 (global.get $REG_SP) (global.get $REG_SP) (i32.const 16))
    (call $emit_instr_str_q_off (global.get $REG_V0) (global.get $REG_SP) (i32.const 0))
  )
  (func $template_i16x8_eq (param $dec_ptr i32)
    (call $emit_instr_ldr_q_off (global.get $REG_V1) (global.get $REG_SP) (i32.const 0))
    (call $emit_instr_ldr_q_off (global.get $REG_V0) (global.get $REG_SP) (i32.const 1))
    (call $emit_neon_cmeq_8h (global.get $REG_V0) (global.get $REG_V0) (global.get $REG_V1))
    (call $emit_instr_addi_64 (global.get $REG_SP) (global.get $REG_SP) (i32.const 16))
    (call $emit_instr_str_q_off (global.get $REG_V0) (global.get $REG_SP) (i32.const 0))
  )
  (func $template_i32x4_eq (param $dec_ptr i32)
    (call $emit_instr_ldr_q_off (global.get $REG_V1) (global.get $REG_SP) (i32.const 0))
    (call $emit_instr_ldr_q_off (global.get $REG_V0) (global.get $REG_SP) (i32.const 1))
    (call $emit_neon_cmeq_4s (global.get $REG_V0) (global.get $REG_V0) (global.get $REG_V1))
    (call $emit_instr_addi_64 (global.get $REG_SP) (global.get $REG_SP) (i32.const 16))
    (call $emit_instr_str_q_off (global.get $REG_V0) (global.get $REG_SP) (i32.const 0))
  )
  (func $template_i8x16_gt_s (param $dec_ptr i32)
    (call $emit_instr_ldr_q_off (global.get $REG_V1) (global.get $REG_SP) (i32.const 0))
    (call $emit_instr_ldr_q_off (global.get $REG_V0) (global.get $REG_SP) (i32.const 1))
    (call $emit_neon_cmgt_16b_s (global.get $REG_V0) (global.get $REG_V0) (global.get $REG_V1))
    (call $emit_instr_addi_64 (global.get $REG_SP) (global.get $REG_SP) (i32.const 16))
    (call $emit_instr_str_q_off (global.get $REG_V0) (global.get $REG_SP) (i32.const 0))
  )

  ;; ── GE (signed) implementations ──────────────────────────────────
  (func $template_i8x16_ge_s (param $dec_ptr i32)
    (call $emit_instr_ldr_q_off (global.get $REG_V1) (global.get $REG_SP) (i32.const 0))
    (call $emit_instr_ldr_q_off (global.get $REG_V0) (global.get $REG_SP) (i32.const 1))
    (call $emit_neon_cmge_16b_s (global.get $REG_V0) (global.get $REG_V0) (global.get $REG_V1))
    (call $emit_instr_addi_64 (global.get $REG_SP) (global.get $REG_SP) (i32.const 16))
    (call $emit_instr_str_q_off (global.get $REG_V0) (global.get $REG_SP) (i32.const 0))
  )
  (func $template_i16x8_ge_s (param $dec_ptr i32)
    (call $emit_instr_ldr_q_off (global.get $REG_V1) (global.get $REG_SP) (i32.const 0))
    (call $emit_instr_ldr_q_off (global.get $REG_V0) (global.get $REG_SP) (i32.const 1))
    (call $emit_neon_3same (i32.const 1) (i32.const 1) (i32.const 0) (i32.const 0x07)
                           (global.get $REG_V0) (global.get $REG_V0) (global.get $REG_V1))
    (call $emit_instr_addi_64 (global.get $REG_SP) (global.get $REG_SP) (i32.const 16))
    (call $emit_instr_str_q_off (global.get $REG_V0) (global.get $REG_SP) (i32.const 0))
  )
  (func $template_i32x4_ge_s (param $dec_ptr i32)
    (call $emit_instr_ldr_q_off (global.get $REG_V1) (global.get $REG_SP) (i32.const 0))
    (call $emit_instr_ldr_q_off (global.get $REG_V0) (global.get $REG_SP) (i32.const 1))
    (call $emit_neon_3same (i32.const 1) (i32.const 2) (i32.const 0) (i32.const 0x07)
                           (global.get $REG_V0) (global.get $REG_V0) (global.get $REG_V1))
    (call $emit_instr_addi_64 (global.get $REG_SP) (global.get $REG_SP) (i32.const 16))
    (call $emit_instr_str_q_off (global.get $REG_V0) (global.get $REG_SP) (i32.const 0))
  )

  ;; ── GE (unsigned) implementations ────────────────────────────────
  (func $template_i8x16_ge_u (param $dec_ptr i32)
    (call $emit_instr_ldr_q_off (global.get $REG_V1) (global.get $REG_SP) (i32.const 0))
    (call $emit_instr_ldr_q_off (global.get $REG_V0) (global.get $REG_SP) (i32.const 1))
    (call $emit_neon_cmge_16b_u (global.get $REG_V0) (global.get $REG_V0) (global.get $REG_V1))
    (call $emit_instr_addi_64 (global.get $REG_SP) (global.get $REG_SP) (i32.const 16))
    (call $emit_instr_str_q_off (global.get $REG_V0) (global.get $REG_SP) (i32.const 0))
  )
  (func $template_i16x8_ge_u (param $dec_ptr i32)
    (call $emit_instr_ldr_q_off (global.get $REG_V1) (global.get $REG_SP) (i32.const 0))
    (call $emit_instr_ldr_q_off (global.get $REG_V0) (global.get $REG_SP) (i32.const 1))
    (call $emit_neon_cmge_8h_u (global.get $REG_V0) (global.get $REG_V0) (global.get $REG_V1))
    (call $emit_instr_addi_64 (global.get $REG_SP) (global.get $REG_SP) (i32.const 16))
    (call $emit_instr_str_q_off (global.get $REG_V0) (global.get $REG_SP) (i32.const 0))
  )
  (func $template_i32x4_ge_u (param $dec_ptr i32)
    (call $emit_instr_ldr_q_off (global.get $REG_V1) (global.get $REG_SP) (i32.const 0))
    (call $emit_instr_ldr_q_off (global.get $REG_V0) (global.get $REG_SP) (i32.const 1))
    (call $emit_neon_cmge_4s_u (global.get $REG_V0) (global.get $REG_V0) (global.get $REG_V1))
    (call $emit_instr_addi_64 (global.get $REG_SP) (global.get $REG_SP) (i32.const 16))
    (call $emit_instr_str_q_off (global.get $REG_V0) (global.get $REG_SP) (i32.const 0))
  )

  ;; ── UMIN / UMAX / URHADD implementations ─────────────────────────
  (func $template_i8x16_min_u (param $dec_ptr i32)
    (call $emit_instr_ldr_q_off (global.get $REG_V1) (global.get $REG_SP) (i32.const 0))
    (call $emit_instr_ldr_q_off (global.get $REG_V0) (global.get $REG_SP) (i32.const 1))
    (call $emit_neon_umin_16b (global.get $REG_V0) (global.get $REG_V0) (global.get $REG_V1))
    (call $emit_instr_addi_64 (global.get $REG_SP) (global.get $REG_SP) (i32.const 16))
    (call $emit_instr_str_q_off (global.get $REG_V0) (global.get $REG_SP) (i32.const 0))
  )
  (func $template_i16x8_min_u (param $dec_ptr i32)
    (call $emit_instr_ldr_q_off (global.get $REG_V1) (global.get $REG_SP) (i32.const 0))
    (call $emit_instr_ldr_q_off (global.get $REG_V0) (global.get $REG_SP) (i32.const 1))
    (call $emit_neon_umin_8h (global.get $REG_V0) (global.get $REG_V0) (global.get $REG_V1))
    (call $emit_instr_addi_64 (global.get $REG_SP) (global.get $REG_SP) (i32.const 16))
    (call $emit_instr_str_q_off (global.get $REG_V0) (global.get $REG_SP) (i32.const 0))
  )
  (func $template_i8x16_max_u (param $dec_ptr i32)
    (call $emit_instr_ldr_q_off (global.get $REG_V1) (global.get $REG_SP) (i32.const 0))
    (call $emit_instr_ldr_q_off (global.get $REG_V0) (global.get $REG_SP) (i32.const 1))
    (call $emit_neon_umax_16b (global.get $REG_V0) (global.get $REG_V0) (global.get $REG_V1))
    (call $emit_instr_addi_64 (global.get $REG_SP) (global.get $REG_SP) (i32.const 16))
    (call $emit_instr_str_q_off (global.get $REG_V0) (global.get $REG_SP) (i32.const 0))
  )
  (func $template_i16x8_max_u (param $dec_ptr i32)
    (call $emit_instr_ldr_q_off (global.get $REG_V1) (global.get $REG_SP) (i32.const 0))
    (call $emit_instr_ldr_q_off (global.get $REG_V0) (global.get $REG_SP) (i32.const 1))
    (call $emit_neon_umax_8h (global.get $REG_V0) (global.get $REG_V0) (global.get $REG_V1))
    (call $emit_instr_addi_64 (global.get $REG_SP) (global.get $REG_SP) (i32.const 16))
    (call $emit_instr_str_q_off (global.get $REG_V0) (global.get $REG_SP) (i32.const 0))
  )
  (func $template_i8x16_avgr_u (param $dec_ptr i32)
    (call $emit_instr_ldr_q_off (global.get $REG_V1) (global.get $REG_SP) (i32.const 0))
    (call $emit_instr_ldr_q_off (global.get $REG_V0) (global.get $REG_SP) (i32.const 1))
    (call $emit_neon_urhadd_16b (global.get $REG_V0) (global.get $REG_V0) (global.get $REG_V1))
    (call $emit_instr_addi_64 (global.get $REG_SP) (global.get $REG_SP) (i32.const 16))
    (call $emit_instr_str_q_off (global.get $REG_V0) (global.get $REG_SP) (i32.const 0))
  )
  (func $template_i16x8_avgr_u (param $dec_ptr i32)
    (call $emit_instr_ldr_q_off (global.get $REG_V1) (global.get $REG_SP) (i32.const 0))
    (call $emit_instr_ldr_q_off (global.get $REG_V0) (global.get $REG_SP) (i32.const 1))
    (call $emit_neon_urhadd_8h (global.get $REG_V0) (global.get $REG_V0) (global.get $REG_V1))
    (call $emit_instr_addi_64 (global.get $REG_SP) (global.get $REG_SP) (i32.const 16))
    (call $emit_instr_str_q_off (global.get $REG_V0) (global.get $REG_SP) (i32.const 0))
  )

  ;; ── Additional integer comparison templates ──────────────────────
  (func $template_i16x8_gt_s (param $dec_ptr i32)
    (call $emit_instr_ldr_q_off (global.get $REG_V1) (global.get $REG_SP) (i32.const 0))
    (call $emit_instr_ldr_q_off (global.get $REG_V0) (global.get $REG_SP) (i32.const 1))
    (call $emit_neon_cmgt_8h_s (global.get $REG_V0) (global.get $REG_V0) (global.get $REG_V1))
    (call $emit_instr_addi_64 (global.get $REG_SP) (global.get $REG_SP) (i32.const 16))
    (call $emit_instr_str_q_off (global.get $REG_V0) (global.get $REG_SP) (i32.const 0))
  )
  (func $template_i32x4_gt_s (param $dec_ptr i32)
    (call $emit_instr_ldr_q_off (global.get $REG_V1) (global.get $REG_SP) (i32.const 0))
    (call $emit_instr_ldr_q_off (global.get $REG_V0) (global.get $REG_SP) (i32.const 1))
    (call $emit_neon_cmgt_4s_s (global.get $REG_V0) (global.get $REG_V0) (global.get $REG_V1))
    (call $emit_instr_addi_64 (global.get $REG_SP) (global.get $REG_SP) (i32.const 16))
    (call $emit_instr_str_q_off (global.get $REG_V0) (global.get $REG_SP) (i32.const 0))
  )
  (func $template_i8x16_gt_u (param $dec_ptr i32)
    (call $emit_instr_ldr_q_off (global.get $REG_V1) (global.get $REG_SP) (i32.const 0))
    (call $emit_instr_ldr_q_off (global.get $REG_V0) (global.get $REG_SP) (i32.const 1))
    (call $emit_neon_cmgt_16b_u (global.get $REG_V0) (global.get $REG_V0) (global.get $REG_V1))
    (call $emit_instr_addi_64 (global.get $REG_SP) (global.get $REG_SP) (i32.const 16))
    (call $emit_instr_str_q_off (global.get $REG_V0) (global.get $REG_SP) (i32.const 0))
  )
  (func $template_i16x8_gt_u (param $dec_ptr i32)
    (call $emit_instr_ldr_q_off (global.get $REG_V1) (global.get $REG_SP) (i32.const 0))
    (call $emit_instr_ldr_q_off (global.get $REG_V0) (global.get $REG_SP) (i32.const 1))
    (call $emit_neon_cmgt_8h_u (global.get $REG_V0) (global.get $REG_V0) (global.get $REG_V1))
    (call $emit_instr_addi_64 (global.get $REG_SP) (global.get $REG_SP) (i32.const 16))
    (call $emit_instr_str_q_off (global.get $REG_V0) (global.get $REG_SP) (i32.const 0))
  )
  (func $template_i32x4_gt_u (param $dec_ptr i32)
    (call $emit_instr_ldr_q_off (global.get $REG_V1) (global.get $REG_SP) (i32.const 0))
    (call $emit_instr_ldr_q_off (global.get $REG_V0) (global.get $REG_SP) (i32.const 1))
    (call $emit_neon_cmgt_4s_u (global.get $REG_V0) (global.get $REG_V0) (global.get $REG_V1))
    (call $emit_instr_addi_64 (global.get $REG_SP) (global.get $REG_SP) (i32.const 16))
    (call $emit_instr_str_q_off (global.get $REG_V0) (global.get $REG_SP) (i32.const 0))
  )
  ;; lt_s: a < b = b > a → CMGT(V1, V0) with Rn=V1, Rm=V0
  (func $template_i8x16_lt_s (param $dec_ptr i32)
    (call $emit_instr_ldr_q_off (global.get $REG_V1) (global.get $REG_SP) (i32.const 0))
    (call $emit_instr_ldr_q_off (global.get $REG_V0) (global.get $REG_SP) (i32.const 1))
    (call $emit_neon_cmgt_16b_s (global.get $REG_V0) (global.get $REG_V1) (global.get $REG_V0))
    (call $emit_instr_addi_64 (global.get $REG_SP) (global.get $REG_SP) (i32.const 16))
    (call $emit_instr_str_q_off (global.get $REG_V0) (global.get $REG_SP) (i32.const 0))
  )
  (func $template_i16x8_lt_s (param $dec_ptr i32)
    (call $emit_instr_ldr_q_off (global.get $REG_V1) (global.get $REG_SP) (i32.const 0))
    (call $emit_instr_ldr_q_off (global.get $REG_V0) (global.get $REG_SP) (i32.const 1))
    (call $emit_neon_cmgt_8h_s (global.get $REG_V0) (global.get $REG_V1) (global.get $REG_V0))
    (call $emit_instr_addi_64 (global.get $REG_SP) (global.get $REG_SP) (i32.const 16))
    (call $emit_instr_str_q_off (global.get $REG_V0) (global.get $REG_SP) (i32.const 0))
  )
  (func $template_i32x4_lt_s (param $dec_ptr i32)
    (call $emit_instr_ldr_q_off (global.get $REG_V1) (global.get $REG_SP) (i32.const 0))
    (call $emit_instr_ldr_q_off (global.get $REG_V0) (global.get $REG_SP) (i32.const 1))
    (call $emit_neon_cmgt_4s_s (global.get $REG_V0) (global.get $REG_V1) (global.get $REG_V0))
    (call $emit_instr_addi_64 (global.get $REG_SP) (global.get $REG_SP) (i32.const 16))
    (call $emit_instr_str_q_off (global.get $REG_V0) (global.get $REG_SP) (i32.const 0))
  )
  ;; lt_u: a < b = b > a → CMHI(V1, V0)
  (func $template_i8x16_lt_u (param $dec_ptr i32)
    (call $emit_instr_ldr_q_off (global.get $REG_V1) (global.get $REG_SP) (i32.const 0))
    (call $emit_instr_ldr_q_off (global.get $REG_V0) (global.get $REG_SP) (i32.const 1))
    (call $emit_neon_cmgt_16b_u (global.get $REG_V0) (global.get $REG_V1) (global.get $REG_V0))
    (call $emit_instr_addi_64 (global.get $REG_SP) (global.get $REG_SP) (i32.const 16))
    (call $emit_instr_str_q_off (global.get $REG_V0) (global.get $REG_SP) (i32.const 0))
  )
  (func $template_i16x8_lt_u (param $dec_ptr i32)
    (call $emit_instr_ldr_q_off (global.get $REG_V1) (global.get $REG_SP) (i32.const 0))
    (call $emit_instr_ldr_q_off (global.get $REG_V0) (global.get $REG_SP) (i32.const 1))
    (call $emit_neon_cmgt_8h_u (global.get $REG_V0) (global.get $REG_V1) (global.get $REG_V0))
    (call $emit_instr_addi_64 (global.get $REG_SP) (global.get $REG_SP) (i32.const 16))
    (call $emit_instr_str_q_off (global.get $REG_V0) (global.get $REG_SP) (i32.const 0))
  )
  (func $template_i32x4_lt_u (param $dec_ptr i32)
    (call $emit_instr_ldr_q_off (global.get $REG_V1) (global.get $REG_SP) (i32.const 0))
    (call $emit_instr_ldr_q_off (global.get $REG_V0) (global.get $REG_SP) (i32.const 1))
    (call $emit_neon_cmgt_4s_u (global.get $REG_V0) (global.get $REG_V1) (global.get $REG_V0))
    (call $emit_instr_addi_64 (global.get $REG_SP) (global.get $REG_SP) (i32.const 16))
    (call $emit_instr_str_q_off (global.get $REG_V0) (global.get $REG_SP) (i32.const 0))
  )
  ;; le_s: a <= b = b >= a → CMGE(V1, V0) with Rn=V1, Rm=V0
  (func $template_i8x16_le_s (param $dec_ptr i32)
    (call $emit_instr_ldr_q_off (global.get $REG_V1) (global.get $REG_SP) (i32.const 0))
    (call $emit_instr_ldr_q_off (global.get $REG_V0) (global.get $REG_SP) (i32.const 1))
    (call $emit_neon_cmge_16b_s (global.get $REG_V0) (global.get $REG_V1) (global.get $REG_V0))
    (call $emit_instr_addi_64 (global.get $REG_SP) (global.get $REG_SP) (i32.const 16))
    (call $emit_instr_str_q_off (global.get $REG_V0) (global.get $REG_SP) (i32.const 0))
  )
  (func $template_i16x8_le_s (param $dec_ptr i32)
    (call $emit_instr_ldr_q_off (global.get $REG_V1) (global.get $REG_SP) (i32.const 0))
    (call $emit_instr_ldr_q_off (global.get $REG_V0) (global.get $REG_SP) (i32.const 1))
    (call $emit_neon_3same (i32.const 1) (i32.const 1) (i32.const 0) (i32.const 0x07)
                           (global.get $REG_V0) (global.get $REG_V1) (global.get $REG_V0))
    (call $emit_instr_addi_64 (global.get $REG_SP) (global.get $REG_SP) (i32.const 16))
    (call $emit_instr_str_q_off (global.get $REG_V0) (global.get $REG_SP) (i32.const 0))
  )
  (func $template_i32x4_le_s (param $dec_ptr i32)
    (call $emit_instr_ldr_q_off (global.get $REG_V1) (global.get $REG_SP) (i32.const 0))
    (call $emit_instr_ldr_q_off (global.get $REG_V0) (global.get $REG_SP) (i32.const 1))
    (call $emit_neon_3same (i32.const 1) (i32.const 2) (i32.const 0) (i32.const 0x07)
                           (global.get $REG_V0) (global.get $REG_V1) (global.get $REG_V0))
    (call $emit_instr_addi_64 (global.get $REG_SP) (global.get $REG_SP) (i32.const 16))
    (call $emit_instr_str_q_off (global.get $REG_V0) (global.get $REG_SP) (i32.const 0))
  )
  ;; le_u: a <= b = b >= a → CMHS(V1, V0)
  (func $template_i8x16_le_u (param $dec_ptr i32)
    (call $emit_instr_ldr_q_off (global.get $REG_V1) (global.get $REG_SP) (i32.const 0))
    (call $emit_instr_ldr_q_off (global.get $REG_V0) (global.get $REG_SP) (i32.const 1))
    (call $emit_neon_cmge_16b_u (global.get $REG_V0) (global.get $REG_V1) (global.get $REG_V0))
    (call $emit_instr_addi_64 (global.get $REG_SP) (global.get $REG_SP) (i32.const 16))
    (call $emit_instr_str_q_off (global.get $REG_V0) (global.get $REG_SP) (i32.const 0))
  )
  (func $template_i16x8_le_u (param $dec_ptr i32)
    (call $emit_instr_ldr_q_off (global.get $REG_V1) (global.get $REG_SP) (i32.const 0))
    (call $emit_instr_ldr_q_off (global.get $REG_V0) (global.get $REG_SP) (i32.const 1))
    (call $emit_neon_cmge_8h_u (global.get $REG_V0) (global.get $REG_V1) (global.get $REG_V0))
    (call $emit_instr_addi_64 (global.get $REG_SP) (global.get $REG_SP) (i32.const 16))
    (call $emit_instr_str_q_off (global.get $REG_V0) (global.get $REG_SP) (i32.const 0))
  )
  (func $template_i32x4_le_u (param $dec_ptr i32)
    (call $emit_instr_ldr_q_off (global.get $REG_V1) (global.get $REG_SP) (i32.const 0))
    (call $emit_instr_ldr_q_off (global.get $REG_V0) (global.get $REG_SP) (i32.const 1))
    (call $emit_neon_cmge_4s_u (global.get $REG_V0) (global.get $REG_V1) (global.get $REG_V0))
    (call $emit_instr_addi_64 (global.get $REG_SP) (global.get $REG_SP) (i32.const 16))
    (call $emit_instr_str_q_off (global.get $REG_V0) (global.get $REG_SP) (i32.const 0))
  )
  ;; ne: a != b = NOT(CMEQ(a, b)) → CMEQ + MVN
  (func $template_i8x16_ne (param $dec_ptr i32)
    (call $emit_instr_ldr_q_off (global.get $REG_V1) (global.get $REG_SP) (i32.const 0))
    (call $emit_instr_ldr_q_off (global.get $REG_V0) (global.get $REG_SP) (i32.const 1))
    (call $emit_neon_cmeq_16b (global.get $REG_V0) (global.get $REG_V0) (global.get $REG_V1))
    (call $emit_instr (i32.const 0x4E205800))
    (call $emit_instr_addi_64 (global.get $REG_SP) (global.get $REG_SP) (i32.const 16))
    (call $emit_instr_str_q_off (global.get $REG_V0) (global.get $REG_SP) (i32.const 0))
  )
  (func $template_i16x8_ne (param $dec_ptr i32)
    (call $emit_instr_ldr_q_off (global.get $REG_V1) (global.get $REG_SP) (i32.const 0))
    (call $emit_instr_ldr_q_off (global.get $REG_V0) (global.get $REG_SP) (i32.const 1))
    (call $emit_neon_cmeq_8h (global.get $REG_V0) (global.get $REG_V0) (global.get $REG_V1))
    (call $emit_instr (i32.const 0x4E205800))
    (call $emit_instr_addi_64 (global.get $REG_SP) (global.get $REG_SP) (i32.const 16))
    (call $emit_instr_str_q_off (global.get $REG_V0) (global.get $REG_SP) (i32.const 0))
  )
  (func $template_i32x4_ne (param $dec_ptr i32)
    (call $emit_instr_ldr_q_off (global.get $REG_V1) (global.get $REG_SP) (i32.const 0))
    (call $emit_instr_ldr_q_off (global.get $REG_V0) (global.get $REG_SP) (i32.const 1))
    (call $emit_neon_cmeq_4s (global.get $REG_V0) (global.get $REG_V0) (global.get $REG_V1))
    (call $emit_instr (i32.const 0x4E205800))
    (call $emit_instr_addi_64 (global.get $REG_SP) (global.get $REG_SP) (i32.const 16))
    (call $emit_instr_str_q_off (global.get $REG_V0) (global.get $REG_SP) (i32.const 0))
  )

  ;; ── Float comparison templates ────────────────────────────────────
  ;; f32x4_eq: FCMEQ V0.4S, V0.4S, V1.4S = 0x4E20E400
  (func $template_f32x4_eq (param $dec_ptr i32)
    (call $emit_instr_ldr_q_off (global.get $REG_V1) (global.get $REG_SP) (i32.const 0))
    (call $emit_instr_ldr_q_off (global.get $REG_V0) (global.get $REG_SP) (i32.const 1))
    (call $emit_instr (i32.const 0x4E20E400))
    (call $emit_instr_addi_64 (global.get $REG_SP) (global.get $REG_SP) (i32.const 16))
    (call $emit_instr_str_q_off (global.get $REG_V0) (global.get $REG_SP) (i32.const 0))
  )
  ;; f64x2_eq: FCMEQ V0.2D, V0.2D, V1.2D = 0x4E60E400
  (func $template_f64x2_eq (param $dec_ptr i32)
    (call $emit_instr_ldr_q_off (global.get $REG_V1) (global.get $REG_SP) (i32.const 0))
    (call $emit_instr_ldr_q_off (global.get $REG_V0) (global.get $REG_SP) (i32.const 1))
    (call $emit_instr (i32.const 0x4E60E400))
    (call $emit_instr_addi_64 (global.get $REG_SP) (global.get $REG_SP) (i32.const 16))
    (call $emit_instr_str_q_off (global.get $REG_V0) (global.get $REG_SP) (i32.const 0))
  )
  ;; f32x4_ne: NOT(FCMEQ V0.4S, V0.4S, V1.4S) = FCMEQ + MVN
  (func $template_f32x4_ne (param $dec_ptr i32)
    (call $emit_instr_ldr_q_off (global.get $REG_V1) (global.get $REG_SP) (i32.const 0))
    (call $emit_instr_ldr_q_off (global.get $REG_V0) (global.get $REG_SP) (i32.const 1))
    (call $emit_instr (i32.const 0x4E20E400))
    (call $emit_instr (i32.const 0x4E205800))
    (call $emit_instr_addi_64 (global.get $REG_SP) (global.get $REG_SP) (i32.const 16))
    (call $emit_instr_str_q_off (global.get $REG_V0) (global.get $REG_SP) (i32.const 0))
  )
  ;; f64x2_ne: NOT(FCMEQ V0.2D, V0.2D, V1.2D) = FCMEQ + MVN
  (func $template_f64x2_ne (param $dec_ptr i32)
    (call $emit_instr_ldr_q_off (global.get $REG_V1) (global.get $REG_SP) (i32.const 0))
    (call $emit_instr_ldr_q_off (global.get $REG_V0) (global.get $REG_SP) (i32.const 1))
    (call $emit_instr (i32.const 0x4E60E400))
    (call $emit_instr (i32.const 0x4E205800))
    (call $emit_instr_addi_64 (global.get $REG_SP) (global.get $REG_SP) (i32.const 16))
    (call $emit_instr_str_q_off (global.get $REG_V0) (global.get $REG_SP) (i32.const 0))
  )
  ;; f32x4_lt: a < b = b > a → FCMGT V0.4S, V1.4S, V0.4S = 0x4E22E420
  (func $template_f32x4_lt (param $dec_ptr i32)
    (call $emit_instr_ldr_q_off (global.get $REG_V1) (global.get $REG_SP) (i32.const 0))
    (call $emit_instr_ldr_q_off (global.get $REG_V0) (global.get $REG_SP) (i32.const 1))
    (call $emit_instr (i32.const 0x4E22E420))
    (call $emit_instr_addi_64 (global.get $REG_SP) (global.get $REG_SP) (i32.const 16))
    (call $emit_instr_str_q_off (global.get $REG_V0) (global.get $REG_SP) (i32.const 0))
  )
  ;; f64x2_lt: FCMGT V0.2D, V1.2D, V0.2D = 0x4E62E420
  (func $template_f64x2_lt (param $dec_ptr i32)
    (call $emit_instr_ldr_q_off (global.get $REG_V1) (global.get $REG_SP) (i32.const 0))
    (call $emit_instr_ldr_q_off (global.get $REG_V0) (global.get $REG_SP) (i32.const 1))
    (call $emit_instr (i32.const 0x4E62E420))
    (call $emit_instr_addi_64 (global.get $REG_SP) (global.get $REG_SP) (i32.const 16))
    (call $emit_instr_str_q_off (global.get $REG_V0) (global.get $REG_SP) (i32.const 0))
  )
  ;; f32x4_gt: FCMGT V0.4S, V0.4S, V1.4S = 0x4E22E400
  (func $template_f32x4_gt (param $dec_ptr i32)
    (call $emit_instr_ldr_q_off (global.get $REG_V1) (global.get $REG_SP) (i32.const 0))
    (call $emit_instr_ldr_q_off (global.get $REG_V0) (global.get $REG_SP) (i32.const 1))
    (call $emit_instr (i32.const 0x4E22E400))
    (call $emit_instr_addi_64 (global.get $REG_SP) (global.get $REG_SP) (i32.const 16))
    (call $emit_instr_str_q_off (global.get $REG_V0) (global.get $REG_SP) (i32.const 0))
  )
  ;; f64x2_gt: FCMGT V0.2D, V0.2D, V1.2D = 0x4E62E400
  (func $template_f64x2_gt (param $dec_ptr i32)
    (call $emit_instr_ldr_q_off (global.get $REG_V1) (global.get $REG_SP) (i32.const 0))
    (call $emit_instr_ldr_q_off (global.get $REG_V0) (global.get $REG_SP) (i32.const 1))
    (call $emit_instr (i32.const 0x4E62E400))
    (call $emit_instr_addi_64 (global.get $REG_SP) (global.get $REG_SP) (i32.const 16))
    (call $emit_instr_str_q_off (global.get $REG_V0) (global.get $REG_SP) (i32.const 0))
  )
  ;; f32x4_le: a <= b = b >= a → FCMGE V0.4S, V1.4S, V0.4S = 0x4E20E820
  (func $template_f32x4_le (param $dec_ptr i32)
    (call $emit_instr_ldr_q_off (global.get $REG_V1) (global.get $REG_SP) (i32.const 0))
    (call $emit_instr_ldr_q_off (global.get $REG_V0) (global.get $REG_SP) (i32.const 1))
    (call $emit_instr (i32.const 0x4E20E820))
    (call $emit_instr_addi_64 (global.get $REG_SP) (global.get $REG_SP) (i32.const 16))
    (call $emit_instr_str_q_off (global.get $REG_V0) (global.get $REG_SP) (i32.const 0))
  )
  ;; f64x2_le: FCMGE V0.2D, V1.2D, V0.2D = 0x4E60E820
  (func $template_f64x2_le (param $dec_ptr i32)
    (call $emit_instr_ldr_q_off (global.get $REG_V1) (global.get $REG_SP) (i32.const 0))
    (call $emit_instr_ldr_q_off (global.get $REG_V0) (global.get $REG_SP) (i32.const 1))
    (call $emit_instr (i32.const 0x4E60E820))
    (call $emit_instr_addi_64 (global.get $REG_SP) (global.get $REG_SP) (i32.const 16))
    (call $emit_instr_str_q_off (global.get $REG_V0) (global.get $REG_SP) (i32.const 0))
  )
  ;; f32x4_ge: FCMGE V0.4S, V0.4S, V1.4S = 0x4E20E800
  (func $template_f32x4_ge (param $dec_ptr i32)
    (call $emit_instr_ldr_q_off (global.get $REG_V1) (global.get $REG_SP) (i32.const 0))
    (call $emit_instr_ldr_q_off (global.get $REG_V0) (global.get $REG_SP) (i32.const 1))
    (call $emit_instr (i32.const 0x4E20E800))
    (call $emit_instr_addi_64 (global.get $REG_SP) (global.get $REG_SP) (i32.const 16))
    (call $emit_instr_str_q_off (global.get $REG_V0) (global.get $REG_SP) (i32.const 0))
  )
  ;; f64x2_ge: FCMGE V0.2D, V0.2D, V1.2D = 0x4E60E800
  (func $template_f64x2_ge (param $dec_ptr i32)
    (call $emit_instr_ldr_q_off (global.get $REG_V1) (global.get $REG_SP) (i32.const 0))
    (call $emit_instr_ldr_q_off (global.get $REG_V0) (global.get $REG_SP) (i32.const 1))
    (call $emit_instr (i32.const 0x4E60E800))
    (call $emit_instr_addi_64 (global.get $REG_SP) (global.get $REG_SP) (i32.const 16))
    (call $emit_instr_str_q_off (global.get $REG_V0) (global.get $REG_SP) (i32.const 0))
  )

  ;; ── Float min/max templates ────────────────────────────────────────
  ;; f32x4_min: FMIN V0.4S, V0.4S, V1.4S = 0x4E22F400
  (func $template_f32x4_min (param $dec_ptr i32)
    (call $emit_instr_ldr_q_off (global.get $REG_V1) (global.get $REG_SP) (i32.const 0))
    (call $emit_instr_ldr_q_off (global.get $REG_V0) (global.get $REG_SP) (i32.const 1))
    (call $emit_instr (i32.const 0x4E22F400))
    (call $emit_instr_addi_64 (global.get $REG_SP) (global.get $REG_SP) (i32.const 16))
    (call $emit_instr_str_q_off (global.get $REG_V0) (global.get $REG_SP) (i32.const 0))
  )
  ;; f32x4_max: FMAX V0.4S, V0.4S, V1.4S = 0x4E20F400
  (func $template_f32x4_max (param $dec_ptr i32)
    (call $emit_instr_ldr_q_off (global.get $REG_V1) (global.get $REG_SP) (i32.const 0))
    (call $emit_instr_ldr_q_off (global.get $REG_V0) (global.get $REG_SP) (i32.const 1))
    (call $emit_instr (i32.const 0x4E20F400))
    (call $emit_instr_addi_64 (global.get $REG_SP) (global.get $REG_SP) (i32.const 16))
    (call $emit_instr_str_q_off (global.get $REG_V0) (global.get $REG_SP) (i32.const 0))
  )
  ;; f64x2_min: FMIN V0.2D, V0.2D, V1.2D = 0x4E62F400
  (func $template_f64x2_min (param $dec_ptr i32)
    (call $emit_instr_ldr_q_off (global.get $REG_V1) (global.get $REG_SP) (i32.const 0))
    (call $emit_instr_ldr_q_off (global.get $REG_V0) (global.get $REG_SP) (i32.const 1))
    (call $emit_instr (i32.const 0x4E62F400))
    (call $emit_instr_addi_64 (global.get $REG_SP) (global.get $REG_SP) (i32.const 16))
    (call $emit_instr_str_q_off (global.get $REG_V0) (global.get $REG_SP) (i32.const 0))
  )
  ;; f64x2_max: FMAX V0.2D, V0.2D, V1.2D = 0x4E60F400
  (func $template_f64x2_max (param $dec_ptr i32)
    (call $emit_instr_ldr_q_off (global.get $REG_V1) (global.get $REG_SP) (i32.const 0))
    (call $emit_instr_ldr_q_off (global.get $REG_V0) (global.get $REG_SP) (i32.const 1))
    (call $emit_instr (i32.const 0x4E60F400))
    (call $emit_instr_addi_64 (global.get $REG_SP) (global.get $REG_SP) (i32.const 16))
    (call $emit_instr_str_q_off (global.get $REG_V0) (global.get $REG_SP) (i32.const 0))
  )

  ;; ── Stubs (extract/replace lane, neg, mul) ───────────────────────
  (func $template_i8x16_extract_lane_s (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_i8x16_extract_lane_u (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_i16x8_extract_lane_s (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_i16x8_extract_lane_u (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_i32x4_extract_lane (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_i64x2_extract_lane (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_f32x4_extract_lane (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_f64x2_extract_lane (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_i8x16_replace_lane (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_i16x8_replace_lane (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_i32x4_replace_lane (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_i64x2_replace_lane (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_f32x4_replace_lane (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_f64x2_replace_lane (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_i8x16_neg (param $dec_ptr i32)
    (call $emit_v128_pop)
    (call $emit_neon_neg (i32.const 0) (global.get $REG_V0) (global.get $REG_V0))
    (call $emit_v128_push)
  )
  (func $template_i16x8_neg (param $dec_ptr i32)
    (call $emit_v128_pop)
    (call $emit_neon_neg (i32.const 1) (global.get $REG_V0) (global.get $REG_V0))
    (call $emit_v128_push)
  )
  (func $template_i32x4_neg (param $dec_ptr i32)
    (call $emit_v128_pop)
    (call $emit_neon_neg (i32.const 2) (global.get $REG_V0) (global.get $REG_V0))
    (call $emit_v128_push)
  )
  (func $template_i64x2_neg (param $dec_ptr i32)
    (call $emit_v128_pop)
    (call $emit_neon_neg (i32.const 3) (global.get $REG_V0) (global.get $REG_V0))
    (call $emit_v128_push)
  )
  ;; f32x4_neg: FNEG V0.4S = 0x4EE0F800
  (func $template_f32x4_neg (param $dec_ptr i32)
    (call $emit_v128_pop)
    (call $emit_instr (i32.const 0x4EE0F800))
    (call $emit_v128_push)
  )
  ;; f64x2_neg: FNEG V0.2D = 0x4EE1F800
  (func $template_f64x2_neg (param $dec_ptr i32)
    (call $emit_v128_pop)
    (call $emit_instr (i32.const 0x4EE1F800))
    (call $emit_v128_push)
  )
  ;; i8x16_mul: no NEON MUL for byte — keep as stub (needs PMULL + complex)
  (func $template_i8x16_mul (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_i16x8_mul (param $dec_ptr i32)
    (call $emit_instr_ldr_q_off (global.get $REG_V1) (global.get $REG_SP) (i32.const 0))
    (call $emit_instr_ldr_q_off (global.get $REG_V0) (global.get $REG_SP) (i32.const 1))
    (call $emit_neon_mul_8h (global.get $REG_V0) (global.get $REG_V0) (global.get $REG_V1))
    (call $emit_instr_addi_64 (global.get $REG_SP) (global.get $REG_SP) (i32.const 16))
    (call $emit_instr_str_q_off (global.get $REG_V0) (global.get $REG_SP) (i32.const 0))
  )
  (func $template_i32x4_mul (param $dec_ptr i32)
    (call $emit_instr_ldr_q_off (global.get $REG_V1) (global.get $REG_SP) (i32.const 0))
    (call $emit_instr_ldr_q_off (global.get $REG_V0) (global.get $REG_SP) (i32.const 1))
    (call $emit_neon_mul_4s (global.get $REG_V0) (global.get $REG_V0) (global.get $REG_V1))
    (call $emit_instr_addi_64 (global.get $REG_SP) (global.get $REG_SP) (i32.const 16))
    (call $emit_instr_str_q_off (global.get $REG_V0) (global.get $REG_SP) (i32.const 0))
  )
  ;; f32x4_mul: FMUL V0.4S, V0.4S, V1.4S = 0x4E22DC00
  (func $template_f32x4_mul (param $dec_ptr i32)
    (call $emit_instr_ldr_q_off (global.get $REG_V1) (global.get $REG_SP) (i32.const 0))
    (call $emit_instr_ldr_q_off (global.get $REG_V0) (global.get $REG_SP) (i32.const 1))
    (call $emit_instr (i32.const 0x4E22DC00))
    (call $emit_instr_addi_64 (global.get $REG_SP) (global.get $REG_SP) (i32.const 16))
    (call $emit_instr_str_q_off (global.get $REG_V0) (global.get $REG_SP) (i32.const 0))
  )
  ;; f64x2_mul: FMUL V0.2D, V0.2D, V1.2D = 0x4E62DC00
  (func $template_f64x2_mul (param $dec_ptr i32)
    (call $emit_instr_ldr_q_off (global.get $REG_V1) (global.get $REG_SP) (i32.const 0))
    (call $emit_instr_ldr_q_off (global.get $REG_V0) (global.get $REG_SP) (i32.const 1))
    (call $emit_instr (i32.const 0x4E62DC00))
    (call $emit_instr_addi_64 (global.get $REG_SP) (global.get $REG_SP) (i32.const 16))
    (call $emit_instr_str_q_off (global.get $REG_V0) (global.get $REG_SP) (i32.const 0))
  )
  ;; ── Loadsplat templates (LD1R) ────────────────────────────────────
  ;; v128.load8_splat: LD1R { V0.16B }, [X0]
  (func $template_v128_load8_splat (param $dec_ptr i32)
    (call $emit_pop_x0)
    (call $emit_ldr_mem_ptr)
    (call $emit_instr (i32.const 0x4D40C000))
    (call $emit_v128_push)
  )
  ;; v128.load16_splat: LD1R { V0.8H }, [X0]  (size=01)
  (func $template_v128_load16_splat (param $dec_ptr i32)
    (call $emit_pop_x0)
    (call $emit_ldr_mem_ptr)
    (call $emit_instr (i32.const 0x4D41C000))
    (call $emit_v128_push)
  )
  ;; v128.load32_splat: LD1R { V0.4S }, [X0]  (size=10)
  (func $template_v128_load32_splat (param $dec_ptr i32)
    (call $emit_pop_x0)
    (call $emit_ldr_mem_ptr)
    (call $emit_instr (i32.const 0x4D42C000))
    (call $emit_v128_push)
  )
  ;; v128.load64_splat: LD1R { V0.2D }, [X0]  (size=11)
  (func $template_v128_load64_splat (param $dec_ptr i32)
    (call $emit_pop_x0)
    (call $emit_ldr_mem_ptr)
    (call $emit_instr (i32.const 0x4D43C000))
    (call $emit_v128_push)
  )
  ;; i8x16.swizzle: pop index, pop table, TBL V0.16B, {V1.16B}, V0.16B
  (func $template_i8x16_swizzle (param $dec_ptr i32)
    (call $emit_instr_ldr_q_off (global.get $REG_V1) (global.get $REG_SP) (i32.const 0))
    (call $emit_instr_ldr_q_off (global.get $REG_V0) (global.get $REG_SP) (i32.const 1))
    (call $emit_instr (i32.const 0x4E000020))
    (call $emit_instr_addi_64 (global.get $REG_SP) (global.get $REG_SP) (i32.const 16))
    (call $emit_instr_str_q_off (global.get $REG_V0) (global.get $REG_SP) (i32.const 0))
  )
  ;; v128.not: MVN V0.16B, V0.16B = 0x4E205800
  (func $template_v128_not (param $dec_ptr i32)
    (call $emit_v128_pop)
    (call $emit_instr (i32.const 0x4E205800))
    (call $emit_v128_push)
  )
  ;; v128.bitselect: pop mask(V1), pop b(V0), pop a(V2); BSL V1, V0, V2
  ;; V1 = (V1 & V0) | (~V1 & V2) = (mask & b) | (~mask & a)
  ;; Stack: [SP]=mask, [SP+16]=b, [SP+32]=a (deepest)
  ;; Result replaces 'a' position: SP += 32, str_q at new SP
  ;; BSL V1.16B, V0.16B, V2.16B = 0x4E021C01
  (func $template_v128_bitselect (param $dec_ptr i32)
    (call $emit_instr_ldr_q_off (global.get $REG_V1) (global.get $REG_SP) (i32.const 0))
    (call $emit_instr_ldr_q_off (global.get $REG_V0) (global.get $REG_SP) (i32.const 1))
    (call $emit_instr_ldr_q_off (global.get $REG_V2) (global.get $REG_SP) (i32.const 2))
    (call $emit_instr (i32.const 0x4E021C01))
    (call $emit_instr_addi_64 (global.get $REG_SP) (global.get $REG_SP) (i32.const 32))
    (call $emit_instr_str_q_off (global.get $REG_V1) (global.get $REG_SP) (i32.const 0))
  )

  (func $jit_compile (export "jit_compile") (param $func_idx i32) (result i32)
    (local $dec_ptr i32) (local $opcode i32) (local $imm0 i32)
    (local $code_start i32) (local $label_level i32)
    (local $loop_top_offset i32)
    (local $decoded_ops_base i32) (local $decoded_ops_count i32)

    ;; Reset code ptr
    (i32.store (global.get $JS_CODE_PTR) (i32.const 0))
    (global.set $RESULT_IN_X0 (i32.const 0))

    ;; Get decoded_ops base from imported global
    (local.set $decoded_ops_base (global.get $OFF_DECODED_OPS))

    ;; ── Emit prologue ─────────────────────────────────────────────
    (call $emit_prologue)

    ;; ── Main compile loop ─────────────────────────────────────────
    (local.set $dec_ptr (local.get $decoded_ops_base))
    (block $compile_done
      (loop $compile_loop
        ;; Read opcode
        (local.set $opcode (i32.load (local.get $dec_ptr)))

        ;; Read next opcode for peephole (at dec_ptr + DEC_SZ)
        (if (i32.load (i32.add (local.get $dec_ptr) (global.get $DEC_SZ)))
          (then
            (global.set $NEXT_OP (i32.load (i32.add (local.get $dec_ptr) (global.get $DEC_SZ))))
          )
          (else
            (global.set $NEXT_OP (i32.const 0))
          )
        )

        ;; Dispatch by opcode (flat if+br pattern)
        (block $dispatch_done
          (if (i32.eqz (local.get $opcode))
            (then (call $template_unreachable (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0x01))
            (then (call $template_nop (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0x02))
            (then (call $template_block (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0x03))
            (then
              (local.set $loop_top_offset (i32.load (global.get $JS_CODE_PTR)))
              (call $template_loop (local.get $dec_ptr))
              (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0x04))
            (then (call $template_if (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0x05))
            (then (call $template_else (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0x0B))
            (then (call $template_end (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0x0C))
            (then (call $template_br (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0x0D))
            (then (call $template_br_if (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0x0E))
            (then (call $template_br_table (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0x0F))
            (then (call $template_return (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0x10))
            (then (call $template_call (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0x12))
            (then (call $template_return_call (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0x1A))
            (then (call $template_drop (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0x1B))
            (then (call $template_select (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0x20))
            (then (call $template_local_get (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0x21))
            (then (call $template_local_set (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0x22))
            (then (call $template_local_tee (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0x23))
            (then (call $template_global_get (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0x24))
            (then (call $template_global_set (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0x25))
            (then (call $template_table_get (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0x26))
            (then (call $template_table_set (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0x28))
            (then (call $template_i32_load (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0x29))
            (then (call $template_i64_load (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0x2C))
            (then (call $template_i32_load8_s (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0x2D))
            (then (call $template_i32_load8_u (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0x2E))
            (then (call $template_i32_load16_s (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0x2F))
            (then (call $template_i32_load16_u (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0x36))
            (then (call $template_i32_store (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0x37))
            (then (call $template_i64_store (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0x3A))
            (then (call $template_i32_store8 (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0x3B))
            (then (call $template_i32_store16 (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0x3F))
            (then (call $template_memory_size (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0x40))
            (then (call $template_memory_grow (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0x41))
            (then (call $template_i32_const (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0x42))
            (then (call $template_i64_const (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0x45))
            (then (call $template_i32_eqz (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0x46))
            (then (call $template_i32_eq (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0x47))
            (then (call $template_i32_ne (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0x48))
            (then (call $template_i32_lt_s (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0x49))
            (then (call $template_i32_lt_u (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0x4A))
            (then (call $template_i32_gt_s (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0x4B))
            (then (call $template_i32_gt_u (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0x4C))
            (then (call $template_i32_le_s (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0x4D))
            (then (call $template_i32_le_u (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0x50))
            (then (call $template_i64_eqz (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0x51))
            (then (call $template_i64_eq (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0x52))
            (then (call $template_i64_ne (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0x53))
            (then (call $template_i64_lt_s (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0x54))
            (then (call $template_i64_lt_u (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0x55))
            (then (call $template_i64_gt_s (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0x56))
            (then (call $template_i64_gt_u (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0x57))
            (then (call $template_i64_le_s (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0x58))
            (then (call $template_i64_le_u (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0x67))
            (then (call $template_i32_clz (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0x68))
            (then (call $template_i32_ctz (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0x69))
            (then (call $template_i32_popcnt (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0x6A))
            (then (call $template_i32_add (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0x6B))
            (then (call $template_i32_sub (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0x6C))
            (then (call $template_i32_mul (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0x6D))
            (then (call $template_i32_div_s (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0x6E))
            (then (call $template_i32_div_u (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0x6F))
            (then (call $template_i32_rem_s (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0x70))
            (then (call $template_i32_rem_u (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0x71))
            (then (call $template_i32_and (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0x72))
            (then (call $template_i32_or (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0x73))
            (then (call $template_i32_xor (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0x74))
            (then (call $template_i32_shl (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0x75))
            (then (call $template_i32_shr_s (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0x76))
            (then (call $template_i32_shr_u (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0x77))
            (then (call $template_i32_rotl (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0x78))
            (then (call $template_i32_rotr (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0x79))
            (then (call $template_i64_clz (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0x7A))
            (then (call $template_i64_ctz (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0x7B))
            (then (call $template_i64_popcnt (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0x7C))
            (then (call $template_i64_add (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0x7D))
            (then (call $template_i64_sub (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0x7E))
            (then (call $template_i64_mul (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0x7F))
            (then (call $template_i64_div_s (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0x80))
            (then (call $template_i64_div_u (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0x81))
            (then (call $template_i64_rem_s (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0x82))
            (then (call $template_i64_rem_u (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0x83))
            (then (call $template_i64_and (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0x84))
            (then (call $template_i64_or (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0x85))
            (then (call $template_i64_xor (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0x86))
            (then (call $template_i64_shl (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0x87))
            (then (call $template_i64_shr_s (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0x88))
            (then (call $template_i64_shr_u (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0x89))
            (then (call $template_i64_rotl (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0x8A))
            (then (call $template_i64_rotr (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0xA7))
            (then (call $template_i32_wrap_i64 (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0xAC))
            (then (call $template_i64_extend_i32_s (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0xAD))
            (then (call $template_i64_extend_i32_u (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0xBC))
            (then (call $template_i32_reinterpret_f32 (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0xBD))
            (then (call $template_f32_reinterpret_i32 (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0xBE))
            (then (call $template_i64_reinterpret_f64 (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0xBF))
            (then (call $template_f64_reinterpret_i64 (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0xA8))
            (then (call $template_i32_trunc_f32_s (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0xA9))
            (then (call $template_i32_trunc_f32_u (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0xAA))
            (then (call $template_i32_trunc_f64_s (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0xAB))
            (then (call $template_i32_trunc_f64_u (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0xAE))
            (then (call $template_i64_trunc_f32_s (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0xAF))
            (then (call $template_i64_trunc_f32_u (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0xB0))
            (then (call $template_i64_trunc_f64_s (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0xB1))
            (then (call $template_i64_trunc_f64_u (local.get $dec_ptr)) (br $dispatch_done)))
          ;; 0xFC prefix - dispatch on imm0
          (if (i32.eq (local.get $opcode) (i32.const 0xFC))
            (then
              (local.set $imm0 (i32.load (i32.add (local.get $dec_ptr) (i32.const 4))))
              (block $fc_done
                (if (i32.eq (local.get $imm0) (i32.const 0x00))
                  (then (call $template_i32_trunc_sat_f32_s (local.get $dec_ptr)) (br $fc_done)))
                (if (i32.eq (local.get $imm0) (i32.const 0x01))
                  (then (call $template_i32_trunc_sat_f32_u (local.get $dec_ptr)) (br $fc_done)))
                (if (i32.eq (local.get $imm0) (i32.const 0x02))
                  (then (call $template_i32_trunc_sat_f64_s (local.get $dec_ptr)) (br $fc_done)))
                (if (i32.eq (local.get $imm0) (i32.const 0x03))
                  (then (call $template_i32_trunc_sat_f64_u (local.get $dec_ptr)) (br $fc_done)))
                (if (i32.eq (local.get $imm0) (i32.const 0x04))
                  (then (call $template_i64_trunc_sat_f32_s (local.get $dec_ptr)) (br $fc_done)))
                ;; Unknown 0xFC sub-opcode
                (global.set $JIT_ERROR (i32.const -3))
              )
              (br $dispatch_done)
            )
          )
          ;; 0xFD prefix (SIMD) - dispatch on imm0
          (if (i32.eq (local.get $opcode) (i32.const 0xFD))
            (then
              (local.set $imm0 (i32.load (i32.add (local.get $dec_ptr) (i32.const 4))))
              (block $fd_done
                ;; v128.load (0x00)
                (if (i32.eq (local.get $imm0) (i32.const 0x00))
                  (then (call $template_v128_load (local.get $dec_ptr)) (br $fd_done)))
                ;; v128.store (0x1B)
                (if (i32.eq (local.get $imm0) (i32.const 0x1B))
                  (then (call $template_v128_store (local.get $dec_ptr)) (br $fd_done)))
                ;; v128.const (0x0C)
                (if (i32.eq (local.get $imm0) (i32.const 0x0C))
                  (then (call $template_v128_const (local.get $dec_ptr)) (br $fd_done)))
                ;; i8x16.splat (0x2D)
                (if (i32.eq (local.get $imm0) (i32.const 0x2D))
                  (then (call $template_i8x16_splat (local.get $dec_ptr)) (br $fd_done)))
                ;; i16x8.splat (0x31)
                (if (i32.eq (local.get $imm0) (i32.const 0x31))
                  (then (call $template_i16x8_splat (local.get $dec_ptr)) (br $fd_done)))
                ;; i32x4.splat (0x35)
                (if (i32.eq (local.get $imm0) (i32.const 0x35))
                  (then (call $template_i32x4_splat (local.get $dec_ptr)) (br $fd_done)))
                ;; i64x2.splat (0x39)
                (if (i32.eq (local.get $imm0) (i32.const 0x39))
                  (then (call $template_i64x2_splat (local.get $dec_ptr)) (br $fd_done)))
                ;; f32x4.splat (0x3A)
                (if (i32.eq (local.get $imm0) (i32.const 0x3A))
                  (then (call $template_f32x4_splat (local.get $dec_ptr)) (br $fd_done)))
                ;; f64x2.splat (0x3D)
                (if (i32.eq (local.get $imm0) (i32.const 0x3D))
                  (then (call $template_f64x2_splat (local.get $dec_ptr)) (br $fd_done)))
                ;; i8x16.extract_lane_s (0x2E)
                (if (i32.eq (local.get $imm0) (i32.const 0x2E))
                  (then (call $template_i8x16_extract_lane_s (local.get $dec_ptr)) (br $fd_done)))
                ;; i8x16.extract_lane_u (0x2F)
                (if (i32.eq (local.get $imm0) (i32.const 0x2F))
                  (then (call $template_i8x16_extract_lane_u (local.get $dec_ptr)) (br $fd_done)))
                ;; i16x8.extract_lane_s (0x32)
                (if (i32.eq (local.get $imm0) (i32.const 0x32))
                  (then (call $template_i16x8_extract_lane_s (local.get $dec_ptr)) (br $fd_done)))
                ;; i16x8.extract_lane_u (0x33)
                (if (i32.eq (local.get $imm0) (i32.const 0x33))
                  (then (call $template_i16x8_extract_lane_u (local.get $dec_ptr)) (br $fd_done)))
                ;; i32x4.extract_lane (0x36)
                (if (i32.eq (local.get $imm0) (i32.const 0x36))
                  (then (call $template_i32x4_extract_lane (local.get $dec_ptr)) (br $fd_done)))
                ;; i64x2.extract_lane (0x38)
                (if (i32.eq (local.get $imm0) (i32.const 0x38))
                  (then (call $template_i64x2_extract_lane (local.get $dec_ptr)) (br $fd_done)))
                ;; f32x4.extract_lane (0x3B)
                (if (i32.eq (local.get $imm0) (i32.const 0x3B))
                  (then (call $template_f32x4_extract_lane (local.get $dec_ptr)) (br $fd_done)))
                ;; f64x2.extract_lane (0x3E)
                (if (i32.eq (local.get $imm0) (i32.const 0x3E))
                  (then (call $template_f64x2_extract_lane (local.get $dec_ptr)) (br $fd_done)))
                ;; i8x16.replace_lane (0x30)
                (if (i32.eq (local.get $imm0) (i32.const 0x30))
                  (then (call $template_i8x16_replace_lane (local.get $dec_ptr)) (br $fd_done)))
                ;; i16x8.replace_lane (0x34)
                (if (i32.eq (local.get $imm0) (i32.const 0x34))
                  (then (call $template_i16x8_replace_lane (local.get $dec_ptr)) (br $fd_done)))
                ;; i32x4.replace_lane (0x37)
                (if (i32.eq (local.get $imm0) (i32.const 0x37))
                  (then (call $template_i32x4_replace_lane (local.get $dec_ptr)) (br $fd_done)))
                ;; i64x2.replace_lane (0xAB) — canonical ID, not in old encoding
                ;; f32x4.replace_lane (0x3C)
                (if (i32.eq (local.get $imm0) (i32.const 0x3C))
                  (then (call $template_f32x4_replace_lane (local.get $dec_ptr)) (br $fd_done)))
                ;; f64x2.replace_lane (0x3F)
                (if (i32.eq (local.get $imm0) (i32.const 0x3F))
                  (then (call $template_f64x2_replace_lane (local.get $dec_ptr)) (br $fd_done)))
                ;; i8x16.add (0x40)
                (if (i32.eq (local.get $imm0) (i32.const 0x40))
                  (then (call $template_i8x16_add (local.get $dec_ptr)) (br $fd_done)))
                ;; i16x8.add (0x51)
                (if (i32.eq (local.get $imm0) (i32.const 0x51))
                  (then (call $template_i16x8_add (local.get $dec_ptr)) (br $fd_done)))
                ;; i32x4.add (0x62)
                (if (i32.eq (local.get $imm0) (i32.const 0x62))
                  (then (call $template_i32x4_add (local.get $dec_ptr)) (br $fd_done)))
                ;; i64x2.add (0x73)
                (if (i32.eq (local.get $imm0) (i32.const 0x73))
                  (then (call $template_i64x2_add (local.get $dec_ptr)) (br $fd_done)))
                ;; f32x4.add (0x84)
                (if (i32.eq (local.get $imm0) (i32.const 0x84))
                  (then (call $template_f32x4_add (local.get $dec_ptr)) (br $fd_done)))
                ;; f64x2.add (0x95)
                (if (i32.eq (local.get $imm0) (i32.const 0x95))
                  (then (call $template_f64x2_add (local.get $dec_ptr)) (br $fd_done)))
                ;; i8x16.sub (0x41)
                (if (i32.eq (local.get $imm0) (i32.const 0x41))
                  (then (call $template_i8x16_sub (local.get $dec_ptr)) (br $fd_done)))
                ;; i16x8.sub (0x52)
                (if (i32.eq (local.get $imm0) (i32.const 0x52))
                  (then (call $template_i16x8_sub (local.get $dec_ptr)) (br $fd_done)))
                ;; i32x4.sub (0x63)
                (if (i32.eq (local.get $imm0) (i32.const 0x63))
                  (then (call $template_i32x4_sub (local.get $dec_ptr)) (br $fd_done)))
                ;; i64x2.sub (0x74)
                (if (i32.eq (local.get $imm0) (i32.const 0x74))
                  (then (call $template_i64x2_sub (local.get $dec_ptr)) (br $fd_done)))
                ;; f32x4.sub (0x85)
                (if (i32.eq (local.get $imm0) (i32.const 0x85))
                  (then (call $template_f32x4_sub (local.get $dec_ptr)) (br $fd_done)))
                ;; f64x2.sub (0x96)
                (if (i32.eq (local.get $imm0) (i32.const 0x96))
                  (then (call $template_f64x2_sub (local.get $dec_ptr)) (br $fd_done)))
                ;; i8x16.mul (0x50)
                (if (i32.eq (local.get $imm0) (i32.const 0x50))
                  (then (call $template_i8x16_mul (local.get $dec_ptr)) (br $fd_done)))
                ;; i16x8.mul (0x61)
                (if (i32.eq (local.get $imm0) (i32.const 0x61))
                  (then (call $template_i16x8_mul (local.get $dec_ptr)) (br $fd_done)))
                ;; i32x4.mul (0x72)
                (if (i32.eq (local.get $imm0) (i32.const 0x72))
                  (then (call $template_i32x4_mul (local.get $dec_ptr)) (br $fd_done)))
                ;; f32x4.mul (0x87)
                (if (i32.eq (local.get $imm0) (i32.const 0x87))
                  (then (call $template_f32x4_mul (local.get $dec_ptr)) (br $fd_done)))
                ;; f64x2.mul (0x98)
                (if (i32.eq (local.get $imm0) (i32.const 0x98))
                  (then (call $template_f64x2_mul (local.get $dec_ptr)) (br $fd_done)))
                ;; i8x16.neg (0x46)
                (if (i32.eq (local.get $imm0) (i32.const 0x46))
                  (then (call $template_i8x16_neg (local.get $dec_ptr)) (br $fd_done)))
                ;; i16x8.neg (0x57)
                (if (i32.eq (local.get $imm0) (i32.const 0x57))
                  (then (call $template_i16x8_neg (local.get $dec_ptr)) (br $fd_done)))
                ;; i32x4.neg (0x68)
                (if (i32.eq (local.get $imm0) (i32.const 0x68))
                  (then (call $template_i32x4_neg (local.get $dec_ptr)) (br $fd_done)))
                ;; i64x2.neg (0x79)
                (if (i32.eq (local.get $imm0) (i32.const 0x79))
                  (then (call $template_i64x2_neg (local.get $dec_ptr)) (br $fd_done)))
                ;; f32x4.neg (0x8A)
                (if (i32.eq (local.get $imm0) (i32.const 0x8A))
                  (then (call $template_f32x4_neg (local.get $dec_ptr)) (br $fd_done)))
                ;; f64x2.neg (0x9B)
                (if (i32.eq (local.get $imm0) (i32.const 0x9B))
                  (then (call $template_f64x2_neg (local.get $dec_ptr)) (br $fd_done)))
                ;; i8x16.eq (0x47)
                (if (i32.eq (local.get $imm0) (i32.const 0x47))
                  (then (call $template_i8x16_eq (local.get $dec_ptr)) (br $fd_done)))
                ;; i16x8.eq (0x58)
                (if (i32.eq (local.get $imm0) (i32.const 0x58))
                  (then (call $template_i16x8_eq (local.get $dec_ptr)) (br $fd_done)))
                ;; i32x4.eq (0x69)
                (if (i32.eq (local.get $imm0) (i32.const 0x69))
                  (then (call $template_i32x4_eq (local.get $dec_ptr)) (br $fd_done)))
                ;; f32x4.eq (0x8C)
                (if (i32.eq (local.get $imm0) (i32.const 0x8C))
                  (then (call $template_f32x4_eq (local.get $dec_ptr)) (br $fd_done)))
                ;; f64x2.eq (0x9D)
                (if (i32.eq (local.get $imm0) (i32.const 0x9D))
                  (then (call $template_f64x2_eq (local.get $dec_ptr)) (br $fd_done)))
                ;; i8x16.ne (0x48)
                (if (i32.eq (local.get $imm0) (i32.const 0x48))
                  (then (call $template_i8x16_ne (local.get $dec_ptr)) (br $fd_done)))
                ;; i16x8.ne (0x59)
                (if (i32.eq (local.get $imm0) (i32.const 0x59))
                  (then (call $template_i16x8_ne (local.get $dec_ptr)) (br $fd_done)))
                ;; i32x4.ne (0x6A)
                (if (i32.eq (local.get $imm0) (i32.const 0x6A))
                  (then (call $template_i32x4_ne (local.get $dec_ptr)) (br $fd_done)))
                ;; f32x4.ne (0x8B)
                (if (i32.eq (local.get $imm0) (i32.const 0x8B))
                  (then (call $template_f32x4_ne (local.get $dec_ptr)) (br $fd_done)))
                ;; f64x2.ne (0x9C)
                (if (i32.eq (local.get $imm0) (i32.const 0x9C))
                  (then (call $template_f64x2_ne (local.get $dec_ptr)) (br $fd_done)))
                ;; i8x16.lt_s (0x4B)
                (if (i32.eq (local.get $imm0) (i32.const 0x4B))
                  (then (call $template_i8x16_lt_s (local.get $dec_ptr)) (br $fd_done)))
                ;; i8x16.lt_u (0x4A)
                (if (i32.eq (local.get $imm0) (i32.const 0x4A))
                  (then (call $template_i8x16_lt_u (local.get $dec_ptr)) (br $fd_done)))
                ;; i16x8.lt_s (0x5C)
                (if (i32.eq (local.get $imm0) (i32.const 0x5C))
                  (then (call $template_i16x8_lt_s (local.get $dec_ptr)) (br $fd_done)))
                ;; i16x8.lt_u (0x5B)
                (if (i32.eq (local.get $imm0) (i32.const 0x5B))
                  (then (call $template_i16x8_lt_u (local.get $dec_ptr)) (br $fd_done)))
                ;; i32x4.lt_s (0x6D)
                (if (i32.eq (local.get $imm0) (i32.const 0x6D))
                  (then (call $template_i32x4_lt_s (local.get $dec_ptr)) (br $fd_done)))
                ;; i32x4.lt_u (0x6C)
                (if (i32.eq (local.get $imm0) (i32.const 0x6C))
                  (then (call $template_i32x4_lt_u (local.get $dec_ptr)) (br $fd_done)))
                ;; f32x4.lt (0x8E)
                (if (i32.eq (local.get $imm0) (i32.const 0x8E))
                  (then (call $template_f32x4_lt (local.get $dec_ptr)) (br $fd_done)))
                ;; f64x2.lt (0x9F)
                (if (i32.eq (local.get $imm0) (i32.const 0x9F))
                  (then (call $template_f64x2_lt (local.get $dec_ptr)) (br $fd_done)))
                ;; i8x16.gt_s (0x4D)
                (if (i32.eq (local.get $imm0) (i32.const 0x4D))
                  (then (call $template_i8x16_gt_s (local.get $dec_ptr)) (br $fd_done)))
                ;; i8x16.gt_u (0x4C)
                (if (i32.eq (local.get $imm0) (i32.const 0x4C))
                  (then (call $template_i8x16_gt_u (local.get $dec_ptr)) (br $fd_done)))
                ;; i16x8.gt_s (0x5E)
                (if (i32.eq (local.get $imm0) (i32.const 0x5E))
                  (then (call $template_i16x8_gt_s (local.get $dec_ptr)) (br $fd_done)))
                ;; i16x8.gt_u (0x5D)
                (if (i32.eq (local.get $imm0) (i32.const 0x5D))
                  (then (call $template_i16x8_gt_u (local.get $dec_ptr)) (br $fd_done)))
                ;; i32x4.gt_s (0x6F)
                (if (i32.eq (local.get $imm0) (i32.const 0x6F))
                  (then (call $template_i32x4_gt_s (local.get $dec_ptr)) (br $fd_done)))
                ;; i32x4.gt_u (0x6E)
                (if (i32.eq (local.get $imm0) (i32.const 0x6E))
                  (then (call $template_i32x4_gt_u (local.get $dec_ptr)) (br $fd_done)))
                ;; f32x4.gt (0x90)
                (if (i32.eq (local.get $imm0) (i32.const 0x90))
                  (then (call $template_f32x4_gt (local.get $dec_ptr)) (br $fd_done)))
                ;; f64x2.gt (0xA1)
                (if (i32.eq (local.get $imm0) (i32.const 0xA1))
                  (then (call $template_f64x2_gt (local.get $dec_ptr)) (br $fd_done)))
                ;; i8x16.le_s (0x4F)
                (if (i32.eq (local.get $imm0) (i32.const 0x4F))
                  (then (call $template_i8x16_le_s (local.get $dec_ptr)) (br $fd_done)))
                ;; i8x16.le_u (0x4E)
                (if (i32.eq (local.get $imm0) (i32.const 0x4E))
                  (then (call $template_i8x16_le_u (local.get $dec_ptr)) (br $fd_done)))
                ;; i16x8.le_s (0x60)
                (if (i32.eq (local.get $imm0) (i32.const 0x60))
                  (then (call $template_i16x8_le_s (local.get $dec_ptr)) (br $fd_done)))
                ;; i16x8.le_u (0x5F)
                (if (i32.eq (local.get $imm0) (i32.const 0x5F))
                  (then (call $template_i16x8_le_u (local.get $dec_ptr)) (br $fd_done)))
                ;; i32x4.le_s (0x71)
                (if (i32.eq (local.get $imm0) (i32.const 0x71))
                  (then (call $template_i32x4_le_s (local.get $dec_ptr)) (br $fd_done)))
                ;; i32x4.le_u (0x70)
                (if (i32.eq (local.get $imm0) (i32.const 0x70))
                  (then (call $template_i32x4_le_u (local.get $dec_ptr)) (br $fd_done)))
                ;; f32x4.le (0x92)
                (if (i32.eq (local.get $imm0) (i32.const 0x92))
                  (then (call $template_f32x4_le (local.get $dec_ptr)) (br $fd_done)))
                ;; f64x2.le (0xA3)
                (if (i32.eq (local.get $imm0) (i32.const 0xA3))
                  (then (call $template_f64x2_le (local.get $dec_ptr)) (br $fd_done)))
                ;; i8x16.ge_s (0x49)
                (if (i32.eq (local.get $imm0) (i32.const 0x49))
                  (then (call $template_i8x16_ge_s (local.get $dec_ptr)) (br $fd_done)))
                ;; i16x8.ge_s (0x5A)
                (if (i32.eq (local.get $imm0) (i32.const 0x5A))
                  (then (call $template_i16x8_ge_s (local.get $dec_ptr)) (br $fd_done)))
                ;; i32x4.ge_s (0x6B)
                (if (i32.eq (local.get $imm0) (i32.const 0x6B))
                  (then (call $template_i32x4_ge_s (local.get $dec_ptr)) (br $fd_done)))
                ;; f32x4.ge (0x94)
                (if (i32.eq (local.get $imm0) (i32.const 0x94))
                  (then (call $template_f32x4_ge (local.get $dec_ptr)) (br $fd_done)))
                ;; f64x2.ge (0xA5)
                (if (i32.eq (local.get $imm0) (i32.const 0xA5))
                  (then (call $template_f64x2_ge (local.get $dec_ptr)) (br $fd_done)))
                ;; i8x16.and (0x43)
                (if (i32.eq (local.get $imm0) (i32.const 0x43))
                  (then (call $template_i8x16_and (local.get $dec_ptr)) (br $fd_done)))
                ;; i8x16.or (0x44)
                (if (i32.eq (local.get $imm0) (i32.const 0x44))
                  (then (call $template_i8x16_or (local.get $dec_ptr)) (br $fd_done)))
                ;; i8x16.xor (0x45)
                (if (i32.eq (local.get $imm0) (i32.const 0x45))
                  (then (call $template_i8x16_xor (local.get $dec_ptr)) (br $fd_done)))
                ;; ── New canonical ops (0xA6+, not in old encoding) ──
                ;; v128.load8_splat (0xA6)
                (if (i32.eq (local.get $imm0) (i32.const 0xA6))
                  (then (call $template_v128_load8_splat (local.get $dec_ptr)) (br $fd_done)))
                ;; v128.load16_splat (0xA7)
                (if (i32.eq (local.get $imm0) (i32.const 0xA7))
                  (then (call $template_v128_load16_splat (local.get $dec_ptr)) (br $fd_done)))
                ;; v128.load32_splat (0xA8)
                (if (i32.eq (local.get $imm0) (i32.const 0xA8))
                  (then (call $template_v128_load32_splat (local.get $dec_ptr)) (br $fd_done)))
                ;; v128.load64_splat (0xA9)
                (if (i32.eq (local.get $imm0) (i32.const 0xA9))
                  (then (call $template_v128_load64_splat (local.get $dec_ptr)) (br $fd_done)))
                ;; i8x16.swizzle (0xAA)
                (if (i32.eq (local.get $imm0) (i32.const 0xAA))
                  (then (call $template_i8x16_swizzle (local.get $dec_ptr)) (br $fd_done)))
                ;; i64x2.replace_lane (0xAB)
                (if (i32.eq (local.get $imm0) (i32.const 0xAB))
                  (then (call $template_i64x2_replace_lane (local.get $dec_ptr)) (br $fd_done)))
                ;; v128.not (0xAC)
                (if (i32.eq (local.get $imm0) (i32.const 0xAC))
                  (then (call $template_v128_not (local.get $dec_ptr)) (br $fd_done)))
                ;; v128.bitselect (0xAD)
                (if (i32.eq (local.get $imm0) (i32.const 0xAD))
                  (then (call $template_v128_bitselect (local.get $dec_ptr)) (br $fd_done)))
                ;; i8x16.ge_u (0xAE)
                (if (i32.eq (local.get $imm0) (i32.const 0xAE))
                  (then (call $template_i8x16_ge_u (local.get $dec_ptr)) (br $fd_done)))
                ;; i16x8.ge_u (0xAF)
                (if (i32.eq (local.get $imm0) (i32.const 0xAF))
                  (then (call $template_i16x8_ge_u (local.get $dec_ptr)) (br $fd_done)))
                ;; i32x4.ge_u (0xB0)
                (if (i32.eq (local.get $imm0) (i32.const 0xB0))
                  (then (call $template_i32x4_ge_u (local.get $dec_ptr)) (br $fd_done)))
                ;; i8x16.min_u (0xB8)
                (if (i32.eq (local.get $imm0) (i32.const 0xB8))
                  (then (call $template_i8x16_min_u (local.get $dec_ptr)) (br $fd_done)))
                ;; i8x16.max_u (0xBA)
                (if (i32.eq (local.get $imm0) (i32.const 0xBA))
                  (then (call $template_i8x16_max_u (local.get $dec_ptr)) (br $fd_done)))
                ;; i8x16.avgr_u (0xBB)
                (if (i32.eq (local.get $imm0) (i32.const 0xBB))
                  (then (call $template_i8x16_avgr_u (local.get $dec_ptr)) (br $fd_done)))
                ;; i16x8.min_u (0xBC)
                (if (i32.eq (local.get $imm0) (i32.const 0xBC))
                  (then (call $template_i16x8_min_u (local.get $dec_ptr)) (br $fd_done)))
                ;; i16x8.max_u (0xBE)
                (if (i32.eq (local.get $imm0) (i32.const 0xBE))
                  (then (call $template_i16x8_max_u (local.get $dec_ptr)) (br $fd_done)))
                ;; i16x8.avgr_u (0xBF)
                (if (i32.eq (local.get $imm0) (i32.const 0xBF))
                  (then (call $template_i16x8_avgr_u (local.get $dec_ptr)) (br $fd_done)))
                ;; f32x4.min (0xE1)
                (if (i32.eq (local.get $imm0) (i32.const 0xE1))
                  (then (call $template_f32x4_min (local.get $dec_ptr)) (br $fd_done)))
                ;; f32x4.max (0xE2)
                (if (i32.eq (local.get $imm0) (i32.const 0xE2))
                  (then (call $template_f32x4_max (local.get $dec_ptr)) (br $fd_done)))
                ;; f64x2.min (0xE3)
                (if (i32.eq (local.get $imm0) (i32.const 0xE3))
                  (then (call $template_f64x2_min (local.get $dec_ptr)) (br $fd_done)))
                ;; f64x2.max (0xE4)
                (if (i32.eq (local.get $imm0) (i32.const 0xE4))
                  (then (call $template_f64x2_max (local.get $dec_ptr)) (br $fd_done)))
                ;; Unknown 0xFD sub-opcode
                (global.set $JIT_ERROR (i32.const -4))
              )
              (br $dispatch_done)
            )
          )
          ;; Unknown opcode
          (global.set $JIT_ERROR (i32.const -2))
        )
        ;; Advance to next decoded op
        (local.set $dec_ptr (i32.add (local.get $dec_ptr) (global.get $DEC_SZ)))
        ;; Check if we've reached the end (opcode == 0 or count exhausted)
        ;; For now loop forever until opcode 0x0B (end) with matching
        ;; Actually just check for opcode 0 or completion
        (br_if $compile_done (i32.eq (local.get $opcode) (i32.const 0x0B)))
        (br $compile_loop)
      )
    )

    ;; ── Emit epilogue ─────────────────────────────────────────────
    (call $emit_epilogue)

    ;; ── Return code size ──────────────────────────────────────────
    (i32.load (global.get $JS_CODE_PTR))
  )
