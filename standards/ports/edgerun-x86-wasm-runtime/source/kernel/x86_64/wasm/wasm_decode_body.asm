%macro wasm_decode_finish_op 0
    mov     eax, esi
    sub     eax, edi
    mov     [decoded_ops + rbx + 4], eax
    inc     dword [decoded_op_count]
%endm

er_wasm_decode_body:
    er_frame_push_regs rbx, r12, r13, r14, r15

    mov     r12, rdi            ; body start
    mov     r13, rdi
    add     r13, rsi            ; body end
    mov     rsi, rdi            ; rsi = bytecode reader (start at body ptr)
    mov     r14, [decoded_op_count]  ; start index

.decode_loop:
    cmp     rsi, r13
    jae     .done

    mov     r15d, [decoded_op_count]
    cmp     r15d, MAX_DECODED_OPS
    jae     .unsupported

    ; Calculate offset from body start
    mov     rax, rsi
    sub     rax, rdi
    mov     edx, eax            ; offset (low 32 bits)

    ; Read opcode byte
    movzx   r11d, byte [rsi]
    inc     rsi

    ; Store decoded op header
    mov     ebx, r15d
    imul    ebx, DECODED_OP_SIZE
    mov     [decoded_ops + rbx], edx      ; offset
    mov     [decoded_ops + rbx + 8], r11b ; opcode_byte

    ; Handle extended prefix
    cmp     r11b, WASM_EXTENDED_PREFIX
    je      .decode_extended

    ; Decode immediate fields based on opcode
    cmp     r11b, 0x02          ; block
    je      .decode_block_type
    cmp     r11b, 0x03          ; loop
    je      .decode_block_type
    cmp     r11b, 0x04          ; if
    je      .decode_block_type
    cmp     r11b, 0x0c          ; br
    je      .decode_leb
    cmp     r11b, 0x0d          ; br_if
    je      .decode_leb
    cmp     r11b, 0x10          ; call
    je      .decode_leb
    cmp     r11b, 0x20          ; local.get
    je      .decode_leb
    cmp     r11b, 0x21          ; local.set
    je      .decode_leb
    cmp     r11b, 0x22          ; local.tee
    je      .decode_leb
    cmp     r11b, 0x23          ; global.get
    je      .decode_leb
    cmp     r11b, 0x24          ; global.set
    je      .decode_leb
    cmp     r11b, 0x25          ; table.get
    je      .decode_table
    cmp     r11b, 0x26          ; table.set
    je      .decode_table
    cmp     r11b, 0x28          ; i32.load
    je      .decode_mem
    cmp     r11b, 0x29          ; i64.load
    je      .decode_mem
    cmp     r11b, 0x2a          ; f32.load
    je      .decode_mem
    cmp     r11b, 0x2b          ; f64.load
    je      .decode_mem
    cmp     r11b, 0x2c          ; i32.load8_s
    je      .decode_mem
    cmp     r11b, 0x2d          ; i32.load8_u
    je      .decode_mem
    cmp     r11b, 0x2e          ; i32.load16_s
    je      .decode_mem
    cmp     r11b, 0x2f          ; i32.load16_u
    je      .decode_mem
    cmp     r11b, 0x30          ; i64.load8_s
    je      .decode_mem
    cmp     r11b, 0x31          ; i64.load8_u
    je      .decode_mem
    cmp     r11b, 0x32          ; i64.load16_s
    je      .decode_mem
    cmp     r11b, 0x33          ; i64.load16_u
    je      .decode_mem
    cmp     r11b, 0x34          ; i64.load32_s
    je      .decode_mem
    cmp     r11b, 0x35          ; i64.load32_u
    je      .decode_mem
    cmp     r11b, 0x36          ; i32.store
    je      .decode_mem
    cmp     r11b, 0x37          ; i64.store
    je      .decode_mem
    cmp     r11b, 0x38          ; f32.store
    je      .decode_mem
    cmp     r11b, 0x39          ; f64.store
    je      .decode_mem
    cmp     r11b, 0x3a          ; i32.store8
    je      .decode_mem
    cmp     r11b, 0x3b          ; i32.store16
    je      .decode_mem
    cmp     r11b, 0x3c          ; i64.store8
    je      .decode_mem
    cmp     r11b, 0x3d          ; i64.store16
    je      .decode_mem
    cmp     r11b, 0x3e          ; i64.store32
    je      .decode_mem
    cmp     r11b, 0x3f          ; memory.size
    je      .decode_memidx
    cmp     r11b, 0x40          ; memory.grow
    je      .decode_memidx
    cmp     r11b, 0x41          ; i32.const
    je      .decode_i32_const
    cmp     r11b, 0x42          ; i64.const
    je      .decode_i64_const
    cmp     r11b, 0x43          ; f32.const
    je      .decode_f32_const
    cmp     r11b, 0x44          ; f64.const
    je      .decode_f64_const
    cmp     r11b, 0x11          ; call_indirect
    je      .decode_call_indirect
    cmp     r11b, 0x1c          ; select_typed
    je      .decode_select_typed
    cmp     r11b, 0xd0          ; ref.null
    je      .decode_ref_null
    cmp     r11b, 0xd2          ; ref.func
    je      .decode_leb
    cmp     r11b, 0x0e          ; br_table
    je      .decode_br_table

    ; Store next_offset for opcodes with no immediates
    mov     eax, esi
    sub     eax, edi            ; relative to body start
    mov     [decoded_ops + rbx + 4], eax  ; next_offset

    inc     dword [decoded_op_count]
    jmp     .decode_loop

    ; --- decode immediate handlers ---
.decode_leb:
    ; Read single LEB128 u32 into imm0
    er_call er_wasm_read_leb_u32, .error
    mov     [decoded_ops + rbx + 12], eax  ; imm0
    mov     eax, esi
    sub     eax, edi
    mov     [decoded_ops + rbx + 4], eax   ; next_offset
    inc     dword [decoded_op_count]
    jmp     .decode_loop

.decode_mem:
    ; Read alignment + offset (two LEB128 u32s)
    er_call er_wasm_read_leb_u32, .error
    mov     [decoded_ops + rbx + 12], eax  ; imm0 = alignment
    er_call er_wasm_read_leb_u32, .error
    mov     [decoded_ops + rbx + 16], eax  ; imm1 = offset
    mov     eax, esi
    sub     eax, edi
    mov     [decoded_ops + rbx + 4], eax   ; next_offset
    inc     dword [decoded_op_count]
    jmp     .decode_loop

.decode_memidx:
    ; Read memory index (must be 0)
    er_call er_wasm_read_leb_u32, .error
    er_check_nonzero eax, .unsupported
    wasm_decode_finish_op
    jmp     .decode_loop

.decode_i32_const:
    er_call er_wasm_read_leb_i32, .error
    mov     [decoded_ops + rbx + 12], eax  ; imm0 = value (as bit pattern)
    wasm_decode_finish_op
    jmp     .decode_loop

.decode_i64_const:
    er_call er_wasm_read_leb_i64, .error
    mov     [decoded_ops + rbx + 12], eax  ; imm0 = low 32 bits
    shr     rax, 32
    mov     [decoded_ops + rbx + 16], eax  ; imm1 = high 32 bits
    wasm_decode_finish_op
    jmp     .decode_loop

.decode_f32_const:
    mov     eax, [rsi]
    add     rsi, 4
    mov     [decoded_ops + rbx + 12], eax  ; imm0 = f32 bits
    wasm_decode_finish_op
    jmp     .decode_loop

.decode_f64_const:
    mov     eax, [rsi]
    mov     [decoded_ops + rbx + 12], eax  ; imm0 = low 32 bits
    mov     eax, [rsi + 4]
    mov     [decoded_ops + rbx + 16], eax  ; imm1 = high 32 bits
    add     rsi, 8
    wasm_decode_finish_op
    jmp     .decode_loop

.decode_block_type:
    ; Read block type (1 byte or LEB128 type index)
    movzx   eax, byte [rsi]
    inc     rsi
    cmp     al, WASM_EMPTY_BLOCK_TYPE
    je      .block_done
    ; Check if it's a value type
    cmp     al, VALUE_TAG_I32
    je      .block_done
    cmp     al, VALUE_TAG_I64
    je      .block_done
    cmp     al, VALUE_TAG_F32
    je      .block_done
    cmp     al, VALUE_TAG_F64
    je      .block_done
    cmp     al, VALUE_TAG_FUNCREF
    je      .block_done
    ; It's a type index (LEB128 starting with this byte)
    ; Continue reading as LEB128
    dec     rsi                 ; put back the byte
    er_call er_wasm_read_leb_u32, .error
.block_done:
    wasm_decode_finish_op
    jmp     .decode_loop

.decode_call_indirect:
    er_call er_wasm_read_leb_u32, .error
    mov     [decoded_ops + rbx + 12], eax  ; imm0 = type_index
    er_call er_wasm_read_leb_u32, .error
    er_check_nonzero eax, .unsupported
    mov     [decoded_ops + rbx + 16], eax  ; imm1 = table_index
    wasm_decode_finish_op
    jmp     .decode_loop

.decode_select_typed:
    call    er_wasm_read_leb_u32  ; type count
    er_check_nonzero edx, .error
    cmp     eax, 1
    jne     .unsupported
    er_call er_wasm_read_value_type, .error
    wasm_decode_finish_op
    jmp     .decode_loop

.decode_table:
    call    er_wasm_read_leb_u32  ; table index
    er_check_nonzero edx, .error
    er_check_nonzero eax, .unsupported
    mov     [decoded_ops + rbx + 12], eax
    wasm_decode_finish_op
    jmp     .decode_loop

.decode_ref_null:
    movzx   eax, byte [rsi]
    inc     rsi
    cmp     al, WASM_FUNCREF_TYPE
    jne     .unsupported
    wasm_decode_finish_op
    jmp     .decode_loop

.decode_br_table:
    er_call er_wasm_read_leb_u32, .error
    mov     r11d, eax           ; target count
    xor     r10d, r10d
.br_table_loop:
    cmp     r10, r11
    jae     .br_table_done
    er_call er_wasm_read_leb_u32, .error
    inc     r10
    jmp     .br_table_loop
.br_table_done:
    call    er_wasm_read_leb_u32  ; default target
    er_check_nonzero edx, .error
    wasm_decode_finish_op
    jmp     .decode_loop

.decode_extended:
    ; Read extended opcode
    er_call er_wasm_read_leb_u32, .error
    mov     [decoded_ops + rbx + 12], eax  ; imm0 = extended opcode
    ; Skip immediate fields based on extended opcode
    cmp     eax, EXT_MEMORY_INIT
    je      .ext_mem_init
    cmp     eax, EXT_DATA_DROP
    je      .ext_data_drop
    cmp     eax, EXT_MEMORY_COPY
    je      .ext_mem_copy
    cmp     eax, EXT_MEMORY_FILL
    je      .ext_mem_fill
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
    ; trunc_sat variants have no immediates
    jmp     .ext_done

.ext_mem_init:
    call    er_wasm_read_leb_u32  ; data segment index
    er_check_nonzero edx, .error
    call    er_wasm_read_leb_u32  ; memory index (must be 0)
    er_check_nonzero edx, .error
    er_check_nonzero eax, .unsupported
    jmp     .ext_done
.ext_data_drop:
    call    er_wasm_read_leb_u32  ; data segment index
    er_check_nonzero edx, .error
    jmp     .ext_done
.ext_mem_copy:
    call    er_wasm_read_leb_u32  ; memory index (dst)
    er_check_nonzero edx, .error
    er_check_nonzero eax, .unsupported
    call    er_wasm_read_leb_u32  ; memory index (src)
    er_check_nonzero edx, .error
    er_check_nonzero eax, .unsupported
    jmp     .ext_done
.ext_mem_fill:
    call    er_wasm_read_leb_u32  ; memory index
    er_check_nonzero edx, .error
    er_check_nonzero eax, .unsupported
    jmp     .ext_done
.ext_table_init:
    call    er_wasm_read_leb_u32  ; element segment index
    er_check_nonzero edx, .error
    call    er_wasm_read_leb_u32  ; table index
    er_check_nonzero edx, .error
    er_check_nonzero eax, .unsupported
    jmp     .ext_done
.ext_elem_drop:
    call    er_wasm_read_leb_u32  ; element segment index
    er_check_nonzero edx, .error
    jmp     .ext_done
.ext_table_copy:
    call    er_wasm_read_leb_u32  ; table index (dst)
    er_check_nonzero edx, .error
    er_check_nonzero eax, .unsupported
    call    er_wasm_read_leb_u32  ; table index (src)
    er_check_nonzero edx, .error
    er_check_nonzero eax, .unsupported
    jmp     .ext_done
.ext_table_grow:
    call    er_wasm_read_leb_u32  ; table index
    er_check_nonzero edx, .error
    er_check_nonzero eax, .unsupported
    jmp     .ext_done
.ext_table_size:
    call    er_wasm_read_leb_u32  ; table index
    er_check_nonzero edx, .error
    er_check_nonzero eax, .unsupported
    jmp     .ext_done
.ext_table_fill:
    call    er_wasm_read_leb_u32  ; table index
    er_check_nonzero edx, .error
    er_check_nonzero eax, .unsupported
    jmp     .ext_done
.ext_done:
    wasm_decode_finish_op
    jmp     .decode_loop

.done:
    er_ok
    jmp     .out
.unsupported:
    er_err  ERROR_UNSUPPORTED
    mov     eax, edx
    jmp     .out
.error:
    mov     eax, edx
.out:
    er_pop_ret rbp, rbx, r12, r13, r14, r15
