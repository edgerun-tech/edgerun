extern er_memmove
extern er_memset

%macro wasm_exec_save_reader_offset 0
    push    rsi
    sub     rsi, [exec_code_body_ptr]
    mov     [exec_reader_offset], rsi
    pop     rsi
%endm

%macro wasm_exec_reader_ptr 0
    mov     rsi, [exec_code_body_ptr]
    add     rsi, [exec_reader_offset]
%endm

er_fn er_fn_pop
    er_frame_push

    mov     rax, [frame_stack_len]
    er_check_zero rax, .underflow

    dec     rax
    mov     [frame_stack_len], rax

    ; Load value data (16 bytes per entry)
    shl     rax, 4
    mov     rax, [frame_stack + rax]      ; value data
    pop     rbp
    ret

.underflow:
    er_err  ERROR_STACK_UNDERFLOW
    mov     rax, -1
    pop     rbp
    ret

; ==================================================================
; Push value to frame stack
; rdi = value data
; =================================================================+
er_fn er_fn_push
    er_frame_push

    mov     rax, [frame_stack_len]
    cmp     rax, MAX_STACK
    jae     .overflow

    shl     rax, 4
    mov     [frame_stack + rax], rdi      ; value data

    inc     qword [frame_stack_len]
    xor     eax, eax
    pop     rbp
    ret

.overflow:
    er_err  ERROR_STACK_OVERFLOW
    mov     rax, -1
    pop     rbp
    ret

; ==================================================================
; Executor entry point
; er_fn_run(runtime_ptr=rdi, wasm_bytes_ptr=rsi, wasm_bytes_len=rdx, export_name_ptr=rcx, export_name_len=r8)
; =================================================================+
; ==================================================================
; Frame save/restore helpers
; =================================================================+

; Save current frame state onto exec_frame_save area
; Clobbers: rax, rcx
; Returns: carry set if save area overflow
er_fn exec_save_frame_state
    er_frame_push

    mov     rax, [exec_frame_save_ptr]
    mov     rcx, [exec_local_count]

    ; Check space: local_count*8 + stack_len*8 + control_len*16 + 80 <= FRAME_SAVE_SIZE - save_ptr
    push    rcx
    imul    rcx, 8
    add     rax, rcx
    pop     rcx
    mov     rcx, [exec_stack_len]
    shl     rcx, 3
    add     rax, rcx
    mov     rcx, [exec_control_len]
    imul    rcx, CONTROL_FRAME_SIZE
    add     rax, rcx
    add     rax, 80
    cmp     rax, FRAME_SAVE_SIZE
    ja      .overflow

    ; Push locals: exec_locals[0..local_count] onto save area
    mov     rcx, [exec_local_count]
    er_check_zero rcx, .save_stack
    mov     rax, exec_locals
    mov     rdx, [exec_frame_save_ptr]
.save_locals_loop:
    mov     r8, [rax]
    mov     [exec_frame_save + rdx], r8
    add     rax, 8
    add     rdx, 8
    dec     rcx
    jnz     .save_locals_loop
    mov     [exec_frame_save_ptr], rdx

.save_stack:
    ; Push stack entries
    mov     rcx, [exec_stack_len]
    er_check_zero rcx, .save_control
    mov     rax, exec_stack
    mov     rdx, [exec_frame_save_ptr]
.save_stack_loop:
    mov     r8, [rax]
    mov     [exec_frame_save + rdx], r8
    add     rax, 8
    add     rdx, 8
    dec     rcx
    jnz     .save_stack_loop
    mov     [exec_frame_save_ptr], rdx

.save_control:
    ; Push control frames
    mov     rcx, [exec_control_len]
    er_check_zero rcx, .save_meta
    mov     rax, exec_control
    mov     rdx, [exec_frame_save_ptr]
.save_control_loop:
    mov     r8, [rax]
    mov     r9, [rax + 8]
    mov     [exec_frame_save + rdx], r8
    mov     [exec_frame_save + rdx + 8], r9
    add     rax, CONTROL_FRAME_SIZE
    add     rdx, CONTROL_FRAME_SIZE
    dec     rcx
    jnz     .save_control_loop
    mov     [exec_frame_save_ptr], rdx

.save_meta:
    ; Push frame metadata.
    mov     rdx, [exec_frame_save_ptr]
    mov     rax, [exec_local_count]
    mov     [exec_frame_save + rdx], rax
    mov     rax, [exec_stack_len]
    mov     [exec_frame_save + rdx + 8], rax
    mov     rax, [exec_control_len]
    mov     [exec_frame_save + rdx + 16], rax
    mov     rax, [exec_decoded_index]
    mov     [exec_frame_save + rdx + 24], rax
    mov     rax, [exec_decoded_end]
    mov     [exec_frame_save + rdx + 32], rax
    mov     rax, [exec_type_index]
    mov     [exec_frame_save + rdx + 40], rax
    mov     rax, [exec_code_body_ptr]
    mov     [exec_frame_save + rdx + 48], rax
    mov     rax, [exec_code_body_len]
    mov     [exec_frame_save + rdx + 56], rax
    mov     rax, [exec_reader_offset]
    mov     [exec_frame_save + rdx + 64], rax
    mov     rax, [exec_result_count]
    mov     [exec_frame_save + rdx + 72], rax
    add     qword [exec_frame_save_ptr], 80
    clc
    pop     rbp
    ret

.overflow:
    er_err  ERROR_NO_MEMORY
    stc
    pop     rbp
    ret

; Restore frame state from exec_frame_save area
; Clobbers: rax, rcx
er_fn exec_restore_frame_state
    er_frame_push

    ; First read metadata at the end
    mov     rax, [exec_frame_save_ptr]
    sub     rax, 80
    mov     rcx, [exec_frame_save + rax]
    mov     [exec_local_count], rcx
    mov     rcx, [exec_frame_save + rax + 8]
    mov     [exec_stack_len], rcx
    mov     rcx, [exec_frame_save + rax + 16]
    mov     [exec_control_len], rcx
    mov     rcx, [exec_frame_save + rax + 24]
    mov     [exec_decoded_index], rcx
    mov     rcx, [exec_frame_save + rax + 32]
    mov     [exec_decoded_end], rcx
    mov     rcx, [exec_frame_save + rax + 40]
    mov     [exec_type_index], rcx
    mov     rcx, [exec_frame_save + rax + 48]
    mov     [exec_code_body_ptr], rcx
    mov     rcx, [exec_frame_save + rax + 56]
    mov     [exec_code_body_len], rcx
    mov     rcx, [exec_frame_save + rax + 64]
    mov     [exec_reader_offset], rcx
    mov     rcx, [exec_frame_save + rax + 72]
    mov     [exec_result_count], rcx
    mov     [exec_frame_save_ptr], rax

    ; Pop control frames
    mov     rcx, [exec_control_len]
    er_check_zero rcx, .rest_stack
    mov     rdx, [exec_frame_save_ptr]
.rest_control_loop:
    sub     rdx, CONTROL_FRAME_SIZE
    mov     r8, [exec_frame_save + rdx]
    mov     r9, [exec_frame_save + rdx + 8]
    mov     rax, exec_control
    push    rcx
    imul    rcx, CONTROL_FRAME_SIZE
    add     rax, rcx
    pop     rcx
    sub     rax, CONTROL_FRAME_SIZE
    mov     [rax], r8
    mov     [rax + 8], r9
    dec     rcx
    jnz     .rest_control_loop
    mov     [exec_frame_save_ptr], rdx

.rest_stack:
    ; Pop stack entries
    mov     rcx, [exec_stack_len]
    er_check_zero rcx, .rest_locals
    mov     rdx, [exec_frame_save_ptr]
.rest_stack_loop:
    sub     rdx, 8
    mov     r8, [exec_frame_save + rdx]
    mov     rax, exec_stack
    push    rcx
    shl     rcx, 3
    add     rax, rcx
    pop     rcx
    sub     rax, 8
    mov     [rax], r8
    dec     rcx
    jnz     .rest_stack_loop
    mov     [exec_frame_save_ptr], rdx

.rest_locals:
    ; Pop locals
    mov     rcx, [exec_local_count]
    er_check_zero rcx, .done_rest
    mov     rdx, [exec_frame_save_ptr]
.rest_locals_loop:
    sub     rdx, 8
    mov     r8, [exec_frame_save + rdx]
    mov     rax, exec_locals
    push    rcx
    shl     rcx, 3
    add     rax, rcx
    pop     rcx
    sub     rax, 8
    mov     [rax], r8
    dec     rcx
    jnz     .rest_locals_loop
    mov     [exec_frame_save_ptr], rdx

.done_rest:
    er_ok
    pop     rbp
    ret

; ==================================================================
; Stack operations
; =================================================================+

; Pop a value from exec_stack into rax
; Returns carry set on underflow
er_fn exec_stack_pop
    er_frame_push
    mov     rax, [exec_stack_len]
    er_check_zero rax, .underflow
    dec     rax
    mov     [exec_stack_len], rax
    mov     rax, [exec_stack + rax * 8]
    clc
    pop     rbp
    ret
.underflow:
    er_err  ERROR_STACK_UNDERFLOW
    stc
    pop     rbp
    ret

; Push rax onto exec_stack
; Returns carry set on overflow
er_fn exec_stack_push
    er_frame_push
    mov     rcx, [exec_stack_len]
    cmp     rcx, MAX_STACK
    jae     .overflow
    mov     [exec_stack + rcx * 8], rax
    inc     qword [exec_stack_len]
    clc
    pop     rbp
    ret
.overflow:
    er_err  ERROR_STACK_OVERFLOW
    stc
    pop     rbp
    ret

; Peek at stack top (no pop)
; Returns value in rax
er_fn exec_stack_peek
    mov     rax, [exec_stack_len]
    er_check_zero rax, .underflow
    mov     rax, [exec_stack + rax * 8 - 8]
    clc
    ret
.underflow:
    stc
    ret

; Duplicate stack top
er_fn exec_stack_dup
    er_frame_push
    call    exec_stack_peek
    jc      .done
    call    exec_stack_push
.done:
    pop     rbp
    ret

; ==================================================================
; Error handler
; Sets error code and returns from enclosing function
; Expects error code in rdx
; =================================================================+
er_fn exec_error
    pop     rbp         ; pop return address of the caller
    pop     rbp         ; restore parent frame
    ret

; ==================================================================
; Read current reader byte without advancing
; =================================================================+
er_fn reader_peek_byte
    mov     rax, [exec_reader_offset]
    cmp     rax, [exec_code_body_len]
    jae     .done
    mov     rsi, [exec_code_body_ptr]
    mov     al, [rsi + rax]
    er_ok
    clc
    ret
.done:
    stc
    ret

; Advance reader by amount in rdi
er_fn reader_advance
    mov     rax, [exec_reader_offset]
    add     rax, rdi
    mov     [exec_reader_offset], rax
    ret

; Read a byte from reader and advance
; Returns byte in al, carry set if past end
er_fn reader_read_byte
    er_frame_push
    call    reader_peek_byte
    jc      .done
    inc     qword [exec_reader_offset]
    clc
.done:
    pop     rbp
    ret
er_fn exec_control_push
    pop     rax     ; return address
    pop     rdx     ; kind
    pop     rcx     ; start
    push    rax     ; restore return address
    mov     rax, [exec_control_len]
    cmp     rax, MAX_CONTROL_DEPTH
    jae     .overflow
    push    r10
    mov     r10, rax
    imul    r10, CONTROL_FRAME_SIZE
    mov     [exec_control + r10], rdx      ; kind
    mov     [exec_control + r10 + 8], rcx  ; start
    pop     r10
    inc     qword [exec_control_len]
    ret
.overflow:
    er_err  ERROR_UNSUPPORTED
    ; Skip the pushed values
    ret

; Branch to control at given depth
; rdi = branch_depth from current position (0 = innermost)
er_fn exec_branch_to_control
    er_frame_push
    mov     r11, rdi        ; preserve branch_depth for forward block skip
    ; target_index = control_len - 1 - branch_depth
    mov     rax, [exec_control_len]
    er_check_zero rax, .empty_error
    sub     rax, 1
    sub     rax, rdi
    jc      .depth_error
    ; Get control frame at target_index
    push    r10
    mov     r10, rax
    imul    r10, CONTROL_FRAME_SIZE
    mov     rcx, [exec_control + r10]        ; kind
    mov     rdx, [exec_control + r10 + 8]   ; start
    pop     r10
    ; Set control length = target_index + 1 (for loop) or target_index (for block)
    mov     r8, rax         ; target_index
    cmp     rcx, CONTROL_LOOP
    je      .branch_loop
    ; block/if_then/if_else
.branch_block:
    mov     [exec_control_len], r8
    mov     rdi, r11
    call    exec_skip_forward_to_block_end
    er_check_nonzero rdx, .return_error
    jmp     .branch_done
.branch_loop:
    add     r8, 1
    mov     [exec_control_len], r8
    mov     [exec_reader_offset], rdx
.branch_done:
    er_ok
    pop     rbp
    ret
.error:
    er_err  ERROR_CORRUPT
    pop     rbp
    ret
.return_error:
    pop     rbp
    ret
.empty_error:
    er_err  ERROR_CORRUPT
    pop     rbp
    ret
.depth_error:
    er_err  ERROR_CORRUPT
    pop     rbp
    ret

; Skip forward after a block/if branch until the selected target block end.
; rdi = branch_depth from the branch site (0 = innermost target)
er_fn exec_skip_forward_to_block_end
    er_frame_push_regs rbx, r12, r13, r14, r15
    mov     r12, rdi
    inc     r12             ; number of outer end opcodes to consume
    xor     r13d, r13d      ; nested control depth inside skipped range
.scan_loop:
    call    reader_read_byte
    jc      .error
    movzx   ebx, al
    cmp     bl, 0x02        ; block
    je      .scan_block
    cmp     bl, 0x03        ; loop
    je      .scan_block
    cmp     bl, 0x04        ; if
    je      .scan_block
    cmp     bl, 0x0b        ; end
    je      .scan_end
    call    exec_skip_opcode_immediates_forward
    er_check_nonzero rdx, .error
    jmp     .scan_loop
.scan_block:
    inc     r13
    call    reader_read_byte ; block type
    jc      .error
    jmp     .scan_loop
.scan_end:
    er_check_zero r13, .outer_end
    dec     r13
    jmp     .scan_loop
.outer_end:
    dec     r12
    er_check_zero r12, .done
    jmp     .scan_loop
.done:
    er_ok
    er_pop_ret rbp, rbx, r12, r13, r14, r15
.error:
    er_err  ERROR_CORRUPT
    er_pop_ret rbp, rbx, r12, r13, r14, r15

; Skip immediates for one already-read opcode in bl.
er_fn exec_skip_opcode_immediates_forward
    er_frame_push_regs r12, r15
    cmp     bl, 0x0c        ; br
    je      .skip_leb
    cmp     bl, 0x0d        ; br_if
    je      .skip_leb
    cmp     bl, 0x0e        ; br_table
    je      .skip_br_table
    cmp     bl, 0x10        ; call
    je      .skip_leb
    cmp     bl, 0x11        ; call_indirect
    je      .skip_call_indirect
    cmp     bl, 0x1c        ; select_typed
    je      .skip_select_typed
    cmp     bl, 0x20        ; local.get
    je      .skip_leb
    cmp     bl, 0x21        ; local.set
    je      .skip_leb
    cmp     bl, 0x22        ; local.tee
    je      .skip_leb
    cmp     bl, 0x23        ; global.get
    je      .skip_leb
    cmp     bl, 0x24        ; global.set
    je      .skip_leb
    cmp     bl, 0x28        ; i32.load etc.
    jb      .done
    cmp     bl, 0x3f        ; below memory.size
    jb      .skip_load_store
    cmp     bl, 0x40        ; memory.size/grow
    jbe     .skip_leb
    cmp     bl, 0x41        ; i32.const
    je      .skip_leb_i32
    cmp     bl, 0x42        ; i64.const
    je      .skip_leb_i64
    cmp     bl, 0xd0        ; ref.null
    je      .skip_byte
    cmp     bl, 0xd2        ; ref.func
    je      .skip_leb
    jmp     .done
.skip_leb:
    wasm_exec_reader_ptr
    call    er_wasm_read_leb_u32
    er_check_nonzero rdx, .error
    wasm_exec_save_reader_offset
    jmp     .done
.skip_leb_i32:
    wasm_exec_reader_ptr
    call    er_wasm_read_leb_i32
    er_check_nonzero rdx, .error
    wasm_exec_save_reader_offset
    jmp     .done
.skip_leb_i64:
    wasm_exec_reader_ptr
    call    er_wasm_read_leb_i64
    er_check_nonzero rdx, .error
    wasm_exec_save_reader_offset
    jmp     .done
.skip_byte:
    call    reader_read_byte
    jc      .error
    jmp     .done
.skip_call_indirect:
    wasm_exec_reader_ptr
    call    er_wasm_read_leb_u32
    er_check_nonzero rdx, .error
    call    er_wasm_read_leb_u32
    er_check_nonzero rdx, .error
    wasm_exec_save_reader_offset
    jmp     .done
.skip_load_store:
    wasm_exec_reader_ptr
    call    er_wasm_read_leb_u32
    er_check_nonzero rdx, .error
    wasm_exec_save_reader_offset
    wasm_exec_reader_ptr
    call    er_wasm_read_leb_u32
    er_check_nonzero rdx, .error
    wasm_exec_save_reader_offset
    jmp     .done
.skip_br_table:
    wasm_exec_reader_ptr
    call    er_wasm_read_leb_u32
    er_check_nonzero rdx, .error
    wasm_exec_save_reader_offset
    mov     r15d, eax
.skip_br_table_loop:
    er_check_zero r15d, .skip_br_table_default
    wasm_exec_reader_ptr
    call    er_wasm_read_leb_u32
    er_check_nonzero rdx, .error
    wasm_exec_save_reader_offset
    dec     r15d
    jmp     .skip_br_table_loop
.skip_br_table_default:
    wasm_exec_reader_ptr
    call    er_wasm_read_leb_u32
    er_check_nonzero rdx, .error
    wasm_exec_save_reader_offset
    jmp     .done
.skip_select_typed:
    wasm_exec_reader_ptr
    call    er_wasm_read_leb_u32
    er_check_nonzero rdx, .error
    cmp     eax, 1
    jne     .error
    cmp     byte [rsi], 0x7f
    jne     .error
    inc     rsi
    wasm_exec_save_reader_offset
    jmp     .done
.done:
    er_ok
    er_pop_ret rbp, r12, r15
.error:
    er_err  ERROR_CORRUPT
    er_pop_ret rbp, r12, r15
; Helper: read alignment and offset immediates, compute address
; Returns address in rax
er_fn exec_memory_prepare
    ; Read alignment (LEB, skip it)
    wasm_exec_reader_ptr
    call    er_wasm_read_leb_u32
    er_check_nonzero rdx, .corrupt_mem
    wasm_exec_save_reader_offset
    ; Read offset (LEB)
    wasm_exec_reader_ptr
    call    er_wasm_read_leb_u32
    er_check_nonzero rdx, .corrupt_mem
    wasm_exec_save_reader_offset
    mov     r15d, eax      ; offset
    ; Pop base address from stack
    call    exec_stack_pop
    jc      .underflow_mem
    mov     ecx, eax        ; base (lower 32 bits)
    ; Compute final address = base + offset
    mov     eax, ecx
    add     eax, r15d       ; add offset
    jc      .no_memory      ; overflow
    clc
    ret
.corrupt_mem:
    er_err  ERROR_CORRUPT
    stc
    ret
.underflow_mem:
    er_err  ERROR_STACK_UNDERFLOW
    stc
    ret
.no_memory:
    er_err  ERROR_NO_MEMORY
    stc
    ret

; Check memory range: address in eax, size in ecx
; Returns pointer in rax, carry on error
er_fn exec_memory_check_range
    er_frame_push
    ; Check address + size <= memory_len
    mov     rdx, [runtime_memory_len]
    mov     r8d, eax
    add     r8d, ecx
    jc      .out_of_range
    cmp     r8d, edx
    ja      .out_of_range
    ; Also check address + size <= memory_limit
    mov     rdx, [executor_memory_limit]
    cmp     r8d, edx
    ja      .out_of_range
    ; Valid: return pointer in rax
    add     rax, [runtime_memory_ptr]
    clc
    pop     rbp
    ret
.out_of_range:
    er_err  ERROR_NO_MEMORY
    stc
    pop     rbp
    ret
