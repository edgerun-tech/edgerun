; ==================================================================
; wasm_jit_templates.asm — JIT opcode emission templates
;
; Each template emits x86_64 instructions into jit_state.code_ptr
; to implement one WASM opcode in native code.
;
; Register model (JIT'd code at runtime):
;   r15  = JitGlobals (set by trampoline)
;   rax  = scratch
;   rcx  = scratch
;   rdx  = scratch
;   rsp  = WASM value stack (push/pop for all values)
;
; Template calling convention (at compile time):
;   rdi = DecodedOp pointer (offset+4, next_offset+4, opcode+1, pad+3, imm0+4, imm1+4)
;   All emitter functions called by templates clobber rax, rcx, rdx.
;   No callee-save needed — orchestrator saves/restores around each template call.
; =================================================================+

%macro jit_template_push_rax 0
    xor     ecx, ecx
    call    jit_emit_push_reg
%endm

%macro jit_template_push_rax_ret 0
    jit_template_push_rax
    ret
%endm

%macro jit_template_pop_rcx_rax 0
    mov     cl, 1
    call    jit_emit_pop_reg
    xor     ecx, ecx
    call    jit_emit_pop_reg
%endm

%macro jit_template_setcc_push_ret 0
    call    jit_emit_setcc
    call    jit_emit_movzx_al_eax
    jit_template_push_rax_ret
%endm

%macro jit_template_setcc_push 0
    call    jit_emit_setcc
    call    jit_emit_movzx_al_eax
    xor     ecx, ecx
    jmp     jit_emit_push_reg
%endm

%macro jit_template_setcc_and_push 0
    call    jit_emit_setcc
    call    jit_emit_and_al_ah
    call    jit_emit_movzx_al_eax
    xor     ecx, ecx
    jmp     jit_emit_push_reg
%endm

%macro jit_template_setcc_or_push 0
    call    jit_emit_setcc
    call    jit_emit_or_al_ah
    call    jit_emit_movzx_al_eax
    xor     ecx, ecx
    jmp     jit_emit_push_reg
%endm

%macro jit_template_f32_sse_bin_tail 1
    mov     al, 0xF3
    mov     ch, %1
    mov     cl, 0xC1
    call    jit_emit_sse_op
    call    jit_emit_movd_eax_xmm0
    xor     ecx, ecx
    jmp     jit_emit_push_reg
%endm

%macro jit_template_f64_sse_bin_tail 1
    mov     al, 0xF2
    mov     ch, %1
    mov     cl, 0xC1
    call    jit_emit_sse_op
    call    jit_emit_movq_rax_xmm0
    xor     ecx, ecx
    jmp     jit_emit_push_reg
%endm

%macro jit_template_f32_round 1
    xor     ecx, ecx
    call    jit_emit_pop_reg
    call    jit_emit_movd_xmm0_eax
    mov     al, 0x0A
    mov     cl, 0xC0
    mov     ch, %1
    call    jit_emit_sse3a_op
    call    jit_emit_movd_eax_xmm0
    xor     ecx, ecx
    jmp     jit_emit_push_reg
%endm

%macro jit_template_f64_round 1
    xor     ecx, ecx
    call    jit_emit_pop_reg
    call    jit_emit_movq_xmm0_rax
    mov     al, 0x0B
    mov     cl, 0xC0
    mov     ch, %1
    call    jit_emit_sse3a_op
    call    jit_emit_movq_rax_xmm0
    xor     ecx, ecx
    jmp     jit_emit_push_reg
%endm

%macro jit_template_modrm_c0_push_rax 0
    mov     al, 0xC0
    call    jit_emit_modrm
    xor     ecx, ecx
    jmp     jit_emit_push_reg
%endm

%macro jit_template_emit_mov_accum_rdx 0
    mov     al, 0x89
    call    jit_emit_byte
    mov     al, 0xD0
    call    jit_emit_modrm
%endm

%macro jit_template_modrm82_disp_r11 0
    mov     al, 0x82
    call    jit_emit_modrm
    mov     eax, r11d
    call    jit_emit_dword
%endm

%macro jit_template_patch_rel32 1
    mov     rax, [jit_state.code_ptr]
    sub     rax, %1
    sub     rax, 4
    mov     rdi, %1
    call    jit_emit_patch_dword
%endm

%macro jit_template_jns_rel32_placeholder 1
    mov     %1, [jit_state.code_ptr]
    add     %1, 2
    mov     cl, 0x09
    xor     eax, eax
    call    jit_emit_jcc_rel32
%endm

%macro jit_template_jmp_rel32_placeholder 1
    mov     %1, [jit_state.code_ptr]
    inc     %1
    mov     al, 0xE9
    call    jit_emit_byte
    xor     eax, eax
    call    jit_emit_dword
%endm

%macro jit_template_push_imm64_parts 0
    mov     eax, r10d
    mov     edx, r11d
    shl     rdx, 32
    or      rax, rdx
    mov     cl, 1
    mov     ch, 0
    xor     r8b, r8b
    xor     r9b, r9b
    call    jit_emit_rex
    mov     al, 0xB8
    call    jit_emit_byte
    mov     eax, r10d
    mov     edx, r11d
    shl     rdx, 32
    or      rax, rdx
    call    jit_emit_qword
    jit_template_push_rax_ret
%endm

; ------------------------------------------------------------------
; Template: unreachable (0x00)
; Emit: ud2
; -----------------------------------------------------------------+
er_fn jit_template_unreachable
    mov     al, 0x0F
    call    jit_emit_byte
    mov     al, 0x0B         ; ud2
    call    jit_emit_byte
    ret

; ------------------------------------------------------------------
; Template: nop (0x01)
; -----------------------------------------------------------------+
er_fn jit_template_nop
    ret

; Control flow ops (0x02-0x0D, 0x0F) are handled directly by
; the orchestrator (wasm_jit.asm). call (0x10) and
; call_indirect (0x11) use their own templates below.

; ------------------------------------------------------------------
; Template: drop (0x1A) — discard TOS
; Emit: add rsp, 8
; -----------------------------------------------------------------+
er_fn jit_template_drop
    mov     al, 0x48         ; REX.W
    call    jit_emit_byte
    mov     al, 0x83         ; add r/m64, imm8
    call    jit_emit_byte
    mov     al, 0xC4         ; ModRM: mod=11, reg=0(add), rm=4(rsp)
    call    jit_emit_modrm
    mov     al, 8            ; imm8 — add rsp, 8
    call    jit_emit_byte
    ret

; ------------------------------------------------------------------
; Template: local.get index (0x20)
; Emit: mov rdx, [r15 + JitGlobals.locals]
;       mov rax, [rdx + index*8]
;       push rax
; -----------------------------------------------------------------+
er_fn jit_template_local_get
    mov     eax, [rdi + 12]  ; local index
    shl     eax, 3           ; *8
    mov     r11d, eax
    mov     cl, 2            ; rdx
    mov     eax, JitGlobals.locals
    call    jit_emit_load_global_to_reg
    ; mov rax, [rdx + disp32]
    mov     cl, 1            ; REX.W
    call    jit_emit_rex_nob
    mov     al, 0x8B         ; mov r64, r/m64
    call    jit_emit_byte
    jit_template_modrm82_disp_r11
    ; push rax
    jit_template_push_rax_ret

; ------------------------------------------------------------------
; Template: local.set index (0x21)
; Emit: pop rax; mov rdx, [r15 + JitGlobals.locals]; mov [rdx + index*8], rax
; -----------------------------------------------------------------+
er_fn jit_template_local_set
    mov     eax, [rdi + 12]  ; local index
    shl     eax, 3
    mov     r11d, eax
    ; pop rax
    xor     ecx, ecx         ; rax
    call    jit_emit_pop_reg
    ; load locals base into rdx
    push    rax              ; save value while we compute addr
    mov     cl, 2            ; rdx
    mov     eax, JitGlobals.locals
    call    jit_emit_load_global_to_reg
    ; mov [rdx + disp32], rax
    pop     rax              ; restore value
    push    rax
    mov     cl, 1            ; REX.W
    call    jit_emit_rex_nob
    mov     al, 0x89         ; mov r/m64, r64
    call    jit_emit_byte
    jit_template_modrm82_disp_r11
    pop     rax              ; consumed
    ret

; ------------------------------------------------------------------
; Template: local.tee index (0x22)
; Emit: same as local.set but keep value on stack
; -----------------------------------------------------------------+
er_fn jit_template_local_tee
    mov     eax, [rdi + 12]
    shl     eax, 3
    mov     r11d, eax
    ; read TOS without consuming it
    mov     cl, 1            ; REX.W
    call    jit_emit_rex_nob
    mov     al, 0x8B         ; mov rax, [rsp]
    call    jit_emit_byte
    mov     al, 0x04         ; ModRM SIB
    call    jit_emit_modrm
    mov     al, 0x24         ; SIB: [rsp]
    call    jit_emit_sib
    ; load locals base into rdx
    mov     cl, 2
    mov     eax, JitGlobals.locals
    call    jit_emit_load_global_to_reg
    mov     cl, 1
    call    jit_emit_rex_nob
    mov     al, 0x89
    call    jit_emit_byte
    jit_template_modrm82_disp_r11
    ret

; ------------------------------------------------------------------
; Template: global.get index (0x23)
; Emit: mov rdx, [r15 + JitGlobals.globals_buf]; mov rax, [rdx + index*32]; push rax
; -----------------------------------------------------------------+
er_fn jit_template_global_get
    mov     ecx, [rdi + 12]
    imul    ecx, 32          ; GLOBAL_SIZE = 32
    add     ecx, 8           ; value_data offset
    mov     r11d, ecx
    mov     cl, 2
    mov     eax, JitGlobals.globals_buf
    call    jit_emit_load_global_to_reg
    ; mov rax, [rdx + rcx] — just load value_data at offset 8 within Global struct
    ; Global layout: value_type(1) + mutable(1) + pad(6) + value_data(8) + value_tag(4) + pad(4) = 24, padded to 32
    ; value_data is at offset 8
    mov     cl, 1
    call    jit_emit_rex_nob
    mov     al, 0x8B
    call    jit_emit_byte
    jit_template_modrm82_disp_r11
    jit_template_push_rax_ret

; ------------------------------------------------------------------
; Template: global.set index (0x24)
; -----------------------------------------------------------------+
er_fn jit_template_global_set
    mov     ecx, [rdi + 12]
    imul    ecx, 32
    add     ecx, 8           ; value_data offset
    mov     r11d, ecx
    xor     ecx, ecx
    call    jit_emit_pop_reg
    push    rax
    mov     cl, 2
    mov     eax, JitGlobals.globals_buf
    call    jit_emit_load_global_to_reg
    pop     rax
    push    rax
    mov     cl, 1
    call    jit_emit_rex_nob
    mov     al, 0x89
    call    jit_emit_byte
    jit_template_modrm82_disp_r11
    pop     rax
    ret

; ------------------------------------------------------------------
; table.get (0x25) — pop index, push table[index]
; -----------------------------------------------------------------+
er_fn jit_template_table_get
    mov     eax, [rdi + 12]         ; imm0 = table index
    er_check_nonzero eax, .table_get_bad_index
    xor     ecx, ecx                ; rax
    call    jit_emit_pop_reg         ; rax = index
    mov     cl, 1                   ; rcx
    mov     eax, JitGlobals.table_entries
    call    jit_emit_load_global_to_reg  ; rcx = table_entries
    ; mov rax, [rcx + rax*8] — load table entry
    mov     al, 0x48
    call    jit_emit_byte
    mov     al, 0x8B
    call    jit_emit_byte
    mov     al, 0x04               ; ModRM: mod=00, reg=rax(0), rm=SIB
    call    jit_emit_modrm
    mov     al, 0xC1               ; SIB: scale=8, index=rax, base=rcx
    call    jit_emit_sib
    xor     ecx, ecx
    jmp     jit_emit_push_reg       ; push rax
.table_get_bad_index:
    mov     rdx, ERROR_NOT_IMPLEMENTED
    mov     [rel jit_template_error], rdx
    ret

; ------------------------------------------------------------------
; table.set (0x26) — pop value, pop index, table[index] = value
; -----------------------------------------------------------------+
er_fn jit_template_table_set
    mov     eax, [rdi + 12]         ; imm0 = table index
    er_check_nonzero eax, .table_set_bad_index
    mov     cl, 2                   ; rdx
    call    jit_emit_pop_reg         ; rdx = value
    xor     ecx, ecx                ; rax
    call    jit_emit_pop_reg         ; rax = index
    mov     cl, 1                   ; rcx
    mov     eax, JitGlobals.table_entries
    call    jit_emit_load_global_to_reg  ; rcx = table_entries
    ; mov [rcx + rax*8], rdx — store table entry
    mov     al, 0x48
    call    jit_emit_byte
    mov     al, 0x89
    call    jit_emit_byte
    mov     al, 0x14               ; ModRM: mod=00, reg=rdx(2), rm=SIB
    call    jit_emit_modrm
    mov     al, 0xC1               ; SIB: scale=8, index=rax, base=rcx
    call    jit_emit_sib
    ret
.table_set_bad_index:
    mov     rdx, ERROR_NOT_IMPLEMENTED
    mov     [rel jit_template_error], rdx
    ret

; ------------------------------------------------------------------
; Template: i32.const imm (0x41)
; Emit: push imm32
; -----------------------------------------------------------------+
er_fn jit_template_i32_const
    mov     eax, [rdi + 12]  ; imm0
    call    jit_emit_push_imm32
    ret

; ------------------------------------------------------------------
; Template: i64.const imm (0x42)
; Emit: push imm64 — uses mov rax, imm64; push rax
; -----------------------------------------------------------------+
er_fn jit_template_i64_const
    mov     r10d, [rdi + 12]  ; imm0 (low)
    mov     r11d, [rdi + 16]  ; imm1 (high)
    jit_template_push_imm64_parts

; ------------------------------------------------------------------
; Emit: push imm32 (5-byte push)
; eax = value
; -----------------------------------------------------------------+
er_fn jit_emit_push_imm32
    push    rax
    mov     al, 0x68         ; push imm32
    call    jit_emit_byte
    pop     rax
    call    jit_emit_dword
    ret

; ------------------------------------------------------------------
; i32 eqz (0x45)
; Emit: pop rax; test eax, eax; sete al; movzx eax, al; push rax
; -----------------------------------------------------------------+
er_fn jit_template_i32_eqz
    xor     ecx, ecx         ; pop rax
    call    jit_emit_pop_reg
    call    jit_emit_test32
    mov     cl, 0x94         ; sete
    call    jit_emit_setcc
    call    jit_emit_movzx_al_eax
    xor     ecx, ecx         ; push rax
    call    jit_emit_push_reg
    ret

; ------------------------------------------------------------------
; i32 eq (0x46)
; Emit: pop rcx; pop rax; cmp eax, ecx; sete al; movzx eax, al; push rax
; -----------------------------------------------------------------+
er_fn jit_template_i32_eq
    mov     cl, 1            ; pop rcx
    call    jit_emit_pop_reg
    xor     ecx, ecx         ; pop rax
    call    jit_emit_pop_reg
    call    jit_emit_cmp32
    mov     cl, 0x94         ; sete
    call    jit_emit_setcc
    call    jit_emit_movzx_al_eax
    xor     ecx, ecx         ; push rax
    call    jit_emit_push_reg
    ret

; ------------------------------------------------------------------
; i32 ne (0x47)
; -----------------------------------------------------------------+
er_fn jit_template_i32_ne
    jit_template_pop_rcx_rax
    call    jit_emit_cmp32
    mov     cl, 0x95         ; setne
    jit_template_setcc_push_ret

; ------------------------------------------------------------------
; i32 lt_s (0x48)
; -----------------------------------------------------------------+
er_fn jit_template_i32_lt_s
    jit_template_pop_rcx_rax
    call    jit_emit_cmp32
    mov     cl, 0x9C         ; setl
    jit_template_setcc_push_ret

; ------------------------------------------------------------------
; i32 lt_u (0x49)
; -----------------------------------------------------------------+
er_fn jit_template_i32_lt_u
    jit_template_pop_rcx_rax
    call    jit_emit_cmp32
    mov     cl, 0x92         ; setb
    jit_template_setcc_push_ret

; ------------------------------------------------------------------
; i32 gt_s (0x4A)
; -----------------------------------------------------------------+
er_fn jit_template_i32_gt_s
    jit_template_pop_rcx_rax
    call    jit_emit_cmp32
    mov     cl, 0x9F         ; setg
    jit_template_setcc_push_ret

; ------------------------------------------------------------------
; i32 gt_u (0x4B)
; -----------------------------------------------------------------+
er_fn jit_template_i32_gt_u
    jit_template_pop_rcx_rax
    call    jit_emit_cmp32
    mov     cl, 0x97         ; seta
    jit_template_setcc_push_ret

; ------------------------------------------------------------------
; i32 le_s (0x4C)
; -----------------------------------------------------------------+
er_fn jit_template_i32_le_s
    jit_template_pop_rcx_rax
    call    jit_emit_cmp32
    mov     cl, 0x9E         ; setle
    jit_template_setcc_push_ret

; ------------------------------------------------------------------
; i32 le_u (0x4D)
; -----------------------------------------------------------------+
er_fn jit_template_i32_le_u
    jit_template_pop_rcx_rax
    call    jit_emit_cmp32
    mov     cl, 0x96         ; setbe
    jit_template_setcc_push_ret

; ------------------------------------------------------------------
; i32 ge_s (0x4E)
; -----------------------------------------------------------------+
er_fn jit_template_i32_ge_s
    jit_template_pop_rcx_rax
    call    jit_emit_cmp32
    mov     cl, 0x9D         ; setge
    jit_template_setcc_push_ret

; ------------------------------------------------------------------
; i32 ge_u (0x4F)
; -----------------------------------------------------------------+
er_fn jit_template_i32_ge_u
    jit_template_pop_rcx_rax
    call    jit_emit_cmp32
    mov     cl, 0x93         ; setae
    jit_template_setcc_push_ret

; ------------------------------------------------------------------
; i32 clz (0x67)
; Emit: pop rax; lzcnt eax, eax; push rax
; -----------------------------------------------------------------+
er_fn jit_template_i32_clz
    xor     ecx, ecx
    call    jit_emit_pop_reg
    call    jit_emit_lzcnt32
    jit_template_push_rax_ret

; ------------------------------------------------------------------
; i32 ctz (0x68)
; -----------------------------------------------------------------+
er_fn jit_template_i32_ctz
    xor     ecx, ecx
    call    jit_emit_pop_reg
    call    jit_emit_tzcnt32
    jit_template_push_rax_ret

; ------------------------------------------------------------------
; i32 popcnt (0x69)
; -----------------------------------------------------------------+
er_fn jit_template_i32_popcnt
    xor     ecx, ecx
    call    jit_emit_pop_reg
    call    jit_emit_popcnt32
    jit_template_push_rax_ret

; ------------------------------------------------------------------
; i32 add (0x6A)
; Emit: pop rcx; pop rax; add eax, ecx; push rax
; -----------------------------------------------------------------+
er_fn jit_template_i32_add
    mov     cl, 1            ; pop rcx
    call    jit_emit_pop_reg
    xor     ecx, ecx         ; pop rax
    call    jit_emit_pop_reg
    call    jit_emit_add32
    xor     ecx, ecx         ; push rax
    call    jit_emit_push_reg
    ret

; ------------------------------------------------------------------
; i32 sub (0x6B)
; -----------------------------------------------------------------+
er_fn jit_template_i32_sub
    jit_template_pop_rcx_rax
    call    jit_emit_sub32
    jit_template_push_rax_ret

; ------------------------------------------------------------------
; i32 mul (0x6C)
; -----------------------------------------------------------------+
er_fn jit_template_i32_mul
    jit_template_pop_rcx_rax
    call    jit_emit_imul32
    jit_template_push_rax_ret

; ------------------------------------------------------------------
; i32 div_s (0x6D)
; Emit: pop rcx; pop rax; cdq; idiv ecx; push rax
; -----------------------------------------------------------------+
er_fn jit_template_i32_div_s
    jit_template_pop_rcx_rax
    mov     al, 0x99         ; cdq
    call    jit_emit_byte
    call    jit_emit_idiv32
    jit_template_push_rax_ret

; ------------------------------------------------------------------
; i32 div_u (0x6E)
; Emit: pop rcx; pop rax; xor edx, edx; div ecx; push rax
; -----------------------------------------------------------------+
er_fn jit_template_i32_div_u
    jit_template_pop_rcx_rax
    call    jit_emit_xor_eax_eax ; zero edx via xor edx, edx
    ; Actually jit_emit_xor_eax_eax does xor eax, eax. We need xor edx, edx.
    ; Emit: xor edx, edx
    mov     al, 0x31
    call    jit_emit_byte
    mov     al, 0xD2         ; xor edx, edx
    call    jit_emit_modrm
    call    jit_emit_div32
    jit_template_push_rax_ret

; ------------------------------------------------------------------
; i32 rem_s (0x6F)
; Emit: pop rcx; pop rax; cdq; idiv ecx; mov eax, edx; push rax
; -----------------------------------------------------------------+
er_fn jit_template_i32_rem_s
    jit_template_pop_rcx_rax
    mov     al, 0x99         ; cdq
    call    jit_emit_byte
    call    jit_emit_idiv32
    ; mov eax, edx
    jit_template_emit_mov_accum_rdx
    jit_template_push_rax_ret

; ------------------------------------------------------------------
; i32 rem_u (0x70)
; Emit: pop rcx; pop rax; xor edx, edx; div ecx; mov eax, edx; push rax
; -----------------------------------------------------------------+
er_fn jit_template_i32_rem_u
    jit_template_pop_rcx_rax
    mov     al, 0x31
    call    jit_emit_byte
    mov     al, 0xD2         ; xor edx, edx
    call    jit_emit_modrm
    call    jit_emit_div32
    jit_template_emit_mov_accum_rdx
    jit_template_push_rax_ret

; ------------------------------------------------------------------
; i32 and (0x71)
; -----------------------------------------------------------------+
er_fn jit_template_i32_and
    jit_template_pop_rcx_rax
    call    jit_emit_and32
    jit_template_push_rax_ret

; ------------------------------------------------------------------
; i32 or (0x72)
; -----------------------------------------------------------------+
er_fn jit_template_i32_or
    jit_template_pop_rcx_rax
    call    jit_emit_or32
    jit_template_push_rax_ret

; ------------------------------------------------------------------
; i32 xor (0x73)
; -----------------------------------------------------------------+
er_fn jit_template_i32_xor
    jit_template_pop_rcx_rax
    call    jit_emit_xor32
    jit_template_push_rax_ret

; ------------------------------------------------------------------
; i32 shl (0x74)
; Emit: pop rcx; pop rax; shl eax, cl; push rax
; -----------------------------------------------------------------+
er_fn jit_template_i32_shl
    jit_template_pop_rcx_rax
    call    jit_emit_shl32
    jit_template_push_rax_ret

; ------------------------------------------------------------------
; i32 shr_s (0x75)
; -----------------------------------------------------------------+
er_fn jit_template_i32_shr_s
    jit_template_pop_rcx_rax
    call    jit_emit_sar32
    jit_template_push_rax_ret

; ------------------------------------------------------------------
; i32 shr_u (0x76)
; -----------------------------------------------------------------+
er_fn jit_template_i32_shr_u
    jit_template_pop_rcx_rax
    call    jit_emit_shr32
    jit_template_push_rax_ret

; ------------------------------------------------------------------
; i32 rotl (0x77)
; -----------------------------------------------------------------+
er_fn jit_template_i32_rotl
    jit_template_pop_rcx_rax
    call    jit_emit_rol32
    jit_template_push_rax_ret

; ------------------------------------------------------------------
; i32 rotr (0x78)
; -----------------------------------------------------------------+
er_fn jit_template_i32_rotr
    jit_template_pop_rcx_rax
    call    jit_emit_ror32
    jit_template_push_rax_ret

; ------------------------------------------------------------------
; i32 wrap_i64 (0xA7)
; Emit: pop rax; push rax (64-to-32 is automatic on x86_64)
; Actually need to zero-extend: mov eax, eax does it
; -----------------------------------------------------------------+
er_fn jit_template_i32_wrap_i64
    xor     ecx, ecx
    call    jit_emit_pop_reg
    ; mov eax, eax (zero-extend)
    mov     al, 0x89
    call    jit_emit_byte
    mov     al, 0xC0
    call    jit_emit_modrm
    jit_template_push_rax_ret

; ------------------------------------------------------------------
; i64 extend_i32_s (0xAC)
; Emit: pop rax; movsxd rax, eax; push rax
; -----------------------------------------------------------------+
er_fn jit_template_i64_extend_i32_s
    xor     ecx, ecx
    call    jit_emit_pop_reg
    ; movsxd rax, eax — REX.W + 63 /r
    mov     al, 0x48         ; REX.W
    call    jit_emit_byte
    mov     al, 0x63         ; movsxd
    call    jit_emit_byte
    mov     al, 0xC0         ; ModRM: mod=11, reg=0(rax), rm=0(rax/eax)
    call    jit_emit_modrm
    jit_template_push_rax_ret

; ------------------------------------------------------------------
; i64 extend_i32_u (0xAD)
; Emit: pop rax; mov eax, eax (zero-extend); push rax
; -----------------------------------------------------------------+
er_fn jit_template_i64_extend_i32_u
    xor     ecx, ecx
    call    jit_emit_pop_reg
    mov     al, 0x89
    call    jit_emit_byte
    mov     al, 0xC0         ; mov eax, eax
    call    jit_emit_modrm
    jit_template_push_rax_ret

; ==================================================================
; Sign extension templates
; =================================================================+

; i32.extend8_s (0xC0) — movsx eax, al
er_fn jit_template_i32_extend8_s
    xor     ecx, ecx
    call    jit_emit_pop_reg
    mov     al, 0x0F
    call    jit_emit_byte
    mov     al, 0xBE
    call    jit_emit_byte
    jit_template_modrm_c0_push_rax

; i32.extend16_s (0xC1) — movsx eax, ax
er_fn jit_template_i32_extend16_s
    xor     ecx, ecx
    call    jit_emit_pop_reg
    mov     al, 0x0F
    call    jit_emit_byte
    mov     al, 0xBF
    call    jit_emit_byte
    jit_template_modrm_c0_push_rax

; i64.extend8_s (0xC2) — REX.W movsx rax, al
er_fn jit_template_i64_extend8_s
    xor     ecx, ecx
    call    jit_emit_pop_reg
    mov     al, 0x48
    call    jit_emit_byte
    mov     al, 0x0F
    call    jit_emit_byte
    mov     al, 0xBE
    call    jit_emit_byte
    jit_template_modrm_c0_push_rax

; i64.extend16_s (0xC3) — REX.W movsx rax, ax
er_fn jit_template_i64_extend16_s
    xor     ecx, ecx
    call    jit_emit_pop_reg
    mov     al, 0x48
    call    jit_emit_byte
    mov     al, 0x0F
    call    jit_emit_byte
    mov     al, 0xBF
    call    jit_emit_byte
    jit_template_modrm_c0_push_rax

; i64.extend32_s (0xC4) — REX.W movsxd rax, eax
er_fn jit_template_i64_extend32_s
    xor     ecx, ecx
    call    jit_emit_pop_reg
    mov     al, 0x48
    call    jit_emit_byte
    mov     al, 0x63
    call    jit_emit_byte
    jit_template_modrm_c0_push_rax

; ==================================================================
; Reference type templates
; =================================================================+

; ref.null (0xD0) — push -1 (null reference)
er_fn jit_template_ref_null
    mov     eax, -1
    jmp     jit_emit_push_imm32

; ref.is_null (0xD1) — pop, cmp -1, sete al, push
er_fn jit_template_ref_is_null
    xor     ecx, ecx
    call    jit_emit_pop_reg
    mov     al, 0x83           ; cmp eax, -1 (83 F8 FF)
    call    jit_emit_byte
    mov     al, 0xF8
    call    jit_emit_modrm
    mov     al, -1
    call    jit_emit_byte
    mov     al, 0x0F           ; sete al (0F 94 C0)
    call    jit_emit_byte
    mov     al, 0x94
    call    jit_emit_byte
    mov     al, 0xC0
    call    jit_emit_modrm
    mov     al, 0x0F           ; movzx eax, al (0F B6 C0)
    call    jit_emit_byte
    mov     al, 0xB6
    call    jit_emit_byte
    jit_template_modrm_c0_push_rax

; ref.func (0xD2) — push function index as reference
er_fn jit_template_ref_func
    mov     eax, [rdi + 12]  ; imm0 = function index
    jmp     jit_emit_push_imm32

; ==================================================================
; Parametric templates
; =================================================================+

; select (0x1B) / select_typed (0x1C)
er_fn jit_template_select
    mov     cl, 2                           ; rdx = condition
    call    jit_emit_pop_reg
    mov     cl, 1                           ; rcx = false_val
    call    jit_emit_pop_reg
    xor     ecx, ecx                        ; rax = true_val
    call    jit_emit_pop_reg
    mov     al, 0x85                        ; test edx, edx
    call    jit_emit_byte
    mov     al, 0xD2
    call    jit_emit_modrm
    mov     al, 0x0F                        ; cmovz eax, ecx
    call    jit_emit_byte
    mov     al, 0x44
    call    jit_emit_byte
    mov     al, 0xC1
    call    jit_emit_modrm
    xor     ecx, ecx
    jmp     jit_emit_push_reg

; ==================================================================
; Memory management templates
; =================================================================+

; memory.size (0x3F) — push current page count
er_fn jit_template_memory_size
    mov     cl, 2                           ; rdx
    mov     eax, JitGlobals.memory_pages
    call    jit_emit_load_global_to_reg
    xor     ecx, ecx
    jmp     jit_emit_push_reg

; memory.grow (0x40) — explicit user-allocation boundary
; JIT path defers to interpreter policy by surfacing ERROR_MEMORY_GROWTH
; so both paths enforce the same constant and user-visible behavior.
er_fn jit_template_memory_grow
    mov     rdx, ERROR_MEMORY_GROWTH
    mov     [rel jit_template_error], rdx
    ret

; ==================================================================
; Memory op templates (i32.load, i64.load, i32.store, i64.store)
; =================================================================+

; ------------------------------------------------------------------
; i32.load (0x28) — pop address, load 4 bytes, push
; -----------------------------------------------------------------+
er_fn jit_template_i32_load
    mov     cl, 1
    call    jit_emit_pop_reg      ; pop rcx (address)
    call    jit_emit_mem_load32
    jit_template_push_rax_ret

; ------------------------------------------------------------------
; i64.load (0x29) — pop address, load 8 bytes, push
; -----------------------------------------------------------------+
er_fn jit_template_i64_load
    mov     cl, 1
    call    jit_emit_pop_reg
    call    jit_emit_mem_load64
    jit_template_push_rax_ret

; ------------------------------------------------------------------
; i32.store (0x36) — pop address, pop value, store 4 bytes
; -----------------------------------------------------------------+
er_fn jit_template_i32_store
    mov     cl, 1
    call    jit_emit_pop_reg      ; pop rcx (address)
    xor     ecx, ecx
    call    jit_emit_pop_reg      ; pop rax (value)
    call    jit_emit_mem_store32
    ret

; ------------------------------------------------------------------
; i64.store (0x37) — pop address, pop value, store 8 bytes
; -----------------------------------------------------------------+
er_fn jit_template_i64_store
    jit_template_pop_rcx_rax
    call    jit_emit_mem_store64
    ret

; ==================================================================
; Narrow memory load/store templates
; =================================================================+

; i32.load8_s (0x2C)
er_fn jit_template_i32_load8_s
    mov     cl, 1
    call    jit_emit_pop_reg
    call    jit_emit_mem_load8_s
    xor     ecx, ecx
    jmp     jit_emit_push_reg

; i32.load8_u (0x2D)
er_fn jit_template_i32_load8_u
    mov     cl, 1
    call    jit_emit_pop_reg
    call    jit_emit_mem_load8_u
    xor     ecx, ecx
    jmp     jit_emit_push_reg

; i32.load16_s (0x2E)
er_fn jit_template_i32_load16_s
    mov     cl, 1
    call    jit_emit_pop_reg
    call    jit_emit_mem_load16_s
    xor     ecx, ecx
    jmp     jit_emit_push_reg

; i32.load16_u (0x2F)
er_fn jit_template_i32_load16_u
    mov     cl, 1
    call    jit_emit_pop_reg
    call    jit_emit_mem_load16_u
    xor     ecx, ecx
    jmp     jit_emit_push_reg

; i64.load8_s (0x30) — REX.W movsx to rax
er_fn jit_template_i64_load8_s
    mov     cl, 1
    call    jit_emit_pop_reg
    call    jit_emit_mem_load8_s_64
    xor     ecx, ecx
    jmp     jit_emit_push_reg

; i64.load16_s (0x32) — REX.W movsx to rax
er_fn jit_template_i64_load16_s
    mov     cl, 1
    call    jit_emit_pop_reg
    call    jit_emit_mem_load16_s_64
    xor     ecx, ecx
    jmp     jit_emit_push_reg

; i64.load32_s (0x34) — movsxd rax, dword
er_fn jit_template_i64_load32_s
    mov     cl, 1
    call    jit_emit_pop_reg
    call    jit_emit_mem_load32_s
    xor     ecx, ecx
    jmp     jit_emit_push_reg

; i32.store8 (0x3A)
er_fn jit_template_i32_store8
    jit_template_pop_rcx_rax
    call    jit_emit_mem_store8
    ret

; i32.store16 (0x3B)
er_fn jit_template_i32_store16
    jit_template_pop_rcx_rax
    call    jit_emit_mem_store16
    ret

; ==================================================================
; i64 comparison templates (REX.W + i32 compare pattern)
; =================================================================+

; ------------------------------------------------------------------
; i64 eqz (0x50)
; -----------------------------------------------------------------+
er_fn jit_template_i64_eqz
    xor     ecx, ecx
    call    jit_emit_pop_reg
    call    jit_emit_test64
    mov     cl, 0x94
    jit_template_setcc_push_ret

; ------------------------------------------------------------------
; i64 eq (0x51)
; -----------------------------------------------------------------+
er_fn jit_template_i64_eq
    jit_template_pop_rcx_rax
    call    jit_emit_cmp64
    mov     cl, 0x94
    jit_template_setcc_push_ret

; ------------------------------------------------------------------
; i64 ne (0x52)
; -----------------------------------------------------------------+
er_fn jit_template_i64_ne
    jit_template_pop_rcx_rax
    call    jit_emit_cmp64
    mov     cl, 0x95
    jit_template_setcc_push_ret

; ------------------------------------------------------------------
; i64 lt_s (0x53)
; -----------------------------------------------------------------+
er_fn jit_template_i64_lt_s
    jit_template_pop_rcx_rax
    call    jit_emit_cmp64
    mov     cl, 0x9C
    jit_template_setcc_push_ret

; ------------------------------------------------------------------
; i64 lt_u (0x54)
; -----------------------------------------------------------------+
er_fn jit_template_i64_lt_u
    jit_template_pop_rcx_rax
    call    jit_emit_cmp64
    mov     cl, 0x92
    jit_template_setcc_push_ret

; ------------------------------------------------------------------
; i64 gt_s (0x55)
; -----------------------------------------------------------------+
er_fn jit_template_i64_gt_s
    jit_template_pop_rcx_rax
    call    jit_emit_cmp64
    mov     cl, 0x9F
    jit_template_setcc_push_ret

; ------------------------------------------------------------------
; i64 gt_u (0x56)
; -----------------------------------------------------------------+
er_fn jit_template_i64_gt_u
    jit_template_pop_rcx_rax
    call    jit_emit_cmp64
    mov     cl, 0x97
    jit_template_setcc_push_ret

; ------------------------------------------------------------------
; i64 le_s (0x57)
; -----------------------------------------------------------------+
er_fn jit_template_i64_le_s
    jit_template_pop_rcx_rax
    call    jit_emit_cmp64
    mov     cl, 0x9E
    jit_template_setcc_push_ret

; ------------------------------------------------------------------
; i64 le_u (0x58)
; -----------------------------------------------------------------+
er_fn jit_template_i64_le_u
    jit_template_pop_rcx_rax
    call    jit_emit_cmp64
    mov     cl, 0x96
    jit_template_setcc_push_ret

; ------------------------------------------------------------------
; i64 ge_s (0x59)
; -----------------------------------------------------------------+
er_fn jit_template_i64_ge_s
    jit_template_pop_rcx_rax
    call    jit_emit_cmp64
    mov     cl, 0x9D
    jit_template_setcc_push_ret

; ------------------------------------------------------------------
; i64 ge_u (0x5A)
; -----------------------------------------------------------------+
er_fn jit_template_i64_ge_u
    jit_template_pop_rcx_rax
    call    jit_emit_cmp64
    mov     cl, 0x93
    jit_template_setcc_push_ret

; ==================================================================
; i64 unary arithmetic (clz, ctz, popcnt)
; =================================================================+

; ------------------------------------------------------------------
; i64 clz (0x79)
; -----------------------------------------------------------------+
er_fn jit_template_i64_clz
    xor     ecx, ecx
    call    jit_emit_pop_reg
    call    jit_emit_lzcnt64
    jit_template_push_rax_ret

; ------------------------------------------------------------------
; i64 ctz (0x7A)
; -----------------------------------------------------------------+
er_fn jit_template_i64_ctz
    xor     ecx, ecx
    call    jit_emit_pop_reg
    call    jit_emit_tzcnt64
    jit_template_push_rax_ret

; ------------------------------------------------------------------
; i64 popcnt (0x7B)
; -----------------------------------------------------------------+
er_fn jit_template_i64_popcnt
    xor     ecx, ecx
    call    jit_emit_pop_reg
    call    jit_emit_popcnt64
    jit_template_push_rax_ret

; ==================================================================
; i64 binary arithmetic
; =================================================================+

; ------------------------------------------------------------------
; i64 add (0x7C)
; -----------------------------------------------------------------+
er_fn jit_template_i64_add
    jit_template_pop_rcx_rax
    call    jit_emit_rex_nob
    call    jit_emit_add32
    jit_template_push_rax_ret

; ------------------------------------------------------------------
; i64 sub (0x7D)
; -----------------------------------------------------------------+
er_fn jit_template_i64_sub
    jit_template_pop_rcx_rax
    call    jit_emit_rex_nob
    call    jit_emit_sub32
    jit_template_push_rax_ret

; ------------------------------------------------------------------
; i64 mul (0x7E)
; -----------------------------------------------------------------+
er_fn jit_template_i64_mul
    jit_template_pop_rcx_rax
    call    jit_emit_rex_nob
    call    jit_emit_imul32
    jit_template_push_rax_ret

; ------------------------------------------------------------------
; i64 div_s (0x7F)
; -----------------------------------------------------------------+
er_fn jit_template_i64_div_s
    jit_template_pop_rcx_rax
    call    jit_emit_cqo
    call    jit_emit_rex_nob
    call    jit_emit_idiv32
    jit_template_push_rax_ret

; ------------------------------------------------------------------
; i64 div_u (0x80)
; -----------------------------------------------------------------+
er_fn jit_template_i64_div_u
    jit_template_pop_rcx_rax
    call    jit_emit_xor_edx_edx
    call    jit_emit_rex_nob
    call    jit_emit_div32
    jit_template_push_rax_ret

; ------------------------------------------------------------------
; i64 rem_s (0x81)
; -----------------------------------------------------------------+
er_fn jit_template_i64_rem_s
    jit_template_pop_rcx_rax
    call    jit_emit_cqo
    call    jit_emit_rex_nob
    call    jit_emit_idiv32
    mov     al, 0x48         ; REX.W for mov rdx, rax
    call    jit_emit_byte
    jit_template_emit_mov_accum_rdx
    jit_template_push_rax_ret

; ------------------------------------------------------------------
; i64 rem_u (0x82)
; -----------------------------------------------------------------+
er_fn jit_template_i64_rem_u
    jit_template_pop_rcx_rax
    call    jit_emit_xor_edx_edx
    call    jit_emit_rex_nob
    call    jit_emit_div32
    mov     al, 0x48         ; REX.W for mov rax, rdx
    call    jit_emit_byte
    jit_template_emit_mov_accum_rdx
    jit_template_push_rax_ret

; ------------------------------------------------------------------
; i64 and (0x83)
; -----------------------------------------------------------------+
er_fn jit_template_i64_and
    jit_template_pop_rcx_rax
    call    jit_emit_rex_nob
    call    jit_emit_and32
    jit_template_push_rax_ret

; ------------------------------------------------------------------
; i64 or (0x84)
; -----------------------------------------------------------------+
er_fn jit_template_i64_or
    jit_template_pop_rcx_rax
    call    jit_emit_rex_nob
    call    jit_emit_or32
    jit_template_push_rax_ret

; ------------------------------------------------------------------
; i64 xor (0x85)
; -----------------------------------------------------------------+
er_fn jit_template_i64_xor
    jit_template_pop_rcx_rax
    call    jit_emit_rex_nob
    call    jit_emit_xor32
    jit_template_push_rax_ret

; ------------------------------------------------------------------
; i64 shl (0x86)
; -----------------------------------------------------------------+
er_fn jit_template_i64_shl
    jit_template_pop_rcx_rax
    call    jit_emit_rex_nob
    call    jit_emit_shl32
    jit_template_push_rax_ret

; ------------------------------------------------------------------
; i64 shr_s (0x87) — arithmetic shift right
; -----------------------------------------------------------------+
er_fn jit_template_i64_shr_s
    jit_template_pop_rcx_rax
    call    jit_emit_rex_nob
    call    jit_emit_sar32
    jit_template_push_rax_ret

; ------------------------------------------------------------------
; i64 shr_u (0x88) — logical shift right
; -----------------------------------------------------------------+
er_fn jit_template_i64_shr_u
    jit_template_pop_rcx_rax
    call    jit_emit_rex_nob
    call    jit_emit_shr32
    jit_template_push_rax_ret

; ------------------------------------------------------------------
; i64 rotl (0x89)
; -----------------------------------------------------------------+
er_fn jit_template_i64_rotl
    jit_template_pop_rcx_rax
    call    jit_emit_rex_nob
    call    jit_emit_rol32
    jit_template_push_rax_ret

; ------------------------------------------------------------------
; i64 rotr (0x8A)
; -----------------------------------------------------------------+
er_fn jit_template_i64_rotr
    jit_template_pop_rcx_rax
    call    jit_emit_rex_nob
    call    jit_emit_ror32
    jit_template_push_rax_ret

; ------------------------------------------------------------------
; Template: call (0x10) — compile-time trampoline
;
; Emitted runtime code:
;   push r15          ; save JitGlobals
;   sub  rsp, n*8     ; allocate temp buffer
;   ; copy params from WASM stack to buffer (WASM order)
;   mov  rdi, func_idx
;   mov  rsi, rsp     ; buffer
;   mov  edx, n       ; param_count
;   mov  rax, er_wasm_jit_exec
;   call rax
;   add  rsp, n*8     ; free buffer
;   ; if result_count > 0: push rax
;   pop  r15          ; restore JitGlobals
; -----------------------------------------------------------------+
er_fn jit_template_call
    push    r12

    mov     r8d, [rdi + 12]         ; r8 = func_idx (imm0)
    mov     edi, r8d
    call    er_wasm_type_index_for_function
    er_check_nonzero rdx, .call_type_error
    mov     r10, rax
    imul    r10, FUNC_TYPE_SIZE
    mov     r11, [rel types_buf + r10 + FUNC_TYPE_PARAM_COUNT_OFF]
    mov     r12, [rel types_buf + r10 + FUNC_TYPE_RESULT_COUNT_OFF]

    cmp     r11, 8
    ja      .call_too_many_params

    mov     cl, 15
    call    jit_emit_push_reg

    mov     eax, r11d
    shl     eax, 3
    call    jit_emit_sub_rsp_imm

    mov     r9d, r11d
    dec     r9d
.call_copy_loop:
    cmp     r9d, 0
    jl      .call_copy_done

    mov     eax, r11d
    shl     eax, 1
    dec     eax
    sub     eax, r9d
    shl     eax, 3
    call    jit_emit_load_rax_rsp_disp

    mov     eax, r9d
    shl     eax, 3
    call    jit_emit_store_rax_rsp_disp

    dec     r9d
    jmp     .call_copy_loop
.call_copy_done:

    mov     rax, r8
    call    jit_emit_mov_rdi_imm64
    call    jit_emit_mov_rsi_rsp
    mov     eax, r11d
    call    jit_emit_mov_edx_imm32
    mov     rax, er_wasm_jit_exec
    call    jit_emit_mov_rax_imm64
    call    jit_emit_call_rax

    mov     eax, r11d
    shl     eax, 3
    call    jit_emit_add_rsp_imm

    mov     cl, 15
    call    jit_emit_pop_reg        ; restore r15 BEFORE result push

    cmp     r12, 1
    jl      .call_no_result
    jit_template_push_rax
.call_no_result:

    pop     r12
    er_ok
    ret

.call_type_error:
    pop     r12
    ; rdx already has error from er_wasm_type_index_for_function
    mov     [rel jit_template_error], rdx
    ret

.call_too_many_params:
    pop     r12
    mov     rdx, ERROR_NOT_IMPLEMENTED
    mov     [rel jit_template_error], rdx
    ret

; ------------------------------------------------------------------
; Template: call_indirect (0x11) — runtime function index
;
; Emitted runtime code:
;   push r15          ; save JitGlobals
;   pop  rdi          ; func_idx from WASM stack
;   sub  rsp, n*8     ; allocate temp buffer
;   ; copy params from WASM stack to buffer (WASM order)
;   mov  rsi, rsp     ; buffer
;   mov  edx, n       ; param_count
;   mov  rax, er_wasm_jit_exec
;   call rax
;   add  rsp, n*8     ; free buffer
;   ; if result_count > 0: push rax
;   pop  r15          ; restore JitGlobals
; -----------------------------------------------------------------+
er_fn jit_template_call_indirect
    push    r12

    mov     r10d, [rdi + 12]        ; r10 = type_index (imm0)
    imul    r10d, FUNC_TYPE_SIZE
    mov     r11, [rel types_buf + r10 + FUNC_TYPE_PARAM_COUNT_OFF]
    mov     r12, [rel types_buf + r10 + FUNC_TYPE_RESULT_COUNT_OFF]

    cmp     r11, 8
    ja      .ci_too_many_params

    mov     cl, 15
    call    jit_emit_push_reg

    mov     cl, 7
    call    jit_emit_pop_reg

    mov     eax, r11d
    shl     eax, 3
    call    jit_emit_sub_rsp_imm

    mov     r8d, r11d
    dec     r8d
.ci_copy_loop:
    cmp     r8d, 0
    jl      .ci_copy_done

    mov     eax, r11d
    shl     eax, 1
    dec     eax
    sub     eax, r8d
    shl     eax, 3
    call    jit_emit_load_rax_rsp_disp

    mov     eax, r8d
    shl     eax, 3
    call    jit_emit_store_rax_rsp_disp

    dec     r8d
    jmp     .ci_copy_loop
.ci_copy_done:

    call    jit_emit_mov_rsi_rsp
    mov     eax, r11d
    call    jit_emit_mov_edx_imm32
    mov     rax, er_wasm_jit_exec
    call    jit_emit_mov_rax_imm64
    call    jit_emit_call_rax

    mov     eax, r11d
    shl     eax, 3
    call    jit_emit_add_rsp_imm

    mov     cl, 15
    call    jit_emit_pop_reg

    cmp     r12, 1
    jl      .ci_no_result
    jit_template_push_rax
.ci_no_result:

    pop     r12
    er_ok
    ret

.ci_too_many_params:
    pop     r12
    mov     rdx, ERROR_NOT_IMPLEMENTED
    mov     [rel jit_template_error], rdx
    ret

; ==================================================================
; Float comparison helpers — shared by f32 and f64 comparison templates
; =================================================================+
; After ucomiss/ucomisd:
;   ZF=1,PF=0,CF=0 → equal
;   ZF=0,PF=0,CF=0 → greater than
;   ZF=0,PF=0,CF=1 → less than
;   ZF=1,PF=1,CF=1 → unordered (NaN)
;
; NaN fixup:
;   eq: setnp al; mov ah, al; sete al; and al, ah  (false on NaN)
;   ne: setne al; setnp ah; ... wait, need OR with PF
;   ne: setp al; ... no
;
;   Actually:
;   eq: sete & ~PF   → setnp; save; sete; and
;   ne: setne | PF   → sete? no
;   ne: the opposite of eq: setne al (1 if !equal); but NaN → ZF=1 → setne=0, we want 1
;   Better: setp al (1 if NaN); setne ah (1 if not equal); or al, ah
;   Or simplest: sete al; setnp ah; xor al, 1; and al, ah; xor al, 1
;     (flip eq result for NaN-safe eq → ne)
;   lt: setb & ~PF   → setnp; save; setb; and
;   gt: seta alone (NaN: CF=1 → seta=0 ✓)
;   le: setbe & ~PF  → setnp; save; setbe; and
;   ge: setae alone (NaN: CF=1 → setae=0 ✓)
;
; For ne: use setp al; mov ah, al; sete al; xor al, 1; and al, ah; xor al, 1
;   NaN: setp al=1 → ah=1; sete al=1; xor 1=0; and 0,1=0; xor 1=1 ✓
;   equal: setp al=0 → ah=0; sete al=1; xor 1=0; and 0,0=0; xor 1=1 ... WRONG!
;   Not equal: setp al=0 → ah=0; sete al=0; xor 1=1; and 1,0=0; xor 1=1 ✓
;
; Hmm, equal case is wrong. Let me try:
; ne = (a != b) || isNaN(a,b) = NOT(eq & ordered)
;   = NOT(sete & ~PF) = (NOT sete) | PF
;   = setne | PF
; So: setp al (PF→al); setne ah; or al, ah

; -----------------------------------------------------------------+
; Emit f32 comparison prologue: pop xmm1(right), pop xmm0(left), ucomiss
; -----------------------------------------------------------------+
er_fn jit_emit_f32_cmp_prologue
    mov     cl, 1
    call    jit_emit_pop_reg          ; pop rcx
    xor     ecx, ecx
    call    jit_emit_pop_reg          ; pop rax
    call    jit_emit_movd_xmm1_ecx
    call    jit_emit_movd_xmm0_eax
    mov     al, 0x0F
    call    jit_emit_byte
    mov     al, 0x2E
    call    jit_emit_byte
    mov     al, 0xC1
    jmp     jit_emit_modrm

; -----------------------------------------------------------------+
; Emit f64 comparison prologue: pop xmm1(right), pop xmm0(left), ucomisd
; -----------------------------------------------------------------+
er_fn jit_emit_f64_cmp_prologue
    mov     cl, 1
    call    jit_emit_pop_reg          ; pop rcx
    xor     ecx, ecx
    call    jit_emit_pop_reg          ; pop rax
    call    jit_emit_movq_xmm1_rcx
    call    jit_emit_movq_xmm0_rax
    mov     al, 0x66
    call    jit_emit_byte
    mov     al, 0x0F
    call    jit_emit_byte
    mov     al, 0x2E
    call    jit_emit_byte
    mov     al, 0xC1
    jmp     jit_emit_modrm

; ==================================================================
; F32 comparison templates (0x5B-0x60)
; Each: cmp_prologue → setcc → NaN fixup → movzx → push
; =================================================================+

; f32.eq (0x5B)
er_fn jit_template_f32_eq
    call    jit_emit_f32_cmp_prologue
    call    jit_emit_setnp_save_ah
    mov     cl, 0x94
    jit_template_setcc_and_push

; f32.ne (0x5C)
er_fn jit_template_f32_ne
    call    jit_emit_f32_cmp_prologue
    call    jit_emit_setp_save_ah
    mov     cl, 0x95
    jit_template_setcc_or_push
; ACTUALLY: setp al; mov ah, al; setne al; or al, ah

; f32.lt (0x5D)
er_fn jit_template_f32_lt
    call    jit_emit_f32_cmp_prologue
    call    jit_emit_setnp_save_ah
    mov     cl, 0x92               ; setb (CF=1, below)
    jit_template_setcc_and_push

; f32.gt (0x5E)
er_fn jit_template_f32_gt
    call    jit_emit_f32_cmp_prologue
    mov     cl, 0x97               ; seta (CF=0,ZF=0)
    jit_template_setcc_push

; f32.le (0x5F)
er_fn jit_template_f32_le
    call    jit_emit_f32_cmp_prologue
    call    jit_emit_setnp_save_ah
    mov     cl, 0x96               ; setbe (CF=1 or ZF=1)
    jit_template_setcc_and_push

; f32.ge (0x60)
er_fn jit_template_f32_ge
    call    jit_emit_f32_cmp_prologue
    mov     cl, 0x93               ; setae (CF=0)
    jit_template_setcc_push

; ==================================================================
; F64 comparison templates (0x61-0x66)
; =================================================================+

; f64.eq (0x61)
er_fn jit_template_f64_eq
    call    jit_emit_f64_cmp_prologue
    call    jit_emit_setnp_save_ah
    mov     cl, 0x94
    jit_template_setcc_and_push

; f64.ne (0x62)
er_fn jit_template_f64_ne
    call    jit_emit_f64_cmp_prologue
    call    jit_emit_setp_save_ah
    mov     cl, 0x95
    jit_template_setcc_or_push

; f64.lt (0x63)
er_fn jit_template_f64_lt
    call    jit_emit_f64_cmp_prologue
    call    jit_emit_setnp_save_ah
    mov     cl, 0x92
    jit_template_setcc_and_push

; f64.gt (0x64)
er_fn jit_template_f64_gt
    call    jit_emit_f64_cmp_prologue
    mov     cl, 0x97
    jit_template_setcc_push

; f64.le (0x65)
er_fn jit_template_f64_le
    call    jit_emit_f64_cmp_prologue
    call    jit_emit_setnp_save_ah
    mov     cl, 0x96
    jit_template_setcc_and_push

; f64.ge (0x66)
er_fn jit_template_f64_ge
    call    jit_emit_f64_cmp_prologue
    mov     cl, 0x93
    jit_template_setcc_push

; ==================================================================
; F32 constant and unary templates
; =================================================================+

; -----------------------------------------------------------------+
; f32.const (0x43) — push imm32 float bits
; -----------------------------------------------------------------+
er_fn jit_template_f32_const
    mov     eax, [rdi + 12]
    call    jit_emit_push_imm32
    ret

; -----------------------------------------------------------------+
; f64.const (0x44) — push imm64 float bits
; -----------------------------------------------------------------+
er_fn jit_template_f64_const
    mov     r10d, [rdi + 12]
    mov     r11d, [rdi + 16]
    jit_template_push_imm64_parts

; -----------------------------------------------------------------+
; f32.abs (0x8B) — pop; clear sign bit 31; push
; -----------------------------------------------------------------+
er_fn jit_template_f32_abs
    xor     ecx, ecx
    call    jit_emit_pop_reg
    mov     cl, 31
    call    jit_emit_btr_eax
    xor     ecx, ecx
    jmp     jit_emit_push_reg

; -----------------------------------------------------------------+
; f32.neg (0x8C) — pop; toggle sign bit 31; push
; -----------------------------------------------------------------+
er_fn jit_template_f32_neg
    xor     ecx, ecx
    call    jit_emit_pop_reg
    mov     cl, 31
    call    jit_emit_btc_eax
    xor     ecx, ecx
    jmp     jit_emit_push_reg

; -----------------------------------------------------------------+
; f64.abs (0x99) — pop; clear sign bit 63; push
; -----------------------------------------------------------------+
er_fn jit_template_f64_abs
    xor     ecx, ecx
    call    jit_emit_pop_reg
    mov     cl, 63
    call    jit_emit_btr_rax
    xor     ecx, ecx
    jmp     jit_emit_push_reg

; -----------------------------------------------------------------+
; f64.neg (0x9A) — pop; toggle sign bit 63; push
; -----------------------------------------------------------------+
er_fn jit_template_f64_neg
    xor     ecx, ecx
    call    jit_emit_pop_reg
    mov     cl, 63
    call    jit_emit_btc_rax
    xor     ecx, ecx
    jmp     jit_emit_push_reg

; -----------------------------------------------------------------+
; f32.sqrt (0x91) — pop, sqrtss, push
; -----------------------------------------------------------------+
er_fn jit_template_f32_sqrt
    xor     ecx, ecx
    call    jit_emit_pop_reg
    call    jit_emit_movd_xmm0_eax
    mov     al, 0xF3
    mov     ch, 0x51
    mov     cl, 0xC0
    call    jit_emit_sse_op
    call    jit_emit_movd_eax_xmm0
    xor     ecx, ecx
    jmp     jit_emit_push_reg

; -----------------------------------------------------------------+
; f64.sqrt (0x9F) — pop, sqrtsd, push
; -----------------------------------------------------------------+
er_fn jit_template_f64_sqrt
    xor     ecx, ecx
    call    jit_emit_pop_reg
    call    jit_emit_movq_xmm0_rax
    mov     al, 0xF2
    mov     ch, 0x51
    mov     cl, 0xC0
    call    jit_emit_sse_op
    call    jit_emit_movq_rax_xmm0
    xor     ecx, ecx
    jmp     jit_emit_push_reg

; ==================================================================
; F32 SSE binary arithmetic templates (0x92-0x95)
; Each: pop rcx, pop rax, movd xmm1, movd xmm0, opss, movd, push
; =================================================================+

; f32.add (0x92)
er_fn jit_template_f32_add
    jit_template_pop_rcx_rax
    call    jit_emit_movd_xmm1_ecx
    call    jit_emit_movd_xmm0_eax
    jit_template_f32_sse_bin_tail 0x58

; f32.sub (0x93)
er_fn jit_template_f32_sub
    jit_template_pop_rcx_rax
    call    jit_emit_movd_xmm1_ecx
    call    jit_emit_movd_xmm0_eax
    jit_template_f32_sse_bin_tail 0x5C

; f32.mul (0x94)
er_fn jit_template_f32_mul
    jit_template_pop_rcx_rax
    call    jit_emit_movd_xmm1_ecx
    call    jit_emit_movd_xmm0_eax
    jit_template_f32_sse_bin_tail 0x59

; f32.div (0x95)
er_fn jit_template_f32_div
    jit_template_pop_rcx_rax
    call    jit_emit_movd_xmm1_ecx
    call    jit_emit_movd_xmm0_eax
    jit_template_f32_sse_bin_tail 0x5E

; ==================================================================
; F64 SSE binary arithmetic templates (0xA0-0xA3)
; Each: pop rcx, pop rax, movq xmm1, movq xmm0, opsd, movq, push
; =================================================================+

; f64.add (0xA0)
er_fn jit_template_f64_add
    jit_template_pop_rcx_rax
    call    jit_emit_movq_xmm1_rcx
    call    jit_emit_movq_xmm0_rax
    jit_template_f64_sse_bin_tail 0x58

; f64.sub (0xA1)
er_fn jit_template_f64_sub
    jit_template_pop_rcx_rax
    call    jit_emit_movq_xmm1_rcx
    call    jit_emit_movq_xmm0_rax
    jit_template_f64_sse_bin_tail 0x5C

; f64.mul (0xA2)
er_fn jit_template_f64_mul
    jit_template_pop_rcx_rax
    call    jit_emit_movq_xmm1_rcx
    call    jit_emit_movq_xmm0_rax
    jit_template_f64_sse_bin_tail 0x59

; f64.div (0xA3)
er_fn jit_template_f64_div
    jit_template_pop_rcx_rax
    call    jit_emit_movq_xmm1_rcx
    call    jit_emit_movq_xmm0_rax
    jit_template_f64_sse_bin_tail 0x5E

; ==================================================================
; SSE4.1 rounding templates
; roundss/roundsd: 66 0F 3A 0A/0B <modrm> <imm8>
; imm8 = 0x08 | mode where mode: 0=nearest,1=floor,2=ceil,3=trunc
; Imm[3]=1 suppresses precision exception
; =================================================================+

; f32.ceil (0x8D)
er_fn jit_template_f32_ceil
    jit_template_f32_round 0x0A

; f32.floor (0x8E)
er_fn jit_template_f32_floor
    jit_template_f32_round 0x09

; f32.trunc (0x8F)
er_fn jit_template_f32_trunc
    jit_template_f32_round 0x0B

; f32.nearest (0x90)
er_fn jit_template_f32_nearest
    jit_template_f32_round 0x08

; f64.ceil (0x9B)
er_fn jit_template_f64_ceil
    jit_template_f64_round 0x0A

; f64.floor (0x9C)
er_fn jit_template_f64_floor
    jit_template_f64_round 0x09

; f64.trunc (0x9D)
er_fn jit_template_f64_trunc
    jit_template_f64_round 0x0B

; f64.nearest (0x9E)
er_fn jit_template_f64_nearest
    jit_template_f64_round 0x08

; ==================================================================
; Reinterpret ops — no-op on x86_64 (WASM stack holds raw bits)
; =================================================================+

; i32.reinterpret_f32 (0xBC)
er_fn jit_template_i32_reinterpret_f32
    ret

; i64.reinterpret_f64 (0xBD)
er_fn jit_template_i64_reinterpret_f64
    ret

; f32.reinterpret_i32 (0xBE)
er_fn jit_template_f32_reinterpret_i32
    ret

; f64.reinterpret_i64 (0xBF)
er_fn jit_template_f64_reinterpret_i64
    ret

; ==================================================================
; Float conversion templates (signed int → float)
; =================================================================+

; f32.convert_i32_s (0xB2) — pop i32, cvtsi2ss, push f32
er_fn jit_template_f32_convert_i32_s
    xor     ecx, ecx
    call    jit_emit_pop_reg
    call    jit_emit_movd_xmm0_eax
    mov     al, 0xF3
    mov     ch, 0x2A
    mov     cl, 0xC0
    call    jit_emit_sse_op               ; cvtsi2ss xmm0, eax
    call    jit_emit_movd_eax_xmm0
    xor     ecx, ecx
    jmp     jit_emit_push_reg

; f32.convert_i32_u (0xB3) — if value >= 2^31: subtract 2^31, convert, add 2^31
er_fn jit_template_f32_convert_i32_u
    xor     ecx, ecx
    call    jit_emit_pop_reg               ; pop rax
    call    jit_emit_test32                ; test eax, eax

    jit_template_jns_rel32_placeholder rdx

    mov     eax, 0x80000000
    call    jit_emit_sub_eax_imm32
    call    jit_emit_movd_xmm0_eax
    mov     al, 0xF3
    mov     ch, 0x2A
    mov     cl, 0xC0
    call    jit_emit_sse_op               ; cvtsi2ss xmm0, eax

    mov     al, 0xB9                       ; mov ecx, imm32
    call    jit_emit_byte
    mov     eax, 0x4F000000                ; 2^31 as f32
    call    jit_emit_dword
    call    jit_emit_movd_xmm1_ecx
    mov     al, 0xF3
    mov     ch, 0x58
    mov     cl, 0xC1
    call    jit_emit_sse_op               ; addss xmm0, xmm1

    jit_template_jmp_rel32_placeholder r8

    ; .positive:
    ; Patch jns displacement
    jit_template_patch_rel32 rdx

    mov     al, 0xF3
    mov     ch, 0x2A
    mov     cl, 0xC0
    call    jit_emit_sse_op               ; cvtsi2ss xmm0, eax

    ; .done:
    ; Patch jmp displacement
    jit_template_patch_rel32 r8

    call    jit_emit_movd_eax_xmm0
    xor     ecx, ecx
    jmp     jit_emit_push_reg

; f32.convert_i64_s (0xB4) — pop i64, cvtsi2ss xmm0, rax, push f32
er_fn jit_template_f32_convert_i64_s
    xor     ecx, ecx
    call    jit_emit_pop_reg
    mov     al, 0xF3
    mov     ch, 0x2A
    mov     cl, 0xC0
    call    jit_emit_sse64_op              ; cvtsi2ss xmm0, rax
    call    jit_emit_movd_eax_xmm0
    xor     ecx, ecx
    jmp     jit_emit_push_reg

; f32.convert_i64_u (0xB5) — unsigned i64 → f32 with subtract-2^63 trick
er_fn jit_template_f32_convert_i64_u
    xor     ecx, ecx
    call    jit_emit_pop_reg
    call    jit_emit_test64                ; test rax, rax

    jit_template_jns_rel32_placeholder rdx

    ; Unsigned path: clear sign bit, convert, add 2^63
    ; btr rax, 63
    mov     cl, 63
    call    jit_emit_btr_rax

    mov     al, 0xF3
    mov     ch, 0x2A
    mov     cl, 0xC0
    call    jit_emit_sse64_op              ; cvtsi2ss xmm0, rax

    ; Load 2^63 as f32 (0x5F000000) into xmm1 and add
    mov     al, 0xB9
    call    jit_emit_byte
    mov     eax, 0x5F000000
    call    jit_emit_dword
    call    jit_emit_movd_xmm1_ecx
    mov     al, 0xF3
    mov     ch, 0x58
    mov     cl, 0xC1
    call    jit_emit_sse_op               ; addss xmm0, xmm1

    jit_template_jmp_rel32_placeholder r8

    ; .positive:
    jit_template_patch_rel32 rdx

    mov     al, 0xF3
    mov     ch, 0x2A
    mov     cl, 0xC0
    call    jit_emit_sse64_op              ; cvtsi2ss xmm0, rax

    ; .done:
    jit_template_patch_rel32 r8

    call    jit_emit_movd_eax_xmm0
    xor     ecx, ecx
    jmp     jit_emit_push_reg

; f32.demote_f64 (0xB6) — pop f64, cvtsd2ss, push f32
er_fn jit_template_f32_demote_f64
    xor     ecx, ecx
    call    jit_emit_pop_reg
    call    jit_emit_movq_xmm0_rax
    mov     al, 0xF2
    mov     ch, 0x5A
    mov     cl, 0xC0
    call    jit_emit_sse_op               ; cvtsd2ss xmm0, xmm0
    call    jit_emit_movd_eax_xmm0
    xor     ecx, ecx
    jmp     jit_emit_push_reg

; f64.convert_i32_s (0xB7) — pop i32, cvtsi2sd, push f64
er_fn jit_template_f64_convert_i32_s
    xor     ecx, ecx
    call    jit_emit_pop_reg
    call    jit_emit_movq_xmm0_rax
    mov     al, 0xF2
    mov     ch, 0x2A
    mov     cl, 0xC0
    call    jit_emit_sse_op               ; cvtsi2sd xmm0, eax
    call    jit_emit_movq_rax_xmm0
    xor     ecx, ecx
    jmp     jit_emit_push_reg

; f64.convert_i32_u (0xB8) — unsigned i32 → f64
er_fn jit_template_f64_convert_i32_u
    xor     ecx, ecx
    call    jit_emit_pop_reg
    call    jit_emit_test32

    jit_template_jns_rel32_placeholder rdx

    mov     eax, 0x80000000
    call    jit_emit_sub_eax_imm32
    call    jit_emit_movq_xmm0_rax
    mov     al, 0xF2
    mov     ch, 0x2A
    mov     cl, 0xC0
    call    jit_emit_sse_op               ; cvtsi2sd xmm0, eax

    ; addsd xmm0, [2^31 as f64]
    ; Load 0x41E0000000000000 into xmm1
    mov     al, 0x48                       ; REX.W
    call    jit_emit_byte
    mov     al, 0xB9                       ; mov rcx, imm64
    call    jit_emit_byte
    mov     rax, 0x41E0000000000000
    call    jit_emit_qword
    call    jit_emit_movq_xmm1_rcx
    mov     al, 0xF2
    mov     ch, 0x58
    mov     cl, 0xC1
    call    jit_emit_sse_op               ; addsd xmm0, xmm1

    jit_template_jmp_rel32_placeholder r8

    ; .positive:
    jit_template_patch_rel32 rdx

    mov     al, 0xF2
    mov     ch, 0x2A
    mov     cl, 0xC0
    call    jit_emit_sse_op               ; cvtsi2sd xmm0, eax

    ; .done:
    jit_template_patch_rel32 r8

    call    jit_emit_movq_rax_xmm0
    xor     ecx, ecx
    jmp     jit_emit_push_reg

; f64.convert_i64_s (0xB9) — pop i64, cvtsi2sd rax, push f64
er_fn jit_template_f64_convert_i64_s
    xor     ecx, ecx
    call    jit_emit_pop_reg
    mov     al, 0xF2
    mov     ch, 0x2A
    mov     cl, 0xC0
    call    jit_emit_sse64_op              ; cvtsi2sd xmm0, rax
    call    jit_emit_movq_rax_xmm0
    xor     ecx, ecx
    jmp     jit_emit_push_reg

; f64.convert_i64_u (0xBA) — unsigned i64 → f64
er_fn jit_template_f64_convert_i64_u
    xor     ecx, ecx
    call    jit_emit_pop_reg
    call    jit_emit_test64

    jit_template_jns_rel32_placeholder rdx

    mov     cl, 63
    call    jit_emit_btr_rax

    mov     al, 0xF2
    mov     ch, 0x2A
    mov     cl, 0xC0
    call    jit_emit_sse64_op              ; cvtsi2sd xmm0, rax

    ; addsd xmm0, [2^63 as f64]
    mov     al, 0x48
    call    jit_emit_byte
    mov     al, 0xB9
    call    jit_emit_byte
    mov     rax, 0x43E0000000000000
    call    jit_emit_qword
    call    jit_emit_movq_xmm1_rcx
    mov     al, 0xF2
    mov     ch, 0x58
    mov     cl, 0xC1
    call    jit_emit_sse_op               ; addsd xmm0, xmm1

    jit_template_jmp_rel32_placeholder r8

    ; .positive:
    jit_template_patch_rel32 rdx

    mov     al, 0xF2
    mov     ch, 0x2A
    mov     cl, 0xC0
    call    jit_emit_sse64_op

    ; .done:
    jit_template_patch_rel32 r8

    call    jit_emit_movq_rax_xmm0
    xor     ecx, ecx
    jmp     jit_emit_push_reg

; f64.promote_f32 (0xBB) — pop f32, cvtss2sd, push f64
er_fn jit_template_f64_promote_f32
    xor     ecx, ecx
    call    jit_emit_pop_reg
    call    jit_emit_movd_xmm0_eax
    mov     al, 0xF3
    mov     ch, 0x5A
    mov     cl, 0xC0
    call    jit_emit_sse_op               ; cvtss2sd xmm0, xmm0
    call    jit_emit_movq_rax_xmm0
    xor     ecx, ecx
    jmp     jit_emit_push_reg

; ==================================================================
; f32/f64.min, max — with NaN propagation fixup
; =================================================================+

; Helper: after a SSE min/max op, check if result is NaN and fix
; Input: xmm0 = result, xmm1 = right operand
; Output: xmm0 = NaN-fixed or original result
er_fn jit_emit_fixup_nan
    ; ucomiss xmm0, xmm0 -> PF=1 if NaN
    mov     al, 0x0F
    call    jit_emit_byte
    mov     al, 0x2E
    call    jit_emit_byte
    mov     al, 0xC0
    call    jit_emit_modrm

    mov     rdx, [jit_state.code_ptr]
    add     rdx, 2

    mov     cl, 0x0B                       ; jnp condition
    xor     eax, eax
    call    jit_emit_jcc_rel32

    mov     eax, 0x7FC00000                ; canonical NaN (f32)
    call    jit_emit_push_imm32
    xor     ecx, ecx
    call    jit_emit_pop_reg
    call    jit_emit_movd_xmm0_eax

    ; Patch jnp displacement
    mov     rax, [jit_state.code_ptr]
    sub     rax, rdx
    sub     rax, 4
    mov     rdi, rdx
    jmp     jit_emit_patch_dword

; Helper: after f64 min/max, NaN fixup
er_fn jit_emit_fixup_nan_f64
    mov     al, 0x66
    call    jit_emit_byte
    mov     al, 0x0F
    call    jit_emit_byte
    mov     al, 0x2E
    call    jit_emit_byte
    mov     al, 0xC0
    call    jit_emit_modrm

    mov     rdx, [jit_state.code_ptr]
    add     rdx, 2

    mov     cl, 0x0B
    xor     eax, eax
    call    jit_emit_jcc_rel32

    mov     rax, 0x7FF8000000000000        ; canonical NaN (f64)
    mov     cl, 1
    mov     ch, 0
    xor     r8b, r8b
    xor     r9b, r9b
    call    jit_emit_rex
    mov     al, 0xB8
    call    jit_emit_byte
    mov     rax, 0x7FF8000000000000
    call    jit_emit_qword
    jit_template_push_rax
    xor     ecx, ecx
    call    jit_emit_pop_reg
    call    jit_emit_movq_xmm0_rax

    mov     rax, [jit_state.code_ptr]
    sub     rax, rdx
    sub     rax, 4
    mov     rdi, rdx
    jmp     jit_emit_patch_dword

; f32.min (0x96)
er_fn jit_template_f32_min
    jit_template_pop_rcx_rax
    call    jit_emit_movd_xmm1_ecx
    call    jit_emit_movd_xmm0_eax
    mov     al, 0xF3
    mov     ch, 0x5D
    mov     cl, 0xC1
    call    jit_emit_sse_op               ; minss xmm0, xmm1
    call    jit_emit_fixup_nan
    call    jit_emit_movd_eax_xmm0
    xor     ecx, ecx
    jmp     jit_emit_push_reg

; f32.max (0x97)
er_fn jit_template_f32_max
    jit_template_pop_rcx_rax
    call    jit_emit_movd_xmm1_ecx
    call    jit_emit_movd_xmm0_eax
    mov     al, 0xF3
    mov     ch, 0x5F
    mov     cl, 0xC1
    call    jit_emit_sse_op               ; maxss xmm0, xmm1
    call    jit_emit_fixup_nan
    call    jit_emit_movd_eax_xmm0
    xor     ecx, ecx
    jmp     jit_emit_push_reg

; f64.min (0xA4)
er_fn jit_template_f64_min
    jit_template_pop_rcx_rax
    call    jit_emit_movq_xmm1_rcx
    call    jit_emit_movq_xmm0_rax
    mov     al, 0xF2
    mov     ch, 0x5D
    mov     cl, 0xC1
    call    jit_emit_sse_op               ; minsd xmm0, xmm1
    call    jit_emit_fixup_nan_f64
    call    jit_emit_movq_rax_xmm0
    xor     ecx, ecx
    jmp     jit_emit_push_reg

; f64.max (0xA5)
er_fn jit_template_f64_max
    jit_template_pop_rcx_rax
    call    jit_emit_movq_xmm1_rcx
    call    jit_emit_movq_xmm0_rax
    mov     al, 0xF2
    mov     ch, 0x5F
    mov     cl, 0xC1
    call    jit_emit_sse_op               ; maxsd xmm0, xmm1
    call    jit_emit_fixup_nan_f64
    call    jit_emit_movq_rax_xmm0
    xor     ecx, ecx
    jmp     jit_emit_push_reg

; ==================================================================
; copysign — bit manipulation in GPR
; =================================================================+

; f32.copysign (0x98): result = abs(left) | signbit(right)
er_fn jit_template_f32_copysign
    mov     cl, 1
    call    jit_emit_pop_reg               ; pop rcx (right)
    xor     ecx, ecx
    call    jit_emit_pop_reg               ; pop rax (left)
    ; rax = left magnitude, rcx = right sign
    mov     cl, 31
    call    jit_emit_btr_eax               ; btr eax, 31 (clear sign of left)
    mov     eax, 0x80000000
    call    jit_emit_and_ecx_imm32          ; and ecx, 0x80000000 (isolate sign of right)
    call    jit_emit_or_eax_ecx             ; or eax, ecx (combine)
    xor     ecx, ecx
    jmp     jit_emit_push_reg

; f64.copysign (0xA6): result = abs(left) | signbit(right)
er_fn jit_template_f64_copysign
    mov     cl, 1
    call    jit_emit_pop_reg               ; pop rcx (right)
    xor     ecx, ecx
    call    jit_emit_pop_reg               ; pop rax (left)

    ; btr rax, 63 (clear sign of left)
    mov     cl, 63
    call    jit_emit_btr_rax

    ; isolate sign bit of right: and rcx, 0x8000000000000000
    mov     cl, 2                           ; rdx
    mov     rax, 0x8000000000000000
    call    jit_emit_mov_reg_imm64           ; mov rdx, mask
    ; and rcx, rdx (mask right to just sign bit)
    mov     cl, 1                           ; REX.W
    call    jit_emit_rex_nob
    mov     al, 0x21                        ; and r/m64, r64
    call    jit_emit_byte
    mov     al, 0xD1                        ; ModRM: reg=2(rdx), rm=1(rcx) → and rcx, rdx
    call    jit_emit_modrm

    ; or rax, rcx (combine)
    mov     cl, 1
    call    jit_emit_rex_nob
    call    jit_emit_or_eax_ecx             ; or rax, rcx
    xor     ecx, ecx
    jmp     jit_emit_push_reg

; ==================================================================
; Float load/store — reuse integer load/store templates
; =================================================================+
; These are registered by alias in the init table.

; -----------------------------------------------------------------+
; Compile refusal template — emit nothing, let orchestrator return ERROR_NOT_IMPLEMENTED
; -----------------------------------------------------------------+
er_fn jit_template_unsupported
    ret

; ------------------------------------------------------------------
; Emit: test eax, eax  (for eqz)
; -----------------------------------------------------------------+
er_fn jit_emit_test32
    mov     al, 0x85
    call    jit_emit_byte
    mov     al, 0xC0         ; test eax, eax
    call    jit_emit_modrm
    ret
