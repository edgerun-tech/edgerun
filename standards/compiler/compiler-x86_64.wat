;; ═════════════════════════════════════════════════════════════════════
  ;; EdgeRun WASM → x86_64 JIT Compiler
  ;;
  ;; Compiles WASM decoded ops into x86_64 machine code in a memory buffer.
  ;; The generated code can be exported and executed natively by a host.
  ;;
  ;; Uses the same linear memory as the interpreter (imported).
  ;; ═════════════════════════════════════════════════════════════════════


  ;; ── Shared constants (from edgerun-core) ─────────────────────────────
  ;; ── x86_64 Linux syscall numbers (from asm/unistd_64.h) ──────────────
  (global $JIT_SLOT_SIZE  i32 (i32.const 0x40000))  ;; 256KB per function

  ;; JIT state at 0x200000 (after guest binary at 0x200000, so use 0x200000+
  ;; Actually guest binary starts at 0x200000. Let's use 0x300000 for JIT state.
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

  ;; Peephole optimization flag: set to 1 when result of current op
  ;; is left in eax instead of being pushed to the x86 stack.
  ;; The next op (e.g. local.set) will read from eax directly.
  (global $RESULT_IN_EAX     (mut i32) (i32.const 0))

  ;; Current decoded op pointer (set before each compile iteration dispatch)
  ;; Used by $emit_x86_maybe_push_rax for peephole lookahead
  (global $CURRENT_DEC_PTR   (mut i32) (i32.const 0))

  ;; ── Helper: write bytes to code cache ──────────────────────────────

  ;; Emit a single byte into the code cache, advance code_ptr

  ;; Emit a dword (4 bytes) little-endian

  ;; Emit a qword (8 bytes) little-endian

  ;; ── x86_64 ModRM / SIB emission ───────────────────────────────────

  ;; Emit ModRM byte: mod=2 bits, reg=3 bits, rm=3 bits
  )

  ;; Emit SIB byte: scale=2 bits, index=3 bits, base=3 bits
  )

  ;; Emit REX prefix byte
  ;; clobbers: W=64bit, R=extended reg, X=extended index, B=extended base
  )

  ;; Emit REX.W (64-bit operand size)

  ;; ── x86_64 instruction helpers ─────────────────────────────────────

  ;; mov rdx, imm64 (10 bytes: REX.W + B8+rdx + qword)

  ;; mov rax, imm64 (10 bytes: REX.W + B8+rax + qword)

  ;; mov rdi, imm64 (10 bytes)

  ;; mov rsi, rsp (3 bytes: 48 89 E6)

  ;; mov edx, imm32 (5 bytes: BA + dword)

  ;; mov eax, imm32 (5 bytes: B8 + dword) — implicitly zeros upper 32

  ;; mov ecx, imm32 (5 bytes: B9 + dword)

  ;; push rax (1 byte: 50)

  ;; push rcx (1 byte: 51)

  ;; push rdx (1 byte: 52)

  ;; push rbx (1 byte: 53)

  ;; push r15 (1 byte: 57, where reg=7)

  ;; pop rax (1 byte: 58)

  ;; pop rcx (1 byte: 59)

  ;; pop rdx (1 byte: 5A)

  ;; pop r15 (1 byte: 5F, where reg=7)

  ;; push imm32 (68 + dword)

  ;; pop reg: 58+reg (0=rax, 1=rcx, 2=rdx)

  ;; push reg: 50+reg (0=rax, 1=rcx, 2=rdx)

  ;; ret (1 byte: C3)

  ;; ud2 (2 bytes: 0F 0B)

  ;; int3 (1 byte: CC)

  ;; Push rax only if RESULT_IN_EAX is 0 (peephole: skip push when
  ;; next op will consume from eax directly).
  ;; Peephole lookahead: if the next decoded op is local.set/tee with
  ;; register-allocated local (idx 0 or 1), leave result in eax and
  ;; set RESULT_IN_EAX flag instead of pushing.
          (else
            (call $emit_x86_byte (i32.const 0x50))
          )
        )
      )
    )
  )

  ;; xor eax, eax (2 bytes: 31 C0)

  ;; xor edx, edx (2 bytes: 31 D2)

  ;; xor rdx, rdx (REX.W + 31 D2 = 48 31 D2)

  ;; xor ecx, ecx (2 bytes: 31 C9)

  ;; cdq (1 byte: 99) — sign-extend eax into edx:eax

  ;; cqo (REX.W + 99) — sign-extend rax into rdx:rax

  ;; test eax, eax (2 bytes: 85 C0)

  ;; test edx, edx (2 bytes: 85 D2)

  ;; test ecx, ecx (2 bytes: 85 C9)

  ;; mov edx, eax (2 bytes: 89 C2)

  ;; mov rdx, rax (3 bytes: 48 89 C2)

  ;; ── i32 binary arithmetic (pop rcx, pop rax, op, push rax) ──────────

  ;; pop rcx, pop rax

  ;; add eax, ecx (2 bytes: 01 C8)

  ;; sub eax, ecx (2 bytes: 29 C8)

  ;; imul eax, ecx (3 bytes: 0F AF C1)

  ;; idiv ecx (2 bytes: F7 F9) — divides edx:eax by ecx

  ;; div ecx (2 bytes: F7 F1) — divides edx:eax by ecx

  ;; and eax, ecx (2 bytes: 21 C8)

  ;; or eax, ecx (2 bytes: 09 C8)

  ;; xor eax, ecx (2 bytes: 31 C8)

  ;; shl eax, cl (2 bytes: D3 E0)

  ;; shr eax, cl (2 bytes: D3 E8)

  ;; sar eax, cl (2 bytes: D3 F8)

  ;; rol eax, cl (2 bytes: D3 C0)

  ;; ror eax, cl (2 bytes: D3 C8)

  ;; btr eax, imm8 (4 bytes: 0F BA F0 imm8) — clear bit of eax

  ;; btr rax, imm8 (4 bytes: 48 0F BA F0 imm8) — clear bit of rax

  ;; cmp eax, ecx (2 bytes: 39 C8)

  ;; mov eax, edx (2 bytes: 89 D0) — accumulate remainder

  ;; mov rax, rdx (REX.W + 89 D0 = 48 89 D0)

  ;; movzx eax, al (3 bytes: 0F B6 C0)

  ;; setcc: 0F 9x+cl (cl = setX opcode byte)
  ;; cl values: 94=sete, 95=setne, 92=setb, 97=seta, 96=setbe, 93=setae,
  ;;            9C=setl, 9F=setg, 9E=setle, 9D=setge

  ;; ── x86_64 i64 (REX.W) helpers ────────────────────────────────────

  ;; REX.W + add rax, rcx (3 bytes: 48 01 C8)

  ;; REX.W + sub rax, rcx

  ;; REX.W + imul rax, rcx (4 bytes: 48 0F AF C1)

  ;; REX.W + idiv rcx

  ;; REX.W + div rcx

  ;; REX.W + and rax, rcx

  ;; REX.W + or rax, rcx

  ;; REX.W + xor rax, rcx

  ;; REX.W + shl rax, cl

  ;; REX.W + shr rax, cl

  ;; REX.W + sar rax, cl

  ;; REX.W + rol rax, cl

  ;; REX.W + ror rax, cl

  ;; REX.W + cmp rax, rcx

  ;; REX.W + movsxd rax, eax (4 bytes: 48 63 C0)

  ;; mov eax, eax (2 bytes: 89 C0) — zero-extend eax to rax

  ;; cmove rdx, rcx (64-bit): if ZF=1, rdx = rcx (4 bytes: 48 0F 44 CA)

  ;; cmovns eax, edx (3 bytes: 0F 49 D0) — eax = edx if SF=0

  ;; cmovns eax, ecx (3 bytes: 0F 49 C1) — eax = ecx if SF=0

  ;; cmovns rax, rdx (4 bytes: 48 0F 49 D0) — rax = rdx if SF=0

  ;; mov ecx, eax (2 bytes: 89 C1)

  ;; mov eax, ecx (2 bytes: 89 C8)

  ;; mov rax, rcx (3 bytes: 48 89 C8) — dest=rax, src=rcx

  ;; mov rcx, rax (3 bytes: 48 89 C1) — dest=rcx, src=rax

  ;; sar ecx, 31 (3 bytes: C1 F9 1F)

  ;; sar rcx, 63 (4 bytes: 48 C1 F9 3F)

  ;; and ecx, imm32 (6 bytes: 81 E1 dword)

  ;; shl rax, imm8 (4 bytes: 48 C1 E0 imm8)

  ;; ── i32 unary helpers (lzcnt, tzcnt, popcnt) ───────────────────────

  ;; lzcnt eax, eax (4 bytes: F3 0F BD C0)

  ;; tzcnt eax, eax (4 bytes: F3 0F BC C0)

  ;; popcnt eax, eax (4 bytes: F3 0F B8 C0)

  ;; REX.W + lzcnt rax, rax (5 bytes: F3 48 0F BD C0)

  ;; REX.W + tzcnt rax, rax

  ;; REX.W + popcnt rax, rax

  ;; ── i32 immediate arithmetic (add/sub/imul/and/or/xor eax, imm32) ──

  ;; add eax, imm32 (5 bytes: 05 + dword)

  ;; sub eax, imm32 (5 bytes: 2D + dword)

  ;; imul eax, eax, imm32 (6 bytes: 69 C0 + dword)

  ;; and eax, imm32 (5 bytes: 25 + dword)

  ;; or eax, imm32 (5 bytes: 0D + dword)

  ;; xor eax, imm32 (5 bytes: 35 + dword)

  ;; shl eax, imm8 (3 bytes: C1 E0 imm8)

  ;; shr eax, imm8 (3 bytes: C1 E8 imm8)

  ;; sar eax, imm8 (3 bytes: C1 F8 imm8)

  ;; rol eax, imm8 (3 bytes: C1 C0 imm8)

  ;; ror eax, imm8 (3 bytes: C1 C8 imm8)

  ;; ── Call / sub rsp / add rsp helpers ──────────────────────────────

  ;; sub rsp, imm32 (6 bytes: 48 81 EC dword) — allocate stack space

  ;; add rsp, imm32 (6 bytes: 48 81 C4 dword)

  ;; call rax (2 bytes: FF D0)

  ;; jmp rel8 (2 bytes: EB disp8) — short jump

  ;; jmp rel32 (5 bytes: E9 dword)

  ;; jz rel32 (6 bytes: 0F 84 dword)

  ;; jne rel32 (6 bytes: 0F 85 dword)

  ;; ── rel8 conditional jumps ──

  ;; jp rel8 (2 bytes: 7A disp8) — jump if parity (PF=1, NaN after ucomiss)

  ;; jnp rel8 (2 bytes: 7B disp8) — jump if not parity (PF=0, ordered after ucomiss)

  ;; jae rel8 (2 bytes: 73 disp8) — jump if above or equal (CF=0)

  ;; jb rel8 (2 bytes: 72 disp8) — jump if below (CF=1)

  ;; ── Load/store helpers (memory through JitGlobals) ─────────────────

  ;; Load [r15 + offset] into rdx (REX.W + 8B 97 + dword)
  ;; The JIT'd code expects r15 to point to a JitGlobals struct

  ;; Store rax to [rdx + disp32] (REX.W + 89 82 + dword)

  ;; Load [rdx + disp32] into rax (REX.W + 8B 82 + dword)

  ;; mov rbx, [r15] — cache JitGlobals.locals ptr in rbx (49 8B 9F 00 00 00 00)

  ;; mov rax, [rbx + disp32] — load local via cached base ptr (48 8B 83 xx xx xx xx)

  ;; mov [rbx + disp32], rax — store local via cached base ptr (48 89 83 xx xx xx xx)

  ;; ── Register-allocated local helpers (r12 = local 0, r13 = local 1) ──

  ;; push r12 (41 54)

  ;; push r13 (41 55)

  ;; pop r12 (41 5C)

  ;; pop r13 (41 5D)

  ;; mov r12, [rbx] — load locals[0] into r12 (4C 8B 23)

  ;; mov r13, [rbx + 8] — load locals[1] into r13 (4C 8B 6B 08)

  ;; mov [rbx], r12 — store r12 to locals[0] (4C 89 23)

  ;; mov [rbx + 8], r13 — store r13 to locals[1] (4C 89 6B 08)

  ;; mov r12, rax — store eax to register-allocated local 0 (49 89 C4)

  ;; mov r13, rax — store eax to register-allocated local 1 (49 89 C5)

  ;; Load [rdx + disp32] into rcx (REX.W + 8B 8A + dword)

  ;; Store rax to [rsp + disp32] (REX.W + 89 84 24 + dword)

  ;; Load rax from [rsp + disp32]

  ;; ── Memory load/store (through JitGlobals.mem_ptr) ─────────────────

  ;; i32.load: pop rcx (addr), mov rdx, [r15 + mem_ptr], mov eax, [rdx + rcx]
  ;; Actually: the original uses a register for mem_ptr. Emit: 
  ;;   mov rdx, [r15 + JitGlobals.mem_ptr]
  ;;   mov eax, [rdx + rcx*1]

  ;; i64.load: REX.W mov rax, [rdx + rcx]

  ;; i32.store: pop rcx (addr), pop rax (val), mov rdx, [r15 + mem_ptr], mov [rdx+rcx], eax

  ;; i64.store: REX.W mov [rdx+rcx], rax

  ;; i32.load8_s: movsx eax, byte [rdx+rcx]  (0F BE 04 0A)

  ;; i32.load8_u: movzx eax, byte [rdx+rcx]  (0F B6 04 0A)

  ;; i32.load16_s: movsx eax, word [rdx+rcx]  (0F BF 04 0A)

  ;; i32.load16_u: movzx eax, word [rdx+rcx]  (0F B7 04 0A)

  ;; i64.load8_s: REX.W movsx rax, byte [rdx+rcx]  (48 0F BE 04 0A)

  ;; i64.load16_s: REX.W movsx rax, word [rdx+rcx]  (48 0F BF 04 0A)

  ;; i64.load32_s: REX.W movsxd rax, dword [rdx+rcx]  (48 63 04 0A)

  ;; i32.store8: mov [rdx+rcx], al  (88 04 0A)

  ;; i32.store16: mov [rdx+rcx], ax  (66 89 04 0A)

  ;; ── SSE helpers ────────────────────────────────────────────────────

  ;; movd xmm0, eax (3 bytes: 66 0F 6E C0)

  ;; movd xmm1, ecx

  ;; movd eax, xmm0 (3 bytes: 66 0F 7E C0)

  ;; movq xmm0, rax (REX.W + 66 0F 6E C0)

  ;; movq xmm1, rcx

  ;; movq xmm1, rax (REX.W + 66 0F 6E C8)

  ;; movq rax, xmm0 (REX.W + 66 0F 7E C0)

  ;; ── SSE integer/float conversion helpers ─────────────────────────────

  ;; cvtsi2ss xmm0, eax (i32→f32): F3 0F 2A C0

  ;; cvtsi2sd xmm0, eax (i32→f64): F2 0F 2A C0

  ;; cvtsi2ss xmm0, rax (i64→f32): F3 48 0F 2A C0

  ;; cvtsi2sd xmm0, rax (i64→f64): F2 48 0F 2A C0

  ;; cvttss2si eax, xmm0 (f32→i32): F3 0F 2C C0

  ;; cvttsd2si eax, xmm0 (f64→i32): F2 0F 2C C0

  ;; cvttss2si rax, xmm0 (f32→i64): F3 48 0F 2C C0

  ;; cvttsd2si rax, xmm0 (f64→i64): F2 48 0F 2C C0

  ;; cvtss2sd xmm0, xmm0 (f32→f64 promote): F3 0F 5A C0

  ;; cvtsd2ss xmm0, xmm0 (f64→f32 demote): F2 0F 5A C0

  ;; SSE binary op: F3(prefix) + 0F + opcode + modrm(C1 = xmm0, xmm1)
  ;; prefix = 0xF3 for f32 or 0xF2 for f64
  ;; ch = opcode byte (e.g., 0x58=add, 0x5C=sub, 0x59=mul, 0x5E=div)

  ;; SSE3A op (roundss/roundsd): 66 + 0F + 3A + opcode + modrm + imm8

  ;; ucomiss xmm0, xmm1 (4 bytes: 0F 2E C1) — compare f32 scalars

  ;; ucomiss xmm0, xmm0 (4 bytes: 0F 2E C0) — self-compare for NaN check

  ;; ucomisd xmm0, xmm1 (5 bytes: 66 0F 2E C1)

  ;; ucomisd xmm0, xmm0 (5 bytes: 66 0F 2E C0) — self-compare for NaN check

  ;; ── f32 unary helpers ──────────────────────────────────────────────

  ;; pop rax, movd xmm0, eax

  ;; movd eax, xmm0, push rax

  ;; ── f64 unary helpers ──────────────────────────────────────────────

  ;; pop rax, movq xmm0, rax

  ;; movq rax, xmm0, push rax

  ;; ── Float comparison helpers ───────────────────────────────────────

  ;; f32 cmp prologue: pop rcx, pop rax, movd xmm1, movd xmm0, ucomiss

  ;; f64 cmp prologue: pop rcx, pop rax, movq xmm1, movq xmm0, ucomisd

  ;; setnp al (0F 9B C0) — mov ah, al for NaN fixup

  ;; setp al (0F 9A C0) — mov ah, al (for NaN in ne)

  ;; setcc (cl=opcode), and al, ah, movzx eax, al, push rax
  ;; Used for NaN-safe eq/lt/le where result must be 0 on NaN

  ;; setp al, setne al, or al, ah, movzx, push (for NaN-safe ne)

  ;; emit get_current_code_ptr → i32 (offset from cache base)

  ;; set current code ptr

  ;; ── Function prologue ──────────────────────────────────────────────

  ;; Prologue: push rbp; mov rbp, rsp

  ;; Epilogue: pop rbp; ret

  ;; ── JIT state helpers ─────────────────────────────────────────────

  ;; Init all JIT state to known values

  ;; Get word at decoded_op_ptr + offset

  ;; (export "get_code" signature TBD)


  ;; ═════════════════════════════════════════════════════════════════════
  ;; Opcode templates — each emits x86_64 code for one WASM opcode
  ;; Caller sets: rdi = decoded_op_ptr before calling template
  ;; ═════════════════════════════════════════════════════════════════════

  ;; ── unreachable (0x00): ud2 ────────────────────────────────────────

  ;; ── nop (0x01): nothing ────────────────────────────────────────────

  ;; ── select (0x1B): pop cond, pop val2, pop val1; push val1 if cond!=0 else val2 ──

  ;; ── i32.const (0x41): push imm32 ──────────────────────────────────

  ;; ── i64.const (0x42): push imm64 ──────────────────────────────────

  ;; ── local.get (0x20): mov rax, [rdx + index*8]; push rax ──────────
        )
      )
    )
  )

  ;; ── local.set (0x21): pop rax/reg; store to locals ──────────────────
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

  ;; ── global.set (0x24) ─────────────────────────────────────────────

  ;; ── table.get (0x25) ─────────────────────────────────────────────

  ;; ── table.set (0x26) ─────────────────────────────────────────────

  ;; ── i32 comparison templates ──────────────────────────────────────

  ;; i32.eqz (0x45) — pop rax, test eax,eax, sete al, movzx, push rax

  ;; i32.eq (0x46) — pop rcx, pop rax, cmp, sete, push

  ;; i32.ne (0x47)

  ;; i32.lt_s (0x48)

  ;; i32.lt_u (0x49)

  ;; i32.gt_s (0x4A)

  ;; i32.gt_u (0x4B)

  ;; i32.le_s (0x4C)

  ;; i32.le_u (0x4D)

  ;; i32.ge_s (0x4E)

  ;; i32.ge_u (0x4F)

  ;; ── i32 binary arithmetic templates ───────────────────────────────
















  ;; ── i64 binary arithmetic templates ───────────────────────────────




  ;; i64.div_s (0x7F) — pop rcx, pop rax, cqo, idiv rcx

  ;; i64.div_u (0x80)

  ;; i64.rem_s (0x81)

  ;; i64.rem_u (0x82)

  ;; i64.and (0x83), or (0x84), xor (0x85)



  ;; i64.shl (0x86), shr_s (0x87), shr_u (0x88), rotl (0x89), rotr (0x8A)





  ;; ── i64 unary templates ──────────────────────────────────────────

  ;; i64.clz (0x79)

  ;; i64.ctz (0x7A)

  ;; i64.popcnt (0x7B)

  ;; ── i64 comparison templates ─────────────────────────────────────

  ;; i64.eqz (0x50) — test rax, rax; sete; movzx

  ;; i64.eq (0x51), ne (0x52), lt_s (0x53), lt_u (0x54)




  ;; i64.gt_s (0x55), gt_u (0x56), le_s (0x57), le_u (0x58)




  ;; i64.ge_s (0x59), ge_u (0x5A)


  ;; ── i32 unary templates ──────────────────────────────────────────




  ;; ── Sign extension templates ──────────────────────────────────────



  ;; i64.extend8_s (0xC2): movsx rax, al (REX.W + 0F BE C0)

  ;; i64.extend16_s (0xC3): movsx rax, ax (REX.W + 0F BF C0)

  ;; i64.extend32_s (0xC4): movsxd rax, eax (48 63 C0)

  ;; ── Conversion templates ─────────────────────────────────────────




  ;; ── Float conversion templates ─────────────────────────────────────

  ;; f32.convert_i32_s (0xB2)

  ;; f32.convert_i64_s (0xB4)

  ;; f32.convert_i32_u (0xB3) — branch-free unsigned i32→f32

  ;; f32.convert_i64_u (0xB5) — branch-free unsigned i64→f32

  ;; f64.convert_i32_s (0xB7)

  ;; f64.convert_i64_s (0xB9)

  ;; f64.convert_i32_u (0xB8) — branch-free unsigned i32→f64

  ;; f64.convert_i64_u (0xBA) — branch-free unsigned i64→f64

  ;; i32.trunc_f32_s (0xA8)

  ;; i32.trunc_f64_s (0xAA)

  ;; i32.trunc_f32_u (0xA9) — branch-free with cmovns

  ;; i32.trunc_f64_u (0xAB) — branch-free with cmovns

  ;; i64.trunc_f32_s (0xAE)

  ;; i64.trunc_f64_s (0xB0)

  ;; i64.trunc_f32_u (0xAF) — branch-free with cmovns

  ;; i64.trunc_f64_u (0xB1) — branch-free with cmovns

  ;; f32.demote_f64 (0xB6)

  ;; f64.promote_f32 (0xBB)

  ;; ── Saturating truncation templates (0xFC prefix) ─────────────────

  ;; Helper: patch a rel8 displacement at position (addr-1)
  )

  ;; Helper: load 4-byte constant into xmm1 (push_imm32 + pop + movd xmm1)

  ;; Helper: load 8-byte constant into xmm1 (mov rax, imm64 + movq xmm1)

  ;; i32.trunc_sat_f32_s (0xFC, 0x00)

  ;; i32.trunc_sat_f32_u (0xFC, 0x01)

  ;; i32.trunc_sat_f64_s (0xFC, 0x02)

  ;; i32.trunc_sat_f64_u (0xFC, 0x03)

  ;; i64.trunc_sat_f32_s (0xFC, 0x04)

  ;; i64.trunc_sat_f32_u (0xFC, 0x05)

  ;; i64.trunc_sat_f64_s (0xFC, 0x06)

  ;; i64.trunc_sat_f64_u (0xFC, 0x07)

  ;; Reinterpret ops (0xBC-0xBF) — no-ops on the JIT stack (same bits)

  ;; f64.load (0x2B)

  ;; f32.store (0x38)

  ;; f64.store (0x39)

  ;; ── Memory load/store templates ──────────────────────────────────











  ;; i64.load8_s (0x30)

  ;; i64.load8_u (0x31)

  ;; i64.load16_s (0x32)

  ;; i64.load16_u (0x33)

  ;; i64.load32_s (0x34)

  ;; i64.load32_u (0x35)

  ;; i64.store8 (0x3C)

  ;; i64.store16 (0x3D)

  ;; i64.store32 (0x3E)

  ;; ── memory.size (0x3F) ──────────────────────────────────────────

  ;; memory.grow (0x40)

  ;; ref.null (0xD0): push 0 (null reference)

  ;; ref.is_null (0xD1): pop, test if zero, push i32 result

  ;; ref.func (0xD2): push function reference (from imm0)


  ;; ── Call / import syscall template ─────────────────────────────────

  ;; call (0x10): if func_idx < import_count, emit inline syscall.
  ;; Otherwise emit unsupported (no multi-function JIT yet).
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

        ;; Emit: allocate 48-byte struct
        (call $emit_x86_sub_rsp_imm (i32.const 48))

        ;; Zero-fill all 6 slots: xor eax,eax; store at each offset
        (call $emit_x86_xor_eax_eax)
        (local.set $i (i32.const 0))
        (block $zfill
          (loop $zloop
            (if (i32.eq (local.get $i) (i32.const 6)) (then (br $zfill)))
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

        ;; Set syscall number in eax
        (call $emit_mov_eax_imm (local.get $sysno))

        ;; Load syscall regs from struct
        (call $emit_load_rdi_rsp_disp (i32.const 0))
        (call $emit_load_rsi_rsp_disp (i32.const 8))
        (call $emit_load_rdx_rsp_disp (i32.const 16))
        (call $emit_load_r10_rsp_disp (i32.const 24))
        (call $emit_load_r8_rsp_disp (i32.const 32))
        (call $emit_load_r9_rsp_disp (i32.const 40))

        ;; syscall
        (call $emit_syscall)

        ;; Deallocate struct
        (call $emit_x86_add_rsp_imm (i32.const 48))

        ;; Push return value onto WASM stack
        (call $emit_x86_byte (i32.const 0x50))  ;; push rax
      )
      (else
        ;; Local function — not yet supported (would need multi-function JIT)
        (call $template_x86_unsupported)
      )
    )
  )

  ;; ── Float arithmetic templates ───────────────────────────────────

  ;; f32.add: pop rcx, pop rax, movd xmm1,ecx, movd xmm0,eax, addss, movd, push




  ;; f32.min (0x96): minss + NaN fixup

  ;; f32.max (0x97): maxss + NaN fixup

  ;; f32.copysign (0x98): result = abs(left) | signbit(right)

  ;; ── f64 binary templates ──────────────────────────────────────────





  ;; f64.min (0xA4): minsd + NaN fixup

  ;; f64.max (0xA5): maxsd + NaN fixup

  ;; f64.copysign (0xA6): result = abs(left) | signbit(right)

  ;; ── f32 comparison templates ──────────────────────────────────────







  ;; ── f64 comparison templates ──────────────────────────────────────







  ;; ── Float unary templates ────────────────────────────────────────








  ;; ── f32.const (0x43): push 4-byte constant ──────────────────────────

  ;; ── f64.const (0x44): push 8-byte constant ──────────────────────────








  ;; ═════════════════════════════════════════════════════════════════════
  ;; Label / fixup helpers (for control flow compilation)
  ;; ═════════════════════════════════════════════════════════════════════








  ;; ── Fixup helpers ─────────────────────────────────────────────────


  ;; Patch fixups at given label depth: compute forward jump displacement
  ;; and write it at each fixup offset
            ;; Remove fixup by swapping with last
            (local.set $count (i32.sub (local.get $count) (i32.const 1)))
            (i32.store (i32.add (global.get $JS_FIXUP_LABEL) (i32.shl (local.get $i) (i32.const 2)))
              (i32.load (i32.add (global.get $JS_FIXUP_LABEL) (i32.shl (local.get $count) (i32.const 2)))))
            (i32.store (i32.add (global.get $JS_FIXUP_OFFSET) (i32.shl (local.get $i) (i32.const 2)))
              (i32.load (i32.add (global.get $JS_FIXUP_OFFSET) (i32.shl (local.get $count) (i32.const 2)))))
            (i32.store (global.get $JS_FIXUP_COUNT) (local.get $count))
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

  (func $emit_x86_mov_rax_imm64_val (param $v i64)
    (call $emit_x86_rex_w)
    (call $emit_x86_byte (i32.const 0xB8))
    (call $emit_x86_qword (local.get $v))
  )

  ;; sub rsp, 16; movdqu [rsp], xmm0 — push 128-bit value onto native stack

  ;; movdqu xmm0, [rsp]; add rsp, 16 — pop 128-bit value into xmm0

  ;; movdqu xmm1, [rsp]; add rsp, 16 — pop 128-bit value into xmm1
  (func $emit_x86_v128_pop_xmm1
    (call $emit_x86_byte (i32.const 0xF3))
    (call $emit_x86_byte (i32.const 0x0F))
    (call $emit_x86_byte (i32.const 0x6F))
    (call $emit_x86_modrm (i32.const 0) (i32.const 1) (i32.const 4))
    (call $emit_x86_sib (i32.const 0) (i32.const 4) (i32.const 4))
    (call $emit_x86_rex_w)
    (call $emit_x86_byte (i32.const 0x83))
    (call $emit_x86_modrm (i32.const 3) (i32.const 0) (i32.const 4))
    (call $emit_x86_byte (i32.const 16))
  )

  ;; movdqu xmm0, [rax] — load 128-bit from address in rax
  (func $emit_movdqu_xmm0_rax
    (call $emit_x86_byte (i32.const 0xF3))
    (call $emit_x86_byte (i32.const 0x0F))
    (call $emit_x86_byte (i32.const 0x6F))
    (call $emit_x86_modrm (i32.const 0) (i32.const 0) (i32.const 0))
  )

  ;; movdqu [rax], xmm0 — store 128-bit to address in rax
  (func $emit_movdqu_rax_xmm0
    (call $emit_x86_byte (i32.const 0xF3))
    (call $emit_x86_byte (i32.const 0x0F))
    (call $emit_x86_byte (i32.const 0x7F))
    (call $emit_x86_modrm (i32.const 0) (i32.const 0) (i32.const 0))
  )

  ;; Helper: pop two v128 values (xmm1, xmm0), do op on xmm0,xmm1, push result
  ;; call (param $prefix i32) (param $opcode i32) — emits prefix 0F opcode C1
  (func $emit_sse128_binop (param $prefix i32) (param $opcode i32)
    (call $emit_x86_v128_pop_xmm1)
    (call $emit_x86_v128_pop)
    (call $emit_x86_byte (local.get $prefix))
    (call $emit_x86_byte (i32.const 0x0F))
    (call $emit_x86_byte (local.get $opcode))
    (call $emit_x86_modrm (i32.const 3) (i32.const 0) (i32.const 1))
    (call $emit_x86_v128_push)
  )

  ;; SSE2 128-bit integer binop: 66 + 0F + opcode + modrm(3,0,1)
  (func $emit_sse128_int_binop (param $opcode i32)
    (call $emit_x86_v128_pop_xmm1)
    (call $emit_x86_v128_pop)
    (call $emit_x86_byte (i32.const 0x66))
    (call $emit_x86_byte (i32.const 0x0F))
    (call $emit_x86_byte (local.get $opcode))
    (call $emit_x86_modrm (i32.const 3) (i32.const 0) (i32.const 1))
    (call $emit_x86_v128_push)
  )

  ;; SSE4.1 128-bit integer binop: 66 + 0F + 38 + opcode + modrm(3,0,1)
  (func $emit_sse41_int_binop (param $opcode i32)
    (call $emit_x86_v128_pop_xmm1)
    (call $emit_x86_v128_pop)
    (call $emit_x86_byte (i32.const 0x66))
    (call $emit_x86_byte (i32.const 0x0F))
    (call $emit_x86_byte (i32.const 0x38))
    (call $emit_x86_byte (local.get $opcode))
    (call $emit_x86_modrm (i32.const 3) (i32.const 0) (i32.const 1))
    (call $emit_x86_v128_push)
  )

  ;; SSE float compare: CMPPS/CMPPD with imm8 — pops two v128, does cmp, pushes
  (func $emit_sse128_cmp (param $prefix i32) (param $imm8 i32)
    (call $emit_x86_v128_pop_xmm1)
    (call $emit_x86_v128_pop)
    (call $emit_x86_byte (local.get $prefix))
    (call $emit_x86_byte (i32.const 0x0F))
    (call $emit_x86_byte (i32.const 0xC2))
    (call $emit_x86_modrm (i32.const 3) (i32.const 0) (i32.const 1))
    (call $emit_x86_byte (local.get $imm8))
    (call $emit_x86_v128_push)
  )

  ;; NOT helper: pxor xmm0, xmm1 where xmm1 = all-ones (PCMPEQD self)
  ;; Used after loading operand into xmm0
  (func $emit_x86_v128_not
    (call $emit_x86_byte (i32.const 0x66))
    (call $emit_x86_byte (i32.const 0x0F))
    (call $emit_x86_byte (i32.const 0x76))
    (call $emit_x86_modrm (i32.const 3) (i32.const 1) (i32.const 1))
    (call $emit_x86_byte (i32.const 0x66))
    (call $emit_x86_byte (i32.const 0x0F))
    (call $emit_x86_byte (i32.const 0xEF))
    (call $emit_x86_modrm (i32.const 3) (i32.const 0) (i32.const 1))
  )

  ;; ═════════════════════════════════════════════════════════════════════
  ;; SIMD template functions (v128.wat — x86-64 SSE/SSE4.1)
  ;; ═════════════════════════════════════════════════════════════════════

  ;; ── v128.load (0xFD, 0x00) ────────────────────────────────────────
  ;; pop i32 address → load 16 bytes → push v128
  (func $template_simd_v128_load (param $dec_ptr i32)
    (call $emit_x86_pop_reg (i32.const 0))
    (call $emit_movdqu_xmm0_rax)
    (call $emit_x86_v128_push)
  )

  ;; ── v128.store (0xFD, 0x0B) ───────────────────────────────────────
  ;; pop i32 address, pop v128 → store 16 bytes
  (func $template_simd_v128_store (param $dec_ptr i32)
    (call $emit_x86_pop_reg (i32.const 0))
    (call $emit_x86_v128_pop)
    (call $emit_movdqu_rax_xmm0)
  )

  ;; ── v128.const (0xFD, 0x0C) ───────────────────────────────────────
  ;; 16 immediate bytes at dec_ptr+16; load into xmm0 and push
  (func $template_simd_v128_const (param $dec_ptr i32)
    (local $lo i64) (local $hi i64)
    (local.set $lo (i64.load (i32.add (local.get $dec_ptr) (i32.const 16))))
    (local.set $hi (i64.load (i32.add (local.get $dec_ptr) (i32.const 24))))
    (call $emit_x86_mov_rax_imm64_val (local.get $lo))
    (call $emit_x86_movq_xmm0_rax)
    (call $emit_x86_mov_rax_imm64_val (local.get $hi))
    (call $emit_x86_movq_xmm1_rax)
    (call $emit_x86_byte (i32.const 0x66))
    (call $emit_x86_byte (i32.const 0x0F))
    (call $emit_x86_byte (i32.const 0x6C))
    (call $emit_x86_modrm (i32.const 3) (i32.const 0) (i32.const 1))
    (call $emit_x86_v128_push)
  )

  ;; ── v128.and / or / xor / not ─────────────────────────────────────
  (func $template_simd_v128_and (param $dec_ptr i32)
    (call $emit_sse128_int_binop (i32.const 0xDB))
  )
  (func $template_simd_v128_or (param $dec_ptr i32)
    (call $emit_sse128_int_binop (i32.const 0xEB))
  )
  (func $template_simd_v128_xor (param $dec_ptr i32)
    (call $emit_sse128_int_binop (i32.const 0xEF))
  )
  (func $template_simd_v128_not (param $dec_ptr i32)
    (call $emit_x86_v128_pop)
    (call $emit_x86_v128_not)
    (call $emit_x86_v128_push)
  )

  ;; ── Splats (scalar → v128 broadcast) ──────────────────────────────

  ;; i8x16.splat (0xFD, 0x25): pop i32 → broadcast byte to all 16 lanes
  (func $template_simd_i8x16_splat (param $dec_ptr i32)
    (call $emit_x86_pop_reg (i32.const 0))
    (call $emit_x86_movd_xmm0_eax)
    (call $emit_x86_byte (i32.const 0x66)) (call $emit_x86_byte (i32.const 0x0F)) (call $emit_x86_byte (i32.const 0x60))
    (call $emit_x86_modrm (i32.const 3) (i32.const 0) (i32.const 0))
    (call $emit_x86_byte (i32.const 0x66)) (call $emit_x86_byte (i32.const 0x0F)) (call $emit_x86_byte (i32.const 0x61))
    (call $emit_x86_modrm (i32.const 3) (i32.const 0) (i32.const 0))
    (call $emit_x86_byte (i32.const 0x66)) (call $emit_x86_byte (i32.const 0x0F)) (call $emit_x86_byte (i32.const 0x70))
    (call $emit_x86_modrm (i32.const 3) (i32.const 0) (i32.const 0))
    (call $emit_x86_byte (i32.const 0))
    (call $emit_x86_v128_push)
  )

  ;; i16x8.splat (0xFD, 0x26): pop i32 → broadcast word to all 8 lanes
  (func $template_simd_i16x8_splat (param $dec_ptr i32)
    (call $emit_x86_pop_reg (i32.const 0))
    (call $emit_x86_movd_xmm0_eax)
    (call $emit_x86_byte (i32.const 0x66)) (call $emit_x86_byte (i32.const 0x0F)) (call $emit_x86_byte (i32.const 0x61))
    (call $emit_x86_modrm (i32.const 3) (i32.const 0) (i32.const 0))
    (call $emit_x86_byte (i32.const 0x66)) (call $emit_x86_byte (i32.const 0x0F)) (call $emit_x86_byte (i32.const 0x70))
    (call $emit_x86_modrm (i32.const 3) (i32.const 0) (i32.const 0))
    (call $emit_x86_byte (i32.const 0))
    (call $emit_x86_v128_push)
  )

  ;; i32x4.splat (0xFD, 0x27): pop i32 → broadcast dword to all 4 lanes via PSHUFD
  (func $template_simd_i32x4_splat (param $dec_ptr i32)
    (call $emit_x86_pop_reg (i32.const 0))
    (call $emit_x86_movd_xmm0_eax)
    (call $emit_x86_byte (i32.const 0x66)) (call $emit_x86_byte (i32.const 0x0F)) (call $emit_x86_byte (i32.const 0x70))
    (call $emit_x86_modrm (i32.const 3) (i32.const 0) (i32.const 0))
    (call $emit_x86_byte (i32.const 0))
    (call $emit_x86_v128_push)
  )

  ;; i64x2.splat (0xFD, 0x28): pop i64 → broadcast qword to both lanes
  (func $template_simd_i64x2_splat (param $dec_ptr i32)
    (call $emit_x86_pop_reg (i32.const 0))
    (call $emit_x86_movq_xmm0_rax)
    (call $emit_x86_byte (i32.const 0x66)) (call $emit_x86_byte (i32.const 0x0F)) (call $emit_x86_byte (i32.const 0x6C))
    (call $emit_x86_modrm (i32.const 3) (i32.const 0) (i32.const 0))
    (call $emit_x86_v128_push)
  )

  ;; f32x4.splat (0xFD, 0x29): pop f32 bit pattern → broadcast to all 4 lanes
  (func $template_simd_f32x4_splat (param $dec_ptr i32)
    (call $emit_x86_pop_reg (i32.const 0))
    (call $emit_x86_movd_xmm0_eax)
    (call $emit_x86_byte (i32.const 0x66)) (call $emit_x86_byte (i32.const 0x0F)) (call $emit_x86_byte (i32.const 0x70))
    (call $emit_x86_modrm (i32.const 3) (i32.const 0) (i32.const 0))
    (call $emit_x86_byte (i32.const 0))
    (call $emit_x86_v128_push)
  )

  ;; f64x2.splat (0xFD, 0x2A): pop f64 bit pattern → broadcast to both lanes
  (func $template_simd_f64x2_splat (param $dec_ptr i32)
    (call $emit_x86_pop_reg (i32.const 0))
    (call $emit_x86_movq_xmm0_rax)
    (call $emit_x86_byte (i32.const 0x66)) (call $emit_x86_byte (i32.const 0x0F)) (call $emit_x86_byte (i32.const 0x6C))
    (call $emit_x86_modrm (i32.const 3) (i32.const 0) (i32.const 0))
    (call $emit_x86_v128_push)
  )

  ;; ── Integer arithmetic ────────────────────────────────────────────

  ;; i8x16.add (0xFD, 0x83): PADDB xmm0, xmm1
  (func $template_simd_i8x16_add (param $dec_ptr i32)
    (call $emit_sse128_int_binop (i32.const 0xFC))
  )
  ;; i16x8.add (0xFD, 0x84): PADDW
  (func $template_simd_i16x8_add (param $dec_ptr i32)
    (call $emit_sse128_int_binop (i32.const 0xFD))
  )
  ;; i32x4.add (0xFD, 0x85): PADDD
  (func $template_simd_i32x4_add (param $dec_ptr i32)
    (call $emit_sse128_int_binop (i32.const 0xFE))
  )
  ;; i64x2.add (0xFD, 0x86): PADDQ
  (func $template_simd_i64x2_add (param $dec_ptr i32)
    (call $emit_sse128_int_binop (i32.const 0xD4))
  )

  ;; i8x16.sub (0xFD, 0x87): PSUBB
  (func $template_simd_i8x16_sub (param $dec_ptr i32)
    (call $emit_sse128_int_binop (i32.const 0xF8))
  )
  ;; i16x8.sub (0xFD, 0x88): PSUBW
  (func $template_simd_i16x8_sub (param $dec_ptr i32)
    (call $emit_sse128_int_binop (i32.const 0xF9))
  )
  ;; i32x4.sub (0xFD, 0x89): PSUBD
  (func $template_simd_i32x4_sub (param $dec_ptr i32)
    (call $emit_sse128_int_binop (i32.const 0xFA))
  )
  ;; i64x2.sub (0xFD, 0x8A): PSUBQ
  (func $template_simd_i64x2_sub (param $dec_ptr i32)
    (call $emit_sse128_int_binop (i32.const 0xFB))
  )

  ;; i16x8.mul (0xFD, 0x8C): PMULLW
  (func $template_simd_i16x8_mul (param $dec_ptr i32)
    (call $emit_sse128_int_binop (i32.const 0xD5))
  )
  ;; i32x4.mul (0xFD, 0x8D): PMULLD (SSE4.1, 66 0F 38 40 /r)
  (func $template_simd_i32x4_mul (param $dec_ptr i32)
    (call $emit_x86_v128_pop_xmm1)
    (call $emit_x86_v128_pop)
    (call $emit_x86_byte (i32.const 0x66))
    (call $emit_x86_byte (i32.const 0x0F))
    (call $emit_x86_byte (i32.const 0x38))
    (call $emit_x86_byte (i32.const 0x40))
    (call $emit_x86_modrm (i32.const 3) (i32.const 0) (i32.const 1))
    (call $emit_x86_v128_push)
  )

  ;; ── Float arithmetic ──────────────────────────────────────────────

  ;; f32x4.add (0xFD, 0xD0): ADDPS
  (func $template_simd_f32x4_add (param $dec_ptr i32)
    (call $emit_sse128_binop (i32.const 0x00) (i32.const 0x58))
  )
  ;; f64x2.add (0xFD, 0xD4): ADDPD
  (func $template_simd_f64x2_add (param $dec_ptr i32)
    (call $emit_sse128_binop (i32.const 0x66) (i32.const 0x58))
  )
  ;; f32x4.sub (0xFD, 0xD1): SUBPS
  (func $template_simd_f32x4_sub (param $dec_ptr i32)
    (call $emit_sse128_binop (i32.const 0x00) (i32.const 0x5C))
  )
  ;; f64x2.sub (0xFD, 0xD5): SUBPD
  (func $template_simd_f64x2_sub (param $dec_ptr i32)
    (call $emit_sse128_binop (i32.const 0x66) (i32.const 0x5C))
  )
  ;; f32x4.mul (0xFD, 0xD2): MULPS
  (func $template_simd_f32x4_mul (param $dec_ptr i32)
    (call $emit_sse128_binop (i32.const 0x00) (i32.const 0x59))
  )
  ;; f64x2.mul (0xFD, 0xD6): MULPD
  (func $template_simd_f64x2_mul (param $dec_ptr i32)
    (call $emit_sse128_binop (i32.const 0x66) (i32.const 0x59))
  )
  ;; f32x4.div (0xFD, 0xD3): DIVPS
  (func $template_simd_f32x4_div (param $dec_ptr i32)
    (call $emit_sse128_binop (i32.const 0x00) (i32.const 0x5E))
  )
  ;; f64x2.div (0xFD, 0xD7): DIVPD
  (func $template_simd_f64x2_div (param $dec_ptr i32)
    (call $emit_sse128_binop (i32.const 0x66) (i32.const 0x5E))
  )

  ;; ── Integer comparisons ───────────────────────────────────────────

  ;; eq: PCMPEQB/W/D
  (func $template_simd_i8x16_eq (param $dec_ptr i32)
    (call $emit_sse128_int_binop (i32.const 0x74))
  )
  (func $template_simd_i16x8_eq (param $dec_ptr i32)
    (call $emit_sse128_int_binop (i32.const 0x75))
  )
  (func $template_simd_i32x4_eq (param $dec_ptr i32)
    (call $emit_sse128_int_binop (i32.const 0x76))
  )

  ;; ne: PCMPEQ + NOT
  (func $template_simd_i8x16_ne (param $dec_ptr i32)
    (call $emit_x86_v128_pop_xmm1) (call $emit_x86_v128_pop)
    (call $emit_x86_byte (i32.const 0x66)) (call $emit_x86_byte (i32.const 0x0F)) (call $emit_x86_byte (i32.const 0x74))
    (call $emit_x86_modrm (i32.const 3) (i32.const 0) (i32.const 1))
    (call $emit_x86_v128_not)
    (call $emit_x86_v128_push)
  )
  (func $template_simd_i16x8_ne (param $dec_ptr i32)
    (call $emit_x86_v128_pop_xmm1) (call $emit_x86_v128_pop)
    (call $emit_x86_byte (i32.const 0x66)) (call $emit_x86_byte (i32.const 0x0F)) (call $emit_x86_byte (i32.const 0x75))
    (call $emit_x86_modrm (i32.const 3) (i32.const 0) (i32.const 1))
    (call $emit_x86_v128_not)
    (call $emit_x86_v128_push)
  )
  (func $template_simd_i32x4_ne (param $dec_ptr i32)
    (call $emit_x86_v128_pop_xmm1) (call $emit_x86_v128_pop)
    (call $emit_x86_byte (i32.const 0x66)) (call $emit_x86_byte (i32.const 0x0F)) (call $emit_x86_byte (i32.const 0x76))
    (call $emit_x86_modrm (i32.const 3) (i32.const 0) (i32.const 1))
    (call $emit_x86_v128_not)
    (call $emit_x86_v128_push)
  )

  ;; gt_s: PCMPGTB/W/D
  (func $template_simd_i8x16_gt_s (param $dec_ptr i32)
    (call $emit_sse128_int_binop (i32.const 0x64))
  )
  (func $template_simd_i16x8_gt_s (param $dec_ptr i32)
    (call $emit_sse128_int_binop (i32.const 0x65))
  )
  (func $template_simd_i32x4_gt_s (param $dec_ptr i32)
    (call $emit_sse128_int_binop (i32.const 0x66))
  )

  ;; lt_s: swap operands + PCMPGT: pop xmm0 first, pop xmm1, then pcmpgt xmm1, xmm0
  ;; Equivalent to: pop rhs into xmm0, pop lhs into xmm1, pcmpgt xmm1,xmm0 (rhs > lhs = lhs < rhs)
  (func $template_simd_i8x16_lt_s (param $dec_ptr i32)
    (call $emit_x86_v128_pop) (call $emit_x86_v128_pop_xmm1)
    (call $emit_x86_byte (i32.const 0x66)) (call $emit_x86_byte (i32.const 0x0F)) (call $emit_x86_byte (i32.const 0x64))
    (call $emit_x86_modrm (i32.const 3) (i32.const 1) (i32.const 0))
    (call $emit_x86_byte (i32.const 0x66)) (call $emit_x86_byte (i32.const 0x0F)) (call $emit_x86_byte (i32.const 0x6F))
    (call $emit_x86_modrm (i32.const 3) (i32.const 0) (i32.const 1))
    (call $emit_x86_v128_push)
  )
  (func $template_simd_i16x8_lt_s (param $dec_ptr i32)
    (call $emit_x86_v128_pop) (call $emit_x86_v128_pop_xmm1)
    (call $emit_x86_byte (i32.const 0x66)) (call $emit_x86_byte (i32.const 0x0F)) (call $emit_x86_byte (i32.const 0x65))
    (call $emit_x86_modrm (i32.const 3) (i32.const 1) (i32.const 0))
    (call $emit_x86_byte (i32.const 0x66)) (call $emit_x86_byte (i32.const 0x0F)) (call $emit_x86_byte (i32.const 0x6F))
    (call $emit_x86_modrm (i32.const 3) (i32.const 0) (i32.const 1))
    (call $emit_x86_v128_push)
  )
  (func $template_simd_i32x4_lt_s (param $dec_ptr i32)
    (call $emit_x86_v128_pop) (call $emit_x86_v128_pop_xmm1)
    (call $emit_x86_byte (i32.const 0x66)) (call $emit_x86_byte (i32.const 0x0F)) (call $emit_x86_byte (i32.const 0x66))
    (call $emit_x86_modrm (i32.const 3) (i32.const 1) (i32.const 0))
    (call $emit_x86_byte (i32.const 0x66)) (call $emit_x86_byte (i32.const 0x0F)) (call $emit_x86_byte (i32.const 0x6F))
    (call $emit_x86_modrm (i32.const 3) (i32.const 0) (i32.const 1))
    (call $emit_x86_v128_push)
  )

  ;; le_s: NOT(gt_s): do gt_s on popped values, then NOT
  (func $template_simd_i8x16_le_s (param $dec_ptr i32)
    (call $emit_x86_v128_pop_xmm1) (call $emit_x86_v128_pop)
    (call $emit_x86_byte (i32.const 0x66)) (call $emit_x86_byte (i32.const 0x0F)) (call $emit_x86_byte (i32.const 0x64))
    (call $emit_x86_modrm (i32.const 3) (i32.const 0) (i32.const 1))
    (call $emit_x86_v128_not)
    (call $emit_x86_v128_push)
  )
  (func $template_simd_i16x8_le_s (param $dec_ptr i32)
    (call $emit_x86_v128_pop_xmm1) (call $emit_x86_v128_pop)
    (call $emit_x86_byte (i32.const 0x66)) (call $emit_x86_byte (i32.const 0x0F)) (call $emit_x86_byte (i32.const 0x65))
    (call $emit_x86_modrm (i32.const 3) (i32.const 0) (i32.const 1))
    (call $emit_x86_v128_not)
    (call $emit_x86_v128_push)
  )
  (func $template_simd_i32x4_le_s (param $dec_ptr i32)
    (call $emit_x86_v128_pop_xmm1) (call $emit_x86_v128_pop)
    (call $emit_x86_byte (i32.const 0x66)) (call $emit_x86_byte (i32.const 0x0F)) (call $emit_x86_byte (i32.const 0x66))
    (call $emit_x86_modrm (i32.const 3) (i32.const 0) (i32.const 1))
    (call $emit_x86_v128_not)
    (call $emit_x86_v128_push)
  )

  ;; ge_s: NOT(swap + PCMPGT) = NOT(lt_s)
  (func $template_simd_i8x16_ge_s (param $dec_ptr i32)
    (call $emit_x86_v128_pop) (call $emit_x86_v128_pop_xmm1)
    (call $emit_x86_byte (i32.const 0x66)) (call $emit_x86_byte (i32.const 0x0F)) (call $emit_x86_byte (i32.const 0x64))
    (call $emit_x86_modrm (i32.const 3) (i32.const 1) (i32.const 0))
    (call $emit_x86_v128_not)
    (call $emit_x86_v128_push)
  )
  (func $template_simd_i16x8_ge_s (param $dec_ptr i32)
    (call $emit_x86_v128_pop) (call $emit_x86_v128_pop_xmm1)
    (call $emit_x86_byte (i32.const 0x66)) (call $emit_x86_byte (i32.const 0x0F)) (call $emit_x86_byte (i32.const 0x65))
    (call $emit_x86_modrm (i32.const 3) (i32.const 1) (i32.const 0))
    (call $emit_x86_v128_not)
    (call $emit_x86_v128_push)
  )
  (func $template_simd_i32x4_ge_s (param $dec_ptr i32)
    (call $emit_x86_v128_pop) (call $emit_x86_v128_pop_xmm1)
    (call $emit_x86_byte (i32.const 0x66)) (call $emit_x86_byte (i32.const 0x0F)) (call $emit_x86_byte (i32.const 0x66))
    (call $emit_x86_modrm (i32.const 3) (i32.const 1) (i32.const 0))
    (call $emit_x86_v128_not)
    (call $emit_x86_v128_push)
  )

  ;; ── Float comparisons (use CMPPS/CMPPD with imm8) ─────────────────

  ;; f32x4.eq: CMPPS xmm0, xmm1, 0 (EQ_OQ)
  (func $template_simd_f32x4_eq (param $dec_ptr i32)
    (call $emit_sse128_cmp (i32.const 0x00) (i32.const 0))
  )
  ;; f64x2.eq: CMPPD xmm0, xmm1, 0
  (func $template_simd_f64x2_eq (param $dec_ptr i32)
    (call $emit_sse128_cmp (i32.const 0x66) (i32.const 0))
  )
  ;; f32x4.ne: CMPPS with imm8=4 (NEQ_UQ)
  (func $template_simd_f32x4_ne (param $dec_ptr i32)
    (call $emit_sse128_cmp (i32.const 0x00) (i32.const 4))
  )
  ;; f64x2.ne: CMPPD with imm8=4
  (func $template_simd_f64x2_ne (param $dec_ptr i32)
    (call $emit_sse128_cmp (i32.const 0x66) (i32.const 4))
  )
  ;; f32x4.lt: CMPPS with imm8=1 (LT_OS)
  (func $template_simd_f32x4_lt (param $dec_ptr i32)
    (call $emit_sse128_cmp (i32.const 0x00) (i32.const 1))
  )
  ;; f64x2.lt: CMPPD with imm8=1
  (func $template_simd_f64x2_lt (param $dec_ptr i32)
    (call $emit_sse128_cmp (i32.const 0x66) (i32.const 1))
  )
  ;; f32x4.gt: CMPPS with imm8=6 (GT_OS)
  (func $template_simd_f32x4_gt (param $dec_ptr i32)
    (call $emit_sse128_cmp (i32.const 0x00) (i32.const 6))
  )
  ;; f64x2.gt: CMPPD with imm8=6
  (func $template_simd_f64x2_gt (param $dec_ptr i32)
    (call $emit_sse128_cmp (i32.const 0x66) (i32.const 6))
  )
  ;; f32x4.le: CMPPS with imm8=2 (LE_OS)
  (func $template_simd_f32x4_le (param $dec_ptr i32)
    (call $emit_sse128_cmp (i32.const 0x00) (i32.const 2))
  )
  ;; f64x2.le: CMPPD with imm8=2
  (func $template_simd_f64x2_le (param $dec_ptr i32)
    (call $emit_sse128_cmp (i32.const 0x66) (i32.const 2))
  )
  ;; f32x4.ge: CMPPS with imm8=5 (GE_OS)
  (func $template_simd_f32x4_ge (param $dec_ptr i32)
    (call $emit_sse128_cmp (i32.const 0x00) (i32.const 5))
  )
  ;; f64x2.ge: CMPPD with imm8=5
  (func $template_simd_f64x2_ge (param $dec_ptr i32)
    (call $emit_sse128_cmp (i32.const 0x66) (i32.const 5))
  )

  ;; ── Negation ──────────────────────────────────────────────────────

  ;; i8x16.neg (0xFD, 0x7C): 0 - x = PXOR(PXOR(x,x), x) — simpler: PSUBB with zero
  ;; We use: pxor xmm1,xmm1; psubb xmm0,xmm1 where xmm0 is popped lhs
  (func $template_simd_i8x16_neg (param $dec_ptr i32)
    (call $emit_x86_v128_pop)
    (call $emit_x86_byte (i32.const 0x66)) (call $emit_x86_byte (i32.const 0x0F)) (call $emit_x86_byte (i32.const 0xEF))
    (call $emit_x86_modrm (i32.const 3) (i32.const 1) (i32.const 1))
    (call $emit_x86_byte (i32.const 0x66)) (call $emit_x86_byte (i32.const 0x0F)) (call $emit_x86_byte (i32.const 0xF8))
    (call $emit_x86_modrm (i32.const 3) (i32.const 1) (i32.const 0))
    (call $emit_x86_v128_push)
  )
  ;; i16x8.neg: pxor xmm1,xmm1; psubw xmm1,xmm0
  (func $template_simd_i16x8_neg (param $dec_ptr i32)
    (call $emit_x86_v128_pop)
    (call $emit_x86_byte (i32.const 0x66)) (call $emit_x86_byte (i32.const 0x0F)) (call $emit_x86_byte (i32.const 0xEF))
    (call $emit_x86_modrm (i32.const 3) (i32.const 1) (i32.const 1))
    (call $emit_x86_byte (i32.const 0x66)) (call $emit_x86_byte (i32.const 0x0F)) (call $emit_x86_byte (i32.const 0xF9))
    (call $emit_x86_modrm (i32.const 3) (i32.const 1) (i32.const 0))
    (call $emit_x86_v128_push)
  )
  ;; i32x4.neg: pxor xmm1,xmm1; psubd xmm1,xmm0
  (func $template_simd_i32x4_neg (param $dec_ptr i32)
    (call $emit_x86_v128_pop)
    (call $emit_x86_byte (i32.const 0x66)) (call $emit_x86_byte (i32.const 0x0F)) (call $emit_x86_byte (i32.const 0xEF))
    (call $emit_x86_modrm (i32.const 3) (i32.const 1) (i32.const 1))
    (call $emit_x86_byte (i32.const 0x66)) (call $emit_x86_byte (i32.const 0x0F)) (call $emit_x86_byte (i32.const 0xFA))
    (call $emit_x86_modrm (i32.const 3) (i32.const 1) (i32.const 0))
    (call $emit_x86_v128_push)
  )
  ;; i64x2.neg: pxor xmm1,xmm1; psubq xmm1,xmm0
  (func $template_simd_i64x2_neg (param $dec_ptr i32)
    (call $emit_x86_v128_pop)
    (call $emit_x86_byte (i32.const 0x66)) (call $emit_x86_byte (i32.const 0x0F)) (call $emit_x86_byte (i32.const 0xEF))
    (call $emit_x86_modrm (i32.const 3) (i32.const 1) (i32.const 1))
    (call $emit_x86_byte (i32.const 0x66)) (call $emit_x86_byte (i32.const 0x0F)) (call $emit_x86_byte (i32.const 0xFB))
    (call $emit_x86_modrm (i32.const 3) (i32.const 1) (i32.const 0))
    (call $emit_x86_v128_push)
  )

  ;; f32x4.neg: XORPS with sign bit (0x80000000 in each lane)
  ;; We use: pcmpeqd xmm1,xmm1; pslld xmm1,31; xorps xmm0,xmm1
  ;; OR: xorps xmm0, [sign_mask_constant]
  ;; Simple: pcmpeqd xmm1,xmm1; pslld xmm1,31; xorps xmm0,xmm1
  (func $template_simd_f32x4_neg (param $dec_ptr i32)
    (call $emit_x86_v128_pop)
    (call $emit_x86_byte (i32.const 0x66)) (call $emit_x86_byte (i32.const 0x0F)) (call $emit_x86_byte (i32.const 0x76))
    (call $emit_x86_modrm (i32.const 3) (i32.const 1) (i32.const 1))
    (call $emit_x86_byte (i32.const 0x66)) (call $emit_x86_byte (i32.const 0x0F)) (call $emit_x86_byte (i32.const 0xF2))
    (call $emit_x86_modrm (i32.const 3) (i32.const 1) (i32.const 1))
    (call $emit_x86_byte (i32.const 31))
    (call $emit_x86_byte (i32.const 0x00)) (call $emit_x86_byte (i32.const 0x0F)) (call $emit_x86_byte (i32.const 0x57))
    (call $emit_x86_modrm (i32.const 3) (i32.const 0) (i32.const 1))
    (call $emit_x86_v128_push)
  )
  ;; f64x2.neg: XORPD with sign bit (0x8000000000000000 in each lane)
  ;; pcmpeqd xmm1,xmm1; psllq xmm1,63; xorpd xmm0,xmm1
  (func $template_simd_f64x2_neg (param $dec_ptr i32)
    (call $emit_x86_v128_pop)
    (call $emit_x86_byte (i32.const 0x66)) (call $emit_x86_byte (i32.const 0x0F)) (call $emit_x86_byte (i32.const 0x76))
    (call $emit_x86_modrm (i32.const 3) (i32.const 1) (i32.const 1))
    (call $emit_x86_byte (i32.const 0x66)) (call $emit_x86_byte (i32.const 0x0F)) (call $emit_x86_byte (i32.const 0xF3))
    (call $emit_x86_modrm (i32.const 3) (i32.const 1) (i32.const 1))
    (call $emit_x86_byte (i32.const 63))
    (call $emit_x86_byte (i32.const 0x66)) (call $emit_x86_byte (i32.const 0x0F)) (call $emit_x86_byte (i32.const 0x57))
    (call $emit_x86_modrm (i32.const 3) (i32.const 0) (i32.const 1))
    (call $emit_x86_v128_push)
  )

  ;; ── Absolute value ────────────────────────────────────────────────

  ;; f32x4.abs: ANDPS with sign bit cleared (0x7FFFFFFF)
  ;; pcmpeqd xmm1,xmm1; pslld xmm1,31; xorps xmm0,xmm1; andps xmm0,xmm1? No...
  ;; ANDPS with 0x7FFFFFFF mask. We'll use: pcmpeqd xmm1,xmm1; pslld xmm1,31;
  ;; xorps xmm0,xmm1 <- wait, that's NEG. For ABS: andps with not-sign-bit.
  ;; Better: pcmpeqd xmm1,xmm1; pslld xmm1,31 (all-ones mask, then shift left 31 to get sign bit only)
  ;; pandn xmm1 (NOT of sign mask) -- actually: we want ANDPS with mask = ~(1<<31)
  ;; pandn xmm0, xmm1 (xmm0 = xmm0 AND NOT xmm1) — SSE2
  ;; PANDA = PANDN: 66 0F DF /r
  (func $template_simd_f32x4_abs (param $dec_ptr i32)
    (call $emit_x86_v128_pop)
    (call $emit_x86_byte (i32.const 0x66)) (call $emit_x86_byte (i32.const 0x0F)) (call $emit_x86_byte (i32.const 0x76))
    (call $emit_x86_modrm (i32.const 3) (i32.const 1) (i32.const 1))
    (call $emit_x86_byte (i32.const 0x66)) (call $emit_x86_byte (i32.const 0x0F)) (call $emit_x86_byte (i32.const 0xF2))
    (call $emit_x86_modrm (i32.const 3) (i32.const 1) (i32.const 1))
    (call $emit_x86_byte (i32.const 31))
    (call $emit_x86_byte (i32.const 0x66)) (call $emit_x86_byte (i32.const 0x0F)) (call $emit_x86_byte (i32.const 0xDF))
    (call $emit_x86_modrm (i32.const 3) (i32.const 0) (i32.const 1))
    (call $emit_x86_v128_push)
  )
  ;; f64x2.abs: PANDN with sign bit mask
  ;; pcmpeqd xmm1,xmm1; psllq xmm1,63; pandn xmm0,xmm1
  (func $template_simd_f64x2_abs (param $dec_ptr i32)
    (call $emit_x86_v128_pop)
    (call $emit_x86_byte (i32.const 0x66)) (call $emit_x86_byte (i32.const 0x0F)) (call $emit_x86_byte (i32.const 0x76))
    (call $emit_x86_modrm (i32.const 3) (i32.const 1) (i32.const 1))
    (call $emit_x86_byte (i32.const 0x66)) (call $emit_x86_byte (i32.const 0x0F)) (call $emit_x86_byte (i32.const 0xF3))
    (call $emit_x86_modrm (i32.const 3) (i32.const 1) (i32.const 1))
    (call $emit_x86_byte (i32.const 63))
    (call $emit_x86_byte (i32.const 0x66)) (call $emit_x86_byte (i32.const 0x0F)) (call $emit_x86_byte (i32.const 0xDF))
    (call $emit_x86_modrm (i32.const 3) (i32.const 0) (i32.const 1))
    (call $emit_x86_v128_push)
  )

  ;; ── Min / Max (unsigned byte and word) ────────────────────────────

  ;; i8x16.min_u: PMINUB (66 0F DA)
  (func $template_simd_i8x16_min_u (param $dec_ptr i32)
    (call $emit_sse128_int_binop (i32.const 0xDA))
  )
  ;; i16x8.min_u: PMINUW (SSE4.1: 66 0F 38 3A)
  (func $template_simd_i16x8_min_u (param $dec_ptr i32)
    (call $emit_x86_v128_pop_xmm1) (call $emit_x86_v128_pop)
    (call $emit_x86_byte (i32.const 0x66)) (call $emit_x86_byte (i32.const 0x0F)) (call $emit_x86_byte (i32.const 0x38))
    (call $emit_x86_byte (i32.const 0x3A))
    (call $emit_x86_modrm (i32.const 3) (i32.const 0) (i32.const 1))
    (call $emit_x86_v128_push)
  )
  ;; i8x16.max_u: PMAXUB (66 0F DE)
  (func $template_simd_i8x16_max_u (param $dec_ptr i32)
    (call $emit_sse128_int_binop (i32.const 0xDE))
  )
  ;; i16x8.max_u: PMAXUW (SSE4.1: 66 0F 38 3E)
  (func $template_simd_i16x8_max_u (param $dec_ptr i32)
    (call $emit_x86_v128_pop_xmm1) (call $emit_x86_v128_pop)
    (call $emit_x86_byte (i32.const 0x66)) (call $emit_x86_byte (i32.const 0x0F)) (call $emit_x86_byte (i32.const 0x38))
    (call $emit_x86_byte (i32.const 0x3E))
    (call $emit_x86_modrm (i32.const 3) (i32.const 0) (i32.const 1))
    (call $emit_x86_v128_push)
  )

  ;; ═════════════════════════════════════════════════════════════════════
  ;; Extract lane (v128 → scalar) — SSE4.1
  ;; ═════════════════════════════════════════════════════════════════════

  ;; i8x16.extract_lane_s (0x2E): pop v128, sign-extract byte lane, push i32
  ;; PEXTRB eax, xmm0, imm8 + MOVSX eax, al
  (func $template_simd_i8x16_extract_lane_s (param $dec_ptr i32)
    (call $emit_x86_v128_pop)
    (call $emit_x86_byte (i32.const 0x66)) (call $emit_x86_byte (i32.const 0x0F)) (call $emit_x86_byte (i32.const 0x3A))
    (call $emit_x86_byte (i32.const 0x14))
    (call $emit_x86_modrm (i32.const 3) (i32.const 0) (i32.const 0))
    (call $emit_x86_byte (i32.load8_u (i32.add (local.get $dec_ptr) (i32.const 8))))
    (call $emit_x86_byte (i32.const 0x0F)) (call $emit_x86_byte (i32.const 0xBE))
    (call $emit_x86_modrm (i32.const 3) (i32.const 0) (i32.const 0))
    (call $emit_x86_push_rax)
  )

  ;; i8x16.extract_lane_u (0x2F): PEXTRB (zero-extends implicitly)
  (func $template_simd_i8x16_extract_lane_u (param $dec_ptr i32)
    (call $emit_x86_v128_pop)
    (call $emit_x86_byte (i32.const 0x66)) (call $emit_x86_byte (i32.const 0x0F)) (call $emit_x86_byte (i32.const 0x3A))
    (call $emit_x86_byte (i32.const 0x14))
    (call $emit_x86_modrm (i32.const 3) (i32.const 0) (i32.const 0))
    (call $emit_x86_byte (i32.load8_u (i32.add (local.get $dec_ptr) (i32.const 8))))
    (call $emit_x86_push_rax)
  )

  ;; i16x8.extract_lane_s (0x32): PEXTRW + MOVSX
  (func $template_simd_i16x8_extract_lane_s (param $dec_ptr i32)
    (call $emit_x86_v128_pop)
    (call $emit_x86_byte (i32.const 0x66)) (call $emit_x86_byte (i32.const 0x0F)) (call $emit_x86_byte (i32.const 0xC5))
    (call $emit_x86_modrm (i32.const 3) (i32.const 0) (i32.const 0))
    (call $emit_x86_byte (i32.load8_u (i32.add (local.get $dec_ptr) (i32.const 8))))
    (call $emit_x86_byte (i32.const 0x0F)) (call $emit_x86_byte (i32.const 0xBF))
    (call $emit_x86_modrm (i32.const 3) (i32.const 0) (i32.const 0))
    (call $emit_x86_push_rax)
  )

  ;; i16x8.extract_lane_u (0x33): PEXTRW (zero-extends)
  (func $template_simd_i16x8_extract_lane_u (param $dec_ptr i32)
    (call $emit_x86_v128_pop)
    (call $emit_x86_byte (i32.const 0x66)) (call $emit_x86_byte (i32.const 0x0F)) (call $emit_x86_byte (i32.const 0xC5))
    (call $emit_x86_modrm (i32.const 3) (i32.const 0) (i32.const 0))
    (call $emit_x86_byte (i32.load8_u (i32.add (local.get $dec_ptr) (i32.const 8))))
    (call $emit_x86_push_rax)
  )

  ;; i32x4.extract_lane (0x36): PEXTRD
  (func $template_simd_i32x4_extract_lane (param $dec_ptr i32)
    (call $emit_x86_v128_pop)
    (call $emit_x86_byte (i32.const 0x66)) (call $emit_x86_byte (i32.const 0x0F)) (call $emit_x86_byte (i32.const 0x3A))
    (call $emit_x86_byte (i32.const 0x16))
    (call $emit_x86_modrm (i32.const 3) (i32.const 0) (i32.const 0))
    (call $emit_x86_byte (i32.load8_u (i32.add (local.get $dec_ptr) (i32.const 8))))
    (call $emit_x86_push_rax)
  )

  ;; i64x2.extract_lane (0x38): PEXTRQ (REX.W)
  (func $template_simd_i64x2_extract_lane (param $dec_ptr i32)
    (call $emit_x86_v128_pop)
    (call $emit_x86_rex_w)
    (call $emit_x86_byte (i32.const 0x66)) (call $emit_x86_byte (i32.const 0x0F)) (call $emit_x86_byte (i32.const 0x3A))
    (call $emit_x86_byte (i32.const 0x16))
    (call $emit_x86_modrm (i32.const 3) (i32.const 0) (i32.const 0))
    (call $emit_x86_byte (i32.load8_u (i32.add (local.get $dec_ptr) (i32.const 8))))
    (call $emit_x86_push_rax)
  )

  ;; f32x4.extract_lane (0x3B): PEXTRD (treat f32 bits as i32)
  (func $template_simd_f32x4_extract_lane (param $dec_ptr i32)
    (call $emit_x86_v128_pop)
    (call $emit_x86_byte (i32.const 0x66)) (call $emit_x86_byte (i32.const 0x0F)) (call $emit_x86_byte (i32.const 0x3A))
    (call $emit_x86_byte (i32.const 0x16))
    (call $emit_x86_modrm (i32.const 3) (i32.const 0) (i32.const 0))
    (call $emit_x86_byte (i32.load8_u (i32.add (local.get $dec_ptr) (i32.const 8))))
    (call $emit_x86_push_rax)
  )

  ;; f64x2.extract_lane (0x3E): PEXTRQ (treat f64 bits as i64)
  (func $template_simd_f64x2_extract_lane (param $dec_ptr i32)
    (call $emit_x86_v128_pop)
    (call $emit_x86_rex_w)
    (call $emit_x86_byte (i32.const 0x66)) (call $emit_x86_byte (i32.const 0x0F)) (call $emit_x86_byte (i32.const 0x3A))
    (call $emit_x86_byte (i32.const 0x16))
    (call $emit_x86_modrm (i32.const 3) (i32.const 0) (i32.const 0))
    (call $emit_x86_byte (i32.load8_u (i32.add (local.get $dec_ptr) (i32.const 8))))
    (call $emit_x86_push_rax)
  )

  ;; ═════════════════════════════════════════════════════════════════════
  ;; Replace lane (v128 = v128 insert scalar) — SSE4.1
  ;; ═════════════════════════════════════════════════════════════════════
  ;; Stack: pop scalar, pop v128 → insert → push v128

  ;; i8x16.replace_lane (0x30): PINSRB
  (func $template_simd_i8x16_replace_lane (param $dec_ptr i32)
    (call $emit_x86_pop_reg (i32.const 0))
    (call $emit_x86_v128_pop_xmm1)
    (call $emit_x86_byte (i32.const 0x66)) (call $emit_x86_byte (i32.const 0x0F)) (call $emit_x86_byte (i32.const 0x3A))
    (call $emit_x86_byte (i32.const 0x20))
    (call $emit_x86_modrm (i32.const 3) (i32.const 1) (i32.const 0))
    (call $emit_x86_byte (i32.load8_u (i32.add (local.get $dec_ptr) (i32.const 8))))
    (call $emit_x86_v128_push)
  )

  ;; i16x8.replace_lane (0x34): PINSRW
  (func $template_simd_i16x8_replace_lane (param $dec_ptr i32)
    (call $emit_x86_pop_reg (i32.const 0))
    (call $emit_x86_v128_pop_xmm1)
    (call $emit_x86_byte (i32.const 0x66)) (call $emit_x86_byte (i32.const 0x0F)) (call $emit_x86_byte (i32.const 0xC4))
    (call $emit_x86_modrm (i32.const 3) (i32.const 1) (i32.const 0))
    (call $emit_x86_byte (i32.load8_u (i32.add (local.get $dec_ptr) (i32.const 8))))
    (call $emit_x86_v128_push)
  )

  ;; i32x4.replace_lane (0x37): PINSRD
  (func $template_simd_i32x4_replace_lane (param $dec_ptr i32)
    (call $emit_x86_pop_reg (i32.const 0))
    (call $emit_x86_v128_pop_xmm1)
    (call $emit_x86_byte (i32.const 0x66)) (call $emit_x86_byte (i32.const 0x0F)) (call $emit_x86_byte (i32.const 0x3A))
    (call $emit_x86_byte (i32.const 0x22))
    (call $emit_x86_modrm (i32.const 3) (i32.const 1) (i32.const 0))
    (call $emit_x86_byte (i32.load8_u (i32.add (local.get $dec_ptr) (i32.const 8))))
    (call $emit_x86_v128_push)
  )

  ;; i64x2.replace_lane (0xAB): PINSRQ (REX.W)
  (func $template_simd_i64x2_replace_lane (param $dec_ptr i32)
    (call $emit_x86_pop_reg (i32.const 0))
    (call $emit_x86_v128_pop_xmm1)
    (call $emit_x86_rex_w)
    (call $emit_x86_byte (i32.const 0x66)) (call $emit_x86_byte (i32.const 0x0F)) (call $emit_x86_byte (i32.const 0x3A))
    (call $emit_x86_byte (i32.const 0x22))
    (call $emit_x86_modrm (i32.const 3) (i32.const 1) (i32.const 0))
    (call $emit_x86_byte (i32.load8_u (i32.add (local.get $dec_ptr) (i32.const 8))))
    (call $emit_x86_v128_push)
  )

  ;; f32x4.replace_lane (0x3C): PINSRD
  (func $template_simd_f32x4_replace_lane (param $dec_ptr i32)
    (call $emit_x86_pop_reg (i32.const 0))
    (call $emit_x86_v128_pop_xmm1)
    (call $emit_x86_byte (i32.const 0x66)) (call $emit_x86_byte (i32.const 0x0F)) (call $emit_x86_byte (i32.const 0x3A))
    (call $emit_x86_byte (i32.const 0x22))
    (call $emit_x86_modrm (i32.const 3) (i32.const 1) (i32.const 0))
    (call $emit_x86_byte (i32.load8_u (i32.add (local.get $dec_ptr) (i32.const 8))))
    (call $emit_x86_v128_push)
  )

  ;; f64x2.replace_lane (0x3F): PINSRQ (REX.W)
  (func $template_simd_f64x2_replace_lane (param $dec_ptr i32)
    (call $emit_x86_pop_reg (i32.const 0))
    (call $emit_x86_v128_pop_xmm1)
    (call $emit_x86_rex_w)
    (call $emit_x86_byte (i32.const 0x66)) (call $emit_x86_byte (i32.const 0x0F)) (call $emit_x86_byte (i32.const 0x3A))
    (call $emit_x86_byte (i32.const 0x22))
    (call $emit_x86_modrm (i32.const 3) (i32.const 1) (i32.const 0))
    (call $emit_x86_byte (i32.load8_u (i32.add (local.get $dec_ptr) (i32.const 8))))
    (call $emit_x86_v128_push)
  )

  ;; ═════════════════════════════════════════════════════════════════════
  ;; Load-splat (memory → broadcast) — SSE2/SSSE3
  ;; ═════════════════════════════════════════════════════════════════════

  ;; v128.load8_splat (0xA6): load byte from [addr], broadcast to 16
  ;; MOVD xmm0, [rax]; PXOR xmm1,xmm1; PSHUFB xmm0,xmm1
  (func $template_simd_v128_load8_splat (param $dec_ptr i32)
    (call $emit_x86_pop_reg (i32.const 0))
    (call $emit_x86_byte (i32.const 0x66)) (call $emit_x86_byte (i32.const 0x0F)) (call $emit_x86_byte (i32.const 0x6E))
    (call $emit_x86_modrm (i32.const 0) (i32.const 0) (i32.const 0))
    (call $emit_x86_byte (i32.const 0x66)) (call $emit_x86_byte (i32.const 0x0F)) (call $emit_x86_byte (i32.const 0xEF))
    (call $emit_x86_modrm (i32.const 3) (i32.const 1) (i32.const 1))
    (call $emit_x86_byte (i32.const 0x66)) (call $emit_x86_byte (i32.const 0x0F)) (call $emit_x86_byte (i32.const 0x38))
    (call $emit_x86_byte (i32.const 0x00))
    (call $emit_x86_modrm (i32.const 3) (i32.const 0) (i32.const 1))
    (call $emit_x86_v128_push)
  )

  ;; v128.load16_splat (0xA7): load word from [rax], broadcast to 8 words
  ;; MOVD xmm0, [rax]; PUNPCKLWD xmm0,xmm0; PSHUFD xmm0,xmm0,0
  (func $template_simd_v128_load16_splat (param $dec_ptr i32)
    (call $emit_x86_pop_reg (i32.const 0))
    (call $emit_x86_byte (i32.const 0x66)) (call $emit_x86_byte (i32.const 0x0F)) (call $emit_x86_byte (i32.const 0x6E))
    (call $emit_x86_modrm (i32.const 0) (i32.const 0) (i32.const 0))
    (call $emit_x86_byte (i32.const 0x66)) (call $emit_x86_byte (i32.const 0x0F)) (call $emit_x86_byte (i32.const 0x61))
    (call $emit_x86_modrm (i32.const 3) (i32.const 0) (i32.const 0))
    (call $emit_x86_byte (i32.const 0x66)) (call $emit_x86_byte (i32.const 0x0F)) (call $emit_x86_byte (i32.const 0x70))
    (call $emit_x86_modrm (i32.const 3) (i32.const 0) (i32.const 0))
    (call $emit_x86_byte (i32.const 0))
    (call $emit_x86_v128_push)
  )

  ;; v128.load32_splat (0xA8): load dword from [rax], broadcast to 4
  ;; MOVD xmm0, [rax]; PSHUFD xmm0,xmm0,0
  (func $template_simd_v128_load32_splat (param $dec_ptr i32)
    (call $emit_x86_pop_reg (i32.const 0))
    (call $emit_x86_byte (i32.const 0x66)) (call $emit_x86_byte (i32.const 0x0F)) (call $emit_x86_byte (i32.const 0x6E))
    (call $emit_x86_modrm (i32.const 0) (i32.const 0) (i32.const 0))
    (call $emit_x86_byte (i32.const 0x66)) (call $emit_x86_byte (i32.const 0x0F)) (call $emit_x86_byte (i32.const 0x70))
    (call $emit_x86_modrm (i32.const 3) (i32.const 0) (i32.const 0))
    (call $emit_x86_byte (i32.const 0))
    (call $emit_x86_v128_push)
  )

  ;; v128.load64_splat (0xA9): load qword from [rax], broadcast to 2
  ;; MOVQ xmm0, [rax]; PUNPCKLQDQ xmm0,xmm0
  (func $template_simd_v128_load64_splat (param $dec_ptr i32)
    (call $emit_x86_pop_reg (i32.const 0))
    (call $emit_x86_rex_w)
    (call $emit_x86_byte (i32.const 0x66)) (call $emit_x86_byte (i32.const 0x0F)) (call $emit_x86_byte (i32.const 0x6E))
    (call $emit_x86_modrm (i32.const 0) (i32.const 0) (i32.const 0))
    (call $emit_x86_byte (i32.const 0x66)) (call $emit_x86_byte (i32.const 0x0F)) (call $emit_x86_byte (i32.const 0x6C))
    (call $emit_x86_modrm (i32.const 3) (i32.const 0) (i32.const 0))
    (call $emit_x86_v128_push)
  )

  ;; ═════════════════════════════════════════════════════════════════════
  ;; Swizzle — SSSE3
  ;; ═════════════════════════════════════════════════════════════════════

  ;; i8x16.swizzle (0xAA): PSHUFB xmm0, xmm1
  (func $template_simd_i8x16_swizzle (param $dec_ptr i32)
    (call $emit_x86_v128_pop_xmm1)
    (call $emit_x86_v128_pop)
    (call $emit_x86_byte (i32.const 0x66)) (call $emit_x86_byte (i32.const 0x0F)) (call $emit_x86_byte (i32.const 0x38))
    (call $emit_x86_byte (i32.const 0x00))
    (call $emit_x86_modrm (i32.const 3) (i32.const 0) (i32.const 1))
    (call $emit_x86_v128_push)
  )

  ;; ═════════════════════════════════════════════════════════════════════
  ;; f32x4/f64x2 min/max
  ;; ═════════════════════════════════════════════════════════════════════

  ;; f32x4.min (0xE1): MINPS xmm0, xmm1: 00 0F 5D /r
  (func $template_simd_f32x4_min (param $dec_ptr i32)
    (call $emit_sse128_binop (i32.const 0x00) (i32.const 0x5D))
  )

  ;; f32x4.max (0xE2): MAXPS xmm0, xmm1: 00 0F 5F /r
  (func $template_simd_f32x4_max (param $dec_ptr i32)
    (call $emit_sse128_binop (i32.const 0x00) (i32.const 0x5F))
  )

  ;; f64x2.min (0xE3): MINPD xmm0, xmm1: 66 0F 5D /r
  (func $template_simd_f64x2_min (param $dec_ptr i32)
    (call $emit_sse128_binop (i32.const 0x66) (i32.const 0x5D))
  )

  ;; f64x2.max (0xE4): MAXPD xmm0, xmm1: 66 0F 5F /r
  (func $template_simd_f64x2_max (param $dec_ptr i32)
    (call $emit_sse128_binop (i32.const 0x66) (i32.const 0x5F))
  )

  ;; ═════════════════════════════════════════════════════════════════════
  ;; avgr_u (average unsigned) — SSE2
  ;; ═════════════════════════════════════════════════════════════════════

  ;; i8x16.avgr_u (0xBB): PAVGB: 66 0F E0 /r
  (func $template_simd_i8x16_avgr_u (param $dec_ptr i32)
    (call $emit_sse128_int_binop (i32.const 0xE0))
  )

  ;; i16x8.avgr_u (0xBF): PAVGW: 66 0F E3 /r
  (func $template_simd_i16x8_avgr_u (param $dec_ptr i32)
    (call $emit_sse128_int_binop (i32.const 0xE3))
  )

  ;; ═════════════════════════════════════════════════════════════════════
  ;; Unsigned comparisons (bias-XOR technique)
  ;; lt_u = PCMPGT(b^bias, a^bias), gt_u = PCMPGT(a^bias, b^bias)
  ;; le_u = NOT(gt_u), ge_u = NOT(lt_u)
  ;; ═════════════════════════════════════════════════════════════════════
  ;;
  ;; Bias generation in xmm2:
  ;;   PXOR xmm2,xmm2; PCMPEQ xmm2,xmm2 (all 1s)
  ;;   PSLLW/D/Q then PACK for byte bias

  ;; i8x16.lt_u (0x4A): PCMPGTB(b^0x80, a^0x80)
  (func $template_simd_i8x16_lt_u (param $dec_ptr i32)
    (call $emit_x86_v128_pop_xmm1)
    (call $emit_x86_v128_pop)
    (call $emit_x86_byte (i32.const 0x66)) (call $emit_x86_byte (i32.const 0x0F)) (call $emit_x86_byte (i32.const 0xEF))
    (call $emit_x86_modrm (i32.const 3) (i32.const 2) (i32.const 2))
    (call $emit_x86_byte (i32.const 0x66)) (call $emit_x86_byte (i32.const 0x0F)) (call $emit_x86_byte (i32.const 0x76))
    (call $emit_x86_modrm (i32.const 3) (i32.const 2) (i32.const 2))
    (call $emit_x86_byte (i32.const 0x66)) (call $emit_x86_byte (i32.const 0x0F)) (call $emit_x86_byte (i32.const 0xF1))
    (call $emit_x86_modrm (i32.const 3) (i32.const 2) (i32.const 2))
    (call $emit_x86_byte (i32.const 7))
    (call $emit_x86_byte (i32.const 0x66)) (call $emit_x86_byte (i32.const 0x0F)) (call $emit_x86_byte (i32.const 0x38))
    (call $emit_x86_byte (i32.const 0x1C))
    (call $emit_x86_modrm (i32.const 3) (i32.const 2) (i32.const 2))
    (call $emit_x86_byte (i32.const 0x66)) (call $emit_x86_byte (i32.const 0x0F)) (call $emit_x86_byte (i32.const 0xEF))
    (call $emit_x86_modrm (i32.const 3) (i32.const 0) (i32.const 2))
    (call $emit_x86_byte (i32.const 0x66)) (call $emit_x86_byte (i32.const 0x0F)) (call $emit_x86_byte (i32.const 0xEF))
    (call $emit_x86_modrm (i32.const 3) (i32.const 1) (i32.const 2))
    (call $emit_x86_byte (i32.const 0x66)) (call $emit_x86_byte (i32.const 0x0F)) (call $emit_x86_byte (i32.const 0x64))
    (call $emit_x86_modrm (i32.const 3) (i32.const 1) (i32.const 0))
    (call $emit_x86_byte (i32.const 0x66)) (call $emit_x86_byte (i32.const 0x0F)) (call $emit_x86_byte (i32.const 0x6F))
    (call $emit_x86_modrm (i32.const 3) (i32.const 0) (i32.const 1))
    (call $emit_x86_v128_push)
  )

  ;; i8x16.gt_u (0x4C): PCMPGTB(a^0x80, b^0x80)
  (func $template_simd_i8x16_gt_u (param $dec_ptr i32)
    (call $emit_x86_v128_pop_xmm1)
    (call $emit_x86_v128_pop)
    (call $emit_x86_byte (i32.const 0x66)) (call $emit_x86_byte (i32.const 0x0F)) (call $emit_x86_byte (i32.const 0xEF))
    (call $emit_x86_modrm (i32.const 3) (i32.const 2) (i32.const 2))
    (call $emit_x86_byte (i32.const 0x66)) (call $emit_x86_byte (i32.const 0x0F)) (call $emit_x86_byte (i32.const 0x76))
    (call $emit_x86_modrm (i32.const 3) (i32.const 2) (i32.const 2))
    (call $emit_x86_byte (i32.const 0x66)) (call $emit_x86_byte (i32.const 0x0F)) (call $emit_x86_byte (i32.const 0xF1))
    (call $emit_x86_modrm (i32.const 3) (i32.const 2) (i32.const 2))
    (call $emit_x86_byte (i32.const 7))
    (call $emit_x86_byte (i32.const 0x66)) (call $emit_x86_byte (i32.const 0x0F)) (call $emit_x86_byte (i32.const 0x38))
    (call $emit_x86_byte (i32.const 0x1C))
    (call $emit_x86_modrm (i32.const 3) (i32.const 2) (i32.const 2))
    (call $emit_x86_byte (i32.const 0x66)) (call $emit_x86_byte (i32.const 0x0F)) (call $emit_x86_byte (i32.const 0xEF))
    (call $emit_x86_modrm (i32.const 3) (i32.const 0) (i32.const 2))
    (call $emit_x86_byte (i32.const 0x66)) (call $emit_x86_byte (i32.const 0x0F)) (call $emit_x86_byte (i32.const 0xEF))
    (call $emit_x86_modrm (i32.const 3) (i32.const 1) (i32.const 2))
    (call $emit_x86_byte (i32.const 0x66)) (call $emit_x86_byte (i32.const 0x0F)) (call $emit_x86_byte (i32.const 0x64))
    (call $emit_x86_modrm (i32.const 3) (i32.const 0) (i32.const 1))
    (call $emit_x86_v128_push)
  )

  ;; i8x16.le_u (0x4E): NOT(gt_u)
  (func $template_simd_i8x16_le_u (param $dec_ptr i32)
    (call $emit_x86_v128_pop_xmm1)
    (call $emit_x86_v128_pop)
    (call $emit_x86_byte (i32.const 0x66)) (call $emit_x86_byte (i32.const 0x0F)) (call $emit_x86_byte (i32.const 0xEF))
    (call $emit_x86_modrm (i32.const 3) (i32.const 2) (i32.const 2))
    (call $emit_x86_byte (i32.const 0x66)) (call $emit_x86_byte (i32.const 0x0F)) (call $emit_x86_byte (i32.const 0x76))
    (call $emit_x86_modrm (i32.const 3) (i32.const 2) (i32.const 2))
    (call $emit_x86_byte (i32.const 0x66)) (call $emit_x86_byte (i32.const 0x0F)) (call $emit_x86_byte (i32.const 0xF1))
    (call $emit_x86_modrm (i32.const 3) (i32.const 2) (i32.const 2))
    (call $emit_x86_byte (i32.const 7))
    (call $emit_x86_byte (i32.const 0x66)) (call $emit_x86_byte (i32.const 0x0F)) (call $emit_x86_byte (i32.const 0x38))
    (call $emit_x86_byte (i32.const 0x1C))
    (call $emit_x86_modrm (i32.const 3) (i32.const 2) (i32.const 2))
    (call $emit_x86_byte (i32.const 0x66)) (call $emit_x86_byte (i32.const 0x0F)) (call $emit_x86_byte (i32.const 0xEF))
    (call $emit_x86_modrm (i32.const 3) (i32.const 0) (i32.const 2))
    (call $emit_x86_byte (i32.const 0x66)) (call $emit_x86_byte (i32.const 0x0F)) (call $emit_x86_byte (i32.const 0xEF))
    (call $emit_x86_modrm (i32.const 3) (i32.const 1) (i32.const 2))
    (call $emit_x86_byte (i32.const 0x66)) (call $emit_x86_byte (i32.const 0x0F)) (call $emit_x86_byte (i32.const 0x64))
    (call $emit_x86_modrm (i32.const 3) (i32.const 0) (i32.const 1))
    (call $emit_x86_v128_not)
    (call $emit_x86_v128_push)
  )

  ;; i8x16.ge_u (0xAE): NOT(lt_u)
  (func $template_simd_i8x16_ge_u (param $dec_ptr i32)
    (call $emit_x86_v128_pop_xmm1)
    (call $emit_x86_v128_pop)
    (call $emit_x86_byte (i32.const 0x66)) (call $emit_x86_byte (i32.const 0x0F)) (call $emit_x86_byte (i32.const 0xEF))
    (call $emit_x86_modrm (i32.const 3) (i32.const 2) (i32.const 2))
    (call $emit_x86_byte (i32.const 0x66)) (call $emit_x86_byte (i32.const 0x0F)) (call $emit_x86_byte (i32.const 0x76))
    (call $emit_x86_modrm (i32.const 3) (i32.const 2) (i32.const 2))
    (call $emit_x86_byte (i32.const 0x66)) (call $emit_x86_byte (i32.const 0x0F)) (call $emit_x86_byte (i32.const 0xF1))
    (call $emit_x86_modrm (i32.const 3) (i32.const 2) (i32.const 2))
    (call $emit_x86_byte (i32.const 7))
    (call $emit_x86_byte (i32.const 0x66)) (call $emit_x86_byte (i32.const 0x0F)) (call $emit_x86_byte (i32.const 0x38))
    (call $emit_x86_byte (i32.const 0x1C))
    (call $emit_x86_modrm (i32.const 3) (i32.const 2) (i32.const 2))
    (call $emit_x86_byte (i32.const 0x66)) (call $emit_x86_byte (i32.const 0x0F)) (call $emit_x86_byte (i32.const 0xEF))
    (call $emit_x86_modrm (i32.const 3) (i32.const 0) (i32.const 2))
    (call $emit_x86_byte (i32.const 0x66)) (call $emit_x86_byte (i32.const 0x0F)) (call $emit_x86_byte (i32.const 0xEF))
    (call $emit_x86_modrm (i32.const 3) (i32.const 1) (i32.const 2))
    (call $emit_x86_byte (i32.const 0x66)) (call $emit_x86_byte (i32.const 0x0F)) (call $emit_x86_byte (i32.const 0x64))
    (call $emit_x86_modrm (i32.const 3) (i32.const 1) (i32.const 0))
    (call $emit_x86_byte (i32.const 0x66)) (call $emit_x86_byte (i32.const 0x0F)) (call $emit_x86_byte (i32.const 0x6F))
    (call $emit_x86_modrm (i32.const 3) (i32.const 0) (i32.const 1))
    (call $emit_x86_v128_not)
    (call $emit_x86_v128_push)
  )

  ;; i16x8.lt_u (0x5B): PCMPGTW(b^0x8000, a^0x8000)
  (func $template_simd_i16x8_lt_u (param $dec_ptr i32)
    (call $emit_x86_v128_pop_xmm1)
    (call $emit_x86_v128_pop)
    (call $emit_x86_byte (i32.const 0x66)) (call $emit_x86_byte (i32.const 0x0F)) (call $emit_x86_byte (i32.const 0xEF))
    (call $emit_x86_modrm (i32.const 3) (i32.const 2) (i32.const 2))
    (call $emit_x86_byte (i32.const 0x66)) (call $emit_x86_byte (i32.const 0x0F)) (call $emit_x86_byte (i32.const 0x76))
    (call $emit_x86_modrm (i32.const 3) (i32.const 2) (i32.const 2))
    (call $emit_x86_byte (i32.const 0x66)) (call $emit_x86_byte (i32.const 0x0F)) (call $emit_x86_byte (i32.const 0xF1))
    (call $emit_x86_modrm (i32.const 3) (i32.const 2) (i32.const 2))
    (call $emit_x86_byte (i32.const 15))
    (call $emit_x86_byte (i32.const 0x66)) (call $emit_x86_byte (i32.const 0x0F)) (call $emit_x86_byte (i32.const 0xEF))
    (call $emit_x86_modrm (i32.const 3) (i32.const 0) (i32.const 2))
    (call $emit_x86_byte (i32.const 0x66)) (call $emit_x86_byte (i32.const 0x0F)) (call $emit_x86_byte (i32.const 0xEF))
    (call $emit_x86_modrm (i32.const 3) (i32.const 1) (i32.const 2))
    (call $emit_x86_byte (i32.const 0x66)) (call $emit_x86_byte (i32.const 0x0F)) (call $emit_x86_byte (i32.const 0x65))
    (call $emit_x86_modrm (i32.const 3) (i32.const 1) (i32.const 0))
    (call $emit_x86_byte (i32.const 0x66)) (call $emit_x86_byte (i32.const 0x0F)) (call $emit_x86_byte (i32.const 0x6F))
    (call $emit_x86_modrm (i32.const 3) (i32.const 0) (i32.const 1))
    (call $emit_x86_v128_push)
  )

  ;; i16x8.gt_u (0x5D): PCMPGTW(a^0x8000, b^0x8000)
  (func $template_simd_i16x8_gt_u (param $dec_ptr i32)
    (call $emit_x86_v128_pop_xmm1)
    (call $emit_x86_v128_pop)
    (call $emit_x86_byte (i32.const 0x66)) (call $emit_x86_byte (i32.const 0x0F)) (call $emit_x86_byte (i32.const 0xEF))
    (call $emit_x86_modrm (i32.const 3) (i32.const 2) (i32.const 2))
    (call $emit_x86_byte (i32.const 0x66)) (call $emit_x86_byte (i32.const 0x0F)) (call $emit_x86_byte (i32.const 0x76))
    (call $emit_x86_modrm (i32.const 3) (i32.const 2) (i32.const 2))
    (call $emit_x86_byte (i32.const 0x66)) (call $emit_x86_byte (i32.const 0x0F)) (call $emit_x86_byte (i32.const 0xF1))
    (call $emit_x86_modrm (i32.const 3) (i32.const 2) (i32.const 2))
    (call $emit_x86_byte (i32.const 15))
    (call $emit_x86_byte (i32.const 0x66)) (call $emit_x86_byte (i32.const 0x0F)) (call $emit_x86_byte (i32.const 0xEF))
    (call $emit_x86_modrm (i32.const 3) (i32.const 0) (i32.const 2))
    (call $emit_x86_byte (i32.const 0x66)) (call $emit_x86_byte (i32.const 0x0F)) (call $emit_x86_byte (i32.const 0xEF))
    (call $emit_x86_modrm (i32.const 3) (i32.const 1) (i32.const 2))
    (call $emit_x86_byte (i32.const 0x66)) (call $emit_x86_byte (i32.const 0x0F)) (call $emit_x86_byte (i32.const 0x65))
    (call $emit_x86_modrm (i32.const 3) (i32.const 0) (i32.const 1))
    (call $emit_x86_v128_push)
  )

  ;; i16x8.le_u (0x5F): NOT(gt_u)
  (func $template_simd_i16x8_le_u (param $dec_ptr i32)
    (call $emit_x86_v128_pop_xmm1)
    (call $emit_x86_v128_pop)
    (call $emit_x86_byte (i32.const 0x66)) (call $emit_x86_byte (i32.const 0x0F)) (call $emit_x86_byte (i32.const 0xEF))
    (call $emit_x86_modrm (i32.const 3) (i32.const 2) (i32.const 2))
    (call $emit_x86_byte (i32.const 0x66)) (call $emit_x86_byte (i32.const 0x0F)) (call $emit_x86_byte (i32.const 0x76))
    (call $emit_x86_modrm (i32.const 3) (i32.const 2) (i32.const 2))
    (call $emit_x86_byte (i32.const 0x66)) (call $emit_x86_byte (i32.const 0x0F)) (call $emit_x86_byte (i32.const 0xF1))
    (call $emit_x86_modrm (i32.const 3) (i32.const 2) (i32.const 2))
    (call $emit_x86_byte (i32.const 15))
    (call $emit_x86_byte (i32.const 0x66)) (call $emit_x86_byte (i32.const 0x0F)) (call $emit_x86_byte (i32.const 0xEF))
    (call $emit_x86_modrm (i32.const 3) (i32.const 0) (i32.const 2))
    (call $emit_x86_byte (i32.const 0x66)) (call $emit_x86_byte (i32.const 0x0F)) (call $emit_x86_byte (i32.const 0xEF))
    (call $emit_x86_modrm (i32.const 3) (i32.const 1) (i32.const 2))
    (call $emit_x86_byte (i32.const 0x66)) (call $emit_x86_byte (i32.const 0x0F)) (call $emit_x86_byte (i32.const 0x65))
    (call $emit_x86_modrm (i32.const 3) (i32.const 0) (i32.const 1))
    (call $emit_x86_v128_not)
    (call $emit_x86_v128_push)
  )

  ;; i16x8.ge_u (0xAF): NOT(lt_u)
  (func $template_simd_i16x8_ge_u (param $dec_ptr i32)
    (call $emit_x86_v128_pop_xmm1)
    (call $emit_x86_v128_pop)
    (call $emit_x86_byte (i32.const 0x66)) (call $emit_x86_byte (i32.const 0x0F)) (call $emit_x86_byte (i32.const 0xEF))
    (call $emit_x86_modrm (i32.const 3) (i32.const 2) (i32.const 2))
    (call $emit_x86_byte (i32.const 0x66)) (call $emit_x86_byte (i32.const 0x0F)) (call $emit_x86_byte (i32.const 0x76))
    (call $emit_x86_modrm (i32.const 3) (i32.const 2) (i32.const 2))
    (call $emit_x86_byte (i32.const 0x66)) (call $emit_x86_byte (i32.const 0x0F)) (call $emit_x86_byte (i32.const 0xF1))
    (call $emit_x86_modrm (i32.const 3) (i32.const 2) (i32.const 2))
    (call $emit_x86_byte (i32.const 15))
    (call $emit_x86_byte (i32.const 0x66)) (call $emit_x86_byte (i32.const 0x0F)) (call $emit_x86_byte (i32.const 0xEF))
    (call $emit_x86_modrm (i32.const 3) (i32.const 0) (i32.const 2))
    (call $emit_x86_byte (i32.const 0x66)) (call $emit_x86_byte (i32.const 0x0F)) (call $emit_x86_byte (i32.const 0xEF))
    (call $emit_x86_modrm (i32.const 3) (i32.const 1) (i32.const 2))
    (call $emit_x86_byte (i32.const 0x66)) (call $emit_x86_byte (i32.const 0x0F)) (call $emit_x86_byte (i32.const 0x65))
    (call $emit_x86_modrm (i32.const 3) (i32.const 1) (i32.const 0))
    (call $emit_x86_byte (i32.const 0x66)) (call $emit_x86_byte (i32.const 0x0F)) (call $emit_x86_byte (i32.const 0x6F))
    (call $emit_x86_modrm (i32.const 3) (i32.const 0) (i32.const 1))
    (call $emit_x86_v128_not)
    (call $emit_x86_v128_push)
  )

  ;; i32x4.lt_u (0x6C): PCMPGTD(b^0x80000000, a^0x80000000)
  (func $template_simd_i32x4_lt_u (param $dec_ptr i32)
    (call $emit_x86_v128_pop_xmm1)
    (call $emit_x86_v128_pop)
    (call $emit_x86_byte (i32.const 0x66)) (call $emit_x86_byte (i32.const 0x0F)) (call $emit_x86_byte (i32.const 0xEF))
    (call $emit_x86_modrm (i32.const 3) (i32.const 2) (i32.const 2))
    (call $emit_x86_byte (i32.const 0x66)) (call $emit_x86_byte (i32.const 0x0F)) (call $emit_x86_byte (i32.const 0x76))
    (call $emit_x86_modrm (i32.const 3) (i32.const 2) (i32.const 2))
    (call $emit_x86_byte (i32.const 0x66)) (call $emit_x86_byte (i32.const 0x0F)) (call $emit_x86_byte (i32.const 0xF2))
    (call $emit_x86_modrm (i32.const 3) (i32.const 2) (i32.const 2))
    (call $emit_x86_byte (i32.const 31))
    (call $emit_x86_byte (i32.const 0x66)) (call $emit_x86_byte (i32.const 0x0F)) (call $emit_x86_byte (i32.const 0xEF))
    (call $emit_x86_modrm (i32.const 3) (i32.const 0) (i32.const 2))
    (call $emit_x86_byte (i32.const 0x66)) (call $emit_x86_byte (i32.const 0x0F)) (call $emit_x86_byte (i32.const 0xEF))
    (call $emit_x86_modrm (i32.const 3) (i32.const 1) (i32.const 2))
    (call $emit_x86_byte (i32.const 0x66)) (call $emit_x86_byte (i32.const 0x0F)) (call $emit_x86_byte (i32.const 0x66))
    (call $emit_x86_modrm (i32.const 3) (i32.const 1) (i32.const 0))
    (call $emit_x86_byte (i32.const 0x66)) (call $emit_x86_byte (i32.const 0x0F)) (call $emit_x86_byte (i32.const 0x6F))
    (call $emit_x86_modrm (i32.const 3) (i32.const 0) (i32.const 1))
    (call $emit_x86_v128_push)
  )

  ;; i32x4.gt_u (0x6E): PCMPGTD(a^0x80000000, b^0x80000000)
  (func $template_simd_i32x4_gt_u (param $dec_ptr i32)
    (call $emit_x86_v128_pop_xmm1)
    (call $emit_x86_v128_pop)
    (call $emit_x86_byte (i32.const 0x66)) (call $emit_x86_byte (i32.const 0x0F)) (call $emit_x86_byte (i32.const 0xEF))
    (call $emit_x86_modrm (i32.const 3) (i32.const 2) (i32.const 2))
    (call $emit_x86_byte (i32.const 0x66)) (call $emit_x86_byte (i32.const 0x0F)) (call $emit_x86_byte (i32.const 0x76))
    (call $emit_x86_modrm (i32.const 3) (i32.const 2) (i32.const 2))
    (call $emit_x86_byte (i32.const 0x66)) (call $emit_x86_byte (i32.const 0x0F)) (call $emit_x86_byte (i32.const 0xF2))
    (call $emit_x86_modrm (i32.const 3) (i32.const 2) (i32.const 2))
    (call $emit_x86_byte (i32.const 31))
    (call $emit_x86_byte (i32.const 0x66)) (call $emit_x86_byte (i32.const 0x0F)) (call $emit_x86_byte (i32.const 0xEF))
    (call $emit_x86_modrm (i32.const 3) (i32.const 0) (i32.const 2))
    (call $emit_x86_byte (i32.const 0x66)) (call $emit_x86_byte (i32.const 0x0F)) (call $emit_x86_byte (i32.const 0xEF))
    (call $emit_x86_modrm (i32.const 3) (i32.const 1) (i32.const 2))
    (call $emit_x86_byte (i32.const 0x66)) (call $emit_x86_byte (i32.const 0x0F)) (call $emit_x86_byte (i32.const 0x66))
    (call $emit_x86_modrm (i32.const 3) (i32.const 0) (i32.const 1))
    (call $emit_x86_v128_push)
  )

  ;; i32x4.le_u (0x70): NOT(gt_u)
  (func $template_simd_i32x4_le_u (param $dec_ptr i32)
    (call $emit_x86_v128_pop_xmm1)
    (call $emit_x86_v128_pop)
    (call $emit_x86_byte (i32.const 0x66)) (call $emit_x86_byte (i32.const 0x0F)) (call $emit_x86_byte (i32.const 0xEF))
    (call $emit_x86_modrm (i32.const 3) (i32.const 2) (i32.const 2))
    (call $emit_x86_byte (i32.const 0x66)) (call $emit_x86_byte (i32.const 0x0F)) (call $emit_x86_byte (i32.const 0x76))
    (call $emit_x86_modrm (i32.const 3) (i32.const 2) (i32.const 2))
    (call $emit_x86_byte (i32.const 0x66)) (call $emit_x86_byte (i32.const 0x0F)) (call $emit_x86_byte (i32.const 0xF2))
    (call $emit_x86_modrm (i32.const 3) (i32.const 2) (i32.const 2))
    (call $emit_x86_byte (i32.const 31))
    (call $emit_x86_byte (i32.const 0x66)) (call $emit_x86_byte (i32.const 0x0F)) (call $emit_x86_byte (i32.const 0xEF))
    (call $emit_x86_modrm (i32.const 3) (i32.const 0) (i32.const 2))
    (call $emit_x86_byte (i32.const 0x66)) (call $emit_x86_byte (i32.const 0x0F)) (call $emit_x86_byte (i32.const 0xEF))
    (call $emit_x86_modrm (i32.const 3) (i32.const 1) (i32.const 2))
    (call $emit_x86_byte (i32.const 0x66)) (call $emit_x86_byte (i32.const 0x0F)) (call $emit_x86_byte (i32.const 0x66))
    (call $emit_x86_modrm (i32.const 3) (i32.const 0) (i32.const 1))
    (call $emit_x86_v128_not)
    (call $emit_x86_v128_push)
  )

  ;; i32x4.ge_u (0xB0): NOT(lt_u)
  (func $template_simd_i32x4_ge_u (param $dec_ptr i32)
    (call $emit_x86_v128_pop_xmm1)
    (call $emit_x86_v128_pop)
    (call $emit_x86_byte (i32.const 0x66)) (call $emit_x86_byte (i32.const 0x0F)) (call $emit_x86_byte (i32.const 0xEF))
    (call $emit_x86_modrm (i32.const 3) (i32.const 2) (i32.const 2))
    (call $emit_x86_byte (i32.const 0x66)) (call $emit_x86_byte (i32.const 0x0F)) (call $emit_x86_byte (i32.const 0x76))
    (call $emit_x86_modrm (i32.const 3) (i32.const 2) (i32.const 2))
    (call $emit_x86_byte (i32.const 0x66)) (call $emit_x86_byte (i32.const 0x0F)) (call $emit_x86_byte (i32.const 0xF2))
    (call $emit_x86_modrm (i32.const 3) (i32.const 2) (i32.const 2))
    (call $emit_x86_byte (i32.const 31))
    (call $emit_x86_byte (i32.const 0x66)) (call $emit_x86_byte (i32.const 0x0F)) (call $emit_x86_byte (i32.const 0xEF))
    (call $emit_x86_modrm (i32.const 3) (i32.const 0) (i32.const 2))
    (call $emit_x86_byte (i32.const 0x66)) (call $emit_x86_byte (i32.const 0x0F)) (call $emit_x86_byte (i32.const 0xEF))
    (call $emit_x86_modrm (i32.const 3) (i32.const 1) (i32.const 2))
    (call $emit_x86_byte (i32.const 0x66)) (call $emit_x86_byte (i32.const 0x0F)) (call $emit_x86_byte (i32.const 0x66))
    (call $emit_x86_modrm (i32.const 3) (i32.const 1) (i32.const 0))
    (call $emit_x86_byte (i32.const 0x66)) (call $emit_x86_byte (i32.const 0x0F)) (call $emit_x86_byte (i32.const 0x6F))
    (call $emit_x86_modrm (i32.const 3) (i32.const 0) (i32.const 1))
    (call $emit_x86_v128_not)
    (call $emit_x86_v128_push)
  )

  ;; ═════════════════════════════════════════════════════════════════════
  ;; v128.bitselect — SSE4.1 PBLENDVB
  ;; ═════════════════════════════════════════════════════════════════════
  ;; PBLENDVB: 66 0F 38 10 /r — selects from xmm1 based on sign bits of
  ;; negation mask in xmm0 (implicit): if xmm0[i] bit 7 set, select from
  ;; xmm2, else from xmm1.
  ;; Stack: pop if_false, pop if_true, pop mask
  ;; PBLENDVB: mask in xmm0 (implicit), if_false in xmm1, if_true in xmm2
  (func $template_simd_v128_bitselect (param $dec_ptr i32)
    ;; Pop if_false → xmm0
    (call $emit_x86_v128_pop)
    ;; Pop if_true → xmm1
    (call $emit_x86_v128_pop_xmm1)
    ;; Pop mask → xmm0 (overwrites if_false, but we have it in xmm1)
    (call $emit_x86_v128_pop)
    ;; Now: mask=xmm0, if_true=xmm1, if_false=on stack
    ;; Pop if_false → xmm2 (but no helper, inline: movdqu xmm2, [rsp]; add rsp,16)
    (call $emit_x86_byte (i32.const 0xF3))
    (call $emit_x86_byte (i32.const 0x0F))
    (call $emit_x86_byte (i32.const 0x6F))
    (call $emit_x86_modrm (i32.const 0) (i32.const 2) (i32.const 4))
    (call $emit_x86_sib (i32.const 0) (i32.const 4) (i32.const 4))
    (call $emit_x86_rex_w)
    (call $emit_x86_byte (i32.const 0x83))
    (call $emit_x86_modrm (i32.const 3) (i32.const 0) (i32.const 4))
    (call $emit_x86_byte (i32.const 16))
    ;; PBLENDVB xmm2, xmm1 (mask in xmm0 implicit): 66 0F 38 10 /r
    ;; ModRM: reg = xmm1 (if_true), rm = xmm2 (if_false)
    ;; Actually PBLENDVB: xmm2 = (mask[i] & 0x80) ? xmm1[i] : xmm2[i]
    ;; So bits from if_true selected where mask sign bit set
    (call $emit_x86_byte (i32.const 0x66)) (call $emit_x86_byte (i32.const 0x0F)) (call $emit_x86_byte (i32.const 0x38))
    (call $emit_x86_byte (i32.const 0x10))
    (call $emit_x86_modrm (i32.const 3) (i32.const 1) (i32.const 2))
    (call $emit_x86_byte (i32.const 0x66)) (call $emit_x86_byte (i32.const 0x0F)) (call $emit_x86_byte (i32.const 0x6F))
    (call $emit_x86_modrm (i32.const 3) (i32.const 0) (i32.const 2))
    (call $emit_x86_v128_push)
  )

  ;; ── SIMD unsupported stub ─────────────────────────────────────────
  (func $template_simd_unsupported (param $dec_ptr i32)
    (call $template_x86_unsupported)
  )

  ;; ═════════════════════════════════════════════════════════════════════
  ;; JIT compile: compile a WASM function to x86_64 machine code
  ;; ═════════════════════════════════════════════════════════════════════


  (func $jit_compile (export "jit_compile") (param $func_idx i32) (result i32)
    (local $slot i32) (local $cache_base i32) (local $cache_end i32)
    (local $dec_start i32) (local $dec_end i32) (local $dec_ptr i32)
    (local $op i32) (local $imm0 i32)
    (local $result_count i32) (local $code_off i32)
    (local $i i32) (local $count i32) (local $save_off i32)
    (local $local_idx i32) (local $import_count i32)

    ;; Compute slot = func_idx & 3
    (local.set $slot (i32.and (local.get $func_idx) (i32.const 3)))
    (local.set $cache_base (i32.add (global.get $JIT_CACHE) (i32.mul (local.get $slot) (global.get $JIT_SLOT_SIZE))))
    (local.set $cache_end (i32.add (local.get $cache_base) (global.get $JIT_SLOT_SIZE)))

    ;; Reset JIT state
    (call $jit_reset_state)
    (i32.store (global.get $JS_CACHE_BASE) (local.get $cache_base))
    (i32.store (global.get $JS_CACHE_END) (local.get $cache_end))
    (i32.store (global.get $JS_FUNC_IDX) (local.get $func_idx))

    ;; Convert global func_idx to local function index (subtract imports)
    (local.set $import_count (i32.load (global.get $OFF_IMPORT_COUNT)))
    (local.set $local_idx (i32.sub (local.get $func_idx) (local.get $import_count)))

    ;; Get result_count from function type (16-bit at offset 136 in type entry)
    (local.set $code_off (i32.mul (local.get $local_idx) (i32.const 16)))
    (local.set $code_off (i32.shl (i32.load (i32.add (global.get $OFF_FUNCTIONS_BUF) (local.get $code_off))) (i32.const 8)))
    (local.set $result_count (i32.load16_u (i32.add (i32.add (global.get $OFF_TYPES_BUF) (local.get $code_off)) (i32.const 136))))
    (i32.store (global.get $JS_RESULT_COUNT) (local.get $result_count))

    ;; Get decoded ops range
    (local.set $code_off (i32.mul (local.get $local_idx) (global.get $SZ_CODE)))
    (local.set $dec_start (i32.load (i32.add (i32.add (global.get $OFF_CODE_BUF) (local.get $code_off)) (i32.const 24))))
    (local.set $dec_end (i32.add (local.get $dec_start) (i32.load (i32.add (i32.add (global.get $OFF_CODE_BUF) (local.get $code_off)) (i32.const 32)))))
    (local.set $dec_ptr (i32.add (global.get $OFF_DECODED_OPS) (i32.mul (local.get $dec_start) (global.get $DEC_SZ))))

    ;; ── Emit function prologue ──
    (call $emit_x86_prologue)

    ;; Cache JitGlobals.locals pointer (at [r15+0]) in rbx for fast local access
    (call $emit_x86_mov_rbx_r15)

    ;; Cache first two locals in r12/r13 (register-allocated)
    (call $emit_x86_load_r12_rbx)
    (call $emit_x86_load_r13_rbx8)

    ;; ── Compile loop ──
    (block $compile_done
      (loop $compile_loop
        ;; Check if done
        (if (i32.ge_u (local.get $dec_start) (local.get $dec_end))
          (then (br $compile_done))
        )

        ;; Read opcode from decoded_op: opcode(1) + pad(3) + imm0(4) + imm1(4) = 12
        (local.set $op (i32.load8_u (local.get $dec_ptr)))
        (local.set $imm0 (i32.load (i32.add (local.get $dec_ptr) (i32.const 4))))
        (global.set $CURRENT_DEC_PTR (local.get $dec_ptr))

        ;; ── Control flow: handle directly ──

        ;; block (0x02)
        (if (i32.eq (local.get $op) (i32.const 0x02))
          (then
            (call $push_label (global.get $JIT_LABEL_BLOCK))
            (local.set $dec_start (i32.add (local.get $dec_start) (i32.const 1)))
            (local.set $dec_ptr (i32.add (local.get $dec_ptr) (global.get $DEC_SZ)))
            (br $compile_loop)
          )
        )

        ;; loop (0x03)
        (if (i32.eq (local.get $op) (i32.const 0x03))
          (then
            (call $push_label (global.get $JIT_LABEL_LOOP))
            (local.set $save_off (i32.load (global.get $JS_CODE_PTR)))
            (call $set_label_offset (i32.sub (i32.load (global.get $JS_LABEL_DEPTH)) (i32.const 1)) (local.get $save_off))
            (local.set $dec_start (i32.add (local.get $dec_start) (i32.const 1)))
            (local.set $dec_ptr (i32.add (local.get $dec_ptr) (global.get $DEC_SZ)))
            (br $compile_loop)
          )
        )

        ;; if (0x04)
        (if (i32.eq (local.get $op) (i32.const 0x04))
          (then
            ;; pop condition, test, jz with placeholder
            (call $emit_x86_pop_rax)
            (call $emit_x86_test_eax)
            (call $emit_x86_jz_rel32 (i32.const 0))  ;; placeholder disp
            (local.set $save_off (i32.sub (i32.load (global.get $JS_CODE_PTR)) (i32.const 4)))  ;; address of disp
            (local.set $i (i32.load (global.get $JS_LABEL_DEPTH)))
            (call $set_label_if_jz (local.get $i) (local.get $save_off))
            (call $push_label (global.get $JIT_LABEL_IF))
            (local.set $dec_start (i32.add (local.get $dec_start) (i32.const 1)))
            (local.set $dec_ptr (i32.add (local.get $dec_ptr) (global.get $DEC_SZ)))
            (br $compile_loop)
          )
        )

        ;; else (0x05)
        (if (i32.eq (local.get $op) (i32.const 0x05))
          (then
            (local.set $i (i32.sub (i32.load (global.get $JS_LABEL_DEPTH)) (i32.const 1)))
            ;; Patch if's jz to jump here
            (local.set $save_off (call $get_label_if_jz (local.get $i)))
            (if (i32.ne (local.get $save_off) (i32.const 0))
              (then
                ;; save_off is offset from JIT_CACHE
                (i32.store
                  (i32.add (global.get $JIT_CACHE) (local.get $save_off))
                  (i32.sub (i32.load (global.get $JS_CODE_PTR)) (i32.add (local.get $save_off) (i32.const 4)))
                )
                (call $set_label_if_jz (local.get $i) (i32.const 0))
              )
            )
            ;; Emit jmp to end (placeholder) and record fixup
            (call $emit_x86_jmp_rel32 (i32.const 0))
            (local.set $save_off (i32.sub (i32.load (global.get $JS_CODE_PTR)) (i32.const 4)))
            (call $push_fixup (local.get $i))
            ;; Also save the fixup offset override
            (i32.store (i32.add (global.get $JS_FIXUP_OFFSET) (i32.shl (i32.sub (i32.load (global.get $JS_FIXUP_COUNT)) (i32.const 1)) (i32.const 2))) (local.get $save_off))
            (local.set $dec_start (i32.add (local.get $dec_start) (i32.const 1)))
            (local.set $dec_ptr (i32.add (local.get $dec_ptr) (global.get $DEC_SZ)))
            (br $compile_loop)
          )
        )

        ;; end (0x0B)
        (if (i32.eq (local.get $op) (i32.const 0x0B))
          (then
            (local.set $i (call $pop_label))
            (if (i32.eq (call $get_label_kind (local.get $i)) (global.get $JIT_LABEL_LOOP))
              (then
                ;; Loop end: jmp back to loop start
                ;; disp = target - (code_ptr + 5)  (jmp_rel32 is 5 bytes)
                (local.set $save_off (call $get_label_offset (local.get $i)))
                (call $emit_x86_jmp_rel32
                  (i32.sub (local.get $save_off) (i32.add (i32.load (global.get $JS_CODE_PTR)) (i32.const 5)))
                )
              )
              (else
                ;; Block/IF end: patch fixups
                (call $patch_fixups (local.get $i))
                ;; If IF (no else), patch jz
                (if (i32.eq (call $get_label_kind (local.get $i)) (global.get $JIT_LABEL_IF))
                  (then
                    (local.set $save_off (call $get_label_if_jz (local.get $i)))
                    (if (i32.ne (local.get $save_off) (i32.const 0))
                      (then
                        ;; save_off is offset from JIT_CACHE
                        (i32.store
                          (i32.add (global.get $JIT_CACHE) (local.get $save_off))
                          (i32.sub (i32.load (global.get $JS_CODE_PTR)) (i32.add (local.get $save_off) (i32.const 4)))
                        )
                      )
                    )
                  )
                )
              )
            )
            (local.set $dec_start (i32.add (local.get $dec_start) (i32.const 1)))
            (local.set $dec_ptr (i32.add (local.get $dec_ptr) (global.get $DEC_SZ)))
            (br $compile_loop)
          )
        )

        ;; br (0x0C)
        (if (i32.eq (local.get $op) (i32.const 0x0C))
          (then
            ;; imm0 = WASM label depth (0 = innermost)
            ;; Target label index = label_depth - 1 - imm0
            (local.set $i (i32.sub (i32.sub (i32.load (global.get $JS_LABEL_DEPTH)) (i32.const 1)) (local.get $imm0)))
            (if (i32.eq (call $get_label_kind (local.get $i)) (global.get $JIT_LABEL_LOOP))
              (then
                ;; Backward branch: compute disp directly
                ;; disp = target - (code_ptr + 5)  (jmp_rel32 is 5 bytes)
                (local.set $save_off (call $get_label_offset (local.get $i)))
                (call $emit_x86_jmp_rel32
                  (i32.sub (local.get $save_off) (i32.add (i32.load (global.get $JS_CODE_PTR)) (i32.const 5)))
                )
              )
              (else
                ;; Forward branch: emit jmp + fixup
                (call $emit_x86_jmp_rel32 (i32.const 0))
                (local.set $save_off (i32.sub (i32.load (global.get $JS_CODE_PTR)) (i32.const 4)))
                (call $push_fixup (local.get $i))
                (i32.store (i32.add (global.get $JS_FIXUP_OFFSET) (i32.shl (i32.sub (i32.load (global.get $JS_FIXUP_COUNT)) (i32.const 1)) (i32.const 2))) (local.get $save_off))
              )
            )
            (local.set $dec_start (i32.add (local.get $dec_start) (i32.const 1)))
            (local.set $dec_ptr (i32.add (local.get $dec_ptr) (global.get $DEC_SZ)))
            (br $compile_loop)
          )
        )

        ;; br_if (0x0D)
        (if (i32.eq (local.get $op) (i32.const 0x0D))
          (then
            (call $emit_x86_pop_rax)
            (call $emit_x86_test_eax)
            (local.set $i (i32.sub (i32.sub (i32.load (global.get $JS_LABEL_DEPTH)) (i32.const 1)) (local.get $imm0)))
            (if (i32.eq (call $get_label_kind (local.get $i)) (global.get $JIT_LABEL_LOOP))
              (then
                ;; disp = target - (code_ptr + 6)  (jne_rel32 is 6 bytes)
                (local.set $save_off (call $get_label_offset (local.get $i)))
                (call $emit_x86_jne_rel32
                  (i32.sub (local.get $save_off) (i32.add (i32.load (global.get $JS_CODE_PTR)) (i32.const 6)))
                )
              )
              (else
                (call $emit_x86_jne_rel32 (i32.const 0))
                (local.set $save_off (i32.sub (i32.load (global.get $JS_CODE_PTR)) (i32.const 4)))
                (call $push_fixup (local.get $i))
                (i32.store (i32.add (global.get $JS_FIXUP_OFFSET) (i32.shl (i32.sub (i32.load (global.get $JS_FIXUP_COUNT)) (i32.const 1)) (i32.const 2))) (local.get $save_off))
              )
            )
            (local.set $dec_start (i32.add (local.get $dec_start) (i32.const 1)))
            (local.set $dec_ptr (i32.add (local.get $dec_ptr) (global.get $DEC_SZ)))
            (br $compile_loop)
          )
        )

        ;; return (0x0F)
        (if (i32.eq (local.get $op) (i32.const 0x0F))
          (then
            (if (i32.eq (i32.load (global.get $JS_RESULT_COUNT)) (i32.const 1))
              (then
                (call $emit_x86_pop_rax)
              )
              (else
                (call $emit_x86_xor_eax_eax)
              )
            )
            (call $emit_x86_xor_edx_edx)
            (call $emit_x86_epilogue)
            (call $emit_x86_ud2)
            (i32.store (global.get $JS_RETURN_EMITTED) (i32.const 1))
            (local.set $dec_start (i32.add (local.get $dec_start) (i32.const 1)))
            (local.set $dec_ptr (i32.add (local.get $dec_ptr) (global.get $DEC_SZ)))
            (br $compile_loop)
          )
        )

        ;; ── 0xFC prefixed ops (trunc_sat, memory/table) ──
        (if (i32.eq (local.get $op) (i32.const 0xFC))
          (then
            (block $fc_done
              ;; i32.trunc_sat_f32_s (0xFC, 0x00)
              (if (i32.eq (local.get $imm0) (i32.const 0x00))
                (then (call $template_x86_i32_trunc_sat_f32_s) (br $fc_done))
              )
              ;; i32.trunc_sat_f32_u (0xFC, 0x01)
              (if (i32.eq (local.get $imm0) (i32.const 0x01))
                (then (call $template_x86_i32_trunc_sat_f32_u) (br $fc_done))
              )
              ;; i32.trunc_sat_f64_s (0xFC, 0x02)
              (if (i32.eq (local.get $imm0) (i32.const 0x02))
                (then (call $template_x86_i32_trunc_sat_f64_s) (br $fc_done))
              )
              ;; i32.trunc_sat_f64_u (0xFC, 0x03)
              (if (i32.eq (local.get $imm0) (i32.const 0x03))
                (then (call $template_x86_i32_trunc_sat_f64_u) (br $fc_done))
              )
              ;; i64.trunc_sat_f32_s (0xFC, 0x04)
              (if (i32.eq (local.get $imm0) (i32.const 0x04))
                (then (call $template_x86_i64_trunc_sat_f32_s) (br $fc_done))
              )
              ;; i64.trunc_sat_f32_u (0xFC, 0x05)
              (if (i32.eq (local.get $imm0) (i32.const 0x05))
                (then (call $template_x86_i64_trunc_sat_f32_u) (br $fc_done))
              )
              ;; i64.trunc_sat_f64_s (0xFC, 0x06)
              (if (i32.eq (local.get $imm0) (i32.const 0x06))
                (then (call $template_x86_i64_trunc_sat_f64_s) (br $fc_done))
              )
              ;; i64.trunc_sat_f64_u (0xFC, 0x07)
              (if (i32.eq (local.get $imm0) (i32.const 0x07))
                (then (call $template_x86_i64_trunc_sat_f64_u) (br $fc_done))
              )
              ;; memory.init / data.drop / memory.copy / memory.fill / table.init /
              ;; table.drop / table.copy / table.fill / table.get / table.set /
              ;; table.grow / table.size (0x08-0x11) — unsupported
              (call $template_x86_unsupported)
            )
            (local.set $dec_start (i32.add (local.get $dec_start) (i32.const 1)))
            (local.set $dec_ptr (i32.add (local.get $dec_ptr) (global.get $DEC_SZ)))
            (br $compile_loop)
          )
        )

        ;; ── 0xFD prefixed ops (WASM SIMD) ──
        ;; Sub-opcode encoding uses canonical IDs (AArch64 old encoding).
        ;; The decoder in interpreter.wat translates modern wabt → canonical.
        (if (i32.eq (local.get $op) (i32.const 0xFD))
          (then
            (local.set $imm0 (i32.load (i32.add (local.get $dec_ptr) (i32.const 4))))
            (block $fd_done
              ;; ── Memory ops ──
              (if (i32.eq (local.get $imm0) (i32.const 0x00)) (then (call $template_simd_v128_load (local.get $dec_ptr)) (br $fd_done)))
              (if (i32.eq (local.get $imm0) (i32.const 0x0C)) (then (call $template_simd_v128_const (local.get $dec_ptr)) (br $fd_done)))
              (if (i32.eq (local.get $imm0) (i32.const 0x1B)) (then (call $template_simd_v128_store (local.get $dec_ptr)) (br $fd_done)))
              (if (i32.eq (local.get $imm0) (i32.const 0xA6)) (then (call $template_simd_v128_load8_splat (local.get $dec_ptr)) (br $fd_done)))
              (if (i32.eq (local.get $imm0) (i32.const 0xA7)) (then (call $template_simd_v128_load16_splat (local.get $dec_ptr)) (br $fd_done)))
              (if (i32.eq (local.get $imm0) (i32.const 0xA8)) (then (call $template_simd_v128_load32_splat (local.get $dec_ptr)) (br $fd_done)))
              (if (i32.eq (local.get $imm0) (i32.const 0xA9)) (then (call $template_simd_v128_load64_splat (local.get $dec_ptr)) (br $fd_done)))
              ;; Swizzle
              (if (i32.eq (local.get $imm0) (i32.const 0xAA)) (then (call $template_simd_i8x16_swizzle (local.get $dec_ptr)) (br $fd_done)))
              ;; ── Splats (0x2D, 0x31, 0x35, 0x39, 0x3A, 0x3D) ──
              (if (i32.eq (local.get $imm0) (i32.const 0x2D)) (then (call $template_simd_i8x16_splat (local.get $dec_ptr)) (br $fd_done)))
              (if (i32.eq (local.get $imm0) (i32.const 0x31)) (then (call $template_simd_i16x8_splat (local.get $dec_ptr)) (br $fd_done)))
              (if (i32.eq (local.get $imm0) (i32.const 0x35)) (then (call $template_simd_i32x4_splat (local.get $dec_ptr)) (br $fd_done)))
              (if (i32.eq (local.get $imm0) (i32.const 0x39)) (then (call $template_simd_i64x2_splat (local.get $dec_ptr)) (br $fd_done)))
              (if (i32.eq (local.get $imm0) (i32.const 0x3A)) (then (call $template_simd_f32x4_splat (local.get $dec_ptr)) (br $fd_done)))
              (if (i32.eq (local.get $imm0) (i32.const 0x3D)) (then (call $template_simd_f64x2_splat (local.get $dec_ptr)) (br $fd_done)))
              ;; ── Extract/replace lanes (0x2E-0x30, 0x32-0x34, 0x36-0x38, 0xAB, 0x3B-0x3C, 0x3E-0x3F) ──
              (if (i32.eq (local.get $imm0) (i32.const 0x2E)) (then (call $template_simd_i8x16_extract_lane_s (local.get $dec_ptr)) (br $fd_done)))
              (if (i32.eq (local.get $imm0) (i32.const 0x2F)) (then (call $template_simd_i8x16_extract_lane_u (local.get $dec_ptr)) (br $fd_done)))
              (if (i32.eq (local.get $imm0) (i32.const 0x30)) (then (call $template_simd_i8x16_replace_lane (local.get $dec_ptr)) (br $fd_done)))
              (if (i32.eq (local.get $imm0) (i32.const 0x32)) (then (call $template_simd_i16x8_extract_lane_s (local.get $dec_ptr)) (br $fd_done)))
              (if (i32.eq (local.get $imm0) (i32.const 0x33)) (then (call $template_simd_i16x8_extract_lane_u (local.get $dec_ptr)) (br $fd_done)))
              (if (i32.eq (local.get $imm0) (i32.const 0x34)) (then (call $template_simd_i16x8_replace_lane (local.get $dec_ptr)) (br $fd_done)))
              (if (i32.eq (local.get $imm0) (i32.const 0x36)) (then (call $template_simd_i32x4_extract_lane (local.get $dec_ptr)) (br $fd_done)))
              (if (i32.eq (local.get $imm0) (i32.const 0x37)) (then (call $template_simd_i32x4_replace_lane (local.get $dec_ptr)) (br $fd_done)))
              (if (i32.eq (local.get $imm0) (i32.const 0x38)) (then (call $template_simd_i64x2_extract_lane (local.get $dec_ptr)) (br $fd_done)))
              (if (i32.eq (local.get $imm0) (i32.const 0xAB)) (then (call $template_simd_i64x2_replace_lane (local.get $dec_ptr)) (br $fd_done)))
              (if (i32.eq (local.get $imm0) (i32.const 0x3B)) (then (call $template_simd_f32x4_extract_lane (local.get $dec_ptr)) (br $fd_done)))
              (if (i32.eq (local.get $imm0) (i32.const 0x3C)) (then (call $template_simd_f32x4_replace_lane (local.get $dec_ptr)) (br $fd_done)))
              (if (i32.eq (local.get $imm0) (i32.const 0x3E)) (then (call $template_simd_f64x2_extract_lane (local.get $dec_ptr)) (br $fd_done)))
              (if (i32.eq (local.get $imm0) (i32.const 0x3F)) (then (call $template_simd_f64x2_replace_lane (local.get $dec_ptr)) (br $fd_done)))
              ;; ── i8x16 arithmetic ──
              (if (i32.eq (local.get $imm0) (i32.const 0x40)) (then (call $template_simd_i8x16_add (local.get $dec_ptr)) (br $fd_done)))
              (if (i32.eq (local.get $imm0) (i32.const 0x41)) (then (call $template_simd_i8x16_sub (local.get $dec_ptr)) (br $fd_done)))
              (if (i32.eq (local.get $imm0) (i32.const 0x46)) (then (call $template_simd_i8x16_neg (local.get $dec_ptr)) (br $fd_done)))
              ;; ── i8x16 comparisons ──
              (if (i32.eq (local.get $imm0) (i32.const 0x47)) (then (call $template_simd_i8x16_eq (local.get $dec_ptr)) (br $fd_done)))
              (if (i32.eq (local.get $imm0) (i32.const 0x48)) (then (call $template_simd_i8x16_ne (local.get $dec_ptr)) (br $fd_done)))
              (if (i32.eq (local.get $imm0) (i32.const 0x49)) (then (call $template_simd_i8x16_ge_s (local.get $dec_ptr)) (br $fd_done)))
              (if (i32.eq (local.get $imm0) (i32.const 0x4B)) (then (call $template_simd_i8x16_lt_s (local.get $dec_ptr)) (br $fd_done)))
              (if (i32.eq (local.get $imm0) (i32.const 0x4D)) (then (call $template_simd_i8x16_gt_s (local.get $dec_ptr)) (br $fd_done)))
              (if (i32.eq (local.get $imm0) (i32.const 0x4F)) (then (call $template_simd_i8x16_le_s (local.get $dec_ptr)) (br $fd_done)))
              (if (i32.eq (local.get $imm0) (i32.const 0x4A)) (then (call $template_simd_i8x16_lt_u (local.get $dec_ptr)) (br $fd_done)))
              (if (i32.eq (local.get $imm0) (i32.const 0x4C)) (then (call $template_simd_i8x16_gt_u (local.get $dec_ptr)) (br $fd_done)))
              (if (i32.eq (local.get $imm0) (i32.const 0x4E)) (then (call $template_simd_i8x16_le_u (local.get $dec_ptr)) (br $fd_done)))
              (if (i32.eq (local.get $imm0) (i32.const 0xAE)) (then (call $template_simd_i8x16_ge_u (local.get $dec_ptr)) (br $fd_done)))
              ;; ── i8x16 min/max/avgr ──
              (if (i32.eq (local.get $imm0) (i32.const 0xB8)) (then (call $template_simd_i8x16_min_u (local.get $dec_ptr)) (br $fd_done)))
              (if (i32.eq (local.get $imm0) (i32.const 0xBA)) (then (call $template_simd_i8x16_max_u (local.get $dec_ptr)) (br $fd_done)))
              (if (i32.eq (local.get $imm0) (i32.const 0xBB)) (then (call $template_simd_i8x16_avgr_u (local.get $dec_ptr)) (br $fd_done)))
              ;; ── i16x8 arithmetic ──
              (if (i32.eq (local.get $imm0) (i32.const 0x51)) (then (call $template_simd_i16x8_add (local.get $dec_ptr)) (br $fd_done)))
              (if (i32.eq (local.get $imm0) (i32.const 0x52)) (then (call $template_simd_i16x8_sub (local.get $dec_ptr)) (br $fd_done)))
              (if (i32.eq (local.get $imm0) (i32.const 0x57)) (then (call $template_simd_i16x8_neg (local.get $dec_ptr)) (br $fd_done)))
              (if (i32.eq (local.get $imm0) (i32.const 0x61)) (then (call $template_simd_i16x8_mul (local.get $dec_ptr)) (br $fd_done)))
              ;; ── i16x8 comparisons ──
              (if (i32.eq (local.get $imm0) (i32.const 0x58)) (then (call $template_simd_i16x8_eq (local.get $dec_ptr)) (br $fd_done)))
              (if (i32.eq (local.get $imm0) (i32.const 0x59)) (then (call $template_simd_i16x8_ne (local.get $dec_ptr)) (br $fd_done)))
              (if (i32.eq (local.get $imm0) (i32.const 0x5A)) (then (call $template_simd_i16x8_ge_s (local.get $dec_ptr)) (br $fd_done)))
              (if (i32.eq (local.get $imm0) (i32.const 0x5C)) (then (call $template_simd_i16x8_lt_s (local.get $dec_ptr)) (br $fd_done)))
              (if (i32.eq (local.get $imm0) (i32.const 0x5E)) (then (call $template_simd_i16x8_gt_s (local.get $dec_ptr)) (br $fd_done)))
              (if (i32.eq (local.get $imm0) (i32.const 0x60)) (then (call $template_simd_i16x8_le_s (local.get $dec_ptr)) (br $fd_done)))
              (if (i32.eq (local.get $imm0) (i32.const 0x5B)) (then (call $template_simd_i16x8_lt_u (local.get $dec_ptr)) (br $fd_done)))
              (if (i32.eq (local.get $imm0) (i32.const 0x5D)) (then (call $template_simd_i16x8_gt_u (local.get $dec_ptr)) (br $fd_done)))
              (if (i32.eq (local.get $imm0) (i32.const 0x5F)) (then (call $template_simd_i16x8_le_u (local.get $dec_ptr)) (br $fd_done)))
              (if (i32.eq (local.get $imm0) (i32.const 0xAF)) (then (call $template_simd_i16x8_ge_u (local.get $dec_ptr)) (br $fd_done)))
              ;; ── i16x8 min/max/avgr ──
              (if (i32.eq (local.get $imm0) (i32.const 0xBC)) (then (call $template_simd_i16x8_min_u (local.get $dec_ptr)) (br $fd_done)))
              (if (i32.eq (local.get $imm0) (i32.const 0xBE)) (then (call $template_simd_i16x8_max_u (local.get $dec_ptr)) (br $fd_done)))
              (if (i32.eq (local.get $imm0) (i32.const 0xBF)) (then (call $template_simd_i16x8_avgr_u (local.get $dec_ptr)) (br $fd_done)))
              ;; ── i32x4 arithmetic ──
              (if (i32.eq (local.get $imm0) (i32.const 0x62)) (then (call $template_simd_i32x4_add (local.get $dec_ptr)) (br $fd_done)))
              (if (i32.eq (local.get $imm0) (i32.const 0x63)) (then (call $template_simd_i32x4_sub (local.get $dec_ptr)) (br $fd_done)))
              (if (i32.eq (local.get $imm0) (i32.const 0x68)) (then (call $template_simd_i32x4_neg (local.get $dec_ptr)) (br $fd_done)))
              (if (i32.eq (local.get $imm0) (i32.const 0x72)) (then (call $template_simd_i32x4_mul (local.get $dec_ptr)) (br $fd_done)))
              ;; ── i32x4 comparisons ──
              (if (i32.eq (local.get $imm0) (i32.const 0x69)) (then (call $template_simd_i32x4_eq (local.get $dec_ptr)) (br $fd_done)))
              (if (i32.eq (local.get $imm0) (i32.const 0x6A)) (then (call $template_simd_i32x4_ne (local.get $dec_ptr)) (br $fd_done)))
              (if (i32.eq (local.get $imm0) (i32.const 0x6B)) (then (call $template_simd_i32x4_ge_s (local.get $dec_ptr)) (br $fd_done)))
              (if (i32.eq (local.get $imm0) (i32.const 0x6D)) (then (call $template_simd_i32x4_lt_s (local.get $dec_ptr)) (br $fd_done)))
              (if (i32.eq (local.get $imm0) (i32.const 0x6F)) (then (call $template_simd_i32x4_gt_s (local.get $dec_ptr)) (br $fd_done)))
              (if (i32.eq (local.get $imm0) (i32.const 0x71)) (then (call $template_simd_i32x4_le_s (local.get $dec_ptr)) (br $fd_done)))
              (if (i32.eq (local.get $imm0) (i32.const 0x6C)) (then (call $template_simd_i32x4_lt_u (local.get $dec_ptr)) (br $fd_done)))
              (if (i32.eq (local.get $imm0) (i32.const 0x6E)) (then (call $template_simd_i32x4_gt_u (local.get $dec_ptr)) (br $fd_done)))
              (if (i32.eq (local.get $imm0) (i32.const 0x70)) (then (call $template_simd_i32x4_le_u (local.get $dec_ptr)) (br $fd_done)))
              (if (i32.eq (local.get $imm0) (i32.const 0xB0)) (then (call $template_simd_i32x4_ge_u (local.get $dec_ptr)) (br $fd_done)))
              ;; ── i64x2 arithmetic ──
              (if (i32.eq (local.get $imm0) (i32.const 0x73)) (then (call $template_simd_i64x2_add (local.get $dec_ptr)) (br $fd_done)))
              (if (i32.eq (local.get $imm0) (i32.const 0x74)) (then (call $template_simd_i64x2_sub (local.get $dec_ptr)) (br $fd_done)))
              (if (i32.eq (local.get $imm0) (i32.const 0x79)) (then (call $template_simd_i64x2_neg (local.get $dec_ptr)) (br $fd_done)))
              ;; i64x2.mul (not in old encoding) — template not yet added
              ;; ── f32x4 unary ──
              (if (i32.eq (local.get $imm0) (i32.const 0x88)) (then (call $template_simd_f32x4_abs (local.get $dec_ptr)) (br $fd_done)))
              (if (i32.eq (local.get $imm0) (i32.const 0x8A)) (then (call $template_simd_f32x4_neg (local.get $dec_ptr)) (br $fd_done)))
              ;; ── f32x4 binary ──
              (if (i32.eq (local.get $imm0) (i32.const 0x84)) (then (call $template_simd_f32x4_add (local.get $dec_ptr)) (br $fd_done)))
              (if (i32.eq (local.get $imm0) (i32.const 0x85)) (then (call $template_simd_f32x4_sub (local.get $dec_ptr)) (br $fd_done)))
              (if (i32.eq (local.get $imm0) (i32.const 0x87)) (then (call $template_simd_f32x4_mul (local.get $dec_ptr)) (br $fd_done)))
              (if (i32.eq (local.get $imm0) (i32.const 0x89)) (then (call $template_simd_f32x4_div (local.get $dec_ptr)) (br $fd_done)))
              (if (i32.eq (local.get $imm0) (i32.const 0xE1)) (then (call $template_simd_f32x4_min (local.get $dec_ptr)) (br $fd_done)))
              (if (i32.eq (local.get $imm0) (i32.const 0xE2)) (then (call $template_simd_f32x4_max (local.get $dec_ptr)) (br $fd_done)))
              ;; ── f32x4 comparisons ──
              (if (i32.eq (local.get $imm0) (i32.const 0x8C)) (then (call $template_simd_f32x4_eq (local.get $dec_ptr)) (br $fd_done)))
              (if (i32.eq (local.get $imm0) (i32.const 0x8B)) (then (call $template_simd_f32x4_ne (local.get $dec_ptr)) (br $fd_done)))
              (if (i32.eq (local.get $imm0) (i32.const 0x8E)) (then (call $template_simd_f32x4_lt (local.get $dec_ptr)) (br $fd_done)))
              (if (i32.eq (local.get $imm0) (i32.const 0x90)) (then (call $template_simd_f32x4_gt (local.get $dec_ptr)) (br $fd_done)))
              (if (i32.eq (local.get $imm0) (i32.const 0x92)) (then (call $template_simd_f32x4_le (local.get $dec_ptr)) (br $fd_done)))
              (if (i32.eq (local.get $imm0) (i32.const 0x94)) (then (call $template_simd_f32x4_ge (local.get $dec_ptr)) (br $fd_done)))
              ;; ── f64x2 unary ──
              (if (i32.eq (local.get $imm0) (i32.const 0x9A)) (then (call $template_simd_f64x2_abs (local.get $dec_ptr)) (br $fd_done)))
              (if (i32.eq (local.get $imm0) (i32.const 0x9B)) (then (call $template_simd_f64x2_neg (local.get $dec_ptr)) (br $fd_done)))
              ;; ── f64x2 binary ──
              (if (i32.eq (local.get $imm0) (i32.const 0x95)) (then (call $template_simd_f64x2_add (local.get $dec_ptr)) (br $fd_done)))
              (if (i32.eq (local.get $imm0) (i32.const 0x96)) (then (call $template_simd_f64x2_sub (local.get $dec_ptr)) (br $fd_done)))
              (if (i32.eq (local.get $imm0) (i32.const 0x98)) (then (call $template_simd_f64x2_mul (local.get $dec_ptr)) (br $fd_done)))
              (if (i32.eq (local.get $imm0) (i32.const 0x99)) (then (call $template_simd_f64x2_div (local.get $dec_ptr)) (br $fd_done)))
              (if (i32.eq (local.get $imm0) (i32.const 0xE3)) (then (call $template_simd_f64x2_min (local.get $dec_ptr)) (br $fd_done)))
              (if (i32.eq (local.get $imm0) (i32.const 0xE4)) (then (call $template_simd_f64x2_max (local.get $dec_ptr)) (br $fd_done)))
              ;; ── f64x2 comparisons ──
              (if (i32.eq (local.get $imm0) (i32.const 0x9D)) (then (call $template_simd_f64x2_eq (local.get $dec_ptr)) (br $fd_done)))
              (if (i32.eq (local.get $imm0) (i32.const 0x9C)) (then (call $template_simd_f64x2_ne (local.get $dec_ptr)) (br $fd_done)))
              (if (i32.eq (local.get $imm0) (i32.const 0x9F)) (then (call $template_simd_f64x2_lt (local.get $dec_ptr)) (br $fd_done)))
              (if (i32.eq (local.get $imm0) (i32.const 0xA1)) (then (call $template_simd_f64x2_gt (local.get $dec_ptr)) (br $fd_done)))
              (if (i32.eq (local.get $imm0) (i32.const 0xA3)) (then (call $template_simd_f64x2_le (local.get $dec_ptr)) (br $fd_done)))
              (if (i32.eq (local.get $imm0) (i32.const 0xA5)) (then (call $template_simd_f64x2_ge (local.get $dec_ptr)) (br $fd_done)))
              ;; ── v128 bitwise ──
              (if (i32.eq (local.get $imm0) (i32.const 0x43)) (then (call $template_simd_v128_and (local.get $dec_ptr)) (br $fd_done)))
              (if (i32.eq (local.get $imm0) (i32.const 0x44)) (then (call $template_simd_v128_or (local.get $dec_ptr)) (br $fd_done)))
              (if (i32.eq (local.get $imm0) (i32.const 0x45)) (then (call $template_simd_v128_xor (local.get $dec_ptr)) (br $fd_done)))
              (if (i32.eq (local.get $imm0) (i32.const 0xAC)) (then (call $template_simd_v128_not (local.get $dec_ptr)) (br $fd_done)))
              (if (i32.eq (local.get $imm0) (i32.const 0xAD)) (then (call $template_simd_v128_bitselect (local.get $dec_ptr)) (br $fd_done)))
              ;; ── Fall through: unsupported SIMD opcode ──
              (call $template_simd_unsupported (local.get $dec_ptr))
            )
            (local.set $dec_start (i32.add (local.get $dec_start) (i32.const 1)))
            (local.set $dec_ptr (i32.add (local.get $dec_ptr) (global.get $DEC_SZ)))
            (br $compile_loop)
          )
        )

        ;; ── All other ops: dispatch to template (if/else chain) ──

        (block $template_done

          ;; local.get (0x20)
          (if (i32.eq (local.get $op) (i32.const 0x20))
            (then (call $template_x86_local_get (local.get $dec_ptr)) (br $template_done))
          )
          ;; local.set (0x21)
          (if (i32.eq (local.get $op) (i32.const 0x21))
            (then (call $template_x86_local_set (local.get $dec_ptr)) (br $template_done))
          )
          ;; local.tee (0x22)
          (if (i32.eq (local.get $op) (i32.const 0x22))
            (then (call $template_x86_local_tee (local.get $dec_ptr)) (br $template_done))
          )
          ;; global.get (0x23)
          (if (i32.eq (local.get $op) (i32.const 0x23))
            (then (call $template_x86_global_get (local.get $dec_ptr)) (br $template_done))
          )
          ;; global.set (0x24)
          (if (i32.eq (local.get $op) (i32.const 0x24))
            (then (call $template_x86_global_set (local.get $dec_ptr)) (br $template_done))
          )
          ;; table.get (0x25)
          (if (i32.eq (local.get $op) (i32.const 0x25))
            (then (call $template_x86_table_get (local.get $dec_ptr)) (br $template_done))
          )
          ;; table.set (0x26)
          (if (i32.eq (local.get $op) (i32.const 0x26))
            (then (call $template_x86_table_set (local.get $dec_ptr)) (br $template_done))
          )
          ;; drop (0x1A)
          (if (i32.eq (local.get $op) (i32.const 0x1A))
            (then (call $template_x86_drop) (br $template_done))
          )
          ;; select (0x1B)
          (if (i32.eq (local.get $op) (i32.const 0x1B))
            (then (call $template_x86_select) (br $template_done))
          )
          ;; select_typed (0x1C) — same behavior as select
          (if (i32.eq (local.get $op) (i32.const 0x1C))
            (then (call $template_x86_select) (br $template_done))
          )
          ;; i32.const (0x41)
          (if (i32.eq (local.get $op) (i32.const 0x41))
            (then (call $template_x86_i32_const (local.get $dec_ptr)) (br $template_done))
          )
          ;; i64.const (0x42)
          (if (i32.eq (local.get $op) (i32.const 0x42))
            (then (call $template_x86_i64_const (local.get $dec_ptr)) (br $template_done))
          )
          ;; f32.const (0x43)
          (if (i32.eq (local.get $op) (i32.const 0x43))
            (then (call $template_x86_f32_const (local.get $dec_ptr)) (br $template_done))
          )
          ;; f64.const (0x44)
          (if (i32.eq (local.get $op) (i32.const 0x44))
            (then (call $template_x86_f64_const (local.get $dec_ptr)) (br $template_done))
          )
          ;; nop (0x01)
          (if (i32.eq (local.get $op) (i32.const 0x01))
            (then (call $template_x86_nop) (br $template_done))
          )
          ;; unreachable (0x00)
          (if (i32.eq (local.get $op) (i32.const 0x00))
            (then (call $template_x86_unreachable) (br $template_done))
          )

          ;; i32 eqz (0x45)
          (if (i32.eq (local.get $op) (i32.const 0x45)) (then (call $template_x86_i32_eqz) (br $template_done)))
          ;; i32 eq/ne/lt/gt/le/ge (0x46-0x4F)
          (if (i32.eq (local.get $op) (i32.const 0x46)) (then (call $template_x86_i32_eq) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0x47)) (then (call $template_x86_i32_ne) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0x48)) (then (call $template_x86_i32_lt_s) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0x49)) (then (call $template_x86_i32_lt_u) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0x4A)) (then (call $template_x86_i32_gt_s) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0x4B)) (then (call $template_x86_i32_gt_u) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0x4C)) (then (call $template_x86_i32_le_s) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0x4D)) (then (call $template_x86_i32_le_u) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0x4E)) (then (call $template_x86_i32_ge_s) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0x4F)) (then (call $template_x86_i32_ge_u) (br $template_done)))

          ;; i32 unary (0x67-0x69)
          (if (i32.eq (local.get $op) (i32.const 0x67)) (then (call $template_x86_i32_clz) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0x68)) (then (call $template_x86_i32_ctz) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0x69)) (then (call $template_x86_i32_popcnt) (br $template_done)))

          ;; i32 binary (0x6A-0x78)
          (if (i32.eq (local.get $op) (i32.const 0x6A)) (then (call $template_x86_i32_add) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0x6B)) (then (call $template_x86_i32_sub) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0x6C)) (then (call $template_x86_i32_mul) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0x6D)) (then (call $template_x86_i32_div_s) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0x6E)) (then (call $template_x86_i32_div_u) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0x6F)) (then (call $template_x86_i32_rem_s) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0x70)) (then (call $template_x86_i32_rem_u) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0x71)) (then (call $template_x86_i32_and) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0x72)) (then (call $template_x86_i32_or) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0x73)) (then (call $template_x86_i32_xor) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0x74)) (then (call $template_x86_i32_shl) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0x75)) (then (call $template_x86_i32_shr_s) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0x76)) (then (call $template_x86_i32_shr_u) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0x77)) (then (call $template_x86_i32_rotl) (br $template_done)))
           (if (i32.eq (local.get $op) (i32.const 0x78)) (then (call $template_x86_i32_rotr) (br $template_done)))

          ;; i64 unary (0x79-0x7B)
          (if (i32.eq (local.get $op) (i32.const 0x79)) (then (call $template_x86_i64_clz) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0x7A)) (then (call $template_x86_i64_ctz) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0x7B)) (then (call $template_x86_i64_popcnt) (br $template_done)))

          ;; i64 binary (0x7C-0x8A)
          (if (i32.eq (local.get $op) (i32.const 0x7C)) (then (call $template_x86_i64_add) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0x7D)) (then (call $template_x86_i64_sub) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0x7E)) (then (call $template_x86_i64_mul) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0x7F)) (then (call $template_x86_i64_div_s) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0x80)) (then (call $template_x86_i64_div_u) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0x81)) (then (call $template_x86_i64_rem_s) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0x82)) (then (call $template_x86_i64_rem_u) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0x83)) (then (call $template_x86_i64_and) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0x84)) (then (call $template_x86_i64_or) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0x85)) (then (call $template_x86_i64_xor) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0x86)) (then (call $template_x86_i64_shl) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0x87)) (then (call $template_x86_i64_shr_s) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0x88)) (then (call $template_x86_i64_shr_u) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0x89)) (then (call $template_x86_i64_rotl) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0x8A)) (then (call $template_x86_i64_rotr) (br $template_done)))

          ;; Conversions
          (if (i32.eq (local.get $op) (i32.const 0xA7)) (then (call $template_x86_i32_wrap_i64) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0xA8)) (then (call $template_x86_i32_trunc_f32_s) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0xA9)) (then (call $template_x86_i32_trunc_f32_u) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0xAA)) (then (call $template_x86_i32_trunc_f64_s) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0xAB)) (then (call $template_x86_i32_trunc_f64_u) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0xAC)) (then (call $template_x86_i64_extend_i32_s) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0xAD)) (then (call $template_x86_i64_extend_i32_u) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0xAE)) (then (call $template_x86_i64_trunc_f32_s) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0xAF)) (then (call $template_x86_i64_trunc_f32_u) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0xB0)) (then (call $template_x86_i64_trunc_f64_s) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0xB1)) (then (call $template_x86_i64_trunc_f64_u) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0xB2)) (then (call $template_x86_f32_convert_i32_s) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0xB3)) (then (call $template_x86_f32_convert_i32_u) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0xB4)) (then (call $template_x86_f32_convert_i64_s) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0xB5)) (then (call $template_x86_f32_convert_i64_u) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0xB6)) (then (call $template_x86_f32_demote_f64) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0xB7)) (then (call $template_x86_f64_convert_i32_s) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0xB8)) (then (call $template_x86_f64_convert_i32_u) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0xB9)) (then (call $template_x86_f64_convert_i64_s) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0xBA)) (then (call $template_x86_f64_convert_i64_u) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0xBB)) (then (call $template_x86_f64_promote_f32) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0xBC)) (then (call $template_x86_i32_reinterpret_f32) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0xBD)) (then (call $template_x86_i64_reinterpret_f64) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0xBE)) (then (call $template_x86_f32_reinterpret_i32) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0xBF)) (then (call $template_x86_f64_reinterpret_i64) (br $template_done)))

          ;; Sign extension
          (if (i32.eq (local.get $op) (i32.const 0xC0)) (then (call $template_x86_i32_extend8_s) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0xC1)) (then (call $template_x86_i32_extend16_s) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0xC2)) (then (call $template_x86_i64_extend8_s) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0xC3)) (then (call $template_x86_i64_extend16_s) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0xC4)) (then (call $template_x86_i64_extend32_s) (br $template_done)))

          ;; Reference types (0xD0-0xD2)
          (if (i32.eq (local.get $op) (i32.const 0xD0)) (then (call $template_x86_ref_null) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0xD1)) (then (call $template_x86_ref_is_null) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0xD2)) (then (call $template_x86_ref_func (local.get $dec_ptr)) (br $template_done)))

           ;; Memory load/store
          (if (i32.eq (local.get $op) (i32.const 0x28)) (then (call $template_x86_i32_load) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0x29)) (then (call $template_x86_i64_load) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0x2A)) (then (call $template_x86_f32_load) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0x2B)) (then (call $template_x86_f64_load) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0x2C)) (then (call $template_x86_i32_load8_s) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0x2D)) (then (call $template_x86_i32_load8_u) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0x2E)) (then (call $template_x86_i32_load16_s) (br $template_done)))
           (if (i32.eq (local.get $op) (i32.const 0x2F)) (then (call $template_x86_i32_load16_u) (br $template_done)))
           ;; i64 narrow loads (0x30-0x35)
           (if (i32.eq (local.get $op) (i32.const 0x30)) (then (call $template_x86_i64_load8_s) (br $template_done)))
           (if (i32.eq (local.get $op) (i32.const 0x31)) (then (call $template_x86_i64_load8_u) (br $template_done)))
           (if (i32.eq (local.get $op) (i32.const 0x32)) (then (call $template_x86_i64_load16_s) (br $template_done)))
           (if (i32.eq (local.get $op) (i32.const 0x33)) (then (call $template_x86_i64_load16_u) (br $template_done)))
           (if (i32.eq (local.get $op) (i32.const 0x34)) (then (call $template_x86_i64_load32_s) (br $template_done)))
           (if (i32.eq (local.get $op) (i32.const 0x35)) (then (call $template_x86_i64_load32_u) (br $template_done)))
           (if (i32.eq (local.get $op) (i32.const 0x36)) (then (call $template_x86_i32_store) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0x37)) (then (call $template_x86_i64_store) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0x38)) (then (call $template_x86_f32_store) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0x39)) (then (call $template_x86_f64_store) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0x3A)) (then (call $template_x86_i32_store8) (br $template_done)))
           (if (i32.eq (local.get $op) (i32.const 0x3B)) (then (call $template_x86_i32_store16) (br $template_done)))
           ;; i64 narrow stores (0x3C-0x3E)
           (if (i32.eq (local.get $op) (i32.const 0x3C)) (then (call $template_x86_i64_store8) (br $template_done)))
           (if (i32.eq (local.get $op) (i32.const 0x3D)) (then (call $template_x86_i64_store16) (br $template_done)))
           (if (i32.eq (local.get $op) (i32.const 0x3E)) (then (call $template_x86_i64_store32) (br $template_done)))

            ;; memory.size / memory.grow
          (if (i32.eq (local.get $op) (i32.const 0x3F)) (then (call $template_x86_memory_size) (br $template_done)))
           (if (i32.eq (local.get $op) (i32.const 0x40)) (then (call $template_x86_memory_grow) (br $template_done)))

          ;; i64 comparisons (0x50-0x5A)
          (if (i32.eq (local.get $op) (i32.const 0x50)) (then (call $template_x86_i64_eqz) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0x51)) (then (call $template_x86_i64_eq) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0x52)) (then (call $template_x86_i64_ne) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0x53)) (then (call $template_x86_i64_lt_s) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0x54)) (then (call $template_x86_i64_lt_u) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0x55)) (then (call $template_x86_i64_gt_s) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0x56)) (then (call $template_x86_i64_gt_u) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0x57)) (then (call $template_x86_i64_le_s) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0x58)) (then (call $template_x86_i64_le_u) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0x59)) (then (call $template_x86_i64_ge_s) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0x5A)) (then (call $template_x86_i64_ge_u) (br $template_done)))

          ;; f32 comparisons (0x5B-0x60)
          (if (i32.eq (local.get $op) (i32.const 0x5B)) (then (call $template_x86_f32_eq) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0x5C)) (then (call $template_x86_f32_ne) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0x5D)) (then (call $template_x86_f32_lt) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0x5E)) (then (call $template_x86_f32_gt) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0x5F)) (then (call $template_x86_f32_le) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0x60)) (then (call $template_x86_f32_ge) (br $template_done)))

          ;; f64 comparisons (0x61-0x66)
          (if (i32.eq (local.get $op) (i32.const 0x61)) (then (call $template_x86_f64_eq) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0x62)) (then (call $template_x86_f64_ne) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0x63)) (then (call $template_x86_f64_lt) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0x64)) (then (call $template_x86_f64_gt) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0x65)) (then (call $template_x86_f64_le) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0x66)) (then (call $template_x86_f64_ge) (br $template_done)))

          ;; f32 binary (0x92-0x98)
          (if (i32.eq (local.get $op) (i32.const 0x92)) (then (call $template_x86_f32_add) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0x93)) (then (call $template_x86_f32_sub) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0x94)) (then (call $template_x86_f32_mul) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0x95)) (then (call $template_x86_f32_div) (br $template_done)))
          ;; f32 min/max/copysign (0x96-0x98)
          (if (i32.eq (local.get $op) (i32.const 0x96)) (then (call $template_x86_f32_min) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0x97)) (then (call $template_x86_f32_max) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0x98)) (then (call $template_x86_f32_copysign) (br $template_done)))

          ;; f64 binary (0xA0-0xA3)
          (if (i32.eq (local.get $op) (i32.const 0xA0)) (then (call $template_x86_f64_add) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0xA1)) (then (call $template_x86_f64_sub) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0xA2)) (then (call $template_x86_f64_mul) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0xA3)) (then (call $template_x86_f64_div) (br $template_done)))
          ;; f64 min/max/copysign (0xA4-0xA6)
          (if (i32.eq (local.get $op) (i32.const 0xA4)) (then (call $template_x86_f64_min) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0xA5)) (then (call $template_x86_f64_max) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0xA6)) (then (call $template_x86_f64_copysign) (br $template_done)))

          ;; f32 unary
          (if (i32.eq (local.get $op) (i32.const 0x8B)) (then (call $template_x86_f32_abs) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0x8C)) (then (call $template_x86_f32_neg) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0x91)) (then (call $template_x86_f32_sqrt) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0x8D)) (then (call $template_x86_f32_ceil) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0x8E)) (then (call $template_x86_f32_floor) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0x8F)) (then (call $template_x86_f32_trunc) (br $template_done)))
           (if (i32.eq (local.get $op) (i32.const 0x90)) (then (call $template_x86_f32_nearest) (br $template_done)))

          ;; f64 unary (0x99-0x9F)
          (if (i32.eq (local.get $op) (i32.const 0x99)) (then (call $template_x86_f64_abs) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0x9A)) (then (call $template_x86_f64_neg) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0x9B)) (then (call $template_x86_f64_sqrt) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0x9C)) (then (call $template_x86_f64_ceil) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0x9D)) (then (call $template_x86_f64_floor) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0x9E)) (then (call $template_x86_f64_trunc) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0x9F)) (then (call $template_x86_f64_nearest) (br $template_done)))

          ;; call (0x10) — dispatch to template_call for import→syscall
          (if (i32.eq (local.get $op) (i32.const 0x10))
            (then (call $template_x86_call (local.get $dec_ptr)) (br $template_done))
          )
          ;; call_indirect (0x11), br_table (0x0E) — not JIT-compilable
          (if (i32.eq (local.get $op) (i32.const 0x11))
            (then (call $template_x86_unsupported) (br $template_done))
          )
          (if (i32.eq (local.get $op) (i32.const 0x0E))
            (then (call $template_x86_unsupported) (br $template_done))
          )

          ;; Unsupported opcode — emit ud2
          (call $template_x86_unsupported)
        )

        ;; Advance to next decoded op
        (local.set $dec_start (i32.add (local.get $dec_start) (i32.const 1)))
        (local.set $dec_ptr (i32.add (local.get $dec_ptr) (global.get $DEC_SZ)))
        (br $compile_loop)
      )
    )

    ;; ── Emit function epilogue ──
    (if (i32.eqz (i32.load (global.get $JS_RETURN_EMITTED)))
      (then
        (if (i32.eq (local.get $result_count) (i32.const 1))
          (then
            (call $emit_x86_pop_rax)
          )
          (else
            (call $emit_x86_xor_eax_eax)
          )
        )
        (call $emit_x86_xor_edx_edx)
        (call $emit_x86_epilogue)
      )
    )

    ;; Return code_ptr (offset from cache_base)
    (return (i32.sub (i32.load (global.get $JS_CODE_PTR)) (i32.mul (local.get $slot) (global.get $JIT_SLOT_SIZE))))
  )

  ;; ── Import → syscall dispatch ─────────────────────────────────────────
  ;; Import count: written by interpreter at address 16648 in linear memory
  (global $OFF_IMPORT_COUNT i32 (i32.const 16648))
  ;; Syscall map table at 0x90000: array of i32 sysno per import index
  ;; Default: sysno[i] = i (import 0 → SYS_read, import 1 → SYS_write, …)
  (global $OFF_SYSCALL_MAP i32 (i32.const 0x90000))

  ;; ═════════════════════════════════════════════════════════════════════
  ;; ELF64 binary output
  ;; ═════════════════════════════════════════════════════════════════════

  (global $ELF_OUT_BUF  i32 (i32.const 0x800000))
  (global $ELF_OUT_OFF  i32 (i32.const 0x700000))  ;; ELF_OUT_BUF - JIT_CACHE

  (global $TEXT_VA      i32 (i32.const 0x400000))
  (global $BSS_VA       i32 (i32.const 0x500000))

  (global $EHDR_SIZE    i32 (i32.const 64))
  (global $PHDR_SIZE    i32 (i32.const 56))
  (global $ELF_STUB_OFF i32 (i32.const 120))       ;; after 1 phdr (64 + 56)
  (global $ELF_CODE_OFF i32 (i32.const 256))        ;; aligned after stub
  (global $BSS_SIZE     i32 (i32.const 0x40000))    ;; 256KB

  ;; BSS item VAs (relative to BSS_VA)
  (global $BSS_JITGLOBALS i32 (i32.const 0x500000))
  (global $BSS_MEM       i32 (i32.const 0x500080))
  (global $BSS_LOCALS    i32 (i32.const 0x510080))
  (global $BSS_GLOBALS   i32 (i32.const 0x520080))
  (global $BSS_TABLE     i32 (i32.const 0x530080))

  ;; ── ELF emit helpers ───────────────────────────────────────────────

  ;; MOV r64, imm32 sign-extended: REX.W + C7 /0 id
  (func $emit_mov_r64_imm32 (param $rex i32) (param $modrm i32) (param $val i32)
    (call $emit_x86_byte (local.get $rex))
    (call $emit_x86_byte (i32.const 0xC7))
    (call $emit_x86_byte (local.get $modrm))
    (call $emit_x86_dword (local.get $val))
  )

  ;; MOV [r15+off8], rax: 49 89 47 <off>
  (func $emit_mov_r15off_rax (param $off i32)
    (call $emit_x86_byte (i32.const 0x49))
    (call $emit_x86_byte (i32.const 0x89))
    (call $emit_x86_byte (i32.const 0x47))
    (call $emit_x86_byte (local.get $off))
  )

  ;; MOV dword [r15+off8], imm32: 41 C7 47 <off> <imm32>
  (func $emit_mov_dword_r15off (param $off i32) (param $val i32)
    (call $emit_x86_byte (i32.const 0x41))
    (call $emit_x86_byte (i32.const 0xC7))
    (call $emit_x86_byte (i32.const 0x47))
    (call $emit_x86_byte (local.get $off))
    (call $emit_x86_dword (local.get $val))
  )

  ;; CALL rel32: E8 <disp32>  (target = current_va + 5 + disp)
  (func $emit_call_rel (param $target_va i32)
    (local $saved i32)
    (local.set $saved (i32.load (global.get $JS_CODE_PTR)))
    (call $emit_x86_byte (i32.const 0xE8))
    (call $emit_x86_dword
      (i32.sub
        (local.get $target_va)
        (i32.add
          (i32.add (global.get $TEXT_VA) (i32.sub (local.get $saved) (global.get $ELF_OUT_OFF)))
          (i32.const 5)
        )
      )
    )
  )

  ;; MOV edi, eax: 89 C7
  (func $emit_mov_edi_eax
    (call $emit_x86_byte (i32.const 0x89))
    (call $emit_x86_byte (i32.const 0xC7))
  )

  ;; MOV eax, imm32: B8 <dword>
  (func $emit_mov_eax_imm (param $val i32)
    (call $emit_x86_byte (i32.const 0xB8))
    (call $emit_x86_dword (local.get $val))
  )

  ;; SYSCALL: 0F 05
  (func $emit_syscall
    (call $emit_x86_byte (i32.const 0x0F))
    (call $emit_x86_byte (i32.const 0x05))
  )

  ;; ── Syscall trampoline helpers ───────────────────────────────────────

  ;; Allocate 48 bytes (6 qwords) on stack for syscall struct
  ;; sub rsp, 48: 48 83 EC 30
  (func $emit_sub_rsp_48
    (call $emit_x86_rex_w)
    (call $emit_x86_byte (i32.const 0x83))
    (call $emit_x86_byte (i32.const 0xEC))
    (call $emit_x86_byte (i32.const 0x30))
  )

  ;; Deallocate 48 bytes: add rsp, 48
  (func $emit_add_rsp_48
    (call $emit_x86_rex_w)
    (call $emit_x86_byte (i32.const 0x83))
    (call $emit_x86_byte (i32.const 0xC4))
    (call $emit_x86_byte (i32.const 0x30))
  )

  ;; mov rdi, [rsp + disp32]: 48 8B BC 24 <dword>
  (func $emit_load_rdi_rsp_disp (param $disp i32)
    (call $emit_x86_rex_w)
    (call $emit_x86_byte (i32.const 0x8B))
    (call $emit_x86_byte (i32.const 0xBC))
    (call $emit_x86_sib (i32.const 0) (i32.const 4) (i32.const 4))
    (call $emit_x86_dword (local.get $disp))
  )

  ;; mov rsi, [rsp + disp32]: 48 8B B4 24 <dword>
  (func $emit_load_rsi_rsp_disp (param $disp i32)
    (call $emit_x86_rex_w)
    (call $emit_x86_byte (i32.const 0x8B))
    (call $emit_x86_byte (i32.const 0xB4))
    (call $emit_x86_sib (i32.const 0) (i32.const 4) (i32.const 4))
    (call $emit_x86_dword (local.get $disp))
  )

  ;; mov rdx, [rsp + disp32]: 48 8B 94 24 <dword>
  (func $emit_load_rdx_rsp_disp (param $disp i32)
    (call $emit_x86_rex_w)
    (call $emit_x86_byte (i32.const 0x8B))
    (call $emit_x86_byte (i32.const 0x94))
    (call $emit_x86_sib (i32.const 0) (i32.const 4) (i32.const 4))
    (call $emit_x86_dword (local.get $disp))
  )

  ;; mov r10, [rsp + disp32]: 4C 8B 54 24 <dword> (no SIB needed)
  ;; mov r10, [rsp + disp32]: 4C 8B 94 24 <dword>
  ;; r10 = reg 10 (0xA): low 3 bits=010(2), REX.R=1 → REX=0x4C
  (func $emit_load_r10_rsp_disp (param $disp i32)
    (call $emit_x86_byte (i32.const 0x4C))  ;; REX.W R=1
    (call $emit_x86_byte (i32.const 0x8B))  ;; mov r64, r/m64
    (call $emit_x86_byte (i32.const 0x94))  ;; ModRM: mod=10, reg=010(r10), rm=100(SIB)
    (call $emit_x86_sib (i32.const 0) (i32.const 4) (i32.const 4))  ;; [rsp]
    (call $emit_x86_dword (local.get $disp))
  )

  ;; mov r8, [rsp + disp32]: 4C 8B 84 24 <dword>  (mod=10 for disp32)
  (func $emit_load_r8_rsp_disp (param $disp i32)
    (call $emit_x86_byte (i32.const 0x4C))  ;; REX.W R=1
    (call $emit_x86_byte (i32.const 0x8B))
    (call $emit_x86_byte (i32.const 0x84))  ;; ModRM: mod=10, reg=000(r8), rm=100(SIB)
    (call $emit_x86_sib (i32.const 0) (i32.const 4) (i32.const 4))  ;; [rsp]
    (call $emit_x86_dword (local.get $disp))
  )

  ;; mov r9, [rsp + disp32]: 4C 8B 8C 24 <dword>
  (func $emit_load_r9_rsp_disp (param $disp i32)
    (call $emit_x86_byte (i32.const 0x4C))  ;; REX.W R=1
    (call $emit_x86_byte (i32.const 0x8B))
    (call $emit_x86_byte (i32.const 0x8C))  ;; ModRM: mod=10, reg=001(r9), rm=100(SIB)
    (call $emit_x86_sib (i32.const 0) (i32.const 4) (i32.const 4))  ;; [rsp]
    (call $emit_x86_dword (local.get $disp))
  )

  ;; xor eax, eax (2 bytes): 31 C0
  (func $emit_x86_xor_eax_eax_32
    (call $emit_x86_byte (i32.const 0x31))
    (call $emit_x86_byte (i32.const 0xC0))
  )

  ;; ── ELF64 header ─────────────────────────────────────────────────


  ;; ── ELF64 program header (56 bytes) ──────────────────────────────


  ;; ── Runtime stub: _start that calls compiled code and exits ──────
  ;; $bss_va = page-aligned start of BSS (right after compiled code)


  ;; ── Copy compiled code from cache into ELF output ────────────────

        (i32.store8
          (i32.add (local.get $dst) (local.get $i))
          (i32.load8_u (i32.add (local.get $src) (local.get $i)))
        )
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $copy)
      )
    )
  )

  ;; ── compile_to_elf: compile a WASM function and emit ELF64 ───────
  ;; Returns (start_address, total_size)
  (func (export "compile_to_elf") (param $func_idx i32) (result i32 i32)
    (local $code_size i32) (local $total_size i32)
    (local $saved i32) (local $bss_va i32)

    ;; 1. Run the compiler (produces code at 0x100000)
    (local.set $code_size (call $jit_compile (local.get $func_idx)))

    ;; 2. Compute total ELF file size
    (local.set $total_size (i32.add (global.get $ELF_CODE_OFF) (local.get $code_size)))

    ;; 3. Compute BSS base: page-align TEXT_VA + total_size
    (local.set $bss_va
      (i32.add
        (i32.and (i32.add (local.get $total_size) (i32.const 0xFFF)) (i32.const -0x1000))
        (global.get $TEXT_VA)
      )
    )

    ;; 4. Save JS_CODE_PTR, redirect emit to ELF output buffer
    (local.set $saved (i32.load (global.get $JS_CODE_PTR)))
    (i32.store (global.get $JS_CODE_PTR) (global.get $ELF_OUT_OFF))

    ;; 5. Emit ELF64 header (1 program header: text + BSS combined)
    (call $emit_elf64_ehdr
      (i32.add (global.get $TEXT_VA) (global.get $ELF_STUB_OFF))  ;; entry = _start
      (i32.const 64)   ;; e_phoff
      (i32.const 1)    ;; 1 program header (text + bss)
    )

    ;; 6. Emit PH#1: text + BSS combined LOAD (R|W|X), offset=0
    ;;     filesz = total_size, memsz includes BSS
    (call $emit_elf64_phdr
      (i32.const 1)         ;; PT_LOAD
      (i32.const 7)         ;; PF_R | PF_W | PF_X
      (i32.const 0)         ;; p_offset = 0
      (global.get $TEXT_VA) ;; p_vaddr
      (local.get $total_size) ;; p_filesz
      (i32.add (i32.and (i32.add (local.get $total_size) (i32.const 0xFFF)) (i32.const -0x1000)) (global.get $BSS_SIZE)) ;; p_memsz includes BSS
    )

    ;; 7. Emit runtime stub (with BSS base address)
    (call $emit_elf_stub (local.get $bss_va))

    ;; 8. Pad with zeros from end of stub to ELF_CODE_OFF
    (block $pad_done
      (loop $pad_loop
        (if (i32.ge_u (i32.load (global.get $JS_CODE_PTR))
                       (i32.add (global.get $ELF_OUT_OFF) (global.get $ELF_CODE_OFF)))
          (then (br $pad_done))
        )
        (call $emit_x86_byte (i32.const 0))
        (br $pad_loop)
      )
    )

    ;; 9. Copy compiled code from cache (0x100000) to ELF output
    (call $copy_compiled_code (global.get $JIT_CACHE) (local.get $code_size))

    ;; 10. Restore JS_CODE_PTR
    (i32.store (global.get $JS_CODE_PTR) (local.get $saved))

    ;; Return (buffer_address, total_size)
    (return (global.get $ELF_OUT_BUF) (local.get $total_size))
  )

  ;; ═════════════════════════════════════════════════════════════════════
  ;; Flat binary output (bare-metal x86-64)
  ;; ═════════════════════════════════════════════════════════════════════

  (global $BIN_OUT_BUF  i32 (i32.const 0x900000))
  (global $BIN_OUT_OFF  i32 (i32.const 0x800000))  ;; BIN_OUT_BUF - JIT_CACHE

  ;; MOV moffs32, eAX: 67 A3 <addr>
  (func $emit_mov_abs32_eax (param $addr i32)
    (call $emit_x86_byte (i32.const 0x67))
    (call $emit_x86_byte (i32.const 0xA3))
    (call $emit_x86_dword (local.get $addr))
  )

  ;; CALL rel32 with displacement computed relative to the binary base
  ;; target_off = offset from start of binary to the target instruction
  (func $emit_call_flat (param $target_off i32)
    (local $saved i32)
    (local.set $saved (i32.load (global.get $JS_CODE_PTR)))
    (call $emit_x86_byte (i32.const 0xE8))
    (call $emit_x86_dword
      (i32.sub
        (local.get $target_off)
        (i32.add (i32.sub (local.get $saved) (global.get $BIN_OUT_OFF)) (i32.const 5))
      )
    )
  )

  ;; Emit bare-metal runtime stub (no ELF headers, starts at offset 0)
  ;; The compiled code will be placed right after the stub.

  ;; Copy compiled code from cache to output buffer at given offset
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $copy)
      )
    )
  )

  ;; ═════════════════════════════════════════════════════════════════════
  ;; compile_to_bin: emit flat binary with bare-metal runtime stub
  ;; Returns (buffer_address, total_size)
  ;; ═════════════════════════════════════════════════════════════════════
  (func (export "compile_to_bin") (param $func_idx i32) (result i32 i32)
    (local $code_size i32) (local $stub_size i32) (local $total_size i32)
    (local $saved i32)

    ;; 1. Run the compiler
    (local.set $code_size (call $jit_compile (local.get $func_idx)))

    ;; 2. Redirect emit to binary output buffer
    (local.set $saved (i32.load (global.get $JS_CODE_PTR)))
    (i32.store (global.get $JS_CODE_PTR) (global.get $BIN_OUT_OFF))

    ;; 3. Emit bare-metal stub — returns stub size (where compiled code goes)
    (local.set $stub_size (call $emit_bare_metal_stub))

    ;; 4. Copy compiled code after stub
    (call $copy_code_to (global.get $BIN_OUT_BUF) (local.get $code_size) (local.get $stub_size))

    ;; 5. Total size = stub_size + code_size
    (local.set $total_size (i32.add (local.get $stub_size) (local.get $code_size)))

    ;; 6. Restore JS_CODE_PTR
    (i32.store (global.get $JS_CODE_PTR) (local.get $saved))

    (return (global.get $BIN_OUT_BUF) (local.get $total_size))
  )

)
