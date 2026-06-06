; ==================================================================
; Missing BSS variables referenced by parsers
; =================================================================+
SECTION .bss
debug_section_id:  resb 1
debug_check:       resq 4
memory_min_pages:  resq 1
memory_max_pages:  resq 1
has_memory:        resb 1
start_function_index: resq 1

SECTION .text

; ==================================================================
; Apply data segments — copy data segment bytes into runtime memory
; er_wasm_apply_data_segments(memory_ptr=rdi, memory_pages=rsi)
; =================================================================+
er_wasm_apply_data_segments:
    er_frame_push_regs rbx, r12, r13

    mov     r12, rdi            ; memory_ptr
    ; Convert pages to bytes
    mov     rdi, rsi
    er_call er_wasm_pages_to_bytes, .error
    mov     r13, rax            ; memory limit in bytes

    xor     r11d, r11d          ; segment index
.seg_loop:
    cmp     r11, [data_segment_count]
    jae     .done

    ; Check if active
    er_load_struct_byte al, data_segments, r11, DATA_SEGMENT_SIZE, 24
    cmp     al, 0               ; active flag
    je      .next

    ; offset + bytes_len <= limit?
    er_load_struct_qword rdx, data_segments, r11, DATA_SEGMENT_SIZE, 0     ; offset
    er_load_struct_qword rcx, data_segments, r11, DATA_SEGMENT_SIZE, 16    ; byte count
    mov     rax, rdx
    add     rax, rcx
    jc      .nomemory
    cmp     rax, r13
    ja      .nomemory
    cmp     rax, [runtime_memory_len]
    ja      .nomemory

    ; Copy bytes: memcpy(memory + offset, segment_bytes, count)
    mov     rdi, r12
    add     rdi, rdx
    er_load_struct_qword rsi, data_segments, r11, DATA_SEGMENT_SIZE, 8     ; bytes ptr
    mov     rdx, rcx
    call    er_memcpy

.next:
    inc     r11
    jmp     .seg_loop

.done:
    er_ok
    jmp     .out
.nomemory:
    er_err  ERROR_NO_MEMORY
    jmp     .out
.error:
    ; rdx already has error from called function — pass through
.out:
    er_pop_ret rbp, rbx, r12, r13

; ==================================================================
; Find export by name
; er_wasm_find_export(name_ptr=rdi, name_len=rsi)
; Returns export index in rax, or error in rdx
; =================================================================+
er_wasm_find_export:
    er_frame_push_regs rbx, r12, r13

    mov     r12, rdi            ; name ptr
    mov     r13, rsi            ; name len

    xor     r11d, r11d
.search_loop:
    cmp     r11, [export_count]
    jae     .not_found

    push    r11
    mov     rbx, r11
    imul    rbx, EXPORT_SIZE
    mov     rdi, [exports_buf + rbx]      ; export name ptr
    mov     rsi, [exports_buf + rbx + 8]  ; export name len
    mov     rdx, r12
    mov     rcx, r13
    call    er_wasm_eql
    pop     r11
    er_check_nonzero rax, .found

    inc     r11
    jmp     .search_loop

.found:
    mov     rax, r11
    er_ok
    jmp     .done
.not_found:
    er_err  ERROR_MISSING_EXPORT
    xor     eax, eax
.done:
    er_pop_ret rbp, rbx, r12, r13

; ==================================================================
; Type index for function
; er_wasm_type_index_for_function(function_index=rdi)
; Returns type_index in rax, error in rdx
; =================================================================+
er_wasm_type_index_for_function:
    er_frame_push

    ; Check if imported function
    mov     rsi, [import_count]
    cmp     rdi, rsi
    jae     .defined

    ; Imported: return imports[function_index].type_index
    er_load_struct_qword rax, imports_buf, rdi, IMPORTED_FUNC_SIZE, 32     ; type_index
    er_ok
    pop     rbp
    ret

.defined:
    ; Defined function
    sub     rdi, rsi            ; subtract import count
    cmp     rdi, [function_count]
    jae     .corrupt
    er_load_struct_qword rax, functions_buf, rdi, FUNCTION_SIZE, 0         ; type_index
    er_ok
    pop     rbp
    ret

.corrupt:
    er_err  ERROR_CORRUPT
    xor     eax, eax
    pop     rbp
    ret

; ==================================================================
; Code index for function  
; er_wasm_code_index_for_function(function_index=rdi)
; Returns code_index in rax, error in rdx
; =================================================================+
er_wasm_code_index_for_function:
    er_frame_push

    mov     rsi, [import_count]
    cmp     rdi, rsi
    jb      .missing_import

    sub     rdi, rsi
    cmp     rdi, [function_count]
    jae     .corrupt

    er_load_struct_qword rax, functions_buf, rdi, FUNCTION_SIZE, 8         ; code_index
    er_ok
    pop     rbp
    ret

.missing_import:
    er_err  ERROR_MISSING_IMPORT
    xor     eax, eax
    pop     rbp
    ret
.corrupt:
    er_err  ERROR_CORRUPT
    xor     eax, eax
    pop     rbp
    ret

; ==================================================================
; Frame (per-function-call state) operations
; The Frame is stored in a dedicated BSS area since only one call
; is active at a time (we don't need the full call depth — we use
; a call stack area for that)
; =================================================================+

; -- Frame state (current invocation)

; ==================================================================
; Pop value from frame stack
; Returns value in rax
; =================================================================+
