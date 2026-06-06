; ==================================================================
; Type section parser
; Parses types from [r12] into types_buf
; =================================================================+
er_wasm_parse_type_section:
    er_frame_push_regs rbx, r12, r13, r14

    mov     rsi, r12
    er_call er_wasm_read_leb_u32, .error
    mov     r14, rax            ; r14 = count
    cmp     r14, MAX_TYPES
    ja      .unsupported
    mov     [type_count], r14

    ; Parse each FuncType
    xor     r13d, r13d          ; index
.type_loop:
    cmp     r13, r14
    jae     .done

    ; Read form byte (must be 0x60)
    movzx   eax, byte [rsi]
    inc     rsi
    cmp     al, 0x60
    jne     .unsupported

    ; Read param count
    er_call er_wasm_read_leb_u32, .error
    cmp     rax, MAX_TYPE_PARAMS
    ja      .unsupported
    
    ; Compute offset into types_buf
    mov     rbx, r13
    imul    rbx, FUNC_TYPE_SIZE
    lea     rbx, [types_buf + rbx]

    ; Store param count
    mov     [rbx + FUNC_TYPE_PARAM_COUNT_OFF], rax
    mov     rcx, rax            ; rcx = param count

    ; Read params
    xor     r8d, r8d
.param_loop:
    cmp     r8, rcx
    jae     .params_done
    er_call er_wasm_read_value_type, .error
    mov     [rbx + r8], al      ; store param type byte
    inc     r8
    jmp     .param_loop
.params_done:

    ; Read result count
    er_call er_wasm_read_leb_u32, .error
    cmp     rax, MAX_TYPE_RESULTS
    ja      .unsupported
    mov     [rbx + FUNC_TYPE_RESULT_COUNT_OFF], rax
    mov     rcx, rax            ; rcx = result count

    ; Read results
    xor     r8d, r8d
.result_loop:
    cmp     r8, rcx
    jae     .results_done
    er_call er_wasm_read_value_type, .error
    mov     [rbx + MAX_TYPE_PARAMS + r8], al  ; store result type after params
    inc     r8
    jmp     .result_loop
.results_done:

    inc     r13
    jmp     .type_loop

.done:
    mov     r12, rsi            ; save updated reader position
    er_ok
    jmp     .out
.unsupported:
    er_err  ERROR_UNSUPPORTED
    jmp     .out
.error:
    er_err  ERROR_CORRUPT
.out:
    er_pop_ret rbp, rbx, r12, r13, r14

; ==================================================================
; Import section parser
; =================================================================+
er_wasm_parse_import_section:
    er_frame_push_regs rbx, r12, r13, r14, r15

    mov     rsi, r12
    er_call er_wasm_read_leb_u32, .error
    mov     r14, rax            ; r14 = count
    cmp     r14, MAX_IMPORTS
    ja      .unsupported

    xor     r13d, r13d          ; import index
.import_loop:
    cmp     r13, r14
    jae     .done

    ; Read module name
    er_call er_wasm_read_leb_u32, .error
    er_store_struct_qword imports_buf, r13, IMPORTED_FUNC_SIZE, 0, rsi     ; module_name_ptr
    er_store_struct_qword imports_buf, r13, IMPORTED_FUNC_SIZE, 8, rax     ; module_name_len
    add     rsi, rax                        ; skip module name bytes

    ; Read import name
    er_call er_wasm_read_leb_u32, .error
    er_store_struct_qword imports_buf, r13, IMPORTED_FUNC_SIZE, 16, rsi    ; func_name_ptr
    er_store_struct_qword imports_buf, r13, IMPORTED_FUNC_SIZE, 24, rax    ; func_name_len
    ; Initialize resolved fields to -1 (unresolved)
    er_store_struct_qword imports_buf, r13, IMPORTED_FUNC_SIZE, 40, -1     ; resolved_module_id
    er_store_struct_qword imports_buf, r13, IMPORTED_FUNC_SIZE, 48, -1     ; resolved_func_index
    add     rsi, rax                        ; skip import name bytes

    ; Read external kind
    movzx   r15d, byte [rsi]
    inc     rsi

    cmp     r15d, EXTERNAL_FUNCTION
    je      .import_function
    cmp     r15d, EXTERNAL_MEMORY
    je      .import_memory
    cmp     r15d, EXTERNAL_GLOBAL
    je      .import_global
    cmp     r15d, EXTERNAL_TABLE
    je      .import_table
    jmp     .unsupported

.import_function:
    ; type_index
    er_call er_wasm_read_leb_u32, .error
    ; Verify type_index < type_count
    mov     rbx, rax
    cmp     rbx, [type_count]
    jae     .corrupt
    er_store_struct_qword imports_buf, r13, IMPORTED_FUNC_SIZE, 32, rbx    ; type_index
    jmp     .import_next

.import_memory:
    ; Store ImportedMemory
    mov     byte [imported_memory_present], 1
    lea     rcx, [rsp - 16]      ; temp Limits struct
    push    rsi
    call    er_wasm_read_limits
    pop     rsi
    er_check_nonzero edx, .error
    mov     rax, [rsp - 16]      ; min
    mov     rbx, [rsp - 8]       ; max
    mov     [imported_memory_min], rax
    mov     [imported_memory_max], rbx
    ; Update module memory limits
    mov     [memory_min_pages], rax
    mov     [memory_max_pages], rbx
    jmp     .import_next

.import_global:
    er_call er_wasm_read_value_type, .error
    movzx   r15d, byte [rsi - 1]  ; reread value type byte
    movzx   ebx, byte [rsi]
    inc     rsi
    cmp     bl, 1
    ja      .corrupt
    mov     rdi, [global_count]
    cmp     rdi, MAX_GLOBALS
    jae     .unsupported
    er_store_struct_byte globals_buf, rdi, GLOBAL_SIZE, 0, r15b            ; value_type
    ; Store ImportedGlobal
    er_store_struct_qword imported_globals_buf, r13, IMPORTED_GLOBAL_SIZE, 16, rdi ; global_index
    inc     qword [global_count]
    inc     qword [imported_global_count]
    jmp     .import_next

.import_table:
    mov     byte [imported_table_present], 1
    movzx   eax, byte [rsi]
    inc     rsi
    cmp     al, WASM_FUNCREF_TYPE
    jne     .unsupported
    lea     rcx, [imported_table_min]
    er_call er_wasm_read_limits, .error
    mov     rax, [imported_table_min]  ; min
    mov     rbx, [imported_table_max]  ; max
    cmp     rax, MAX_TABLE_ENTRIES
    ja      .unsupported
    er_check_zero rbx, .table_ok
    cmp     rbx, MAX_TABLE_ENTRIES
    ja      .unsupported
.table_ok:
    mov     [imported_table_min], rax
    mov     [imported_table_max], rbx
    mov     byte [table_has], 1
    mov     [table_min], rax
    mov     [table_max], rbx
    jmp     .import_next

.import_next:
    inc     qword [import_count]
    inc     r13
    jmp     .import_loop

.done:
    mov     r12, rsi
    er_ok
    jmp     .out
.unsupported:
    er_err  ERROR_UNSUPPORTED
    jmp     .out
.corrupt:
    er_err  ERROR_CORRUPT
    jmp     .out
.error:
    mov     edx, edx
.out:
    er_pop_ret rbp, rbx, r12, r13, r14, r15

; ==================================================================
; Function section parser
; =================================================================+
er_wasm_parse_function_section:
    er_frame_push_regs rbx, r12

    mov     rsi, r12
    er_call er_wasm_read_leb_u32, .error
    mov     rbx, rax            ; rbx = count
    cmp     rbx, MAX_FUNCTIONS
    ja      .unsupported
    ; Check import_count + count <= MAX_FUNCTIONS
    mov     rax, [import_count]
    add     rax, rbx
    cmp     rax, MAX_FUNCTIONS
    ja      .unsupported
    mov     [function_count], rbx

    xor     r11d, r11d          ; index
.func_loop:
    cmp     r11, rbx
    jae     .done
    er_call er_wasm_read_leb_u32, .error
    ; Verify type_index < type_count
    cmp     rax, [type_count]
    jae     .corrupt
    ; Store Function { type_index, code_index = index }
    er_store_struct_qword functions_buf, r11, FUNCTION_SIZE, 0, rax       ; type_index
    er_store_struct_qword functions_buf, r11, FUNCTION_SIZE, 8, r11       ; code_index
    inc     r11
    jmp     .func_loop

.done:
    mov     r12, rsi
    er_ok
    jmp     .out
.corrupt:
    er_err  ERROR_CORRUPT
    jmp     .out
.unsupported:
    er_err  ERROR_UNSUPPORTED
    jmp     .out
.error:
    mov     edx, edx
.out:
    er_pop_ret rbp, rbx, r12

; ==================================================================
; Table section parser
; =================================================================+
er_wasm_parse_table_section:
    er_frame_push
    push    r12

    mov     rsi, r12
    er_call er_wasm_read_leb_u32, .error
    cmp     rax, 1
    ja      .unsupported
    er_check_zero rax, .done

    cmp     byte [imported_table_present], 1
    je      .corrupt

    movzx   eax, byte [rsi]
    inc     rsi
    cmp     al, WASM_FUNCREF_TYPE
    jne     .unsupported

    lea     rcx, [table_min]
    er_call er_wasm_read_limits, .error
    mov     rax, [table_min]    ; min
    mov     rbx, [table_max]    ; max
    cmp     rax, MAX_TABLE_ENTRIES
    ja      .unsupported
    er_check_zero rbx, .table_ok
    cmp     rbx, MAX_TABLE_ENTRIES
    ja      .unsupported
.table_ok:
    mov     byte [table_has], 1
    mov     [table_min], rax
    mov     [table_max], rbx

.done:
    mov     r12, rsi
    er_ok
    jmp     .out
.corrupt:
    er_err  ERROR_CORRUPT
    jmp     .out
.unsupported:
    er_err  ERROR_UNSUPPORTED
    jmp     .out
.error:
    mov     edx, edx
.out:
    er_pop_ret rbp, r12

; ==================================================================
; Memory section parser
; =================================================================+
er_wasm_parse_memory_section:
    er_frame_push

    mov     rsi, r12
    er_call er_wasm_read_leb_u32, .error
    cmp     rax, 1
    ja      .unsupported
    er_check_zero rax, .done

    cmp     byte [imported_memory_present], 1
    je      .corrupt

    lea     rcx, [memory_min_pages]
    er_call er_wasm_read_limits, .error

.done:
    mov     r12, rsi
    er_ok
    jmp     .out
.corrupt:
    er_err  ERROR_CORRUPT
    jmp     .out
.unsupported:
    er_err  ERROR_UNSUPPORTED
    jmp     .out
.error:
    mov     edx, edx
.out:
    pop     rbp
    ret

; ==================================================================
; Export section parser
; =================================================================+
er_wasm_parse_export_section:
    er_frame_push_regs rbx, r12, r13, r14, r15

    mov     rsi, r12
    er_call er_wasm_read_leb_u32, .error
    mov     rbx, rax
    cmp     rbx, MAX_FUNCTIONS
    ja      .unsupported
    mov     [export_count], rbx

    xor     r14d, r14d
.export_loop:
    cmp     r14, rbx
    jae     .done

    ; Read name
    er_call er_wasm_read_leb_u32, .error
    mov     r13, rsi            ; save name start
    mov     r15, rax            ; save name length in r15
    add     rsi, rax            ; skip name

    ; Read kind
    movzx   r11d, byte [rsi]
    inc     rsi

    ; Read index
    er_call er_wasm_read_leb_u32, .error
    ; Store export
    er_store_struct_qword exports_buf, r14, EXPORT_SIZE, 0, r13      ; name ptr
    er_store_struct_qword exports_buf, r14, EXPORT_SIZE, 8, r15      ; name length
    er_store_struct_byte  exports_buf, r14, EXPORT_SIZE, 16, r11b    ; kind
    er_store_struct_qword exports_buf, r14, EXPORT_SIZE, 24, rax     ; index

    inc     r14
    jmp     .export_loop

.done:
    mov     r12, rsi
    er_ok
    jmp     .out
.unsupported:
    er_err  ERROR_UNSUPPORTED
    jmp     .out
.corrupt:
    er_err  ERROR_CORRUPT
    jmp     .out
.error:
    mov     edx, edx
.out:
    er_pop_ret rbp, rbx, r12, r13, r14, r15

; ==================================================================
; Start section parser
; =================================================================+
er_wasm_parse_start_section:
    er_frame_push

    mov     rsi, r12
    er_call er_wasm_read_leb_u32, .error
    mov     [start_function_index], rax
    mov     r12, rsi
    er_ok
    pop     rbp
    ret
.error:
    mov     edx, edx
    pop     rbp
    ret

; ==================================================================
; Data count section parser
; =================================================================+
er_wasm_parse_data_count_section:
    er_frame_push

    mov     rsi, r12
    er_call er_wasm_read_leb_u32, .error
    cmp     rax, MAX_DATA_SEGMENTS
    ja      .unsupported
    mov     [declared_data_count], rax
    mov     r12, rsi
    er_ok
    pop     rbp
    ret
.unsupported:
    er_err  ERROR_UNSUPPORTED
    pop     rbp
    ret
.error:
    mov     edx, edx
    pop     rbp
    ret

; ==================================================================
; Code section parser (most complex — also decodes ops)
; =================================================================+
er_wasm_parse_code_section:
    er_frame_push_regs rbx, r12, r13, r14, r15

    mov     rsi, r12
    er_call er_wasm_read_leb_u32, .error
    mov     rbx, rax            ; count
    cmp     rbx, [function_count]
    jne     .corrupt
    cmp     rbx, MAX_FUNCTIONS
    ja      .corrupt
    mov     [code_count], rbx

    xor     r14d, r14d          ; function index
.code_loop:
    cmp     r14, rbx
    jae     .done

    ; Read body size
    er_call er_wasm_read_leb_u32, .error
    mov     r15, rsi            ; r15 = body start
    add     r15, rax            ; r15 = body end (for bounds check)
    ; For now we use rsi as the reader into the body

    ; Read local group count
    er_call er_wasm_read_leb_u32, .error
    mov     r13, rax            ; local_group_count

    ; Store code entry: body_offset(8) + body_len(8) + local_count(8)
    er_store_struct_qword code_buf, r14, CODE_SIZE, 0, rsi          ; provisional body_offset before locals
    ; body_len = r15 - rsi (computed later after locals)
    er_store_struct_qword code_buf, r14, CODE_SIZE, 16, r13         ; local_group_count (reuse field, converted later)

    ; Parse local groups
    xor     r11d, r11d          ; total local count
.local_group_loop:
    er_check_zero r13, .locals_done
    dec     r13

    ; Read repeat count
    er_call er_wasm_read_leb_u32, .error
    mov     r12, rax            ; repeat count

    ; Read value type
    er_call er_wasm_read_value_type, .error
    mov     r9b, al             ; type byte

    ; Verify local_count + repeat <= MAX_LOCALS
    mov     rax, r11
    add     rax, r12
    cmp     rax, MAX_LOCALS
    ja      .unsupported

    ; Fill local_types
    mov     rdi, code_local_types
    imul    r10, r14, MAX_LOCALS
    add     rdi, r10
    xor     r10d, r10d
.repeat_loop:
    cmp     r10, r12
    jae     .repeat_done
    mov     [rdi + r11], r9b
    inc     r11
    inc     r10
    jmp     .repeat_loop
.repeat_done:
    jmp     .local_group_loop

.locals_done:
    ; Store the executable body start after local declarations.
    er_store_struct_qword code_buf, r14, CODE_SIZE, 0, rsi          ; body_offset (pointer into wasm bytes)
    ; Store local count
    er_store_struct_qword code_buf, r14, CODE_SIZE, 16, r11         ; local_count
    ; Store body end pointer - start = body length
    mov     rax, r15
    sub     rax, rsi
    er_store_struct_qword code_buf, r14, CODE_SIZE, 8, rax          ; body_len
    ; Store decoded_start = current decoded_op_count
    mov     rax, [decoded_op_count]
    er_store_struct_qword code_buf, r14, CODE_SIZE, 24, rax         ; decoded_start
    ; Decode the body
    er_push r14, r15
    mov     r12, rsi            ; body start for decoder
    ; We need the body bytes — already at rsi, length = r15 - rsi
    mov     rax, r15
    sub     rax, rsi
    mov     rdi, rsi            ; body ptr
    mov     rsi, rax            ; body len
    call    er_wasm_decode_body
    er_pop  r14, r15
    er_check_nonzero edx, .error
    ; Store decoded_count = decoded_op_count - decoded_start
    mov     rax, [decoded_op_count]
    er_load_struct_qword r9, code_buf, r14, CODE_SIZE, 24
    sub     rax, r9
    er_store_struct_qword code_buf, r14, CODE_SIZE, 32, rax       ; decoded_count
    mov     rsi, r15
    inc     r14
    jmp     .code_loop

.done:
    mov     r12, rsi
    er_ok
    jmp     .out
.corrupt:
    er_err  ERROR_CORRUPT
    jmp     .out
.unsupported:
    er_err  ERROR_UNSUPPORTED
    jmp     .out
.error:
    mov     edx, edx
.out:
    er_pop_ret rbp, rbx, r12, r13, r14, r15

; ==================================================================
; Global section parser
; =================================================================+
er_wasm_parse_global_section:
    er_frame_push_regs rbx, r12, r13, r14

    mov     rsi, r12
    er_call er_wasm_read_leb_u32, .error
    mov     rbx, rax
    mov     rax, [global_count]
    add     rax, rbx
    cmp     rax, MAX_GLOBALS
    ja      .unsupported

    xor     r13d, r13d
.global_loop:
    cmp     r13, rbx
    jae     .done

    ; Read value type
    er_call er_wasm_read_value_type, .error
    mov     r14, rax            ; save value_type

    ; Read mutability
    movzx   r11d, byte [rsi]
    inc     rsi
    cmp     r11b, 1
    ja      .corrupt

    ; Read constant expression
    mov     rdi, r14            ; value_type for read_constant
    er_call er_wasm_read_constant_value, .error

    ; Store global
    mov     rdi, [global_count]
    er_store_struct_byte  globals_buf, rdi, GLOBAL_SIZE, 0, r14b          ; value_type
    er_store_struct_byte  globals_buf, rdi, GLOBAL_SIZE, 1, r11b          ; mutable
    ; value stored in rax
    er_store_struct_qword globals_buf, rdi, GLOBAL_SIZE, 8, rax           ; value_data
    inc     qword [global_count]
    inc     r13
    jmp     .global_loop

.done:
    mov     r12, rsi
    er_ok
    jmp     .out
.corrupt:
    er_err  ERROR_CORRUPT
    jmp     .out
.unsupported:
    er_err  ERROR_UNSUPPORTED
    jmp     .out
.error:
    mov     edx, edx
.out:
    er_pop_ret rbp, rbx, r12, r13, r14

; ==================================================================
; Helper: er_wasm_read_constant_value
; Reads a constant value expression from [rsi] for the given value_type (in rdi).
; Returns value in rax.
; =================================================================+
er_wasm_read_constant_value:
    er_frame_push
    push    rbx

    movzx   eax, byte [rsi]
    inc     rsi
    mov     rbx, rax            ; opcode

    cmp     dil, VALUE_TAG_I32
    je      .read_i32
    cmp     dil, VALUE_TAG_I64
    je      .read_i64
    cmp     dil, VALUE_TAG_F32
    je      .read_f32
    cmp     dil, VALUE_TAG_F64
    je      .read_f64
    cmp     dil, VALUE_TAG_FUNCREF
    je      .read_funcref
    jmp     .unsupported

.read_i32:
    cmp     bl, 0x41            ; i32.const
    jne     .unsupported
    er_call er_wasm_read_leb_i32, .error
    push    rax
    jmp     .check_end
.read_i64:
    cmp     bl, 0x42            ; i64.const
    jne     .unsupported
    er_call er_wasm_read_leb_i64, .error
    push    rax
    jmp     .check_end
.read_f32:
    cmp     bl, 0x43            ; f32.const
    jne     .unsupported
    mov     eax, [rsi]
    add     rsi, 4
    push    rax
    jmp     .check_end
.read_f64:
    cmp     bl, 0x44            ; f64.const
    jne     .unsupported
    mov     rax, [rsi]
    add     rsi, 8
    push    rax
    jmp     .check_end
.read_funcref:
    cmp     bl, 0xd0            ; ref.null
    je      .ref_null
    cmp     bl, 0xd2            ; ref.func
    je      .ref_func
    jmp     .unsupported
.ref_null:
    movzx   eax, byte [rsi]
    inc     rsi
    cmp     al, WASM_FUNCREF_TYPE
    jne     .unsupported
    push    0                   ; null = 0
    jmp     .check_end
.ref_func:
    er_call er_wasm_read_leb_u32, .error
    push    rax
    jmp     .check_end

.check_end:
    movzx   eax, byte [rsi]
    inc     rsi
    cmp     al, 0x0b            ; end
    jne     .unsupported
    pop     rax
    er_ok
    jmp     .done

.unsupported:
    er_err  ERROR_UNSUPPORTED
    er_pop_ret rbp, rbx
.error:
    mov     edx, edx
.done:
    er_pop_ret rbp, rbx

; ==================================================================
; Element section parser
; =================================================================+
er_wasm_parse_element_section:
    er_frame_push_regs rbx, r11, r12, r13, r14, r15

    mov     rsi, r12
    er_call er_wasm_read_leb_u32, .error
    mov     r15, rax
    cmp     r15, MAX_DATA_SEGMENTS
    ja      .unsupported_section
    mov     [element_segment_count], r15

    xor     r11d, r11d          ; segment index
.seg_loop:
    cmp     r11, r15
    jae     .done

    ; Initialize segment
    er_store_struct_qword element_segments, r11, ELEMENT_SEGMENT_SIZE, 256, 0 ; count = 0
    er_store_struct_byte  element_segments, r11, ELEMENT_SEGMENT_SIZE, 264, 0 ; passive = 0
    er_store_struct_byte  element_segments, r11, ELEMENT_SEGMENT_SIZE, 265, 0 ; dropped = 0

    ; Read mode
    er_call er_wasm_read_leb_u32, .error

    cmp     eax, 0
    je      .mode_0
    cmp     eax, 1
    je      .mode_1
    cmp     eax, 2
    je      .mode_2
    cmp     eax, 3
    je      .mode_3
    ; Modes 4-7 carry expression vectors. They must not be accepted until
    ; ref.func/ref.null element expression application is implemented.
    jmp     .unsupported_section

.mode_0:
    ; Active implicit table 0: offset_expr, elem_kind, vec(func_idx)
    cmp     byte [table_has], 1
    jne     .unsupported_section
    er_call er_wasm_read_constant_i32, .error
    mov     rbx, rax
    jmp     .active_tail

.mode_1:
    ; Passive: elem_kind byte + vec(func_idx)
    xor     ebx, ebx
    jmp     .passive_tail

.mode_2:
    ; Active with explicit table: table_idx, offset_expr, elem_kind, vec
    cmp     byte [table_has], 1
    jne     .unsupported_section
    er_call er_wasm_read_leb_u32, .error  ; table index
    er_check_nonzero eax, .unsupported_section
    er_call er_wasm_read_constant_i32, .error
    mov     rbx, rax
    jmp     .active_tail

.active_tail:
    movzx   eax, byte [rsi]     ; elem_kind
    inc     rsi
    er_check_nonzero al, .unsupported_section
    er_call er_wasm_read_leb_u32, .error
    mov     r14, rax
    cmp     r14, MAX_TABLE_ENTRIES
    ja      .unsupported_section
    mov     rax, rbx
    add     rax, r14
    jc      .unsupported_section
    cmp     rax, [table_min]
    ja      .unsupported_section

    er_store_struct_qword element_segments, r11, ELEMENT_SEGMENT_SIZE, 256, r14 ; count
    er_store_struct_byte  element_segments, r11, ELEMENT_SEGMENT_SIZE, 264, 0   ; active

    xor     r13d, r13d
.active_idx_loop:
    cmp     r13, r14
    jae     .next
    er_call er_wasm_read_leb_u32, .error
    mov     rcx, [import_count]
    add     rcx, [function_count]
    cmp     rax, rcx
    jae     .unsupported_section
    er_store_struct_qword element_segments, r11, ELEMENT_SEGMENT_SIZE, r13 * 8, rax
    mov     r10, rbx
    add     r10, r13
    mov     [table_entries + r10 * 8], rax
    inc     r13
    jmp     .active_idx_loop

.passive_tail:
    movzx   eax, byte [rsi]     ; elem_kind
    inc     rsi
    er_check_nonzero al, .unsupported_section
    er_call er_wasm_read_leb_u32, .error
    mov     r14, rax            ; count
    cmp     r14, MAX_TABLE_ENTRIES
    ja      .unsupported_section

    ; Store in element segment
    er_store_struct_qword element_segments, r11, ELEMENT_SEGMENT_SIZE, 256, r14 ; count
    er_store_struct_byte  element_segments, r11, ELEMENT_SEGMENT_SIZE, 264, 1   ; passive = 1

    xor     r13d, r13d
.idx_loop:
    cmp     r13, r14
    jae     .next
    er_call er_wasm_read_leb_u32, .error
    mov     rcx, [import_count]
    add     rcx, [function_count]
    cmp     rax, rcx
    jae     .unsupported_section
    er_store_struct_qword element_segments, r11, ELEMENT_SEGMENT_SIZE, r13 * 8, rax
    inc     r13
    jmp     .idx_loop

.mode_3:
    ; Declarative: elem_kind byte + vec(func_idx) — validate and discard.
    movzx   eax, byte [rsi]
    inc     rsi
    er_check_nonzero al, .unsupported_section
    er_call er_wasm_read_leb_u32, .error
    mov     r14, rax
    cmp     r14, MAX_TABLE_ENTRIES
    ja      .unsupported_section
.skip_idx_loop:
    er_check_zero r14, .next
    er_call er_wasm_read_leb_u32, .error
    mov     rcx, [import_count]
    add     rcx, [function_count]
    cmp     rax, rcx
    jae     .unsupported_section
    dec     r14
    jmp     .skip_idx_loop

.next:
    inc     r11
    jmp     .seg_loop

.done:
    er_ok
    er_pop_ret rbp, rbx, r11, r12, r13, r14, r15

.error:
    ; rdx already has error from er_call
    er_pop_ret rbp, rbx, r11, r12, r13, r14, r15

.unsupported:
.unsupported_section:
    er_err  ERROR_UNSUPPORTED
    er_pop_ret rbp, rbx, r11, r12, r13, r14, r15

; ==================================================================
; Data section parser
; =================================================================+
er_wasm_parse_data_section:
    er_frame_push
    push    r12

    mov     rsi, r12
    er_call er_wasm_read_leb_u32, .error
    mov     r12, rax
    cmp     r12, MAX_DATA_SEGMENTS
    ja      .unsupported
    mov     [data_segment_count], r12

    xor     r11d, r11d
.data_loop:
    cmp     r11, r12
    jae     .done

    ; Read mode
    er_call er_wasm_read_leb_u32, .error
    cmp     eax, 0
    je      .active_0
    cmp     eax, 1
    je      .passive
    cmp     eax, 2
    je      .active_2
    jmp     .unsupported

.active_0:
    ; mode 0: active, memory 0 implicit
    er_call er_wasm_read_constant_i32, .error
    push    rax                 ; offset
    er_call er_wasm_read_leb_u32, .error
    mov     rcx, rax            ; byte count
    pop     rdx                 ; offset
    ; Store DataSegment
    er_store_struct_qword data_segments, r11, DATA_SEGMENT_SIZE, 0, rdx    ; offset
    er_store_struct_qword data_segments, r11, DATA_SEGMENT_SIZE, 8, rsi    ; bytes ptr
    er_store_struct_qword data_segments, r11, DATA_SEGMENT_SIZE, 16, rcx   ; bytes len
    er_store_struct_byte  data_segments, r11, DATA_SEGMENT_SIZE, 24, 1     ; active
    er_store_struct_byte  data_segments, r11, DATA_SEGMENT_SIZE, 25, 0     ; not dropped
    add     rsi, rcx
    jmp     .next

.passive:
    er_call er_wasm_read_leb_u32, .error
    er_store_struct_qword data_segments, r11, DATA_SEGMENT_SIZE, 8, rsi    ; bytes ptr
    er_store_struct_qword data_segments, r11, DATA_SEGMENT_SIZE, 16, rax   ; bytes len
    er_store_struct_byte  data_segments, r11, DATA_SEGMENT_SIZE, 24, 0     ; not active
    er_store_struct_byte  data_segments, r11, DATA_SEGMENT_SIZE, 25, 0
    add     rsi, rax
    jmp     .next

.active_2:
    ; mode 2: active with explicit memory index
    call    er_wasm_read_leb_u32  ; read and ignore memory index (must be 0)
    er_check_nonzero edx, .error
    er_check_nonzero eax, .unsupported
    er_call er_wasm_read_constant_i32, .error
    push    rax                 ; offset
    er_call er_wasm_read_leb_u32, .error
    mov     rcx, rax
    pop     rdx
    er_store_struct_qword data_segments, r11, DATA_SEGMENT_SIZE, 0, rdx
    er_store_struct_qword data_segments, r11, DATA_SEGMENT_SIZE, 8, rsi
    er_store_struct_qword data_segments, r11, DATA_SEGMENT_SIZE, 16, rcx
    er_store_struct_byte  data_segments, r11, DATA_SEGMENT_SIZE, 24, 1
    er_store_struct_byte  data_segments, r11, DATA_SEGMENT_SIZE, 25, 0
    add     rsi, rcx
    jmp     .next

.next:
    inc     r11
    jmp     .data_loop

.done:
    mov     r12, rsi
    er_ok
    jmp     .out
.unsupported:
    er_err  ERROR_UNSUPPORTED
    jmp     .out
.error:
    mov     edx, edx
.out:
    er_pop_ret rbp, r12

; ==================================================================
; Decoded op cache
; er_wasm_decode_body(bytes_ptr=rdi, bytes_len=rsi)
; Decodes opcodes from a function body, populates decoded_ops[]
; =================================================================+
