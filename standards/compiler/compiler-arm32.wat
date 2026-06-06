  ;; ═════════════════════════════════════════════════════════════════════
  ;; EdgeRun WASM → ARM32 (ARMv7-A) JIT Compiler
  ;;
  ;; Compiles WASM decoded ops into ARM32 machine code in a memory buffer.
  ;; Follows the same architecture as the x86_64 JIT (compiler.wat).
  ;; ARM32 instructions are fixed 4 bytes, little-endian.
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




  ;; ── ARM32 instruction emitter ─────────────────────────────────────
  ;; All ARM instructions are exactly 4 bytes (32 bits)


  ;; ── ARM32 Data Processing (ALU) helpers ───────────────────────────
  ;; Base: cond=AL(1110), opcode(4), S=0, Rn(4), Rd(4), operand2
  ;; For register operand2: imm5(5) shift(2) shift_type(2) Rm(4)

  ;; MOV Rd, Rm: 0xE1A00000 | (Rd << 12) | Rm

  ;; MOV Rd, #imm (8-bit, zero-extended)
  ;; ARM immediate encoding: 0xE3A00000 | (Rd << 12) | imm8
  ;; Only handles 0-255 directly

  ;; ADD Rd, Rn, Rm: 0xE0800000 | (Rn << 16) | (Rd << 12) | Rm

  ;; SUB Rd, Rn, Rm: 0xE0400000 | (Rn << 16) | (Rd << 12) | Rm

  ;; RSB Rd, Rn, Rm (reverse sub: Rd = Rm - Rn): 0xE0600000 | ...

  ;; AND Rd, Rn, Rm: 0xE0000000 | (Rn << 16) | (Rd << 12) | Rm

  ;; ORR Rd, Rn, Rm: 0xE1800000 | (Rn << 16) | (Rd << 12) | Rm

  ;; EOR Rd, Rn, Rm: 0xE0200000 | (Rn << 16) | (Rd << 12) | Rm

  ;; MUL Rd, Rn, Rm: 0xE0000900 | (Rd << 16) | (Rn << 12) | Rm

  ;; SDIV Rd, Rn, Rm (ARMv7): 0xE710F010 | (Rn << 16) | (Rd << 12) | Rm

  ;; UDIV Rd, Rn, Rm (ARMv7): 0xE730F010 | (Rn << 16) | (Rd << 12) | Rm

  ;; LSL Rd, Rn, Rm (variable shift): 0xE1A00010 | (Rn << 16) | (Rd << 12) | Rm
  ;; Actually for variable LSL: MOV Rd, Rn, LSL Rm
  ;; Encoding: cond 0001 1010 S Rn Rd 0001 0 Rm  -- with shift
  ;; Actually: MOV Rd, Rn, LSL Rm = 0xE1A00010 | (Rn << 16) | (Rd << 12) | Rm
  ;; Wait, that doesn't look right. MOV Rd, Rn, LSL Rm:
  ;;   cond=1110, 000 1101 0, S=0, Rn, Rd, 0001 0, Rm
  ;; Hmm, the barrel shifter encoding for LSL with register is:
  ;;   shift_imm=0000, shift=00, 1, Rm (for LSL by register)
  ;; This gives: 0xE1A00010 | (Rn << 16) | (Rd << 12) | Rm

  ;; LSR Rd, Rn, Rm: MOV Rd, Rn, LSR Rm = 0xE1A00030 | ...

  ;; ASR Rd, Rn, Rm: MOV Rd, Rn, ASR Rm = 0xE1A00050 | ...

  ;; ROR Rd, Rn, Rm: MOV Rd, Rn, ROR Rm = 0xE1A00070 | ...

  ;; ── ARM32 ADD/SUB with immediate ─────────────────────────────────

  ;; ADD Rd, Rn, #imm8 (with rotate): 0xE2800000 | (Rn << 16) | (Rd << 12) | imm12
  ;; imm12 = [rotate:imm8] where rotate is 4-bit (even rotate amount)
  ;; For small immediates (0-255): rotate=0, so imm12 = imm8

  ;; SUB Rd, Rn, #imm8: 0xE2400000 | (Rn << 16) | (Rd << 12) | imm8

  ;; ── ARM32 CLZ (count leading zeros) ──────────────────────────────
  ;; CLZ Rd, Rm: 0xE16F0F10 | (Rd << 12) | Rm

  ;; ── ARM32 RBIT (reverse bits) ────────────────────────────────────
  ;; RBIT Rd, Rm: 0xE6FF0F30 | (Rd << 12) | Rm

  ;; ── ARM32 Load/Store helpers ─────────────────────────────────────
  ;; LDR Rt, [Rn, Rm]: 0xE7900000 | (Rn << 16) | (Rt << 12) | Rm

  ;; STR Rt, [Rn, Rm]: 0xE7800000 | (Rn << 16) | (Rt << 12) | Rm

  ;; LDR Rt, [Rn, #imm12]: 0xE5900000 | (Rn << 16) | (Rt << 12) | imm12

  ;; STR Rt, [Rn, #imm12]: 0xE5800000 | (Rn << 16) | (Rt << 12) | imm12

  ;; LDR pre/post-index helpers for stack

  ;; STR Rt, [SP, #-4]! (pre-index, push): 0xE52D0004 | (Rt << 12)
  ;; Actually STMDB SP!, {Rt} would be better
  ;; But for single reg: STR Rt, [SP, -#imm12]! = 0xE52D0000 | (Rt << 12) | imm12
  ;; For imm12=4: 0xE52D0004 | (Rt << 12)

  ;; LDR Rt, [SP], #4 (post-index, pop): 0xE49D0004 | (Rt << 12)

  ;; ── ARM32 Stack push/pop (using real ARM stack) ──────────────────

  ;; Push R0: STR R0, [SP, #-4]!

  ;; Pop R0: LDR R0, [SP], #4

  ;; Push R1

  ;; Pop R1

  ;; Push R2

  ;; Pop R2

  ;; Pop pair: pop R1 then R0

  ;; Push pair: push R1 then R0 (actually reverse: push R0 then R1)

  ;; ── Standard push/pop names ─────────────────────────────────────

  ;; ── ARM32 branch instructions ────────────────────────────────────

  ;; B #imm (unconditional branch, ±32MB): 0xEA000000 | imm24
  ;; imm24 = (target - pc - 8) >> 2, signed 24-bit
  )

  ;; BL #imm (branch with link): 0xEB000000 | imm24
  )

  ;; B.cond #imm (conditional branch, ±32MB): cond 1010 imm24
  ;; cond codes: EQ=0, NE=1, CS=2, CC=3, MI=4, PL=5, VS=6, VC=7
  ;; HI=8, LS=9, GE=10, LT=11, GT=12, LE=13, AL=14
  )

  ;; CBZ/CBNZ not available in ARM32 (it's a Thumb-2 instruction)
  ;; Use CMP + B cond instead

  ;; ── ARM32 CMP/TST helpers ────────────────────────────────────────

  ;; CMP Rn, Rm: 0xE1500000 | (Rn << 16) | Rm

  ;; CMP Rn, #imm8: 0xE3500000 | (Rn << 16) | imm8

  ;; TST Rn, Rm: 0xE1100000 | (Rn << 16) | Rm

  ;; ── Conditional moves (for CSET) ──────────────────────────────────
  ;; MOVNE R0, #1 (move if not equal): 0x13A00001 | ...
  ;; Actually: MOVcc Rd, #imm8 where cc is the condition
  ;; MOVNE R0, #1: 0x13A00001

  ;; ── ARM32 sign extension ─────────────────────────────────────────
  ;; SXTB Rd, Rm (sign-extend byte to word): 0xE6AF0070 | (Rd << 12) | Rm

  ;; SXTH Rd, Rm (sign-extend halfword to word): 0xE6BF0070 | (Rd << 12) | Rm

  ;; ── ARM32 Memory load/store through JitGlobals ──────────────────
  ;; JitGlobals layout matches x86-64/AArch64:
  ;;   +0: locals pointer
  ;;   +4: mem_ptr (WASM linear memory base)
  ;;   +8: ...
  ;;   +24: globals_buf
  ;;   +48: table_entries
  ;;   +64: memory_pages

  ;; Load mem_ptr into R2: LDR R2, [R8, #4]

  ;; LDR R0, [R8, #offset]

  ;; STR R0, [R8, #offset]

  ;; LDR R0, [R9, #offset]

  ;; STR R0, [R9, #offset]

  ;; ── ARM32 memory load/store via register offset ─────────────────

  ;; i32.load: address in R1, result in R0
  ;; LDR R0, [R2, R1] where R2 = mem_ptr

  ;; i64.load: same as i32.load (ARM32 is 32-bit, handles 32-bit only)
  ;; LDR R0, [R2, R1] - only loads lower 32 bits

  ;; i32.store: address in R1, value in R0

  ;; i64.store: same as i32.store (32-bit only)

  ;; ── ARM32 Unary arithmetic helpers (R0←op(R0)) ──────────────────

  ;; CLZ R0, R0

  ;; RBIT R0, R0

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

  ;; VFP 3-operand double: same as single with Dreg = Dnum << 1 (into S encoding)

  ;; ── VFP 2-operand monadic single (VABS/VNEG/VSQRT/FRINT.F32) ──
  ;; Encoding: cond 1110 D 1 op2 Vd Vn 1011 N 0 M 0 Vm
  ;; For monadic: Vd[3:0]=Sd[3:0]+bit23=Sd[4], Vm[3:0]=Sm[3:0]+bit5=Sm[4]


  ;; ── VFP compare single ──────────────────────────────────────────
  ;; FCMP Sd, Sm: cond 1110 D 1 0 1 Vd Vn 1011 N 0 M 0 Vm  (bits 20:16 = 10100?)
  ;; Using kernel defines: FEXT_FCMP = 0x00040000 (bit 18), bit7=0
  ;; Base with Sd=0, Sm=0: 0xEEB40A40


  ;; ── Move ARM ↔ VFP (VMOV) ─────────────────────────────────────
  ;; VMOV Sd, Rt (ARM→VFP 32-bit):
  ;;   cond 1110 00 0 0 Vn Rt 1010 N 001 0000
  ;;   Where Vn:N = (Sd>>1):(Sd&1)
  ;;   = 0xEE000A10 | ((Sd>>1)<<16) | ((Sd&1)<<7) | Rt

  ;; VMOV Rd, Sn (VFP→ARM 32-bit):
  ;;   cond 1110 00 0 1 Vn Rd 1010 N 001 0000
  ;;   = 0xEE100A10 | ((Sn>>1)<<16) | ((Sn&1)<<7) | Rd

  ;; VMOV Dd, Xn (ARM→VFP 64-bit) not needed for ARM32 (no 64-bit ARM)

  ;; VMOV Xd, Dn (VFP→ARM 64-bit) not needed for ARM32

  ;; ── VFP 3-operand bases (F32) ──────────────────────────────────
  ;; VADD.F32: base = 0x0E300A00, VSUB.F32: base = 0x0E300A40
  ;; VMUL.F32: base = 0x0E200A00, VDIV.F32: base = 0x0E800A00
  ;; VADD.F64: base = 0x0E300B00 (bit8=1), VSUB.F64: base = 0x0E300B40
  ;; VMUL.F64: base = 0x0E200B00, VDIV.F64: base = 0x0E800B00


  ;; ── Int-to-float conversions (SCVTF/UCVTF) ────────────────────
  ;; VCVT.F32.S32 Sd, Sm: base = 0x0EB80A40
  ;; VCVT.F32.U32 Sd, Sm: base = 0x0EB80A40 (bit for signed/unsigned may differ)


  ;; ── Floating precision conversion (FCVT) ──────────────────────
  ;; VCVT.F32.F64 Sd, Dm: base = 0x0EB60B40
  ;; VCVT.F64.F32 Dd, Sm: base = 0x0EB70A40


  ;; ── FRINT (rounding) ──────────────────────────────────────────
  ;; ARM32 VFPv3 doesn't have dedicated FRINT; uses VCVT round-trip
  ;; For now, emit NOP-like instruction to keep the value


  ;; ── ELF32 header (52 bytes) ─────────────────────────────────────


  ;; ── ELF32 program header (32 bytes) ────────────────────────────


  ;; ── Runtime stub for ELF: sets up R8=JitGlobals, calls code, exits ──


  ;; ── Copy compiled code from JIT cache to output ────────────────

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

    (call $emit_arm32_elf32_ehdr
      (i32.add (global.get $TEXT_VA) (global.get $ELF_STUB_OFF))
      (i32.const 52)
      (i32.const 1))

    (call $emit_arm32_elf32_phdr
      (i32.const 1)         ;; PT_LOAD
      (i32.const 7)         ;; PF_R | PF_W | PF_X
      (i32.const 0)         ;; p_offset
      (global.get $TEXT_VA) ;; p_vaddr
      (local.get $total_size) ;; p_filesz
      (i32.add (i32.and (i32.add (local.get $total_size) (i32.const 0xFFF)) (i32.const -0x1000)) (global.get $BSS_SIZE))) ;; p_memsz

    (call $emit_arm32_elf_stub (local.get $bss_va))

    (block $pad_done
      (loop $pad_loop
        (if (i32.ge_u (i32.load (global.get $JS_CODE_PTR))
                       (i32.add (global.get $ELF_OUT_OFF) (global.get $ELF_CODE_OFF)))
          (then (br $pad_done)))
        (call $emit_arm32_byte (i32.const 0))
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

  ;; Emit bare-metal stub (no headers). Returns stub size.
  ;; Compiled code placed right after stub.

  ;; ── Copy code to binary output buffer ──────────────────────────

    )
  )

  ;; ── compile_to_bin: emit flat binary ───────────────────────────

  (func (export "compile_to_bin") (param $func_idx i32) (result i32 i32)
    (local $code_size i32) (local $stub_size i32) (local $total_size i32)
    (local $saved i32)

    (local.set $code_size (call $jit_compile (local.get $func_idx)))
    (local.set $saved (i32.load (global.get $JS_CODE_PTR)))
    (i32.store (global.get $JS_CODE_PTR) (global.get $BIN_OUT_OFF))

    (local.set $stub_size (call $emit_arm32_bare_metal_stub))
    (call $copy_code_to (global.get $BIN_OUT_BUF) (local.get $code_size) (local.get $stub_size))
    (local.set $total_size (i32.add (local.get $stub_size) (local.get $code_size)))

    (i32.store (global.get $JS_CODE_PTR) (local.get $saved))
    (return (global.get $BIN_OUT_BUF) (local.get $total_size))
  )

  ;; ── ARM32 function prologue/epilogue ──────────────────────────────

  ;; Prologue: save LR, save callee-saved regs, setup frame, zero temp regs

  ;; Epilogue: restore callee-saved, return

  ;; ── JIT state helpers ─────────────────────────────────────────────




  ;; ── Convenience arithmetic/stack helpers ──────────────────────────

  ;; ADD R0, R0, R1

  ;; SUB R0, R0, R1

  ;; MUL R0, R0, R1

  ;; SDIV R0, R0, R1

  ;; UDIV R0, R0, R1

  ;; AND R0, R0, R1

  ;; ORR R0, R0, R1

  ;; EOR R0, R0, R1

  ;; LSL R0, R0, R1 (variable shift)

  ;; LSR R0, R0, R1

  ;; ASR R0, R0, R1

  ;; ROR R0, R0, R1

  ;; ── Pop two values: pop R1 (right), pop R0 (left) ────────────────

  ;; ── XOR R0, R0 (zero register) ────────────────────────────────────
  ;; EOR R0, R0, R0 = 0xE0200000

  ;; EOR R1, R1, R1

  ;; EOR R2, R2, R2

  ;; MOV R0, R1

  ;; MOV R1, R0

  ;; ── Alias: "x0" names for templates expecting x86-like names ──────

  ;; CSET X0, cond: set R0 to 1 if cond true, else 0
  ;; Input: inv_cond = inverted condition (since templates pass inverted)
  ;; original cond = inv_cond ^ 1
  ;; Strategy:
  ;;   MOVcc R0, #1   (where cc = original cond)
  ;;   MOV!cc R0, #0  (where !cc = original cond ^ 1)

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


  ;; MOVK (move keep): MOVT Rd, #imm16 (sets bits 31:16)
  ;; MOVT Rd, #imm16: 0xE3400000 | (Rd << 12) | imm12 | ((imm16>>12) << 16)

  ;; MOVN (move not): MVN Rd, #imm

  ;; ADD immediate (aliases)

  ;; LDR immediate (aliases)

  ;; Pre/post index stack helpers (aliases)

  ;; Register offset load/store (aliases for 'x' naming)

  ;; LDR x0 from x19/r8 aliases

  ;; CBZ/CBNZ equivalents using CMP + B.cond

  ;; ── Missing aliases for template compatibility ────────────────────

  ;; 64-bit CLZ/RBIT (same as 32-bit on ARM32)

  ;; Raw instruction helpers called directly from templates

  ;; ═════════════════════════════════════════════════════════════════════
  ;; ARM32 NEON (SIMD) emit helpers — v128 load/store/push/pop
  ;; ═════════════════════════════════════════════════════════════════════

  ;; Push Q0 onto WASM value stack: VSTMDB SP!, {D0-D1}
  ;; Stores D0 and D1 (16 bytes = 128 bits), decrements SP by 16.
  ;; Encoding: cond=1110, 110, P=1, U=0, D=0, W=1, L=0, Rn=SP=13, Vd=0, 1011, imm8=2

  ;; Pop Q0 from WASM value stack: VLDMIA SP!, {D0-D1}
  ;; Loads D0 and D1 (16 bytes = 128 bits), increments SP by 16.
  ;; Encoding: cond=1110, 110, P=0, U=1, D=0, W=1, L=1, Rn=SP=13, Vd=0, 1011, imm8=2

  ;; Load Q0 from address in R1: VLDMIA R1, {D0-D1}
  ;; Loads 16 bytes from [R1] into D0,D1.
  ;; Encoding: cond=1110, 110, P=0, U=1, D=0, W=0, L=1, Rn=1, Vd=0, 1011, imm8=2

  ;; Store Q0 to address in R1: VSTMIA R1, {D0-D1}
  ;; Stores 16 bytes from D0,D1 into [R1].
  ;; Encoding: cond=1110, 110, P=0, U=1, D=0, W=0, L=0, Rn=1, Vd=0, 1011, imm8=2

  ;; Load 32-bit constant address into R1 (MOVW/MOVT)

  ;; ── Pop two, operate, push (pattern for binary ops) ──────────────

  ;; Push R0 only if RESULT_IN_X0 is 0 (peephole)
          (else
            (call $emit_arm32_push_r0)
          )
        )
      )
    )
  )

  ;; Opcode templates — each emits AArch64 code for one WASM opcode
  ;; ═════════════════════════════════════════════════════════════════════

  ;; ── unreachable (0x00): BRK #0 (debug breakpoint) ────────────────

  ;; ── nop (0x01): nothing ────────────────────────────────────────────

  ;; ── select (0x1B): pop cond, pop val2, pop val1, select ──────────

  ;; ── i32.const (0x41): push imm32 ──────────────────────────────────
    )
    (call $emit_arm32_maybe_push_x0)
  )

  ;; ── i64.const (0x42): push imm64 ──────────────────────────────────
    (if (i32.ne (local.get $val2) (i32.const 0))
      (then (call $emit_arm32_instr_movk_64 (global.get $REG_X0) (i32.const 2) (local.get $val2)))
    )
    (if (i32.ne (local.get $val3) (i32.const 0))
      (then (call $emit_arm32_instr_movk_64 (global.get $REG_X0) (i32.const 3) (local.get $val3)))
    )
    (call $emit_arm32_maybe_push_x0)
  )

  ;; ── local.get (0x20): load local at index ─────────────────────────
      (else
        (if (i32.eq (local.get $idx) (i32.const 1))
          (then
            (call $emit_arm32_instr (i32.const 0xAA0003F6))  ;; MOV X0, X22
            (call $emit_arm32_push_x0)
          )
          (else
            ;; Load from memory via X20 (locals base)
            (call $emit_arm32_ldr_x0_x20 (local.get $idx))  ;; actual offset = idx * 8 / 8 = idx
            (call $emit_arm32_maybe_push_x0)
          )
        )
      )
    )
  )

  ;; ── local.set (0x21): pop/store to local ──────────────────────────
          (else
            (if (i32.eq (local.get $idx) (i32.const 1))
              (then (call $emit_arm32_instr (i32.const 0xAA0003C0)))  ;; MOV X22, X0
              (else (call $emit_arm32_str_x0_x20 (i32.mul (local.get $idx) (i32.const 1))))  ;; offset = idx*8/8 = idx
            )
          )
        )
      )
      (else
        (if (i32.eqz (local.get $idx))
          (then
            (call $emit_arm32_pop_x0)
            (call $emit_arm32_instr (i32.const 0xAA0003E0))  ;; MOV X21, X0
          )
          (else
            (if (i32.eq (local.get $idx) (i32.const 1))
              (then
                (call $emit_arm32_pop_x0)
                (call $emit_arm32_instr (i32.const 0xAA0003C0))  ;; MOV X22, X0
              )
              (else
                (call $emit_arm32_pop_x0)
                (call $emit_arm32_str_x0_x20 (i32.mul (local.get $idx) (i32.const 1)))
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
        (call $emit_arm32_instr_ldr_64_off (global.get $REG_X0) (global.get $REG_SP) (i32.const 0))
        (call $emit_arm32_push_x0)
        (if (i32.eqz (local.get $idx))
          (then
            (call $emit_arm32_pop_x0)
            (call $emit_arm32_instr (i32.const 0xAA0003E0))  ;; MOV X21, X0
          )
          (else
            (if (i32.eq (local.get $idx) (i32.const 1))
              (then
                (call $emit_arm32_pop_x0)
                (call $emit_arm32_instr (i32.const 0xAA0003C0))  ;; MOV X22, X0
              )
              (else
                (call $emit_arm32_pop_x0)
                (call $emit_arm32_str_x0_x20 (i32.mul (local.get $idx) (i32.const 1)))
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
    (call $emit_arm32_pop_x0)
    (call $emit_arm32_cbz_x (global.get $REG_X0) (i32.const 0))  ;; placeholder offset
  )
  (func $template_else (param $dec_ptr i32)
    (call $emit_arm32_b (i32.const 0))  ;; placeholder branch
  )
  (func $template_end (param $dec_ptr i32))
  (func $template_br (param $dec_ptr i32)
    (call $emit_arm32_b (i32.const 0))  ;; placeholder
  )
  (func $template_br_if (param $dec_ptr i32)
    (call $emit_arm32_pop_x0)
    (call $emit_arm32_cbnz_x (global.get $REG_X0) (i32.const 0))  ;; placeholder
  )
  (func $template_br_table (param $dec_ptr i32)
    (call $emit_arm32_pop_x0)
    (call $emit_arm32_b (i32.const 0))  ;; placeholder (jump to default)
  )
  (func $template_return (param $dec_ptr i32)
    (call $emit_arm32_epilogue)
  )
  (func $template_return_call (param $dec_ptr i32)
    (call $emit_arm32_epilogue)
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


  ;; ═════════════════════════════════════════════════════════════════════
  ;; WASM SIMD (0xFD prefix) — ARM32 stubs (NEON TBD)
  ;; ═════════════════════════════════════════════════════════════════════
  (func $template_arm32_v128_const (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_arm32_v128_load (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_arm32_v128_store (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_arm32_i8x16_splat (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_arm32_i16x8_splat (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_arm32_i32x4_splat (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_arm32_i64x2_splat (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_arm32_f32x4_splat (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_arm32_f64x2_splat (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_arm32_i8x16_add (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_arm32_i16x8_add (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_arm32_i32x4_add (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_arm32_i64x2_add (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_arm32_i8x16_sub (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_arm32_i16x8_sub (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_arm32_i32x4_sub (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_arm32_i64x2_sub (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_arm32_f32x4_add (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_arm32_f64x2_add (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_arm32_f32x4_sub (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_arm32_f64x2_sub (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_arm32_i8x16_mul (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_arm32_i16x8_mul (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_arm32_i32x4_mul (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_arm32_f32x4_mul (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_arm32_f64x2_mul (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_arm32_i8x16_neg (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_arm32_i16x8_neg (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_arm32_i32x4_neg (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_arm32_i64x2_neg (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_arm32_f32x4_neg (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_arm32_f64x2_neg (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_arm32_i8x16_eq (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_arm32_i16x8_eq (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_arm32_i32x4_eq (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_arm32_f32x4_eq (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_arm32_f64x2_eq (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_arm32_i8x16_ne (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_arm32_i16x8_ne (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_arm32_i32x4_ne (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_arm32_f32x4_ne (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_arm32_f64x2_ne (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_arm32_i8x16_lt_s (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_arm32_i8x16_lt_u (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_arm32_i16x8_lt_s (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_arm32_i16x8_lt_u (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_arm32_i32x4_lt_s (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_arm32_i32x4_lt_u (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_arm32_f32x4_lt (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_arm32_f64x2_lt (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_arm32_i8x16_gt_s (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_arm32_i8x16_gt_u (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_arm32_i16x8_gt_s (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_arm32_i16x8_gt_u (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_arm32_i32x4_gt_s (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_arm32_i32x4_gt_u (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_arm32_f32x4_gt (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_arm32_f64x2_gt (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_arm32_i8x16_le_s (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_arm32_i8x16_le_u (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_arm32_i16x8_le_s (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_arm32_i16x8_le_u (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_arm32_i32x4_le_s (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_arm32_i32x4_le_u (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_arm32_f32x4_le (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_arm32_f64x2_le (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_arm32_i8x16_ge_s (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_arm32_i16x8_ge_s (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_arm32_i32x4_ge_s (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_arm32_f32x4_ge (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_arm32_f64x2_ge (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_arm32_i8x16_and (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_arm32_i8x16_or (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_arm32_i8x16_xor (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_arm32_i8x16_extract_lane_s (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_arm32_i8x16_extract_lane_u (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_arm32_i16x8_extract_lane_s (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_arm32_i16x8_extract_lane_u (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_arm32_i32x4_extract_lane (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_arm32_i64x2_extract_lane (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_arm32_f32x4_extract_lane (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_arm32_f64x2_extract_lane (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_arm32_i8x16_replace_lane (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_arm32_i16x8_replace_lane (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_arm32_i32x4_replace_lane (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_arm32_i64x2_replace_lane (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_arm32_f32x4_replace_lane (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_arm32_f64x2_replace_lane (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  ;; ── New canonical op stubs (0xA6+) ────────────────────────────────
  (func $template_arm32_v128_load8_splat (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_arm32_v128_load16_splat (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_arm32_v128_load32_splat (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_arm32_v128_load64_splat (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_arm32_i8x16_swizzle (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_arm32_v128_not (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_arm32_v128_bitselect (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_arm32_i8x16_ge_u (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_arm32_i16x8_ge_u (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_arm32_i32x4_ge_u (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_arm32_i8x16_min_u (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_arm32_i8x16_max_u (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_arm32_i8x16_avgr_u (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_arm32_i16x8_min_u (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_arm32_i16x8_max_u (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_arm32_i16x8_avgr_u (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_arm32_f32x4_min (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_arm32_f32x4_max (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_arm32_f64x2_min (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))
  (func $template_arm32_f64x2_max (param $dec_ptr i32) (global.set $JIT_ERROR (i32.const -4)))

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
    (call $emit_arm32_prologue)

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
                  (then (call $template_arm32_v128_load (local.get $dec_ptr)) (br $fd_done)))
                ;; v128.store (0x1B)
                (if (i32.eq (local.get $imm0) (i32.const 0x1B))
                  (then (call $template_arm32_v128_store (local.get $dec_ptr)) (br $fd_done)))
                ;; v128.const (0x0C)
                (if (i32.eq (local.get $imm0) (i32.const 0x0C))
                  (then (call $template_arm32_v128_const (local.get $dec_ptr)) (br $fd_done)))
                ;; i8x16.splat (0x2D)
                (if (i32.eq (local.get $imm0) (i32.const 0x2D))
                  (then (call $template_arm32_i8x16_splat (local.get $dec_ptr)) (br $fd_done)))
                ;; i16x8.splat (0x31)
                (if (i32.eq (local.get $imm0) (i32.const 0x31))
                  (then (call $template_arm32_i16x8_splat (local.get $dec_ptr)) (br $fd_done)))
                ;; i32x4.splat (0x35)
                (if (i32.eq (local.get $imm0) (i32.const 0x35))
                  (then (call $template_arm32_i32x4_splat (local.get $dec_ptr)) (br $fd_done)))
                ;; i64x2.splat (0x39)
                (if (i32.eq (local.get $imm0) (i32.const 0x39))
                  (then (call $template_arm32_i64x2_splat (local.get $dec_ptr)) (br $fd_done)))
                ;; f32x4.splat (0x3A)
                (if (i32.eq (local.get $imm0) (i32.const 0x3A))
                  (then (call $template_arm32_f32x4_splat (local.get $dec_ptr)) (br $fd_done)))
                ;; f64x2.splat (0x3D)
                (if (i32.eq (local.get $imm0) (i32.const 0x3D))
                  (then (call $template_arm32_f64x2_splat (local.get $dec_ptr)) (br $fd_done)))
                ;; i8x16.extract_lane_s (0x2E)
                (if (i32.eq (local.get $imm0) (i32.const 0x2E))
                  (then (call $template_arm32_i8x16_extract_lane_s (local.get $dec_ptr)) (br $fd_done)))
                ;; i8x16.extract_lane_u (0x2F)
                (if (i32.eq (local.get $imm0) (i32.const 0x2F))
                  (then (call $template_arm32_i8x16_extract_lane_u (local.get $dec_ptr)) (br $fd_done)))
                ;; i16x8.extract_lane_s (0x32)
                (if (i32.eq (local.get $imm0) (i32.const 0x32))
                  (then (call $template_arm32_i16x8_extract_lane_s (local.get $dec_ptr)) (br $fd_done)))
                ;; i16x8.extract_lane_u (0x33)
                (if (i32.eq (local.get $imm0) (i32.const 0x33))
                  (then (call $template_arm32_i16x8_extract_lane_u (local.get $dec_ptr)) (br $fd_done)))
                ;; i32x4.extract_lane (0x36)
                (if (i32.eq (local.get $imm0) (i32.const 0x36))
                  (then (call $template_arm32_i32x4_extract_lane (local.get $dec_ptr)) (br $fd_done)))
                ;; i64x2.extract_lane (0x38)
                (if (i32.eq (local.get $imm0) (i32.const 0x38))
                  (then (call $template_arm32_i64x2_extract_lane (local.get $dec_ptr)) (br $fd_done)))
                ;; f32x4.extract_lane (0x3B)
                (if (i32.eq (local.get $imm0) (i32.const 0x3B))
                  (then (call $template_arm32_f32x4_extract_lane (local.get $dec_ptr)) (br $fd_done)))
                ;; f64x2.extract_lane (0x3E)
                (if (i32.eq (local.get $imm0) (i32.const 0x3E))
                  (then (call $template_arm32_f64x2_extract_lane (local.get $dec_ptr)) (br $fd_done)))
                ;; i8x16.replace_lane (0x30)
                (if (i32.eq (local.get $imm0) (i32.const 0x30))
                  (then (call $template_arm32_i8x16_replace_lane (local.get $dec_ptr)) (br $fd_done)))
                ;; i16x8.replace_lane (0x34)
                (if (i32.eq (local.get $imm0) (i32.const 0x34))
                  (then (call $template_arm32_i16x8_replace_lane (local.get $dec_ptr)) (br $fd_done)))
                ;; i32x4.replace_lane (0x37)
                (if (i32.eq (local.get $imm0) (i32.const 0x37))
                  (then (call $template_arm32_i32x4_replace_lane (local.get $dec_ptr)) (br $fd_done)))
                ;; i64x2.replace_lane (0xAB) — canonical ID, not in old encoding
                ;; f32x4.replace_lane (0x3C)
                (if (i32.eq (local.get $imm0) (i32.const 0x3C))
                  (then (call $template_arm32_f32x4_replace_lane (local.get $dec_ptr)) (br $fd_done)))
                ;; f64x2.replace_lane (0x3F)
                (if (i32.eq (local.get $imm0) (i32.const 0x3F))
                  (then (call $template_arm32_f64x2_replace_lane (local.get $dec_ptr)) (br $fd_done)))
                ;; i8x16.add (0x40)
                (if (i32.eq (local.get $imm0) (i32.const 0x40))
                  (then (call $template_arm32_i8x16_add (local.get $dec_ptr)) (br $fd_done)))
                ;; i16x8.add (0x51)
                (if (i32.eq (local.get $imm0) (i32.const 0x51))
                  (then (call $template_arm32_i16x8_add (local.get $dec_ptr)) (br $fd_done)))
                ;; i32x4.add (0x62)
                (if (i32.eq (local.get $imm0) (i32.const 0x62))
                  (then (call $template_arm32_i32x4_add (local.get $dec_ptr)) (br $fd_done)))
                ;; i64x2.add (0x73)
                (if (i32.eq (local.get $imm0) (i32.const 0x73))
                  (then (call $template_arm32_i64x2_add (local.get $dec_ptr)) (br $fd_done)))
                ;; f32x4.add (0x84)
                (if (i32.eq (local.get $imm0) (i32.const 0x84))
                  (then (call $template_arm32_f32x4_add (local.get $dec_ptr)) (br $fd_done)))
                ;; f64x2.add (0x95)
                (if (i32.eq (local.get $imm0) (i32.const 0x95))
                  (then (call $template_arm32_f64x2_add (local.get $dec_ptr)) (br $fd_done)))
                ;; i8x16.sub (0x41)
                (if (i32.eq (local.get $imm0) (i32.const 0x41))
                  (then (call $template_arm32_i8x16_sub (local.get $dec_ptr)) (br $fd_done)))
                ;; i16x8.sub (0x52)
                (if (i32.eq (local.get $imm0) (i32.const 0x52))
                  (then (call $template_arm32_i16x8_sub (local.get $dec_ptr)) (br $fd_done)))
                ;; i32x4.sub (0x63)
                (if (i32.eq (local.get $imm0) (i32.const 0x63))
                  (then (call $template_arm32_i32x4_sub (local.get $dec_ptr)) (br $fd_done)))
                ;; i64x2.sub (0x74)
                (if (i32.eq (local.get $imm0) (i32.const 0x74))
                  (then (call $template_arm32_i64x2_sub (local.get $dec_ptr)) (br $fd_done)))
                ;; f32x4.sub (0x85)
                (if (i32.eq (local.get $imm0) (i32.const 0x85))
                  (then (call $template_arm32_f32x4_sub (local.get $dec_ptr)) (br $fd_done)))
                ;; f64x2.sub (0x96)
                (if (i32.eq (local.get $imm0) (i32.const 0x96))
                  (then (call $template_arm32_f64x2_sub (local.get $dec_ptr)) (br $fd_done)))
                ;; i8x16.mul (0x50)
                (if (i32.eq (local.get $imm0) (i32.const 0x50))
                  (then (call $template_arm32_i8x16_mul (local.get $dec_ptr)) (br $fd_done)))
                ;; i16x8.mul (0x61)
                (if (i32.eq (local.get $imm0) (i32.const 0x61))
                  (then (call $template_arm32_i16x8_mul (local.get $dec_ptr)) (br $fd_done)))
                ;; i32x4.mul (0x72)
                (if (i32.eq (local.get $imm0) (i32.const 0x72))
                  (then (call $template_arm32_i32x4_mul (local.get $dec_ptr)) (br $fd_done)))
                ;; f32x4.mul (0x87)
                (if (i32.eq (local.get $imm0) (i32.const 0x87))
                  (then (call $template_arm32_f32x4_mul (local.get $dec_ptr)) (br $fd_done)))
                ;; f64x2.mul (0x98)
                (if (i32.eq (local.get $imm0) (i32.const 0x98))
                  (then (call $template_arm32_f64x2_mul (local.get $dec_ptr)) (br $fd_done)))
                ;; i8x16.neg (0x46)
                (if (i32.eq (local.get $imm0) (i32.const 0x46))
                  (then (call $template_arm32_i8x16_neg (local.get $dec_ptr)) (br $fd_done)))
                ;; i16x8.neg (0x57)
                (if (i32.eq (local.get $imm0) (i32.const 0x57))
                  (then (call $template_arm32_i16x8_neg (local.get $dec_ptr)) (br $fd_done)))
                ;; i32x4.neg (0x68)
                (if (i32.eq (local.get $imm0) (i32.const 0x68))
                  (then (call $template_arm32_i32x4_neg (local.get $dec_ptr)) (br $fd_done)))
                ;; i64x2.neg (0x79)
                (if (i32.eq (local.get $imm0) (i32.const 0x79))
                  (then (call $template_arm32_i64x2_neg (local.get $dec_ptr)) (br $fd_done)))
                ;; f32x4.neg (0x8A)
                (if (i32.eq (local.get $imm0) (i32.const 0x8A))
                  (then (call $template_arm32_f32x4_neg (local.get $dec_ptr)) (br $fd_done)))
                ;; f64x2.neg (0x9B)
                (if (i32.eq (local.get $imm0) (i32.const 0x9B))
                  (then (call $template_arm32_f64x2_neg (local.get $dec_ptr)) (br $fd_done)))
                ;; i8x16.eq (0x47)
                (if (i32.eq (local.get $imm0) (i32.const 0x47))
                  (then (call $template_arm32_i8x16_eq (local.get $dec_ptr)) (br $fd_done)))
                ;; i16x8.eq (0x58)
                (if (i32.eq (local.get $imm0) (i32.const 0x58))
                  (then (call $template_arm32_i16x8_eq (local.get $dec_ptr)) (br $fd_done)))
                ;; i32x4.eq (0x69)
                (if (i32.eq (local.get $imm0) (i32.const 0x69))
                  (then (call $template_arm32_i32x4_eq (local.get $dec_ptr)) (br $fd_done)))
                ;; f32x4.eq (0x8C)
                (if (i32.eq (local.get $imm0) (i32.const 0x8C))
                  (then (call $template_arm32_f32x4_eq (local.get $dec_ptr)) (br $fd_done)))
                ;; f64x2.eq (0x9D)
                (if (i32.eq (local.get $imm0) (i32.const 0x9D))
                  (then (call $template_arm32_f64x2_eq (local.get $dec_ptr)) (br $fd_done)))
                ;; i8x16.ne (0x48)
                (if (i32.eq (local.get $imm0) (i32.const 0x48))
                  (then (call $template_arm32_i8x16_ne (local.get $dec_ptr)) (br $fd_done)))
                ;; i16x8.ne (0x59)
                (if (i32.eq (local.get $imm0) (i32.const 0x59))
                  (then (call $template_arm32_i16x8_ne (local.get $dec_ptr)) (br $fd_done)))
                ;; i32x4.ne (0x6A)
                (if (i32.eq (local.get $imm0) (i32.const 0x6A))
                  (then (call $template_arm32_i32x4_ne (local.get $dec_ptr)) (br $fd_done)))
                ;; f32x4.ne (0x8B)
                (if (i32.eq (local.get $imm0) (i32.const 0x8B))
                  (then (call $template_arm32_f32x4_ne (local.get $dec_ptr)) (br $fd_done)))
                ;; f64x2.ne (0x9C)
                (if (i32.eq (local.get $imm0) (i32.const 0x9C))
                  (then (call $template_arm32_f64x2_ne (local.get $dec_ptr)) (br $fd_done)))
                ;; i8x16.lt_s (0x4B)
                (if (i32.eq (local.get $imm0) (i32.const 0x4B))
                  (then (call $template_arm32_i8x16_lt_s (local.get $dec_ptr)) (br $fd_done)))
                ;; i8x16.lt_u (0x4A)
                (if (i32.eq (local.get $imm0) (i32.const 0x4A))
                  (then (call $template_arm32_i8x16_lt_u (local.get $dec_ptr)) (br $fd_done)))
                ;; i16x8.lt_s (0x5C)
                (if (i32.eq (local.get $imm0) (i32.const 0x5C))
                  (then (call $template_arm32_i16x8_lt_s (local.get $dec_ptr)) (br $fd_done)))
                ;; i16x8.lt_u (0x5B)
                (if (i32.eq (local.get $imm0) (i32.const 0x5B))
                  (then (call $template_arm32_i16x8_lt_u (local.get $dec_ptr)) (br $fd_done)))
                ;; i32x4.lt_s (0x6D)
                (if (i32.eq (local.get $imm0) (i32.const 0x6D))
                  (then (call $template_arm32_i32x4_lt_s (local.get $dec_ptr)) (br $fd_done)))
                ;; i32x4.lt_u (0x6C)
                (if (i32.eq (local.get $imm0) (i32.const 0x6C))
                  (then (call $template_arm32_i32x4_lt_u (local.get $dec_ptr)) (br $fd_done)))
                ;; f32x4.lt (0x8E)
                (if (i32.eq (local.get $imm0) (i32.const 0x8E))
                  (then (call $template_arm32_f32x4_lt (local.get $dec_ptr)) (br $fd_done)))
                ;; f64x2.lt (0x9F)
                (if (i32.eq (local.get $imm0) (i32.const 0x9F))
                  (then (call $template_arm32_f64x2_lt (local.get $dec_ptr)) (br $fd_done)))
                ;; i8x16.gt_s (0x4D)
                (if (i32.eq (local.get $imm0) (i32.const 0x4D))
                  (then (call $template_arm32_i8x16_gt_s (local.get $dec_ptr)) (br $fd_done)))
                ;; i8x16.gt_u (0x4C)
                (if (i32.eq (local.get $imm0) (i32.const 0x4C))
                  (then (call $template_arm32_i8x16_gt_u (local.get $dec_ptr)) (br $fd_done)))
                ;; i16x8.gt_s (0x5E)
                (if (i32.eq (local.get $imm0) (i32.const 0x5E))
                  (then (call $template_arm32_i16x8_gt_s (local.get $dec_ptr)) (br $fd_done)))
                ;; i16x8.gt_u (0x5D)
                (if (i32.eq (local.get $imm0) (i32.const 0x5D))
                  (then (call $template_arm32_i16x8_gt_u (local.get $dec_ptr)) (br $fd_done)))
                ;; i32x4.gt_s (0x6F)
                (if (i32.eq (local.get $imm0) (i32.const 0x6F))
                  (then (call $template_arm32_i32x4_gt_s (local.get $dec_ptr)) (br $fd_done)))
                ;; i32x4.gt_u (0x6E)
                (if (i32.eq (local.get $imm0) (i32.const 0x6E))
                  (then (call $template_arm32_i32x4_gt_u (local.get $dec_ptr)) (br $fd_done)))
                ;; f32x4.gt (0x90)
                (if (i32.eq (local.get $imm0) (i32.const 0x90))
                  (then (call $template_arm32_f32x4_gt (local.get $dec_ptr)) (br $fd_done)))
                ;; f64x2.gt (0xA1)
                (if (i32.eq (local.get $imm0) (i32.const 0xA1))
                  (then (call $template_arm32_f64x2_gt (local.get $dec_ptr)) (br $fd_done)))
                ;; i8x16.le_s (0x4F)
                (if (i32.eq (local.get $imm0) (i32.const 0x4F))
                  (then (call $template_arm32_i8x16_le_s (local.get $dec_ptr)) (br $fd_done)))
                ;; i8x16.le_u (0x4E)
                (if (i32.eq (local.get $imm0) (i32.const 0x4E))
                  (then (call $template_arm32_i8x16_le_u (local.get $dec_ptr)) (br $fd_done)))
                ;; i16x8.le_s (0x60)
                (if (i32.eq (local.get $imm0) (i32.const 0x60))
                  (then (call $template_arm32_i16x8_le_s (local.get $dec_ptr)) (br $fd_done)))
                ;; i16x8.le_u (0x5F)
                (if (i32.eq (local.get $imm0) (i32.const 0x5F))
                  (then (call $template_arm32_i16x8_le_u (local.get $dec_ptr)) (br $fd_done)))
                ;; i32x4.le_s (0x71)
                (if (i32.eq (local.get $imm0) (i32.const 0x71))
                  (then (call $template_arm32_i32x4_le_s (local.get $dec_ptr)) (br $fd_done)))
                ;; i32x4.le_u (0x70)
                (if (i32.eq (local.get $imm0) (i32.const 0x70))
                  (then (call $template_arm32_i32x4_le_u (local.get $dec_ptr)) (br $fd_done)))
                ;; f32x4.le (0x92)
                (if (i32.eq (local.get $imm0) (i32.const 0x92))
                  (then (call $template_arm32_f32x4_le (local.get $dec_ptr)) (br $fd_done)))
                ;; f64x2.le (0xA3)
                (if (i32.eq (local.get $imm0) (i32.const 0xA3))
                  (then (call $template_arm32_f64x2_le (local.get $dec_ptr)) (br $fd_done)))
                ;; i8x16.ge_s (0x49)
                (if (i32.eq (local.get $imm0) (i32.const 0x49))
                  (then (call $template_arm32_i8x16_ge_s (local.get $dec_ptr)) (br $fd_done)))
                ;; i16x8.ge_s (0x5A)
                (if (i32.eq (local.get $imm0) (i32.const 0x5A))
                  (then (call $template_arm32_i16x8_ge_s (local.get $dec_ptr)) (br $fd_done)))
                ;; i32x4.ge_s (0x6B)
                (if (i32.eq (local.get $imm0) (i32.const 0x6B))
                  (then (call $template_arm32_i32x4_ge_s (local.get $dec_ptr)) (br $fd_done)))
                ;; f32x4.ge (0x94)
                (if (i32.eq (local.get $imm0) (i32.const 0x94))
                  (then (call $template_arm32_f32x4_ge (local.get $dec_ptr)) (br $fd_done)))
                ;; f64x2.ge (0xA5)
                (if (i32.eq (local.get $imm0) (i32.const 0xA5))
                  (then (call $template_arm32_f64x2_ge (local.get $dec_ptr)) (br $fd_done)))
                ;; i8x16.and (0x43)
                (if (i32.eq (local.get $imm0) (i32.const 0x43))
                  (then (call $template_arm32_i8x16_and (local.get $dec_ptr)) (br $fd_done)))
                ;; i8x16.or (0x44)
                (if (i32.eq (local.get $imm0) (i32.const 0x44))
                  (then (call $template_arm32_i8x16_or (local.get $dec_ptr)) (br $fd_done)))
                ;; i8x16.xor (0x45)
                (if (i32.eq (local.get $imm0) (i32.const 0x45))
                  (then (call $template_arm32_i8x16_xor (local.get $dec_ptr)) (br $fd_done)))
                ;; ── New canonical ops (0xA6+, not in old encoding) ──
                ;; v128.load8_splat (0xA6)
                (if (i32.eq (local.get $imm0) (i32.const 0xA6))
                  (then (call $template_arm32_v128_load8_splat (local.get $dec_ptr)) (br $fd_done)))
                ;; v128.load16_splat (0xA7)
                (if (i32.eq (local.get $imm0) (i32.const 0xA7))
                  (then (call $template_arm32_v128_load16_splat (local.get $dec_ptr)) (br $fd_done)))
                ;; v128.load32_splat (0xA8)
                (if (i32.eq (local.get $imm0) (i32.const 0xA8))
                  (then (call $template_arm32_v128_load32_splat (local.get $dec_ptr)) (br $fd_done)))
                ;; v128.load64_splat (0xA9)
                (if (i32.eq (local.get $imm0) (i32.const 0xA9))
                  (then (call $template_arm32_v128_load64_splat (local.get $dec_ptr)) (br $fd_done)))
                ;; i8x16.swizzle (0xAA)
                (if (i32.eq (local.get $imm0) (i32.const 0xAA))
                  (then (call $template_arm32_i8x16_swizzle (local.get $dec_ptr)) (br $fd_done)))
                ;; i64x2.replace_lane (0xAB)
                (if (i32.eq (local.get $imm0) (i32.const 0xAB))
                  (then (call $template_arm32_i64x2_replace_lane (local.get $dec_ptr)) (br $fd_done)))
                ;; v128.not (0xAC)
                (if (i32.eq (local.get $imm0) (i32.const 0xAC))
                  (then (call $template_arm32_v128_not (local.get $dec_ptr)) (br $fd_done)))
                ;; v128.bitselect (0xAD)
                (if (i32.eq (local.get $imm0) (i32.const 0xAD))
                  (then (call $template_arm32_v128_bitselect (local.get $dec_ptr)) (br $fd_done)))
                ;; i8x16.ge_u (0xAE)
                (if (i32.eq (local.get $imm0) (i32.const 0xAE))
                  (then (call $template_arm32_i8x16_ge_u (local.get $dec_ptr)) (br $fd_done)))
                ;; i16x8.ge_u (0xAF)
                (if (i32.eq (local.get $imm0) (i32.const 0xAF))
                  (then (call $template_arm32_i16x8_ge_u (local.get $dec_ptr)) (br $fd_done)))
                ;; i32x4.ge_u (0xB0)
                (if (i32.eq (local.get $imm0) (i32.const 0xB0))
                  (then (call $template_arm32_i32x4_ge_u (local.get $dec_ptr)) (br $fd_done)))
                ;; i8x16.min_u (0xB8)
                (if (i32.eq (local.get $imm0) (i32.const 0xB8))
                  (then (call $template_arm32_i8x16_min_u (local.get $dec_ptr)) (br $fd_done)))
                ;; i8x16.max_u (0xBA)
                (if (i32.eq (local.get $imm0) (i32.const 0xBA))
                  (then (call $template_arm32_i8x16_max_u (local.get $dec_ptr)) (br $fd_done)))
                ;; i8x16.avgr_u (0xBB)
                (if (i32.eq (local.get $imm0) (i32.const 0xBB))
                  (then (call $template_arm32_i8x16_avgr_u (local.get $dec_ptr)) (br $fd_done)))
                ;; i16x8.min_u (0xBC)
                (if (i32.eq (local.get $imm0) (i32.const 0xBC))
                  (then (call $template_arm32_i16x8_min_u (local.get $dec_ptr)) (br $fd_done)))
                ;; i16x8.max_u (0xBE)
                (if (i32.eq (local.get $imm0) (i32.const 0xBE))
                  (then (call $template_arm32_i16x8_max_u (local.get $dec_ptr)) (br $fd_done)))
                ;; i16x8.avgr_u (0xBF)
                (if (i32.eq (local.get $imm0) (i32.const 0xBF))
                  (then (call $template_arm32_i16x8_avgr_u (local.get $dec_ptr)) (br $fd_done)))
                ;; f32x4.min (0xE1)
                (if (i32.eq (local.get $imm0) (i32.const 0xE1))
                  (then (call $template_arm32_f32x4_min (local.get $dec_ptr)) (br $fd_done)))
                ;; f32x4.max (0xE2)
                (if (i32.eq (local.get $imm0) (i32.const 0xE2))
                  (then (call $template_arm32_f32x4_max (local.get $dec_ptr)) (br $fd_done)))
                ;; f64x2.min (0xE3)
                (if (i32.eq (local.get $imm0) (i32.const 0xE3))
                  (then (call $template_arm32_f64x2_min (local.get $dec_ptr)) (br $fd_done)))
                ;; f64x2.max (0xE4)
                (if (i32.eq (local.get $imm0) (i32.const 0xE4))
                  (then (call $template_arm32_f64x2_max (local.get $dec_ptr)) (br $fd_done)))
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
    (call $emit_arm32_epilogue)

    ;; ── Return code size ──────────────────────────────────────────
    (i32.load (global.get $JS_CODE_PTR))
  )
