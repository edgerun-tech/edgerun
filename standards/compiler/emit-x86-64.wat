;; ── Helper: write bytes to code cache ──────────────────────────────

  ;; Emit a single byte into the code cache, advance code_ptr
  (func $emit_x86_byte (param $b i32)
    (local $p i32)
    (local.set $p (i32.add (global.get $JIT_CACHE) (i32.load (global.get $JS_CODE_PTR))))
    (i32.store8 (local.get $p) (local.get $b))
    (i32.store (global.get $JS_CODE_PTR) (i32.add (i32.load (global.get $JS_CODE_PTR)) (i32.const 1)))
  )

  ;; Emit a dword (4 bytes) little-endian
  (func $emit_x86_dword (param $v i32)
    (call $emit_x86_byte (i32.and (local.get $v) (i32.const 0xFF)))
    (call $emit_x86_byte (i32.and (i32.shr_u (local.get $v) (i32.const 8)) (i32.const 0xFF)))
    (call $emit_x86_byte (i32.and (i32.shr_u (local.get $v) (i32.const 16)) (i32.const 0xFF)))
    (call $emit_x86_byte (i32.and (i32.shr_u (local.get $v) (i32.const 24)) (i32.const 0xFF)))
  )

  ;; Emit a qword (8 bytes) little-endian
  (func $emit_x86_qword (param $v i64)
    (call $emit_x86_dword (i32.wrap_i64 (local.get $v)))
    (call $emit_x86_dword (i32.wrap_i64 (i64.shr_u (local.get $v) (i64.const 32))))
  )

  ;; ── x86_64 ModRM / SIB emission ───────────────────────────────────

  ;; Emit ModRM byte: mod=2 bits, reg=3 bits, rm=3 bits
  (func $emit_x86_modrm (param $mod i32) (param $reg i32) (param $rm i32)
    (call $emit_x86_byte
      (i32.or (i32.shl (local.get $mod) (i32.const 6))
              (i32.or (i32.shl (local.get $reg) (i32.const 3))
                      (local.get $rm)))
    )
  )

  ;; Emit SIB byte: scale=2 bits, index=3 bits, base=3 bits
  (func $emit_x86_sib (param $scale i32) (param $index i32) (param $base i32)
    (call $emit_x86_byte
      (i32.or (i32.shl (local.get $scale) (i32.const 6))
              (i32.or (i32.shl (local.get $index) (i32.const 3))
                      (local.get $base)))
    )
  )

  ;; Emit REX prefix byte
  ;; clobbers: W=64bit, R=extended reg, X=extended index, B=extended base
  (func $emit_x86_rex (param $w i32) (param $r i32) (param $x i32) (param $b i32)
    (call $emit_x86_byte
      (i32.or (i32.const 0x40)
              (i32.or (i32.shl (local.get $w) (i32.const 3))
                      (i32.or (i32.shl (local.get $r) (i32.const 2))
                              (i32.or (i32.shl (local.get $x) (i32.const 1))
                                      (local.get $b)))))
    )
  )

  ;; Emit REX.W (64-bit operand size)
  (func $emit_x86_rex_w
    (call $emit_x86_byte (i32.const 0x48))
  )

  ;; ── x86_64 instruction helpers ─────────────────────────────────────

  ;; mov rdx, imm64 (10 bytes: REX.W + B8+rdx + qword)
  (func $emit_x86_mov_rdx_imm64
    (call $emit_x86_rex_w)
    (call $emit_x86_byte (i32.const 0xBA))
    (call $emit_x86_qword (i64.const 0))  ;; placeholder, caller patches
  )

  ;; mov rax, imm64 (10 bytes: REX.W + B8+rax + qword)
  (func $emit_x86_mov_rax_imm64
    (call $emit_x86_rex_w)
    (call $emit_x86_byte (i32.const 0xB8))
    (call $emit_x86_qword (i64.const 0))  ;; placeholder
  )

  ;; mov rdi, imm64 (10 bytes)
  (func $emit_x86_mov_rdi_imm64
    (call $emit_x86_rex_w)
    (call $emit_x86_byte (i32.const 0xBF))
    (call $emit_x86_qword (i64.const 0))  ;; placeholder
  )

  ;; mov rsi, rsp (3 bytes: 48 89 E6)
  (func $emit_x86_mov_rsi_rsp
    (call $emit_x86_rex_w)
    (call $emit_x86_byte (i32.const 0x89))
    (call $emit_x86_modrm (i32.const 3) (i32.const 6) (i32.const 4))
  )

  ;; mov edx, imm32 (5 bytes: BA + dword)
  (func $emit_x86_mov_edx_imm32 (param $v i32)
    (call $emit_x86_byte (i32.const 0xBA))
    (call $emit_x86_dword (local.get $v))
  )

  ;; mov eax, imm32 (5 bytes: B8 + dword) — implicitly zeros upper 32
  (func $emit_x86_mov_eax_imm32 (param $v i32)
    (call $emit_x86_byte (i32.const 0xB8))
    (call $emit_x86_dword (local.get $v))
  )

  ;; mov ecx, imm32 (5 bytes: B9 + dword)
  (func $emit_x86_mov_ecx_imm32 (param $v i32)
    (call $emit_x86_byte (i32.const 0xB9))
    (call $emit_x86_dword (local.get $v))
  )

  ;; push rax (1 byte: 50)
  (func $emit_x86_push_rax
    (call $emit_x86_byte (i32.const 0x50))
  )

  ;; push rcx (1 byte: 51)
  (func $emit_x86_push_rcx
    (call $emit_x86_byte (i32.const 0x51))
  )

  ;; push rdx (1 byte: 52)
  (func $emit_x86_push_rdx
    (call $emit_x86_byte (i32.const 0x52))
  )

  ;; push rbx (1 byte: 53)
  (func $emit_x86_push_rbx
    (call $emit_x86_byte (i32.const 0x53))
  )

  ;; push r15 (1 byte: 57, where reg=7)
  (func $emit_x86_push_r15
    (call $emit_x86_byte (i32.const 0x57))
  )

  ;; pop rax (1 byte: 58)
  (func $emit_x86_pop_rax
    (call $emit_x86_byte (i32.const 0x58))
  )

  ;; pop rcx (1 byte: 59)
  (func $emit_x86_pop_rcx
    (call $emit_x86_byte (i32.const 0x59))
  )

  ;; pop rdx (1 byte: 5A)
  (func $emit_x86_pop_rdx
    (call $emit_x86_byte (i32.const 0x5A))
  )

  ;; pop r15 (1 byte: 5F, where reg=7)
  (func $emit_x86_pop_r15
    (call $emit_x86_byte (i32.const 0x5F))
  )

  ;; push imm32 (68 + dword)
  (func $emit_x86_push_imm32 (param $v i32)
    (call $emit_x86_byte (i32.const 0x68))
    (call $emit_x86_dword (local.get $v))
  )

  ;; pop reg: 58+reg (0=rax, 1=rcx, 2=rdx)
  (func $emit_x86_pop_reg (param $reg i32)
    (call $emit_x86_byte (i32.add (i32.const 0x58) (local.get $reg)))
  )

  ;; push reg: 50+reg (0=rax, 1=rcx, 2=rdx)
  (func $emit_x86_push_reg (param $reg i32)
    (call $emit_x86_byte (i32.add (i32.const 0x50) (local.get $reg)))
  )

  ;; ret (1 byte: C3)
  (func $emit_x86_ret
    (call $emit_x86_byte (i32.const 0xC3))
  )

  ;; ud2 (2 bytes: 0F 0B)
  (func $emit_x86_ud2
    (call $emit_x86_byte (i32.const 0x0F))
    (call $emit_x86_byte (i32.const 0x0B))
  )

  ;; int3 (1 byte: CC)
  (func $emit_x86_int3
    (call $emit_x86_byte (i32.const 0xCC))
  )

  ;; Push rax only if RESULT_IN_EAX is 0 (peephole: skip push when
  ;; next op will consume from eax directly).
  ;; Peephole lookahead: if the next decoded op is local.set/tee with
  ;; register-allocated local (idx 0 or 1), leave result in eax and
  ;; set RESULT_IN_EAX flag instead of pushing.
  (func $emit_x86_maybe_push_rax
    (local $next_op i32)
    (local $next_imm0 i32)
    (if (i32.eqz (global.get $RESULT_IN_EAX))
      (then
        (local.set $next_op (i32.load8_u (i32.add (global.get $CURRENT_DEC_PTR) (global.get $DEC_SZ))))
        (local.set $next_imm0 (i32.load (i32.add (i32.add (global.get $CURRENT_DEC_PTR) (global.get $DEC_SZ)) (i32.const 4))))
        (if (i32.and (i32.or (i32.eq (local.get $next_op) (i32.const 0x21)) (i32.eq (local.get $next_op) (i32.const 0x22)))
                      (i32.le_u (local.get $next_imm0) (i32.const 1)))
          (then
            (global.set $RESULT_IN_EAX (i32.const 1))
          )
          (else
            (call $emit_x86_byte (i32.const 0x50))
          )
        )
      )
    )
  )

  ;; xor eax, eax (2 bytes: 31 C0)
  (func $emit_x86_xor_eax_eax
    (call $emit_x86_byte (i32.const 0x31))
    (call $emit_x86_modrm (i32.const 3) (i32.const 0) (i32.const 0))
  )

  ;; xor edx, edx (2 bytes: 31 D2)
  (func $emit_x86_xor_edx_edx
    (call $emit_x86_byte (i32.const 0x31))
    (call $emit_x86_modrm (i32.const 3) (i32.const 2) (i32.const 2))
  )

  ;; xor rdx, rdx (REX.W + 31 D2 = 48 31 D2)
  (func $emit_x86_xor_rdx_rdx
    (call $emit_x86_rex_w)
    (call $emit_x86_byte (i32.const 0x31))
    (call $emit_x86_modrm (i32.const 3) (i32.const 2) (i32.const 2))
  )

  ;; xor ecx, ecx (2 bytes: 31 C9)
  (func $emit_x86_xor_ecx_ecx
    (call $emit_x86_byte (i32.const 0x31))
    (call $emit_x86_modrm (i32.const 3) (i32.const 1) (i32.const 1))
  )

  ;; cdq (1 byte: 99) — sign-extend eax into edx:eax
  (func $emit_x86_cdq
    (call $emit_x86_byte (i32.const 0x99))
  )

  ;; cqo (REX.W + 99) — sign-extend rax into rdx:rax
  (func $emit_x86_cqo
    (call $emit_x86_rex_w)
    (call $emit_x86_byte (i32.const 0x99))
  )

  ;; test eax, eax (2 bytes: 85 C0)
  (func $emit_x86_test_eax
    (call $emit_x86_byte (i32.const 0x85))
    (call $emit_x86_modrm (i32.const 3) (i32.const 0) (i32.const 0))
  )

  ;; test edx, edx (2 bytes: 85 D2)
  (func $emit_x86_test_edx
    (call $emit_x86_byte (i32.const 0x85))
    (call $emit_x86_modrm (i32.const 3) (i32.const 2) (i32.const 2))
  )

  ;; test ecx, ecx (2 bytes: 85 C9)
  (func $emit_x86_test_ecx
    (call $emit_x86_byte (i32.const 0x85))
    (call $emit_x86_modrm (i32.const 3) (i32.const 1) (i32.const 1))
  )

  ;; mov edx, eax (2 bytes: 89 C2)
  (func $emit_x86_mov_edx_eax
    (call $emit_x86_byte (i32.const 0x89))
    (call $emit_x86_modrm (i32.const 3) (i32.const 2) (i32.const 0))
  )

  ;; mov rdx, rax (3 bytes: 48 89 C2)
  (func $emit_x86_mov_rdx_rax
    (call $emit_x86_rex_w)
    (call $emit_x86_byte (i32.const 0x89))
    (call $emit_x86_modrm (i32.const 3) (i32.const 2) (i32.const 0))
  )

  ;; ── i32 binary arithmetic (pop rcx, pop rax, op, push rax) ──────────

  ;; pop rcx, pop rax
  (func $emit_x86_pop2_rcx_rax
    (call $emit_x86_pop_reg (i32.const 1))  ;; pop rcx
    (call $emit_x86_pop_reg (i32.const 0))  ;; pop rax
  )

  ;; add eax, ecx (2 bytes: 01 C8)
  (func $emit_x86_add32
    (call $emit_x86_byte (i32.const 0x01))
    (call $emit_x86_modrm (i32.const 3) (i32.const 1) (i32.const 0))
  )

  ;; sub eax, ecx (2 bytes: 29 C8)
  (func $emit_x86_sub32
    (call $emit_x86_byte (i32.const 0x29))
    (call $emit_x86_modrm (i32.const 3) (i32.const 1) (i32.const 0))
  )

  ;; imul eax, ecx (3 bytes: 0F AF C1)
  (func $emit_x86_imul32
    (call $emit_x86_byte (i32.const 0x0F))
    (call $emit_x86_byte (i32.const 0xAF))
    (call $emit_x86_modrm (i32.const 3) (i32.const 0) (i32.const 1))
  )

  ;; idiv ecx (2 bytes: F7 F9) — divides edx:eax by ecx
  (func $emit_x86_idiv32
    (call $emit_x86_byte (i32.const 0xF7))
    (call $emit_x86_modrm (i32.const 3) (i32.const 7) (i32.const 1))
  )

  ;; div ecx (2 bytes: F7 F1) — divides edx:eax by ecx
  (func $emit_x86_div32
    (call $emit_x86_byte (i32.const 0xF7))
    (call $emit_x86_modrm (i32.const 3) (i32.const 6) (i32.const 1))
  )

  ;; and eax, ecx (2 bytes: 21 C8)
  (func $emit_x86_and32
    (call $emit_x86_byte (i32.const 0x21))
    (call $emit_x86_modrm (i32.const 3) (i32.const 1) (i32.const 0))
  )

  ;; or eax, ecx (2 bytes: 09 C8)
  (func $emit_x86_or32
    (call $emit_x86_byte (i32.const 0x09))
    (call $emit_x86_modrm (i32.const 3) (i32.const 1) (i32.const 0))
  )

  ;; xor eax, ecx (2 bytes: 31 C8)
  (func $emit_x86_xor32
    (call $emit_x86_byte (i32.const 0x31))
    (call $emit_x86_modrm (i32.const 3) (i32.const 1) (i32.const 0))
  )

  ;; shl eax, cl (2 bytes: D3 E0)
  (func $emit_x86_shl32
    (call $emit_x86_byte (i32.const 0xD3))
    (call $emit_x86_modrm (i32.const 3) (i32.const 4) (i32.const 0))
  )

  ;; shr eax, cl (2 bytes: D3 E8)
  (func $emit_x86_shr32
    (call $emit_x86_byte (i32.const 0xD3))
    (call $emit_x86_modrm (i32.const 3) (i32.const 5) (i32.const 0))
  )

  ;; sar eax, cl (2 bytes: D3 F8)
  (func $emit_x86_sar32
    (call $emit_x86_byte (i32.const 0xD3))
    (call $emit_x86_modrm (i32.const 3) (i32.const 7) (i32.const 0))
  )

  ;; rol eax, cl (2 bytes: D3 C0)
  (func $emit_x86_rol32
    (call $emit_x86_byte (i32.const 0xD3))
    (call $emit_x86_modrm (i32.const 3) (i32.const 0) (i32.const 0))
  )

  ;; ror eax, cl (2 bytes: D3 C8)
  (func $emit_x86_ror32
    (call $emit_x86_byte (i32.const 0xD3))
    (call $emit_x86_modrm (i32.const 3) (i32.const 1) (i32.const 0))
  )

  ;; btr eax, imm8 (4 bytes: 0F BA F0 imm8) — clear bit of eax
  (func $emit_x86_btr_eax_imm8 (param $v i32)
    (call $emit_x86_byte (i32.const 0x0F))
    (call $emit_x86_byte (i32.const 0xBA))
    (call $emit_x86_modrm (i32.const 3) (i32.const 6) (i32.const 0))  ;; btr eax, imm8
    (call $emit_x86_byte (local.get $v))
  )

  ;; btr rax, imm8 (4 bytes: 48 0F BA F0 imm8) — clear bit of rax
  (func $emit_x86_btr_rax_imm8 (param $v i32)
    (call $emit_x86_rex_w)
    (call $emit_x86_byte (i32.const 0x0F))
    (call $emit_x86_byte (i32.const 0xBA))
    (call $emit_x86_modrm (i32.const 3) (i32.const 6) (i32.const 0))
    (call $emit_x86_byte (local.get $v))
  )

  ;; cmp eax, ecx (2 bytes: 39 C8)
  (func $emit_x86_cmp32
    (call $emit_x86_byte (i32.const 0x39))
    (call $emit_x86_modrm (i32.const 3) (i32.const 1) (i32.const 0))
  )

  ;; mov eax, edx (2 bytes: 89 D0) — accumulate remainder
  (func $emit_x86_mov_eax_edx
    (call $emit_x86_byte (i32.const 0x89))
    (call $emit_x86_modrm (i32.const 3) (i32.const 2) (i32.const 0))
  )

  ;; mov rax, rdx (REX.W + 89 D0 = 48 89 D0)
  (func $emit_x86_mov_rax_rdx
    (call $emit_x86_rex_w)
    (call $emit_x86_byte (i32.const 0x89))
    (call $emit_x86_modrm (i32.const 3) (i32.const 2) (i32.const 0))
  )

  ;; movzx eax, al (3 bytes: 0F B6 C0)
  (func $emit_x86_movzx_eax_al
    (call $emit_x86_byte (i32.const 0x0F))
    (call $emit_x86_byte (i32.const 0xB6))
    (call $emit_x86_modrm (i32.const 3) (i32.const 0) (i32.const 0))
  )

  ;; setcc: 0F 9x+cl (cl = setX opcode byte)
  ;; cl values: 94=sete, 95=setne, 92=setb, 97=seta, 96=setbe, 93=setae,
  ;;            9C=setl, 9F=setg, 9E=setle, 9D=setge
  (func $emit_x86_setcc (param $cl i32)
    (call $emit_x86_byte (i32.const 0x0F))
    (call $emit_x86_byte (local.get $cl))
    (call $emit_x86_modrm (i32.const 3) (i32.const 0) (i32.const 0))
  )

  ;; ── x86_64 i64 (REX.W) helpers ────────────────────────────────────

  ;; REX.W + add rax, rcx (3 bytes: 48 01 C8)
  (func $emit_x86_add64
    (call $emit_x86_rex_w)
    (call $emit_x86_add32)
  )

  ;; REX.W + sub rax, rcx
  (func $emit_x86_sub64
    (call $emit_x86_rex_w)
    (call $emit_x86_sub32)
  )

  ;; REX.W + imul rax, rcx (4 bytes: 48 0F AF C1)
  (func $emit_x86_imul64
    (call $emit_x86_rex_w)
    (call $emit_x86_imul32)
  )

  ;; REX.W + idiv rcx
  (func $emit_x86_idiv64
    (call $emit_x86_rex_w)
    (call $emit_x86_idiv32)
  )

  ;; REX.W + div rcx
  (func $emit_x86_div64
    (call $emit_x86_rex_w)
    (call $emit_x86_div32)
  )

  ;; REX.W + and rax, rcx
  (func $emit_x86_and64
    (call $emit_x86_rex_w)
    (call $emit_x86_and32)
  )

  ;; REX.W + or rax, rcx
  (func $emit_x86_or64
    (call $emit_x86_rex_w)
    (call $emit_x86_or32)
  )

  ;; REX.W + xor rax, rcx
  (func $emit_x86_xor64
    (call $emit_x86_rex_w)
    (call $emit_x86_xor32)
  )

  ;; REX.W + shl rax, cl
  (func $emit_x86_shl64
    (call $emit_x86_rex_w)
    (call $emit_x86_shl32)
  )

  ;; REX.W + shr rax, cl
  (func $emit_x86_shr64
    (call $emit_x86_rex_w)
    (call $emit_x86_shr32)
  )

  ;; REX.W + sar rax, cl
  (func $emit_x86_sar64
    (call $emit_x86_rex_w)
    (call $emit_x86_sar32)
  )

  ;; REX.W + rol rax, cl
  (func $emit_x86_rol64
    (call $emit_x86_rex_w)
    (call $emit_x86_rol32)
  )

  ;; REX.W + ror rax, cl
  (func $emit_x86_ror64
    (call $emit_x86_rex_w)
    (call $emit_x86_ror32)
  )

  ;; REX.W + cmp rax, rcx
  (func $emit_x86_cmp64
    (call $emit_x86_rex_w)
    (call $emit_x86_cmp32)
  )

  ;; REX.W + movsxd rax, eax (4 bytes: 48 63 C0)
  (func $emit_x86_movsxd_rax_eax
    (call $emit_x86_rex_w)
    (call $emit_x86_byte (i32.const 0x63))
    (call $emit_x86_modrm (i32.const 3) (i32.const 0) (i32.const 0))
  )

  ;; mov eax, eax (2 bytes: 89 C0) — zero-extend eax to rax
  (func $emit_x86_mov_eax_eax
    (call $emit_x86_byte (i32.const 0x89))
    (call $emit_x86_modrm (i32.const 3) (i32.const 0) (i32.const 0))
  )

  ;; cmove rdx, rcx (64-bit): if ZF=1, rdx = rcx (4 bytes: 48 0F 44 CA)
  (func $emit_x86_cmove_rdx_rcx
    (call $emit_x86_rex_w)
    (call $emit_x86_byte (i32.const 0x0F))
    (call $emit_x86_byte (i32.const 0x44))
    (call $emit_x86_modrm (i32.const 3) (i32.const 1) (i32.const 2))
  )

  ;; cmovns eax, edx (3 bytes: 0F 49 D0) — eax = edx if SF=0
  (func $emit_x86_cmovns_eax_edx
    (call $emit_x86_byte (i32.const 0x0F))
    (call $emit_x86_byte (i32.const 0x49))
    (call $emit_x86_modrm (i32.const 3) (i32.const 2) (i32.const 0))
  )

  ;; cmovns eax, ecx (3 bytes: 0F 49 C1) — eax = ecx if SF=0
  (func $emit_x86_cmovns_eax_ecx
    (call $emit_x86_byte (i32.const 0x0F))
    (call $emit_x86_byte (i32.const 0x49))
    (call $emit_x86_modrm (i32.const 3) (i32.const 1) (i32.const 0))
  )

  ;; cmovns rax, rdx (4 bytes: 48 0F 49 D0) — rax = rdx if SF=0
  (func $emit_x86_cmovns_rax_rdx
    (call $emit_x86_rex_w)
    (call $emit_x86_byte (i32.const 0x0F))
    (call $emit_x86_byte (i32.const 0x49))
    (call $emit_x86_modrm (i32.const 3) (i32.const 2) (i32.const 0))
  )

  ;; mov ecx, eax (2 bytes: 89 C1)
  (func $emit_x86_mov_ecx_eax
    (call $emit_x86_byte (i32.const 0x89))
    (call $emit_x86_modrm (i32.const 3) (i32.const 0) (i32.const 1))
  )

  ;; mov eax, ecx (2 bytes: 89 C8)
  (func $emit_x86_mov_eax_ecx
    (call $emit_x86_byte (i32.const 0x89))
    (call $emit_x86_modrm (i32.const 3) (i32.const 1) (i32.const 0))
  )

  ;; mov rax, rcx (3 bytes: 48 89 C8) — dest=rax, src=rcx
  (func $emit_x86_mov_rax_rcx
    (call $emit_x86_rex_w)
    (call $emit_x86_byte (i32.const 0x89))
    (call $emit_x86_modrm (i32.const 3) (i32.const 1) (i32.const 0))
  )

  ;; mov rcx, rax (3 bytes: 48 89 C1) — dest=rcx, src=rax
  (func $emit_x86_mov_rcx_rax
    (call $emit_x86_rex_w)
    (call $emit_x86_byte (i32.const 0x89))
    (call $emit_x86_modrm (i32.const 3) (i32.const 0) (i32.const 1))
  )

  ;; sar ecx, 31 (3 bytes: C1 F9 1F)
  (func $emit_x86_sar_ecx_31
    (call $emit_x86_byte (i32.const 0xC1))
    (call $emit_x86_modrm (i32.const 3) (i32.const 7) (i32.const 1))
    (call $emit_x86_byte (i32.const 31))
  )

  ;; sar rcx, 63 (4 bytes: 48 C1 F9 3F)
  (func $emit_x86_sar_rcx_63
    (call $emit_x86_rex_w)
    (call $emit_x86_byte (i32.const 0xC1))
    (call $emit_x86_modrm (i32.const 3) (i32.const 7) (i32.const 1))
    (call $emit_x86_byte (i32.const 63))
  )

  ;; and ecx, imm32 (6 bytes: 81 E1 dword)
  (func $emit_x86_and_ecx_imm32 (param $v i32)
    (call $emit_x86_byte (i32.const 0x81))
    (call $emit_x86_modrm (i32.const 3) (i32.const 4) (i32.const 1))
    (call $emit_x86_dword (local.get $v))
  )

  ;; shl rax, imm8 (4 bytes: 48 C1 E0 imm8)
  (func $emit_x86_shl_rax_imm8 (param $v i32)
    (call $emit_x86_rex_w)
    (call $emit_x86_byte (i32.const 0xC1))
    (call $emit_x86_modrm (i32.const 3) (i32.const 4) (i32.const 0))
    (call $emit_x86_byte (local.get $v))
  )

  ;; ── i32 unary helpers (lzcnt, tzcnt, popcnt) ───────────────────────

  ;; lzcnt eax, eax (4 bytes: F3 0F BD C0)
  (func $emit_x86_lzcnt32
    (call $emit_x86_byte (i32.const 0xF3))
    (call $emit_x86_byte (i32.const 0x0F))
    (call $emit_x86_byte (i32.const 0xBD))
    (call $emit_x86_modrm (i32.const 3) (i32.const 0) (i32.const 0))
  )

  ;; tzcnt eax, eax (4 bytes: F3 0F BC C0)
  (func $emit_x86_tzcnt32
    (call $emit_x86_byte (i32.const 0xF3))
    (call $emit_x86_byte (i32.const 0x0F))
    (call $emit_x86_byte (i32.const 0xBC))
    (call $emit_x86_modrm (i32.const 3) (i32.const 0) (i32.const 0))
  )

  ;; popcnt eax, eax (4 bytes: F3 0F B8 C0)
  (func $emit_x86_popcnt32
    (call $emit_x86_byte (i32.const 0xF3))
    (call $emit_x86_byte (i32.const 0x0F))
    (call $emit_x86_byte (i32.const 0xB8))
    (call $emit_x86_modrm (i32.const 3) (i32.const 0) (i32.const 0))
  )

  ;; REX.W + lzcnt rax, rax (5 bytes: F3 48 0F BD C0)
  (func $emit_x86_lzcnt64
    (call $emit_x86_byte (i32.const 0xF3))
    (call $emit_x86_rex_w)
    (call $emit_x86_byte (i32.const 0x0F))
    (call $emit_x86_byte (i32.const 0xBD))
    (call $emit_x86_modrm (i32.const 3) (i32.const 0) (i32.const 0))
  )

  ;; REX.W + tzcnt rax, rax
  (func $emit_x86_tzcnt64
    (call $emit_x86_byte (i32.const 0xF3))
    (call $emit_x86_rex_w)
    (call $emit_x86_byte (i32.const 0x0F))
    (call $emit_x86_byte (i32.const 0xBC))
    (call $emit_x86_modrm (i32.const 3) (i32.const 0) (i32.const 0))
  )

  ;; REX.W + popcnt rax, rax
  (func $emit_x86_popcnt64
    (call $emit_x86_byte (i32.const 0xF3))
    (call $emit_x86_rex_w)
    (call $emit_x86_byte (i32.const 0x0F))
    (call $emit_x86_byte (i32.const 0xB8))
    (call $emit_x86_modrm (i32.const 3) (i32.const 0) (i32.const 0))
  )

  ;; ── i32 immediate arithmetic (add/sub/imul/and/or/xor eax, imm32) ──

  ;; add eax, imm32 (5 bytes: 05 + dword)
  (func $emit_x86_add_eax_imm32 (param $v i32)
    (call $emit_x86_byte (i32.const 0x05))
    (call $emit_x86_dword (local.get $v))
  )

  ;; sub eax, imm32 (5 bytes: 2D + dword)
  (func $emit_x86_sub_eax_imm32 (param $v i32)
    (call $emit_x86_byte (i32.const 0x2D))
    (call $emit_x86_dword (local.get $v))
  )

  ;; imul eax, eax, imm32 (6 bytes: 69 C0 + dword)
  (func $emit_x86_imul_eax_imm32 (param $v i32)
    (call $emit_x86_byte (i32.const 0x69))
    (call $emit_x86_modrm (i32.const 3) (i32.const 0) (i32.const 0))
    (call $emit_x86_dword (local.get $v))
  )

  ;; and eax, imm32 (5 bytes: 25 + dword)
  (func $emit_x86_and_eax_imm32 (param $v i32)
    (call $emit_x86_byte (i32.const 0x25))
    (call $emit_x86_dword (local.get $v))
  )

  ;; or eax, imm32 (5 bytes: 0D + dword)
  (func $emit_x86_or_eax_imm32 (param $v i32)
    (call $emit_x86_byte (i32.const 0x0D))
    (call $emit_x86_dword (local.get $v))
  )

  ;; xor eax, imm32 (5 bytes: 35 + dword)
  (func $emit_x86_xor_eax_imm32 (param $v i32)
    (call $emit_x86_byte (i32.const 0x35))
    (call $emit_x86_dword (local.get $v))
  )

  ;; shl eax, imm8 (3 bytes: C1 E0 imm8)
  (func $emit_x86_shl_eax_imm8 (param $v i32)
    (call $emit_x86_byte (i32.const 0xC1))
    (call $emit_x86_modrm (i32.const 3) (i32.const 4) (i32.const 0))
    (call $emit_x86_byte (local.get $v))
  )

  ;; shr eax, imm8 (3 bytes: C1 E8 imm8)
  (func $emit_x86_shr_eax_imm8 (param $v i32)
    (call $emit_x86_byte (i32.const 0xC1))
    (call $emit_x86_modrm (i32.const 3) (i32.const 5) (i32.const 0))
    (call $emit_x86_byte (local.get $v))
  )

  ;; sar eax, imm8 (3 bytes: C1 F8 imm8)
  (func $emit_x86_sar_eax_imm8 (param $v i32)
    (call $emit_x86_byte (i32.const 0xC1))
    (call $emit_x86_modrm (i32.const 3) (i32.const 7) (i32.const 0))
    (call $emit_x86_byte (local.get $v))
  )

  ;; rol eax, imm8 (3 bytes: C1 C0 imm8)
  (func $emit_x86_rol_eax_imm8 (param $v i32)
    (call $emit_x86_byte (i32.const 0xC1))
    (call $emit_x86_modrm (i32.const 3) (i32.const 0) (i32.const 0))
    (call $emit_x86_byte (local.get $v))
  )

  ;; ror eax, imm8 (3 bytes: C1 C8 imm8)
  (func $emit_x86_ror_eax_imm8 (param $v i32)
    (call $emit_x86_byte (i32.const 0xC1))
    (call $emit_x86_modrm (i32.const 3) (i32.const 1) (i32.const 0))
    (call $emit_x86_byte (local.get $v))
  )

  ;; ── Call / sub rsp / add rsp helpers ──────────────────────────────

  ;; sub rsp, imm32 (6 bytes: 48 81 EC dword) — allocate stack space
  (func $emit_x86_sub_rsp_imm (param $v i32)
    (call $emit_x86_rex_w)
    (call $emit_x86_byte (i32.const 0x81))
    (call $emit_x86_modrm (i32.const 3) (i32.const 5) (i32.const 4))  ;; sub r/m64, imm32
    (call $emit_x86_dword (local.get $v))
  )

  ;; add rsp, imm32 (6 bytes: 48 81 C4 dword)
  (func $emit_x86_add_rsp_imm (param $v i32)
    (call $emit_x86_rex_w)
    (call $emit_x86_byte (i32.const 0x81))
    (call $emit_x86_modrm (i32.const 3) (i32.const 0) (i32.const 4))  ;; add r/m64, imm32
    (call $emit_x86_dword (local.get $v))
  )

  ;; call rax (2 bytes: FF D0)
  (func $emit_x86_call_rax
    (call $emit_x86_byte (i32.const 0xFF))
    (call $emit_x86_modrm (i32.const 3) (i32.const 2) (i32.const 0))
  )

  ;; jmp rel8 (2 bytes: EB disp8) — short jump
  (func $emit_x86_jmp_rel8 (param $disp i32)
    (call $emit_x86_byte (i32.const 0xEB))
    (call $emit_x86_byte (local.get $disp))
  )

  ;; jmp rel32 (5 bytes: E9 dword)
  (func $emit_x86_jmp_rel32 (param $disp i32)
    (call $emit_x86_byte (i32.const 0xE9))
    (call $emit_x86_dword (local.get $disp))
  )

  ;; jz rel32 (6 bytes: 0F 84 dword)
  (func $emit_x86_jz_rel32 (param $disp i32)
    (call $emit_x86_byte (i32.const 0x0F))
    (call $emit_x86_byte (i32.const 0x84))
    (call $emit_x86_dword (local.get $disp))
  )

  ;; jne rel32 (6 bytes: 0F 85 dword)
  (func $emit_x86_jne_rel32 (param $disp i32)
    (call $emit_x86_byte (i32.const 0x0F))
    (call $emit_x86_byte (i32.const 0x85))
    (call $emit_x86_dword (local.get $disp))
  )

  ;; ── rel8 conditional jumps ──

  ;; jp rel8 (2 bytes: 7A disp8) — jump if parity (PF=1, NaN after ucomiss)
  (func $emit_x86_jp_rel8 (param $disp i32)
    (call $emit_x86_byte (i32.const 0x7A))
    (call $emit_x86_byte (local.get $disp))
  )

  ;; jnp rel8 (2 bytes: 7B disp8) — jump if not parity (PF=0, ordered after ucomiss)
  (func $emit_x86_jnp_rel8 (param $disp i32)
    (call $emit_x86_byte (i32.const 0x7B))
    (call $emit_x86_byte (local.get $disp))
  )

  ;; jae rel8 (2 bytes: 73 disp8) — jump if above or equal (CF=0)
  (func $emit_x86_jae_rel8 (param $disp i32)
    (call $emit_x86_byte (i32.const 0x73))
    (call $emit_x86_byte (local.get $disp))
  )

  ;; jb rel8 (2 bytes: 72 disp8) — jump if below (CF=1)
  (func $emit_x86_jb_rel8 (param $disp i32)
    (call $emit_x86_byte (i32.const 0x72))
    (call $emit_x86_byte (local.get $disp))
  )

  ;; ── Load/store helpers (memory through JitGlobals) ─────────────────

  ;; Load [r15 + offset] into rdx (REX.W + 8B 97 + dword)
  ;; The JIT'd code expects r15 to point to a JitGlobals struct
  (func $emit_x86_load_r15_to_rdx (param $offset i32)
    (call $emit_x86_byte (i32.const 0x49))   ;; REX.W + REX.B (for r15 base)
    (call $emit_x86_byte (i32.const 0x8B))   ;; mov r64, r/m64
    (call $emit_x86_byte (i32.const 0x97))   ;; ModRM: mod=10, reg=rdx(2), rm=r15(7)
    (call $emit_x86_dword (local.get $offset))
  )

  ;; Store rax to [rdx + disp32] (REX.W + 89 82 + dword)
  (func $emit_x86_store_rax_rdx_disp (param $disp i32)
    (call $emit_x86_rex_w)
    (call $emit_x86_byte (i32.const 0x89))   ;; mov r/m64, r64
    (call $emit_x86_byte (i32.const 0x82))   ;; ModRM: mod=10, reg=rax(0), rm=rdx(2)
    (call $emit_x86_dword (local.get $disp))
  )

  ;; Load [rdx + disp32] into rax (REX.W + 8B 82 + dword)
  (func $emit_x86_load_rax_rdx_disp (param $disp i32)
    (call $emit_x86_rex_w)
    (call $emit_x86_byte (i32.const 0x8B))   ;; mov r64, r/m64
    (call $emit_x86_byte (i32.const 0x82))   ;; ModRM: mod=10, reg=rax(0), rm=rdx(2)
    (call $emit_x86_dword (local.get $disp))
  )

  ;; mov rbx, [r15] — cache JitGlobals.locals ptr in rbx (49 8B 9F 00 00 00 00)
  (func $emit_x86_mov_rbx_r15
    (call $emit_x86_byte (i32.const 0x49))   ;; REX.W + REX.B (r15 base)
    (call $emit_x86_byte (i32.const 0x8B))   ;; mov r64, r/m64
    (call $emit_x86_byte (i32.const 0x9F))   ;; ModRM: mod=10, reg=rbx(3), rm=r15(7)
    (call $emit_x86_dword (i32.const 0))
  )

  ;; mov rax, [rbx + disp32] — load local via cached base ptr (48 8B 83 xx xx xx xx)
  (func $emit_x86_load_rax_rbx_disp (param $disp i32)
    (call $emit_x86_rex_w)
    (call $emit_x86_byte (i32.const 0x8B))   ;; mov r64, r/m64
    (call $emit_x86_byte (i32.const 0x83))   ;; ModRM: mod=10, reg=rax(0), rm=rbx(3)
    (call $emit_x86_dword (local.get $disp))
  )

  ;; mov [rbx + disp32], rax — store local via cached base ptr (48 89 83 xx xx xx xx)
  (func $emit_x86_store_rax_rbx_disp (param $disp i32)
    (call $emit_x86_rex_w)
    (call $emit_x86_byte (i32.const 0x89))   ;; mov r/m64, r64
    (call $emit_x86_byte (i32.const 0x83))   ;; ModRM: mod=10, reg=rax(0), rm=rbx(3)
    (call $emit_x86_dword (local.get $disp))
  )

  ;; ── Register-allocated local helpers (r12 = local 0, r13 = local 1) ──

  ;; push r12 (41 54)
  (func $emit_x86_push_r12
    (call $emit_x86_byte (i32.const 0x41))
    (call $emit_x86_byte (i32.const 0x54))
  )

  ;; push r13 (41 55)
  (func $emit_x86_push_r13
    (call $emit_x86_byte (i32.const 0x41))
    (call $emit_x86_byte (i32.const 0x55))
  )

  ;; pop r12 (41 5C)
  (func $emit_x86_pop_r12
    (call $emit_x86_byte (i32.const 0x41))
    (call $emit_x86_byte (i32.const 0x5C))
  )

  ;; pop r13 (41 5D)
  (func $emit_x86_pop_r13
    (call $emit_x86_byte (i32.const 0x41))
    (call $emit_x86_byte (i32.const 0x5D))
  )

  ;; mov r12, [rbx] — load locals[0] into r12 (4C 8B 23)
  (func $emit_x86_load_r12_rbx
    (call $emit_x86_byte (i32.const 0x4C))   ;; REX.W + REX.R (r12 dest)
    (call $emit_x86_byte (i32.const 0x8B))   ;; mov r64, r/m64
    (call $emit_x86_byte (i32.const 0x23))   ;; ModRM: mod=00, reg=r12(4), rm=rbx(3)
  )

  ;; mov r13, [rbx + 8] — load locals[1] into r13 (4C 8B 6B 08)
  (func $emit_x86_load_r13_rbx8
    (call $emit_x86_byte (i32.const 0x4C))   ;; REX.W + REX.R (r13 dest)
    (call $emit_x86_byte (i32.const 0x8B))   ;; mov r64, r/m64
    (call $emit_x86_byte (i32.const 0x6B))   ;; ModRM: mod=01, reg=r13(5), rm=rbx(3)
    (call $emit_x86_byte (i32.const 0x08))
  )

  ;; mov [rbx], r12 — store r12 to locals[0] (4C 89 23)
  (func $emit_x86_store_r12_rbx
    (call $emit_x86_byte (i32.const 0x4C))   ;; REX.W + REX.R (r12 source)
    (call $emit_x86_byte (i32.const 0x89))   ;; mov r/m64, r64
    (call $emit_x86_byte (i32.const 0x23))   ;; ModRM: mod=00, reg=r12(4), rm=rbx(3)
  )

  ;; mov [rbx + 8], r13 — store r13 to locals[1] (4C 89 6B 08)
  (func $emit_x86_store_r13_rbx8
    (call $emit_x86_byte (i32.const 0x4C))   ;; REX.W + REX.R (r13 source)
    (call $emit_x86_byte (i32.const 0x89))   ;; mov r/m64, r64
    (call $emit_x86_byte (i32.const 0x6B))   ;; ModRM: mod=01, reg=r13(5), rm=rbx(3)
    (call $emit_x86_byte (i32.const 0x08))
  )

  ;; mov r12, rax — store eax to register-allocated local 0 (49 89 C4)
  (func $emit_x86_store_rax_r12
    (call $emit_x86_byte (i32.const 0x49))
    (call $emit_x86_byte (i32.const 0x89))
    (call $emit_x86_byte (i32.const 0xC4))
  )

  ;; mov r13, rax — store eax to register-allocated local 1 (49 89 C5)
  (func $emit_x86_store_rax_r13
    (call $emit_x86_byte (i32.const 0x49))
    (call $emit_x86_byte (i32.const 0x89))
    (call $emit_x86_byte (i32.const 0xC5))
  )

  ;; Load [rdx + disp32] into rcx (REX.W + 8B 8A + dword)
  (func $emit_x86_load_rcx_rdx_disp (param $disp i32)
    (call $emit_x86_rex_w)
    (call $emit_x86_byte (i32.const 0x8B))
    (call $emit_x86_byte (i32.const 0x8A))   ;; ModRM: mod=10, reg=rcx(1), rm=rdx(2)
    (call $emit_x86_dword (local.get $disp))
  )

  ;; Store rax to [rsp + disp32] (REX.W + 89 84 24 + dword)
  (func $emit_x86_store_rax_rsp_disp (param $disp i32)
    (call $emit_x86_rex_w)
    (call $emit_x86_byte (i32.const 0x89))
    (call $emit_x86_byte (i32.const 0x84))   ;; ModRM: mod=00, reg=rax(0), rm=SIB(4)
    (call $emit_x86_sib (i32.const 0) (i32.const 4) (i32.const 4))  ;; [rsp]
    (call $emit_x86_dword (local.get $disp))
  )

  ;; Load rax from [rsp + disp32]
  (func $emit_x86_load_rax_rsp_disp (param $disp i32)
    (call $emit_x86_rex_w)
    (call $emit_x86_byte (i32.const 0x8B))
    (call $emit_x86_byte (i32.const 0x84))   ;; mod=00 reg=0 rm=SIB
    (call $emit_x86_sib (i32.const 0) (i32.const 4) (i32.const 4))
    (call $emit_x86_dword (local.get $disp))
  )

  ;; ── Memory load/store (through JitGlobals.mem_ptr) ─────────────────

  ;; i32.load: pop rcx (addr), mov rdx, [r15 + mem_ptr], mov eax, [rdx + rcx]
  ;; Actually: the original uses a register for mem_ptr. Emit: 
  ;;   mov rdx, [r15 + JitGlobals.mem_ptr]
  ;;   mov eax, [rdx + rcx*1]
  (func $emit_x86_mem_load32
    (call $emit_x86_load_r15_to_rdx (i32.const 8))   ;; JitGlobals.mem_ptr at offset 8
    ;; mov eax, [rdx + rcx*1] using [rdx + rcx] addressing
    (call $emit_x86_byte (i32.const 0x8B))   ;; mov r32, r/m32 (no REX)
    (call $emit_x86_byte (i32.const 0x04))   ;; ModRM: mod=00, reg=0(rax), rm=SIB(4)
    (call $emit_x86_sib (i32.const 0) (i32.const 1) (i32.const 2))  ;; [rcx*1 + rdx]
  )

  ;; i64.load: REX.W mov rax, [rdx + rcx]
  (func $emit_x86_mem_load64
    (call $emit_x86_load_r15_to_rdx (i32.const 8))
    (call $emit_x86_rex_w)
    (call $emit_x86_byte (i32.const 0x8B))
    (call $emit_x86_byte (i32.const 0x04))
    (call $emit_x86_sib (i32.const 0) (i32.const 1) (i32.const 2))
  )

  ;; i32.store: pop rcx (addr), pop rax (val), mov rdx, [r15 + mem_ptr], mov [rdx+rcx], eax
  (func $emit_x86_mem_store32
    ;; after pop rcx (addr), pop rax (val)
    (call $emit_x86_load_r15_to_rdx (i32.const 8))
    (call $emit_x86_byte (i32.const 0x89))   ;; mov r/m32, r32
    (call $emit_x86_byte (i32.const 0x04))
    (call $emit_x86_sib (i32.const 0) (i32.const 1) (i32.const 2))  ;; [rcx*1 + rdx]
  )

  ;; i64.store: REX.W mov [rdx+rcx], rax
  (func $emit_x86_mem_store64
    (call $emit_x86_load_r15_to_rdx (i32.const 8))
    (call $emit_x86_rex_w)
    (call $emit_x86_byte (i32.const 0x89))
    (call $emit_x86_byte (i32.const 0x04))
    (call $emit_x86_sib (i32.const 0) (i32.const 1) (i32.const 2))
  )

  ;; i32.load8_s: movsx eax, byte [rdx+rcx]  (0F BE 04 0A)
  (func $emit_x86_mem_load8_s
    (call $emit_x86_load_r15_to_rdx (i32.const 8))
    (call $emit_x86_byte (i32.const 0x0F))
    (call $emit_x86_byte (i32.const 0xBE))
    (call $emit_x86_byte (i32.const 0x04))
    (call $emit_x86_sib (i32.const 0) (i32.const 1) (i32.const 2))
  )

  ;; i32.load8_u: movzx eax, byte [rdx+rcx]  (0F B6 04 0A)
  (func $emit_x86_mem_load8_u
    (call $emit_x86_load_r15_to_rdx (i32.const 8))
    (call $emit_x86_byte (i32.const 0x0F))
    (call $emit_x86_byte (i32.const 0xB6))
    (call $emit_x86_byte (i32.const 0x04))
    (call $emit_x86_sib (i32.const 0) (i32.const 1) (i32.const 2))
  )

  ;; i32.load16_s: movsx eax, word [rdx+rcx]  (0F BF 04 0A)
  (func $emit_x86_mem_load16_s
    (call $emit_x86_load_r15_to_rdx (i32.const 8))
    (call $emit_x86_byte (i32.const 0x0F))
    (call $emit_x86_byte (i32.const 0xBF))
    (call $emit_x86_byte (i32.const 0x04))
    (call $emit_x86_sib (i32.const 0) (i32.const 1) (i32.const 2))
  )

  ;; i32.load16_u: movzx eax, word [rdx+rcx]  (0F B7 04 0A)
  (func $emit_x86_mem_load16_u
    (call $emit_x86_load_r15_to_rdx (i32.const 8))
    (call $emit_x86_byte (i32.const 0x0F))
    (call $emit_x86_byte (i32.const 0xB7))
    (call $emit_x86_byte (i32.const 0x04))
    (call $emit_x86_sib (i32.const 0) (i32.const 1) (i32.const 2))
  )

  ;; i64.load8_s: REX.W movsx rax, byte [rdx+rcx]  (48 0F BE 04 0A)
  (func $emit_x86_mem_load8_s_64
    (call $emit_x86_load_r15_to_rdx (i32.const 8))
    (call $emit_x86_rex_w)
    (call $emit_x86_byte (i32.const 0x0F))
    (call $emit_x86_byte (i32.const 0xBE))
    (call $emit_x86_byte (i32.const 0x04))
    (call $emit_x86_sib (i32.const 0) (i32.const 1) (i32.const 2))
  )

  ;; i64.load16_s: REX.W movsx rax, word [rdx+rcx]  (48 0F BF 04 0A)
  (func $emit_x86_mem_load16_s_64
    (call $emit_x86_load_r15_to_rdx (i32.const 8))
    (call $emit_x86_rex_w)
    (call $emit_x86_byte (i32.const 0x0F))
    (call $emit_x86_byte (i32.const 0xBF))
    (call $emit_x86_byte (i32.const 0x04))
    (call $emit_x86_sib (i32.const 0) (i32.const 1) (i32.const 2))
  )

  ;; i64.load32_s: REX.W movsxd rax, dword [rdx+rcx]  (48 63 04 0A)
  (func $emit_x86_mem_load32_s
    (call $emit_x86_load_r15_to_rdx (i32.const 8))
    (call $emit_x86_rex_w)
    (call $emit_x86_byte (i32.const 0x63))
    (call $emit_x86_byte (i32.const 0x04))
    (call $emit_x86_sib (i32.const 0) (i32.const 1) (i32.const 2))
  )

  ;; i32.store8: mov [rdx+rcx], al  (88 04 0A)
  (func $emit_x86_mem_store8
    (call $emit_x86_load_r15_to_rdx (i32.const 8))
    (call $emit_x86_byte (i32.const 0x88))
    (call $emit_x86_byte (i32.const 0x04))
    (call $emit_x86_sib (i32.const 0) (i32.const 1) (i32.const 2))
  )

  ;; i32.store16: mov [rdx+rcx], ax  (66 89 04 0A)
  (func $emit_x86_mem_store16
    (call $emit_x86_load_r15_to_rdx (i32.const 8))
    (call $emit_x86_byte (i32.const 0x66))   ;; 16-bit operand size prefix
    (call $emit_x86_byte (i32.const 0x89))
    (call $emit_x86_byte (i32.const 0x04))
    (call $emit_x86_sib (i32.const 0) (i32.const 1) (i32.const 2))
  )

  ;; ── SSE helpers ────────────────────────────────────────────────────

  ;; movd xmm0, eax (3 bytes: 66 0F 6E C0)
  (func $emit_x86_movd_xmm0_eax
    (call $emit_x86_byte (i32.const 0x66))
    (call $emit_x86_byte (i32.const 0x0F))
    (call $emit_x86_byte (i32.const 0x6E))
    (call $emit_x86_modrm (i32.const 3) (i32.const 0) (i32.const 0))
  )

  ;; movd xmm1, ecx
  (func $emit_x86_movd_xmm1_ecx
    (call $emit_x86_byte (i32.const 0x66))
    (call $emit_x86_byte (i32.const 0x0F))
    (call $emit_x86_byte (i32.const 0x6E))
    (call $emit_x86_modrm (i32.const 3) (i32.const 1) (i32.const 1))
  )

  ;; movd eax, xmm0 (3 bytes: 66 0F 7E C0)
  (func $emit_x86_movd_eax_xmm0
    (call $emit_x86_byte (i32.const 0x66))
    (call $emit_x86_byte (i32.const 0x0F))
    (call $emit_x86_byte (i32.const 0x7E))
    (call $emit_x86_modrm (i32.const 3) (i32.const 0) (i32.const 0))
  )

  ;; movq xmm0, rax (REX.W + 66 0F 6E C0)
  (func $emit_x86_movq_xmm0_rax
    (call $emit_x86_rex_w)
    (call $emit_x86_byte (i32.const 0x66))
    (call $emit_x86_byte (i32.const 0x0F))
    (call $emit_x86_byte (i32.const 0x6E))
    (call $emit_x86_modrm (i32.const 3) (i32.const 0) (i32.const 0))
  )

  ;; movq xmm1, rcx
  (func $emit_x86_movq_xmm1_rcx
    (call $emit_x86_rex_w)
    (call $emit_x86_byte (i32.const 0x66))
    (call $emit_x86_byte (i32.const 0x0F))
    (call $emit_x86_byte (i32.const 0x6E))
    (call $emit_x86_modrm (i32.const 3) (i32.const 1) (i32.const 1))
  )

  ;; movq xmm1, rax (REX.W + 66 0F 6E C8)
  (func $emit_x86_movq_xmm1_rax
    (call $emit_x86_rex_w)
    (call $emit_x86_byte (i32.const 0x66))
    (call $emit_x86_byte (i32.const 0x0F))
    (call $emit_x86_byte (i32.const 0x6E))
    (call $emit_x86_modrm (i32.const 3) (i32.const 1) (i32.const 0))
  )

  ;; movq rax, xmm0 (REX.W + 66 0F 7E C0)
  (func $emit_x86_movq_rax_xmm0
    (call $emit_x86_rex_w)
    (call $emit_x86_byte (i32.const 0x66))
    (call $emit_x86_byte (i32.const 0x0F))
    (call $emit_x86_byte (i32.const 0x7E))
    (call $emit_x86_modrm (i32.const 3) (i32.const 0) (i32.const 0))
  )

  ;; ── SSE integer/float conversion helpers ─────────────────────────────

  ;; cvtsi2ss xmm0, eax (i32→f32): F3 0F 2A C0
  (func $emit_x86_cvtsi2ss_xmm0_eax
    (call $emit_x86_byte (i32.const 0xF3))
    (call $emit_x86_byte (i32.const 0x0F))
    (call $emit_x86_byte (i32.const 0x2A))
    (call $emit_x86_modrm (i32.const 3) (i32.const 0) (i32.const 0))
  )

  ;; cvtsi2sd xmm0, eax (i32→f64): F2 0F 2A C0
  (func $emit_x86_cvtsi2sd_xmm0_eax
    (call $emit_x86_byte (i32.const 0xF2))
    (call $emit_x86_byte (i32.const 0x0F))
    (call $emit_x86_byte (i32.const 0x2A))
    (call $emit_x86_modrm (i32.const 3) (i32.const 0) (i32.const 0))
  )

  ;; cvtsi2ss xmm0, rax (i64→f32): F3 48 0F 2A C0
  (func $emit_x86_cvtsi2ss_xmm0_rax
    (call $emit_x86_byte (i32.const 0xF3))
    (call $emit_x86_rex_w)
    (call $emit_x86_byte (i32.const 0x0F))
    (call $emit_x86_byte (i32.const 0x2A))
    (call $emit_x86_modrm (i32.const 3) (i32.const 0) (i32.const 0))
  )

  ;; cvtsi2sd xmm0, rax (i64→f64): F2 48 0F 2A C0
  (func $emit_x86_cvtsi2sd_xmm0_rax
    (call $emit_x86_byte (i32.const 0xF2))
    (call $emit_x86_rex_w)
    (call $emit_x86_byte (i32.const 0x0F))
    (call $emit_x86_byte (i32.const 0x2A))
    (call $emit_x86_modrm (i32.const 3) (i32.const 0) (i32.const 0))
  )

  ;; cvttss2si eax, xmm0 (f32→i32): F3 0F 2C C0
  (func $emit_x86_cvttss2si_eax_xmm0
    (call $emit_x86_byte (i32.const 0xF3))
    (call $emit_x86_byte (i32.const 0x0F))
    (call $emit_x86_byte (i32.const 0x2C))
    (call $emit_x86_modrm (i32.const 3) (i32.const 0) (i32.const 0))
  )

  ;; cvttsd2si eax, xmm0 (f64→i32): F2 0F 2C C0
  (func $emit_x86_cvttsd2si_eax_xmm0
    (call $emit_x86_byte (i32.const 0xF2))
    (call $emit_x86_byte (i32.const 0x0F))
    (call $emit_x86_byte (i32.const 0x2C))
    (call $emit_x86_modrm (i32.const 3) (i32.const 0) (i32.const 0))
  )

  ;; cvttss2si rax, xmm0 (f32→i64): F3 48 0F 2C C0
  (func $emit_x86_cvttss2si_rax_xmm0
    (call $emit_x86_byte (i32.const 0xF3))
    (call $emit_x86_rex_w)
    (call $emit_x86_byte (i32.const 0x0F))
    (call $emit_x86_byte (i32.const 0x2C))
    (call $emit_x86_modrm (i32.const 3) (i32.const 0) (i32.const 0))
  )

  ;; cvttsd2si rax, xmm0 (f64→i64): F2 48 0F 2C C0
  (func $emit_x86_cvttsd2si_rax_xmm0
    (call $emit_x86_byte (i32.const 0xF2))
    (call $emit_x86_rex_w)
    (call $emit_x86_byte (i32.const 0x0F))
    (call $emit_x86_byte (i32.const 0x2C))
    (call $emit_x86_modrm (i32.const 3) (i32.const 0) (i32.const 0))
  )

  ;; cvtss2sd xmm0, xmm0 (f32→f64 promote): F3 0F 5A C0
  (func $emit_x86_cvtss2sd_xmm0_xmm0
    (call $emit_x86_byte (i32.const 0xF3))
    (call $emit_x86_byte (i32.const 0x0F))
    (call $emit_x86_byte (i32.const 0x5A))
    (call $emit_x86_modrm (i32.const 3) (i32.const 0) (i32.const 0))
  )

  ;; cvtsd2ss xmm0, xmm0 (f64→f32 demote): F2 0F 5A C0
  (func $emit_x86_cvtsd2ss_xmm0_xmm0
    (call $emit_x86_byte (i32.const 0xF2))
    (call $emit_x86_byte (i32.const 0x0F))
    (call $emit_x86_byte (i32.const 0x5A))
    (call $emit_x86_modrm (i32.const 3) (i32.const 0) (i32.const 0))
  )

  ;; SSE binary op: F3(prefix) + 0F + opcode + modrm(C1 = xmm0, xmm1)
  ;; prefix = 0xF3 for f32 or 0xF2 for f64
  ;; ch = opcode byte (e.g., 0x58=add, 0x5C=sub, 0x59=mul, 0x5E=div)
  (func $emit_x86_sse_op (param $prefix i32) (param $opcode i32)
    (call $emit_x86_byte (local.get $prefix))
    (call $emit_x86_byte (i32.const 0x0F))
    (call $emit_x86_byte (local.get $opcode))
    (call $emit_x86_modrm (i32.const 3) (i32.const 0) (i32.const 1))  ;; xmm0, xmm1
  )

  ;; SSE3A op (roundss/roundsd): 66 + 0F + 3A + opcode + modrm + imm8
  (func $emit_x86_sse3a_op (param $opcode i32) (param $mode i32)
    (call $emit_x86_byte (i32.const 0x66))
    (call $emit_x86_byte (i32.const 0x0F))
    (call $emit_x86_byte (i32.const 0x3A))
    (call $emit_x86_byte (local.get $opcode))
    (call $emit_x86_modrm (i32.const 3) (i32.const 0) (i32.const 0))  ;; xmm0, xmm0
    (call $emit_x86_byte (local.get $mode))
  )

  ;; ucomiss xmm0, xmm1 (4 bytes: 0F 2E C1) — compare f32 scalars
  (func $emit_x86_ucomiss
    (call $emit_x86_byte (i32.const 0x0F))
    (call $emit_x86_byte (i32.const 0x2E))
    (call $emit_x86_modrm (i32.const 3) (i32.const 0) (i32.const 1))
  )

  ;; ucomiss xmm0, xmm0 (4 bytes: 0F 2E C0) — self-compare for NaN check
  (func $emit_x86_ucomiss_xmm0
    (call $emit_x86_byte (i32.const 0x0F))
    (call $emit_x86_byte (i32.const 0x2E))
    (call $emit_x86_modrm (i32.const 3) (i32.const 0) (i32.const 0))
  )

  ;; ucomisd xmm0, xmm1 (5 bytes: 66 0F 2E C1)
  (func $emit_x86_ucomisd
    (call $emit_x86_byte (i32.const 0x66))
    (call $emit_x86_byte (i32.const 0x0F))
    (call $emit_x86_byte (i32.const 0x2E))
    (call $emit_x86_modrm (i32.const 3) (i32.const 0) (i32.const 1))
  )

  ;; ucomisd xmm0, xmm0 (5 bytes: 66 0F 2E C0) — self-compare for NaN check
  (func $emit_x86_ucomisd_xmm0
    (call $emit_x86_byte (i32.const 0x66))
    (call $emit_x86_byte (i32.const 0x0F))
    (call $emit_x86_byte (i32.const 0x2E))
    (call $emit_x86_modrm (i32.const 3) (i32.const 0) (i32.const 0))
  )

  ;; ── f32 unary helpers ──────────────────────────────────────────────

  ;; pop rax, movd xmm0, eax
  (func $emit_x86_f32_unop_prologue
    (call $emit_x86_pop_reg (i32.const 0))
    (call $emit_x86_movd_xmm0_eax)
  )

  ;; movd eax, xmm0, push rax
  (func $emit_x86_f32_unop_epilogue
    (call $emit_x86_movd_eax_xmm0)
    (call $emit_x86_maybe_push_rax)
  )

  ;; ── f64 unary helpers ──────────────────────────────────────────────

  ;; pop rax, movq xmm0, rax
  (func $emit_x86_f64_unop_prologue
    (call $emit_x86_pop_reg (i32.const 0))
    (call $emit_x86_movq_xmm0_rax)
  )

  ;; movq rax, xmm0, push rax
  (func $emit_x86_f64_unop_epilogue
    (call $emit_x86_movq_rax_xmm0)
    (call $emit_x86_maybe_push_rax)
  )

  ;; ── Float comparison helpers ───────────────────────────────────────

  ;; f32 cmp prologue: pop rcx, pop rax, movd xmm1, movd xmm0, ucomiss
  (func $emit_x86_f32_cmp_prologue
    (call $emit_x86_pop_reg (i32.const 1))
    (call $emit_x86_pop_reg (i32.const 0))
    (call $emit_x86_movd_xmm1_ecx)
    (call $emit_x86_movd_xmm0_eax)
    (call $emit_x86_ucomiss)
  )

  ;; f64 cmp prologue: pop rcx, pop rax, movq xmm1, movq xmm0, ucomisd
  (func $emit_x86_f64_cmp_prologue
    (call $emit_x86_pop_reg (i32.const 1))
    (call $emit_x86_pop_reg (i32.const 0))
    (call $emit_x86_movq_xmm1_rcx)
    (call $emit_x86_movq_xmm0_rax)
    (call $emit_x86_ucomisd)
  )

  ;; setnp al (0F 9B C0) — mov ah, al for NaN fixup
  (func $emit_x86_setnp_save_ah
    (call $emit_x86_byte (i32.const 0x0F))
    (call $emit_x86_byte (i32.const 0x9B))   ;; setnp al
    (call $emit_x86_modrm (i32.const 3) (i32.const 0) (i32.const 0))
    ;; mov ah, al (88 E0)
    (call $emit_x86_byte (i32.const 0x88))
    (call $emit_x86_modrm (i32.const 3) (i32.const 4) (i32.const 0))  ;; mov ah, al
  )

  ;; setp al (0F 9A C0) — mov ah, al (for NaN in ne)
  (func $emit_x86_setp_save_ah
    (call $emit_x86_byte (i32.const 0x0F))
    (call $emit_x86_byte (i32.const 0x9A))   ;; setp al
    (call $emit_x86_modrm (i32.const 3) (i32.const 0) (i32.const 0))
    (call $emit_x86_byte (i32.const 0x88))
    (call $emit_x86_modrm (i32.const 3) (i32.const 4) (i32.const 0))
  )

  ;; setcc (cl=opcode), and al, ah, movzx eax, al, push rax
  ;; Used for NaN-safe eq/lt/le where result must be 0 on NaN
  (func $emit_x86_setcc_and_push (param $cl i32)
    (call $emit_x86_setcc (local.get $cl))
    ;; and al, ah (20 E0)
    (call $emit_x86_byte (i32.const 0x20))
    (call $emit_x86_modrm (i32.const 3) (i32.const 4) (i32.const 0))
    (call $emit_x86_movzx_eax_al)
    (call $emit_x86_maybe_push_rax)
  )

  ;; setp al, setne al, or al, ah, movzx, push (for NaN-safe ne)
  (func $emit_x86_setcc_or_push (param $cl i32)
    (call $emit_x86_setcc (local.get $cl))
    ;; or al, ah (08 E0)
    (call $emit_x86_byte (i32.const 0x08))
    (call $emit_x86_modrm (i32.const 3) (i32.const 4) (i32.const 0))
    (call $emit_x86_movzx_eax_al)
    (call $emit_x86_maybe_push_rax)
  )

  ;; emit get_current_code_ptr → i32 (offset from cache base)
  (func $get_x86_code_ptr (result i32)
    (i32.load (global.get $JS_CODE_PTR))
  )

  ;; set current code ptr
  (func $set_x86_code_ptr (param $p i32)
    (i32.store (global.get $JS_CODE_PTR) (local.get $p))
  )

  ;; ── Function prologue ──────────────────────────────────────────────

  ;; Prologue: push rbp; mov rbp, rsp
  (func $emit_x86_prologue
    (call $emit_x86_byte (i32.const 0x55))       ;; push rbp
    (call $emit_x86_rex_w)                        ;; REX.W
    (call $emit_x86_byte (i32.const 0x89))        ;; mov rbp, rsp
    (call $emit_x86_modrm (i32.const 3) (i32.const 4) (i32.const 5))
  )

  ;; Epilogue: pop rbp; ret
  (func $emit_x86_epilogue
    (call $emit_x86_byte (i32.const 0x5D))       ;; pop rbp
    (call $emit_x86_ret)
  )

  ;; ── JIT state helpers ─────────────────────────────────────────────

  ;; Init all JIT state to known values
  (func $jit_reset_state
    (i32.store (global.get $JS_CODE_PTR) (i32.const 0))
    (i32.store (global.get $JS_LABEL_DEPTH) (i32.const 0))
    (i32.store (global.get $JS_FIXUP_COUNT) (i32.const 0))
    (i32.store (global.get $JS_RETURN_EMITTED) (i32.const 0))
    (i32.store (global.get $JS_STACK_DEPTH) (i32.const 0))
    (i32.store (global.get $JS_MAX_STACK) (i32.const 0))
  )

  ;; Emit UD2 (undefined instruction) — 0F 0B
  ;; Used by unreachable and unsupported opcode templates.
  (func $emit_ud2
    (call $emit_x86_byte (i32.const 0x0F))
    (call $emit_x86_byte (i32.const 0x0B))
  )

  ;; Get word at decoded_op_ptr + offset
  (func $get_x86_decoded_imm (param $dec_ptr i32) (param $offset i32) (result i32)
    (i32.load (i32.add (local.get $dec_ptr) (local.get $offset)))
  )

  ;; (export "get_code" signature TBD)
  (func $get_x86_compiled_code (export "get_compiled_code") (param $func_idx i32) (result i32 i32)
    (local $slot i32) (local $base i32)
    (local.set $slot (i32.and (local.get $func_idx) (i32.const 3)))
    (local.set $base (i32.add (global.get $JIT_CACHE) (i32.mul (local.get $slot) (global.get $JIT_SLOT_SIZE))))
    (local.get $base)
    (i32.sub (i32.load (global.get $JS_CODE_PTR)) (i32.mul (local.get $slot) (global.get $JIT_SLOT_SIZE)))
  )

  ;; ═════════════════════════════════════════════════════════════════════
  ;; ELF64 binary output
  ;; ═════════════════════════════════════════════════════════════════════

  (global $ELF_OUT_BUF  i32 (i32.const 0x400000))
  (global $ELF_OUT_OFF  i32 (i32.const 0x700000))  ;; ELF_OUT_BUF - JIT_CACHE

  (global $TEXT_VA      i32 (i32.const 0x400000))
  (global $BSS_VA       i32 (i32.const 0x500000))

  (global $EHDR_SIZE    i32 (i32.const 64))
  (global $PHDR_SIZE    i32 (i32.const 56))
  (global $ELF_STUB_OFF i32 (i32.const 120))       ;; after 1 phdr (64 + 56)
  (global $ELF_CODE_OFF i32 (i32.const 256))        ;; aligned after stub
  (global $BSS_SIZE     i32 (i32.const 0x100000))   ;; 1MB — covers import_count (0x4108), syscall_map (0x90000), runtime state

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

  ;; MOV dword [r14+disp32], imm32: 41 C7 86 <disp32> <imm32>
  (func $emit_mov_dword_r14disp32 (param $disp i32) (param $val i32)
    (call $emit_x86_byte (i32.const 0x41))
    (call $emit_x86_byte (i32.const 0xC7))
    (call $emit_x86_byte (i32.const 0x86))
    (call $emit_x86_dword (local.get $disp))
    (call $emit_x86_dword (local.get $val))
  )

  ;; MOV r14, [r15+8] (load $mem): 4D 8B 77 08
  (func $emit_mov_r14_r15off8
    (call $emit_x86_byte (i32.const 0x4D))
    (call $emit_x86_byte (i32.const 0x8B))
    (call $emit_x86_byte (i32.const 0x77))
    (call $emit_x86_byte (i32.const 0x08))
  )

  ;; MOV esi, imm32 (zero-extends to rsi): BE <dword>
  (func $emit_mov_esi_imm32 (param $val i32)
    (call $emit_x86_byte (i32.const 0xBE))
    (call $emit_x86_dword (local.get $val))
  )

  ;; MOV rdi, [r15+8] (load $mem): 49 8B 7F 08
  (func $emit_mov_rdi_r15off8
    (call $emit_x86_byte (i32.const 0x49))
    (call $emit_x86_byte (i32.const 0x8B))
    (call $emit_x86_byte (i32.const 0x7F))
    (call $emit_x86_byte (i32.const 0x08))
  )

  ;; ADD rdi, imm32: 48 81 C7 <dword>
  (func $emit_add_rdi_imm32 (param $val i32)
    (call $emit_x86_rex_w)
    (call $emit_x86_byte (i32.const 0x81))
    (call $emit_x86_byte (i32.const 0xC7))
    (call $emit_x86_dword (local.get $val))
  )

  ;; MOV ecx, imm32: B9 <dword>
  (func $emit_mov_ecx_imm32 (param $val i32)
    (call $emit_x86_byte (i32.const 0xB9))
    (call $emit_x86_dword (local.get $val))
  )

  ;; REP MOVSB: F3 A4
  (func $emit_rep_movsb
    (call $emit_x86_byte (i32.const 0xF3))
    (call $emit_x86_byte (i32.const 0xA4))
  )

  ;; MOV eax, imm32: B8 <dword>
  (func $emit_mov_eax_imm (param $val i32)
    (call $emit_x86_byte (i32.const 0xB8))
    (call $emit_x86_dword (local.get $val))
  )

  ;; ── ELF64 header (64 bytes) ───────────────────────────────────────

  (func $emit_elf64_ehdr (param $entry_va i32) (param $phoff i32) (param $phnum i32)
    (call $emit_x86_byte (i32.const 0x7F))
    (call $emit_x86_byte (i32.const 0x45)) (call $emit_x86_byte (i32.const 0x4C)) (call $emit_x86_byte (i32.const 0x46))
    (call $emit_x86_byte (i32.const 2))     (call $emit_x86_byte (i32.const 1))
    (call $emit_x86_byte (i32.const 1))     (call $emit_x86_byte (i32.const 0))
    (call $emit_x86_qword (i64.const 0))    ;; padding bytes 8-15
    (call $emit_x86_byte (i32.const 2)) (call $emit_x86_byte (i32.const 0))  ;; e_type=ET_EXEC
    (call $emit_x86_byte (i32.const 0x3E)) (call $emit_x86_byte (i32.const 0))  ;; e_machine=x86_64
    (call $emit_x86_dword (i32.const 1))    ;; e_version
    (call $emit_x86_qword (i64.extend_i32_u (local.get $entry_va)))  ;; e_entry
    (call $emit_x86_qword (i64.extend_i32_u (local.get $phoff)))     ;; e_phoff
    (call $emit_x86_qword (i64.const 0))    ;; e_shoff
    (call $emit_x86_dword (i32.const 0))    ;; e_flags
    (call $emit_x86_byte (i32.const 64)) (call $emit_x86_byte (i32.const 0))  ;; e_ehsize
    (call $emit_x86_byte (i32.const 56)) (call $emit_x86_byte (i32.const 0))  ;; e_phentsize
    (call $emit_x86_byte (i32.wrap_i64 (i64.extend_i32_u (local.get $phnum))))
    (call $emit_x86_byte (i32.const 0))   ;; e_phnum
    (call $emit_x86_byte (i32.const 0)) (call $emit_x86_byte (i32.const 0))  ;; e_shentsize=0
    (call $emit_x86_byte (i32.const 0)) (call $emit_x86_byte (i32.const 0))  ;; e_shnum=0
    (call $emit_x86_byte (i32.const 0)) (call $emit_x86_byte (i32.const 0))  ;; e_shstrndx=0
  )

  ;; ── ELF64 program header (56 bytes) ──────────────────────────────

  (func $emit_elf64_phdr (param $type i32) (param $flags i32)
                         (param $offset i32) (param $vaddr i32)
                         (param $filesz i32) (param $memsz i32)
    (call $emit_x86_dword (local.get $type))     ;; p_type
    (call $emit_x86_dword (local.get $flags))    ;; p_flags
    (call $emit_x86_qword (i64.extend_i32_u (local.get $offset)))  ;; p_offset
    (call $emit_x86_qword (i64.extend_i32_u (local.get $vaddr)))   ;; p_vaddr
    (call $emit_x86_qword (i64.extend_i32_u (local.get $vaddr)))   ;; p_paddr = p_vaddr
    (call $emit_x86_qword (i64.extend_i32_u (local.get $filesz)))  ;; p_filesz
    (call $emit_x86_qword (i64.extend_i32_u (local.get $memsz)))   ;; p_memsz
    (call $emit_x86_qword (i64.const 0x1000))    ;; p_align
  )

  ;; ── Runtime stub: _start that calls compiled code and exits ──────
  ;; $bss_va = page-aligned start of BSS (right after compiled code)

  (func $emit_elf_stub (param $bss_va i32) (param $syscall_data_va i32) (param $import_count i32)
    (local $jitglobs i32) (local $mem i32) (local $locals i32)
    (local $globals i32) (local $table i32) (local $syscall_data_size i32)

    (local.set $jitglobs (local.get $bss_va))
    (local.set $mem     (i32.add (local.get $bss_va) (i32.const 0x80)))
    (local.set $locals  (i32.add (local.get $bss_va) (i32.const 0x10080)))
    (local.set $globals (i32.add (local.get $bss_va) (i32.const 0x20080)))
    (local.set $table   (i32.add (local.get $bss_va) (i32.const 0x30080)))
    (local.set $syscall_data_size (i32.shl (local.get $import_count) (i32.const 2)))

    ;; mov r15, $jitglobs
    (call $emit_mov_r64_imm32 (i32.const 0x4D) (i32.const 0xC7) (local.get $jitglobs))

    ;; mov rax, $mem; mov [r15+8], rax   (JitGlobals.mem_ptr)
    (call $emit_mov_r64_imm32 (i32.const 0x48) (i32.const 0xC0) (local.get $mem))
    (call $emit_mov_r15off_rax (i32.const 8))

    ;; mov rax, $locals; mov [r15+0], rax  (JitGlobals.locals)
    (call $emit_mov_r64_imm32 (i32.const 0x48) (i32.const 0xC0) (local.get $locals))
    (call $emit_mov_r15off_rax (i32.const 0))

    ;; mov rax, $globals; mov [r15+24], rax (JitGlobals.globals_buf)
    (call $emit_mov_r64_imm32 (i32.const 0x48) (i32.const 0xC0) (local.get $globals))
    (call $emit_mov_r15off_rax (i32.const 24))

    ;; mov rax, $table; mov [r15+48], rax  (JitGlobals.table_entries)
    (call $emit_mov_r64_imm32 (i32.const 0x48) (i32.const 0xC0) (local.get $table))
    (call $emit_mov_r15off_rax (i32.const 48))

    ;; mov dword [r15+64], 1  (JitGlobals.memory_pages)
    (call $emit_mov_dword_r15off (i32.const 64) (i32.const 1))

    ;; ── Initialize syscall map and import count for JIT-compiled code ──
    ;; The JIT's template_x86_call reads import count from WASM addr 0x4108
    ;; and sysno[i] from WASM addr 0x90000 + i*4.
    ;; These are in the BSS (zeroed by OS), so we write them here.

    ;; r14 = $mem (base of WASM linear memory)
    (call $emit_mov_r14_r15off8)

    ;; Store import_count at $mem + 0x4108
    (call $emit_mov_dword_r14disp32 (i32.const 0x4108) (local.get $import_count))

    ;; Copy syscall map data from text segment to $mem + 0x90000
    ;; Source: rsi = syscall_data_va (embedded in ELF text after compiled code)
    ;; Dest:   rdi = $mem + 0x90000
    ;; Count:  ecx = import_count * 4
    (call $emit_mov_esi_imm32 (local.get $syscall_data_va))
    (call $emit_mov_rdi_r15off8)
    (call $emit_add_rdi_imm32 (i32.const 0x90000))
    (call $emit_mov_ecx_imm32 (local.get $syscall_data_size))
    (call $emit_rep_movsb)

    ;; Load locals pointer into rbx
    (call $emit_x86_mov_rbx_r15)

    ;; Load locals[0] into r12, locals[1] into r13 (function params)
    (call $emit_x86_load_r12_rbx)
    (call $emit_x86_load_r13_rbx8)

    ;; call compiled_code
    (call $emit_call_rel (i32.add (global.get $TEXT_VA) (global.get $ELF_CODE_OFF)))

    ;; exit(result)  — rax holds the return value from compiled code
    (call $emit_mov_edi_eax)
    (call $emit_mov_eax_imm (global.get $LINUX_SYS_X64_EXIT))  ;; SYS_exit
    (call $emit_syscall)
  )

  ;; ── Copy compiled code from cache into ELF output ────────────────

  (func $copy_compiled_code (param $src i32) (param $size i32)
    (local $i i32) (local $dst i32)
    (local.set $dst (i32.add (global.get $ELF_OUT_BUF) (global.get $ELF_CODE_OFF)))
    (local.set $i (i32.const 0))
    (block $done
      (loop $copy
        (if (i32.ge_u (local.get $i) (local.get $size))
          (then (br $done))
        )
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
  ;; Returns (buffer_address, total_size)
  (func (export "compile_to_elf") (param $func_idx i32) (result i32 i32)
    (local $code_size i32) (local $total_size i32) (local $import_count i32)
    (local $syscall_data_size i32) (local $saved i32) (local $bss_va i32)
    (local $data_va i32) (local $i i32) (local $sysno i32)

    ;; 1. Run the compiler (produces code at 0x100000)
    (local.set $code_size (call $jit_compile (local.get $func_idx)))

    ;; 2. Read import count from interpreter state (WASM addr 0x4108)
    (local.set $import_count (i32.load (i32.const 0x4108)))
    (local.set $syscall_data_size (i32.shl (local.get $import_count) (i32.const 2)))

    ;; 3. Compute total ELF file size (header + stub + code + syscall map data)
    (local.set $total_size
      (i32.add
        (i32.add (global.get $ELF_CODE_OFF) (local.get $code_size))
        (local.get $syscall_data_size)
      )
    )

    ;; 4. Compute BSS base: page-align TEXT_VA + total_size
    (local.set $bss_va
      (i32.add
        (i32.and (i32.add (local.get $total_size) (i32.const 0xFFF)) (i32.const -0x1000))
        (global.get $TEXT_VA)
      )
    )

    ;; 5. Compute VA of syscall map data (right after compiled code)
    (local.set $data_va
      (i32.add
        (global.get $TEXT_VA)
        (i32.add (global.get $ELF_CODE_OFF) (local.get $code_size))
      )
    )

    ;; 6. Save JS_CODE_PTR, redirect emit to ELF output buffer
    (local.set $saved (i32.load (global.get $JS_CODE_PTR)))
    (i32.store (global.get $JS_CODE_PTR) (global.get $ELF_OUT_OFF))

    ;; 7. Emit ELF64 header (1 program header: text + BSS combined)
    (call $emit_elf64_ehdr
      (i32.add (global.get $TEXT_VA) (global.get $ELF_STUB_OFF))  ;; entry = _start
      (i32.const 64)   ;; e_phoff
      (i32.const 1)    ;; 1 program header (text + bss)
    )

    ;; 8. Emit PH#1: text + BSS combined LOAD (R|W|X), offset=0
    ;;     filesz = total_size, memsz includes BSS
    (call $emit_elf64_phdr
      (i32.const 1)         ;; PT_LOAD
      (i32.const 7)         ;; PF_R | PF_W | PF_X
      (i32.const 0)         ;; p_offset = 0
      (global.get $TEXT_VA) ;; p_vaddr
      (local.get $total_size) ;; p_filesz
      (i32.add (i32.and (i32.add (local.get $total_size) (i32.const 0xFFF)) (i32.const -0x1000)) (global.get $BSS_SIZE)) ;; p_memsz includes BSS
    )

    ;; 9. Emit runtime stub (with BSS base, syscall data VA, import count)
    (call $emit_elf_stub (local.get $bss_va) (local.get $data_va) (local.get $import_count))

    ;; 10. Pad with zeros from end of stub to ELF_CODE_OFF
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

    ;; 11. Copy compiled code from cache (0x100000) to ELF output
    (call $copy_compiled_code (global.get $JIT_CACHE) (local.get $code_size))

    ;; 12. Advance JS_CODE_PTR past the compiled code
    (i32.store (global.get $JS_CODE_PTR)
      (i32.add (i32.load (global.get $JS_CODE_PTR)) (local.get $code_size))
    )

    ;; 13. Write syscall map data (read from interpreter's syscall map at 0x90000)
    (local.set $i (i32.const 0))
    (block $data_loop_done
      (loop $data_loop
        (if (i32.ge_u (local.get $i) (local.get $import_count))
          (then (br $data_loop_done))
        )
        (local.set $sysno
          (i32.load (i32.add (i32.const 0x90000) (i32.shl (local.get $i) (i32.const 2))))
        )
        (call $emit_x86_dword (local.get $sysno))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $data_loop)
      )
    )

    ;; 14. Restore JS_CODE_PTR
    (i32.store (global.get $JS_CODE_PTR) (local.get $saved))

    ;; Return (buffer_address, total_size)
    (return (global.get $ELF_OUT_BUF) (local.get $total_size))
  )
