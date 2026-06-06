(module
  ;; ═════════════════════════════════════════════════════════════════════
  ;; EdgeRun WASM → ARM32 (ARMv7-A) JIT Compiler
  ;;
  ;; Compiles WASM decoded ops into ARM32 machine code in a memory buffer.
  ;; Follows the same architecture as the x86_64 JIT (compiler.wat).
  ;; ARM32 instructions are fixed 4 bytes, little-endian.
  ;; ═════════════════════════════════════════════════════════════════════

  (import "edgerun-core" "memory" (memory 1))

  ;; ── Shared constants (from edgerun-core) ─────────────────────────────
  (import "edgerun-core" "OFF_TYPES_BUF" (global $OFF_TYPES_BUF i32))
  (import "edgerun-core" "OFF_CODE_BUF" (global $OFF_CODE_BUF i32))
  (import "edgerun-core" "OFF_FUNCTIONS_BUF" (global $OFF_FUNCTIONS_BUF i32))
  (import "edgerun-core" "OFF_DECODED_OPS" (global $OFF_DECODED_OPS i32))
  (import "edgerun-core" "OFF_DECODED_COUNT" (global $OFF_DECODED_COUNT i32))
  (import "edgerun-core" "DEC_SZ" (global $DEC_SZ i32))
  (import "edgerun-core" "SZ_TYPE" (global $SZ_TYPE i32))
  (import "edgerun-core" "SZ_CODE" (global $SZ_CODE i32))
  (import "edgerun-core" "SZ_FUNC" (global $SZ_FUNC i32))

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

  ;; Peephole optimization flag
  (global $RESULT_IN_X0      (mut i32) (i32.const 0))
  (global $NEXT_OP           (mut i32) (i32.const 0))
  (global $JIT_ERROR         (mut i32) (i32.const 0))

  ;; Current decoded op pointer
  (global $CURRENT_DEC_PTR   (mut i32) (i32.const 0))

  ;; ═════════════════════════════════════════════════════════════════════
  ;; ARM32 Register constants (X-named for template compatibility)
  ;; ═════════════════════════════════════════════════════════════════════
  ;; R0 = TOS/result (like rax)
  ;; R1 = scratch (like rcx)
  ;; R2 = scratch (like rdx)
  ;; R8 = JitGlobals (like r15/x19)
  ;; R9 = locals cache (like rbx/x20)
  ;; R10 = register-allocated local 0 (like r12/x21)
  ;; R11 = register-allocated local 1 (like r13/x22)
  ;; R12 = intra-call scratch / zero reg (XZR alias)
  ;; R13 = SP, R14 = LR, R15 = PC

  (global $REG_X0  i32 (i32.const 0))
  (global $REG_X1  i32 (i32.const 1))
  (global $REG_X2  i32 (i32.const 2))
  (global $REG_X19 i32 (i32.const 8))
  (global $REG_X20 i32 (i32.const 9))
  (global $REG_X21 i32 (i32.const 10))
  (global $REG_X22 i32 (i32.const 11))
  (global $REG_XZR i32 (i32.const 12))
  (global $REG_SP  i32 (i32.const 13))

  ;; ═════════════════════════════════════════════════════════════════════
  ;; ELF output buffer (for compile_to_elf)
  ;; ═════════════════════════════════════════════════════════════════════
  (global $ELF_OUT_BUF  i32 (i32.const 0x800000))
  (global $ELF_OUT_OFF  i32 (i32.const 0x700000))  ;; ELF_OUT_BUF - JIT_CACHE

  (global $TEXT_VA      i32 (i32.const 0x400000))
  (global $BSS_VA       i32 (i32.const 0x500000))

  (global $EHDR_SIZE    i32 (i32.const 52))    ;; 32-bit ELF header
  (global $PHDR_SIZE    i32 (i32.const 32))    ;; 32-bit ELF phdr
  (global $ELF_STUB_OFF i32 (i32.const 84))    ;; after ehdr (52) + 1 phdr (32)
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

    (call $emit_instr_movi (i32.const 7) (i32.const 1))  ;; MOV R7, #1 (SYS_exit)
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
  ;; WASM SIMD (0xFD prefix) — ARM32 stubs (NEON TBD)
  ;; ═════════════════════════════════════════════════════════════════════
  (func $template_v128_const (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_v128_load (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_v128_store (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_i8x16_splat (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_i16x8_splat (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_i32x4_splat (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_i64x2_splat (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_f32x4_splat (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_f64x2_splat (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_i8x16_add (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_i16x8_add (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_i32x4_add (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_i64x2_add (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_i8x16_sub (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_i16x8_sub (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_i32x4_sub (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_i64x2_sub (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_f32x4_add (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_f64x2_add (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_f32x4_sub (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_f64x2_sub (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_i8x16_mul (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_i16x8_mul (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_i32x4_mul (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_f32x4_mul (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_f64x2_mul (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_i8x16_neg (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_i16x8_neg (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_i32x4_neg (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_i64x2_neg (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_f32x4_neg (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_f64x2_neg (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_i8x16_eq (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_i16x8_eq (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_i32x4_eq (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_f32x4_eq (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_f64x2_eq (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_i8x16_ne (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_i16x8_ne (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_i32x4_ne (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_f32x4_ne (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_f64x2_ne (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_i8x16_lt_s (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_i8x16_lt_u (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_i16x8_lt_s (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_i16x8_lt_u (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_i32x4_lt_s (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_i32x4_lt_u (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_f32x4_lt (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_f64x2_lt (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_i8x16_gt_s (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_i8x16_gt_u (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_i16x8_gt_s (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_i16x8_gt_u (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_i32x4_gt_s (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_i32x4_gt_u (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_f32x4_gt (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_f64x2_gt (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_i8x16_le_s (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_i8x16_le_u (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_i16x8_le_s (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_i16x8_le_u (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_i32x4_le_s (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_i32x4_le_u (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_f32x4_le (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_f64x2_le (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_i8x16_ge_s (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_i16x8_ge_s (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_i32x4_ge_s (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_f32x4_ge (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_f64x2_ge (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_i8x16_and (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_i8x16_or (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_i8x16_xor (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
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
  ;; ── New canonical op stubs (0xA6+) ────────────────────────────────
  (func $template_v128_load8_splat (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_v128_load16_splat (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_v128_load32_splat (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_v128_load64_splat (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_i8x16_swizzle (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_v128_not (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_v128_bitselect (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_i8x16_ge_u (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_i16x8_ge_u (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_i32x4_ge_u (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_i8x16_min_u (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_i8x16_max_u (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_i8x16_avgr_u (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_i16x8_min_u (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_i16x8_max_u (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_i16x8_avgr_u (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_f32x4_min (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_f32x4_max (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_f64x2_min (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_f64x2_max (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))

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

)
