; ==================================================================
; Decoded straight-line integer fast path
; =================================================================+
exec_decoded_fast_supported:
    er_frame_push
    mov     r8, [rel exec_decoded_index]
    mov     r9, [rel exec_decoded_end]
.scan_loop:
    cmp     r8, r9
    jae     .supported
    mov     r10, r8
    imul    r10, DECODED_OP_SIZE
    movzx   eax, byte [rel decoded_ops + r10 + 8]
    cmp     al, 0x0B
    je      .scan_next
    cmp     al, 0x20
    je      .scan_next
    cmp     al, 0x21
    je      .scan_next
    cmp     al, 0x22
    je      .scan_next
    cmp     al, 0x41
    je      .scan_next
    cmp     al, 0x46
    je      .scan_next
    cmp     al, 0x47
    je      .scan_next
    cmp     al, 0x48
    je      .scan_next
    cmp     al, 0x4A
    je      .scan_next
    cmp     al, 0x4C
    je      .scan_next
    cmp     al, 0x4E
    je      .scan_next
    cmp     al, 0x6A
    je      .scan_next
    cmp     al, 0x6B
    je      .scan_next
    cmp     al, 0x6C
    je      .scan_next
    cmp     al, 0x6D
    je      .scan_next
    cmp     al, 0x6F
    je      .scan_next
    cmp     al, 0x71
    je      .scan_next
    cmp     al, 0x72
    je      .scan_next
    cmp     al, 0x73
    je      .scan_next
    cmp     al, 0x74
    je      .scan_next
    cmp     al, 0x75
    je      .scan_next
    cmp     al, 0x76
    je      .scan_next
    cmp     al, 0x77
    je      .scan_next
    cmp     al, 0x78
    je      .scan_next
    er_err  ERROR_UNSUPPORTED
    pop     rbp
    ret
.scan_next:
    inc     r8
    jmp     .scan_loop
.supported:
    er_ok
    pop     rbp
    ret

exec_decoded_round_trace_supported:
    er_frame_push_regs rbx, r12, r13, r14

    mov     r8, [rel exec_decoded_index]
    mov     r9, [rel exec_decoded_end]
    mov     rax, r9
    sub     rax, r8
    cmp     rax, 11
    jb      .unsupported
    cmp     qword [rel exec_result_count], 1
    jne     .unsupported

    imul    r8, DECODED_OP_SIZE
    lea     r8, [rel decoded_ops + r8]
    imul    r9, DECODED_OP_SIZE
    lea     r9, [rel decoded_ops + r9]

    cmp     byte [r8 + 8], 0x20        ; initial local.get
    jne     .unsupported
    mov     r12d, [r8 + 12]            ; input local
    add     r8, DECODED_OP_SIZE

    lea     rax, [r8 + DECODED_OP_SIZE * 9]
    cmp     rax, r9
    ja      .unsupported
    call    .validate_round
    jc      .unsupported
    mov     r13d, ebx                  ; multiply constant
    mov     r14d, ecx                  ; local.tee/local.get index
    mov     r12d, edx                  ; shift constant
    add     r8, DECODED_OP_SIZE * 9

.scan_rounds:
    cmp     r8, r9
    jae     .unsupported
    cmp     byte [r8 + 8], 0x0B        ; end
    je      .supported
    lea     rax, [r8 + DECODED_OP_SIZE * 9]
    cmp     rax, r9
    ja      .unsupported
    call    .validate_round
    jc      .unsupported
    cmp     ebx, r13d
    jne     .unsupported
    cmp     ecx, r14d
    jne     .unsupported
    cmp     edx, r12d
    jne     .unsupported
    add     r8, DECODED_OP_SIZE * 9
    jmp     .scan_rounds

.validate_round:
    cmp     byte [r8 + 8], 0x41
    jne     .validate_bad
    mov     ebx, [r8 + 12]
    cmp     byte [r8 + DECODED_OP_SIZE + 8], 0x6C
    jne     .validate_bad
    cmp     byte [r8 + DECODED_OP_SIZE * 2 + 8], 0x41
    jne     .validate_bad
    cmp     byte [r8 + DECODED_OP_SIZE * 3 + 8], 0x6A
    jne     .validate_bad
    cmp     byte [r8 + DECODED_OP_SIZE * 4 + 8], 0x22
    jne     .validate_bad
    mov     ecx, [r8 + DECODED_OP_SIZE * 4 + 12]
    cmp     byte [r8 + DECODED_OP_SIZE * 5 + 8], 0x41
    jne     .validate_bad
    mov     edx, [r8 + DECODED_OP_SIZE * 5 + 12]
    cmp     byte [r8 + DECODED_OP_SIZE * 6 + 8], 0x76
    jne     .validate_bad
    cmp     byte [r8 + DECODED_OP_SIZE * 7 + 8], 0x20
    jne     .validate_bad
    cmp     ecx, [r8 + DECODED_OP_SIZE * 7 + 12]
    jne     .validate_bad
    cmp     byte [r8 + DECODED_OP_SIZE * 8 + 8], 0x73
    jne     .validate_bad
    clc
    ret
.validate_bad:
    stc
    ret

.supported:
    er_ok
    jmp     .done
.unsupported:
    er_err  ERROR_UNSUPPORTED
.done:
    er_pop_ret rbp, rbx, r12, r13, r14

exec_decoded_round_trace_loop:
    er_frame_push_regs rbx, r12, r15

    mov     r8, [rel exec_decoded_index]
    imul    r8, DECODED_OP_SIZE
    lea     r8, [rel decoded_ops + r8]
    mov     r9, [rel exec_decoded_end]
    imul    r9, DECODED_OP_SIZE
    lea     r9, [rel decoded_ops + r9]

    mov     ecx, [r8 + 12]
    cmp     rcx, [rel exec_local_count]
    jae     .corrupt
    mov     ebx, [rel exec_locals + rcx * 8]
    add     r8, DECODED_OP_SIZE

    mov     rax, [r8 + DECODED_OP_SIZE * 4 + 12]
    cmp     rax, [rel exec_local_count]
    jae     .corrupt
    mov     r15d, [r8 + 12]
    mov     r12d, [r8 + DECODED_OP_SIZE * 5 + 12]

.round_loop:
    cmp     byte [r8 + 8], 0x0B
    je      .return
    mov     eax, ebx
    imul    eax, r15d
    add     eax, [r8 + DECODED_OP_SIZE * 2 + 12]
    mov     edi, eax
    mov     ecx, r12d
    shr     edi, cl
    xor     eax, edi
    mov     ebx, eax
    add     r8, DECODED_OP_SIZE * 9
    jmp     .round_loop

.return:
    mov     eax, ebx
    mov     [rel exec_result_values], rax
    er_ok
    jmp     .done
.corrupt:
    er_err  ERROR_CORRUPT
.done:
    er_pop_ret rbp, rbx, r12, r15

exec_decoded_fast_loop:
    er_frame_push_regs rbx, r12, r13, r14, r15
    xor     edx, edx                    ; cached TOS valid flag
    mov     r8, [rel exec_decoded_index]
    imul    r8, DECODED_OP_SIZE
    lea     r8, [rel decoded_ops + r8]
    mov     r9, [rel exec_decoded_end]
    imul    r9, DECODED_OP_SIZE
    lea     r9, [rel decoded_ops + r9]
.fast_loop:
    cmp     r8, r9
    jae     .fast_return
    mov     r11, r8
    movzx   eax, byte [r11 + 8]
    add     r8, DECODED_OP_SIZE

    cmp     al, 0x0B
    je      .fast_return
    cmp     al, 0x20
    je      .fast_local_get
    cmp     al, 0x21
    je      .fast_local_set
    cmp     al, 0x22
    je      .fast_local_tee
    cmp     al, 0x41
    je      .try_fast_round_fuse
    cmp     al, 0x46
    je      .fast_i32_eq
    cmp     al, 0x47
    je      .fast_i32_ne
    cmp     al, 0x48
    je      .fast_i32_lt_s
    cmp     al, 0x4A
    je      .fast_i32_gt_s
    cmp     al, 0x4C
    je      .fast_i32_le_s
    cmp     al, 0x4E
    je      .fast_i32_ge_s
    cmp     al, 0x6A
    je      .fast_i32_add
    cmp     al, 0x6B
    je      .fast_i32_sub
    cmp     al, 0x6C
    je      .fast_i32_mul
    cmp     al, 0x6D
    je      .fast_i32_div_s
    cmp     al, 0x6F
    je      .fast_i32_rem_s
    cmp     al, 0x71
    je      .fast_i32_and
    cmp     al, 0x72
    je      .fast_i32_or
    cmp     al, 0x73
    je      .fast_i32_xor
    cmp     al, 0x74
    je      .fast_i32_shl
    cmp     al, 0x75
    je      .fast_i32_shr_s
    cmp     al, 0x76
    je      .fast_i32_shr_u
    cmp     al, 0x77
    je      .fast_i32_rotl
    cmp     al, 0x78
    je      .fast_i32_rotr
    er_err  ERROR_UNSUPPORTED
    jmp     .fast_done

.try_fast_round_fuse:
    mov     r12d, [r11 + 12]        ; multiply constant
    lea     rax, [r11 + DECODED_OP_SIZE * 9]
    cmp     rax, r9
    ja      .fast_i32_const

    cmp     byte [r11 + DECODED_OP_SIZE + 8], 0x6C    ; i32.mul
    jne     .fast_i32_const

    cmp     byte [r11 + DECODED_OP_SIZE * 2 + 8], 0x41    ; i32.const
    jne     .fast_i32_const
    mov     r13d, [r11 + DECODED_OP_SIZE * 2 + 12]        ; add constant

    cmp     byte [r11 + DECODED_OP_SIZE * 3 + 8], 0x6A    ; i32.add
    jne     .fast_i32_const

    cmp     byte [r11 + DECODED_OP_SIZE * 4 + 8], 0x22    ; local.tee
    jne     .fast_i32_const
    mov     r14d, [r11 + DECODED_OP_SIZE * 4 + 12]        ; local index

    cmp     byte [r11 + DECODED_OP_SIZE * 5 + 8], 0x41    ; i32.const
    jne     .fast_i32_const
    mov     r15d, [r11 + DECODED_OP_SIZE * 5 + 12]        ; shift

    cmp     byte [r11 + DECODED_OP_SIZE * 6 + 8], 0x76    ; i32.shr_u
    jne     .fast_i32_const

    cmp     byte [r11 + DECODED_OP_SIZE * 7 + 8], 0x20    ; local.get
    jne     .fast_i32_const
    cmp     r14d, [r11 + DECODED_OP_SIZE * 7 + 12]
    jne     .fast_i32_const

    cmp     byte [r11 + DECODED_OP_SIZE * 8 + 8], 0x73    ; i32.xor
    jne     .fast_i32_const

    cmp     r14, [rel exec_local_count]
    jae     .fast_corrupt
    er_check_zero edx, .fast_round_pop_stack
    mov     eax, ebx
    jmp     .fast_round_have_input
.fast_round_pop_stack:
    mov     rax, [rel exec_stack_len]
    er_check_zero rax, .fast_underflow
    dec     rax
    mov     [rel exec_stack_len], rax
    mov     eax, [rel exec_stack + rax * 8]
.fast_round_have_input:
    imul    eax, r12d
    add     eax, r13d
    mov     [rel exec_locals + r14 * 8], rax
    mov     edi, eax
    mov     ecx, r15d
    shr     edi, cl
    xor     eax, edi
    mov     ebx, eax
    mov     edx, 1
    add     r8, DECODED_OP_SIZE * 8
    jmp     .fast_fuse_next

.fast_fuse_next:
    cmp     r8, r9
    jae     .fast_return
    mov     r11, r8
    lea     rax, [r11 + DECODED_OP_SIZE * 9]
    cmp     rax, r9
    ja      .fast_loop
    cmp     byte [r11 + 8], 0x41
    jne     .fast_loop
    cmp     r12d, [r11 + 12]
    jne     .fast_loop
    cmp     byte [r11 + DECODED_OP_SIZE + 8], 0x6C
    jne     .fast_loop
    cmp     byte [r11 + DECODED_OP_SIZE * 2 + 8], 0x41
    jne     .fast_loop
    mov     r13d, [r11 + DECODED_OP_SIZE * 2 + 12]
    cmp     byte [r11 + DECODED_OP_SIZE * 3 + 8], 0x6A
    jne     .fast_loop
    cmp     byte [r11 + DECODED_OP_SIZE * 4 + 8], 0x22
    jne     .fast_loop
    cmp     r14d, [r11 + DECODED_OP_SIZE * 4 + 12]
    jne     .fast_loop
    cmp     byte [r11 + DECODED_OP_SIZE * 5 + 8], 0x41
    jne     .fast_loop
    cmp     r15d, [r11 + DECODED_OP_SIZE * 5 + 12]
    jne     .fast_loop
    cmp     byte [r11 + DECODED_OP_SIZE * 6 + 8], 0x76
    jne     .fast_loop
    cmp     byte [r11 + DECODED_OP_SIZE * 7 + 8], 0x20
    jne     .fast_loop
    cmp     r14d, [r11 + DECODED_OP_SIZE * 7 + 12]
    jne     .fast_loop
    cmp     byte [r11 + DECODED_OP_SIZE * 8 + 8], 0x73
    jne     .fast_loop

    mov     eax, ebx
    imul    eax, r12d
    add     eax, r13d
    mov     [rel exec_locals + r14 * 8], rax
    mov     edi, eax
    mov     ecx, r15d
    shr     edi, cl
    xor     eax, edi
    mov     ebx, eax
    lea     r8, [r11 + DECODED_OP_SIZE * 9]
    jmp     .fast_fuse_next

.fast_i32_const:
    er_check_zero edx, .fast_i32_const_push
    cmp     r8, r9
    jae     .fast_i32_const_push
    movzx   eax, byte [r8 + 8]
    mov     ecx, [r11 + 12]
    cmp     al, 0x6A
    je      .fast_const_add
    cmp     al, 0x6B
    je      .fast_const_sub
    cmp     al, 0x6C
    je      .fast_const_mul
    cmp     al, 0x6D
    je      .fast_const_div_s
    cmp     al, 0x6F
    je      .fast_const_rem_s
    cmp     al, 0x71
    je      .fast_const_and
    cmp     al, 0x72
    je      .fast_const_or
    cmp     al, 0x73
    je      .fast_const_xor
    cmp     al, 0x74
    je      .fast_const_shl
    cmp     al, 0x75
    je      .fast_const_shr_s
    cmp     al, 0x76
    je      .fast_const_shr_u
    cmp     al, 0x77
    je      .fast_const_rotl
    cmp     al, 0x78
    je      .fast_const_rotr
.fast_i32_const_push:
    movsxd  rax, dword [r11 + 12]
    jmp     .fast_push_tos

.fast_const_add:
    add     ebx, ecx
    add     r8, DECODED_OP_SIZE
    jmp     .fast_loop
.fast_const_sub:
    sub     ebx, ecx
    add     r8, DECODED_OP_SIZE
    jmp     .fast_loop
.fast_const_mul:
    imul    ebx, ecx
    add     r8, DECODED_OP_SIZE
    jmp     .fast_loop
.fast_const_div_s:
    er_check_zero ecx, .fast_arithmetic_trap
    cmp     ebx, 0x80000000
    jne     .fast_const_div_s_ok
    cmp     ecx, -1
    je      .fast_arithmetic_trap
.fast_const_div_s_ok:
    mov     eax, ebx
    cdq
    idiv    ecx
    mov     ebx, eax
    mov     edx, 1
    add     r8, DECODED_OP_SIZE
    jmp     .fast_loop
.fast_const_rem_s:
    er_check_zero ecx, .fast_arithmetic_trap
    cmp     ebx, 0x80000000
    jne     .fast_const_rem_s_ok
    cmp     ecx, -1
    jne     .fast_const_rem_s_ok
    xor     ebx, ebx
    add     r8, DECODED_OP_SIZE
    jmp     .fast_loop
.fast_const_rem_s_ok:
    mov     eax, ebx
    cdq
    idiv    ecx
    mov     ebx, edx
    mov     edx, 1
    add     r8, DECODED_OP_SIZE
    jmp     .fast_loop
.fast_const_and:
    and     ebx, ecx
    add     r8, DECODED_OP_SIZE
    jmp     .fast_loop
.fast_const_or:
    or      ebx, ecx
    add     r8, DECODED_OP_SIZE
    jmp     .fast_loop
.fast_const_xor:
    xor     ebx, ecx
    add     r8, DECODED_OP_SIZE
    jmp     .fast_loop
.fast_const_shl:
    shl     ebx, cl
    add     r8, DECODED_OP_SIZE
    jmp     .fast_loop
.fast_const_shr_s:
    sar     ebx, cl
    add     r8, DECODED_OP_SIZE
    jmp     .fast_loop
.fast_const_shr_u:
    shr     ebx, cl
    add     r8, DECODED_OP_SIZE
    jmp     .fast_loop
.fast_const_rotl:
    rol     ebx, cl
    add     r8, DECODED_OP_SIZE
    jmp     .fast_loop
.fast_const_rotr:
    ror     ebx, cl
    add     r8, DECODED_OP_SIZE
    jmp     .fast_loop

.fast_local_get:
    mov     ecx, [r11 + 12]
    cmp     rcx, [rel exec_local_count]
    jae     .fast_corrupt
    er_check_zero edx, .fast_local_get_push
    cmp     r8, r9
    jae     .fast_local_get_push
    movzx   eax, byte [r8 + 8]
    cmp     al, 0x6A
    je      .fast_local_add
    cmp     al, 0x6B
    je      .fast_local_sub
    cmp     al, 0x6C
    je      .fast_local_mul
    cmp     al, 0x71
    je      .fast_local_and
    cmp     al, 0x72
    je      .fast_local_or
    cmp     al, 0x73
    je      .fast_local_xor
    cmp     al, 0x74
    je      .fast_local_shl
    cmp     al, 0x75
    je      .fast_local_shr_s
    cmp     al, 0x76
    je      .fast_local_shr_u
    cmp     al, 0x77
    je      .fast_local_rotl
    cmp     al, 0x78
    je      .fast_local_rotr
.fast_local_get_push:
    mov     rax, [rel exec_locals + rcx * 8]
    jmp     .fast_push_tos

.fast_local_add:
    mov     ecx, [rel exec_locals + rcx * 8]
    add     ebx, ecx
    add     r8, DECODED_OP_SIZE
    jmp     .fast_loop
.fast_local_sub:
    mov     ecx, [rel exec_locals + rcx * 8]
    sub     ebx, ecx
    add     r8, DECODED_OP_SIZE
    jmp     .fast_loop
.fast_local_mul:
    mov     ecx, [rel exec_locals + rcx * 8]
    imul    ebx, ecx
    add     r8, DECODED_OP_SIZE
    jmp     .fast_loop
.fast_local_and:
    mov     ecx, [rel exec_locals + rcx * 8]
    and     ebx, ecx
    add     r8, DECODED_OP_SIZE
    jmp     .fast_loop
.fast_local_or:
    mov     ecx, [rel exec_locals + rcx * 8]
    or      ebx, ecx
    add     r8, DECODED_OP_SIZE
    jmp     .fast_loop
.fast_local_xor:
    mov     ecx, [rel exec_locals + rcx * 8]
    xor     ebx, ecx
    add     r8, DECODED_OP_SIZE
    jmp     .fast_loop
.fast_local_shl:
    mov     ecx, [rel exec_locals + rcx * 8]
    shl     ebx, cl
    add     r8, DECODED_OP_SIZE
    jmp     .fast_loop
.fast_local_shr_s:
    mov     ecx, [rel exec_locals + rcx * 8]
    sar     ebx, cl
    add     r8, DECODED_OP_SIZE
    jmp     .fast_loop
.fast_local_shr_u:
    mov     ecx, [rel exec_locals + rcx * 8]
    shr     ebx, cl
    add     r8, DECODED_OP_SIZE
    jmp     .fast_loop
.fast_local_rotl:
    mov     ecx, [rel exec_locals + rcx * 8]
    rol     ebx, cl
    add     r8, DECODED_OP_SIZE
    jmp     .fast_loop
.fast_local_rotr:
    mov     ecx, [rel exec_locals + rcx * 8]
    ror     ebx, cl
    add     r8, DECODED_OP_SIZE
    jmp     .fast_loop

.fast_local_set:
    mov     ecx, [r11 + 12]
    cmp     rcx, [rel exec_local_count]
    jae     .fast_corrupt
    call    .fast_pop_tos_qword
    jc      .fast_underflow
    mov     [rel exec_locals + rcx * 8], rax
    jmp     .fast_loop

.fast_local_tee:
    mov     ecx, [r11 + 12]
    cmp     rcx, [rel exec_local_count]
    jae     .fast_corrupt
    call    .fast_peek_tos_qword
    jc      .fast_underflow
    mov     [rel exec_locals + rcx * 8], rax
    er_check_zero edx, .fast_loop
    cmp     r8, r9
    jae     .fast_loop
    cmp     byte [r8 + 8], 0x20
    jne     .fast_loop
    cmp     ecx, [r8 + 12]
    jne     .fast_loop
    mov     rax, [rel exec_stack_len]
    cmp     rax, MAX_STACK
    jae     .fast_overflow
    mov     [rel exec_stack + rax * 8], rbx
    inc     qword [rel exec_stack_len]
    add     r8, DECODED_OP_SIZE
    jmp     .fast_loop

.fast_i32_add:
    call    .fast_pop_two_i32
    jc      .fast_underflow
    add     eax, ecx
    jmp     .fast_push_tos

.fast_i32_sub:
    call    .fast_pop_two_i32
    jc      .fast_underflow
    sub     eax, ecx
    jmp     .fast_push_tos

.fast_i32_mul:
    call    .fast_pop_two_i32
    jc      .fast_underflow
    imul    eax, ecx
    jmp     .fast_push_tos

.fast_i32_eq:
    call    .fast_pop_two_i32
    jc      .fast_underflow
    cmp     eax, ecx
    sete    al
    movzx   eax, al
    jmp     .fast_push_tos

.fast_i32_ne:
    call    .fast_pop_two_i32
    jc      .fast_underflow
    cmp     eax, ecx
    setne   al
    movzx   eax, al
    jmp     .fast_push_tos

.fast_i32_lt_s:
    call    .fast_pop_two_i32
    jc      .fast_underflow
    cmp     eax, ecx
    setl    al
    movzx   eax, al
    jmp     .fast_push_tos

.fast_i32_gt_s:
    call    .fast_pop_two_i32
    jc      .fast_underflow
    cmp     eax, ecx
    setg    al
    movzx   eax, al
    jmp     .fast_push_tos

.fast_i32_le_s:
    call    .fast_pop_two_i32
    jc      .fast_underflow
    cmp     eax, ecx
    setle   al
    movzx   eax, al
    jmp     .fast_push_tos

.fast_i32_ge_s:
    call    .fast_pop_two_i32
    jc      .fast_underflow
    cmp     eax, ecx
    setge   al
    movzx   eax, al
    jmp     .fast_push_tos

.fast_i32_div_s:
    call    .fast_pop_two_i32
    jc      .fast_underflow
    er_check_zero ecx, .fast_arithmetic_trap
    cmp     eax, 0x80000000
    jne     .fast_i32_div_s_ok
    cmp     ecx, -1
    je      .fast_arithmetic_trap
.fast_i32_div_s_ok:
    cdq
    idiv    ecx
    xor     edx, edx
    jmp     .fast_push_tos

.fast_i32_rem_s:
    call    .fast_pop_two_i32
    jc      .fast_underflow
    er_check_zero ecx, .fast_arithmetic_trap
    cmp     eax, 0x80000000
    jne     .fast_i32_rem_s_ok
    cmp     ecx, -1
    jne     .fast_i32_rem_s_ok
    xor     eax, eax
    xor     edx, edx
    jmp     .fast_push_tos
.fast_i32_rem_s_ok:
    cdq
    idiv    ecx
    mov     eax, edx
    xor     edx, edx
    jmp     .fast_push_tos

.fast_i32_and:
    call    .fast_pop_two_i32
    jc      .fast_underflow
    and     eax, ecx
    jmp     .fast_push_tos

.fast_i32_or:
    call    .fast_pop_two_i32
    jc      .fast_underflow
    or      eax, ecx
    jmp     .fast_push_tos

.fast_i32_xor:
    call    .fast_pop_two_i32
    jc      .fast_underflow
    xor     eax, ecx
    jmp     .fast_push_tos

.fast_i32_shl:
    call    .fast_pop_two_i32
    jc      .fast_underflow
    shl     eax, cl
    jmp     .fast_push_tos

.fast_i32_shr_s:
    call    .fast_pop_two_i32
    jc      .fast_underflow
    sar     eax, cl
    jmp     .fast_push_tos

.fast_i32_shr_u:
    call    .fast_pop_two_i32
    jc      .fast_underflow
    shr     eax, cl
    jmp     .fast_push_tos

.fast_i32_rotl:
    call    .fast_pop_two_i32
    jc      .fast_underflow
    rol     eax, cl
    jmp     .fast_push_tos

.fast_i32_rotr:
    call    .fast_pop_two_i32
    jc      .fast_underflow
    ror     eax, cl
    jmp     .fast_push_tos

.fast_pop_two_i32:
    er_check_zero edx, .pop_two_from_stack
    mov     rax, [rel exec_stack_len]
    er_check_zero rax, .pop_two_underflow
    dec     rax
    mov     [rel exec_stack_len], rax
    mov     ecx, ebx
    mov     eax, [rel exec_stack + rax * 8]
    xor     edx, edx
    clc
    ret
.pop_two_from_stack:
    mov     rax, [rel exec_stack_len]
    cmp     rax, 2
    jb      .pop_two_underflow
    sub     rax, 2
    mov     [rel exec_stack_len], rax
    mov     ecx, [rel exec_stack + rax * 8 + 8]
    mov     eax, [rel exec_stack + rax * 8]
    clc
    ret
.pop_two_underflow:
    stc
    ret

.fast_pop_tos_qword:
    er_check_zero edx, .pop_tos_stack
    mov     rax, rbx
    xor     edx, edx
    clc
    ret
.pop_tos_stack:
    mov     rax, [rel exec_stack_len]
    er_check_zero rax, .pop_tos_underflow
    dec     rax
    mov     [rel exec_stack_len], rax
    mov     rax, [rel exec_stack + rax * 8]
    clc
    ret
.pop_tos_underflow:
    stc
    ret

.fast_pop_tos_i32:
    call    .fast_pop_tos_qword
    ret

.fast_peek_tos_qword:
    er_check_zero edx, .peek_tos_stack
    mov     rax, rbx
    clc
    ret
.peek_tos_stack:
    mov     rax, [rel exec_stack_len]
    er_check_zero rax, .pop_tos_underflow
    dec     rax
    mov     rax, [rel exec_stack + rax * 8]
    clc
    ret

.fast_push_tos:
    er_check_zero edx, .store_tos
    mov     rcx, [rel exec_stack_len]
    cmp     rcx, MAX_STACK
    jae     .fast_overflow
    mov     [rel exec_stack + rcx * 8], rbx
    inc     qword [rel exec_stack_len]
.store_tos:
    mov     rbx, rax
    mov     edx, 1
    jmp     .fast_loop

.fast_return:
    mov     rcx, [rel exec_result_count]
    er_check_zero rcx, .fast_ok
    cmp     rcx, 1
    jne     .fast_unsupported
    call    .fast_pop_tos_qword
    jc      .fast_underflow
    mov     [rel exec_result_values], rax
.fast_ok:
    er_ok
    jmp     .fast_done
.fast_corrupt:
    er_err  ERROR_CORRUPT
    jmp     .fast_done
.fast_underflow:
    er_err  ERROR_STACK_UNDERFLOW
    jmp     .fast_done
.fast_overflow:
    er_err  ERROR_STACK_OVERFLOW
    jmp     .fast_done
.fast_arithmetic_trap:
    er_err  ERROR_ARITHMETIC_TRAP
    jmp     .fast_done
.fast_unsupported:
    er_err  ERROR_UNSUPPORTED
.fast_done:
    er_pop_ret rbp, rbx, r12, r13, r14, r15

; ==================================================================
; Main instruction dispatch loop
; =================================================================+
exec_dispatch_loop:
    er_frame_push

    ; One-time opcode table population
    cmp     byte [rel opcode_table_populated], 0
    jne     .dispatch_next
    inc     byte [rel opcode_table_populated]

    ; Default all 256 entries to unsupported_error
    lea     rax, [rel .unsupported_error]
    mov     ecx, 256
    lea     rdx, [rel opcode_table]
.fill_table:
    mov     [rdx + rcx * 8 - 8], rax
    dec     ecx
    jnz     .fill_table

    ; Set handler for each implemented opcode
    %macro _init_op 2
        lea     rax, [rel .%2]
        mov     [rel opcode_table + %1 * 8], rax
    %endm

    _init_op 0x00, op_unreachable
    _init_op 0x01, op_nop
    _init_op 0x02, op_block
    _init_op 0x03, op_loop
    _init_op 0x04, op_if
    _init_op 0x05, op_else
    _init_op 0x0b, op_end
    _init_op 0x0c, op_br
    _init_op 0x0d, op_br_if
    _init_op 0x0e, op_br_table
    _init_op 0x0f, op_return
    _init_op 0x10, op_call
    _init_op 0x11, op_call_indirect
    _init_op 0x1a, op_drop
    _init_op 0x1b, op_select
    _init_op 0x1c, op_select_typed
    _init_op 0x20, op_local_get
    _init_op 0x21, op_local_set
    _init_op 0x22, op_local_tee
    _init_op 0x23, op_global_get
    _init_op 0x24, op_global_set
    _init_op 0x25, op_table_get
    _init_op 0x26, op_table_set
    _init_op 0x28, op_i32_load
    _init_op 0x29, op_i64_load
    _init_op 0x2a, op_f32_load
    _init_op 0x2b, op_f64_load
    _init_op 0x2c, op_i32_load8_s
    _init_op 0x2d, op_i32_load8_u
    _init_op 0x2e, op_i32_load16_s
    _init_op 0x2f, op_i32_load16_u
    _init_op 0x30, op_i64_load8_s
    _init_op 0x31, op_i64_load8_u
    _init_op 0x32, op_i64_load16_s
    _init_op 0x33, op_i64_load16_u
    _init_op 0x34, op_i64_load32_s
    _init_op 0x35, op_i64_load32_u
    _init_op 0x36, op_i32_store
    _init_op 0x37, op_i64_store
    _init_op 0x38, op_f32_store
    _init_op 0x39, op_f64_store
    _init_op 0x3a, op_i32_store8
    _init_op 0x3b, op_i32_store16
    _init_op 0x3c, op_i64_store8
    _init_op 0x3d, op_i64_store16
    _init_op 0x3e, op_i64_store32
    _init_op 0x3f, op_memory_size
    _init_op 0x40, op_memory_grow
    _init_op 0x41, op_i32_const
    _init_op 0x42, op_i64_const
    _init_op 0x43, op_f32_const
    _init_op 0x44, op_f64_const
    _init_op 0x45, op_i32_eqz
    _init_op 0x46, op_i32_eq
    _init_op 0x47, op_i32_ne
    _init_op 0x48, op_i32_lt_s
    _init_op 0x49, op_i32_lt_u
    _init_op 0x4a, op_i32_gt_s
    _init_op 0x4b, op_i32_gt_u
    _init_op 0x4c, op_i32_le_s
    _init_op 0x4d, op_i32_le_u
    _init_op 0x4e, op_i32_ge_s
    _init_op 0x4f, op_i32_ge_u
    _init_op 0x50, op_i64_eqz
    _init_op 0x51, op_i64_eq
    _init_op 0x52, op_i64_ne
    _init_op 0x53, op_i64_lt_s
    _init_op 0x54, op_i64_lt_u
    _init_op 0x55, op_i64_gt_s
    _init_op 0x56, op_i64_gt_u
    _init_op 0x57, op_i64_le_s
    _init_op 0x58, op_i64_le_u
    _init_op 0x59, op_i64_ge_s
    _init_op 0x5a, op_i64_ge_u
    _init_op 0x5b, op_f32_eq
    _init_op 0x5c, op_f32_ne
    _init_op 0x5d, op_f32_lt
    _init_op 0x5e, op_f32_gt
    _init_op 0x5f, op_f32_le
    _init_op 0x60, op_f32_ge
    _init_op 0x61, op_f64_eq
    _init_op 0x62, op_f64_ne
    _init_op 0x63, op_f64_lt
    _init_op 0x64, op_f64_gt
    _init_op 0x65, op_f64_le
    _init_op 0x66, op_f64_ge
    _init_op 0x67, op_i32_clz
    _init_op 0x68, op_i32_ctz
    _init_op 0x69, op_i32_popcnt
    _init_op 0x6a, op_i32_add
    _init_op 0x6b, op_i32_sub
    _init_op 0x6c, op_i32_mul
    _init_op 0x6d, op_i32_div_s
    _init_op 0x6e, op_i32_div_u
    _init_op 0x6f, op_i32_rem_s
    _init_op 0x70, op_i32_rem_u
    _init_op 0x71, op_i32_and
    _init_op 0x72, op_i32_or
    _init_op 0x73, op_i32_xor
    _init_op 0x74, op_i32_shl
    _init_op 0x75, op_i32_shr_s
    _init_op 0x76, op_i32_shr_u
    _init_op 0x77, op_i32_rotl
    _init_op 0x78, op_i32_rotr
    _init_op 0x79, op_i64_clz
    _init_op 0x7a, op_i64_ctz
    _init_op 0x7b, op_i64_popcnt
    _init_op 0x7c, op_i64_add
    _init_op 0x7d, op_i64_sub
    _init_op 0x7e, op_i64_mul
    _init_op 0x7f, op_i64_div_s
    _init_op 0x80, op_i64_div_u
    _init_op 0x81, op_i64_rem_s
    _init_op 0x82, op_i64_rem_u
    _init_op 0x83, op_i64_and
    _init_op 0x84, op_i64_or
    _init_op 0x85, op_i64_xor
    _init_op 0x86, op_i64_shl
    _init_op 0x87, op_i64_shr_s
    _init_op 0x88, op_i64_shr_u
    _init_op 0x89, op_i64_rotl
    _init_op 0x8a, op_i64_rotr
    _init_op 0x8b, op_f32_abs
    _init_op 0x8c, op_f32_neg
    _init_op 0x8d, op_f32_ceil
    _init_op 0x8e, op_f32_floor
    _init_op 0x8f, op_f32_trunc
    _init_op 0x90, op_f32_nearest
    _init_op 0x91, op_f32_sqrt
    _init_op 0x92, op_f32_add
    _init_op 0x93, op_f32_sub
    _init_op 0x94, op_f32_mul
    _init_op 0x95, op_f32_div
    _init_op 0x96, op_f32_min
    _init_op 0x97, op_f32_max
    _init_op 0x98, op_f32_copysign
    _init_op 0x99, op_f64_abs
    _init_op 0x9a, op_f64_neg
    _init_op 0x9b, op_f64_ceil
    _init_op 0x9c, op_f64_floor
    _init_op 0x9d, op_f64_trunc
    _init_op 0x9e, op_f64_nearest
    _init_op 0x9f, op_f64_sqrt
    _init_op 0xa0, op_f64_add
    _init_op 0xa1, op_f64_sub
    _init_op 0xa2, op_f64_mul
    _init_op 0xa3, op_f64_div
    _init_op 0xa4, op_f64_min
    _init_op 0xa5, op_f64_max
    _init_op 0xa6, op_f64_copysign
    _init_op 0xa7, op_i32_wrap_i64
    _init_op 0xa8, op_i32_trunc_f32_s
    _init_op 0xa9, op_i32_trunc_f32_u
    _init_op 0xaa, op_i32_trunc_f64_s
    _init_op 0xab, op_i32_trunc_f64_u
    _init_op 0xac, op_i64_extend_i32_s
    _init_op 0xad, op_i64_extend_i32_u
    _init_op 0xae, op_i64_trunc_f32_s
    _init_op 0xaf, op_i64_trunc_f32_u
    _init_op 0xb0, op_i64_trunc_f64_s
    _init_op 0xb1, op_i64_trunc_f64_u
    _init_op 0xb2, op_f32_convert_i32_s
    _init_op 0xb3, op_f32_convert_i32_u
    _init_op 0xb4, op_f32_convert_i64_s
    _init_op 0xb5, op_f32_convert_i64_u
    _init_op 0xb6, op_f32_demote_f64
    _init_op 0xb7, op_f64_convert_i32_s
    _init_op 0xb8, op_f64_convert_i32_u
    _init_op 0xb9, op_f64_convert_i64_s
    _init_op 0xba, op_f64_convert_i64_u
    _init_op 0xbb, op_f64_promote_f32
    _init_op 0xbc, op_i32_reinterpret_f32
    _init_op 0xbd, op_i64_reinterpret_f64
    _init_op 0xbe, op_f32_reinterpret_i32
    _init_op 0xbf, op_f64_reinterpret_i64
    _init_op 0xc0, op_i32_extend8_s
    _init_op 0xc1, op_i32_extend16_s
    _init_op 0xc2, op_i64_extend8_s
    _init_op 0xc3, op_i64_extend16_s
    _init_op 0xc4, op_i64_extend32_s
    _init_op 0xd0, op_ref_null
    _init_op 0xd1, op_ref_is_null
    _init_op 0xd2, op_ref_func

.dispatch_next:
    ; Check reader done: reader_offset >= body_len
    mov     rax, [exec_reader_offset]
    cmp     rax, [exec_code_body_len]
    jae     .done

    ; Read opcode byte directly
    call    reader_read_byte
    jc      .corrupt_error

    movzx   ebx, al          ; opcode byte in bl

    ; Check for extended prefix (0xfc)
    cmp     bl, WASM_EXTENDED_PREFIX
    je      .extended_opcode

    ; Dispatch through opcode table (O(1), was O(n) ladder)
    jmp     [rel opcode_table + rbx * 8]
