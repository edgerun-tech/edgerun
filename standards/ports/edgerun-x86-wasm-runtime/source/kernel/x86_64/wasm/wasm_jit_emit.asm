; ==================================================================
; wasm_jit_emit.asm — x86_64 instruction encoder for JIT compiler
;
; All functions write to the code cache at [jit_state.code_ptr].
; They advance code_ptr past the emitted instruction and return.
; No registers clobbered beyond explicit outputs.
; =================================================================+

; ------------------------------------------------------------------
; Write single byte
; al = byte
; -----------------------------------------------------------------+
er_fn jit_emit_byte
    mov     rdi, [jit_state.code_ptr]
    mov     [rdi], al
    inc     qword [jit_state.code_ptr]
    ret

; ------------------------------------------------------------------
; Write 4-byte little-endian dword
; eax = value
; -----------------------------------------------------------------+
er_fn jit_emit_dword
    mov     rdi, [jit_state.code_ptr]
    mov     [rdi], eax
    add     qword [jit_state.code_ptr], 4
    ret

; ------------------------------------------------------------------
; Write 8-byte qword
; rax = value
; -----------------------------------------------------------------+
er_fn jit_emit_qword
    mov     rdi, [jit_state.code_ptr]
    mov     [rdi], rax
    add     qword [jit_state.code_ptr], 8
    ret

; ------------------------------------------------------------------
; Write ModRM byte: mod(2) + reg(3) + rm(3)
; al = modrm value
; -----------------------------------------------------------------+
er_fn jit_emit_modrm
    mov     rdi, [jit_state.code_ptr]
    mov     [rdi], al
    inc     qword [jit_state.code_ptr]
    ret

; ------------------------------------------------------------------
; Write SIB byte: scale(2) + index(3) + base(3)
; al = sib value
; -----------------------------------------------------------------+
er_fn jit_emit_sib
    mov     rdi, [jit_state.code_ptr]
    mov     [rdi], al
    inc     qword [jit_state.code_ptr]
    ret

; ------------------------------------------------------------------
; Build ModRM: mod + reg + rm
; edi = mod (0-3), ecx = reg (0-7), edx = rm (0-7)
; Returns ModRM byte in al
; -----------------------------------------------------------------+
er_fn jit_build_modrm
    mov     eax, edi        ; mod
    and     eax, 3
    shl     eax, 6
    mov     r10d, ecx       ; reg
    and     r10d, 7
    shl     r10d, 3
    or      eax, r10d
    mov     r10d, edx       ; rm
    and     r10d, 7
    or      eax, r10d
    ret

; ------------------------------------------------------------------
; Build SIB: scale + index + base
; cl = scale (0-3), ch = index (0-7), edx_low = base (0-7)
; Returns SIB byte in al
; -----------------------------------------------------------------+
er_fn jit_build_sib
    mov     eax, edx        ; base
    and     eax, 7
    mov     ecx, esi        ; index
    and     ecx, 7
    shl     ecx, 3          ; index << 3
    or      eax, ecx
    mov     ecx, edi        ; scale
    and     ecx, 3
    shl     ecx, 6          ; scale << 6
    or      eax, ecx
    ret

; ------------------------------------------------------------------
; Write REX prefix: W=cl, R=ch, X=r8b_low, B=r9b_low
; -----------------------------------------------------------------+
er_fn jit_emit_rex
    mov     eax, 0x40       ; REX base
    test    cl, 1
    jz      .no_w
    or      al, 0x08        ; REX.W
.no_w:
    test    ch, 1
    jz      .no_r
    or      al, 0x04        ; REX.R
.no_r:
    test    r8b, 1
    jz      .no_x
    or      al, 0x02        ; REX.X
.no_x:
    test    r9b, 1
    jz      .no_b
    or      al, 0x01        ; REX.B
.no_b:
    mov     rdi, [jit_state.code_ptr]
    mov     [rdi], al
    inc     qword [jit_state.code_ptr]
    ret

; ------------------------------------------------------------------
; Emit: mov rax, [r15 + offset]  — load global from JitGlobals
; offset in eax
; -----------------------------------------------------------------+
er_fn jit_emit_load_global
    push    rax
    ; REX.W = 1, REX.B = 1 (r15 base requires REX.B)
    mov     cl, 1            ; W=1 (64-bit)
    mov     ch, 0            ; R=0
    mov     r8b, 0           ; X=0
    mov     r9b, 1           ; B=1 (r15)
    call    jit_emit_rex
    ; opcode: 8B (mov r64, r/m64) with /r encoding
    mov     al, 0x8B
    call    jit_emit_byte
    ; ModRM: mod=10 ([reg+disp32]), reg=0 (rax), rm=15 (r15 with REX.B)
    mov     edi, 2           ; mod=10
    mov     ecx, 0           ; reg=0 (rax)
    mov     edx, 15          ; rm=15 -> r15 (with REX.B)
    call    jit_build_modrm
    call    jit_emit_modrm
    ; displacement dword
    pop     rax
    call    jit_emit_dword
    ret

; ------------------------------------------------------------------
; Emit: mov reg, [r15 + offset]  — load global into any reg
; cl = register (0-15), eax = offset in JitGlobals
; -----------------------------------------------------------------+
er_fn jit_emit_load_global_to_reg
    er_push rax, rcx
    ; Determine REX
    mov     ch, 0            ; R=0 (reg in ModRM.reg)
    test    cl, 8
    jz      .lgt_no_r
    mov     ch, 1            ; REX.R for reg > 7
.lgt_no_r:
    mov     r9b, 1           ; B=1 (r15 base)
    mov     r8b, 0           ; X=0
    mov     cl, 1            ; W=1 (64-bit)
    call    jit_emit_rex
    mov     al, 0x8B         ; mov r64, r/m64
    call    jit_emit_byte
    ; ModRM: mod=10, reg=src_reg, rm=15 (r15)
    pop     rcx              ; original cl = register
    push    rcx
    and     ecx, 7           ; lower 3 bits of reg
    mov     edx, 15          ; rm=15
    mov     edi, 2           ; mod=10
    call    jit_build_modrm
    call    jit_emit_modrm
    er_pop  rax, rcx
    call    jit_emit_dword   ; displacement
    ret

; ------------------------------------------------------------------
; Emit: mov [r15 + offset], reg  — store reg to global
; cl = register (0-15), eax = offset in JitGlobals
; -----------------------------------------------------------------+
er_fn jit_emit_store_global_from_reg
    er_push rax, rcx
    mov     ch, 0
    test    cl, 8
    jz      .sgt_no_r
    mov     ch, 1
.sgt_no_r:
    mov     r9b, 1           ; B=1 (r15 base)
    mov     r8b, 0
    mov     cl, 1            ; W=1
    call    jit_emit_rex
    mov     al, 0x89         ; mov r/m64, r64
    call    jit_emit_byte
    pop     rcx
    push    rcx
    and     ecx, 7
    mov     edx, 15          ; rm=15 (r15)
    mov     edi, 2           ; mod=10
    call    jit_build_modrm
    call    jit_emit_modrm
    er_pop  rax, rcx
    call    jit_emit_dword
    ret

; ------------------------------------------------------------------
; Emit 64-bit: mov reg64, imm64
; cl = register, rax = imm64
; -----------------------------------------------------------------+
er_fn jit_emit_mov_reg_imm64
    push    rax
    ; REX.W + B if reg > 7
    mov     cl, 1            ; W=1
    mov     ch, 0
    mov     r8b, 0
    mov     r9b, 0
    pop     rax              ; original rax (imm64)
    er_push rax, rcx
    pop     rcx              ; original cl back
    test    cl, 8
    jz      .mri_no_b
    mov     r9b, 1
.mri_no_b:
    push    rcx
    call    jit_emit_rex
    pop     rcx
    ; mov reg, imm64 opcode: 0xB8 + low3(reg)
    mov     al, 0xB8
    and     ecx, 7
    or      al, cl
    call    jit_emit_byte
    ; imm64
    pop     rax
    call    jit_emit_qword
    ret

; ------------------------------------------------------------------
; Emit: add eax, ecx  (32-bit, no REX.W)
; -----------------------------------------------------------------+
er_fn jit_emit_add32
    mov     al, 0x01         ; add r/m32, r32
    call    jit_emit_byte
    mov     al, 0xC8         ; ModRM: mod=11, reg=1(ecx), rm=0(eax) → add eax, ecx
    call    jit_emit_modrm
    ret

; ------------------------------------------------------------------
; Emit: add eax, imm32
; eax = imm32
; -----------------------------------------------------------------+
er_fn jit_emit_add_eax_imm32
    push    rax
    mov     al, 0x05
    call    jit_emit_byte
    pop     rax
    call    jit_emit_dword
    ret

; ------------------------------------------------------------------
; Emit: imul eax, eax, imm32
; eax = imm32
; -----------------------------------------------------------------+
er_fn jit_emit_imul_eax_imm32
    push    rax
    mov     al, 0x69
    call    jit_emit_byte
    mov     al, 0xC0
    call    jit_emit_modrm
    pop     rax
    call    jit_emit_dword
    ret

; ------------------------------------------------------------------
; Emit: or eax, imm32
; eax = imm32
; -----------------------------------------------------------------+
er_fn jit_emit_or_eax_imm32
    push    rax
    mov     al, 0x0D
    call    jit_emit_byte
    pop     rax
    call    jit_emit_dword
    ret

; ------------------------------------------------------------------
; Emit: xor eax, imm32
; eax = imm32
; -----------------------------------------------------------------+
er_fn jit_emit_xor_eax_imm32
    push    rax
    mov     al, 0x35
    call    jit_emit_byte
    pop     rax
    call    jit_emit_dword
    ret

; ------------------------------------------------------------------
; Emit: shl eax, imm8
; al = imm8
; -----------------------------------------------------------------+
er_fn jit_emit_shl_eax_imm8
    push    rax
    mov     al, 0xC1
    call    jit_emit_byte
    mov     al, 0xE0
    call    jit_emit_modrm
    pop     rax
    call    jit_emit_byte
    ret

; ------------------------------------------------------------------
; Emit: shr eax, imm8
; al = imm8
; -----------------------------------------------------------------+
er_fn jit_emit_shr_eax_imm8
    push    rax
    mov     al, 0xC1
    call    jit_emit_byte
    mov     al, 0xE8
    call    jit_emit_modrm
    pop     rax
    call    jit_emit_byte
    ret

; ------------------------------------------------------------------
; Emit: shr edx, imm8
; al = imm8
; -----------------------------------------------------------------+
er_fn jit_emit_shr_edx_imm8
    push    rax
    mov     al, 0xC1
    call    jit_emit_byte
    mov     al, 0xEA
    call    jit_emit_modrm
    pop     rax
    call    jit_emit_byte
    ret

; ------------------------------------------------------------------
; Emit: sar eax, imm8
; al = imm8
; -----------------------------------------------------------------+
er_fn jit_emit_sar_eax_imm8
    push    rax
    mov     al, 0xC1
    call    jit_emit_byte
    mov     al, 0xF8
    call    jit_emit_modrm
    pop     rax
    call    jit_emit_byte
    ret

; ------------------------------------------------------------------
; Emit: rol eax, imm8
; al = imm8
; -----------------------------------------------------------------+
er_fn jit_emit_rol_eax_imm8
    push    rax
    mov     al, 0xC1
    call    jit_emit_byte
    mov     al, 0xC0
    call    jit_emit_modrm
    pop     rax
    call    jit_emit_byte
    ret

; ------------------------------------------------------------------
; Emit: ror eax, imm8
; al = imm8
; -----------------------------------------------------------------+
er_fn jit_emit_ror_eax_imm8
    push    rax
    mov     al, 0xC1
    call    jit_emit_byte
    mov     al, 0xC8
    call    jit_emit_modrm
    pop     rax
    call    jit_emit_byte
    ret

; ------------------------------------------------------------------
; Emit: mov ecx, eax
; -----------------------------------------------------------------+
er_fn jit_emit_mov_ecx_eax
    mov     al, 0x89
    call    jit_emit_byte
    mov     al, 0xC1
    call    jit_emit_modrm
    ret

; ------------------------------------------------------------------
; Emit: mov edx, ecx
; -----------------------------------------------------------------+
er_fn jit_emit_mov_edx_ecx
    mov     al, 0x89
    call    jit_emit_byte
    mov     al, 0xCA
    call    jit_emit_modrm
    ret

; ------------------------------------------------------------------
; Emit: add eax, edx
; -----------------------------------------------------------------+
er_fn jit_emit_add_eax_edx
    mov     al, 0x01
    call    jit_emit_byte
    mov     al, 0xD0
    call    jit_emit_modrm
    ret

; ------------------------------------------------------------------
; Emit: mov ecx, [rdx + disp32]
; eax = disp32
; -----------------------------------------------------------------+
er_fn jit_emit_load_ecx_from_rdx_disp32
    push    rax
    mov     al, 0x8B
    call    jit_emit_byte
    mov     al, 0x8A
    call    jit_emit_modrm
    pop     rax
    call    jit_emit_dword
    ret

; ------------------------------------------------------------------
; Emit: mov [rdx + disp32], rax
; eax = disp32
; -----------------------------------------------------------------+
er_fn jit_emit_store_rax_to_rdx_disp32
    push    rax
    mov     cl, 1
    call    jit_emit_rex_nob
    mov     al, 0x89
    call    jit_emit_byte
    mov     al, 0x82
    call    jit_emit_modrm
    pop     rax
    call    jit_emit_dword
    ret

; ------------------------------------------------------------------
; Emit: shr ecx, imm8
; al = imm8
; -----------------------------------------------------------------+
er_fn jit_emit_shr_ecx_imm8
    push    rax
    mov     al, 0xC1
    call    jit_emit_byte
    mov     al, 0xE9
    call    jit_emit_modrm
    pop     rax
    call    jit_emit_byte
    ret

; ------------------------------------------------------------------
; Emit: sub eax, ecx  (32-bit)
; -----------------------------------------------------------------+
er_fn jit_emit_sub32
    mov     al, 0x29
    call    jit_emit_byte
    mov     al, 0xC8
    call    jit_emit_modrm
    ret

; ------------------------------------------------------------------
; Emit: imul eax, ecx  (32-bit signed multiply)
; Note: imul r32, r/m32 uses reg=dest, rm=src (opposite of add/sub)
; -----------------------------------------------------------------+
er_fn jit_emit_imul32
    mov     al, 0x0F
    call    jit_emit_byte
    mov     al, 0xAF
    call    jit_emit_byte
    mov     al, 0xC1         ; ModRM: mod=11, reg=0(eax=dest), rm=1(ecx=src) → imul eax, ecx
    call    jit_emit_modrm
    ret

; ------------------------------------------------------------------
; Emit: and eax, ecx  (32-bit)
; -----------------------------------------------------------------+
er_fn jit_emit_and32
    mov     al, 0x21
    call    jit_emit_byte
    mov     al, 0xC8
    call    jit_emit_modrm
    ret

; ------------------------------------------------------------------
; Emit: or eax, ecx  (32-bit)
; -----------------------------------------------------------------+
er_fn jit_emit_or32
    mov     al, 0x09
    call    jit_emit_byte
    mov     al, 0xC8
    call    jit_emit_modrm
    ret

; ------------------------------------------------------------------
; Emit: xor eax, ecx  (32-bit)
; -----------------------------------------------------------------+
er_fn jit_emit_xor32
    mov     al, 0x31
    call    jit_emit_byte
    mov     al, 0xC8
    call    jit_emit_modrm
    ret

; ------------------------------------------------------------------
; Emit: shl eax, cl  (32-bit shift)
; -----------------------------------------------------------------+
er_fn jit_emit_shl32
    mov     al, 0xD3
    call    jit_emit_byte
    mov     al, 0xE0         ; shl eax, cl
    call    jit_emit_modrm
    ret

; ------------------------------------------------------------------
; Emit: shr eax, cl  (32-bit)
; -----------------------------------------------------------------+
er_fn jit_emit_shr32
    mov     al, 0xD3
    call    jit_emit_byte
    mov     al, 0xE8         ; shr eax, cl
    call    jit_emit_modrm
    ret

; ------------------------------------------------------------------
; Emit: sar eax, cl  (32-bit arithmetic)
; -----------------------------------------------------------------+
er_fn jit_emit_sar32
    mov     al, 0xD3
    call    jit_emit_byte
    mov     al, 0xF8         ; sar eax, cl
    call    jit_emit_modrm
    ret

; ------------------------------------------------------------------
; Emit: ror eax, cl  (32-bit rotate right)
; -----------------------------------------------------------------+
er_fn jit_emit_ror32
    mov     al, 0xD3
    call    jit_emit_byte
    mov     al, 0xC8         ; ror eax, cl (ModRM: mod=11, reg=1, rm=0)
    call    jit_emit_modrm
    ret

; ------------------------------------------------------------------
; Emit: rol eax, cl  (32-bit rotate left)
; -----------------------------------------------------------------+
er_fn jit_emit_rol32
    mov     al, 0xD3
    call    jit_emit_byte
    mov     al, 0xC0         ; rol eax, cl (ModRM: mod=11, reg=0, rm=0)
    call    jit_emit_modrm
    ret

; ------------------------------------------------------------------
; Emit: cmp eax, ecx  (32-bit compare)
; -----------------------------------------------------------------+
er_fn jit_emit_cmp32
    mov     al, 0x39
    call    jit_emit_byte
    mov     al, 0xC8         ; ModRM: mod=11, reg=1(ecx), rm=0(eax) → cmp eax, ecx
    call    jit_emit_modrm
    ret

; ------------------------------------------------------------------
; Emit: cmp eax, ecx  (64-bit)
; -----------------------------------------------------------------+
er_fn jit_emit_cmp64
    mov     cl, 1            ; REX.W
    mov     ch, 0
    mov     r8b, 0
    mov     r9b, 0
    call    jit_emit_rex
    mov     al, 0x39
    call    jit_emit_byte
    mov     al, 0xC8         ; ModRM: reg=1(rcx), rm=0(rax) → cmp rax, rcx
    call    jit_emit_modrm
    ret

; ------------------------------------------------------------------
; Emit: setcc al — convert condition to setcc opcode
; cl = condition code (0=o, 1=no, 2=b, 3=ae, 4=e, 5=ne, 6=be, 7=a,
;                      8=s, 9=ns, a=p, b=np, c=l, d=ge, e=le, f=g)
; -----------------------------------------------------------------+
er_fn jit_emit_setcc
    mov     al, 0x0F
    call    jit_emit_byte
    mov     al, 0x90         ; setcc base opcode
    or      al, cl
    call    jit_emit_byte
    mov     al, 0xC0         ; ModRM: mod=11, rm=0 (al)
    call    jit_emit_modrm
    ret

; ------------------------------------------------------------------
; Emit: movzx eax, al  — zero-extend byte to dword
; -----------------------------------------------------------------+
er_fn jit_emit_movzx_al_eax
    mov     al, 0x0F
    call    jit_emit_byte
    mov     al, 0xB6
    call    jit_emit_byte
    mov     al, 0xC0         ; movzx eax, al
    call    jit_emit_modrm
    ret

; ------------------------------------------------------------------
; Emit: cdqe  — sign-extend eax to rax
; -----------------------------------------------------------------+
er_fn jit_emit_cdqe
    mov     al, 0x48         ; REX.W
    call    jit_emit_byte
    mov     al, 0x63
    call    jit_emit_byte
    ret

; ------------------------------------------------------------------
; Emit: 64-bit add rax, [r15 + offset]  — add global to rax
; eax = offset in JitGlobals
; -----------------------------------------------------------------+
er_fn jit_emit_add_global
    push    rax
    mov     cl, 1            ; REX.W
    mov     ch, 0
    mov     r8b, 0
    mov     r9b, 1           ; B=1 (r15 base)
    call    jit_emit_rex
    mov     al, 0x03         ; add rax, r/m64
    call    jit_emit_byte
    pop     rcx              ; offset in ecx
    push    rcx
    and     ecx, 7           ; just need low bits for ModRM.rm
    ; ModRM: mod=10, reg=0 (rax), rm=15 (r15 with B)
    ; BUT rm=15 means [r15+disp32] with REX.B, so ModRM.rm=15
    mov     edi, 2           ; mod=10
    mov     ecx, 0           ; reg=0 (rax)
    mov     edx, 15          ; rm=15
    call    jit_build_modrm
    call    jit_emit_modrm
    pop     rax
    call    jit_emit_dword
    ret

; ------------------------------------------------------------------
; Emit REX.W prefix (no R, X, B bits set) — short for most 64-bit ops
; -----------------------------------------------------------------+
er_fn jit_emit_rex_nob
    mov     al, 0x48         ; REX.W
    call    jit_emit_byte
    ret

; ------------------------------------------------------------------
; Reserve space on native stack (sub rsp, N)
; eax = size (must be 16-byte aligned)
; -----------------------------------------------------------------+
er_fn jit_emit_sub_rsp
    push    rax
    cmp     eax, 128
    ja      .sub_large
    ; sub rsp, imm8
    mov     al, 0x48         ; REX.W
    call    jit_emit_byte
    mov     al, 0x83         ; sub r/m64, imm8
    call    jit_emit_byte
    mov     al, 0xEC         ; ModRM: mod=11, reg=5 (sub), rm=4 (rsp)
    call    jit_emit_modrm
    pop     rax
    call    jit_emit_byte    ; imm8
    ret
.sub_large:
    ; sub rsp, imm32
    mov     al, 0x48         ; REX.W
    call    jit_emit_byte
    mov     al, 0x81         ; sub r/m64, imm32
    call    jit_emit_byte
    mov     al, 0xEC         ; ModRM: mod=11, reg=5, rm=4
    call    jit_emit_modrm
    pop     rax
    call    jit_emit_dword   ; imm32
    ret

; ------------------------------------------------------------------
; Emit: ret
; -----------------------------------------------------------------+
er_fn jit_emit_ret
    mov     al, 0xC3
    call    jit_emit_byte
    ret

; ------------------------------------------------------------------
; Emit push reg (cl = register 0-15)
; -----------------------------------------------------------------+
er_fn jit_emit_push_reg
    test    cl, 8
    jz      .push_no_rex
    push    rax
    mov     al, 0x41
    call    jit_emit_byte
    pop     rax
    mov     al, 0x50
    and     cl, 7
    or      al, cl
    call    jit_emit_byte
    ret
.push_no_rex:
    mov     al, 0x50
    or      al, cl
    call    jit_emit_byte
    ret

er_fn jit_emit_pop_reg
    test    cl, 8
    jz      .pop_no_rex
    push    rax
    mov     al, 0x41
    call    jit_emit_byte
    pop     rax
    mov     al, 0x58
    and     cl, 7
    or      al, cl
    call    jit_emit_byte
    ret
.pop_no_rex:
    mov     al, 0x58
    or      al, cl
    call    jit_emit_byte
    ret

; ------------------------------------------------------------------
; Emit: mov [rsp + disp32], reg64  (store to spill slot)
; cl = register, eax = displacement
; -----------------------------------------------------------------+
er_fn jit_emit_store_spill
    er_push rax, rcx
    ; Determine REX
    mov     ch, 0
    test    cl, 8
    jz      .ss_no_r
    mov     ch, 1            ; REX.R
.ss_no_r:
    mov     r8b, 0
    mov     r9b, 0
    mov     cl, 1            ; REX.W
    call    jit_emit_rex
    mov     al, 0x89         ; mov r/m64, r64
    call    jit_emit_byte
    pop     rcx
    push    rcx
    and     ecx, 7
    ; ModRM: mod=10 ([rsp+disp32]), reg=src, rm=4 (SIB follows)
    mov     edi, 2           ; mod=10
    mov     edx, 4           ; rm=4 (SIB)
    call    jit_build_modrm
    call    jit_emit_modrm
    ; SIB: scale=0, index=4 (none), base=4 (rsp)
    mov     al, 0x24
    call    jit_emit_sib
    er_pop  rax, rcx
    call    jit_emit_dword   ; displacement
    ret

; ------------------------------------------------------------------
; Emit: mov reg64, [rsp + disp32]  (load from spill slot)
; cl = register, eax = displacement
; -----------------------------------------------------------------+
er_fn jit_emit_load_spill
    er_push rax, rcx
    mov     ch, 0
    test    cl, 8
    jz      .ls_no_r
    mov     ch, 1
.ls_no_r:
    mov     r8b, 0
    mov     r9b, 0
    mov     cl, 1            ; REX.W
    call    jit_emit_rex
    mov     al, 0x8B         ; mov r64, r/m64
    call    jit_emit_byte
    pop     rcx
    push    rcx
    and     ecx, 7
    mov     edi, 2           ; mod=10
    mov     edx, 4           ; rm=4 (SIB)
    call    jit_build_modrm
    call    jit_emit_modrm
    mov     al, 0x24
    call    jit_emit_sib
    er_pop  rax, rcx
    call    jit_emit_dword
    ret

; ------------------------------------------------------------------
; Emit: jmp rel32 (unconditional jump, 5 bytes)
; eax = relative displacement
; -----------------------------------------------------------------+
er_fn jit_emit_jmp_rel32
    push    rax
    mov     al, 0xE9         ; jmp rel32
    call    jit_emit_byte
    pop     rax
    call    jit_emit_dword
    ret

; ------------------------------------------------------------------
; Emit: jcc rel32 (conditional jump, 6 bytes)
; cl = condition code, eax = relative displacement
; -----------------------------------------------------------------+
er_fn jit_emit_jcc_rel32
    er_push rax, rcx
    mov     al, 0x0F         ; two-byte opcode prefix
    call    jit_emit_byte
    mov     al, 0x80         ; jcc base + condition
    or      al, cl
    call    jit_emit_byte
    er_pop  rax, rcx
    call    jit_emit_dword
    ret

; ------------------------------------------------------------------
; Emit: mov rax, [memory_base + rcx*1]  — load from linear memory
; memory base is at JitGlobals.mem_ptr
; Assumes address check already done.
; -----------------------------------------------------------------+
er_fn jit_emit_mem_load32
    ; Load memory base into rdx
    mov     cl, 2            ; reg = rdx
    mov     eax, JitGlobals.mem_ptr
    call    jit_emit_load_global_to_reg
    ; mov eax, [rdx + rcx]  (32-bit load)
    mov     al, 0x8B         ; mov r32, r/m32
    call    jit_emit_byte
    ; ModRM: mod=00, reg=0 (eax), rm=4 (SIB)
    mov     al, 0x04
    call    jit_emit_modrm
    ; SIB: scale=0, index=1 (rcx), base=2 (rdx)
    mov     al, 0x0A
    call    jit_emit_sib
    ret

; ------------------------------------------------------------------
; Emit: mov [memory_base + rcx*1], eax  — store to linear memory
; -----------------------------------------------------------------+
er_fn jit_emit_mem_store32
    mov     cl, 2            ; rdx
    mov     eax, JitGlobals.mem_ptr
    call    jit_emit_load_global_to_reg
    mov     al, 0x89         ; mov r/m32, r32
    call    jit_emit_byte
    mov     al, 0x04         ; ModRM SIB
    call    jit_emit_modrm
    mov     al, 0x0A         ; SIB: [rdx + rcx]
    call    jit_emit_sib
    ret

; ------------------------------------------------------------------
; Emit 64-bit: add rax, rcx
; -----------------------------------------------------------------+
er_fn jit_emit_add64
    mov     cl, 1
    mov     ch, 0
    mov     r8b, 0
    mov     r9b, 0
    call    jit_emit_rex
    mov     al, 0x01
    call    jit_emit_byte
    mov     al, 0xC1
    call    jit_emit_modrm
    ret

; ------------------------------------------------------------------
; Emit 64-bit: sub rax, rcx
; -----------------------------------------------------------------+
er_fn jit_emit_sub64
    mov     cl, 1
    mov     ch, 0
    mov     r8b, 0
    mov     r9b, 0
    call    jit_emit_rex
    mov     al, 0x29
    call    jit_emit_byte
    mov     al, 0xC1
    call    jit_emit_modrm
    ret

; ------------------------------------------------------------------
; Emit: mul ecx  (unsigned multiply eax *= ecx, edx:eax = result)
; -----------------------------------------------------------------+
er_fn jit_emit_mul32
    mov     al, 0xF7
    call    jit_emit_byte
    mov     al, 0xE1         ; mul ecx
    call    jit_emit_modrm
    ret

; ------------------------------------------------------------------
; Emit: imul ecx  (signed multiply eax *= ecx)
; -----------------------------------------------------------------+
er_fn jit_emit_imul32_single
    mov     al, 0xF7
    call    jit_emit_byte
    mov     al, 0xE9         ; imul ecx
    call    jit_emit_modrm
    ret

; ------------------------------------------------------------------
; Emit: div ecx  (unsigned divide edx:eax / ecx, eax=quot, edx=rem)
; -----------------------------------------------------------------+
er_fn jit_emit_div32
    mov     al, 0xF7
    call    jit_emit_byte
    mov     al, 0xF1         ; div ecx
    call    jit_emit_modrm
    ret

; ------------------------------------------------------------------
; Emit: idiv ecx  (signed divide)
; -----------------------------------------------------------------+
er_fn jit_emit_idiv32
    mov     al, 0xF7
    call    jit_emit_byte
    mov     al, 0xF9         ; idiv ecx
    call    jit_emit_modrm
    ret

; ------------------------------------------------------------------
; Emit: xor eax, eax  (zero register)
; -----------------------------------------------------------------+
er_fn jit_emit_xor_eax_eax
    mov     al, 0x31
    call    jit_emit_byte
    mov     al, 0xC0
    call    jit_emit_modrm
    ret

; ------------------------------------------------------------------
; Emit: lzcnt eax, ecx  (leading zero count)
; -----------------------------------------------------------------+
er_fn jit_emit_lzcnt32
    mov     al, 0xF3         ; F3 prefix for lzcnt
    call    jit_emit_byte
    mov     al, 0x0F
    call    jit_emit_byte
    mov     al, 0xBD
    call    jit_emit_byte
    mov     al, 0xC1         ; lzcnt eax, ecx
    call    jit_emit_modrm
    ret

; ------------------------------------------------------------------
; Emit: tzcnt eax, ecx  (trailing zero count)
; -----------------------------------------------------------------+
er_fn jit_emit_tzcnt32
    mov     al, 0xF3
    call    jit_emit_byte
    mov     al, 0x0F
    call    jit_emit_byte
    mov     al, 0xBC
    call    jit_emit_byte
    mov     al, 0xC1
    call    jit_emit_modrm
    ret

; ------------------------------------------------------------------
; Emit: popcnt eax, ecx  (population count)
; -----------------------------------------------------------------+
er_fn jit_emit_popcnt32
    mov     al, 0xF3
    call    jit_emit_byte
    mov     al, 0x0F
    call    jit_emit_byte
    mov     al, 0xB8
    call    jit_emit_byte
    mov     al, 0xC1
    call    jit_emit_modrm
    ret

; ------------------------------------------------------------------
; Emit: movsxd rax, ecx  (sign-extend dword to qword)
; -----------------------------------------------------------------+
er_fn jit_emit_movsxd
    mov     al, 0x48         ; REX.W
    call    jit_emit_byte
    mov     al, 0x63
    call    jit_emit_byte
    mov     al, 0xC1         ; movsxd rax, ecx
    call    jit_emit_modrm
    ret

; ------------------------------------------------------------------
; Emit: mov rax, [memory_base + rcx]  — 64-bit load from linear memory
; -----------------------------------------------------------------+
er_fn jit_emit_mem_load64
    mov     cl, 2            ; rdx
    mov     eax, JitGlobals.mem_ptr
    call    jit_emit_load_global_to_reg
    mov     al, 0x48         ; REX.W
    call    jit_emit_byte
    mov     al, 0x8B         ; mov r64, r/m64
    call    jit_emit_byte
    mov     al, 0x04         ; ModRM SIB
    call    jit_emit_modrm
    mov     al, 0x0A         ; SIB: [rdx + rcx]
    call    jit_emit_sib
    ret

; ------------------------------------------------------------------
; Emit: mov [memory_base + rcx], rax  — 64-bit store to linear memory
; -----------------------------------------------------------------+
er_fn jit_emit_mem_store64
    mov     cl, 2            ; rdx
    mov     eax, JitGlobals.mem_ptr
    call    jit_emit_load_global_to_reg
    mov     al, 0x48         ; REX.W
    call    jit_emit_byte
    mov     al, 0x89         ; mov r/m64, r64
    call    jit_emit_byte
    mov     al, 0x04         ; ModRM SIB
    call    jit_emit_modrm
    mov     al, 0x0A         ; SIB: [rdx + rcx]
    call    jit_emit_sib
    ret

; ------------------------------------------------------------------
; Narrow memory load/store helpers — SIB addressing via [rdx + rcx]
; Input: rcx = byte offset, rdx loaded internally with mem_ptr
; -----------------------------------------------------------------+

; Emit: movsx eax, byte [memory_base + rcx]  (0F BE 04 0A)
er_fn jit_emit_mem_load8_s
    mov     cl, 2
    mov     eax, JitGlobals.mem_ptr
    call    jit_emit_load_global_to_reg
    mov     al, 0x0F
    call    jit_emit_byte
    mov     al, 0xBE
    call    jit_emit_byte
    mov     al, 0x04
    call    jit_emit_modrm
    mov     al, 0x0A
    jmp     jit_emit_sib

; Emit: movzx eax, byte [memory_base + rcx]  (0F B6 04 0A)
er_fn jit_emit_mem_load8_u
    mov     cl, 2
    mov     eax, JitGlobals.mem_ptr
    call    jit_emit_load_global_to_reg
    mov     al, 0x0F
    call    jit_emit_byte
    mov     al, 0xB6
    call    jit_emit_byte
    mov     al, 0x04
    call    jit_emit_modrm
    mov     al, 0x0A
    jmp     jit_emit_sib

; Emit: movsx eax, word [memory_base + rcx]  (0F BF 04 0A)
er_fn jit_emit_mem_load16_s
    mov     cl, 2
    mov     eax, JitGlobals.mem_ptr
    call    jit_emit_load_global_to_reg
    mov     al, 0x0F
    call    jit_emit_byte
    mov     al, 0xBF
    call    jit_emit_byte
    mov     al, 0x04
    call    jit_emit_modrm
    mov     al, 0x0A
    jmp     jit_emit_sib

; Emit: movzx eax, word [memory_base + rcx]  (0F B7 04 0A)
er_fn jit_emit_mem_load16_u
    mov     cl, 2
    mov     eax, JitGlobals.mem_ptr
    call    jit_emit_load_global_to_reg
    mov     al, 0x0F
    call    jit_emit_byte
    mov     al, 0xB7
    call    jit_emit_byte
    mov     al, 0x04
    call    jit_emit_modrm
    mov     al, 0x0A
    jmp     jit_emit_sib

; Emit: REX.W movsx rax, byte [memory_base + rcx]  (48 0F BE 04 0A)
er_fn jit_emit_mem_load8_s_64
    mov     cl, 2
    mov     eax, JitGlobals.mem_ptr
    call    jit_emit_load_global_to_reg
    mov     al, 0x48
    call    jit_emit_byte
    mov     al, 0x0F
    call    jit_emit_byte
    mov     al, 0xBE
    call    jit_emit_byte
    mov     al, 0x04
    call    jit_emit_modrm
    mov     al, 0x0A
    jmp     jit_emit_sib

; Emit: REX.W movsx rax, word [memory_base + rcx]  (48 0F BF 04 0A)
er_fn jit_emit_mem_load16_s_64
    mov     cl, 2
    mov     eax, JitGlobals.mem_ptr
    call    jit_emit_load_global_to_reg
    mov     al, 0x48
    call    jit_emit_byte
    mov     al, 0x0F
    call    jit_emit_byte
    mov     al, 0xBF
    call    jit_emit_byte
    mov     al, 0x04
    call    jit_emit_modrm
    mov     al, 0x0A
    jmp     jit_emit_sib

; Emit: movsxd rax, dword [memory_base + rcx]  (48 63 04 0A)
er_fn jit_emit_mem_load32_s
    mov     cl, 2
    mov     eax, JitGlobals.mem_ptr
    call    jit_emit_load_global_to_reg
    mov     al, 0x48
    call    jit_emit_byte
    mov     al, 0x63
    call    jit_emit_byte
    mov     al, 0x04
    call    jit_emit_modrm
    mov     al, 0x0A
    jmp     jit_emit_sib

; Emit: mov [memory_base + rcx], al  (88 04 0A)
er_fn jit_emit_mem_store8
    mov     cl, 2
    mov     eax, JitGlobals.mem_ptr
    call    jit_emit_load_global_to_reg
    mov     al, 0x88
    call    jit_emit_byte
    mov     al, 0x04
    call    jit_emit_modrm
    mov     al, 0x0A
    jmp     jit_emit_sib

; Emit: mov [memory_base + rcx], ax  (66 89 04 0A)
er_fn jit_emit_mem_store16
    mov     cl, 2
    mov     eax, JitGlobals.mem_ptr
    call    jit_emit_load_global_to_reg
    mov     al, 0x66
    call    jit_emit_byte
    mov     al, 0x89
    call    jit_emit_byte
    mov     al, 0x04
    call    jit_emit_modrm
    mov     al, 0x0A
    jmp     jit_emit_sib

; ------------------------------------------------------------------
; Emit: mov rsi, rsp  (48 8B F4)
; -----------------------------------------------------------------+
er_fn jit_emit_mov_rsi_rsp
    mov     al, 0x48         ; REX.W
    call    jit_emit_byte
    mov     al, 0x8B         ; mov r64, r/m64
    call    jit_emit_byte
    mov     al, 0xF4         ; ModRM: mod=11, reg=6(rsi), rm=4(rsp)
    call    jit_emit_modrm
    ret

; ------------------------------------------------------------------
; Emit: mov rdi, imm64  (48 BF <8 bytes>)
; rax = imm64
; -----------------------------------------------------------------+
er_fn jit_emit_mov_rdi_imm64
    push    rax
    mov     al, 0x48         ; REX.W
    call    jit_emit_byte
    mov     al, 0xBF         ; mov rdi, imm64
    call    jit_emit_byte
    pop     rax
    call    jit_emit_qword
    ret

; ------------------------------------------------------------------
; Emit: mov rax, imm64  (48 B8 <8 bytes>)
; rax = imm64
; -----------------------------------------------------------------+
er_fn jit_emit_mov_rax_imm64
    push    rax
    mov     al, 0x48         ; REX.W
    call    jit_emit_byte
    mov     al, 0xB8         ; mov rax, imm64
    call    jit_emit_byte
    pop     rax
    call    jit_emit_qword
    ret

; ------------------------------------------------------------------
; Emit: mov edx, imm32  (BA <4 bytes>)
; eax = imm32
; -----------------------------------------------------------------+
er_fn jit_emit_mov_edx_imm32
    push    rax
    mov     al, 0xBA         ; mov edx, imm32
    call    jit_emit_byte
    pop     rax
    call    jit_emit_dword
    ret

; ------------------------------------------------------------------
; Emit: call rax  (FF D0)
; -----------------------------------------------------------------+
er_fn jit_emit_call_rax
    mov     al, 0xFF
    call    jit_emit_byte
    mov     al, 0xD0         ; ModRM: mod=11, reg=2(call), rm=0(rax)
    call    jit_emit_modrm
    ret

; ------------------------------------------------------------------
; Emit: sub rsp, imm32  (48 81 EC / 48 83 EC)
; eax = unsigned value
; -----------------------------------------------------------------+
er_fn jit_emit_sub_rsp_imm
    push    rax
    mov     al, 0x48         ; REX.W
    call    jit_emit_byte
    pop     rax
    cmp     eax, 128
    jb      .sub_small
    push    rax
    mov     al, 0x81
    call    jit_emit_byte
    mov     al, 0xEC         ; ModRM: mod=11, reg=5(sub), rm=4(rsp)
    call    jit_emit_modrm
    pop     rax
    call    jit_emit_dword
    ret
.sub_small:
    push    rax
    mov     al, 0x83
    call    jit_emit_byte
    mov     al, 0xEC
    call    jit_emit_modrm
    pop     rax
    call    jit_emit_byte
    ret

; ------------------------------------------------------------------
; Emit: add rsp, imm32  (48 81 C4 / 48 83 C4)
; eax = unsigned value
; -----------------------------------------------------------------+
er_fn jit_emit_add_rsp_imm
    push    rax
    mov     al, 0x48         ; REX.W
    call    jit_emit_byte
    pop     rax
    cmp     eax, 128
    jb      .add_small
    push    rax
    mov     al, 0x81
    call    jit_emit_byte
    mov     al, 0xC4         ; ModRM: mod=11, reg=0(add), rm=4(rsp)
    call    jit_emit_modrm
    pop     rax
    call    jit_emit_dword
    ret
.add_small:
    push    rax
    mov     al, 0x83
    call    jit_emit_byte
    mov     al, 0xC4
    call    jit_emit_modrm
    pop     rax
    call    jit_emit_byte
    ret

; ------------------------------------------------------------------
; Emit: mov rax, [rsp + disp]  (48 8B 44 24 <disp8> or 48 8B 84 24 <disp32>)
; eax = unsigned displacement
; -----------------------------------------------------------------+
er_fn jit_emit_load_rax_rsp_disp
    push    rax
    mov     al, 0x48         ; REX.W
    call    jit_emit_byte
    mov     al, 0x8B         ; mov r64, r/m64
    call    jit_emit_byte
    pop     rax
    cmp     eax, 128
    jb      .ld_small
    ; disp32 form: ModRM 84 with SIB 24
    push    rax
    mov     al, 0x84         ; mod=10, reg=0, rm=4(SIB)
    call    jit_emit_modrm
    mov     al, 0x24         ; SIB: [rsp]
    call    jit_emit_sib
    pop     rax
    call    jit_emit_dword
    ret
.ld_small:
    ; disp8 form: ModRM 44 with SIB 24
    push    rax
    mov     al, 0x44         ; mod=01, reg=0, rm=4(SIB)
    call    jit_emit_modrm
    mov     al, 0x24         ; SIB: [rsp]
    call    jit_emit_sib
    pop     rax
    call    jit_emit_byte
    ret

; ------------------------------------------------------------------
; Emit: mov [rsp + disp], rax  (48 89 44 24 <disp8> or 48 89 84 24 <disp32>)
; eax = unsigned displacement
; -----------------------------------------------------------------+
er_fn jit_emit_store_rax_rsp_disp
    push    rax
    mov     al, 0x48         ; REX.W
    call    jit_emit_byte
    mov     al, 0x89         ; mov r/m64, r64
    call    jit_emit_byte
    pop     rax
    cmp     eax, 128
    jb      .st_small
    push    rax
    mov     al, 0x84         ; mod=10, reg=0, rm=4(SIB)
    call    jit_emit_modrm
    mov     al, 0x24         ; SIB: [rsp]
    call    jit_emit_sib
    pop     rax
    call    jit_emit_dword
    ret
.st_small:
    push    rax
    mov     al, 0x44         ; mod=01, reg=0, rm=4(SIB)
    call    jit_emit_modrm
    mov     al, 0x24         ; SIB: [rsp]
    call    jit_emit_sib
    pop     rax
    call    jit_emit_byte
    ret

; ------------------------------------------------------------------
; Emit: test rax, rax  (64-bit)
; -----------------------------------------------------------------+
er_fn jit_emit_test64
    mov     al, 0x48         ; REX.W
    call    jit_emit_byte
    mov     al, 0x85
    call    jit_emit_byte
    mov     al, 0xC0         ; test rax, rax
    call    jit_emit_modrm
    ret

; ------------------------------------------------------------------
; Emit: lzcnt rax, rax  (64-bit leading zero count)
; -----------------------------------------------------------------+
er_fn jit_emit_lzcnt64
    mov     al, 0x48         ; REX.W
    call    jit_emit_byte
    mov     al, 0xF3
    call    jit_emit_byte
    mov     al, 0x0F
    call    jit_emit_byte
    mov     al, 0xBD
    call    jit_emit_byte
    mov     al, 0xC0         ; lzcnt rax, rax
    call    jit_emit_modrm
    ret

; ------------------------------------------------------------------
; Emit: tzcnt rax, rax  (64-bit trailing zero count)
; -----------------------------------------------------------------+
er_fn jit_emit_tzcnt64
    mov     al, 0x48         ; REX.W
    call    jit_emit_byte
    mov     al, 0xF3
    call    jit_emit_byte
    mov     al, 0x0F
    call    jit_emit_byte
    mov     al, 0xBC
    call    jit_emit_byte
    mov     al, 0xC0         ; tzcnt rax, rax
    call    jit_emit_modrm
    ret

; ------------------------------------------------------------------
; Emit: popcnt rax, rax  (64-bit population count)
; -----------------------------------------------------------------+
er_fn jit_emit_popcnt64
    mov     al, 0x48         ; REX.W
    call    jit_emit_byte
    mov     al, 0xF3
    call    jit_emit_byte
    mov     al, 0x0F
    call    jit_emit_byte
    mov     al, 0xB8
    call    jit_emit_byte
    mov     al, 0xC0         ; popcnt rax, rax
    call    jit_emit_modrm
    ret

; ------------------------------------------------------------------
; Emit: cqo (sign-extend rax to rdx:rax, REX.W + 99)
; -----------------------------------------------------------------+
er_fn jit_emit_cqo
    mov     al, 0x48         ; REX.W
    call    jit_emit_byte
    mov     al, 0x99         ; cqo
    call    jit_emit_byte
    ret

; ------------------------------------------------------------------
; Emit: 64-bit: xor edx, edx (zero-extends to rdx)
; -----------------------------------------------------------------+
er_fn jit_emit_xor_edx_edx
    mov     al, 0x31
    call    jit_emit_byte
    mov     al, 0xD2         ; xor edx, edx
    call    jit_emit_modrm
    ret

; ==================================================================
; SSE/Float instruction emitters for JIT templates
; =================================================================+
; Each function emits a complete SSE instruction into the code cache.
; Calling convention matches the rest of the JIT emitter helpers.

; ------------------------------------------------------------------
; Generic SSE opcode emitter: <prefix> 0F <opcode> <modrm>
; al = prefix (0xF2=sd, 0xF3=ss, 0x66=packed/pd), ch = opcode, cl = modrm
; -----------------------------------------------------------------+
er_fn jit_emit_sse_op
    call    jit_emit_byte
    mov     al, 0x0F
    call    jit_emit_byte
    mov     al, ch
    call    jit_emit_byte
    mov     al, cl
    jmp     jit_emit_modrm

; ------------------------------------------------------------------
; Generic SSE opcode with 64-bit REX.W: <prefix> 48 0F <opcode> <modrm>
; al = prefix, ch = opcode, cl = modrm
; -----------------------------------------------------------------+
er_fn jit_emit_sse64_op
    call    jit_emit_byte
    mov     al, 0x48         ; REX.W
    call    jit_emit_byte
    mov     al, 0x0F
    call    jit_emit_byte
    mov     al, ch
    call    jit_emit_byte
    mov     al, cl
    jmp     jit_emit_modrm

; ------------------------------------------------------------------
; SSE4.1 three-byte opcode: 66 0F 3A <opcode> <modrm> <imm8>
; al = opcode byte, cl = modrm, ch = imm8
; -----------------------------------------------------------------+
er_fn jit_emit_sse3a_op
    push    rax
    mov     al, 0x66
    call    jit_emit_byte
    mov     al, 0x0F
    call    jit_emit_byte
    mov     al, 0x3A
    call    jit_emit_byte
    pop     rax
    call    jit_emit_byte
    mov     al, cl
    call    jit_emit_modrm
    mov     al, ch
    jmp     jit_emit_byte

; ------------------------------------------------------------------
; movd xmm0, eax  (66 0F 6E C0)
; -----------------------------------------------------------------+
er_fn jit_emit_movd_xmm0_eax
    mov     al, 0x66
    call    jit_emit_byte
    mov     al, 0x0F
    call    jit_emit_byte
    mov     al, 0x6E
    call    jit_emit_byte
    mov     al, 0xC0
    jmp     jit_emit_modrm

; ------------------------------------------------------------------
; movd xmm1, ecx  (66 0F 6E C9)
; -----------------------------------------------------------------+
er_fn jit_emit_movd_xmm1_ecx
    mov     al, 0x66
    call    jit_emit_byte
    mov     al, 0x0F
    call    jit_emit_byte
    mov     al, 0x6E
    call    jit_emit_byte
    mov     al, 0xC9
    jmp     jit_emit_modrm

; ------------------------------------------------------------------
; movd eax, xmm0  (66 0F 7E C0)
; -----------------------------------------------------------------+
er_fn jit_emit_movd_eax_xmm0
    mov     al, 0x66
    call    jit_emit_byte
    mov     al, 0x0F
    call    jit_emit_byte
    mov     al, 0x7E
    call    jit_emit_byte
    mov     al, 0xC0
    jmp     jit_emit_modrm

; ------------------------------------------------------------------
; movq xmm0, rax  (66 48 0F 6E C0)
; -----------------------------------------------------------------+
er_fn jit_emit_movq_xmm0_rax
    mov     al, 0x66
    call    jit_emit_byte
    mov     al, 0x48         ; REX.W
    call    jit_emit_byte
    mov     al, 0x0F
    call    jit_emit_byte
    mov     al, 0x6E
    call    jit_emit_byte
    mov     al, 0xC0
    jmp     jit_emit_modrm

; ------------------------------------------------------------------
; movq xmm1, rcx  (66 48 0F 6E C9)
; -----------------------------------------------------------------+
er_fn jit_emit_movq_xmm1_rcx
    mov     al, 0x66
    call    jit_emit_byte
    mov     al, 0x48         ; REX.W
    call    jit_emit_byte
    mov     al, 0x0F
    call    jit_emit_byte
    mov     al, 0x6E
    call    jit_emit_byte
    mov     al, 0xC9
    jmp     jit_emit_modrm

; ------------------------------------------------------------------
; movq rax, xmm0  (66 48 0F 7E C0)
; -----------------------------------------------------------------+
er_fn jit_emit_movq_rax_xmm0
    mov     al, 0x66
    call    jit_emit_byte
    mov     al, 0x48         ; REX.W
    call    jit_emit_byte
    mov     al, 0x0F
    call    jit_emit_byte
    mov     al, 0x7E
    call    jit_emit_byte
    mov     al, 0xC0
    jmp     jit_emit_modrm

; ------------------------------------------------------------------
; btr eax, imm8  (0F BA /5 ib) — bit test and reset (for f32 abs)
; cl = imm8 bit position
; -----------------------------------------------------------------+
er_fn jit_emit_btr_eax
    mov     al, 0x0F
    call    jit_emit_byte
    mov     al, 0xBA
    call    jit_emit_byte
    mov     al, 0xE8         ; ModRM: mod=11, reg=5(btr), rm=0(eax)
    call    jit_emit_modrm
    mov     al, cl
    jmp     jit_emit_byte

; ------------------------------------------------------------------
; btc eax, imm8  (0F BA /7 ib) — bit test and complement (for f32 neg)
; cl = imm8 bit position
; -----------------------------------------------------------------+
er_fn jit_emit_btc_eax
    mov     al, 0x0F
    call    jit_emit_byte
    mov     al, 0xBA
    call    jit_emit_byte
    mov     al, 0xF8         ; ModRM: mod=11, reg=7(btc), rm=0(eax)
    call    jit_emit_modrm
    mov     al, cl
    jmp     jit_emit_byte

; ------------------------------------------------------------------
; btr rax, imm8 with REX.W  (48 0F BA /5 ib) — f64 abs
; cl = imm8 bit position
; -----------------------------------------------------------------+
er_fn jit_emit_btr_rax
    mov     al, 0x48         ; REX.W
    call    jit_emit_byte
    mov     al, 0x0F
    call    jit_emit_byte
    mov     al, 0xBA
    call    jit_emit_byte
    mov     al, 0xE8         ; ModRM: mod=11, reg=5(btr), rm=0(rax)
    call    jit_emit_modrm
    mov     al, cl
    jmp     jit_emit_byte

; ------------------------------------------------------------------
; btc rax, imm8 with REX.W  (48 0F BA /7 ib) — f64 neg
; cl = imm8 bit position
; -----------------------------------------------------------------+
er_fn jit_emit_btc_rax
    mov     al, 0x48         ; REX.W
    call    jit_emit_byte
    mov     al, 0x0F
    call    jit_emit_byte
    mov     al, 0xBA
    call    jit_emit_byte
    mov     al, 0xF8         ; ModRM: mod=11, reg=7(btc), rm=0(rax)
    call    jit_emit_modrm
    mov     al, cl
    jmp     jit_emit_byte

; ------------------------------------------------------------------
; NaN fixup helpers for float comparisons
; After ucomiss/ucomisd, PF is set for unordered (NaN).
; -----------------------------------------------------------------+

; Emit: setnp al; mov ah, al  — save ~PF (ordered=1) to ah
er_fn jit_emit_setnp_save_ah
    mov     al, 0x0F
    call    jit_emit_byte
    mov     al, 0x9B
    call    jit_emit_byte
    mov     al, 0xC0
    call    jit_emit_modrm
    mov     al, 0x88
    call    jit_emit_byte
    mov     al, 0xC4
    jmp     jit_emit_modrm

; Emit: setp al; mov ah, al  — save PF (unordered=1) to ah
er_fn jit_emit_setp_save_ah
    mov     al, 0x0F
    call    jit_emit_byte
    mov     al, 0x9A
    call    jit_emit_byte
    mov     al, 0xC0
    call    jit_emit_modrm
    mov     al, 0x88
    call    jit_emit_byte
    mov     al, 0xC4
    jmp     jit_emit_modrm

; ------------------------------------------------------------------
; Emit: and al, ah  (20 E0)
; -----------------------------------------------------------------+
er_fn jit_emit_and_al_ah
    mov     al, 0x20
    call    jit_emit_byte
    mov     al, 0xE0
    jmp     jit_emit_modrm

; ------------------------------------------------------------------
; Emit: or al, ah  (08 E0)
; -----------------------------------------------------------------+
er_fn jit_emit_or_al_ah
    mov     al, 0x08
    call    jit_emit_byte
    mov     al, 0xE0
    jmp     jit_emit_modrm

; ------------------------------------------------------------------
; Emit: and eax, imm32  (25 <imm32>)
; eax = imm32 value
; -----------------------------------------------------------------+
er_fn jit_emit_and_eax_imm32
    push    rax
    mov     al, 0x25
    call    jit_emit_byte
    pop     rax
    jmp     jit_emit_dword

; ------------------------------------------------------------------
; Emit: and ecx, imm32  (81 E1 <imm32>)
; eax = imm32 value
; -----------------------------------------------------------------+
er_fn jit_emit_and_ecx_imm32
    push    rax
    mov     al, 0x81
    call    jit_emit_byte
    mov     al, 0xE1
    call    jit_emit_modrm
    pop     rax
    jmp     jit_emit_dword

; ------------------------------------------------------------------
; Emit: or ecx, imm32  (81 C9 <imm32>)
; eax = imm32 value
; -----------------------------------------------------------------+
er_fn jit_emit_or_ecx_imm32
    push    rax
    mov     al, 0x81
    call    jit_emit_byte
    mov     al, 0xC9
    call    jit_emit_modrm
    pop     rax
    jmp     jit_emit_dword

; ------------------------------------------------------------------
; Emit: or eax, ecx  (09 C8)
; -----------------------------------------------------------------+
er_fn jit_emit_or_eax_ecx
    mov     al, 0x09
    call    jit_emit_byte
    mov     al, 0xC8
    jmp     jit_emit_modrm

; ------------------------------------------------------------------
; Emit: sub eax, imm32  (2D <imm32>)
; eax = imm32 value
; -----------------------------------------------------------------+
er_fn jit_emit_sub_eax_imm32
    push    rax
    mov     al, 0x2D
    call    jit_emit_byte
    pop     rax
    jmp     jit_emit_dword

; ------------------------------------------------------------------
; Patch dword at absolute code cache address
; rdi = address to write, eax = dword value
; -----------------------------------------------------------------+
er_fn jit_emit_patch_dword
    mov     [rdi], eax
    ret
