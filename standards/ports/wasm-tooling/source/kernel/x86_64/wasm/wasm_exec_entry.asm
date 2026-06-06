; ==================================================================
; Set up frame for a function
; rdi = function_index
; rsi = args pointer (array of int64)
; rdx = args count
; =================================================================+
er_fn er_fn_exec
    er_frame_push_regs rbx, r12, r13, r14, r15
    sub     rsp, 8

    mov     r12, rdi        ; function_index
    mov     r13, rsi        ; args ptr
    mov     r14, rdx        ; args count
    mov     qword [rbp - 48], 0

    cmp     qword [exec_call_depth], 0
    je      .no_save
    call    exec_save_frame_state
    jc      .depth_error_no_frame
    mov     qword [rbp - 48], 1
.no_save:
    inc     qword [exec_call_depth]

    ; Check if imported function
    mov     rax, r12
    cmp     rax, [import_count]
    jb      .imported_func

    ; Defined function
    mov     rdi, r12               ; pass absolute function_index
    call    er_wasm_code_index_for_function
    er_check_nonzero rdx, .error

    mov     r15, rax               ; code_index

    ; Get code info
    ; Code struct: body_offset(8) + body_len(8) + local_count(8) + decoded_start(8) + decoded_count(8)
    push    r10
    mov     r10, r15
    imul    r10, CODE_SIZE
    mov     rax, [code_buf + r10]      ; body_offset (from code body start)
    mov     [exec_code_body_ptr], rax
    mov     rax, [code_buf + r10 + 8]  ; body_len
    mov     [exec_code_body_len], rax
    mov     rax, [code_buf + r10 + 16] ; local_count
    mov     rbx, rax                   ; local_count
    mov     rax, [code_buf + r10 + 24] ; decoded_start
    mov     [exec_decoded_index], rax
    mov     rax, [code_buf + r10 + 32] ; decoded_count
    add     rax, [exec_decoded_index]
    mov     [exec_decoded_end], rax
    pop     r10

    ; Get function type (pass absolute function_index)
    mov     rdi, r12
    call    er_wasm_type_index_for_function
    er_check_nonzero rdx, .error
    ; Store type info for result handling
    mov     [exec_type_index], rax

    ; param_count + result_count from FuncType
    push    r10
    mov     r10, rax
    imul    r10, FUNC_TYPE_SIZE
    mov     rax, [types_buf + r10 + FUNC_TYPE_PARAM_COUNT_OFF]  ; param_count
    mov     rcx, rax
    mov     rax, [types_buf + r10 + FUNC_TYPE_RESULT_COUNT_OFF] ; result_count
    mov     [exec_result_count], rax
    pop     r10

    ; Local count = param_count + code local_count
    ; The code local_count is the number of zero-initialized locals after params
    ; rbx already has local_count from code struct
    mov     rax, rcx     ; param_count
    add     rax, rbx     ; + code local_count
    mov     [exec_local_count], rax
    cmp     rax, MAX_LOCALS
    ja      .unsupported_error

    ; Initialize locals from args
    xor     r15d, r15d
.init_locals_loop:
    cmp     r15, rcx     ; param_count
    jae     .init_zero
    ; Copy arg
    mov     rax, [r13 + r15 * 8]
    mov     [exec_locals + r15 * 8], rax
    inc     r15
    jmp     .init_locals_loop
.init_zero:
    ; Zero remaining locals
    cmp     r15, [exec_local_count]
    jae     .init_done
    mov     qword [exec_locals + r15 * 8], 0
    inc     r15
    jmp     .init_zero
.init_done:
    ; Clear value stack and control stack
    mov     qword [exec_stack_len], 0
    mov     qword [exec_control_len], 0
    mov     qword [exec_reader_offset], 0

    mov     rax, [exec_decoded_index]
    cmp     rax, [exec_fast_cache_start]
    jne     .fast_cache_miss
    mov     rax, [exec_decoded_end]
    cmp     rax, [exec_fast_cache_end]
    jne     .fast_cache_miss
    mov     rax, [exec_fast_cache_state]
    cmp     rax, 1
    je      .dispatch_fast
    cmp     rax, 2
    je      .dispatch_full
    cmp     rax, 3
    je      .dispatch_trace

.fast_cache_miss:
    mov     rax, [exec_decoded_index]
    mov     [exec_fast_cache_start], rax
    mov     rax, [exec_decoded_end]
    mov     [exec_fast_cache_end], rax
    call    exec_decoded_round_trace_supported
    er_check_nonzero rdx, .fast_check_supported
    mov     qword [exec_fast_cache_state], 3
.dispatch_trace:
    call    exec_decoded_round_trace_loop
    jmp     .dispatch_done

.fast_check_supported:
    call    exec_decoded_fast_supported
    er_check_nonzero rdx, .fast_mark_unsupported
    mov     qword [exec_fast_cache_state], 1
.dispatch_fast:
    call    exec_decoded_fast_loop
    jmp     .dispatch_done

.fast_mark_unsupported:
    mov     qword [exec_fast_cache_state], 2
.dispatch_full:
    call    exec_dispatch_loop

.dispatch_done:
    mov     r15, rax      ; save return value
    mov     r14, rdx      ; save error code

.after_dispatch:
    jmp     .finish_entered

.imported_func:
    ; Imported function - call through host import dispatch
    call    er_wasm_call_imported
    mov     r15, rax
    mov     r14, rdx
    jmp     .finish_entered

.depth_error:
    er_err  ERROR_UNSUPPORTED
    xor     r15d, r15d
    mov     r14, rdx
    jmp     .finish_entered
.depth_error_no_frame:
    er_err  ERROR_UNSUPPORTED
    jmp     .done_no_frame
.unsupported_error:
    er_err  ERROR_UNSUPPORTED
    xor     r15d, r15d
    mov     r14, rdx
    jmp     .finish_entered
.error:
    ; rdx already set
    xor     r15d, r15d
    mov     r14, rdx
.finish_entered:
    dec     qword [exec_call_depth]
    cmp     qword [rbp - 48], 0
    je      .after_restore
    call    exec_restore_frame_state
.after_restore:
    mov     rax, r15
    mov     rdx, r14
.done_no_frame:
    add     rsp, 8
    er_pop_ret rbp, rbx, r12, r13, r14, r15

; ==================================================================
; Imported function dispatch
; rdi = function_index (absolute, < import_count)
; =================================================================+
er_wasm_call_imported:
    er_frame_push_regs r15, r14

    mov     r10, rdi
    imul    r10, IMPORTED_FUNC_SIZE

    ; Get resolved host import index and convert to function pointer
    mov     r15, [imports_buf + r10 + IMPORT_RESOLVED_FUNC_IDX_OFF]
    mov     rax, r15
    imul    rax, HOST_IMPORT_SIZE
    add     rax, [runtime_imports_ptr]
    mov     r15, [rax + HOST_IMPORT_FN_PTR_OFF]

    ; Get type info for param/result count
    mov     rax, [imports_buf + r10 + IMPORT_TYPE_INDEX_OFF]
    mov     r10, rax
    imul    r10, FUNC_TYPE_SIZE
    mov     r14, [types_buf + r10 + FUNC_TYPE_PARAM_COUNT_OFF]   ; param_count
    mov     rcx, [types_buf + r10 + FUNC_TYPE_RESULT_COUNT_OFF]  ; result_count

    ; Pop args from eval stack (WASM order: last pushed = last param)
    ; For x86_64 calling convention: arg0 = rdi, arg1 = rsi, arg2 = rdx, arg3 = rcx, arg4 = r8, arg5 = r9
    xor     edi, edi
    xor     esi, esi
    xor     edx, edx
    xor     ecx, ecx
    xor     r8d, r8d
    xor     r9d, r9d

    cmp     r14, 0
    je      .import_do_call
    cmp     r14, 1
    je      .import_one_arg
    cmp     r14, 2
    je      .import_two_args

    ; >2 args not supported yet
    er_err  ERROR_UNSUPPORTED
    er_pop_ret rbp, r15, r14

.import_two_args:
    call    exec_stack_pop
    jc      .import_underflow
    mov     rsi, rax        ; arg1 = second param

.import_one_arg:
    call    exec_stack_pop
    jc      .import_underflow
    mov     rdi, rax        ; arg0 = first param

.import_do_call:
    ; Call host function
    call    r15

    ; Push result(s) to eval stack
    er_check_zero rcx, .import_no_result
    mov     rdi, rax
    call    exec_stack_push
    jc      .import_overflow

.import_no_result:
    xor     edx, edx        ; ERROR_SUCCESS
    er_pop_ret rbp, r15, r14

.import_underflow:
    er_err  ERROR_STACK_UNDERFLOW
    er_pop_ret rbp, r15, r14

.import_overflow:
    er_err  ERROR_STACK_OVERFLOW
    er_pop_ret rbp, r15, r14
