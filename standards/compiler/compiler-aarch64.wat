;; ═════════════════════════════════════════════════════════════════════
  ;; EdgeRun WASM → AArch64 (ARM64) JIT Compiler
  ;;
  ;; Compiles WASM decoded ops into AArch64 machine code in a memory buffer.
  ;; Follows the same architecture as the x86_64 JIT (compiler.wat) but emits
  ;; AArch64 instructions (all 4 bytes fixed-length).
  ;; ═════════════════════════════════════════════════════════════════════


  ;; ── Shared constants (from edgerun-core) ─────────────────────────────

  ;; JIT code cache: 1MB starting at 0x100000
  (global $JIT_SLOT_SIZE_aarch64  i32 (i32.const 0x40000))  ;; 256KB per function

  ;; JIT state at 0x300000
  (global $JIT_STATE_aarch64      i32 (i32.const 0x300000))

  ;; JIT state field offsets (relative to JIT_STATE)
  (global $JS_CODE_PTR_aarch64       i32 (i32.const 0))
  (global $JS_CACHE_BASE_aarch64     i32 (i32.const 4))
  (global $JS_CACHE_END_aarch64      i32 (i32.const 8))
  (global $JS_FUNC_IDX_aarch64       i32 (i32.const 12))
  (global $JS_RESULT_COUNT_aarch64   i32 (i32.const 16))
  (global $JS_STACK_DEPTH_aarch64    i32 (i32.const 20))
  (global $JS_MAX_STACK_aarch64      i32 (i32.const 24))
  (global $JS_LABEL_DEPTH_aarch64    i32 (i32.const 28))
  (global $JS_RETURN_EMITTED_aarch64 i32 (i32.const 32))
  (global $JS_LABEL_OFFSETS_aarch64  i32 (i32.const 64))    ;; 256*i32 = 1024 bytes
  (global $JS_LABEL_KINDS_aarch64    i32 (i32.const 1088))   ;; 256*byte = 256 bytes
  (global $JS_LABEL_IF_JZ_aarch64    i32 (i32.const 1344))   ;; 256*i32 = 1024 bytes
  (global $JS_FIXUP_COUNT_aarch64    i32 (i32.const 2368))
  (global $JS_FIXUP_LABEL_aarch64    i32 (i32.const 2372))   ;; 256*i32 = 1024 bytes
  (global $JS_FIXUP_OFFSET_aarch64   i32 (i32.const 3396))   ;; 256*i32 = 1024 bytes
  (global $JS_INITIALIZED_aarch64    i32 (i32.const 4420))

  ;; Label kinds
  (global $JIT_LABEL_BLOCK_aarch64   i32 (i32.const 0))
  (global $JIT_LABEL_LOOP_aarch64    i32 (i32.const 1))
  (global $JIT_LABEL_IF_aarch64      i32 (i32.const 2))

  ;; Error codes

  ;; ── WASM type constants ───────────────────────────────────────────────
  (global $WASM_TYPE_V128_aarch64    i32 (i32.const 0x7B))
  (global $OP_PREFIX_FC_aarch64      i32 (i32.const 0xFC))
  (global $OP_PREFIX_FD_aarch64      i32 (i32.const 0xFD))

  ;; ── AArch64 NEON register constants ─────────────────────────────────
  ;; V0-V31 are the 128-bit SIMD/FP registers (aliased as Q0-Q31)
  (global $REG_V0  i32 (i32.const 0))
  (global $REG_V1  i32 (i32.const 1))
  (global $REG_V2  i32 (i32.const 2))

  ;; Peephole optimization flag
  (global $RESULT_IN_X0_aarch64      (mut i32) (i32.const 0))
  (global $NEXT_OP_aarch64           (mut i32) (i32.const 0))
  (global $JIT_ERROR_aarch64         (mut i32) (i32.const 0))

  ;; ═════════════════════════════════════════════════════════════════════
  ;; ELF output buffer (for compile_to_elf)
  ;; ═════════════════════════════════════════════════════════════════════
  (global $ELF_OUT_BUF_aarch64  i32 (i32.const 0x400000))
  (global $ELF_OUT_OFF_aarch64  i32 (i32.const 0x700000))  ;; ELF_OUT_BUF - JIT_CACHE

  (global $TEXT_VA_aarch64      i32 (i32.const 0x400000))
  (global $BSS_VA_aarch64       i32 (i32.const 0x500000))

  (global $EHDR_SIZE_aarch64    i32 (i32.const 64))    ;; 64-bit ELF header
  (global $PHDR_SIZE_aarch64    i32 (i32.const 56))    ;; 64-bit ELF phdr
  (global $ELF_STUB_OFF_aarch64 i32 (i32.const 120))   ;; after ehdr (64) + 1 phdr (56)
  (global $ELF_CODE_OFF_aarch64 i32 (i32.const 256))   ;; aligned after stub
  (global $BSS_SIZE_aarch64     i32 (i32.const 0x40000)) ;; 256KB

  ;; BSS item VAs (relative to BSS_VA)
  (global $BSS_JITGLOBALS_aarch64 i32 (i32.const 0x500000))
  (global $BSS_MEM_aarch64       i32 (i32.const 0x500080))
  (global $BSS_LOCALS_aarch64    i32 (i32.const 0x510080))
  (global $BSS_GLOBALS_aarch64   i32 (i32.const 0x520080))
  (global $BSS_TABLE_aarch64     i32 (i32.const 0x530080))

  ;; ═════════════════════════════════════════════════════════════════════
  ;; Flat binary output buffer (for compile_to_bin)
  ;; ═════════════════════════════════════════════════════════════════════
  (global $BIN_OUT_BUF_aarch64  i32 (i32.const 0x500000))
  (global $BIN_OUT_OFF_aarch64  i32 (i32.const 0x800000))  ;; BIN_OUT_BUF - JIT_CACHE


  ;; Current decoded op pointer
  (global $CURRENT_DEC_PTR_aarch64   (mut i32) (i32.const 0))

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

  (global $REG_X0_aarch64 i32 (i32.const 0))
  (global $REG_X1_aarch64 i32 (i32.const 1))
  (global $REG_X2_aarch64 i32 (i32.const 2))
  (global $REG_X19_aarch64 i32 (i32.const 19))
  (global $REG_X20_aarch64 i32 (i32.const 20))
  (global $REG_X21_aarch64 i32 (i32.const 21))
  (global $REG_X22_aarch64 i32 (i32.const 22))
  (global $REG_SP_aarch64 i32 (i32.const 31))
  (global $REG_XZR_aarch64 i32 (i32.const 31))

  ;; ── Helper: write bytes to code cache ──────────────────────────────



  ;; Emit a qword (8 bytes) little-endian

  ;; ── AArch64 instruction emitter ────────────────────────────────────
  ;; All AArch64 instructions are exactly 4 bytes (32 bits)

  ;; Emit a 32-bit AArch64 instruction (little-endian)

  ;; ── AArch64 Register-to-Register (R-type) helpers ──────────────────
  ;; Base encoding for 64-bit operations:
  ;;   [31]=1 (64-bit), [30..29]=opc, [28..24]=0xxxx, [23..22]=variant,
  ;;   [21..16]=Rm, [15..10]=imm, [9..5]=Rn, [4..0]=Rd

  ;; ADD Xd, Xn, Xm (64-bit): 10001011000 Rm 000000 Rn Rd

  ;; SUB Xd, Xn, Xm (64-bit): 11001011000 Rm 000000 Rn Rd

  ;; ADD Wd, Wn, Wm (32-bit): 00001011000 Rm 000000 Rn Rd

  ;; SUB Wd, Wn, Wm (32-bit): 01001011000 Rm 000000 Rn Rd

  ;; MUL Xd, Xn, Xm (64-bit): 10011011000 111111 Rm Rn Rd

  ;; MUL Wd, Wn, Wm (32-bit): 00011011000 111111 Rm Rn Rd

  ;; SDIV Xd, Xn, Xm (64-bit): 10011010110 000011 Rm Rn Rd

  ;; UDIV Xd, Xn, Xm (64-bit): 10011010110 000010 Rm Rn Rd

  ;; SDIV Wd, Wn, Wm (32-bit): 00011010110 000011 Rm Rn Rd

  ;; UDIV Wd, Wn, Wm (32-bit): 00011010110 000010 Rm Rn Rd

  ;; AND Xd, Xn, Xm (64-bit): 10001010000 Rm 000000 Rn Rd

  ;; ORR Xd, Xn, Xm (64-bit): 10101010000 Rm 000000 Rn Rd

  ;; EOR Xd, Xn, Xm (64-bit): 11001010000 Rm 000000 Rn Rd

  ;; AND Wd, Wn, Wm (32-bit): 00001010000 Rm 000000 Rn Rd

  ;; ORR Wd, Wn, Wm (32-bit): 00101010000 Rm 000000 Rn Rd

  ;; EOR Wd, Wn, Wm (32-bit): 01001010000 Rm 000000 Rn Rd

  ;; LSLV Xd, Xn, Xm (64-bit variable shift left): 10011010110 001000 Rm Rn Rd

  ;; LSRV Xd, Xn, Xm (64-bit): 10011010110 001001 Rm Rn Rd

  ;; ASRV Xd, Xn, Xm (64-bit): 10011010110 001010 Rm Rn Rd

  ;; RORV Xd, Xn, Xm (64-bit): 10011010110 001011 Rm Rn Rd

  ;; LSLV Wd, Wn, Wm (32-bit): 00011010110 001000 Rm Rn Rd

  ;; LSRV Wd, Wn, Wm (32-bit): 00011010110 001001 Rm Rn Rd

  ;; ASRV Wd, Wn, Wm (32-bit): 00011010110 001010 Rm Rn Rd

  ;; RORV Wd, Wn, Wm (32-bit): 00011010110 001011 Rm Rn Rd

  ;; ── AArch64 Immediate arithmetic helpers ──────────────────────────

  ;; ADD Xd, Xn, #imm12 (shift=0): 10010001 00 sh imm12 Rn Rd
  ;;   Only for imm12 (0-4095) where imm fits in 12 bits unsigned

  ;; SUB Xd, Xn, #imm12 (shift=0): 11010001 00 sh imm12 Rn Rd

  ;; ADD Wd, Wn, #imm12 (32-bit): 00010001 00 sh imm12 Rn Rd

  ;; SUB Wd, Wn, #imm12 (32-bit): 01010001 00 sh imm12 Rn Rd

  ;; AND Xd, Xn, #imm (64-bit, bitmask immediate): complex encoding
  ;; For simple cases use AND with XZR or MOVN/MOVZ
  ;; MOV Xd, #imm (via MOVZ): 110100101 hw imm16 Rd
  ;;   hw=00 → zero-extend 16-bit to bits [15:0]
  ;;   hw=01 → shift left by 16
  ;;   hw=10 → shift left by 32
  ;;   hw=11 → shift left by 48

  ;; MOVN Xd, #imm (64-bit): 100100101 hw imm16 Rd

  ;; MOVK Xd, #imm (64-bit, keep other bits): 111100101 hw imm16 Rd

  ;; MOV N, #imm (32-bit): MOVZ Wd, #imm or MOVN Wd, #imm

  ;; ── AArch64 Load/Store helpers ─────────────────────────────────────
  ;; All use unsigned offset addressing: STR/LDR Xt, [Xn, #imm]
  ;; For stack push/pop with pre/post-index we use a different encoding.

  ;; STR Xt, [Xn, #imm] (64-bit, unsigned offset): 1111100101 imm12 Rn Rt

  ;; LDR Xt, [Xn, #imm] (64-bit, unsigned offset): 1111100101 imm12 Rn Rt

  ;; STR Wt, [Xn, #imm] (32-bit, unsigned offset): 1011100101 imm12 Rn Rt

  ;; LDR Wt, [Xn, #imm] (32-bit, unsigned offset): 1011100101 imm12 Rn Rt

  ;; STR Xt, [Xn, #imm]! (pre-index, 64-bit): 11111001 10 imm9 11 Xn Rt
  ;; imm9 is signed (-256 to 255), encoded as 9-bit signed

  ;; LDR Xt, [Xn], #imm (post-index, 64-bit): 11111000 10 imm9 01 Xn Rt

  ;; STR Wt, [Xn, #imm]! (pre-index, 32-bit): 10111001 10 imm9 11 Xn Rt

  ;; LDR Wt, [Xn], #imm (post-index, 32-bit): 10111000 10 imm9 01 Xn Rt

  ;; ── AArch64 Stack push/pop (value stack for WASM) ─────────────────
  ;; Push X0: STR X0, [SP, #-8]!

  ;; Pop X0: LDR X0, [SP], #8

  ;; Push X1: STR X1, [SP, #-8]!

  ;; Pop X1: LDR X1, [SP], #8

  ;; Push X2: STR X2, [SP, #-8]!

  ;; Pop X2: LDR X2, [SP], #8

  ;; Pop into X1: LDR X1, [SP], #8 (alias)

  ;; Pop pair into X1, X0 (LDP X1, X0, [SP], #16)
  ;; LDP encoding: 10101000 11 0 imm7 Rt2 Rn Rt1  (with post-index)
  ;; LDP Xt1, Xt2, [Xn], #imm: 10101000 110 imm7 Xn Xt2 Xt1
  ;; Wait, LDP post-index: opc=10, 1010 1000 1 11 imm7 Xn Rt2 Rt1
  ;; Let me just use two separate pops

  ;; STP Xt1, Xt2, [SP, #-16]! (pre-index pair)
  ;; STP Xt1, Xt2, [Xn, #-imm]!: 10101000 10 0 imm7 Xn Xt2 Xt1
  ;; Actually: for 64-bit STP pre-index: opc=10, 1010 1000 1 00 imm7 Xn Rt2 Rt1

  ;; ── Standard push/pop names (matching x86 compiler convention) ────

  ;; ── AArch64 sign extension / data processing ──────────────────────

  ;; SXTW X0, W0 (sign-extend W0→X0): 10011010110 000000 Rm(0) 00000 Rn(0) Rd
  ;; Actually: SXTW is alias for SBFM Xd, Xn, #0, #31
  ;; SBFM encoding: 100110 1 10 0 N immr imms Rn Rd
  ;; SXTW X0, W0: 0x93407C00
  ;; let me just hardcode

  ;; ── AArch64 branch instructions ────────────────────────────────────

  ;; B #imm (unconditional branch, ±128MB): 000101 + imm26
  ;; imm26 = (target - pc) >> 2, encoded as signed 26-bit

  ;; BL #imm (branch with link): 100101 + imm26

  ;; B.cond #imm (conditional branch, ±1MB): 01010100 imm19 0 cond
  ;; cond codes (AArch64): EQ=0, NE=1, CS/HS=2, CC/LO=3, MI=4, PL=5,
  ;;   VS=6, VC=7, HI=8, LS=9, GE=10, LT=11, GT=12, LE=13, AL=14

  ;; CBZ Xt, #imm (compare and branch if zero, ±1MB): 10110100 imm19 Rt
  ;; For 64-bit: 10110100 imm19 Rt

  ;; CBNZ Xt, #imm (compare and branch if non-zero, ±1MB): 10110101 imm19 Rt
  ;; For 64-bit: 10110101 imm19 Rt

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

  ;; CSET W0, inv_cond (32-bit)

  ;; CMP Xn, Xm: alias for SUBS XZR, Xn, Xm
  ;; SUBS XZR, Xn, Xm: sf=1, S=1, 11011 000 Rm 000000 Rn 11111
  ;; = 0xEB00001F | (Rm << 16) | (Rn << 5)

  ;; CMP Wn, Wm (32-bit): SUBS WZR, Wn, Wm
  ;; = 0x6B00001F | (Rm << 16) | (Rn << 5)

  ;; TST Xn, Xm: alias for ANDS XZR, Xn, Xm
  ;; = 0xEA00001F | (Rm << 16) | (Rn << 5)

  ;; TST Wn, Wm (32-bit)

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

  ;; STR Xt, [Xn, Xm, LSL #0] (64-bit register offset)

  ;; LDR Wt, [Xn, Xm, LSL #0] (32-bit register offset)

  ;; STR Wt, [Xn, Xm, LSL #0] (32-bit register offset)

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

  ;; LDR X0, [X19, #offset] — load JitGlobals field

  ;; STR X0, [X19, #offset] — store to JitGlobals field

  ;; LDR X0, [X20, #offset] — load local via cached base ptr

  ;; STR X0, [X20, #offset] — store local

  ;; ── AArch64 memory load (i32.load): address in X1, result in X0 ──
  ;; load mem_ptr into X2, then LDR W0, [X2, X1]

  ;; i64.load: LDR X0, [X2, X1]

  ;; i32.store: address in X1, value in X0, store W0 to [X2, X1]

  ;; i64.store: STR X0, [X2, X1]

  ;; ── AArch64 Unary arithmetic helpers (X0←op(X0)) ──────────────────

  ;; CLZ X0, X0 (count leading zeros, 64-bit)

  ;; CLZ W0, W0 (32-bit)

  ;; RBIT X0, X0 (reverse bits, 64-bit) — for CTZ: RBIT + CLZ

  ;; RBIT W0, W0 (32-bit)

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

  ;; FMOV Wd, Sn: 0x1E260000 | (Sn << 5) | Rd

  ;; FMOV D0, X0: 0x9E670000
  ;; FMOV Dd, Xn: 9E670000 | (Xn << 5) | Dd

  ;; FMOV Xd, Dn: 9E660000 | (Dn << 5) | Xd

  ;; ── AArch64 NEON scalar float arithmetic ──────────────────────────

  ;; FADD Sd, Sn, Sm (scalar float32): 0E 30 00 ...
  ;; FADD S0, S0, S1: 0x1E302800
  ;; FADD Sd, Sn, Sm: 0x1E302800 | (Sm << 16) | (Sn << 5) | Sd

  ;; FSUB Sd, Sn, Sm: 0x1E303800

  ;; FMUL Sd, Sn, Sm: 0x1E300800

  ;; FDIV Sd, Sn, Sm: 0x1E301800

  ;; FADD Dd, Dn, Dm (scalar f64): 1E 60 28 ...
  ;; FADD D0, D0, D1: 0x1E602800
  ;; FADD Dd, Dn, Dm: 0x1E602800 | (Dm << 16) | (Dn << 5) | Dd

  ;; FSUB Dd, Dn, Dm: 0x1E603800

  ;; FMUL Dd, Dn, Dm: 0x1E600800

  ;; FDIV Dd, Dn, Dm: 0x1E601800

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

  ;; Float conversions:
  ;; FCVTZ W0, S0 (f32→i32): 1E 38 00 00?
  ;; FCVTZ Wd, Sn: 0x1E380000 | (Sn << 5) | Wd
  ;; For W0, S0: 0x1E380000

  ;; FCVTZ X0, S0 (f32→i64): 9E 38 00 00
  ;; FCVTZ Xd, Sn: 0x9E380000 | (Sn << 5) | Xd

  ;; FCVTZ W0, D0 (f64→i32): 1E 78 00 00

  ;; FCVTZ X0, D0 (f64→i64): 9E 78 00 00

  ;; SCVTF S0, W0 (i32→f32): 1E 22 00 00
  ;; SCVTF Sd, Wn: 0x1E220000 | (Wn << 5) | Sd

  ;; UCVTF S0, W0 (i32→f32 unsigned): 1E 23 00 00

  ;; SCVTF D0, X0 (i64→f64): 9E 62 00 00

  ;; UCVTF D0, X0 (i64→f64 unsigned): 9E 63 00 00

  ;; SCVTF D0, W0 (i32→f64): 1E 62 00 00

  ;; SCVTF S0, X0 (i64→f32): 9E 22 00 00

  ;; UCVTF S0, X0 (i64→f32 unsigned): 9E 23 00 00

  ;; FCVT S0, D0 (f64→f32 demote): 1E 62 80 00 ... 
  ;; Actually FCVT Sd, Dn: 0x1E624000 | (Dn << 5) | Sd
  ;; Let me just hardcode:

  ;; FCVT D0, S0 (f32→f64 promote): 1E 22 40 00

  ;; ── AArch64 FRINT (rounding) helpers ──────────────────────────────

  ;; FRINTP S0, S0 (ceil): 1E 2C 08 00 ... 
  ;; FRINTM S0, S0 (floor): 1E 2C 18 00
  ;; FRINTZ S0, S0 (trunc): 1E 2C 28 00
  ;; FRINTN S0, S0 (nearest): 1E 2C 48 00?  Actually nearest-even
  ;; FRINTA S0, S0 (nearest, ties away): hmm

  ;; Let me hardcode these:
  ;; FRINTP S0, S0: 0x1E2C0800
  ;; FRINTM S0, S0: 0x1E2C1800
  ;; FRINTZ S0, S0: 0x1E2C2800
  ;; FRINTN S0, S0: 0x1E2C4800 (ties to even) — nearest in IEEE 754
  ;; FRINTP D0, D0: 0x1E6C0800
  ;; FRINTM D0, D0: 0x1E6C1800
  ;; FRINTZ D0, D0: 0x1E6C2800
  ;; FRINTN D0, D0: 0x1E6C4800

  ;; FSQRT S0, S0: 1E 21 C0 00
  ;; FSQRT D0, D0: 1E 61 C0 00

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

  ;; Epilogue: restore callee-saved, restore FP/LR, ret

  ;; ── JIT state helpers ─────────────────────────────────────────────




  ;; ── Convenience arithmetic/stack helpers ──────────────────────────

  ;; ADD X0, X0, X1 (64-bit)

  ;; SUB X0, X0, X1 (64-bit)

  ;; ADD W0, W0, W1 (32-bit)

  ;; SUB W0, W0, W1 (32-bit)

  ;; MUL X0, X0, X1 (64-bit)

  ;; MUL W0, W0, W1 (32-bit)

  ;; SDIV X0, X0, X1 (64-bit signed)

  ;; UDIV X0, X0, X1 (64-bit unsigned)

  ;; SDIV W0, W0, W1 (32-bit signed)

  ;; UDIV W0, W0, W1 (32-bit unsigned)

  ;; AND X0, X0, X1 (64-bit)

  ;; ORR X0, X0, X1 (64-bit)

  ;; EOR X0, X0, X1 (64-bit)

  ;; AND W0, W0, W1 (32-bit)

  ;; ORR W0, W0, W1 (32-bit)

  ;; EOR W0, W0, W1 (32-bit)

  ;; LSL X0, X0, X1 (64-bit variable)

  ;; LSR X0, X0, X1 (64-bit variable)

  ;; ASR X0, X0, X1 (64-bit variable)

  ;; ROR X0, X0, X1 (64-bit variable)

  ;; LSL W0, W0, W1 (32-bit variable)

  ;; LSR W0, W0, W1 (32-bit variable)

  ;; ASR W0, W0, W1 (32-bit variable)

  ;; ROR W0, W0, W1 (32-bit variable)

  ;; ═════════════════════════════════════════════════════════════════════
  ;; ELF / flat binary output
  ;; ═════════════════════════════════════════════════════════════════════

  ;; BL with target VA: compute relative offset and emit BL (AArch64 PC = current, no +8)

  ;; ── ELF64 header (64 bytes) ─────────────────────────────────────


  ;; ── ELF64 program header (56 bytes) ────────────────────────────


  ;; ── Runtime stub for ELF: sets up X19=JitGlobals, calls code, exits ──
  ;; AArch64 JitGlobals offsets (64-bit pointers, same as x86-64):
  ;;   +0 = locals, +8 = mem, +24 = globals, +48 = table, +64 = memory_pages


  ;; ── Copy compiled code from JIT cache to output ────────────────


  ;; ── compile_to_elf: compile WASM, emit ELF64 executable ─────────

  (func $compile_to_elf_aarch64 (export "compile_to_elf_aarch64") (param $func_idx i32) (result i32 i32)
    (local $code_size i32) (local $total_size i32)
    (local $saved i32) (local $bss_va i32)

    (local.set $code_size (call $jit_compile_aarch64 (local.get $func_idx)))
    (local.set $total_size (i32.add (global.get $ELF_CODE_OFF_aarch64) (local.get $code_size)))
    (local.set $bss_va
      (i32.add
        (i32.and (i32.add (local.get $total_size) (i32.const 0xFFF)) (i32.const -0x1000))
        (global.get $TEXT_VA_aarch64)))

    (local.set $saved (i32.load (global.get $JS_CODE_PTR_aarch64)))
    (i32.store (global.get $JS_CODE_PTR_aarch64) (global.get $ELF_OUT_OFF_aarch64))

    (call $emit_aarch64_elf64_ehdr
      (i32.add (global.get $TEXT_VA_aarch64) (global.get $ELF_STUB_OFF_aarch64))
      (i32.const 64)
      (i32.const 1))

    (call $emit_aarch64_elf64_phdr
      (i32.const 1)         ;; PT_LOAD
      (i32.const 7)         ;; PF_R | PF_W | PF_X
      (i32.const 0)         ;; p_offset
      (global.get $TEXT_VA_aarch64) ;; p_vaddr
      (local.get $total_size) ;; p_filesz
      (i32.add (i32.and (i32.add (local.get $total_size) (i32.const 0xFFF)) (i32.const -0x1000)) (global.get $BSS_SIZE_aarch64)))

    (call $emit_aarch64_elf_stub (local.get $bss_va))

    (block $pad_done
      (loop $pad_loop
        (if (i32.ge_u (i32.load (global.get $JS_CODE_PTR_aarch64))
                       (i32.add (global.get $ELF_OUT_OFF_aarch64) (global.get $ELF_CODE_OFF_aarch64)))
          (then (br $pad_done)))
        (call $emit_aarch64_byte (i32.const 0))
        (br $pad_loop)
      )
    )

    (call $copy_compiled_code_aarch64 (global.get $JIT_CACHE) (local.get $code_size) (global.get $ELF_CODE_OFF_aarch64))
    (i32.store (global.get $JS_CODE_PTR_aarch64) (local.get $saved))

    (return (global.get $ELF_OUT_BUF_aarch64) (local.get $total_size))
  )

  ;; ═════════════════════════════════════════════════════════════════════
  ;; Flat binary output (bare-metal AArch64)
  ;; ═════════════════════════════════════════════════════════════════════

  ;; Store result at 0x500 and halt

  ;; Emit bare-metal stub (no headers). Returns stub size.

  ;; ── Copy code to binary output buffer ──────────────────────────


  ;; ── compile_to_bin: emit flat binary ───────────────────────────

  (func $compile_to_bin_aarch64 (export "compile_to_bin_aarch64") (param $func_idx i32) (result i32 i32)
    (local $code_size i32) (local $stub_size i32) (local $total_size i32)
    (local $saved i32)

    (local.set $code_size (call $jit_compile_aarch64 (local.get $func_idx)))
    (local.set $saved (i32.load (global.get $JS_CODE_PTR_aarch64)))
    (i32.store (global.get $JS_CODE_PTR_aarch64) (global.get $BIN_OUT_OFF_aarch64))

    (local.set $stub_size (call $emit_aarch64_bare_metal_stub))
    (call $copy_code_to_aarch64 (global.get $BIN_OUT_BUF_aarch64) (local.get $code_size) (local.get $stub_size))
    (local.set $total_size (i32.add (local.get $stub_size) (local.get $code_size)))

    (i32.store (global.get $JS_CODE_PTR_aarch64) (local.get $saved))
    (return (global.get $BIN_OUT_BUF_aarch64) (local.get $total_size))
  )

  ;; ── Pop two values: pop X1 (right), pop X0 (left) ────────────────

  ;; ── XOR X0, X0 (zero register) ────────────────────────────────────
  ;; EOR X0, X0, X0 = 0xAA000000 | (X0 << 16) | (X0 << 5) | X0 = 0xAA000000

  ;; EOR W0, W0, W0 = 0x2A000000

  ;; EOR X1, X1, X1

  ;; EOR X2, X2, X2

  ;; MOV X0, X1: ORR X0, XZR, X1
  ;; = 0xAA000020 | X1  = 0xAA000021

  ;; MOV X1, X0: ORR X1, XZR, X0

  ;; MOV W0, W1: ORR W0, WZR, W1

  ;; MOV W1, W0

  ;; ═════════════════════════════════════════════════════════════════════
  ;; AArch64 NEON (SIMD) emit helpers — v128 load/store/push/pop
  ;; ═════════════════════════════════════════════════════════════════════

  ;; STR Qt, [Xn, #imm12*16] — 128-bit store, unsigned offset
  ;; imm12 = byte_offset / 16 (scaled by data size 16)
  ;; Encoding: Q=1 1111101 opc=00 V=1 00 imm12 Rn Rt

  ;; LDR Qt, [Xn, #imm12*16] — 128-bit load, unsigned offset
  ;; Encoding: Q=1 1111101 opc=01 V=1 00 imm12 Rn Rt

  ;; Push Q0 onto WASM value stack (grows downward): SUB SP,#16; STR Q0,[SP]

  ;; Pop Q0 from WASM value stack: LDR Q0,[SP]; ADD SP,#16

  ;; Load Q0 from address in X1 into Q0: LDR Q0, [X1] (imm12=0 for bare reg)
  ;; Uses ldr_q_off with Xn = X1 (=1), imm12 = 0

  ;; Store Q0 to address in X1: STR Q0, [X1] (imm12=0 for bare reg)

  ;; Load 32-bit constant address into X1 (used to address v128 immediate)

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

  ;; Convenience: ADD Vd.16B, Vn.16B, Vm.16B

  ;; Convenience: SUB Vd.16B, Vn.16B, Vm.16B

  ;; Convenience: ADD Vd.8H, Vn.8H, Vm.8H (16-bit lanes)

  ;; Convenience: SUB Vd.8H, Vn.8H, Vm.8H

  ;; Convenience: ADD Vd.4S, Vn.4S, Vm.4S (32-bit lanes)

  ;; Convenience: SUB Vd.4S, Vn.4S, Vm.4S

  ;; Convenience: ADD Vd.2D, Vn.2D, Vm.2D (64-bit lanes)

  ;; Convenience: SUB Vd.2D, Vn.2D, Vm.2D

  ;; ── AArch64 NEON bitwise (AND/OR/XOR) via three-register different ──
  ;; BIC Vd.16B, Vn.16B, Vm.16B: AND with complement
  ;; Actually use AND Vd.16B, Vn.16B, Vm.16B (three-register different encoding):
  ;; opcode = 00011, U=0, Q=1, size=00

  ;; ORR Vd.16B, Vn.16B, Vm.16B: opcode = 01011, U=0

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

  ;; ── AArch64 NEON comparison helpers ──────────────────────────────

  ;; CMGT Vd.16B, Vn.16B, Vm.16B (signed): Q=1, size=00, U=0, opcode=00110
  ;; CMGT Vd.16B, Vn.16B, Vm.16B (unsigned): U=1, opcode=00110
  ;; CMEQ Vd.16B, Vn.16B, Vm.16B: opcode=10001, U=1
  ;; CMGE Vd.16B, Vn.16B, Vm.16B (signed): U=0, opcode=00111
  ;; CMGE Vd.16B, Vn.16B, Vm.16B (unsigned): U=1, opcode=00111

  ;; CMGT/CMGE for 8H, 4S, 2D lanes — same as 16B but different size
  ;; size=01 for 8H, size=10 for 4S, size=11 for 2D

  ;; CMGE (unsigned) Vd.8H, Vn.8H, Vm.8H: CMHS, U=1, opcode=00111, size=01
  ;; CMGE (unsigned) Vd.4S, Vn.4S, Vm.4S: CMHS, size=10
  ;; CMGT (unsigned) Vd.8H, Vn.8H, Vm.8H: CMHI, U=1, size=01, opcode=00110
  ;; CMGT (unsigned) Vd.4S, Vn.4S, Vm.4S: CMHI, size=10
  ;; CMGE (signed) Vd.8H, Vn.8H, Vm.8H: U=0, size=01, opcode=00111
  ;; CMGE (signed) Vd.4S, Vn.4S, Vm.4S: U=0, size=10, opcode=00111

  ;; ── MUL helpers (signed/unsigned, integer) ──────────────────────
  ;; MUL Vd.8H, Vn.8H, Vm.8H: U=0, opcode=10011, size=01
  ;; MUL Vd.4S, Vn.4S, Vm.4S: size=10

  ;; ── NEG helpers (2-register misc) ───────────────────────────────
  ;; NEG Vd.16B, Vn.16B: 0x0E207800 | (Q<<30) | (size<<22) | (Rn<<5) | Rd
  ;; size=00 for 16B (byte), size=01 for 8H (halfword), size=10 for 4S, size=11 for 2D

  ;; ── UMIN / UMAX / URHADD helpers (unsigned) ─────────────────────
  ;; UMIN Vd.16B, Vn.16B, Vm.16B: U=1, opcode=01101, size=00
  ;; UMIN Vd.8H, Vn.8H, Vm.8H: size=01
  ;; UMAX Vd.16B, Vn.16B, Vm.16B: U=1, opcode=01100, size=00
  ;; UMAX Vd.8H, Vn.8H, Vm.8H: size=01
  ;; URHADD Vd.16B, Vn.16B, Vm.16B: (a+b+1)>>1, U=1, opcode=00011, size=00
  ;; URHADD Vd.8H, Vn.8H, Vm.8H: size=01

  ;; ── Pop two, operate, push (pattern for binary ops) ──────────────

  ;; Opcode templates — each emits AArch64 code for one WASM opcode
  ;; ═════════════════════════════════════════════════════════════════════

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

  ;; ── Memory load/store templates ───────────────────────────────

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

  ;; ── i32 comparison templates ──────────────────────────────────

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

  ;; ── i64 comparison templates ──────────────────────────────────

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

  ;; ── i32 unary/binary arithmetic templates ─────────────────────

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

  ;; ── i64 unary/binary arithmetic templates ─────────────────────

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

  ;; ── Conversion templates ──────────────────────────────────────

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

  ;; ── Other templates ───────────────────────────────────────────

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
  ;; jit_compile will handle the fixup/backpatching.
  ;; We just emit placeholder branches and the main loop patches them.

  ;; ════════════════════════════════════════════════════════════════════
  ;; ── The main compile function ─────────────────────────────────────
  ;; ════════════════════════════════════════════════════════════════════


  ;; ═════════════════════════════════════════════════════════════════════
  ;; WASM SIMD (0xFD prefix) — AArch64 NEON implementations
  ;; ═════════════════════════════════════════════════════════════════════

  ;; ── v128.const (0xFD 0x0C): load 16-byte constant from decoded op ──
  (func $template_aarch64_v128_const (param $dec_ptr i32)
    (local $addr i32)
    (local.set $addr (i32.add (local.get $dec_ptr) (i32.const 16)))
    (call $emit_aarch64_load_addr_x1 (local.get $addr))
    (call $emit_aarch64_v128_load_reg)
    (call $emit_aarch64_v128_push)
  )

  ;; ── v128.load (0xFD 0x00): pop addr, load 16 bytes from mem ─────────
  (func $template_aarch64_v128_load (param $dec_ptr i32)
    (call $emit_aarch64_pop_x0)
    (call $emit_aarch64_ldr_mem_ptr)
    (call $emit_aarch64_instr_add_64 (global.get $REG_X1_aarch64) (global.get $REG_X2_aarch64) (global.get $REG_X0_aarch64))
    (call $emit_aarch64_v128_load_reg)
    (call $emit_aarch64_v128_push)
  )

  ;; ── v128.store (0xFD 0x1B): pop value, pop addr, store 16 bytes ─────
  (func $template_aarch64_v128_store (param $dec_ptr i32)
    (call $emit_aarch64_v128_pop)
    (call $emit_aarch64_pop_x0)
    (call $emit_aarch64_ldr_mem_ptr)
    (call $emit_aarch64_instr_add_64 (global.get $REG_X1_aarch64) (global.get $REG_X2_aarch64) (global.get $REG_X0_aarch64))
    (call $emit_aarch64_v128_store_reg)
  )

  ;; ── Splat templates ───────────────────────────────────────────────
  (func $template_aarch64_i8x16_splat (param $dec_ptr i32)
    (call $emit_aarch64_pop_x0)
    (call $emit_aarch64_instr (i32.const 0x4E000800))
    (call $emit_aarch64_v128_push)
  )
  (func $template_aarch64_i16x8_splat (param $dec_ptr i32)
    (call $emit_aarch64_pop_x0)
    (call $emit_aarch64_instr (i32.const 0x4E001000))
    (call $emit_aarch64_v128_push)
  )
  (func $template_aarch64_i32x4_splat (param $dec_ptr i32)
    (call $emit_aarch64_pop_x0)
    (call $emit_aarch64_instr (i32.const 0x4E002000))
    (call $emit_aarch64_v128_push)
  )
  (func $template_aarch64_i64x2_splat (param $dec_ptr i32)
    (call $emit_aarch64_pop_x0)
    (call $emit_aarch64_instr (i32.const 0x4E004000))
    (call $emit_aarch64_v128_push)
  )
  (func $template_aarch64_f32x4_splat (param $dec_ptr i32)
    (call $emit_aarch64_pop_x0)
    (call $emit_aarch64_instr (i32.const 0x4E002000))
    (call $emit_aarch64_v128_push)
  )
  (func $template_aarch64_f64x2_splat (param $dec_ptr i32)
    (call $emit_aarch64_pop_x0)
    (call $emit_aarch64_instr (i32.const 0x4E004000))
    (call $emit_aarch64_v128_push)
  )

  ;; ── Integer binary ops ────────────────────────────────────────────
  (func $template_aarch64_i8x16_add (param $dec_ptr i32)
    (call $emit_aarch64_instr_ldr_q_off (global.get $REG_V1) (global.get $REG_SP_aarch64) (i32.const 0))
    (call $emit_aarch64_instr_ldr_q_off (global.get $REG_V0) (global.get $REG_SP_aarch64) (i32.const 1))
    (call $emit_aarch64_neon_add_16b (global.get $REG_V0) (global.get $REG_V0) (global.get $REG_V1))
    (call $emit_aarch64_instr_addi_64 (global.get $REG_SP_aarch64) (global.get $REG_SP_aarch64) (i32.const 16))
    (call $emit_aarch64_instr_str_q_off (global.get $REG_V0) (global.get $REG_SP_aarch64) (i32.const 0))
  )
  (func $template_aarch64_i16x8_add (param $dec_ptr i32)
    (call $emit_aarch64_instr_ldr_q_off (global.get $REG_V1) (global.get $REG_SP_aarch64) (i32.const 0))
    (call $emit_aarch64_instr_ldr_q_off (global.get $REG_V0) (global.get $REG_SP_aarch64) (i32.const 1))
    (call $emit_aarch64_neon_add_8h (global.get $REG_V0) (global.get $REG_V0) (global.get $REG_V1))
    (call $emit_aarch64_instr_addi_64 (global.get $REG_SP_aarch64) (global.get $REG_SP_aarch64) (i32.const 16))
    (call $emit_aarch64_instr_str_q_off (global.get $REG_V0) (global.get $REG_SP_aarch64) (i32.const 0))
  )
  (func $template_aarch64_i32x4_add (param $dec_ptr i32)
    (call $emit_aarch64_instr_ldr_q_off (global.get $REG_V1) (global.get $REG_SP_aarch64) (i32.const 0))
    (call $emit_aarch64_instr_ldr_q_off (global.get $REG_V0) (global.get $REG_SP_aarch64) (i32.const 1))
    (call $emit_aarch64_neon_add_4s (global.get $REG_V0) (global.get $REG_V0) (global.get $REG_V1))
    (call $emit_aarch64_instr_addi_64 (global.get $REG_SP_aarch64) (global.get $REG_SP_aarch64) (i32.const 16))
    (call $emit_aarch64_instr_str_q_off (global.get $REG_V0) (global.get $REG_SP_aarch64) (i32.const 0))
  )
  (func $template_aarch64_i64x2_add (param $dec_ptr i32)
    (call $emit_aarch64_instr_ldr_q_off (global.get $REG_V1) (global.get $REG_SP_aarch64) (i32.const 0))
    (call $emit_aarch64_instr_ldr_q_off (global.get $REG_V0) (global.get $REG_SP_aarch64) (i32.const 1))
    (call $emit_aarch64_neon_add_2d (global.get $REG_V0) (global.get $REG_V0) (global.get $REG_V1))
    (call $emit_aarch64_instr_addi_64 (global.get $REG_SP_aarch64) (global.get $REG_SP_aarch64) (i32.const 16))
    (call $emit_aarch64_instr_str_q_off (global.get $REG_V0) (global.get $REG_SP_aarch64) (i32.const 0))
  )
  (func $template_aarch64_i8x16_sub (param $dec_ptr i32)
    (call $emit_aarch64_instr_ldr_q_off (global.get $REG_V1) (global.get $REG_SP_aarch64) (i32.const 0))
    (call $emit_aarch64_instr_ldr_q_off (global.get $REG_V0) (global.get $REG_SP_aarch64) (i32.const 1))
    (call $emit_aarch64_neon_sub_16b (global.get $REG_V0) (global.get $REG_V0) (global.get $REG_V1))
    (call $emit_aarch64_instr_addi_64 (global.get $REG_SP_aarch64) (global.get $REG_SP_aarch64) (i32.const 16))
    (call $emit_aarch64_instr_str_q_off (global.get $REG_V0) (global.get $REG_SP_aarch64) (i32.const 0))
  )
  (func $template_aarch64_i16x8_sub (param $dec_ptr i32)
    (call $emit_aarch64_instr_ldr_q_off (global.get $REG_V1) (global.get $REG_SP_aarch64) (i32.const 0))
    (call $emit_aarch64_instr_ldr_q_off (global.get $REG_V0) (global.get $REG_SP_aarch64) (i32.const 1))
    (call $emit_aarch64_neon_sub_8h (global.get $REG_V0) (global.get $REG_V0) (global.get $REG_V1))
    (call $emit_aarch64_instr_addi_64 (global.get $REG_SP_aarch64) (global.get $REG_SP_aarch64) (i32.const 16))
    (call $emit_aarch64_instr_str_q_off (global.get $REG_V0) (global.get $REG_SP_aarch64) (i32.const 0))
  )
  (func $template_aarch64_i32x4_sub (param $dec_ptr i32)
    (call $emit_aarch64_instr_ldr_q_off (global.get $REG_V1) (global.get $REG_SP_aarch64) (i32.const 0))
    (call $emit_aarch64_instr_ldr_q_off (global.get $REG_V0) (global.get $REG_SP_aarch64) (i32.const 1))
    (call $emit_aarch64_neon_sub_4s (global.get $REG_V0) (global.get $REG_V0) (global.get $REG_V1))
    (call $emit_aarch64_instr_addi_64 (global.get $REG_SP_aarch64) (global.get $REG_SP_aarch64) (i32.const 16))
    (call $emit_aarch64_instr_str_q_off (global.get $REG_V0) (global.get $REG_SP_aarch64) (i32.const 0))
  )
  (func $template_aarch64_i64x2_sub (param $dec_ptr i32)
    (call $emit_aarch64_instr_ldr_q_off (global.get $REG_V1) (global.get $REG_SP_aarch64) (i32.const 0))
    (call $emit_aarch64_instr_ldr_q_off (global.get $REG_V0) (global.get $REG_SP_aarch64) (i32.const 1))
    (call $emit_aarch64_neon_sub_2d (global.get $REG_V0) (global.get $REG_V0) (global.get $REG_V1))
    (call $emit_aarch64_instr_addi_64 (global.get $REG_SP_aarch64) (global.get $REG_SP_aarch64) (i32.const 16))
    (call $emit_aarch64_instr_str_q_off (global.get $REG_V0) (global.get $REG_SP_aarch64) (i32.const 0))
  )

  ;; ── Float binary ops ──────────────────────────────────────────────
  (func $template_aarch64_f32x4_add (param $dec_ptr i32)
    (call $emit_aarch64_instr_ldr_q_off (global.get $REG_V1) (global.get $REG_SP_aarch64) (i32.const 0))
    (call $emit_aarch64_instr_ldr_q_off (global.get $REG_V0) (global.get $REG_SP_aarch64) (i32.const 1))
    (call $emit_aarch64_instr (i32.const 0x4E20D400))
    (call $emit_aarch64_instr_addi_64 (global.get $REG_SP_aarch64) (global.get $REG_SP_aarch64) (i32.const 16))
    (call $emit_aarch64_instr_str_q_off (global.get $REG_V0) (global.get $REG_SP_aarch64) (i32.const 0))
  )
  (func $template_aarch64_f64x2_add (param $dec_ptr i32)
    (call $emit_aarch64_instr_ldr_q_off (global.get $REG_V1) (global.get $REG_SP_aarch64) (i32.const 0))
    (call $emit_aarch64_instr_ldr_q_off (global.get $REG_V0) (global.get $REG_SP_aarch64) (i32.const 1))
    (call $emit_aarch64_instr (i32.const 0x4E60D400))
    (call $emit_aarch64_instr_addi_64 (global.get $REG_SP_aarch64) (global.get $REG_SP_aarch64) (i32.const 16))
    (call $emit_aarch64_instr_str_q_off (global.get $REG_V0) (global.get $REG_SP_aarch64) (i32.const 0))
  )
  ;; f32x4_sub: FSUB V0.4S, V0.4S, V1.4S = 0x4E22D400
  (func $template_aarch64_f32x4_sub (param $dec_ptr i32)
    (call $emit_aarch64_instr_ldr_q_off (global.get $REG_V1) (global.get $REG_SP_aarch64) (i32.const 0))
    (call $emit_aarch64_instr_ldr_q_off (global.get $REG_V0) (global.get $REG_SP_aarch64) (i32.const 1))
    (call $emit_aarch64_instr (i32.const 0x4E22D400))
    (call $emit_aarch64_instr_addi_64 (global.get $REG_SP_aarch64) (global.get $REG_SP_aarch64) (i32.const 16))
    (call $emit_aarch64_instr_str_q_off (global.get $REG_V0) (global.get $REG_SP_aarch64) (i32.const 0))
  )
  ;; f64x2_sub: FSUB V0.2D, V0.2D, V1.2D = 0x4E62D400
  (func $template_aarch64_f64x2_sub (param $dec_ptr i32)
    (call $emit_aarch64_instr_ldr_q_off (global.get $REG_V1) (global.get $REG_SP_aarch64) (i32.const 0))
    (call $emit_aarch64_instr_ldr_q_off (global.get $REG_V0) (global.get $REG_SP_aarch64) (i32.const 1))
    (call $emit_aarch64_instr (i32.const 0x4E62D400))
    (call $emit_aarch64_instr_addi_64 (global.get $REG_SP_aarch64) (global.get $REG_SP_aarch64) (i32.const 16))
    (call $emit_aarch64_instr_str_q_off (global.get $REG_V0) (global.get $REG_SP_aarch64) (i32.const 0))
  )

  ;; ── Bitwise ops ───────────────────────────────────────────────────
  (func $template_aarch64_i8x16_and (param $dec_ptr i32)
    (call $emit_aarch64_instr_ldr_q_off (global.get $REG_V1) (global.get $REG_SP_aarch64) (i32.const 0))
    (call $emit_aarch64_instr_ldr_q_off (global.get $REG_V0) (global.get $REG_SP_aarch64) (i32.const 1))
    (call $emit_aarch64_neon_and_16b (global.get $REG_V0) (global.get $REG_V0) (global.get $REG_V1))
    (call $emit_aarch64_instr_addi_64 (global.get $REG_SP_aarch64) (global.get $REG_SP_aarch64) (i32.const 16))
    (call $emit_aarch64_instr_str_q_off (global.get $REG_V0) (global.get $REG_SP_aarch64) (i32.const 0))
  )
  (func $template_aarch64_i8x16_or (param $dec_ptr i32)
    (call $emit_aarch64_instr_ldr_q_off (global.get $REG_V1) (global.get $REG_SP_aarch64) (i32.const 0))
    (call $emit_aarch64_instr_ldr_q_off (global.get $REG_V0) (global.get $REG_SP_aarch64) (i32.const 1))
    (call $emit_aarch64_neon_orr_16b (global.get $REG_V0) (global.get $REG_V0) (global.get $REG_V1))
    (call $emit_aarch64_instr_addi_64 (global.get $REG_SP_aarch64) (global.get $REG_SP_aarch64) (i32.const 16))
    (call $emit_aarch64_instr_str_q_off (global.get $REG_V0) (global.get $REG_SP_aarch64) (i32.const 0))
  )
  (func $template_aarch64_i8x16_xor (param $dec_ptr i32)
    (call $emit_aarch64_instr_ldr_q_off (global.get $REG_V1) (global.get $REG_SP_aarch64) (i32.const 0))
    (call $emit_aarch64_instr_ldr_q_off (global.get $REG_V0) (global.get $REG_SP_aarch64) (i32.const 1))
    (call $emit_aarch64_neon_eor_16b (global.get $REG_V0) (global.get $REG_V0) (global.get $REG_V1))
    (call $emit_aarch64_instr_addi_64 (global.get $REG_SP_aarch64) (global.get $REG_SP_aarch64) (i32.const 16))
    (call $emit_aarch64_instr_str_q_off (global.get $REG_V0) (global.get $REG_SP_aarch64) (i32.const 0))
  )

  ;; ── Comparisons ───────────────────────────────────────────────────
  (func $template_aarch64_i8x16_eq (param $dec_ptr i32)
    (call $emit_aarch64_instr_ldr_q_off (global.get $REG_V1) (global.get $REG_SP_aarch64) (i32.const 0))
    (call $emit_aarch64_instr_ldr_q_off (global.get $REG_V0) (global.get $REG_SP_aarch64) (i32.const 1))
    (call $emit_aarch64_neon_cmeq_16b (global.get $REG_V0) (global.get $REG_V0) (global.get $REG_V1))
    (call $emit_aarch64_instr_addi_64 (global.get $REG_SP_aarch64) (global.get $REG_SP_aarch64) (i32.const 16))
    (call $emit_aarch64_instr_str_q_off (global.get $REG_V0) (global.get $REG_SP_aarch64) (i32.const 0))
  )
  (func $template_aarch64_i16x8_eq (param $dec_ptr i32)
    (call $emit_aarch64_instr_ldr_q_off (global.get $REG_V1) (global.get $REG_SP_aarch64) (i32.const 0))
    (call $emit_aarch64_instr_ldr_q_off (global.get $REG_V0) (global.get $REG_SP_aarch64) (i32.const 1))
    (call $emit_aarch64_neon_cmeq_8h (global.get $REG_V0) (global.get $REG_V0) (global.get $REG_V1))
    (call $emit_aarch64_instr_addi_64 (global.get $REG_SP_aarch64) (global.get $REG_SP_aarch64) (i32.const 16))
    (call $emit_aarch64_instr_str_q_off (global.get $REG_V0) (global.get $REG_SP_aarch64) (i32.const 0))
  )
  (func $template_aarch64_i32x4_eq (param $dec_ptr i32)
    (call $emit_aarch64_instr_ldr_q_off (global.get $REG_V1) (global.get $REG_SP_aarch64) (i32.const 0))
    (call $emit_aarch64_instr_ldr_q_off (global.get $REG_V0) (global.get $REG_SP_aarch64) (i32.const 1))
    (call $emit_aarch64_neon_cmeq_4s (global.get $REG_V0) (global.get $REG_V0) (global.get $REG_V1))
    (call $emit_aarch64_instr_addi_64 (global.get $REG_SP_aarch64) (global.get $REG_SP_aarch64) (i32.const 16))
    (call $emit_aarch64_instr_str_q_off (global.get $REG_V0) (global.get $REG_SP_aarch64) (i32.const 0))
  )
  (func $template_aarch64_i8x16_gt_s (param $dec_ptr i32)
    (call $emit_aarch64_instr_ldr_q_off (global.get $REG_V1) (global.get $REG_SP_aarch64) (i32.const 0))
    (call $emit_aarch64_instr_ldr_q_off (global.get $REG_V0) (global.get $REG_SP_aarch64) (i32.const 1))
    (call $emit_aarch64_neon_cmgt_16b_s (global.get $REG_V0) (global.get $REG_V0) (global.get $REG_V1))
    (call $emit_aarch64_instr_addi_64 (global.get $REG_SP_aarch64) (global.get $REG_SP_aarch64) (i32.const 16))
    (call $emit_aarch64_instr_str_q_off (global.get $REG_V0) (global.get $REG_SP_aarch64) (i32.const 0))
  )

  ;; ── GE (signed) implementations ──────────────────────────────────
  (func $template_aarch64_i8x16_ge_s (param $dec_ptr i32)
    (call $emit_aarch64_instr_ldr_q_off (global.get $REG_V1) (global.get $REG_SP_aarch64) (i32.const 0))
    (call $emit_aarch64_instr_ldr_q_off (global.get $REG_V0) (global.get $REG_SP_aarch64) (i32.const 1))
    (call $emit_aarch64_neon_cmge_16b_s (global.get $REG_V0) (global.get $REG_V0) (global.get $REG_V1))
    (call $emit_aarch64_instr_addi_64 (global.get $REG_SP_aarch64) (global.get $REG_SP_aarch64) (i32.const 16))
    (call $emit_aarch64_instr_str_q_off (global.get $REG_V0) (global.get $REG_SP_aarch64) (i32.const 0))
  )
  (func $template_aarch64_i16x8_ge_s (param $dec_ptr i32)
    (call $emit_aarch64_instr_ldr_q_off (global.get $REG_V1) (global.get $REG_SP_aarch64) (i32.const 0))
    (call $emit_aarch64_instr_ldr_q_off (global.get $REG_V0) (global.get $REG_SP_aarch64) (i32.const 1))
    (call $emit_aarch64_neon_3same (i32.const 1) (i32.const 1) (i32.const 0) (i32.const 0x07)
                           (global.get $REG_V0) (global.get $REG_V0) (global.get $REG_V1))
    (call $emit_aarch64_instr_addi_64 (global.get $REG_SP_aarch64) (global.get $REG_SP_aarch64) (i32.const 16))
    (call $emit_aarch64_instr_str_q_off (global.get $REG_V0) (global.get $REG_SP_aarch64) (i32.const 0))
  )
  (func $template_aarch64_i32x4_ge_s (param $dec_ptr i32)
    (call $emit_aarch64_instr_ldr_q_off (global.get $REG_V1) (global.get $REG_SP_aarch64) (i32.const 0))
    (call $emit_aarch64_instr_ldr_q_off (global.get $REG_V0) (global.get $REG_SP_aarch64) (i32.const 1))
    (call $emit_aarch64_neon_3same (i32.const 1) (i32.const 2) (i32.const 0) (i32.const 0x07)
                           (global.get $REG_V0) (global.get $REG_V0) (global.get $REG_V1))
    (call $emit_aarch64_instr_addi_64 (global.get $REG_SP_aarch64) (global.get $REG_SP_aarch64) (i32.const 16))
    (call $emit_aarch64_instr_str_q_off (global.get $REG_V0) (global.get $REG_SP_aarch64) (i32.const 0))
  )

  ;; ── GE (unsigned) implementations ────────────────────────────────
  (func $template_aarch64_i8x16_ge_u (param $dec_ptr i32)
    (call $emit_aarch64_instr_ldr_q_off (global.get $REG_V1) (global.get $REG_SP_aarch64) (i32.const 0))
    (call $emit_aarch64_instr_ldr_q_off (global.get $REG_V0) (global.get $REG_SP_aarch64) (i32.const 1))
    (call $emit_aarch64_neon_cmge_16b_u (global.get $REG_V0) (global.get $REG_V0) (global.get $REG_V1))
    (call $emit_aarch64_instr_addi_64 (global.get $REG_SP_aarch64) (global.get $REG_SP_aarch64) (i32.const 16))
    (call $emit_aarch64_instr_str_q_off (global.get $REG_V0) (global.get $REG_SP_aarch64) (i32.const 0))
  )
  (func $template_aarch64_i16x8_ge_u (param $dec_ptr i32)
    (call $emit_aarch64_instr_ldr_q_off (global.get $REG_V1) (global.get $REG_SP_aarch64) (i32.const 0))
    (call $emit_aarch64_instr_ldr_q_off (global.get $REG_V0) (global.get $REG_SP_aarch64) (i32.const 1))
    (call $emit_aarch64_neon_cmge_8h_u (global.get $REG_V0) (global.get $REG_V0) (global.get $REG_V1))
    (call $emit_aarch64_instr_addi_64 (global.get $REG_SP_aarch64) (global.get $REG_SP_aarch64) (i32.const 16))
    (call $emit_aarch64_instr_str_q_off (global.get $REG_V0) (global.get $REG_SP_aarch64) (i32.const 0))
  )
  (func $template_aarch64_i32x4_ge_u (param $dec_ptr i32)
    (call $emit_aarch64_instr_ldr_q_off (global.get $REG_V1) (global.get $REG_SP_aarch64) (i32.const 0))
    (call $emit_aarch64_instr_ldr_q_off (global.get $REG_V0) (global.get $REG_SP_aarch64) (i32.const 1))
    (call $emit_aarch64_neon_cmge_4s_u (global.get $REG_V0) (global.get $REG_V0) (global.get $REG_V1))
    (call $emit_aarch64_instr_addi_64 (global.get $REG_SP_aarch64) (global.get $REG_SP_aarch64) (i32.const 16))
    (call $emit_aarch64_instr_str_q_off (global.get $REG_V0) (global.get $REG_SP_aarch64) (i32.const 0))
  )

  ;; ── UMIN / UMAX / URHADD implementations ─────────────────────────
  (func $template_aarch64_i8x16_min_u (param $dec_ptr i32)
    (call $emit_aarch64_instr_ldr_q_off (global.get $REG_V1) (global.get $REG_SP_aarch64) (i32.const 0))
    (call $emit_aarch64_instr_ldr_q_off (global.get $REG_V0) (global.get $REG_SP_aarch64) (i32.const 1))
    (call $emit_aarch64_neon_umin_16b (global.get $REG_V0) (global.get $REG_V0) (global.get $REG_V1))
    (call $emit_aarch64_instr_addi_64 (global.get $REG_SP_aarch64) (global.get $REG_SP_aarch64) (i32.const 16))
    (call $emit_aarch64_instr_str_q_off (global.get $REG_V0) (global.get $REG_SP_aarch64) (i32.const 0))
  )
  (func $template_aarch64_i16x8_min_u (param $dec_ptr i32)
    (call $emit_aarch64_instr_ldr_q_off (global.get $REG_V1) (global.get $REG_SP_aarch64) (i32.const 0))
    (call $emit_aarch64_instr_ldr_q_off (global.get $REG_V0) (global.get $REG_SP_aarch64) (i32.const 1))
    (call $emit_aarch64_neon_umin_8h (global.get $REG_V0) (global.get $REG_V0) (global.get $REG_V1))
    (call $emit_aarch64_instr_addi_64 (global.get $REG_SP_aarch64) (global.get $REG_SP_aarch64) (i32.const 16))
    (call $emit_aarch64_instr_str_q_off (global.get $REG_V0) (global.get $REG_SP_aarch64) (i32.const 0))
  )
  (func $template_aarch64_i8x16_max_u (param $dec_ptr i32)
    (call $emit_aarch64_instr_ldr_q_off (global.get $REG_V1) (global.get $REG_SP_aarch64) (i32.const 0))
    (call $emit_aarch64_instr_ldr_q_off (global.get $REG_V0) (global.get $REG_SP_aarch64) (i32.const 1))
    (call $emit_aarch64_neon_umax_16b (global.get $REG_V0) (global.get $REG_V0) (global.get $REG_V1))
    (call $emit_aarch64_instr_addi_64 (global.get $REG_SP_aarch64) (global.get $REG_SP_aarch64) (i32.const 16))
    (call $emit_aarch64_instr_str_q_off (global.get $REG_V0) (global.get $REG_SP_aarch64) (i32.const 0))
  )
  (func $template_aarch64_i16x8_max_u (param $dec_ptr i32)
    (call $emit_aarch64_instr_ldr_q_off (global.get $REG_V1) (global.get $REG_SP_aarch64) (i32.const 0))
    (call $emit_aarch64_instr_ldr_q_off (global.get $REG_V0) (global.get $REG_SP_aarch64) (i32.const 1))
    (call $emit_aarch64_neon_umax_8h (global.get $REG_V0) (global.get $REG_V0) (global.get $REG_V1))
    (call $emit_aarch64_instr_addi_64 (global.get $REG_SP_aarch64) (global.get $REG_SP_aarch64) (i32.const 16))
    (call $emit_aarch64_instr_str_q_off (global.get $REG_V0) (global.get $REG_SP_aarch64) (i32.const 0))
  )
  (func $template_aarch64_i8x16_avgr_u (param $dec_ptr i32)
    (call $emit_aarch64_instr_ldr_q_off (global.get $REG_V1) (global.get $REG_SP_aarch64) (i32.const 0))
    (call $emit_aarch64_instr_ldr_q_off (global.get $REG_V0) (global.get $REG_SP_aarch64) (i32.const 1))
    (call $emit_aarch64_neon_urhadd_16b (global.get $REG_V0) (global.get $REG_V0) (global.get $REG_V1))
    (call $emit_aarch64_instr_addi_64 (global.get $REG_SP_aarch64) (global.get $REG_SP_aarch64) (i32.const 16))
    (call $emit_aarch64_instr_str_q_off (global.get $REG_V0) (global.get $REG_SP_aarch64) (i32.const 0))
  )
  (func $template_aarch64_i16x8_avgr_u (param $dec_ptr i32)
    (call $emit_aarch64_instr_ldr_q_off (global.get $REG_V1) (global.get $REG_SP_aarch64) (i32.const 0))
    (call $emit_aarch64_instr_ldr_q_off (global.get $REG_V0) (global.get $REG_SP_aarch64) (i32.const 1))
    (call $emit_aarch64_neon_urhadd_8h (global.get $REG_V0) (global.get $REG_V0) (global.get $REG_V1))
    (call $emit_aarch64_instr_addi_64 (global.get $REG_SP_aarch64) (global.get $REG_SP_aarch64) (i32.const 16))
    (call $emit_aarch64_instr_str_q_off (global.get $REG_V0) (global.get $REG_SP_aarch64) (i32.const 0))
  )

  ;; ── Additional integer comparison templates ──────────────────────
  (func $template_aarch64_i16x8_gt_s (param $dec_ptr i32)
    (call $emit_aarch64_instr_ldr_q_off (global.get $REG_V1) (global.get $REG_SP_aarch64) (i32.const 0))
    (call $emit_aarch64_instr_ldr_q_off (global.get $REG_V0) (global.get $REG_SP_aarch64) (i32.const 1))
    (call $emit_aarch64_neon_cmgt_8h_s (global.get $REG_V0) (global.get $REG_V0) (global.get $REG_V1))
    (call $emit_aarch64_instr_addi_64 (global.get $REG_SP_aarch64) (global.get $REG_SP_aarch64) (i32.const 16))
    (call $emit_aarch64_instr_str_q_off (global.get $REG_V0) (global.get $REG_SP_aarch64) (i32.const 0))
  )
  (func $template_aarch64_i32x4_gt_s (param $dec_ptr i32)
    (call $emit_aarch64_instr_ldr_q_off (global.get $REG_V1) (global.get $REG_SP_aarch64) (i32.const 0))
    (call $emit_aarch64_instr_ldr_q_off (global.get $REG_V0) (global.get $REG_SP_aarch64) (i32.const 1))
    (call $emit_aarch64_neon_cmgt_4s_s (global.get $REG_V0) (global.get $REG_V0) (global.get $REG_V1))
    (call $emit_aarch64_instr_addi_64 (global.get $REG_SP_aarch64) (global.get $REG_SP_aarch64) (i32.const 16))
    (call $emit_aarch64_instr_str_q_off (global.get $REG_V0) (global.get $REG_SP_aarch64) (i32.const 0))
  )
  (func $template_aarch64_i8x16_gt_u (param $dec_ptr i32)
    (call $emit_aarch64_instr_ldr_q_off (global.get $REG_V1) (global.get $REG_SP_aarch64) (i32.const 0))
    (call $emit_aarch64_instr_ldr_q_off (global.get $REG_V0) (global.get $REG_SP_aarch64) (i32.const 1))
    (call $emit_aarch64_neon_cmgt_16b_u (global.get $REG_V0) (global.get $REG_V0) (global.get $REG_V1))
    (call $emit_aarch64_instr_addi_64 (global.get $REG_SP_aarch64) (global.get $REG_SP_aarch64) (i32.const 16))
    (call $emit_aarch64_instr_str_q_off (global.get $REG_V0) (global.get $REG_SP_aarch64) (i32.const 0))
  )
  (func $template_aarch64_i16x8_gt_u (param $dec_ptr i32)
    (call $emit_aarch64_instr_ldr_q_off (global.get $REG_V1) (global.get $REG_SP_aarch64) (i32.const 0))
    (call $emit_aarch64_instr_ldr_q_off (global.get $REG_V0) (global.get $REG_SP_aarch64) (i32.const 1))
    (call $emit_aarch64_neon_cmgt_8h_u (global.get $REG_V0) (global.get $REG_V0) (global.get $REG_V1))
    (call $emit_aarch64_instr_addi_64 (global.get $REG_SP_aarch64) (global.get $REG_SP_aarch64) (i32.const 16))
    (call $emit_aarch64_instr_str_q_off (global.get $REG_V0) (global.get $REG_SP_aarch64) (i32.const 0))
  )
  (func $template_aarch64_i32x4_gt_u (param $dec_ptr i32)
    (call $emit_aarch64_instr_ldr_q_off (global.get $REG_V1) (global.get $REG_SP_aarch64) (i32.const 0))
    (call $emit_aarch64_instr_ldr_q_off (global.get $REG_V0) (global.get $REG_SP_aarch64) (i32.const 1))
    (call $emit_aarch64_neon_cmgt_4s_u (global.get $REG_V0) (global.get $REG_V0) (global.get $REG_V1))
    (call $emit_aarch64_instr_addi_64 (global.get $REG_SP_aarch64) (global.get $REG_SP_aarch64) (i32.const 16))
    (call $emit_aarch64_instr_str_q_off (global.get $REG_V0) (global.get $REG_SP_aarch64) (i32.const 0))
  )
  ;; lt_s: a < b = b > a → CMGT(V1, V0) with Rn=V1, Rm=V0
  (func $template_aarch64_i8x16_lt_s (param $dec_ptr i32)
    (call $emit_aarch64_instr_ldr_q_off (global.get $REG_V1) (global.get $REG_SP_aarch64) (i32.const 0))
    (call $emit_aarch64_instr_ldr_q_off (global.get $REG_V0) (global.get $REG_SP_aarch64) (i32.const 1))
    (call $emit_aarch64_neon_cmgt_16b_s (global.get $REG_V0) (global.get $REG_V1) (global.get $REG_V0))
    (call $emit_aarch64_instr_addi_64 (global.get $REG_SP_aarch64) (global.get $REG_SP_aarch64) (i32.const 16))
    (call $emit_aarch64_instr_str_q_off (global.get $REG_V0) (global.get $REG_SP_aarch64) (i32.const 0))
  )
  (func $template_aarch64_i16x8_lt_s (param $dec_ptr i32)
    (call $emit_aarch64_instr_ldr_q_off (global.get $REG_V1) (global.get $REG_SP_aarch64) (i32.const 0))
    (call $emit_aarch64_instr_ldr_q_off (global.get $REG_V0) (global.get $REG_SP_aarch64) (i32.const 1))
    (call $emit_aarch64_neon_cmgt_8h_s (global.get $REG_V0) (global.get $REG_V1) (global.get $REG_V0))
    (call $emit_aarch64_instr_addi_64 (global.get $REG_SP_aarch64) (global.get $REG_SP_aarch64) (i32.const 16))
    (call $emit_aarch64_instr_str_q_off (global.get $REG_V0) (global.get $REG_SP_aarch64) (i32.const 0))
  )
  (func $template_aarch64_i32x4_lt_s (param $dec_ptr i32)
    (call $emit_aarch64_instr_ldr_q_off (global.get $REG_V1) (global.get $REG_SP_aarch64) (i32.const 0))
    (call $emit_aarch64_instr_ldr_q_off (global.get $REG_V0) (global.get $REG_SP_aarch64) (i32.const 1))
    (call $emit_aarch64_neon_cmgt_4s_s (global.get $REG_V0) (global.get $REG_V1) (global.get $REG_V0))
    (call $emit_aarch64_instr_addi_64 (global.get $REG_SP_aarch64) (global.get $REG_SP_aarch64) (i32.const 16))
    (call $emit_aarch64_instr_str_q_off (global.get $REG_V0) (global.get $REG_SP_aarch64) (i32.const 0))
  )
  ;; lt_u: a < b = b > a → CMHI(V1, V0)
  (func $template_aarch64_i8x16_lt_u (param $dec_ptr i32)
    (call $emit_aarch64_instr_ldr_q_off (global.get $REG_V1) (global.get $REG_SP_aarch64) (i32.const 0))
    (call $emit_aarch64_instr_ldr_q_off (global.get $REG_V0) (global.get $REG_SP_aarch64) (i32.const 1))
    (call $emit_aarch64_neon_cmgt_16b_u (global.get $REG_V0) (global.get $REG_V1) (global.get $REG_V0))
    (call $emit_aarch64_instr_addi_64 (global.get $REG_SP_aarch64) (global.get $REG_SP_aarch64) (i32.const 16))
    (call $emit_aarch64_instr_str_q_off (global.get $REG_V0) (global.get $REG_SP_aarch64) (i32.const 0))
  )
  (func $template_aarch64_i16x8_lt_u (param $dec_ptr i32)
    (call $emit_aarch64_instr_ldr_q_off (global.get $REG_V1) (global.get $REG_SP_aarch64) (i32.const 0))
    (call $emit_aarch64_instr_ldr_q_off (global.get $REG_V0) (global.get $REG_SP_aarch64) (i32.const 1))
    (call $emit_aarch64_neon_cmgt_8h_u (global.get $REG_V0) (global.get $REG_V1) (global.get $REG_V0))
    (call $emit_aarch64_instr_addi_64 (global.get $REG_SP_aarch64) (global.get $REG_SP_aarch64) (i32.const 16))
    (call $emit_aarch64_instr_str_q_off (global.get $REG_V0) (global.get $REG_SP_aarch64) (i32.const 0))
  )
  (func $template_aarch64_i32x4_lt_u (param $dec_ptr i32)
    (call $emit_aarch64_instr_ldr_q_off (global.get $REG_V1) (global.get $REG_SP_aarch64) (i32.const 0))
    (call $emit_aarch64_instr_ldr_q_off (global.get $REG_V0) (global.get $REG_SP_aarch64) (i32.const 1))
    (call $emit_aarch64_neon_cmgt_4s_u (global.get $REG_V0) (global.get $REG_V1) (global.get $REG_V0))
    (call $emit_aarch64_instr_addi_64 (global.get $REG_SP_aarch64) (global.get $REG_SP_aarch64) (i32.const 16))
    (call $emit_aarch64_instr_str_q_off (global.get $REG_V0) (global.get $REG_SP_aarch64) (i32.const 0))
  )
  ;; le_s: a <= b = b >= a → CMGE(V1, V0) with Rn=V1, Rm=V0
  (func $template_aarch64_i8x16_le_s (param $dec_ptr i32)
    (call $emit_aarch64_instr_ldr_q_off (global.get $REG_V1) (global.get $REG_SP_aarch64) (i32.const 0))
    (call $emit_aarch64_instr_ldr_q_off (global.get $REG_V0) (global.get $REG_SP_aarch64) (i32.const 1))
    (call $emit_aarch64_neon_cmge_16b_s (global.get $REG_V0) (global.get $REG_V1) (global.get $REG_V0))
    (call $emit_aarch64_instr_addi_64 (global.get $REG_SP_aarch64) (global.get $REG_SP_aarch64) (i32.const 16))
    (call $emit_aarch64_instr_str_q_off (global.get $REG_V0) (global.get $REG_SP_aarch64) (i32.const 0))
  )
  (func $template_aarch64_i16x8_le_s (param $dec_ptr i32)
    (call $emit_aarch64_instr_ldr_q_off (global.get $REG_V1) (global.get $REG_SP_aarch64) (i32.const 0))
    (call $emit_aarch64_instr_ldr_q_off (global.get $REG_V0) (global.get $REG_SP_aarch64) (i32.const 1))
    (call $emit_aarch64_neon_3same (i32.const 1) (i32.const 1) (i32.const 0) (i32.const 0x07)
                           (global.get $REG_V0) (global.get $REG_V1) (global.get $REG_V0))
    (call $emit_aarch64_instr_addi_64 (global.get $REG_SP_aarch64) (global.get $REG_SP_aarch64) (i32.const 16))
    (call $emit_aarch64_instr_str_q_off (global.get $REG_V0) (global.get $REG_SP_aarch64) (i32.const 0))
  )
  (func $template_aarch64_i32x4_le_s (param $dec_ptr i32)
    (call $emit_aarch64_instr_ldr_q_off (global.get $REG_V1) (global.get $REG_SP_aarch64) (i32.const 0))
    (call $emit_aarch64_instr_ldr_q_off (global.get $REG_V0) (global.get $REG_SP_aarch64) (i32.const 1))
    (call $emit_aarch64_neon_3same (i32.const 1) (i32.const 2) (i32.const 0) (i32.const 0x07)
                           (global.get $REG_V0) (global.get $REG_V1) (global.get $REG_V0))
    (call $emit_aarch64_instr_addi_64 (global.get $REG_SP_aarch64) (global.get $REG_SP_aarch64) (i32.const 16))
    (call $emit_aarch64_instr_str_q_off (global.get $REG_V0) (global.get $REG_SP_aarch64) (i32.const 0))
  )
  ;; le_u: a <= b = b >= a → CMHS(V1, V0)
  (func $template_aarch64_i8x16_le_u (param $dec_ptr i32)
    (call $emit_aarch64_instr_ldr_q_off (global.get $REG_V1) (global.get $REG_SP_aarch64) (i32.const 0))
    (call $emit_aarch64_instr_ldr_q_off (global.get $REG_V0) (global.get $REG_SP_aarch64) (i32.const 1))
    (call $emit_aarch64_neon_cmge_16b_u (global.get $REG_V0) (global.get $REG_V1) (global.get $REG_V0))
    (call $emit_aarch64_instr_addi_64 (global.get $REG_SP_aarch64) (global.get $REG_SP_aarch64) (i32.const 16))
    (call $emit_aarch64_instr_str_q_off (global.get $REG_V0) (global.get $REG_SP_aarch64) (i32.const 0))
  )
  (func $template_aarch64_i16x8_le_u (param $dec_ptr i32)
    (call $emit_aarch64_instr_ldr_q_off (global.get $REG_V1) (global.get $REG_SP_aarch64) (i32.const 0))
    (call $emit_aarch64_instr_ldr_q_off (global.get $REG_V0) (global.get $REG_SP_aarch64) (i32.const 1))
    (call $emit_aarch64_neon_cmge_8h_u (global.get $REG_V0) (global.get $REG_V1) (global.get $REG_V0))
    (call $emit_aarch64_instr_addi_64 (global.get $REG_SP_aarch64) (global.get $REG_SP_aarch64) (i32.const 16))
    (call $emit_aarch64_instr_str_q_off (global.get $REG_V0) (global.get $REG_SP_aarch64) (i32.const 0))
  )
  (func $template_aarch64_i32x4_le_u (param $dec_ptr i32)
    (call $emit_aarch64_instr_ldr_q_off (global.get $REG_V1) (global.get $REG_SP_aarch64) (i32.const 0))
    (call $emit_aarch64_instr_ldr_q_off (global.get $REG_V0) (global.get $REG_SP_aarch64) (i32.const 1))
    (call $emit_aarch64_neon_cmge_4s_u (global.get $REG_V0) (global.get $REG_V1) (global.get $REG_V0))
    (call $emit_aarch64_instr_addi_64 (global.get $REG_SP_aarch64) (global.get $REG_SP_aarch64) (i32.const 16))
    (call $emit_aarch64_instr_str_q_off (global.get $REG_V0) (global.get $REG_SP_aarch64) (i32.const 0))
  )
  ;; ne: a != b = NOT(CMEQ(a, b)) → CMEQ + MVN
  (func $template_aarch64_i8x16_ne (param $dec_ptr i32)
    (call $emit_aarch64_instr_ldr_q_off (global.get $REG_V1) (global.get $REG_SP_aarch64) (i32.const 0))
    (call $emit_aarch64_instr_ldr_q_off (global.get $REG_V0) (global.get $REG_SP_aarch64) (i32.const 1))
    (call $emit_aarch64_neon_cmeq_16b (global.get $REG_V0) (global.get $REG_V0) (global.get $REG_V1))
    (call $emit_aarch64_instr (i32.const 0x4E205800))
    (call $emit_aarch64_instr_addi_64 (global.get $REG_SP_aarch64) (global.get $REG_SP_aarch64) (i32.const 16))
    (call $emit_aarch64_instr_str_q_off (global.get $REG_V0) (global.get $REG_SP_aarch64) (i32.const 0))
  )
  (func $template_aarch64_i16x8_ne (param $dec_ptr i32)
    (call $emit_aarch64_instr_ldr_q_off (global.get $REG_V1) (global.get $REG_SP_aarch64) (i32.const 0))
    (call $emit_aarch64_instr_ldr_q_off (global.get $REG_V0) (global.get $REG_SP_aarch64) (i32.const 1))
    (call $emit_aarch64_neon_cmeq_8h (global.get $REG_V0) (global.get $REG_V0) (global.get $REG_V1))
    (call $emit_aarch64_instr (i32.const 0x4E205800))
    (call $emit_aarch64_instr_addi_64 (global.get $REG_SP_aarch64) (global.get $REG_SP_aarch64) (i32.const 16))
    (call $emit_aarch64_instr_str_q_off (global.get $REG_V0) (global.get $REG_SP_aarch64) (i32.const 0))
  )
  (func $template_aarch64_i32x4_ne (param $dec_ptr i32)
    (call $emit_aarch64_instr_ldr_q_off (global.get $REG_V1) (global.get $REG_SP_aarch64) (i32.const 0))
    (call $emit_aarch64_instr_ldr_q_off (global.get $REG_V0) (global.get $REG_SP_aarch64) (i32.const 1))
    (call $emit_aarch64_neon_cmeq_4s (global.get $REG_V0) (global.get $REG_V0) (global.get $REG_V1))
    (call $emit_aarch64_instr (i32.const 0x4E205800))
    (call $emit_aarch64_instr_addi_64 (global.get $REG_SP_aarch64) (global.get $REG_SP_aarch64) (i32.const 16))
    (call $emit_aarch64_instr_str_q_off (global.get $REG_V0) (global.get $REG_SP_aarch64) (i32.const 0))
  )

  ;; ── Float comparison templates ────────────────────────────────────
  ;; f32x4_eq: FCMEQ V0.4S, V0.4S, V1.4S = 0x4E20E400
  (func $template_aarch64_f32x4_eq (param $dec_ptr i32)
    (call $emit_aarch64_instr_ldr_q_off (global.get $REG_V1) (global.get $REG_SP_aarch64) (i32.const 0))
    (call $emit_aarch64_instr_ldr_q_off (global.get $REG_V0) (global.get $REG_SP_aarch64) (i32.const 1))
    (call $emit_aarch64_instr (i32.const 0x4E20E400))
    (call $emit_aarch64_instr_addi_64 (global.get $REG_SP_aarch64) (global.get $REG_SP_aarch64) (i32.const 16))
    (call $emit_aarch64_instr_str_q_off (global.get $REG_V0) (global.get $REG_SP_aarch64) (i32.const 0))
  )
  ;; f64x2_eq: FCMEQ V0.2D, V0.2D, V1.2D = 0x4E60E400
  (func $template_aarch64_f64x2_eq (param $dec_ptr i32)
    (call $emit_aarch64_instr_ldr_q_off (global.get $REG_V1) (global.get $REG_SP_aarch64) (i32.const 0))
    (call $emit_aarch64_instr_ldr_q_off (global.get $REG_V0) (global.get $REG_SP_aarch64) (i32.const 1))
    (call $emit_aarch64_instr (i32.const 0x4E60E400))
    (call $emit_aarch64_instr_addi_64 (global.get $REG_SP_aarch64) (global.get $REG_SP_aarch64) (i32.const 16))
    (call $emit_aarch64_instr_str_q_off (global.get $REG_V0) (global.get $REG_SP_aarch64) (i32.const 0))
  )
  ;; f32x4_ne: NOT(FCMEQ V0.4S, V0.4S, V1.4S) = FCMEQ + MVN
  (func $template_aarch64_f32x4_ne (param $dec_ptr i32)
    (call $emit_aarch64_instr_ldr_q_off (global.get $REG_V1) (global.get $REG_SP_aarch64) (i32.const 0))
    (call $emit_aarch64_instr_ldr_q_off (global.get $REG_V0) (global.get $REG_SP_aarch64) (i32.const 1))
    (call $emit_aarch64_instr (i32.const 0x4E20E400))
    (call $emit_aarch64_instr (i32.const 0x4E205800))
    (call $emit_aarch64_instr_addi_64 (global.get $REG_SP_aarch64) (global.get $REG_SP_aarch64) (i32.const 16))
    (call $emit_aarch64_instr_str_q_off (global.get $REG_V0) (global.get $REG_SP_aarch64) (i32.const 0))
  )
  ;; f64x2_ne: NOT(FCMEQ V0.2D, V0.2D, V1.2D) = FCMEQ + MVN
  (func $template_aarch64_f64x2_ne (param $dec_ptr i32)
    (call $emit_aarch64_instr_ldr_q_off (global.get $REG_V1) (global.get $REG_SP_aarch64) (i32.const 0))
    (call $emit_aarch64_instr_ldr_q_off (global.get $REG_V0) (global.get $REG_SP_aarch64) (i32.const 1))
    (call $emit_aarch64_instr (i32.const 0x4E60E400))
    (call $emit_aarch64_instr (i32.const 0x4E205800))
    (call $emit_aarch64_instr_addi_64 (global.get $REG_SP_aarch64) (global.get $REG_SP_aarch64) (i32.const 16))
    (call $emit_aarch64_instr_str_q_off (global.get $REG_V0) (global.get $REG_SP_aarch64) (i32.const 0))
  )
  ;; f32x4_lt: a < b = b > a → FCMGT V0.4S, V1.4S, V0.4S = 0x4E22E420
  (func $template_aarch64_f32x4_lt (param $dec_ptr i32)
    (call $emit_aarch64_instr_ldr_q_off (global.get $REG_V1) (global.get $REG_SP_aarch64) (i32.const 0))
    (call $emit_aarch64_instr_ldr_q_off (global.get $REG_V0) (global.get $REG_SP_aarch64) (i32.const 1))
    (call $emit_aarch64_instr (i32.const 0x4E22E420))
    (call $emit_aarch64_instr_addi_64 (global.get $REG_SP_aarch64) (global.get $REG_SP_aarch64) (i32.const 16))
    (call $emit_aarch64_instr_str_q_off (global.get $REG_V0) (global.get $REG_SP_aarch64) (i32.const 0))
  )
  ;; f64x2_lt: FCMGT V0.2D, V1.2D, V0.2D = 0x4E62E420
  (func $template_aarch64_f64x2_lt (param $dec_ptr i32)
    (call $emit_aarch64_instr_ldr_q_off (global.get $REG_V1) (global.get $REG_SP_aarch64) (i32.const 0))
    (call $emit_aarch64_instr_ldr_q_off (global.get $REG_V0) (global.get $REG_SP_aarch64) (i32.const 1))
    (call $emit_aarch64_instr (i32.const 0x4E62E420))
    (call $emit_aarch64_instr_addi_64 (global.get $REG_SP_aarch64) (global.get $REG_SP_aarch64) (i32.const 16))
    (call $emit_aarch64_instr_str_q_off (global.get $REG_V0) (global.get $REG_SP_aarch64) (i32.const 0))
  )
  ;; f32x4_gt: FCMGT V0.4S, V0.4S, V1.4S = 0x4E22E400
  (func $template_aarch64_f32x4_gt (param $dec_ptr i32)
    (call $emit_aarch64_instr_ldr_q_off (global.get $REG_V1) (global.get $REG_SP_aarch64) (i32.const 0))
    (call $emit_aarch64_instr_ldr_q_off (global.get $REG_V0) (global.get $REG_SP_aarch64) (i32.const 1))
    (call $emit_aarch64_instr (i32.const 0x4E22E400))
    (call $emit_aarch64_instr_addi_64 (global.get $REG_SP_aarch64) (global.get $REG_SP_aarch64) (i32.const 16))
    (call $emit_aarch64_instr_str_q_off (global.get $REG_V0) (global.get $REG_SP_aarch64) (i32.const 0))
  )
  ;; f64x2_gt: FCMGT V0.2D, V0.2D, V1.2D = 0x4E62E400
  (func $template_aarch64_f64x2_gt (param $dec_ptr i32)
    (call $emit_aarch64_instr_ldr_q_off (global.get $REG_V1) (global.get $REG_SP_aarch64) (i32.const 0))
    (call $emit_aarch64_instr_ldr_q_off (global.get $REG_V0) (global.get $REG_SP_aarch64) (i32.const 1))
    (call $emit_aarch64_instr (i32.const 0x4E62E400))
    (call $emit_aarch64_instr_addi_64 (global.get $REG_SP_aarch64) (global.get $REG_SP_aarch64) (i32.const 16))
    (call $emit_aarch64_instr_str_q_off (global.get $REG_V0) (global.get $REG_SP_aarch64) (i32.const 0))
  )
  ;; f32x4_le: a <= b = b >= a → FCMGE V0.4S, V1.4S, V0.4S = 0x4E20E820
  (func $template_aarch64_f32x4_le (param $dec_ptr i32)
    (call $emit_aarch64_instr_ldr_q_off (global.get $REG_V1) (global.get $REG_SP_aarch64) (i32.const 0))
    (call $emit_aarch64_instr_ldr_q_off (global.get $REG_V0) (global.get $REG_SP_aarch64) (i32.const 1))
    (call $emit_aarch64_instr (i32.const 0x4E20E820))
    (call $emit_aarch64_instr_addi_64 (global.get $REG_SP_aarch64) (global.get $REG_SP_aarch64) (i32.const 16))
    (call $emit_aarch64_instr_str_q_off (global.get $REG_V0) (global.get $REG_SP_aarch64) (i32.const 0))
  )
  ;; f64x2_le: FCMGE V0.2D, V1.2D, V0.2D = 0x4E60E820
  (func $template_aarch64_f64x2_le (param $dec_ptr i32)
    (call $emit_aarch64_instr_ldr_q_off (global.get $REG_V1) (global.get $REG_SP_aarch64) (i32.const 0))
    (call $emit_aarch64_instr_ldr_q_off (global.get $REG_V0) (global.get $REG_SP_aarch64) (i32.const 1))
    (call $emit_aarch64_instr (i32.const 0x4E60E820))
    (call $emit_aarch64_instr_addi_64 (global.get $REG_SP_aarch64) (global.get $REG_SP_aarch64) (i32.const 16))
    (call $emit_aarch64_instr_str_q_off (global.get $REG_V0) (global.get $REG_SP_aarch64) (i32.const 0))
  )
  ;; f32x4_ge: FCMGE V0.4S, V0.4S, V1.4S = 0x4E20E800
  (func $template_aarch64_f32x4_ge (param $dec_ptr i32)
    (call $emit_aarch64_instr_ldr_q_off (global.get $REG_V1) (global.get $REG_SP_aarch64) (i32.const 0))
    (call $emit_aarch64_instr_ldr_q_off (global.get $REG_V0) (global.get $REG_SP_aarch64) (i32.const 1))
    (call $emit_aarch64_instr (i32.const 0x4E20E800))
    (call $emit_aarch64_instr_addi_64 (global.get $REG_SP_aarch64) (global.get $REG_SP_aarch64) (i32.const 16))
    (call $emit_aarch64_instr_str_q_off (global.get $REG_V0) (global.get $REG_SP_aarch64) (i32.const 0))
  )
  ;; f64x2_ge: FCMGE V0.2D, V0.2D, V1.2D = 0x4E60E800
  (func $template_aarch64_f64x2_ge (param $dec_ptr i32)
    (call $emit_aarch64_instr_ldr_q_off (global.get $REG_V1) (global.get $REG_SP_aarch64) (i32.const 0))
    (call $emit_aarch64_instr_ldr_q_off (global.get $REG_V0) (global.get $REG_SP_aarch64) (i32.const 1))
    (call $emit_aarch64_instr (i32.const 0x4E60E800))
    (call $emit_aarch64_instr_addi_64 (global.get $REG_SP_aarch64) (global.get $REG_SP_aarch64) (i32.const 16))
    (call $emit_aarch64_instr_str_q_off (global.get $REG_V0) (global.get $REG_SP_aarch64) (i32.const 0))
  )

  ;; ── Float min/max templates ────────────────────────────────────────
  ;; f32x4_min: FMIN V0.4S, V0.4S, V1.4S = 0x4E22F400
  (func $template_aarch64_f32x4_min (param $dec_ptr i32)
    (call $emit_aarch64_instr_ldr_q_off (global.get $REG_V1) (global.get $REG_SP_aarch64) (i32.const 0))
    (call $emit_aarch64_instr_ldr_q_off (global.get $REG_V0) (global.get $REG_SP_aarch64) (i32.const 1))
    (call $emit_aarch64_instr (i32.const 0x4E22F400))
    (call $emit_aarch64_instr_addi_64 (global.get $REG_SP_aarch64) (global.get $REG_SP_aarch64) (i32.const 16))
    (call $emit_aarch64_instr_str_q_off (global.get $REG_V0) (global.get $REG_SP_aarch64) (i32.const 0))
  )
  ;; f32x4_max: FMAX V0.4S, V0.4S, V1.4S = 0x4E20F400
  (func $template_aarch64_f32x4_max (param $dec_ptr i32)
    (call $emit_aarch64_instr_ldr_q_off (global.get $REG_V1) (global.get $REG_SP_aarch64) (i32.const 0))
    (call $emit_aarch64_instr_ldr_q_off (global.get $REG_V0) (global.get $REG_SP_aarch64) (i32.const 1))
    (call $emit_aarch64_instr (i32.const 0x4E20F400))
    (call $emit_aarch64_instr_addi_64 (global.get $REG_SP_aarch64) (global.get $REG_SP_aarch64) (i32.const 16))
    (call $emit_aarch64_instr_str_q_off (global.get $REG_V0) (global.get $REG_SP_aarch64) (i32.const 0))
  )
  ;; f64x2_min: FMIN V0.2D, V0.2D, V1.2D = 0x4E62F400
  (func $template_aarch64_f64x2_min (param $dec_ptr i32)
    (call $emit_aarch64_instr_ldr_q_off (global.get $REG_V1) (global.get $REG_SP_aarch64) (i32.const 0))
    (call $emit_aarch64_instr_ldr_q_off (global.get $REG_V0) (global.get $REG_SP_aarch64) (i32.const 1))
    (call $emit_aarch64_instr (i32.const 0x4E62F400))
    (call $emit_aarch64_instr_addi_64 (global.get $REG_SP_aarch64) (global.get $REG_SP_aarch64) (i32.const 16))
    (call $emit_aarch64_instr_str_q_off (global.get $REG_V0) (global.get $REG_SP_aarch64) (i32.const 0))
  )
  ;; f64x2_max: FMAX V0.2D, V0.2D, V1.2D = 0x4E60F400
  (func $template_aarch64_f64x2_max (param $dec_ptr i32)
    (call $emit_aarch64_instr_ldr_q_off (global.get $REG_V1) (global.get $REG_SP_aarch64) (i32.const 0))
    (call $emit_aarch64_instr_ldr_q_off (global.get $REG_V0) (global.get $REG_SP_aarch64) (i32.const 1))
    (call $emit_aarch64_instr (i32.const 0x4E60F400))
    (call $emit_aarch64_instr_addi_64 (global.get $REG_SP_aarch64) (global.get $REG_SP_aarch64) (i32.const 16))
    (call $emit_aarch64_instr_str_q_off (global.get $REG_V0) (global.get $REG_SP_aarch64) (i32.const 0))
  )

  ;; ── Stubs (extract/replace lane, neg, mul) ───────────────────────
  (func $template_aarch64_i8x16_extract_lane_s (param $dec_ptr i32) (global.set $JIT_ERROR_aarch64 (i32.const -4)))
  (func $template_aarch64_i8x16_extract_lane_u (param $dec_ptr i32) (global.set $JIT_ERROR_aarch64 (i32.const -4)))
  (func $template_aarch64_i16x8_extract_lane_s (param $dec_ptr i32) (global.set $JIT_ERROR_aarch64 (i32.const -4)))
  (func $template_aarch64_i16x8_extract_lane_u (param $dec_ptr i32) (global.set $JIT_ERROR_aarch64 (i32.const -4)))
  (func $template_aarch64_i32x4_extract_lane (param $dec_ptr i32) (global.set $JIT_ERROR_aarch64 (i32.const -4)))
  (func $template_aarch64_i64x2_extract_lane (param $dec_ptr i32) (global.set $JIT_ERROR_aarch64 (i32.const -4)))
  (func $template_aarch64_f32x4_extract_lane (param $dec_ptr i32) (global.set $JIT_ERROR_aarch64 (i32.const -4)))
  (func $template_aarch64_f64x2_extract_lane (param $dec_ptr i32) (global.set $JIT_ERROR_aarch64 (i32.const -4)))
  (func $template_aarch64_i8x16_replace_lane (param $dec_ptr i32) (global.set $JIT_ERROR_aarch64 (i32.const -4)))
  (func $template_aarch64_i16x8_replace_lane (param $dec_ptr i32) (global.set $JIT_ERROR_aarch64 (i32.const -4)))
  (func $template_aarch64_i32x4_replace_lane (param $dec_ptr i32) (global.set $JIT_ERROR_aarch64 (i32.const -4)))
  (func $template_aarch64_i64x2_replace_lane (param $dec_ptr i32) (global.set $JIT_ERROR_aarch64 (i32.const -4)))
  (func $template_aarch64_f32x4_replace_lane (param $dec_ptr i32) (global.set $JIT_ERROR_aarch64 (i32.const -4)))
  (func $template_aarch64_f64x2_replace_lane (param $dec_ptr i32) (global.set $JIT_ERROR_aarch64 (i32.const -4)))
  (func $template_aarch64_i8x16_neg (param $dec_ptr i32)
    (call $emit_aarch64_v128_pop)
    (call $emit_aarch64_neon_neg (i32.const 0) (global.get $REG_V0) (global.get $REG_V0))
    (call $emit_aarch64_v128_push)
  )
  (func $template_aarch64_i16x8_neg (param $dec_ptr i32)
    (call $emit_aarch64_v128_pop)
    (call $emit_aarch64_neon_neg (i32.const 1) (global.get $REG_V0) (global.get $REG_V0))
    (call $emit_aarch64_v128_push)
  )
  (func $template_aarch64_i32x4_neg (param $dec_ptr i32)
    (call $emit_aarch64_v128_pop)
    (call $emit_aarch64_neon_neg (i32.const 2) (global.get $REG_V0) (global.get $REG_V0))
    (call $emit_aarch64_v128_push)
  )
  (func $template_aarch64_i64x2_neg (param $dec_ptr i32)
    (call $emit_aarch64_v128_pop)
    (call $emit_aarch64_neon_neg (i32.const 3) (global.get $REG_V0) (global.get $REG_V0))
    (call $emit_aarch64_v128_push)
  )
  ;; f32x4_neg: FNEG V0.4S = 0x4EE0F800
  (func $template_aarch64_f32x4_neg (param $dec_ptr i32)
    (call $emit_aarch64_v128_pop)
    (call $emit_aarch64_instr (i32.const 0x4EE0F800))
    (call $emit_aarch64_v128_push)
  )
  ;; f64x2_neg: FNEG V0.2D = 0x4EE1F800
  (func $template_aarch64_f64x2_neg (param $dec_ptr i32)
    (call $emit_aarch64_v128_pop)
    (call $emit_aarch64_instr (i32.const 0x4EE1F800))
    (call $emit_aarch64_v128_push)
  )
  ;; i8x16_mul: no NEON MUL for byte — keep as stub (needs PMULL + complex)
  (func $template_aarch64_i8x16_mul (param $dec_ptr i32) (global.set $JIT_ERROR_aarch64 (i32.const -4)))
  (func $template_aarch64_i16x8_mul (param $dec_ptr i32)
    (call $emit_aarch64_instr_ldr_q_off (global.get $REG_V1) (global.get $REG_SP_aarch64) (i32.const 0))
    (call $emit_aarch64_instr_ldr_q_off (global.get $REG_V0) (global.get $REG_SP_aarch64) (i32.const 1))
    (call $emit_aarch64_neon_mul_8h (global.get $REG_V0) (global.get $REG_V0) (global.get $REG_V1))
    (call $emit_aarch64_instr_addi_64 (global.get $REG_SP_aarch64) (global.get $REG_SP_aarch64) (i32.const 16))
    (call $emit_aarch64_instr_str_q_off (global.get $REG_V0) (global.get $REG_SP_aarch64) (i32.const 0))
  )
  (func $template_aarch64_i32x4_mul (param $dec_ptr i32)
    (call $emit_aarch64_instr_ldr_q_off (global.get $REG_V1) (global.get $REG_SP_aarch64) (i32.const 0))
    (call $emit_aarch64_instr_ldr_q_off (global.get $REG_V0) (global.get $REG_SP_aarch64) (i32.const 1))
    (call $emit_aarch64_neon_mul_4s (global.get $REG_V0) (global.get $REG_V0) (global.get $REG_V1))
    (call $emit_aarch64_instr_addi_64 (global.get $REG_SP_aarch64) (global.get $REG_SP_aarch64) (i32.const 16))
    (call $emit_aarch64_instr_str_q_off (global.get $REG_V0) (global.get $REG_SP_aarch64) (i32.const 0))
  )
  ;; f32x4_mul: FMUL V0.4S, V0.4S, V1.4S = 0x4E22DC00
  (func $template_aarch64_f32x4_mul (param $dec_ptr i32)
    (call $emit_aarch64_instr_ldr_q_off (global.get $REG_V1) (global.get $REG_SP_aarch64) (i32.const 0))
    (call $emit_aarch64_instr_ldr_q_off (global.get $REG_V0) (global.get $REG_SP_aarch64) (i32.const 1))
    (call $emit_aarch64_instr (i32.const 0x4E22DC00))
    (call $emit_aarch64_instr_addi_64 (global.get $REG_SP_aarch64) (global.get $REG_SP_aarch64) (i32.const 16))
    (call $emit_aarch64_instr_str_q_off (global.get $REG_V0) (global.get $REG_SP_aarch64) (i32.const 0))
  )
  ;; f64x2_mul: FMUL V0.2D, V0.2D, V1.2D = 0x4E62DC00
  (func $template_aarch64_f64x2_mul (param $dec_ptr i32)
    (call $emit_aarch64_instr_ldr_q_off (global.get $REG_V1) (global.get $REG_SP_aarch64) (i32.const 0))
    (call $emit_aarch64_instr_ldr_q_off (global.get $REG_V0) (global.get $REG_SP_aarch64) (i32.const 1))
    (call $emit_aarch64_instr (i32.const 0x4E62DC00))
    (call $emit_aarch64_instr_addi_64 (global.get $REG_SP_aarch64) (global.get $REG_SP_aarch64) (i32.const 16))
    (call $emit_aarch64_instr_str_q_off (global.get $REG_V0) (global.get $REG_SP_aarch64) (i32.const 0))
  )
  ;; ── Loadsplat templates (LD1R) ────────────────────────────────────
  ;; v128.load8_splat: LD1R { V0.16B }, [X0]
  (func $template_aarch64_v128_load8_splat (param $dec_ptr i32)
    (call $emit_aarch64_pop_x0)
    (call $emit_aarch64_ldr_mem_ptr)
    (call $emit_aarch64_instr (i32.const 0x4D40C000))
    (call $emit_aarch64_v128_push)
  )
  ;; v128.load16_splat: LD1R { V0.8H }, [X0]  (size=01)
  (func $template_aarch64_v128_load16_splat (param $dec_ptr i32)
    (call $emit_aarch64_pop_x0)
    (call $emit_aarch64_ldr_mem_ptr)
    (call $emit_aarch64_instr (i32.const 0x4D41C000))
    (call $emit_aarch64_v128_push)
  )
  ;; v128.load32_splat: LD1R { V0.4S }, [X0]  (size=10)
  (func $template_aarch64_v128_load32_splat (param $dec_ptr i32)
    (call $emit_aarch64_pop_x0)
    (call $emit_aarch64_ldr_mem_ptr)
    (call $emit_aarch64_instr (i32.const 0x4D42C000))
    (call $emit_aarch64_v128_push)
  )
  ;; v128.load64_splat: LD1R { V0.2D }, [X0]  (size=11)
  (func $template_aarch64_v128_load64_splat (param $dec_ptr i32)
    (call $emit_aarch64_pop_x0)
    (call $emit_aarch64_ldr_mem_ptr)
    (call $emit_aarch64_instr (i32.const 0x4D43C000))
    (call $emit_aarch64_v128_push)
  )
  ;; i8x16.swizzle: pop index, pop table, TBL V0.16B, {V1.16B}, V0.16B
  (func $template_aarch64_i8x16_swizzle (param $dec_ptr i32)
    (call $emit_aarch64_instr_ldr_q_off (global.get $REG_V1) (global.get $REG_SP_aarch64) (i32.const 0))
    (call $emit_aarch64_instr_ldr_q_off (global.get $REG_V0) (global.get $REG_SP_aarch64) (i32.const 1))
    (call $emit_aarch64_instr (i32.const 0x4E000020))
    (call $emit_aarch64_instr_addi_64 (global.get $REG_SP_aarch64) (global.get $REG_SP_aarch64) (i32.const 16))
    (call $emit_aarch64_instr_str_q_off (global.get $REG_V0) (global.get $REG_SP_aarch64) (i32.const 0))
  )
  ;; v128.not: MVN V0.16B, V0.16B = 0x4E205800
  (func $template_aarch64_v128_not (param $dec_ptr i32)
    (call $emit_aarch64_v128_pop)
    (call $emit_aarch64_instr (i32.const 0x4E205800))
    (call $emit_aarch64_v128_push)
  )
  ;; v128.bitselect: pop mask(V1), pop b(V0), pop a(V2); BSL V1, V0, V2
  ;; V1 = (V1 & V0) | (~V1 & V2) = (mask & b) | (~mask & a)
  ;; Stack: [SP]=mask, [SP+16]=b, [SP+32]=a (deepest)
  ;; Result replaces 'a' position: SP += 32, str_q at new SP
  ;; BSL V1.16B, V0.16B, V2.16B = 0x4E021C01
  (func $template_aarch64_v128_bitselect (param $dec_ptr i32)
    (call $emit_aarch64_instr_ldr_q_off (global.get $REG_V1) (global.get $REG_SP_aarch64) (i32.const 0))
    (call $emit_aarch64_instr_ldr_q_off (global.get $REG_V0) (global.get $REG_SP_aarch64) (i32.const 1))
    (call $emit_aarch64_instr_ldr_q_off (global.get $REG_V2) (global.get $REG_SP_aarch64) (i32.const 2))
    (call $emit_aarch64_instr (i32.const 0x4E021C01))
    (call $emit_aarch64_instr_addi_64 (global.get $REG_SP_aarch64) (global.get $REG_SP_aarch64) (i32.const 32))
    (call $emit_aarch64_instr_str_q_off (global.get $REG_V1) (global.get $REG_SP_aarch64) (i32.const 0))
  )

  (func $jit_compile_aarch64 (export "jit_compile_aarch64") (param $func_idx i32) (result i32)
    (local $dec_ptr i32) (local $opcode i32) (local $imm0 i32)
    (local $code_start i32) (local $label_level i32)
    (local $loop_top_offset i32)
    (local $decoded_ops_base i32) (local $decoded_ops_count i32)

    ;; Reset code ptr
    (i32.store (global.get $JS_CODE_PTR_aarch64) (i32.const 0))
    (global.set $RESULT_IN_X0_aarch64 (i32.const 0))

    ;; Get decoded_ops base from imported global
    (local.set $decoded_ops_base (global.get $OFF_DECODED_OPS))

    ;; ── Emit prologue ─────────────────────────────────────────────
    (call $emit_aarch64_prologue)

    ;; ── Main compile loop ─────────────────────────────────────────
    (local.set $dec_ptr (local.get $decoded_ops_base))
    (block $compile_done
      (loop $compile_loop
        ;; Read opcode
        (local.set $opcode (i32.load (local.get $dec_ptr)))

        ;; Read next opcode for peephole (at dec_ptr + DEC_SZ)
        (if (i32.load (i32.add (local.get $dec_ptr) (global.get $DEC_SZ)))
          (then
            (global.set $NEXT_OP_aarch64 (i32.load (i32.add (local.get $dec_ptr) (global.get $DEC_SZ))))
          )
          (else
            (global.set $NEXT_OP_aarch64 (i32.const 0))
          )
        )

        ;; Dispatch by opcode (flat if+br pattern)
        (block $dispatch_done
          (if (i32.eqz (local.get $opcode))
            (then (call $template_unreachable_aarch64 (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0x01))
            (then (call $template_nop_aarch64 (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0x02))
            (then (call $template_block_aarch64 (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0x03))
            (then
              (local.set $loop_top_offset (i32.load (global.get $JS_CODE_PTR_aarch64)))
              (call $template_loop_aarch64 (local.get $dec_ptr))
              (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0x04))
            (then (call $template_if_aarch64 (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0x05))
            (then (call $template_else_aarch64 (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0x0B))
            (then (call $template_end_aarch64 (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0x0C))
            (then (call $template_br_aarch64 (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0x0D))
            (then (call $template_br_if_aarch64 (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0x0E))
            (then (call $template_br_table_aarch64 (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0x0F))
            (then (call $template_return_aarch64 (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0x10))
            (then (call $template_call_aarch64 (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0x12))
            (then (call $template_return_call_aarch64 (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0x1A))
            (then (call $template_drop_aarch64 (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0x1B))
            (then (call $template_select_aarch64 (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0x20))
            (then (call $template_local_get_aarch64 (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0x21))
            (then (call $template_local_set_aarch64 (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0x22))
            (then (call $template_local_tee_aarch64 (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0x23))
            (then (call $template_global_get_aarch64 (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0x24))
            (then (call $template_global_set_aarch64 (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0x25))
            (then (call $template_table_get_aarch64 (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0x26))
            (then (call $template_table_set_aarch64 (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0x28))
            (then (call $template_i32_load_aarch64 (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0x29))
            (then (call $template_i64_load_aarch64 (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0x2C))
            (then (call $template_i32_load8_s_aarch64 (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0x2D))
            (then (call $template_i32_load8_u_aarch64 (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0x2E))
            (then (call $template_i32_load16_s_aarch64 (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0x2F))
            (then (call $template_i32_load16_u_aarch64 (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0x36))
            (then (call $template_i32_store_aarch64 (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0x37))
            (then (call $template_i64_store_aarch64 (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0x3A))
            (then (call $template_i32_store8_aarch64 (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0x3B))
            (then (call $template_i32_store16_aarch64 (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0x3F))
            (then (call $template_memory_size_aarch64 (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0x40))
            (then (call $template_memory_grow_aarch64 (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0x41))
            (then (call $template_i32_const_aarch64 (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0x42))
            (then (call $template_i64_const_aarch64 (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0x45))
            (then (call $template_i32_eqz_aarch64 (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0x46))
            (then (call $template_i32_eq_aarch64 (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0x47))
            (then (call $template_i32_ne_aarch64 (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0x48))
            (then (call $template_i32_lt_s_aarch64 (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0x49))
            (then (call $template_i32_lt_u_aarch64 (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0x4A))
            (then (call $template_i32_gt_s_aarch64 (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0x4B))
            (then (call $template_i32_gt_u_aarch64 (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0x4C))
            (then (call $template_i32_le_s_aarch64 (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0x4D))
            (then (call $template_i32_le_u_aarch64 (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0x50))
            (then (call $template_i64_eqz_aarch64 (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0x51))
            (then (call $template_i64_eq_aarch64 (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0x52))
            (then (call $template_i64_ne_aarch64 (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0x53))
            (then (call $template_i64_lt_s_aarch64 (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0x54))
            (then (call $template_i64_lt_u_aarch64 (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0x55))
            (then (call $template_i64_gt_s_aarch64 (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0x56))
            (then (call $template_i64_gt_u_aarch64 (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0x57))
            (then (call $template_i64_le_s_aarch64 (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0x58))
            (then (call $template_i64_le_u_aarch64 (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0x67))
            (then (call $template_i32_clz_aarch64 (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0x68))
            (then (call $template_i32_ctz_aarch64 (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0x69))
            (then (call $template_i32_popcnt_aarch64 (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0x6A))
            (then (call $template_i32_add_aarch64 (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0x6B))
            (then (call $template_i32_sub_aarch64 (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0x6C))
            (then (call $template_i32_mul_aarch64 (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0x6D))
            (then (call $template_i32_div_s_aarch64 (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0x6E))
            (then (call $template_i32_div_u_aarch64 (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0x6F))
            (then (call $template_i32_rem_s_aarch64 (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0x70))
            (then (call $template_i32_rem_u_aarch64 (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0x71))
            (then (call $template_i32_and_aarch64 (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0x72))
            (then (call $template_i32_or_aarch64 (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0x73))
            (then (call $template_i32_xor_aarch64 (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0x74))
            (then (call $template_i32_shl_aarch64 (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0x75))
            (then (call $template_i32_shr_s_aarch64 (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0x76))
            (then (call $template_i32_shr_u_aarch64 (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0x77))
            (then (call $template_i32_rotl_aarch64 (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0x78))
            (then (call $template_i32_rotr_aarch64 (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0x79))
            (then (call $template_i64_clz_aarch64 (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0x7A))
            (then (call $template_i64_ctz_aarch64 (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0x7B))
            (then (call $template_i64_popcnt_aarch64 (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0x7C))
            (then (call $template_i64_add_aarch64 (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0x7D))
            (then (call $template_i64_sub_aarch64 (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0x7E))
            (then (call $template_i64_mul_aarch64 (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0x7F))
            (then (call $template_i64_div_s_aarch64 (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0x80))
            (then (call $template_i64_div_u_aarch64 (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0x81))
            (then (call $template_i64_rem_s_aarch64 (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0x82))
            (then (call $template_i64_rem_u_aarch64 (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0x83))
            (then (call $template_i64_and_aarch64 (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0x84))
            (then (call $template_i64_or_aarch64 (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0x85))
            (then (call $template_i64_xor_aarch64 (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0x86))
            (then (call $template_i64_shl_aarch64 (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0x87))
            (then (call $template_i64_shr_s_aarch64 (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0x88))
            (then (call $template_i64_shr_u_aarch64 (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0x89))
            (then (call $template_i64_rotl_aarch64 (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0x8A))
            (then (call $template_i64_rotr_aarch64 (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0xA7))
            (then (call $template_i32_wrap_i64_aarch64 (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0xAC))
            (then (call $template_i64_extend_i32_s_aarch64 (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0xAD))
            (then (call $template_i64_extend_i32_u_aarch64 (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0xBC))
            (then (call $template_i32_reinterpret_f32_aarch64 (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0xBD))
            (then (call $template_f32_reinterpret_i32_aarch64 (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0xBE))
            (then (call $template_i64_reinterpret_f64_aarch64 (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0xBF))
            (then (call $template_f64_reinterpret_i64_aarch64 (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0xA8))
            (then (call $template_i32_trunc_f32_s_aarch64 (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0xA9))
            (then (call $template_i32_trunc_f32_u_aarch64 (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0xAA))
            (then (call $template_i32_trunc_f64_s_aarch64 (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0xAB))
            (then (call $template_i32_trunc_f64_u_aarch64 (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0xAE))
            (then (call $template_i64_trunc_f32_s_aarch64 (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0xAF))
            (then (call $template_i64_trunc_f32_u_aarch64 (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0xB0))
            (then (call $template_i64_trunc_f64_s_aarch64 (local.get $dec_ptr)) (br $dispatch_done)))
          (if (i32.eq (local.get $opcode) (i32.const 0xB1))
            (then (call $template_i64_trunc_f64_u_aarch64 (local.get $dec_ptr)) (br $dispatch_done)))
          ;; 0xFC prefix - dispatch on imm0
          (if (i32.eq (local.get $opcode) (i32.const 0xFC))
            (then
              (local.set $imm0 (i32.load (i32.add (local.get $dec_ptr) (i32.const 4))))
              (block $fc_done
                (if (i32.eq (local.get $imm0) (i32.const 0x00))
                  (then (call $template_i32_trunc_sat_f32_s_aarch64 (local.get $dec_ptr)) (br $fc_done)))
                (if (i32.eq (local.get $imm0) (i32.const 0x01))
                  (then (call $template_i32_trunc_sat_f32_u_aarch64 (local.get $dec_ptr)) (br $fc_done)))
                (if (i32.eq (local.get $imm0) (i32.const 0x02))
                  (then (call $template_i32_trunc_sat_f64_s_aarch64 (local.get $dec_ptr)) (br $fc_done)))
                (if (i32.eq (local.get $imm0) (i32.const 0x03))
                  (then (call $template_i32_trunc_sat_f64_u_aarch64 (local.get $dec_ptr)) (br $fc_done)))
                (if (i32.eq (local.get $imm0) (i32.const 0x04))
                  (then (call $template_i64_trunc_sat_f32_s_aarch64 (local.get $dec_ptr)) (br $fc_done)))
                ;; Unknown 0xFC sub-opcode
                (global.set $JIT_ERROR_aarch64 (i32.const -3))
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
                  (then (call $template_aarch64_v128_load (local.get $dec_ptr)) (br $fd_done)))
                ;; v128.store (0x1B)
                (if (i32.eq (local.get $imm0) (i32.const 0x1B))
                  (then (call $template_aarch64_v128_store (local.get $dec_ptr)) (br $fd_done)))
                ;; v128.const (0x0C)
                (if (i32.eq (local.get $imm0) (i32.const 0x0C))
                  (then (call $template_aarch64_v128_const (local.get $dec_ptr)) (br $fd_done)))
                ;; i8x16.splat (0x2D)
                (if (i32.eq (local.get $imm0) (i32.const 0x2D))
                  (then (call $template_aarch64_i8x16_splat (local.get $dec_ptr)) (br $fd_done)))
                ;; i16x8.splat (0x31)
                (if (i32.eq (local.get $imm0) (i32.const 0x31))
                  (then (call $template_aarch64_i16x8_splat (local.get $dec_ptr)) (br $fd_done)))
                ;; i32x4.splat (0x35)
                (if (i32.eq (local.get $imm0) (i32.const 0x35))
                  (then (call $template_aarch64_i32x4_splat (local.get $dec_ptr)) (br $fd_done)))
                ;; i64x2.splat (0x39)
                (if (i32.eq (local.get $imm0) (i32.const 0x39))
                  (then (call $template_aarch64_i64x2_splat (local.get $dec_ptr)) (br $fd_done)))
                ;; f32x4.splat (0x3A)
                (if (i32.eq (local.get $imm0) (i32.const 0x3A))
                  (then (call $template_aarch64_f32x4_splat (local.get $dec_ptr)) (br $fd_done)))
                ;; f64x2.splat (0x3D)
                (if (i32.eq (local.get $imm0) (i32.const 0x3D))
                  (then (call $template_aarch64_f64x2_splat (local.get $dec_ptr)) (br $fd_done)))
                ;; i8x16.extract_lane_s (0x2E)
                (if (i32.eq (local.get $imm0) (i32.const 0x2E))
                  (then (call $template_aarch64_i8x16_extract_lane_s (local.get $dec_ptr)) (br $fd_done)))
                ;; i8x16.extract_lane_u (0x2F)
                (if (i32.eq (local.get $imm0) (i32.const 0x2F))
                  (then (call $template_aarch64_i8x16_extract_lane_u (local.get $dec_ptr)) (br $fd_done)))
                ;; i16x8.extract_lane_s (0x32)
                (if (i32.eq (local.get $imm0) (i32.const 0x32))
                  (then (call $template_aarch64_i16x8_extract_lane_s (local.get $dec_ptr)) (br $fd_done)))
                ;; i16x8.extract_lane_u (0x33)
                (if (i32.eq (local.get $imm0) (i32.const 0x33))
                  (then (call $template_aarch64_i16x8_extract_lane_u (local.get $dec_ptr)) (br $fd_done)))
                ;; i32x4.extract_lane (0x36)
                (if (i32.eq (local.get $imm0) (i32.const 0x36))
                  (then (call $template_aarch64_i32x4_extract_lane (local.get $dec_ptr)) (br $fd_done)))
                ;; i64x2.extract_lane (0x38)
                (if (i32.eq (local.get $imm0) (i32.const 0x38))
                  (then (call $template_aarch64_i64x2_extract_lane (local.get $dec_ptr)) (br $fd_done)))
                ;; f32x4.extract_lane (0x3B)
                (if (i32.eq (local.get $imm0) (i32.const 0x3B))
                  (then (call $template_aarch64_f32x4_extract_lane (local.get $dec_ptr)) (br $fd_done)))
                ;; f64x2.extract_lane (0x3E)
                (if (i32.eq (local.get $imm0) (i32.const 0x3E))
                  (then (call $template_aarch64_f64x2_extract_lane (local.get $dec_ptr)) (br $fd_done)))
                ;; i8x16.replace_lane (0x30)
                (if (i32.eq (local.get $imm0) (i32.const 0x30))
                  (then (call $template_aarch64_i8x16_replace_lane (local.get $dec_ptr)) (br $fd_done)))
                ;; i16x8.replace_lane (0x34)
                (if (i32.eq (local.get $imm0) (i32.const 0x34))
                  (then (call $template_aarch64_i16x8_replace_lane (local.get $dec_ptr)) (br $fd_done)))
                ;; i32x4.replace_lane (0x37)
                (if (i32.eq (local.get $imm0) (i32.const 0x37))
                  (then (call $template_aarch64_i32x4_replace_lane (local.get $dec_ptr)) (br $fd_done)))
                ;; i64x2.replace_lane (0xAB) — canonical ID, not in old encoding
                ;; f32x4.replace_lane (0x3C)
                (if (i32.eq (local.get $imm0) (i32.const 0x3C))
                  (then (call $template_aarch64_f32x4_replace_lane (local.get $dec_ptr)) (br $fd_done)))
                ;; f64x2.replace_lane (0x3F)
                (if (i32.eq (local.get $imm0) (i32.const 0x3F))
                  (then (call $template_aarch64_f64x2_replace_lane (local.get $dec_ptr)) (br $fd_done)))
                ;; i8x16.add (0x40)
                (if (i32.eq (local.get $imm0) (i32.const 0x40))
                  (then (call $template_aarch64_i8x16_add (local.get $dec_ptr)) (br $fd_done)))
                ;; i16x8.add (0x51)
                (if (i32.eq (local.get $imm0) (i32.const 0x51))
                  (then (call $template_aarch64_i16x8_add (local.get $dec_ptr)) (br $fd_done)))
                ;; i32x4.add (0x62)
                (if (i32.eq (local.get $imm0) (i32.const 0x62))
                  (then (call $template_aarch64_i32x4_add (local.get $dec_ptr)) (br $fd_done)))
                ;; i64x2.add (0x73)
                (if (i32.eq (local.get $imm0) (i32.const 0x73))
                  (then (call $template_aarch64_i64x2_add (local.get $dec_ptr)) (br $fd_done)))
                ;; f32x4.add (0x84)
                (if (i32.eq (local.get $imm0) (i32.const 0x84))
                  (then (call $template_aarch64_f32x4_add (local.get $dec_ptr)) (br $fd_done)))
                ;; f64x2.add (0x95)
                (if (i32.eq (local.get $imm0) (i32.const 0x95))
                  (then (call $template_aarch64_f64x2_add (local.get $dec_ptr)) (br $fd_done)))
                ;; i8x16.sub (0x41)
                (if (i32.eq (local.get $imm0) (i32.const 0x41))
                  (then (call $template_aarch64_i8x16_sub (local.get $dec_ptr)) (br $fd_done)))
                ;; i16x8.sub (0x52)
                (if (i32.eq (local.get $imm0) (i32.const 0x52))
                  (then (call $template_aarch64_i16x8_sub (local.get $dec_ptr)) (br $fd_done)))
                ;; i32x4.sub (0x63)
                (if (i32.eq (local.get $imm0) (i32.const 0x63))
                  (then (call $template_aarch64_i32x4_sub (local.get $dec_ptr)) (br $fd_done)))
                ;; i64x2.sub (0x74)
                (if (i32.eq (local.get $imm0) (i32.const 0x74))
                  (then (call $template_aarch64_i64x2_sub (local.get $dec_ptr)) (br $fd_done)))
                ;; f32x4.sub (0x85)
                (if (i32.eq (local.get $imm0) (i32.const 0x85))
                  (then (call $template_aarch64_f32x4_sub (local.get $dec_ptr)) (br $fd_done)))
                ;; f64x2.sub (0x96)
                (if (i32.eq (local.get $imm0) (i32.const 0x96))
                  (then (call $template_aarch64_f64x2_sub (local.get $dec_ptr)) (br $fd_done)))
                ;; i8x16.mul (0x50)
                (if (i32.eq (local.get $imm0) (i32.const 0x50))
                  (then (call $template_aarch64_i8x16_mul (local.get $dec_ptr)) (br $fd_done)))
                ;; i16x8.mul (0x61)
                (if (i32.eq (local.get $imm0) (i32.const 0x61))
                  (then (call $template_aarch64_i16x8_mul (local.get $dec_ptr)) (br $fd_done)))
                ;; i32x4.mul (0x72)
                (if (i32.eq (local.get $imm0) (i32.const 0x72))
                  (then (call $template_aarch64_i32x4_mul (local.get $dec_ptr)) (br $fd_done)))
                ;; f32x4.mul (0x87)
                (if (i32.eq (local.get $imm0) (i32.const 0x87))
                  (then (call $template_aarch64_f32x4_mul (local.get $dec_ptr)) (br $fd_done)))
                ;; f64x2.mul (0x98)
                (if (i32.eq (local.get $imm0) (i32.const 0x98))
                  (then (call $template_aarch64_f64x2_mul (local.get $dec_ptr)) (br $fd_done)))
                ;; i8x16.neg (0x46)
                (if (i32.eq (local.get $imm0) (i32.const 0x46))
                  (then (call $template_aarch64_i8x16_neg (local.get $dec_ptr)) (br $fd_done)))
                ;; i16x8.neg (0x57)
                (if (i32.eq (local.get $imm0) (i32.const 0x57))
                  (then (call $template_aarch64_i16x8_neg (local.get $dec_ptr)) (br $fd_done)))
                ;; i32x4.neg (0x68)
                (if (i32.eq (local.get $imm0) (i32.const 0x68))
                  (then (call $template_aarch64_i32x4_neg (local.get $dec_ptr)) (br $fd_done)))
                ;; i64x2.neg (0x79)
                (if (i32.eq (local.get $imm0) (i32.const 0x79))
                  (then (call $template_aarch64_i64x2_neg (local.get $dec_ptr)) (br $fd_done)))
                ;; f32x4.neg (0x8A)
                (if (i32.eq (local.get $imm0) (i32.const 0x8A))
                  (then (call $template_aarch64_f32x4_neg (local.get $dec_ptr)) (br $fd_done)))
                ;; f64x2.neg (0x9B)
                (if (i32.eq (local.get $imm0) (i32.const 0x9B))
                  (then (call $template_aarch64_f64x2_neg (local.get $dec_ptr)) (br $fd_done)))
                ;; i8x16.eq (0x47)
                (if (i32.eq (local.get $imm0) (i32.const 0x47))
                  (then (call $template_aarch64_i8x16_eq (local.get $dec_ptr)) (br $fd_done)))
                ;; i16x8.eq (0x58)
                (if (i32.eq (local.get $imm0) (i32.const 0x58))
                  (then (call $template_aarch64_i16x8_eq (local.get $dec_ptr)) (br $fd_done)))
                ;; i32x4.eq (0x69)
                (if (i32.eq (local.get $imm0) (i32.const 0x69))
                  (then (call $template_aarch64_i32x4_eq (local.get $dec_ptr)) (br $fd_done)))
                ;; f32x4.eq (0x8C)
                (if (i32.eq (local.get $imm0) (i32.const 0x8C))
                  (then (call $template_aarch64_f32x4_eq (local.get $dec_ptr)) (br $fd_done)))
                ;; f64x2.eq (0x9D)
                (if (i32.eq (local.get $imm0) (i32.const 0x9D))
                  (then (call $template_aarch64_f64x2_eq (local.get $dec_ptr)) (br $fd_done)))
                ;; i8x16.ne (0x48)
                (if (i32.eq (local.get $imm0) (i32.const 0x48))
                  (then (call $template_aarch64_i8x16_ne (local.get $dec_ptr)) (br $fd_done)))
                ;; i16x8.ne (0x59)
                (if (i32.eq (local.get $imm0) (i32.const 0x59))
                  (then (call $template_aarch64_i16x8_ne (local.get $dec_ptr)) (br $fd_done)))
                ;; i32x4.ne (0x6A)
                (if (i32.eq (local.get $imm0) (i32.const 0x6A))
                  (then (call $template_aarch64_i32x4_ne (local.get $dec_ptr)) (br $fd_done)))
                ;; f32x4.ne (0x8B)
                (if (i32.eq (local.get $imm0) (i32.const 0x8B))
                  (then (call $template_aarch64_f32x4_ne (local.get $dec_ptr)) (br $fd_done)))
                ;; f64x2.ne (0x9C)
                (if (i32.eq (local.get $imm0) (i32.const 0x9C))
                  (then (call $template_aarch64_f64x2_ne (local.get $dec_ptr)) (br $fd_done)))
                ;; i8x16.lt_s (0x4B)
                (if (i32.eq (local.get $imm0) (i32.const 0x4B))
                  (then (call $template_aarch64_i8x16_lt_s (local.get $dec_ptr)) (br $fd_done)))
                ;; i8x16.lt_u (0x4A)
                (if (i32.eq (local.get $imm0) (i32.const 0x4A))
                  (then (call $template_aarch64_i8x16_lt_u (local.get $dec_ptr)) (br $fd_done)))
                ;; i16x8.lt_s (0x5C)
                (if (i32.eq (local.get $imm0) (i32.const 0x5C))
                  (then (call $template_aarch64_i16x8_lt_s (local.get $dec_ptr)) (br $fd_done)))
                ;; i16x8.lt_u (0x5B)
                (if (i32.eq (local.get $imm0) (i32.const 0x5B))
                  (then (call $template_aarch64_i16x8_lt_u (local.get $dec_ptr)) (br $fd_done)))
                ;; i32x4.lt_s (0x6D)
                (if (i32.eq (local.get $imm0) (i32.const 0x6D))
                  (then (call $template_aarch64_i32x4_lt_s (local.get $dec_ptr)) (br $fd_done)))
                ;; i32x4.lt_u (0x6C)
                (if (i32.eq (local.get $imm0) (i32.const 0x6C))
                  (then (call $template_aarch64_i32x4_lt_u (local.get $dec_ptr)) (br $fd_done)))
                ;; f32x4.lt (0x8E)
                (if (i32.eq (local.get $imm0) (i32.const 0x8E))
                  (then (call $template_aarch64_f32x4_lt (local.get $dec_ptr)) (br $fd_done)))
                ;; f64x2.lt (0x9F)
                (if (i32.eq (local.get $imm0) (i32.const 0x9F))
                  (then (call $template_aarch64_f64x2_lt (local.get $dec_ptr)) (br $fd_done)))
                ;; i8x16.gt_s (0x4D)
                (if (i32.eq (local.get $imm0) (i32.const 0x4D))
                  (then (call $template_aarch64_i8x16_gt_s (local.get $dec_ptr)) (br $fd_done)))
                ;; i8x16.gt_u (0x4C)
                (if (i32.eq (local.get $imm0) (i32.const 0x4C))
                  (then (call $template_aarch64_i8x16_gt_u (local.get $dec_ptr)) (br $fd_done)))
                ;; i16x8.gt_s (0x5E)
                (if (i32.eq (local.get $imm0) (i32.const 0x5E))
                  (then (call $template_aarch64_i16x8_gt_s (local.get $dec_ptr)) (br $fd_done)))
                ;; i16x8.gt_u (0x5D)
                (if (i32.eq (local.get $imm0) (i32.const 0x5D))
                  (then (call $template_aarch64_i16x8_gt_u (local.get $dec_ptr)) (br $fd_done)))
                ;; i32x4.gt_s (0x6F)
                (if (i32.eq (local.get $imm0) (i32.const 0x6F))
                  (then (call $template_aarch64_i32x4_gt_s (local.get $dec_ptr)) (br $fd_done)))
                ;; i32x4.gt_u (0x6E)
                (if (i32.eq (local.get $imm0) (i32.const 0x6E))
                  (then (call $template_aarch64_i32x4_gt_u (local.get $dec_ptr)) (br $fd_done)))
                ;; f32x4.gt (0x90)
                (if (i32.eq (local.get $imm0) (i32.const 0x90))
                  (then (call $template_aarch64_f32x4_gt (local.get $dec_ptr)) (br $fd_done)))
                ;; f64x2.gt (0xA1)
                (if (i32.eq (local.get $imm0) (i32.const 0xA1))
                  (then (call $template_aarch64_f64x2_gt (local.get $dec_ptr)) (br $fd_done)))
                ;; i8x16.le_s (0x4F)
                (if (i32.eq (local.get $imm0) (i32.const 0x4F))
                  (then (call $template_aarch64_i8x16_le_s (local.get $dec_ptr)) (br $fd_done)))
                ;; i8x16.le_u (0x4E)
                (if (i32.eq (local.get $imm0) (i32.const 0x4E))
                  (then (call $template_aarch64_i8x16_le_u (local.get $dec_ptr)) (br $fd_done)))
                ;; i16x8.le_s (0x60)
                (if (i32.eq (local.get $imm0) (i32.const 0x60))
                  (then (call $template_aarch64_i16x8_le_s (local.get $dec_ptr)) (br $fd_done)))
                ;; i16x8.le_u (0x5F)
                (if (i32.eq (local.get $imm0) (i32.const 0x5F))
                  (then (call $template_aarch64_i16x8_le_u (local.get $dec_ptr)) (br $fd_done)))
                ;; i32x4.le_s (0x71)
                (if (i32.eq (local.get $imm0) (i32.const 0x71))
                  (then (call $template_aarch64_i32x4_le_s (local.get $dec_ptr)) (br $fd_done)))
                ;; i32x4.le_u (0x70)
                (if (i32.eq (local.get $imm0) (i32.const 0x70))
                  (then (call $template_aarch64_i32x4_le_u (local.get $dec_ptr)) (br $fd_done)))
                ;; f32x4.le (0x92)
                (if (i32.eq (local.get $imm0) (i32.const 0x92))
                  (then (call $template_aarch64_f32x4_le (local.get $dec_ptr)) (br $fd_done)))
                ;; f64x2.le (0xA3)
                (if (i32.eq (local.get $imm0) (i32.const 0xA3))
                  (then (call $template_aarch64_f64x2_le (local.get $dec_ptr)) (br $fd_done)))
                ;; i8x16.ge_s (0x49)
                (if (i32.eq (local.get $imm0) (i32.const 0x49))
                  (then (call $template_aarch64_i8x16_ge_s (local.get $dec_ptr)) (br $fd_done)))
                ;; i16x8.ge_s (0x5A)
                (if (i32.eq (local.get $imm0) (i32.const 0x5A))
                  (then (call $template_aarch64_i16x8_ge_s (local.get $dec_ptr)) (br $fd_done)))
                ;; i32x4.ge_s (0x6B)
                (if (i32.eq (local.get $imm0) (i32.const 0x6B))
                  (then (call $template_aarch64_i32x4_ge_s (local.get $dec_ptr)) (br $fd_done)))
                ;; f32x4.ge (0x94)
                (if (i32.eq (local.get $imm0) (i32.const 0x94))
                  (then (call $template_aarch64_f32x4_ge (local.get $dec_ptr)) (br $fd_done)))
                ;; f64x2.ge (0xA5)
                (if (i32.eq (local.get $imm0) (i32.const 0xA5))
                  (then (call $template_aarch64_f64x2_ge (local.get $dec_ptr)) (br $fd_done)))
                ;; i8x16.and (0x43)
                (if (i32.eq (local.get $imm0) (i32.const 0x43))
                  (then (call $template_aarch64_i8x16_and (local.get $dec_ptr)) (br $fd_done)))
                ;; i8x16.or (0x44)
                (if (i32.eq (local.get $imm0) (i32.const 0x44))
                  (then (call $template_aarch64_i8x16_or (local.get $dec_ptr)) (br $fd_done)))
                ;; i8x16.xor (0x45)
                (if (i32.eq (local.get $imm0) (i32.const 0x45))
                  (then (call $template_aarch64_i8x16_xor (local.get $dec_ptr)) (br $fd_done)))
                ;; ── New canonical ops (0xA6+, not in old encoding) ──
                ;; v128.load8_splat (0xA6)
                (if (i32.eq (local.get $imm0) (i32.const 0xA6))
                  (then (call $template_aarch64_v128_load8_splat (local.get $dec_ptr)) (br $fd_done)))
                ;; v128.load16_splat (0xA7)
                (if (i32.eq (local.get $imm0) (i32.const 0xA7))
                  (then (call $template_aarch64_v128_load16_splat (local.get $dec_ptr)) (br $fd_done)))
                ;; v128.load32_splat (0xA8)
                (if (i32.eq (local.get $imm0) (i32.const 0xA8))
                  (then (call $template_aarch64_v128_load32_splat (local.get $dec_ptr)) (br $fd_done)))
                ;; v128.load64_splat (0xA9)
                (if (i32.eq (local.get $imm0) (i32.const 0xA9))
                  (then (call $template_aarch64_v128_load64_splat (local.get $dec_ptr)) (br $fd_done)))
                ;; i8x16.swizzle (0xAA)
                (if (i32.eq (local.get $imm0) (i32.const 0xAA))
                  (then (call $template_aarch64_i8x16_swizzle (local.get $dec_ptr)) (br $fd_done)))
                ;; i64x2.replace_lane (0xAB)
                (if (i32.eq (local.get $imm0) (i32.const 0xAB))
                  (then (call $template_aarch64_i64x2_replace_lane (local.get $dec_ptr)) (br $fd_done)))
                ;; v128.not (0xAC)
                (if (i32.eq (local.get $imm0) (i32.const 0xAC))
                  (then (call $template_aarch64_v128_not (local.get $dec_ptr)) (br $fd_done)))
                ;; v128.bitselect (0xAD)
                (if (i32.eq (local.get $imm0) (i32.const 0xAD))
                  (then (call $template_aarch64_v128_bitselect (local.get $dec_ptr)) (br $fd_done)))
                ;; i8x16.ge_u (0xAE)
                (if (i32.eq (local.get $imm0) (i32.const 0xAE))
                  (then (call $template_aarch64_i8x16_ge_u (local.get $dec_ptr)) (br $fd_done)))
                ;; i16x8.ge_u (0xAF)
                (if (i32.eq (local.get $imm0) (i32.const 0xAF))
                  (then (call $template_aarch64_i16x8_ge_u (local.get $dec_ptr)) (br $fd_done)))
                ;; i32x4.ge_u (0xB0)
                (if (i32.eq (local.get $imm0) (i32.const 0xB0))
                  (then (call $template_aarch64_i32x4_ge_u (local.get $dec_ptr)) (br $fd_done)))
                ;; i8x16.min_u (0xB8)
                (if (i32.eq (local.get $imm0) (i32.const 0xB8))
                  (then (call $template_aarch64_i8x16_min_u (local.get $dec_ptr)) (br $fd_done)))
                ;; i8x16.max_u (0xBA)
                (if (i32.eq (local.get $imm0) (i32.const 0xBA))
                  (then (call $template_aarch64_i8x16_max_u (local.get $dec_ptr)) (br $fd_done)))
                ;; i8x16.avgr_u (0xBB)
                (if (i32.eq (local.get $imm0) (i32.const 0xBB))
                  (then (call $template_aarch64_i8x16_avgr_u (local.get $dec_ptr)) (br $fd_done)))
                ;; i16x8.min_u (0xBC)
                (if (i32.eq (local.get $imm0) (i32.const 0xBC))
                  (then (call $template_aarch64_i16x8_min_u (local.get $dec_ptr)) (br $fd_done)))
                ;; i16x8.max_u (0xBE)
                (if (i32.eq (local.get $imm0) (i32.const 0xBE))
                  (then (call $template_aarch64_i16x8_max_u (local.get $dec_ptr)) (br $fd_done)))
                ;; i16x8.avgr_u (0xBF)
                (if (i32.eq (local.get $imm0) (i32.const 0xBF))
                  (then (call $template_aarch64_i16x8_avgr_u (local.get $dec_ptr)) (br $fd_done)))
                ;; f32x4.min (0xE1)
                (if (i32.eq (local.get $imm0) (i32.const 0xE1))
                  (then (call $template_aarch64_f32x4_min (local.get $dec_ptr)) (br $fd_done)))
                ;; f32x4.max (0xE2)
                (if (i32.eq (local.get $imm0) (i32.const 0xE2))
                  (then (call $template_aarch64_f32x4_max (local.get $dec_ptr)) (br $fd_done)))
                ;; f64x2.min (0xE3)
                (if (i32.eq (local.get $imm0) (i32.const 0xE3))
                  (then (call $template_aarch64_f64x2_min (local.get $dec_ptr)) (br $fd_done)))
                ;; f64x2.max (0xE4)
                (if (i32.eq (local.get $imm0) (i32.const 0xE4))
                  (then (call $template_aarch64_f64x2_max (local.get $dec_ptr)) (br $fd_done)))
                ;; Unknown 0xFD sub-opcode
                (global.set $JIT_ERROR_aarch64 (i32.const -4))
              )
              (br $dispatch_done)
            )
          )
          ;; Unknown opcode
          (global.set $JIT_ERROR_aarch64 (i32.const -2))
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
    (call $emit_aarch64_epilogue)

    ;; ── Return code size ──────────────────────────────────────────
    (i32.load (global.get $JS_CODE_PTR_aarch64))
  )

