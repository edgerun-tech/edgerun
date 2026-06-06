; ==================================================================
; Legacy placeholder notes were here before module parser entry.
; Import resolution is handled by er_wasm_resolve_imports in wasm_run.asm.
; ==================================================================

; ==================================================================
; Helper: checked_add
; Returns rdi + rsi in rax, CF set if overflow
; ==================================================================
er_wasm_checked_add:
    mov     rax, rdi
    add     rax, rsi
    ret

; ==================================================================
; Helper: pages_to_bytes
; rdi = pages, returns bytes in rax, rdx = 0 on success, error on overflow
; ==================================================================
er_wasm_pages_to_bytes:
    mov     rax, rdi
    shr     rax, 48             ; if pages > 2^48, overflow (since 2^48 >> 16 = 2^32)
    er_check_nonzero rax, .overflow
    mov     rax, rdi
    shl     rax, WASM_PAGE_SHIFT
    er_ok
    ret
.overflow:
    er_err  ERROR_UNSUPPORTED
    ret

; ==================================================================
; Module parser entry point
; er_wasm_parse_module(bytes_ptr=rdi, bytes_len=rsi)
; Returns: rdx = 0 on success, otherwise error code (per macros.inc convention)
; Destroys current module state, re-parses into global structures
; =================================================================+
er_wasm_parse_module:
    er_frame_push_regs rbx, r12, r13, r14, r15
    sub     rsp, 64             ; local vars

    ; Reset module state
    mov     qword [type_count], 0
    mov     qword [import_count], 0
    mov     qword [function_count], 0
    mov     qword [code_count], 0
    mov     qword [export_count], 0
    mov     qword [global_count], 0
    mov     qword [imported_global_count], 0
    mov     qword [element_segment_count], 0
    mov     qword [data_segment_count], 0
    mov     qword [declared_data_count], 0
    mov     qword [table_min], 0
    mov     qword [table_max], 0
    mov     byte [table_has], 0
    mov     qword [memory_min_pages], 0
    mov     qword [memory_max_pages], 0
    mov     qword [imported_memory_min], 0
    mov     qword [imported_memory_max], 0
    mov     qword [imported_table_min], 0
    mov     qword [imported_table_max], 0
    mov     byte [imported_memory_present], 0
    mov     byte [imported_table_present], 0
    mov     qword [decoded_op_count], 0
    mov     qword [start_function_index], -1
    lea     rax, [table_entries]
    mov     rcx, MAX_TABLE_ENTRIES
.clear_table_entries:
    mov     qword [rax], 0
    add     rax, 8
    dec     rcx
    jnz     .clear_table_entries

    ; Check magic bytes
    cmp     rsi, 8
    jb      .corrupt
    ; Compare wasm magic
    mov     r8d, [rdi]
    cmp     r8d, 0x6d736100      ; little-endian "\x00asm"
    jne     .corrupt
    ; Compare wasm version (0x00000001 at offset 4)
    mov     r8d, [rdi + 4]
    cmp     r8d, 0x00000001
    jne     .corrupt

    ; Set up reader at offset 8
    lea     r12, [rdi + 8]      ; r12 = current read position
    lea     r13, [rdi + rsi]    ; r13 = end pointer
    xor     r14b, r14b          ; previous_section_order

.parse_loop:
    cmp     r12, r13
    jae     .parse_done

    ; Read section ID
    movzx   r15d, byte [r12]
    inc     r12

    ; Read section size (LEB128)
    mov     rsi, r12
    er_call er_wasm_read_leb_u32, .error
    mov     r11, rax            ; r11 = section_size

    ; Skip custom sections (id == 0)
    er_check_zero r15d, .skip_section

    ; Validate section order (section must have strictly increasing order)
    movzx   eax, byte [section_order + r15]
    cmp     al, r14b
    jbe     .corrupt
    mov     r14b, al

    ; Section payload bounds check
    mov     rax, r12
    add     rax, r11
    cmp     rax, r13
    ja      .corrupt

    ; Save rsi (payload start) and r11 (payload len), dispatch
    mov     [rsp], rsi          ; save payload start (past section-size LEB)
    mov     [rsp + 8], r11      ; save payload length
    mov     [rsp + 16], r13     ; save end pointer

    ; r12 = payload pointer for section parsing
    mov     r12, rsi
    mov     [debug_section_id], r15b  ; debug: mark section ID
    cmp     r15d, SECTION_TYPE
    je      .parse_type
    cmp     r15d, SECTION_IMPORT
    je      .parse_import
    cmp     r15d, SECTION_FUNCTION
    je     .parse_function
    cmp     r15d, SECTION_TABLE
    je      .parse_table
    cmp     r15d, SECTION_MEMORY
    je      .parse_memory
    cmp     r15d, SECTION_GLOBAL
    je      .parse_global
    cmp     r15d, SECTION_EXPORT
    je      .parse_export
    cmp     r15d, SECTION_START
    je      .parse_start
    cmp     r15d, SECTION_ELEMENT
    je      .parse_element
    cmp     r15d, SECTION_DATA_COUNT
    je      .parse_data_count
    cmp     r15d, SECTION_CODE
    je      .parse_code
    cmp     r15d, SECTION_DATA
    je      .parse_data
    jmp     .unsupported

.skip_section:
    mov     r12, rsi
    add     r12, r11
    jmp     .parse_loop

.parse_type:
    er_call er_wasm_parse_type_section, .error
    ; advance past section
    mov     r12, [rsp]
    add     r12, [rsp + 8]
    mov     r13, [rsp + 16]
    jmp     .parse_loop

.parse_import:
    er_call er_wasm_parse_import_section, .error
    mov     r12, [rsp]
    add     r12, [rsp + 8]
    mov     r13, [rsp + 16]
    jmp     .parse_loop

.parse_function:
    er_call er_wasm_parse_function_section, .error
    mov     r12, [rsp]
    add     r12, [rsp + 8]
    mov     r13, [rsp + 16]
    jmp     .parse_loop

.parse_table:
    er_call er_wasm_parse_table_section, .error
    mov     r12, [rsp]
    add     r12, [rsp + 8]
    mov     r13, [rsp + 16]
    jmp     .parse_loop

.parse_memory:
    er_call er_wasm_parse_memory_section, .error
    mov     r12, [rsp]
    add     r12, [rsp + 8]
    mov     r13, [rsp + 16]
    jmp     .parse_loop

.parse_global:
    er_call er_wasm_parse_global_section, .error
    mov     r12, [rsp]
    add     r12, [rsp + 8]
    mov     r13, [rsp + 16]
    jmp     .parse_loop

.parse_export:
    er_call er_wasm_parse_export_section, .error
    mov     r12, [rsp]
    add     r12, [rsp + 8]
    mov     r13, [rsp + 16]
    jmp     .parse_loop

.parse_start:
    er_call er_wasm_parse_start_section, .error
    mov     r12, [rsp]
    add     r12, [rsp + 8]
    mov     r13, [rsp + 16]
    jmp     .parse_loop

.parse_element:
    er_call er_wasm_parse_element_section, .error
    mov     r12, [rsp]
    add     r12, [rsp + 8]
    mov     r13, [rsp + 16]
    jmp     .parse_loop

.parse_data_count:
    er_call er_wasm_parse_data_count_section, .error
    mov     r12, [rsp]
    add     r12, [rsp + 8]
    mov     r13, [rsp + 16]
    jmp     .parse_loop

.parse_code:
    er_call er_wasm_parse_code_section, .error
    mov     r12, [rsp]
    add     r12, [rsp + 8]
    mov     r13, [rsp + 16]
    jmp     .parse_loop

.parse_data:
    er_call er_wasm_parse_data_section, .error
    mov     r12, [rsp]
    add     r12, [rsp + 8]
    mov     r13, [rsp + 16]
    jmp     .parse_loop

.parse_done:
    ; Validate function_count == code_count
    mov     rax, [function_count]
    cmp     rax, [code_count]
    jne     .corrupt
    ; Validate declared_data_count matches data_segment_count if present
    mov     rax, [declared_data_count]
    er_check_zero rax, .success
    cmp     rax, [data_segment_count]
    jne     .corrupt
.success:
    er_ok
    jmp     .done
.corrupt:
    er_err  ERROR_CORRUPT
    jmp     .done
.unsupported:
    er_err  ERROR_UNSUPPORTED
    jmp     .done
.error:
    ; rdx already has error from called function — pass through
.done:
    lea     rsp, [rbp - 40]
    er_pop_ret rbp, rbx, r12, r13, r14, r15
