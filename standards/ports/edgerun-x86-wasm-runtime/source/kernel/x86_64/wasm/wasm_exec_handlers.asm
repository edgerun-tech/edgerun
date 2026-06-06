; ==================================================================
; Hot opcode handlers
; =================================================================+

.op_i32_const:
    ; Read i32 const value via LEB128
    wasm_exec_reader_ptr
    call    er_wasm_read_leb_i32
    er_check_nonzero rdx, .corrupt_error
    ; Update reader offset (LEB consumed bytes in rsi - body_ptr)
    wasm_exec_save_reader_offset
    call    exec_stack_push
    jc      .overflow_error
    jmp     .dispatch_next

.op_i64_const:
    wasm_exec_reader_ptr
    call    er_wasm_read_leb_i64
    er_check_nonzero rdx, .corrupt_error
    wasm_exec_save_reader_offset
    call    exec_stack_push
    jc      .overflow_error
    jmp     .dispatch_next

.op_local_get:
    ; Read local index (LEB128)
    wasm_exec_reader_ptr
    call    er_wasm_read_leb_u32
    er_check_nonzero rdx, .corrupt_error
    ; Update reader
    wasm_exec_save_reader_offset
    ; Check bounds
    cmp     rax, [exec_local_count]
    jae     .corrupt_error
    ; Read local
    mov     rax, [exec_locals + rax * 8]
    call    exec_stack_push
    jc      .overflow_error
    jmp     .dispatch_next

.op_local_set:
    wasm_exec_reader_ptr
    call    er_wasm_read_leb_u32
    er_check_nonzero rdx, .corrupt_error
    wasm_exec_save_reader_offset
    cmp     rax, [exec_local_count]
    jae     .corrupt_error
    push    rax        ; save index
    call    exec_stack_pop
    jc      .underflow_error
    pop     rcx
    mov     [exec_locals + rcx * 8], rax
    jmp     .dispatch_next

.op_local_tee:
    wasm_exec_reader_ptr
    call    er_wasm_read_leb_u32
    er_check_nonzero rdx, .corrupt_error
    wasm_exec_save_reader_offset
    cmp     rax, [exec_local_count]
    jae     .corrupt_error
    push    rax
    call    exec_stack_peek
    jc      .underflow_error
    pop     rcx
    mov     [exec_locals + rcx * 8], rax
    jmp     .dispatch_next

.op_global_get:
    wasm_exec_reader_ptr
    call    er_wasm_read_leb_u32
    er_check_nonzero rdx, .corrupt_error
    wasm_exec_save_reader_offset
    cmp     rax, [global_count]
    jae     .corrupt_error
    push    r10
    mov     r10, rax
    imul    r10, GLOBAL_SIZE
    mov     rax, [globals_buf + r10 + 8]  ; value_data
    pop     r10
    call    exec_stack_push
    jc      .overflow_error
    jmp     .dispatch_next

.op_global_set:
    wasm_exec_reader_ptr
    call    er_wasm_read_leb_u32
    er_check_nonzero rdx, .corrupt_error
    wasm_exec_save_reader_offset
    cmp     rax, [global_count]
    jae     .corrupt_error
    push    rax
    call    exec_stack_pop
    jc      .underflow_error
    pop     rcx
    push    r10
    mov     r10, rcx
    imul    r10, GLOBAL_SIZE
    mov     [globals_buf + r10 + 8], rax  ; value_data
    pop     r10
    jmp     .dispatch_next

.op_drop:
    call    exec_stack_pop
    jc      .underflow_error
    jmp     .dispatch_next

.op_select:
    call    exec_stack_pop  ; condition
    jc      .underflow_error
    mov     rcx, rax
    call    exec_stack_pop  ; false_val
    jc      .underflow_error
    mov     rdx, rax
    call    exec_stack_pop  ; true_val
    jc      .underflow_error
    test    ecx, ecx
    cmovz   rax, rdx
    call    exec_stack_push
    jc      .overflow_error
    jmp     .dispatch_next

.op_select_typed:
    ; Skip type immediates (read_byte for type count, then read_value_type)
    call    reader_read_byte   ; type_count (must be 1)
    jc      .corrupt_error
    cmp     al, 1
    jne     .unsupported_error
    call    reader_read_byte   ; value type byte
    jc      .corrupt_error
    ; Same as select
    call    exec_stack_pop  ; condition
    jc      .underflow_error
    mov     rcx, rax
    call    exec_stack_pop  ; false_val
    jc      .underflow_error
    mov     rdx, rax
    call    exec_stack_pop  ; true_val
    jc      .underflow_error
    test    ecx, ecx
    cmovz   rax, rdx
    call    exec_stack_push
    jc      .overflow_error
    jmp     .dispatch_next

.op_nop:
    jmp     .dispatch_next

.op_unreachable:
    er_err  ERROR_TRAP
    jmp     .error_return

.op_end:
    ; End of block/function
    mov     rax, [exec_control_len]
    er_check_zero rax, .function_return
    ; Pop control frame
    dec     rax
    mov     [exec_control_len], rax
    jmp     .dispatch_next

.op_return:
    jmp     .function_return

.op_block:
    ; Read block type byte (skip it for now)
    call    reader_read_byte
    jc      .corrupt_error
    ; Push control frame: kind=BLOCK, start=reader_offset
    push    qword [exec_reader_offset]  ; start offset
    push    CONTROL_BLOCK
    call    exec_control_push
    jmp     .dispatch_next

.op_loop:
    call    reader_read_byte
    jc      .corrupt_error
    push    qword [exec_reader_offset]
    push    CONTROL_LOOP
    call    exec_control_push
    jmp     .dispatch_next

.op_if:
    call    reader_read_byte
    jc      .corrupt_error
    call    exec_stack_pop   ; condition
    jc      .underflow_error
    er_check_nonzero eax, .if_taken
    ; Skip to else or end
    ; Need to balance block/loop/if/else/end
    ; Simplified: scan forward counting nested blocks
    xor     r15d, r15d       ; depth
.skip_if_loop:
    call    reader_read_byte
    jc      .corrupt_error
    movzx   ebx, al
    cmp     bl, 0x02         ; block
    je      .skip_if_block
    cmp     bl, 0x03         ; loop
    je      .skip_if_block
    cmp     bl, 0x04         ; if
    je      .skip_if_block
    cmp     bl, 0x05         ; else
    je      .skip_if_else
    cmp     bl, 0x0b         ; end
    je      .skip_if_end
    ; Regular opcode - skip immediates
    call    .skip_opcode_immediates
    jmp     .skip_if_loop
.skip_if_block:
    inc     r15d
    call    reader_read_byte    ; skip block type
    jc      .corrupt_error
    jmp     .skip_if_loop
.skip_if_else:
    er_check_zero r15d, .if_else_found
    dec     r15d
    jmp     .skip_if_loop
.skip_if_end:
    er_check_zero r15d, .if_end_found
    dec     r15d
    jmp     .skip_if_loop
.if_else_found:
    push    qword [exec_reader_offset]
    push    CONTROL_IF_ELSE
    call    exec_control_push
    jmp     .dispatch_next
.if_end_found:
    ; No else branch. The scanner consumed the matching end opcode, so
    ; continue with the instruction after the skipped if body.
    jmp     .dispatch_next
.if_taken:
    push    qword [exec_reader_offset]
    push    CONTROL_IF_THEN
    call    exec_control_push
    jmp     .dispatch_next

.op_else:
    ; Check control stack top is if_then
    mov     rax, [exec_control_len]
    er_check_zero rax, .corrupt_error
    ; The then arm already ran, so pop the if frame and skip the else arm.
    dec     rax
    mov     [exec_control_len], rax
    xor     r15d, r15d
.skip_else_loop:
    call    reader_read_byte
    jc      .corrupt_error
    movzx   ebx, al
    cmp     bl, 0x02         ; block
    je      .skip_else_block
    cmp     bl, 0x03         ; loop
    je      .skip_else_block
    cmp     bl, 0x04         ; if
    je      .skip_else_block
    cmp     bl, 0x0b         ; end
    je      .skip_else_end
    call    .skip_opcode_immediates
    jmp     .skip_else_loop
.skip_else_block:
    inc     r15d
    call    reader_read_byte
    jc      .corrupt_error
    jmp     .skip_else_loop
.skip_else_end:
    er_check_zero r15d, .dispatch_next
    dec     r15d
    jmp     .skip_else_loop

.op_br:
    ; Read branch depth
    wasm_exec_reader_ptr
    call    er_wasm_read_leb_u32
    er_check_nonzero rdx, .corrupt_error
    wasm_exec_save_reader_offset
    ; Branch to control
    mov     rdi, rax        ; depth
    call    exec_branch_to_control
    er_check_nonzero rdx, .error_return
    jmp     .dispatch_next

.op_br_if:
    wasm_exec_reader_ptr
    call    er_wasm_read_leb_u32
    er_check_nonzero rdx, .corrupt_error
    wasm_exec_save_reader_offset
    push    rax             ; save depth
    call    exec_stack_pop  ; condition
    jc      .underflow_error
    pop     rcx
    er_check_zero eax, .dispatch_next
    mov     rdi, rcx
    call    exec_branch_to_control
    er_check_nonzero rdx, .error_return
    jmp     .dispatch_next

.op_call:
    ; Read function index
    wasm_exec_reader_ptr
    call    er_wasm_read_leb_u32
    er_check_nonzero rdx, .corrupt_error
    wasm_exec_save_reader_offset
    mov     r15, rax                ; save function_index

    ; Get param count and result count from function type
    mov     rdi, r15
    call    er_wasm_type_index_for_function
    er_check_nonzero rdx, .error_return
    imul    rax, FUNC_TYPE_SIZE
    mov     r14, [types_buf + rax + FUNC_TYPE_PARAM_COUNT_OFF]   ; param_count
    mov     r13, [types_buf + rax + FUNC_TYPE_RESULT_COUNT_OFF]  ; result_count

    ; Pop args from eval stack into buffer (reversed order)
    mov     r12, r14                ; loop counter = param_count
    cmp     r12, 8
    ja      .unsupported_error
    sub     rsp, 64                 ; buffer for up to 8 * 8 = 64 bytes
    lea     rbx, [rsp]
.pop_args:
    er_check_zero r12, .args_popped
    dec     r12
    call    exec_stack_pop
    jc      .underflow_error
    mov     [rbx + r12 * 8], rax
    jmp     .pop_args
.args_popped:

    ; Call function
    mov     rdi, r15                ; function_index
    mov     rsi, rbx                ; args ptr
    mov     rdx, r14                ; args count
    call    er_fn_exec
    add     rsp, 64                 ; reclaim buffer
    er_check_nonzero rdx, .error_return

    ; Push return value(s) onto eval stack
    mov     rcx, r13                ; result_count
    xor     r12, r12
.push_results:
    cmp     r12, rcx
    jae     .dispatch_next
    mov     rax, [exec_result_values + r12 * 8]
    push    rax
    call    exec_stack_push
    pop     rax
    inc     r12
    jmp     .push_results

.op_call_indirect:
    wasm_exec_reader_ptr
    call    er_wasm_read_leb_u32    ; type_index
    er_check_nonzero rdx, .corrupt_error
    push    rax
    wasm_exec_reader_ptr
    call    er_wasm_read_leb_u32    ; table_index
    er_check_nonzero rdx, .corrupt_error
    er_check_nonzero eax, .memory_trap
    wasm_exec_save_reader_offset
    pop     r15                     ; type_index -> r15
    ; Pop function index from stack
    call    exec_stack_pop
    jc      .underflow_error
    mov     r14, rax                ; save function_index

    ; Get param count and result count from the type_index already read
    mov     rax, r15
    imul    rax, FUNC_TYPE_SIZE
    mov     r12, [types_buf + rax + FUNC_TYPE_PARAM_COUNT_OFF]   ; param_count
    mov     r13, [types_buf + rax + FUNC_TYPE_RESULT_COUNT_OFF]  ; result_count

    ; Pop args from eval stack into buffer (reversed order)
    mov     rbx, r12                ; loop counter = param_count
    cmp     rbx, 8
    ja      .unsupported_error
    sub     rsp, 64                 ; buffer for up to 8 * 8 = 64 bytes
    lea     r11, [rsp]
.pop_args_indirect:
    er_check_zero rbx, .args_popped_indirect
    dec     rbx
    call    exec_stack_pop
    jc      .underflow_error
    mov     [r11 + rbx * 8], rax
    jmp     .pop_args_indirect
.args_popped_indirect:

    ; Call function
    mov     rdi, r14                ; function_index
    mov     rsi, r11                ; args ptr
    mov     rdx, r12                ; args count
    call    er_fn_exec
    add     rsp, 64                 ; reclaim buffer
    er_check_nonzero rdx, .error_return

    ; Push return value(s) onto eval stack
    mov     rcx, r13                ; result_count
    xor     r12, r12
.push_results_indirect:
    cmp     r12, rcx
    jae     .dispatch_next
    mov     rax, [exec_result_values + r12 * 8]
    push    rax
    call    exec_stack_push
    pop     rax
    inc     r12
    jmp     .push_results_indirect

.op_br_table:
    ; Read target count
    wasm_exec_reader_ptr
    call    er_wasm_read_leb_u32
    er_check_nonzero rdx, .corrupt_error
    wasm_exec_save_reader_offset
    mov     r15d, eax       ; target_count
    call    exec_stack_pop
    jc      .underflow_error
    mov     r14d, eax       ; selector
    xor     r13d, r13d      ; target_index
    xor     r12d, r12d      ; selected branch depth
.br_table_loop:
    cmp     r13d, r15d
    jae     .br_table_default
    wasm_exec_reader_ptr
    call    er_wasm_read_leb_u32
    er_check_nonzero rdx, .corrupt_error
    wasm_exec_save_reader_offset
    cmp     r13d, r14d
    jne     .br_table_next
    mov     r12d, eax       ; found matching target
.br_table_next:
    inc     r13d
    jmp     .br_table_loop
.br_table_default:
    ; Read default target
    wasm_exec_reader_ptr
    call    er_wasm_read_leb_u32
    er_check_nonzero rdx, .corrupt_error
    wasm_exec_save_reader_offset
    cmp     r14d, r15d
    jb      .br_table_branch
    mov     r12d, eax       ; selector outside table uses default target
.br_table_branch:
    mov     rdi, r12        ; branch depth
    call    exec_branch_to_control
    er_check_nonzero rdx, .error_return
    jmp     .dispatch_next

; ==================================================================
; Control frame push/pop/branch
; =================================================================+

; Push control frame: expects values on stack (kind=8 bytes, start=8 bytes)
; Used as: push kind; push start; call exec_control_push

; ==================================================================
; Integer i32 binary ops
; =================================================================+
.op_i32_add:
    _wasm_binop e, add

.op_i32_sub:
    _wasm_binop e, sub

.op_i32_mul:
    _wasm_binop e, imul

.op_i32_div_s:
    call    exec_stack_pop
    jc      .underflow_error
    mov     ecx, eax
    call    exec_stack_pop
    jc      .underflow_error
    er_check_zero ecx, .arithmetic_trap
    cmp     eax, 0x80000000
    jne     .div_s_ok
    cmp     ecx, -1
    jne     .div_s_ok
    ; Overflow: INT_MIN / -1
    er_err  ERROR_ARITHMETIC_TRAP
    jmp     .error_return
.div_s_ok:
    cdq
    idiv    ecx
    call    exec_stack_push
    jc      .overflow_error
    jmp     .dispatch_next

.op_i32_div_u:
    call    exec_stack_pop
    jc      .underflow_error
    mov     ecx, eax
    call    exec_stack_pop
    jc      .underflow_error
    er_check_zero ecx, .arithmetic_trap
    er_ok
    div     ecx
    call    exec_stack_push
    jc      .overflow_error
    jmp     .dispatch_next

.op_i32_rem_s:
    call    exec_stack_pop
    jc      .underflow_error
    mov     ecx, eax
    call    exec_stack_pop
    jc      .underflow_error
    er_check_zero ecx, .arithmetic_trap
    mov     r8d, eax        ; save dividend
    ; If dividend was INT_MIN and divisor was -1, remainder = 0
    cmp     r8d, 0x80000000
    jne     .rem_s_done
    cmp     ecx, -1
    jne     .rem_s_done
    xor     eax, eax
    jmp     .rem_s_push
.rem_s_done:
    cdq
    idiv    ecx
    mov     eax, edx        ; remainder
.rem_s_push:
    call    exec_stack_push
    jc      .overflow_error
    jmp     .dispatch_next

.op_i32_rem_u:
    call    exec_stack_pop
    jc      .underflow_error
    mov     ecx, eax
    call    exec_stack_pop
    jc      .underflow_error
    er_check_zero ecx, .arithmetic_trap
    er_ok
    div     ecx
    mov     eax, edx
    call    exec_stack_push
    jc      .overflow_error
    jmp     .dispatch_next

.op_i32_and:
    _wasm_binop e, and

.op_i32_or:
    _wasm_binop e, or

.op_i32_xor:
    _wasm_binop e, xor

.op_i32_shl:
    _wasm_shiftop e, shl

.op_i32_shr_s:
    _wasm_shiftop e, sar

.op_i32_shr_u:
    _wasm_shiftop e, shr

.op_i32_rotl:
    _wasm_shiftop e, rol

.op_i32_rotr:
    _wasm_shiftop e, ror

.op_i32_clz:
    call    exec_stack_pop
    jc      .underflow_error
    ; Count leading zeros (eax -> eax)
    bsr     ecx, eax
    jnz     .clz_found
    mov     eax, 32
    jmp     .clz_done
.clz_found:
    mov     eax, 31
    sub     eax, ecx
.clz_done:
    call    exec_stack_push
    jc      .overflow_error
    jmp     .dispatch_next

.op_i32_ctz:
    call    exec_stack_pop
    jc      .underflow_error
    bsf     ecx, eax
    jnz     .ctz_found
    mov     eax, 32
    jmp     .ctz_done
.ctz_found:
    mov     eax, ecx
.ctz_done:
    call    exec_stack_push
    jc      .overflow_error
    jmp     .dispatch_next

.op_i32_popcnt:
    _wasm_unop e, popcnt

; ==================================================================
; Integer i32 comparison ops
; =================================================================+
.op_i32_eqz:
    _wasm_eqz e

.op_i32_eq:
    _wasm_cmpop e, sete

.op_i32_ne:
    _wasm_cmpop e, setne

.op_i32_lt_s:
    _wasm_cmpop e, setl

.op_i32_lt_u:
    _wasm_cmpop e, setb

.op_i32_gt_s:
    _wasm_cmpop e, setg

.op_i32_gt_u:
    _wasm_cmpop e, seta

.op_i32_le_s:
    _wasm_cmpop e, setle

.op_i32_le_u:
    _wasm_cmpop e, setbe

.op_i32_ge_s:
    _wasm_cmpop e, setge

.op_i32_ge_u:
    _wasm_cmpop e, setae

; ==================================================================
; Integer i64 binary ops
; =================================================================+
.op_i64_add:
    _wasm_binop r, add

.op_i64_sub:
    _wasm_binop r, sub

.op_i64_mul:
    _wasm_binop r, imul

.op_i64_div_s:
    call    exec_stack_pop
    jc      .underflow_error
    mov     rcx, rax
    call    exec_stack_pop
    jc      .underflow_error
    er_check_zero rcx, .arithmetic_trap
    mov     rdx, rax
    mov     rax, 0x8000000000000000
    cmp     rdx, rax
    jne     .div_s64_ok
    cmp     rcx, -1
    jne     .div_s64_ok
    ; Overflow: INT64_MIN / -1
    er_err  ERROR_ARITHMETIC_TRAP
    jmp     .error_return
.div_s64_ok:
    mov     rax, rdx
    cqo
    idiv    rcx
    call    exec_stack_push
    jc      .overflow_error
    jmp     .dispatch_next

.op_i64_div_u:
    call    exec_stack_pop
    jc      .underflow_error
    mov     rcx, rax
    call    exec_stack_pop
    jc      .underflow_error
    er_check_zero rcx, .arithmetic_trap
    xor     rdx, rdx
    div     rcx
    call    exec_stack_push
    jc      .overflow_error
    jmp     .dispatch_next

.op_i64_rem_s:
    call    exec_stack_pop
    jc      .underflow_error
    mov     rcx, rax
    call    exec_stack_pop
    jc      .underflow_error
    er_check_zero rcx, .arithmetic_trap
    mov     r8, rax
    cqo
    idiv    rcx
    mov     rax, rdx
    mov     rdx, 0x8000000000000000
    cmp     r8, rdx
    jne     .rem_s64_done
    cmp     rcx, -1
    jne     .rem_s64_done
    xor     eax, eax
.rem_s64_done:
    call    exec_stack_push
    jc      .overflow_error
    jmp     .dispatch_next

.op_i64_rem_u:
    call    exec_stack_pop
    jc      .underflow_error
    mov     rcx, rax
    call    exec_stack_pop
    jc      .underflow_error
    er_check_zero rcx, .arithmetic_trap
    xor     rdx, rdx
    div     rcx
    mov     rax, rdx
    call    exec_stack_push
    jc      .overflow_error
    jmp     .dispatch_next

.op_i64_and:
    _wasm_binop r, and

.op_i64_or:
    _wasm_binop r, or

.op_i64_xor:
    _wasm_binop r, xor

.op_i64_shl:
    _wasm_shiftop r, shl

.op_i64_shr_s:
    _wasm_shiftop r, sar

.op_i64_shr_u:
    _wasm_shiftop r, shr

.op_i64_rotl:
    _wasm_shiftop r, rol

.op_i64_rotr:
    _wasm_shiftop r, ror

.op_i64_clz:
    call    exec_stack_pop
    jc      .underflow_error
    bsr     rcx, rax
    jnz     .clz64_found
    mov     eax, 64
    jmp     .clz64_done
.clz64_found:
    mov     eax, 63
    sub     eax, ecx
.clz64_done:
    call    exec_stack_push
    jc      .overflow_error
    jmp     .dispatch_next

.op_i64_ctz:
    call    exec_stack_pop
    jc      .underflow_error
    bsf     rcx, rax
    jnz     .ctz64_found
    mov     eax, 64
    jmp     .ctz64_done
.ctz64_found:
    mov     eax, ecx
.ctz64_done:
    call    exec_stack_push
    jc      .overflow_error
    jmp     .dispatch_next

.op_i64_popcnt:
    _wasm_unop r, popcnt

; ==================================================================
; Integer i64 comparison ops
; =================================================================+
.op_i64_eqz:
    _wasm_eqz r

.op_i64_eq:
    _wasm_cmpop r, sete

.op_i64_ne:
    _wasm_cmpop r, setne

.op_i64_lt_s:
    _wasm_cmpop r, setl

.op_i64_lt_u:
    _wasm_cmpop r, setb

.op_i64_gt_s:
    _wasm_cmpop r, setg

.op_i64_gt_u:
    _wasm_cmpop r, seta

.op_i64_le_s:
    _wasm_cmpop r, setle

.op_i64_le_u:
    _wasm_cmpop r, setbe

.op_i64_ge_s:
    _wasm_cmpop r, setge

.op_i64_ge_u:
    _wasm_cmpop r, setae

; ==================================================================
; Conversion ops
; =================================================================+
.op_i32_wrap_i64:
    call    exec_stack_pop
    jc      .underflow_error
    ; Just take low 32 bits (already in rax, truncate upper bits)
    mov     eax, eax
    call    exec_stack_push
    jc      .overflow_error
    jmp     .dispatch_next

.op_i64_extend_i32_s:
    call    exec_stack_pop
    jc      .underflow_error
    movsxd  rax, eax
    call    exec_stack_push
    jc      .overflow_error
    jmp     .dispatch_next

.op_i64_extend_i32_u:
    call    exec_stack_pop
    jc      .underflow_error
    ; Zero extend (rax already has i32 in low bits, high bits 0 from 32-bit ops)
    mov     eax, eax    ; zero extend
    call    exec_stack_push
    jc      .overflow_error
    jmp     .dispatch_next

.op_i32_extend8_s:
    call    exec_stack_pop
    jc      .underflow_error
    movsx   eax, al
    call    exec_stack_push
    jc      .overflow_error
    jmp     .dispatch_next

.op_i32_extend16_s:
    call    exec_stack_pop
    jc      .underflow_error
    movsx   eax, ax
    call    exec_stack_push
    jc      .overflow_error
    jmp     .dispatch_next

.op_i64_extend8_s:
    call    exec_stack_pop
    jc      .underflow_error
    movsx   rax, al
    call    exec_stack_push
    jc      .overflow_error
    jmp     .dispatch_next

.op_i64_extend16_s:
    call    exec_stack_pop
    jc      .underflow_error
    movsx   rax, ax
    call    exec_stack_push
    jc      .overflow_error
    jmp     .dispatch_next

.op_i64_extend32_s:
    call    exec_stack_pop
    jc      .underflow_error
    movsxd  rax, eax
    call    exec_stack_push
    jc      .overflow_error
    jmp     .dispatch_next

; ==================================================================
; Reinterpret ops
; =================================================================+
.op_i32_reinterpret_f32:
    ; f32 bits are already on stack as 64-bit value
    ; Just take low 32 bits
    call    exec_stack_peek
    ; Value is already in rax, just keep as is
    ; (but tagged as i32, which we don't track)
    jmp     .dispatch_next

.op_f32_reinterpret_i32:
    ; i32 bits are already on stack
    jmp     .dispatch_next

.op_i64_reinterpret_f64:
    jmp     .dispatch_next

.op_f64_reinterpret_i64:
    jmp     .dispatch_next

; ==================================================================
; Reference ops
; =================================================================+
.op_ref_null:
    mov     eax, -1
    call    exec_stack_push
    jc      .overflow_error
    jmp     .dispatch_next

.op_ref_is_null:
    ; Pop reference value
    call    exec_stack_pop
    jc      .underflow_error
    cmp     eax, -1
    sete    al
    movzx   eax, al
    call    exec_stack_push
    jc      .overflow_error
    jmp     .dispatch_next

.op_ref_func:
    ; Read function index
    wasm_exec_reader_ptr
    call    er_wasm_read_leb_u32
    er_check_nonzero rdx, .corrupt_error
    wasm_exec_save_reader_offset
    mov     rdi, rax
    ; Verify function index < totalFunctionCount
    mov     rax, [import_count]
    add     rax, [function_count]
    cmp     rdi, rax
    jae     .corrupt_error
    ; Push function index as funcref
    mov     rax, rdi
    call    exec_stack_push
    jc      .overflow_error
    jmp     .dispatch_next

; ==================================================================
; Memory ops
; =================================================================+


.op_i32_load:
    call    exec_memory_prepare
    jc      .error_return
    mov     ecx, 4          ; size
    call    exec_memory_check_range
    jc      .error_return
    mov     eax, [rax]      ; load 32 bits
    call    exec_stack_push
    jc      .overflow_error
    jmp     .dispatch_next

.op_i32_load8_s:
    call    exec_memory_prepare
    jc      .error_return
    mov     ecx, 1
    call    exec_memory_check_range
    jc      .error_return
    movsx   eax, byte [rax]
    call    exec_stack_push
    jc      .overflow_error
    jmp     .dispatch_next

.op_i32_load8_u:
    call    exec_memory_prepare
    jc      .error_return
    mov     ecx, 1
    call    exec_memory_check_range
    jc      .error_return
    movzx   eax, byte [rax]
    call    exec_stack_push
    jc      .overflow_error
    jmp     .dispatch_next

.op_i32_load16_s:
    call    exec_memory_prepare
    jc      .error_return
    mov     ecx, 2
    call    exec_memory_check_range
    jc      .error_return
    movsx   eax, word [rax]
    call    exec_stack_push
    jc      .overflow_error
    jmp     .dispatch_next

.op_i32_load16_u:
    call    exec_memory_prepare
    jc      .error_return
    mov     ecx, 2
    call    exec_memory_check_range
    jc      .error_return
    movzx   eax, word [rax]
    call    exec_stack_push
    jc      .overflow_error
    jmp     .dispatch_next

.op_i32_store:
    call    exec_stack_pop
    jc      .underflow_error
    push    rax
    call    exec_memory_prepare
    jc      .error_return
    mov     ecx, 4
    call    exec_memory_check_range
    jc      .error_return
    pop     rcx
    mov     [rax], ecx
    jmp     .dispatch_next

.op_i32_store8:
    call    exec_stack_pop
    jc      .underflow_error
    push    rax
    call    exec_memory_prepare
    jc      .error_return
    mov     ecx, 1
    call    exec_memory_check_range
    jc      .error_return
    pop     rcx
    mov     [rax], cl
    jmp     .dispatch_next

.op_i32_store16:
    call    exec_memory_prepare
    jc      .error_return
    mov     ecx, 2
    call    exec_memory_check_range
    jc      .error_return
    push    rax
    call    exec_stack_pop
    jc      .underflow_error
    pop     rcx
    mov     [rcx], ax
    jmp     .dispatch_next

.op_i64_load:
    call    exec_memory_prepare
    jc      .error_return
    mov     ecx, 8
    call    exec_memory_check_range
    jc      .error_return
    mov     rax, [rax]
    call    exec_stack_push
    jc      .overflow_error
    jmp     .dispatch_next

.op_i64_load8_s:
    call    exec_memory_prepare
    jc      .error_return
    mov     ecx, 1
    call    exec_memory_check_range
    jc      .error_return
    movsx   rax, byte [rax]
    call    exec_stack_push
    jc      .overflow_error
    jmp     .dispatch_next

.op_i64_load8_u:
    call    exec_memory_prepare
    jc      .error_return
    mov     ecx, 1
    call    exec_memory_check_range
    jc      .error_return
    movzx   rax, byte [rax]
    call    exec_stack_push
    jc      .overflow_error
    jmp     .dispatch_next

.op_i64_load16_s:
    call    exec_memory_prepare
    jc      .error_return
    mov     ecx, 2
    call    exec_memory_check_range
    jc      .error_return
    movsx   rax, word [rax]
    call    exec_stack_push
    jc      .overflow_error
    jmp     .dispatch_next

.op_i64_load16_u:
    call    exec_memory_prepare
    jc      .error_return
    mov     ecx, 2
    call    exec_memory_check_range
    jc      .error_return
    movzx   rax, word [rax]
    call    exec_stack_push
    jc      .overflow_error
    jmp     .dispatch_next

.op_i64_load32_s:
    call    exec_memory_prepare
    jc      .error_return
    mov     ecx, 4
    call    exec_memory_check_range
    jc      .error_return
    movsxd  rax, dword [rax]
    call    exec_stack_push
    jc      .overflow_error
    jmp     .dispatch_next

.op_i64_load32_u:
    call    exec_memory_prepare
    jc      .error_return
    mov     ecx, 4
    call    exec_memory_check_range
    jc      .error_return
    mov     eax, [rax]
    call    exec_stack_push
    jc      .overflow_error
    jmp     .dispatch_next

.op_i64_store:
    call    exec_memory_prepare
    jc      .error_return
    mov     ecx, 8
    call    exec_memory_check_range
    jc      .error_return
    push    rax
    call    exec_stack_pop
    jc      .underflow_error
    pop     rcx
    mov     [rcx], rax
    jmp     .dispatch_next

.op_i64_store8:
    call    exec_memory_prepare
    jc      .error_return
    mov     ecx, 1
    call    exec_memory_check_range
    jc      .error_return
    push    rax
    call    exec_stack_pop
    jc      .underflow_error
    pop     rcx
    mov     [rcx], al
    jmp     .dispatch_next

.op_i64_store16:
    call    exec_memory_prepare
    jc      .error_return
    mov     ecx, 2
    call    exec_memory_check_range
    jc      .error_return
    push    rax
    call    exec_stack_pop
    jc      .underflow_error
    pop     rcx
    mov     [rcx], ax
    jmp     .dispatch_next

.op_i64_store32:
    call    exec_memory_prepare
    jc      .error_return
    mov     ecx, 4
    call    exec_memory_check_range
    jc      .error_return
    push    rax
    call    exec_stack_pop
    jc      .underflow_error
    pop     rcx
    mov     [rcx], eax
    jmp     .dispatch_next

.op_memory_size:
    mov     rax, [executor_memory_pages]
    call    exec_stack_push
    jc      .overflow_error
    jmp     .dispatch_next

.op_memory_grow:
    call    exec_stack_pop
    jc      .underflow_error
    mov     r15, rax         ; delta pages
    mov     r14, [executor_memory_pages] ; previous pages
    mov     r13, r14
    add     r13, r15         ; requested pages = previous + delta
    jc      .memory_grow_requires_grant

    mov     rax, [memory_max_pages]
    er_check_zero rax, .memory_grow_check_bytes
    cmp     r13, rax
    ja      .memory_grow_requires_grant

.memory_grow_check_bytes:
    mov     rdi, r13
    call    er_wasm_pages_to_bytes
    er_check_nonzero rdx, .memory_grow_requires_grant
    mov     r12, rax         ; requested bytes

    cmp     r12, [runtime_memory_len]
    ja      .memory_grow_requires_grant

    mov     [executor_memory_pages], r13
    mov     [executor_memory_limit], r12
    mov     rax, r14
    call    exec_stack_push
    jc      .overflow_error
    jmp     .dispatch_next
.memory_grow_requires_grant:
    er_err  ERROR_MEMORY_GROWTH
    jmp     .error_return

; ==================================================================
; Table ops
; =================================================================+
.op_table_get:
    wasm_exec_reader_ptr
    call    er_wasm_read_leb_u32
    er_check_nonzero rdx, .corrupt_error
    er_check_nonzero eax, .memory_trap
    wasm_exec_save_reader_offset
    call    exec_stack_pop
    jc      .underflow_error
    cmp     rax, [table_min]
    jae     .corrupt_error
    mov     rax, [table_entries + rax * 8]
    call    exec_stack_push
    jc      .overflow_error
    jmp     .dispatch_next

.op_table_set:
    wasm_exec_reader_ptr
    call    er_wasm_read_leb_u32
    er_check_nonzero rdx, .corrupt_error
    er_check_nonzero eax, .memory_trap
    wasm_exec_save_reader_offset
    call    exec_stack_pop   ; value
    jc      .underflow_error
    push    rax
    call    exec_stack_pop   ; index
    jc      .underflow_error
    pop     rcx
    cmp     rax, [table_min]
    jae     .corrupt_error
    mov     [table_entries + rax * 8], rcx
    jmp     .dispatch_next

; ==================================================================
; Float ops
; =================================================================+

; ------------------------------------------------------------------
; Load / Store
; -----------------------------------------------------------------+
.op_f32_load:
    call    exec_memory_prepare
    jc      .error_return
    mov     ecx, 4
    call    exec_memory_check_range
    jc      .error_return
    mov     eax, [rax]
    call    exec_stack_push
    jc      .overflow_error
    jmp     .dispatch_next

.op_f64_load:
    call    exec_memory_prepare
    jc      .error_return
    mov     ecx, 8
    call    exec_memory_check_range
    jc      .error_return
    mov     rax, [rax]
    call    exec_stack_push
    jc      .overflow_error
    jmp     .dispatch_next

.op_f32_store:
    call    exec_memory_prepare
    jc      .error_return
    mov     ecx, 4
    call    exec_memory_check_range
    jc      .error_return
    push    rax
    call    exec_stack_pop
    jc      .underflow_error
    pop     rcx
    mov     [rcx], eax
    jmp     .dispatch_next

.op_f64_store:
    call    exec_memory_prepare
    jc      .error_return
    mov     ecx, 8
    call    exec_memory_check_range
    jc      .error_return
    push    rax
    call    exec_stack_pop
    jc      .underflow_error
    pop     rcx
    mov     [rcx], rax
    jmp     .dispatch_next

; ------------------------------------------------------------------
; Constant ops
; -----------------------------------------------------------------+
.op_f32_const:
    wasm_exec_reader_ptr
    mov     rax, [exec_reader_offset]
    add     rax, 4
    cmp     rax, [exec_code_body_len]
    ja      .corrupt_error
    mov     eax, [rsi]
    add     qword [exec_reader_offset], 4
    call    exec_stack_push
    jc      .overflow_error
    jmp     .dispatch_next

.op_f64_const:
    wasm_exec_reader_ptr
    mov     rax, [exec_reader_offset]
    add     rax, 8
    cmp     rax, [exec_code_body_len]
    ja      .corrupt_error
    mov     rax, [rsi]
    add     qword [exec_reader_offset], 8
    call    exec_stack_push
    jc      .overflow_error
    jmp     .dispatch_next

; ------------------------------------------------------------------
; f32 Comparisons
; -----------------------------------------------------------------+
.op_f32_eq:
    call    exec_stack_pop
    jc      .underflow_error
    movd    xmm1, eax
    call    exec_stack_pop
    jc      .underflow_error
    movd    xmm0, eax
    ucomiss xmm0, xmm1
    setz    al
    jnp     .f32_eq_done
    xor     al, al
.f32_eq_done:
    movzx   eax, al
    call    exec_stack_push
    jc      .overflow_error
    jmp     .dispatch_next

.op_f32_ne:
    call    exec_stack_pop
    jc      .underflow_error
    movd    xmm1, eax
    call    exec_stack_pop
    jc      .underflow_error
    movd    xmm0, eax
    ucomiss xmm0, xmm1
    setnz   al
    jnp     .f32_ne_done
    mov     al, 1
.f32_ne_done:
    movzx   eax, al
    call    exec_stack_push
    jc      .overflow_error
    jmp     .dispatch_next

.op_f32_lt:
    call    exec_stack_pop
    jc      .underflow_error
    movd    xmm1, eax
    call    exec_stack_pop
    jc      .underflow_error
    movd    xmm0, eax
    ucomiss xmm0, xmm1
    setnp   cl
    setb    al
    and     al, cl
    movzx   eax, al
    call    exec_stack_push
    jc      .overflow_error
    jmp     .dispatch_next

.op_f32_gt:
    call    exec_stack_pop
    jc      .underflow_error
    movd    xmm1, eax
    call    exec_stack_pop
    jc      .underflow_error
    movd    xmm0, eax
    ucomiss xmm1, xmm0
    setnp   cl
    setb    al
    and     al, cl
    movzx   eax, al
    call    exec_stack_push
    jc      .overflow_error
    jmp     .dispatch_next

.op_f32_le:
    call    exec_stack_pop
    jc      .underflow_error
    movd    xmm1, eax
    call    exec_stack_pop
    jc      .underflow_error
    movd    xmm0, eax
    ucomiss xmm0, xmm1
    setnp   cl
    setbe   al
    and     al, cl
    movzx   eax, al
    call    exec_stack_push
    jc      .overflow_error
    jmp     .dispatch_next

.op_f32_ge:
    call    exec_stack_pop
    jc      .underflow_error
    movd    xmm1, eax
    call    exec_stack_pop
    jc      .underflow_error
    movd    xmm0, eax
    ucomiss xmm1, xmm0
    setnp   cl
    setbe   al
    and     al, cl
    movzx   eax, al
    call    exec_stack_push
    jc      .overflow_error
    jmp     .dispatch_next

; ------------------------------------------------------------------
; f64 Comparisons
; -----------------------------------------------------------------+
.op_f64_eq:
    call    exec_stack_pop
    jc      .underflow_error
    movq    xmm1, rax
    call    exec_stack_pop
    jc      .underflow_error
    movq    xmm0, rax
    ucomisd xmm0, xmm1
    setnp   cl
    setz    al
    and     al, cl
    movzx   eax, al
    call    exec_stack_push
    jc      .overflow_error
    jmp     .dispatch_next

.op_f64_ne:
    call    exec_stack_pop
    jc      .underflow_error
    movq    xmm1, rax
    call    exec_stack_pop
    jc      .underflow_error
    movq    xmm0, rax
    ucomisd xmm0, xmm1
    setp    cl
    setnz   al
    or      al, cl
    movzx   eax, al
    call    exec_stack_push
    jc      .overflow_error
    jmp     .dispatch_next

.op_f64_lt:
    call    exec_stack_pop
    jc      .underflow_error
    movq    xmm1, rax
    call    exec_stack_pop
    jc      .underflow_error
    movq    xmm0, rax
    ucomisd xmm0, xmm1
    setnp   cl
    setb    al
    and     al, cl
    movzx   eax, al
    call    exec_stack_push
    jc      .overflow_error
    jmp     .dispatch_next

.op_f64_gt:
    call    exec_stack_pop
    jc      .underflow_error
    movq    xmm1, rax
    call    exec_stack_pop
    jc      .underflow_error
    movq    xmm0, rax
    ucomisd xmm1, xmm0
    setnp   cl
    setb    al
    and     al, cl
    movzx   eax, al
    call    exec_stack_push
    jc      .overflow_error
    jmp     .dispatch_next

.op_f64_le:
    call    exec_stack_pop
    jc      .underflow_error
    movq    xmm1, rax
    call    exec_stack_pop
    jc      .underflow_error
    movq    xmm0, rax
    ucomisd xmm0, xmm1
    setnp   cl
    setbe   al
    and     al, cl
    movzx   eax, al
    call    exec_stack_push
    jc      .overflow_error
    jmp     .dispatch_next

.op_f64_ge:
    call    exec_stack_pop
    jc      .underflow_error
    movq    xmm1, rax
    call    exec_stack_pop
    jc      .underflow_error
    movq    xmm0, rax
    ucomisd xmm1, xmm0
    setnp   cl
    setbe   al
    and     al, cl
    movzx   eax, al
    call    exec_stack_push
    jc      .overflow_error
    jmp     .dispatch_next

; ------------------------------------------------------------------
; f32 Arithmetic
; -----------------------------------------------------------------+
.op_f32_add:
    call    exec_stack_pop
    jc      .underflow_error
    movd    xmm1, eax
    call    exec_stack_pop
    jc      .underflow_error
    movd    xmm0, eax
    addss   xmm0, xmm1
    movd    eax, xmm0
    call    exec_stack_push
    jc      .overflow_error
    jmp     .dispatch_next

.op_f32_sub:
    call    exec_stack_pop
    jc      .underflow_error
    movd    xmm1, eax
    call    exec_stack_pop
    jc      .underflow_error
    movd    xmm0, eax
    subss   xmm0, xmm1
    movd    eax, xmm0
    call    exec_stack_push
    jc      .overflow_error
    jmp     .dispatch_next

.op_f32_mul:
    call    exec_stack_pop
    jc      .underflow_error
    movd    xmm1, eax
    call    exec_stack_pop
    jc      .underflow_error
    movd    xmm0, eax
    mulss   xmm0, xmm1
    movd    eax, xmm0
    call    exec_stack_push
    jc      .overflow_error
    jmp     .dispatch_next

.op_f32_div:
    call    exec_stack_pop
    jc      .underflow_error
    movd    xmm1, eax
    call    exec_stack_pop
    jc      .underflow_error
    movd    xmm0, eax
    divss   xmm0, xmm1
    movd    eax, xmm0
    call    exec_stack_push
    jc      .overflow_error
    jmp     .dispatch_next

.op_f32_min:
    call    exec_stack_pop
    jc      .underflow_error
    movd    xmm1, eax
    call    exec_stack_pop
    jc      .underflow_error
    movd    xmm0, eax
    ucomiss xmm0, xmm0
    jp      .f32_min_ret_a
    ucomiss xmm1, xmm1
    jp      .f32_min_ret_b
    minss   xmm0, xmm1
    movd    eax, xmm0
    call    exec_stack_push
    jc      .overflow_error
    jmp     .dispatch_next
.f32_min_ret_a:
    movd    eax, xmm0
    call    exec_stack_push
    jc      .overflow_error
    jmp     .dispatch_next
.f32_min_ret_b:
    movd    eax, xmm1
    call    exec_stack_push
    jc      .overflow_error
    jmp     .dispatch_next

.op_f32_max:
    call    exec_stack_pop
    jc      .underflow_error
    movd    xmm1, eax
    call    exec_stack_pop
    jc      .underflow_error
    movd    xmm0, eax
    ucomiss xmm0, xmm0
    jp      .f32_max_ret_a
    ucomiss xmm1, xmm1
    jp      .f32_max_ret_b
    maxss   xmm0, xmm1
    movd    eax, xmm0
    call    exec_stack_push
    jc      .overflow_error
    jmp     .dispatch_next
.f32_max_ret_a:
    movd    eax, xmm0
    call    exec_stack_push
    jc      .overflow_error
    jmp     .dispatch_next
.f32_max_ret_b:
    movd    eax, xmm1
    call    exec_stack_push
    jc      .overflow_error
    jmp     .dispatch_next

.op_f32_copysign:
    call    exec_stack_pop
    jc      .underflow_error
    push    rax
    call    exec_stack_pop
    jc      .underflow_error
    pop     rcx
    and     ecx, FLOAT_SIGN_BIT
    and     eax, FLOAT_ABS_MASK
    or      eax, ecx
    call    exec_stack_push
    jc      .overflow_error
    jmp     .dispatch_next

; ------------------------------------------------------------------
; f32 Unary
; -----------------------------------------------------------------+
.op_f32_abs:
    call    exec_stack_pop
    jc      .underflow_error
    and     eax, FLOAT_ABS_MASK
    call    exec_stack_push
    jc      .overflow_error
    jmp     .dispatch_next

.op_f32_neg:
    call    exec_stack_pop
    jc      .underflow_error
    xor     eax, FLOAT_SIGN_BIT
    call    exec_stack_push
    jc      .overflow_error
    jmp     .dispatch_next

.op_f32_sqrt:
    call    exec_stack_pop
    jc      .underflow_error
    movd    xmm0, eax
    sqrtss  xmm0, xmm0
    movd    eax, xmm0
    call    exec_stack_push
    jc      .overflow_error
    jmp     .dispatch_next

.op_f32_ceil:
    call    exec_stack_pop
    jc      .underflow_error
    movd    xmm0, eax
    roundss xmm0, xmm0, 0xA
    movd    eax, xmm0
    call    exec_stack_push
    jc      .overflow_error
    jmp     .dispatch_next

.op_f32_floor:
    call    exec_stack_pop
    jc      .underflow_error
    movd    xmm0, eax
    roundss xmm0, xmm0, 0x9
    movd    eax, xmm0
    call    exec_stack_push
    jc      .overflow_error
    jmp     .dispatch_next

.op_f32_trunc:
    call    exec_stack_pop
    jc      .underflow_error
    movd    xmm0, eax
    roundss xmm0, xmm0, 0xB
    movd    eax, xmm0
    call    exec_stack_push
    jc      .overflow_error
    jmp     .dispatch_next

.op_f32_nearest:
    call    exec_stack_pop
    jc      .underflow_error
    movd    xmm0, eax
    roundss xmm0, xmm0, 0x8
    movd    eax, xmm0
    call    exec_stack_push
    jc      .overflow_error
    jmp     .dispatch_next

; ------------------------------------------------------------------
; f64 Arithmetic
; -----------------------------------------------------------------+
.op_f64_add:
    call    exec_stack_pop
    jc      .underflow_error
    movq    xmm1, rax
    call    exec_stack_pop
    jc      .underflow_error
    movq    xmm0, rax
    addsd   xmm0, xmm1
    movq    rax, xmm0
    call    exec_stack_push
    jc      .overflow_error
    jmp     .dispatch_next

.op_f64_sub:
    call    exec_stack_pop
    jc      .underflow_error
    movq    xmm1, rax
    call    exec_stack_pop
    jc      .underflow_error
    movq    xmm0, rax
    subsd   xmm0, xmm1
    movq    rax, xmm0
    call    exec_stack_push
    jc      .overflow_error
    jmp     .dispatch_next

.op_f64_mul:
    call    exec_stack_pop
    jc      .underflow_error
    movq    xmm1, rax
    call    exec_stack_pop
    jc      .underflow_error
    movq    xmm0, rax
    mulsd   xmm0, xmm1
    movq    rax, xmm0
    call    exec_stack_push
    jc      .overflow_error
    jmp     .dispatch_next

.op_f64_div:
    call    exec_stack_pop
    jc      .underflow_error
    movq    xmm1, rax
    call    exec_stack_pop
    jc      .underflow_error
    movq    xmm0, rax
    divsd   xmm0, xmm1
    movq    rax, xmm0
    call    exec_stack_push
    jc      .overflow_error
    jmp     .dispatch_next

.op_f64_min:
    call    exec_stack_pop
    jc      .underflow_error
    movq    xmm1, rax
    call    exec_stack_pop
    jc      .underflow_error
    movq    xmm0, rax
    ucomisd xmm0, xmm0
    jp      .f64_min_ret_a
    ucomisd xmm1, xmm1
    jp      .f64_min_ret_b
    minsd   xmm0, xmm1
    movq    rax, xmm0
    call    exec_stack_push
    jc      .overflow_error
    jmp     .dispatch_next
.f64_min_ret_a:
    movq    rax, xmm0
    call    exec_stack_push
    jc      .overflow_error
    jmp     .dispatch_next
.f64_min_ret_b:
    movq    rax, xmm1
    call    exec_stack_push
    jc      .overflow_error
    jmp     .dispatch_next

.op_f64_max:
    call    exec_stack_pop
    jc      .underflow_error
    movq    xmm1, rax
    call    exec_stack_pop
    jc      .underflow_error
    movq    xmm0, rax
    ucomisd xmm0, xmm0
    jp      .f64_max_ret_a
    ucomisd xmm1, xmm1
    jp      .f64_max_ret_b
    maxsd   xmm0, xmm1
    movq    rax, xmm0
    call    exec_stack_push
    jc      .overflow_error
    jmp     .dispatch_next
.f64_max_ret_a:
    movq    rax, xmm0
    call    exec_stack_push
    jc      .overflow_error
    jmp     .dispatch_next
.f64_max_ret_b:
    movq    rax, xmm1
    call    exec_stack_push
    jc      .overflow_error
    jmp     .dispatch_next

.op_f64_copysign:
    call    exec_stack_pop
    jc      .underflow_error
    push    rax
    call    exec_stack_pop
    jc      .underflow_error
    pop     rcx
    mov     rdx, DOUBLE_SIGN_BIT
    and     rcx, rdx
    mov     rdx, DOUBLE_ABS_MASK
    and     rax, rdx
    or      rax, rcx
    call    exec_stack_push
    jc      .overflow_error
    jmp     .dispatch_next

; ------------------------------------------------------------------
; f64 Unary
; -----------------------------------------------------------------+
.op_f64_abs:
    call    exec_stack_pop
    jc      .underflow_error
    mov     rcx, DOUBLE_ABS_MASK
    and     rax, rcx
    call    exec_stack_push
    jc      .overflow_error
    jmp     .dispatch_next

.op_f64_neg:
    call    exec_stack_pop
    jc      .underflow_error
    mov     rcx, DOUBLE_SIGN_BIT
    xor     rax, rcx
    call    exec_stack_push
    jc      .overflow_error
    jmp     .dispatch_next

.op_f64_sqrt:
    call    exec_stack_pop
    jc      .underflow_error
    movq    xmm0, rax
    sqrtsd  xmm0, xmm0
    movq    rax, xmm0
    call    exec_stack_push
    jc      .overflow_error
    jmp     .dispatch_next

.op_f64_ceil:
    call    exec_stack_pop
    jc      .underflow_error
    movq    xmm0, rax
    roundsd xmm0, xmm0, 0xA
    movq    rax, xmm0
    call    exec_stack_push
    jc      .overflow_error
    jmp     .dispatch_next

.op_f64_floor:
    call    exec_stack_pop
    jc      .underflow_error
    movq    xmm0, rax
    roundsd xmm0, xmm0, 0x9
    movq    rax, xmm0
    call    exec_stack_push
    jc      .overflow_error
    jmp     .dispatch_next

.op_f64_trunc:
    call    exec_stack_pop
    jc      .underflow_error
    movq    xmm0, rax
    roundsd xmm0, xmm0, 0xB
    movq    rax, xmm0
    call    exec_stack_push
    jc      .overflow_error
    jmp     .dispatch_next

.op_f64_nearest:
    call    exec_stack_pop
    jc      .underflow_error
    movq    xmm0, rax
    roundsd xmm0, xmm0, 0x8
    movq    rax, xmm0
    call    exec_stack_push
    jc      .overflow_error
    jmp     .dispatch_next

; ------------------------------------------------------------------
; Truncations (trap on NaN / overflow)
; -----------------------------------------------------------------+
.op_i32_trunc_f32_s:
    call    exec_stack_pop
    jc      .underflow_error
    movd    xmm0, eax
    ucomiss xmm0, xmm0
    jp      .arithmetic_trap
    movss   xmm1, [rel f32_sat_2pow31]
    ucomiss xmm0, xmm1
    jae     .arithmetic_trap
    movss   xmm1, [rel f32_sat_minus_2pow31]
    ucomiss xmm0, xmm1
    jb      .arithmetic_trap
    cvttss2si eax, xmm0
    call    exec_stack_push
    jc      .overflow_error
    jmp     .dispatch_next

.op_i32_trunc_f32_u:
    call    exec_stack_pop
    jc      .underflow_error
    movd    xmm0, eax
    ucomiss xmm0, xmm0
    jp      .arithmetic_trap
    pxor    xmm1, xmm1
    ucomiss xmm0, xmm1
    jb      .arithmetic_trap
    movss   xmm1, [rel f32_sat_2pow32]
    ucomiss xmm0, xmm1
    jae     .arithmetic_trap
    movss   xmm1, [rel f32_sat_2pow31]
    ucomiss xmm0, xmm1
    jae     .trunc_f32_u_high
    cvttss2si eax, xmm0
    call    exec_stack_push
    jc      .overflow_error
    jmp     .dispatch_next
.trunc_f32_u_high:
    subss   xmm0, [rel f32_sat_2pow31]
    cvttss2si eax, xmm0
    add     eax, 0x80000000
    call    exec_stack_push
    jc      .overflow_error
    jmp     .dispatch_next

.op_i32_trunc_f64_s:
    call    exec_stack_pop
    jc      .underflow_error
    movq    xmm0, rax
    ucomisd xmm0, xmm0
    jp      .arithmetic_trap
    cvttsd2si eax, xmm0
    cmp     eax, 0x80000000
    jne     .trunc_f64_s_push
    movsd   xmm1, [rel f64_2pow31]
    ucomisd xmm0, xmm1
    jae     .arithmetic_trap
    movsd   xmm1, [rel f64_minus_2pow31]
    ucomisd xmm0, xmm1
    jb      .arithmetic_trap
.trunc_f64_s_push:
    call    exec_stack_push
    jc      .overflow_error
    jmp     .dispatch_next

.op_i32_trunc_f64_u:
    call    exec_stack_pop
    jc      .underflow_error
    movq    xmm0, rax
    ucomisd xmm0, xmm0
    jp      .arithmetic_trap
    pxor    xmm1, xmm1
    ucomisd xmm0, xmm1
    jb      .arithmetic_trap
    movsd   xmm1, [rel f64_2pow32]
    ucomisd xmm0, xmm1
    jae     .arithmetic_trap
    movsd   xmm1, [rel f64_2pow31]
    ucomisd xmm0, xmm1
    jae     .trunc_f64_u_high
    cvttsd2si eax, xmm0
    call    exec_stack_push
    jc      .overflow_error
    jmp     .dispatch_next
.trunc_f64_u_high:
    subsd   xmm0, [rel f64_2pow31]
    cvttsd2si eax, xmm0
    add     eax, 0x80000000
    call    exec_stack_push
    jc      .overflow_error
    jmp     .dispatch_next

.op_i64_trunc_f32_s:
    call    exec_stack_pop
    jc      .underflow_error
    movd    xmm0, eax
    ucomiss xmm0, xmm0
    jp      .arithmetic_trap
    cvttss2si rax, xmm0
    mov     rcx, 0x8000000000000000
    cmp     rax, rcx
    jne     .trunc_i64_f32_s_push
    movss   xmm1, [rel f32_sat_2pow63]
    ucomiss xmm0, xmm1
    jae     .arithmetic_trap
    movss   xmm1, [rel f32_sat_minus_2pow63]
    ucomiss xmm0, xmm1
    jb      .arithmetic_trap
.trunc_i64_f32_s_push:
    call    exec_stack_push
    jc      .overflow_error
    jmp     .dispatch_next

.op_i64_trunc_f32_u:
    call    exec_stack_pop
    jc      .underflow_error
    movd    xmm0, eax
    ucomiss xmm0, xmm0
    jp      .arithmetic_trap
    pxor    xmm1, xmm1
    ucomiss xmm0, xmm1
    jb      .arithmetic_trap
    movss   xmm1, [rel f32_sat_2pow64]
    ucomiss xmm0, xmm1
    jae     .arithmetic_trap
    movss   xmm1, [rel f32_sat_2pow63]
    ucomiss xmm0, xmm1
    jae     .trunc_i64_f32_u_high
    cvttss2si rax, xmm0
    call    exec_stack_push
    jc      .overflow_error
    jmp     .dispatch_next
.trunc_i64_f32_u_high:
    subss   xmm0, [rel f32_sat_2pow63]
    cvttss2si rax, xmm0
    bts     rax, 63
    call    exec_stack_push
    jc      .overflow_error
    jmp     .dispatch_next

.op_i64_trunc_f64_s:
    call    exec_stack_pop
    jc      .underflow_error
    movq    xmm0, rax
    ucomisd xmm0, xmm0
    jp      .arithmetic_trap
    cvttsd2si rax, xmm0
    mov     rcx, 0x8000000000000000
    cmp     rax, rcx
    jne     .trunc_i64_f64_s_push
    movsd   xmm1, [rel f64_sat_2pow63]
    ucomisd xmm0, xmm1
    jae     .arithmetic_trap
    movsd   xmm1, [rel f64_sat_minus_2pow63]
    ucomisd xmm0, xmm1
    jb      .arithmetic_trap
.trunc_i64_f64_s_push:
    call    exec_stack_push
    jc      .overflow_error
    jmp     .dispatch_next

.op_i64_trunc_f64_u:
    call    exec_stack_pop
    jc      .underflow_error
    movq    xmm0, rax
    ucomisd xmm0, xmm0
    jp      .arithmetic_trap
    pxor    xmm1, xmm1
    ucomisd xmm0, xmm1
    jb      .arithmetic_trap
    movsd   xmm1, [rel f64_sat_2pow64]
    ucomisd xmm0, xmm1
    jae     .arithmetic_trap
    movsd   xmm1, [rel f64_sat_2pow63]
    ucomisd xmm0, xmm1
    jae     .trunc_i64_f64_u_high
    cvttsd2si rax, xmm0
    call    exec_stack_push
    jc      .overflow_error
    jmp     .dispatch_next
.trunc_i64_f64_u_high:
    subsd   xmm0, [rel f64_sat_2pow63]
    cvttsd2si rax, xmm0
    bts     rax, 63
    call    exec_stack_push
    jc      .overflow_error
    jmp     .dispatch_next

; ------------------------------------------------------------------
; Conversions
; -----------------------------------------------------------------+
.op_f32_convert_i32_s:
    call    exec_stack_pop
    jc      .underflow_error
    cvtsi2ss xmm0, eax
    movd    eax, xmm0
    call    exec_stack_push
    jc      .overflow_error
    jmp     .dispatch_next

.op_f32_convert_i32_u:
    call    exec_stack_pop
    jc      .underflow_error
    test    eax, eax
    js      .cvt_f32_u32_high
    cvtsi2ss xmm0, eax
    movd    eax, xmm0
    call    exec_stack_push
    jc      .overflow_error
    jmp     .dispatch_next
.cvt_f32_u32_high:
    movd    xmm0, eax
    cvtsi2ss xmm0, eax
    addss   xmm0, [rel f32_sat_2pow32]
    movd    eax, xmm0
    call    exec_stack_push
    jc      .overflow_error
    jmp     .dispatch_next

.op_f32_convert_i64_s:
    call    exec_stack_pop
    jc      .underflow_error
    cvtsi2ss xmm0, rax
    movd    eax, xmm0
    call    exec_stack_push
    jc      .overflow_error
    jmp     .dispatch_next

.op_f32_convert_i64_u:
    call    exec_stack_pop
    jc      .underflow_error
    test    rax, rax
    js      .cvt_f32_u64_high
    cvtsi2ss xmm0, rax
    movd    eax, xmm0
    call    exec_stack_push
    jc      .overflow_error
    jmp     .dispatch_next
.cvt_f32_u64_high:
    mov     rcx, 0x8000000000000000
    sub     rax, rcx
    cvtsi2ss xmm0, rax
    addss   xmm0, [rel f32_sat_2pow63]
    movd    eax, xmm0
    call    exec_stack_push
    jc      .overflow_error
    jmp     .dispatch_next

.op_f32_demote_f64:
    call    exec_stack_pop
    jc      .underflow_error
    movq    xmm0, rax
    cvtsd2ss xmm0, xmm0
    movd    eax, xmm0
    call    exec_stack_push
    jc      .overflow_error
    jmp     .dispatch_next

.op_f64_convert_i32_s:
    call    exec_stack_pop
    jc      .underflow_error
    cvtsi2sd xmm0, eax
    movq    rax, xmm0
    call    exec_stack_push
    jc      .overflow_error
    jmp     .dispatch_next

.op_f64_convert_i32_u:
    call    exec_stack_pop
    jc      .underflow_error
    test    eax, eax
    js      .cvt_f64_u32_high
    cvtsi2sd xmm0, eax
    movq    rax, xmm0
    call    exec_stack_push
    jc      .overflow_error
    jmp     .dispatch_next
.cvt_f64_u32_high:
    movd    xmm0, eax
    cvtsi2sd xmm0, eax
    addsd   xmm0, [rel f64_2pow32]
    movq    rax, xmm0
    call    exec_stack_push
    jc      .overflow_error
    jmp     .dispatch_next

.op_f64_convert_i64_s:
    call    exec_stack_pop
    jc      .underflow_error
    cvtsi2sd xmm0, rax
    movq    rax, xmm0
    call    exec_stack_push
    jc      .overflow_error
    jmp     .dispatch_next

.op_f64_convert_i64_u:
    call    exec_stack_pop
    jc      .underflow_error
    test    rax, rax
    js      .cvt_f64_u64_high
    cvtsi2sd xmm0, rax
    movq    rax, xmm0
    call    exec_stack_push
    jc      .overflow_error
    jmp     .dispatch_next
.cvt_f64_u64_high:
    mov     rcx, 0x8000000000000000
    sub     rax, rcx
    cvtsi2sd xmm0, rax
    addsd   xmm0, [rel f64_sat_2pow63]
    movq    rax, xmm0
    call    exec_stack_push
    jc      .overflow_error
    jmp     .dispatch_next

.op_f64_promote_f32:
    call    exec_stack_pop
    jc      .underflow_error
    movd    xmm0, eax
    cvtss2sd xmm0, xmm0
    movq    rax, xmm0
    call    exec_stack_push
    jc      .overflow_error
    jmp     .dispatch_next

; ==================================================================
; Extended opcodes (0xfc prefix)
; =================================================================+
.extended_opcode:
    ; Read extended opcode
    wasm_exec_reader_ptr
    call    er_wasm_read_leb_u32
    er_check_nonzero rdx, .corrupt_error
    wasm_exec_save_reader_offset

    cmp     eax, EXT_I32_TRUNC_SAT_F32_S
    je      .ext_i32_trunc_sat_f32_s
    cmp     eax, EXT_I32_TRUNC_SAT_F32_U
    je      .ext_i32_trunc_sat_f32_u
    cmp     eax, EXT_I32_TRUNC_SAT_F64_S
    je      .ext_i32_trunc_sat_f64_s
    cmp     eax, EXT_I32_TRUNC_SAT_F64_U
    je      .ext_i32_trunc_sat_f64_u
    cmp     eax, EXT_I64_TRUNC_SAT_F32_S
    je      .ext_i64_trunc_sat_f32_s
    cmp     eax, EXT_I64_TRUNC_SAT_F32_U
    je      .ext_i64_trunc_sat_f32_u
    cmp     eax, EXT_I64_TRUNC_SAT_F64_S
    je      .ext_i64_trunc_sat_f64_s
    cmp     eax, EXT_I64_TRUNC_SAT_F64_U
    je      .ext_i64_trunc_sat_f64_u
    cmp     eax, EXT_MEMORY_INIT
    je      .ext_memory_init
    cmp     eax, EXT_DATA_DROP
    je      .ext_data_drop
    cmp     eax, EXT_MEMORY_COPY
    je      .ext_memory_copy
    cmp     eax, EXT_MEMORY_FILL
    je      .ext_memory_fill
    cmp     eax, EXT_TABLE_INIT
    je      .ext_table_init
    cmp     eax, EXT_ELEM_DROP
    je      .ext_elem_drop
    cmp     eax, EXT_TABLE_COPY
    je      .ext_table_copy
    cmp     eax, EXT_TABLE_GROW
    je      .ext_table_grow
    cmp     eax, EXT_TABLE_SIZE
    je      .ext_table_size
    cmp     eax, EXT_TABLE_FILL
    je      .ext_table_fill

    jmp     .unsupported_error

; ==================================================================
; Saturating truncation ops (0xfc prefix, opcodes 0-7)
; =================================================================+

.ext_i32_trunc_sat_f32_s:
    call    exec_stack_pop
    jc      .underflow_error
    movd    xmm0, eax
    ucomiss xmm0, xmm0
    jp      .sat_zero_i32
    movss   xmm1, [rel f32_sat_2pow31]
    ucomiss xmm0, xmm1
    jae     .sat_i32_max
    movss   xmm1, [rel f32_sat_minus_2pow31]
    ucomiss xmm0, xmm1
    jb      .sat_i32_min
    cvttss2si eax, xmm0
    call    exec_stack_push
    jc      .overflow_error
    jmp     .dispatch_next
.sat_i32_max:
    mov     eax, 0x7fffffff
    call    exec_stack_push
    jc      .overflow_error
    jmp     .dispatch_next
.sat_i32_min:
    mov     eax, 0x80000000
    call    exec_stack_push
    jc      .overflow_error
    jmp     .dispatch_next
.sat_zero_i32:
    xor     eax, eax
    call    exec_stack_push
    jc      .overflow_error
    jmp     .dispatch_next

.ext_i32_trunc_sat_f32_u:
    call    exec_stack_pop
    jc      .underflow_error
    movd    xmm0, eax
    ucomiss xmm0, xmm0
    jp      .sat_zero_i32
    xorps   xmm1, xmm1
    ucomiss xmm0, xmm1
    jbe     .sat_zero_i32
    movss   xmm1, [rel f32_sat_2pow32]
    ucomiss xmm0, xmm1
    jae     .sat_u32_max
    movss   xmm1, [rel f32_sat_2pow31]
    ucomiss xmm0, xmm1
    jae     .sat_u32_high
    cvttss2si eax, xmm0
    jmp     .push_sat_i32
.sat_u32_high:
    subss   xmm0, xmm1
    cvttss2si eax, xmm0
    add     eax, 0x80000000
    jmp     .push_sat_i32
.sat_u32_max:
    mov     eax, 0xffffffff
    jmp     .push_sat_i32

.ext_i32_trunc_sat_f64_s:
    call    exec_stack_pop
    jc      .underflow_error
    movq    xmm0, rax
    ucomisd xmm0, xmm0
    jp      .sat_zero_i32
    cvttsd2si eax, xmm0
    cmp     eax, 0x80000000
    jne     .push_sat_i32
    ucomisd xmm0, xmm0
    jp      .sat_zero_i32
    movq    rax, xmm0
    bt      rax, 63
    jc      .sat_i32_min
    jmp     .sat_i32_max
.push_sat_i32:
    call    exec_stack_push
    jc      .overflow_error
    jmp     .dispatch_next

.ext_i32_trunc_sat_f64_u:
    call    exec_stack_pop
    jc      .underflow_error
    movq    xmm0, rax
    ucomisd xmm0, xmm0
    jp      .sat_zero_i32
    xorps   xmm1, xmm1
    ucomisd xmm0, xmm1
    jbe     .sat_zero_i32
    cvttsd2si eax, xmm0
    cmp     eax, 0x80000000
    jne     .push_sat_i32
    movq    rax, xmm0
    bt      rax, 63
    jc      .sat_zero_i32
    movsd   xmm1, [rel f64_sat_2pow64]
    ucomisd xmm0, xmm1
    jae     .sat_u32_max
    movsd   xmm1, [rel f64_sat_2pow63]
    subsd   xmm0, xmm1
    cvttsd2si eax, xmm0
    mov     ecx, 0x80000000
    xor     eax, ecx
    jmp     .push_sat_i32

.ext_i64_trunc_sat_f32_s:
    call    exec_stack_pop
    jc      .underflow_error
    movd    xmm0, eax
    ucomiss xmm0, xmm0
    jp      .sat_zero_i64
    cvttss2si rax, xmm0
    mov     rcx, 0x8000000000000000
    cmp     rax, rcx
    jne     .push_sat_i64
    movd    eax, xmm0
    bt      rax, 31
    jc      .sat_i64_min
    jmp     .sat_i64_max

.ext_i64_trunc_sat_f32_u:
    call    exec_stack_pop
    jc      .underflow_error
    movd    xmm0, eax
    ucomiss xmm0, xmm0
    jp      .sat_zero_i64
    xorps   xmm1, xmm1
    ucomiss xmm0, xmm1
    jbe     .sat_zero_i64
    cvttss2si rax, xmm0
    mov     rcx, 0x8000000000000000
    cmp     rax, rcx
    jne     .push_sat_i64
    movd    eax, xmm0
    bt      rax, 31
    jc      .sat_zero_i64
    mov     eax, 0x5F800000
    movd    xmm1, eax
    ucomiss xmm0, xmm1
    jae     .sat_u64_max_overflow
    mov     eax, 0x5F000000
    movd    xmm1, eax
    subss   xmm0, xmm1
    cvttss2si rax, xmm0
    mov     rcx, 0x8000000000000000
    xor     rax, rcx
    jmp     .push_sat_i64

.ext_i64_trunc_sat_f64_s:
    call    exec_stack_pop
    jc      .underflow_error
    movq    xmm0, rax
    ucomisd xmm0, xmm0
    jp      .sat_zero_i64
    cvttsd2si rax, xmm0
    mov     rcx, 0x8000000000000000
    cmp     rax, rcx
    jne     .push_sat_i64
    movq    rcx, xmm0
    bt      rcx, 63
    jc      .sat_i64_min
    jmp     .sat_i64_max

.ext_i64_trunc_sat_f64_u:
    call    exec_stack_pop
    jc      .underflow_error
    movq    xmm0, rax
    ucomisd xmm0, xmm0
    jp      .sat_zero_i64
    xorps   xmm1, xmm1
    ucomisd xmm0, xmm1
    jbe     .sat_zero_i64
    cvttsd2si rax, xmm0
    mov     rcx, 0x8000000000000000
    cmp     rax, rcx
    jne     .push_sat_i64
    movq    rcx, xmm0
    bt      rcx, 63
    jc      .sat_zero_i64
    mov     rcx, 0x43F0000000000000
    movq    xmm1, rcx
    ucomisd xmm0, xmm1
    jae     .sat_u64_max_overflow
    mov     rcx, 0x43E0000000000000
    movq    xmm1, rcx
    subsd   xmm0, xmm1
    cvttsd2si rax, xmm0
    mov     rcx, 0x8000000000000000
    xor     rax, rcx
    jmp     .push_sat_i64

.sat_zero_i64:
    xor     eax, eax
    xor     edx, edx
    jmp     .push_sat_i64_64
.sat_i64_max:
    mov     rax, 0x7fffffffffffffff
    jmp     .push_sat_i64_64
.sat_i64_min:
    mov     rax, 0x8000000000000000
    jmp     .push_sat_i64_64
.sat_u64_max_overflow:
    mov     rax, 0xffffffffffffffff
    jmp     .push_sat_i64_64
.push_sat_i64:
.push_sat_i64_64:
    call    exec_stack_push
    jc      .overflow_error
    jmp     .dispatch_next

.ext_memory_init:
    ; Read segment index, then memory index
    wasm_exec_reader_ptr
    call    er_wasm_read_leb_u32
    er_check_nonzero rdx, .corrupt_error
    push    rax             ; segment index
    wasm_exec_save_reader_offset
    wasm_exec_reader_ptr
    call    er_wasm_read_leb_u32
    er_check_nonzero rdx, .corrupt_error
    er_check_nonzero eax, .memory_trap
    wasm_exec_save_reader_offset
    pop     r15             ; segment index
    ; Pop dest, src, count from stack
    call    exec_stack_pop  ; n
    jc      .underflow_error
    mov     r14, rax
    call    exec_stack_pop  ; s
    jc      .underflow_error
    mov     r13, rax
    call    exec_stack_pop  ; d
    jc      .underflow_error
    mov     r12, rax
    ; Validate segment index
    cmp     r15, [data_segment_count]
    jae     .memory_trap

    ; Calculate segment descriptor offset
    mov     r10, r15
    imul    r10, DATA_SEGMENT_SIZE
    mov     rbx, r10

    ; memory.init consumes passive, not-yet-dropped data segments.
    cmp     byte [data_segments + rbx + 24], 0
    jne     .memory_trap
    cmp     byte [data_segments + rbx + 25], 0
    jne     .memory_trap

    ; Check s + n <= bytes_len
    mov     rax, [data_segments + rbx + 16]  ; bytes_len
    mov     rcx, r13
    add     rcx, r14
    jc      .memory_trap
    cmp     rcx, rax
    ja      .memory_trap

    ; Check d + n <= memory_len
    mov     rax, [runtime_memory_len]
    mov     rcx, r12
    add     rcx, r14
    jc      .memory_trap
    cmp     rcx, rax
    ja      .memory_trap

    ; Copy: memcpy(memory + d, bytes_ptr + s, n)
    mov     rdi, [runtime_memory_ptr]
    add     rdi, r12
    mov     rsi, [data_segments + rbx + 8]  ; bytes_ptr
    add     rsi, r13
    mov     rdx, r14
    call    er_memcpy

    jmp     .dispatch_next

.ext_data_drop:
    wasm_exec_reader_ptr
    call    er_wasm_read_leb_u32
    er_check_nonzero rdx, .corrupt_error
    wasm_exec_save_reader_offset
    ; Validate segment index
    cmp     rax, [data_segment_count]
    jae     .memory_trap

    ; Calculate segment descriptor offset
    mov     r10, rax
    imul    r10, DATA_SEGMENT_SIZE

    cmp     byte [data_segments + r10 + 24], 0
    jne     .memory_trap

    ; Set active = 0, dropped = 1
    mov     byte [data_segments + r10 + 24], 0
    mov     byte [data_segments + r10 + 25], 1

    jmp     .dispatch_next

.ext_memory_copy:
    ; Read dest memory index, then src memory index
    wasm_exec_reader_ptr
    call    er_wasm_read_leb_u32
    er_check_nonzero rdx, .corrupt_error
    er_check_nonzero eax, .memory_trap
    wasm_exec_save_reader_offset
    wasm_exec_reader_ptr
    call    er_wasm_read_leb_u32
    er_check_nonzero rdx, .corrupt_error
    er_check_nonzero eax, .memory_trap
    wasm_exec_save_reader_offset
    ; Pop n, s, d
    call    exec_stack_pop  ; n
    jc      .underflow_error
    mov     r14, rax
    call    exec_stack_pop  ; s
    jc      .underflow_error
    mov     r13, rax
    call    exec_stack_pop  ; d
    jc      .underflow_error
    mov     r12, rax
    ; Validate d + n <= memory_len
    mov     rax, [runtime_memory_len]
    mov     rcx, r12
    add     rcx, r14
    jc      .memory_trap
    cmp     rcx, rax
    ja      .memory_trap

    ; Validate s + n <= memory_len
    mov     rcx, r13
    add     rcx, r14
    jc      .memory_trap
    cmp     rcx, rax
    ja      .memory_trap

    ; memmove(memory + d, memory + s, n)
    mov     rdi, [runtime_memory_ptr]
    add     rdi, r12
    mov     rsi, [runtime_memory_ptr]
    add     rsi, r13
    mov     rdx, r14
    call    er_memmove

    jmp     .dispatch_next

.ext_memory_fill:
    wasm_exec_reader_ptr
    call    er_wasm_read_leb_u32
    er_check_nonzero rdx, .corrupt_error
    er_check_nonzero eax, .memory_trap
    wasm_exec_save_reader_offset
    ; Pop n, val, d
    call    exec_stack_pop  ; n
    jc      .underflow_error
    mov     r14, rax
    call    exec_stack_pop  ; val
    jc      .underflow_error
    mov     r13, rax
    call    exec_stack_pop  ; d
    jc      .underflow_error
    mov     r12, rax
    ; Validate d + n <= memory_len
    mov     rax, [runtime_memory_len]
    mov     rcx, r12
    add     rcx, r14
    jc      .memory_trap
    cmp     rcx, rax
    ja      .memory_trap

    ; memset(memory + d, val, n)
    mov     rdi, [runtime_memory_ptr]
    add     rdi, r12
    mov     esi, r13d           ; val (byte)
    mov     rdx, r14            ; n
    call    er_memset

    jmp     .dispatch_next

; ==================================================================
; Table bulk ops
; =================================================================+
.ext_table_init:
    wasm_exec_reader_ptr
    call    er_wasm_read_leb_u32
    er_check_nonzero rdx, .corrupt_error
    mov     r15, rax            ; element segment index
    wasm_exec_save_reader_offset
    wasm_exec_reader_ptr
    call    er_wasm_read_leb_u32
    er_check_nonzero rdx, .corrupt_error
    er_check_nonzero eax, .memory_trap
    wasm_exec_save_reader_offset
    call    exec_stack_pop      ; n
    jc      .underflow_error
    mov     r14, rax
    call    exec_stack_pop      ; s
    jc      .underflow_error
    mov     r13, rax
    call    exec_stack_pop      ; d
    jc      .underflow_error
    mov     r12, rax
    cmp     r15, [element_segment_count]
    jae     .memory_trap
    mov     r10, r15
    imul    r10, ELEMENT_SEGMENT_SIZE
    cmp     byte [element_segments + r10 + 264], 1
    jne     .memory_trap
    cmp     byte [element_segments + r10 + 265], 0
    jne     .memory_trap
    mov     rax, r13
    add     rax, r14
    jc      .memory_trap
    cmp     rax, [element_segments + r10 + 256]
    ja      .memory_trap
    mov     rax, r12
    add     rax, r14
    jc      .memory_trap
    cmp     rax, [table_min]
    ja      .memory_trap
    xor     r11d, r11d
.table_init_loop:
    cmp     r11, r14
    jae     .dispatch_next
    mov     rax, [element_segments + r10 + r13 * 8]
    mov     [table_entries + r12 * 8], rax
    inc     r13
    inc     r12
    inc     r11
    jmp     .table_init_loop

.ext_elem_drop:
    wasm_exec_reader_ptr
    call    er_wasm_read_leb_u32
    er_check_nonzero rdx, .corrupt_error
    wasm_exec_save_reader_offset
    cmp     rax, [element_segment_count]
    jae     .memory_trap
    mov     r10, rax
    imul    r10, ELEMENT_SEGMENT_SIZE
    cmp     byte [element_segments + r10 + 264], 1
    jne     .memory_trap
    mov     byte [element_segments + r10 + 265], 1
    jmp     .dispatch_next

.ext_table_copy:
    wasm_exec_reader_ptr
    call    er_wasm_read_leb_u32
    er_check_nonzero rdx, .corrupt_error
    er_check_nonzero eax, .memory_trap
    wasm_exec_save_reader_offset
    wasm_exec_reader_ptr
    call    er_wasm_read_leb_u32
    er_check_nonzero rdx, .corrupt_error
    er_check_nonzero eax, .memory_trap
    wasm_exec_save_reader_offset
    call    exec_stack_pop      ; n
    jc      .underflow_error
    mov     r14, rax
    call    exec_stack_pop      ; s
    jc      .underflow_error
    mov     r13, rax
    call    exec_stack_pop      ; d
    jc      .underflow_error
    mov     r12, rax
    mov     rax, r13
    add     rax, r14
    jc      .memory_trap
    cmp     rax, [table_min]
    ja      .memory_trap
    mov     rax, r12
    add     rax, r14
    jc      .memory_trap
    cmp     rax, [table_min]
    ja      .memory_trap
    er_check_zero r14, .dispatch_next
    cmp     r12, r13
    ja      .table_copy_backward
.table_copy_forward:
    mov     rax, [table_entries + r13 * 8]
    mov     [table_entries + r12 * 8], rax
    inc     r13
    inc     r12
    dec     r14
    jnz     .table_copy_forward
    jmp     .dispatch_next
.table_copy_backward:
    add     r13, r14
    add     r12, r14
.table_copy_backward_loop:
    dec     r13
    dec     r12
    mov     rax, [table_entries + r13 * 8]
    mov     [table_entries + r12 * 8], rax
    dec     r14
    jnz     .table_copy_backward_loop
    jmp     .dispatch_next

.ext_table_grow:
    wasm_exec_reader_ptr
    call    er_wasm_read_leb_u32
    er_check_nonzero rdx, .corrupt_error
    er_check_nonzero eax, .memory_trap
    wasm_exec_save_reader_offset
    call    exec_stack_pop      ; n
    jc      .underflow_error
    mov     r14, rax
    call    exec_stack_pop      ; init value
    jc      .underflow_error
    mov     r13, rax
    mov     r12, [table_min]
    mov     rax, r12
    add     rax, r14
    jc      .table_grow_fail
    cmp     rax, MAX_TABLE_ENTRIES
    ja      .table_grow_fail
    mov     rcx, [table_max]
    er_check_zero rcx, .table_grow_allowed
    cmp     rax, rcx
    ja      .table_grow_fail
.table_grow_allowed:
    mov     r11, r12
.table_grow_loop:
    cmp     r11, rax
    jae     .table_grow_done
    mov     [table_entries + r11 * 8], r13
    inc     r11
    jmp     .table_grow_loop
.table_grow_done:
    mov     [table_min], rax
    mov     rax, r12
    call    exec_stack_push
    jc      .overflow_error
    jmp     .dispatch_next
.table_grow_fail:
    mov     rax, -1
    call    exec_stack_push
    jc      .overflow_error
    jmp     .dispatch_next

.ext_table_size:
    wasm_exec_reader_ptr
    call    er_wasm_read_leb_u32
    er_check_nonzero rdx, .corrupt_error
    er_check_nonzero eax, .memory_trap
    wasm_exec_save_reader_offset
    mov     rax, [table_min]
    call    exec_stack_push
    jc      .overflow_error
    jmp     .dispatch_next

.ext_table_fill:
    wasm_exec_reader_ptr
    call    er_wasm_read_leb_u32
    er_check_nonzero rdx, .corrupt_error
    er_check_nonzero eax, .memory_trap
    wasm_exec_save_reader_offset
    call    exec_stack_pop      ; n
    jc      .underflow_error
    mov     r14, rax
    call    exec_stack_pop      ; val
    jc      .underflow_error
    mov     r13, rax
    call    exec_stack_pop      ; i
    jc      .underflow_error
    mov     r12, rax
    mov     rax, r12
    add     rax, r14
    jc      .memory_trap
    cmp     rax, [table_min]
    ja      .memory_trap
.table_fill_loop:
    er_check_zero r14, .dispatch_next
    mov     [table_entries + r12 * 8], r13
    inc     r12
    dec     r14
    jmp     .table_fill_loop

; ==================================================================
; Opcode immediate skipping helper
; =================================================================+
.skip_opcode_immediates:
    ; ebx = opcode byte
    movzx   ebx, bl
    ; Most opcodes have no immediates
    ; Check opcodes that DO have immediates
    cmp     bl, 0x02         ; block
    je      .skip_block_type
    cmp     bl, 0x03         ; loop
    je      .skip_block_type
    cmp     bl, 0x04         ; if
    je      .skip_block_type
    cmp     bl, 0x0c         ; br
    je      .skip_leb
    cmp     bl, 0x0d         ; br_if
    je      .skip_leb
    cmp     bl, 0x10         ; call
    je      .skip_leb
    cmp     bl, 0x11         ; call_indirect
    je      .skip_call_indirect
    cmp     bl, 0x20         ; local.get
    je      .skip_leb
    cmp     bl, 0x21         ; local.set
    je      .skip_leb
    cmp     bl, 0x22         ; local.tee
    je      .skip_leb
    cmp     bl, 0x23         ; global.get
    je      .skip_leb
    cmp     bl, 0x24         ; global.set
    je      .skip_leb
    cmp     bl, 0x41         ; i32.const
    je      .skip_leb_i32
    cmp     bl, 0x42         ; i64.const
    je      .skip_leb_i64
    cmp     bl, 0x0e         ; br_table
    je      .skip_br_table
    cmp     bl, 0x1c         ; select_typed
    je      .skip_select_typed
    cmp     bl, 0x28         ; i32.load etc.
    jb      .skip_no_immediate
    cmp     bl, 0x3f         ; below memory_size
    jb      .skip_load_store
    cmp     bl, 0x40         ; memory.size/grow
    jbe     .skip_leb
    cmp     bl, 0xd0         ; ref.null
    je      .skip_byte
    cmp     bl, 0xd2         ; ref.func
    je      .skip_leb
    ; For most opcodes, no immediates to skip
.skip_no_immediate:
    ret

.skip_leb:
    wasm_exec_reader_ptr
    call    er_wasm_read_leb_u32
    er_check_nonzero rdx, .skip_error
    wasm_exec_save_reader_offset
    ret

.skip_leb_i32:
    wasm_exec_reader_ptr
    call    er_wasm_read_leb_i32
    er_check_nonzero rdx, .skip_error
    wasm_exec_save_reader_offset
    ret

.skip_leb_i64:
    wasm_exec_reader_ptr
    call    er_wasm_read_leb_i64
    er_check_nonzero rdx, .skip_error
    wasm_exec_save_reader_offset
    ret

.skip_block_type:
    call    reader_read_byte
    jc      .skip_error
    ret

.skip_byte:
    call    reader_read_byte
    jc      .skip_error
    ret

.skip_call_indirect:
    wasm_exec_reader_ptr
    call    er_wasm_read_leb_u32    ; type_index
    er_check_nonzero rdx, .skip_error
    call    er_wasm_read_leb_u32    ; table_index
    er_check_nonzero rdx, .skip_error
    wasm_exec_save_reader_offset
    ret

.skip_load_store:
    ; Two LEBs: alignment then offset
    wasm_exec_reader_ptr
    call    er_wasm_read_leb_u32
    er_check_nonzero rdx, .skip_error
    wasm_exec_save_reader_offset
    wasm_exec_reader_ptr
    call    er_wasm_read_leb_u32
    er_check_nonzero rdx, .skip_error
    wasm_exec_save_reader_offset
    ret

.skip_br_table:
    ; Read target count, then targets, then default
    wasm_exec_reader_ptr
    call    er_wasm_read_leb_u32
    er_check_nonzero rdx, .skip_error
    wasm_exec_save_reader_offset
    mov     r15d, eax      ; target count
.skip_br_table_loop:
    er_check_zero r15d, .skip_br_table_default
    wasm_exec_reader_ptr
    call    er_wasm_read_leb_u32
    er_check_nonzero rdx, .skip_error
    wasm_exec_save_reader_offset
    dec     r15d
    jmp     .skip_br_table_loop
.skip_br_table_default:
    wasm_exec_reader_ptr
    call    er_wasm_read_leb_u32
    er_check_nonzero rdx, .skip_error
    wasm_exec_save_reader_offset
    ret

.skip_select_typed:
    wasm_exec_reader_ptr
    call    er_wasm_read_leb_u32
    er_check_nonzero rdx, .skip_error
    cmp     eax, 1
    jne     .skip_error
    cmp     byte [rsi], 0x7f
    jne     .skip_error
    inc     rsi
    wasm_exec_save_reader_offset
    ret

.skip_error:
    ; On skip error, just return (will be caught later)
    ret

; ==================================================================
; Function return handling
; =================================================================+
.function_return:
    ; Collect results from stack based on result count
    mov     rcx, [exec_result_count]
    er_check_zero rcx, .return_done
    mov     r15, rcx
    ; Pop results in reverse order
.return_collect:
    cmp     r15, 0
    je      .return_done
    dec     r15
    call    exec_stack_pop
    jc      .underflow_error
    mov     [exec_result_values + r15 * 8], rax
    jmp     .return_collect
.return_done:
    ; If this is the outermost call (stack_len was 0 before call), exit loop
    ; Otherwise, the result will be pushed back by caller
    er_ok
    pop     rbp
    ret

; ==================================================================
; Error exit points
; =================================================================+
.corrupt_error:
    er_err  ERROR_CORRUPT
    jmp     .error_return
.underflow_error:
    er_err  ERROR_STACK_UNDERFLOW
    jmp     .error_return
.overflow_error:
    er_err  ERROR_STACK_OVERFLOW
    jmp     .error_return
.unsupported_error:
    er_err  ERROR_UNSUPPORTED
    jmp     .error_return
.arithmetic_trap:
    er_err  ERROR_ARITHMETIC_TRAP
    jmp     .error_return
.memory_trap:
    er_err  ERROR_TRAP
    jmp     .error_return

; ==================================================================
; Outer dispatch loop exit (error)
; =================================================================+
.done:
    er_ok
    pop     rbp
    ret

.error_return:
    ; rdx set by caller
    mov     rax, -1
    pop     rbp
    ret
