; ==================================================================
; SECTION .text
; ==================================================================
SECTION .text

extern er_memcpy

; ==================================================================
; Helper: er_wasm_read_leb_u32
; Reads a LEB128 unsigned 32-bit value from [rsi], returns in rax.
; Updates rsi to point past the value.
; Returns error in rdx (0 = ok, nonzero = error).
; ==================================================================
er_wasm_read_leb_u32:
    xor     eax, eax
    xor     ecx, ecx            ; shift count (in bits)
    xor     r8d, r8d            ; byte counter
.leb_loop:
    cmp     r8d, LEB32_MAX_BYTES
    jge     .leb_error
    movzx   r9d, byte [rsi]
    inc     rsi
    mov     r10d, r9d
    and     r10d, LEB_PAYLOAD_MASK
    shl     r10d, cl
    or      eax, r10d
    add     ecx, LEB_BITS_PER_BYTE
    inc     r8d
    test    r9d, LEB_CONTINUE_MASK
    jnz     .leb_loop
    er_ok            ; no error
    ret
.leb_error:
    er_err  ERROR_CORRUPT
    ret

; ==================================================================
; Helper: er_wasm_read_leb_i32
; Reads a LEB128 signed 32-bit value from [rsi], returns in rax.
; Updates rsi to point past the value.
; Returns error in rdx.
; ==================================================================
er_wasm_read_leb_i32:
    xor     eax, eax
    xor     ecx, ecx
    xor     r8d, r8d
    mov     r9b, 0              ; will store last byte
.leb_loop_s:
    cmp     r8d, LEB32_MAX_BYTES
    jge     .leb_error_s
    mov     r9b, byte [rsi]
    inc     rsi
    mov     r10d, r9d
    and     r10d, LEB_PAYLOAD_MASK
    shl     r10d, cl
    or      eax, r10d
    add     ecx, LEB_BITS_PER_BYTE
    inc     r8d
    test    r9b, LEB_CONTINUE_MASK
    jnz     .leb_loop_s
    mov     ecx, r8d
    imul    ecx, LEB_BITS_PER_BYTE
    cmp     ecx, 32
    jge     .done_s
    test    r9b, LEB_SIGN_MASK
    jz      .done_s
    mov     r10d, -1
    shl     r10d, cl            ; shift -1 left by bits_read
    or      eax, r10d
.done_s:
    er_ok
    ret
.leb_error_s:
    er_err  ERROR_CORRUPT
    ret

; ==================================================================
; Helper: er_wasm_read_leb_i64
; Reads a LEB128 signed 64-bit value from [rsi], returns in rax.
; Updates rsi to point past the value.
; Returns error in rdx.
; ==================================================================
er_wasm_read_leb_i64:
    xor     eax, eax
    xor     ecx, ecx
    xor     r8d, r8d
    mov     r9b, 0
.leb_loop_l:
    cmp     r8d, LEB64_MAX_BYTES
    jge     .leb_error_l
    mov     r9b, byte [rsi]
    inc     rsi
    mov     r10, r9
    and     r10, LEB_PAYLOAD_MASK
    shl     r10, cl
    or      rax, r10
    add     ecx, LEB_BITS_PER_BYTE
    inc     r8d
    test    r9b, LEB_CONTINUE_MASK
    jnz     .leb_loop_l
    mov     ecx, r8d
    imul    ecx, LEB_BITS_PER_BYTE
    cmp     ecx, 64
    jge     .done_l
    test    r9b, LEB_SIGN_MASK
    jz      .done_l
    mov     r10, -1
    shl     r10, cl
    or      rax, r10
.done_l:
    er_ok
    ret
.leb_error_l:
    er_err  ERROR_CORRUPT
    ret

; ==================================================================
; Helper: er_wasm_read_value_type
; Reads a ValueType byte from [rsi], returns in al.
; Updates rsi. Returns error in rdx.
; ==================================================================
er_wasm_read_value_type:
    movzx   eax, byte [rsi]
    inc     rsi
    cmp     al, VALUE_TAG_I32
    je      .valid
    cmp     al, VALUE_TAG_I64
    je      .valid
    cmp     al, VALUE_TAG_F32
    je      .valid
    cmp     al, VALUE_TAG_F64
    je      .valid
    cmp     al, VALUE_TAG_FUNCREF
    je      .valid
    er_err  ERROR_UNSUPPORTED
    ret
.valid:
    er_ok
    ret

; ==================================================================
; Helper: er_wasm_read_limits
; Reads limits from [rsi], stores in [rcx] (Limits struct: min=0, max=8).
; Updates rsi.
; NOTE: er_wasm_read_leb_u32 clobbers rcx (shift counter).
;       Save rcx in rbx before calling it.
; ==================================================================
er_wasm_read_limits:
    push    rbx
    mov     rbx, rcx            ; preserve output pointer in callee-saved rbx
    movzx   eax, byte [rsi]
    inc     rsi
    cmp     al, LIMITS_MIN_ONLY
    je      .min_only
    cmp     al, LIMITS_MIN_MAX
    je      .min_max
    er_err  ERROR_UNSUPPORTED
    pop     rbx
    ret
.min_only:
    er_call er_wasm_read_leb_u32, .error
    mov     [rbx], rax          ; store min
    mov     qword [rbx + 8], 0  ; max = null (0)
    er_ok
    pop     rbx
    ret
.min_max:
    er_call er_wasm_read_leb_u32, .error
    mov     [rbx], rax          ; store min
    push    rax                 ; save min on stack for comparison
    er_call er_wasm_read_leb_u32, .error_pop
    pop     rcx                 ; restore min for comparison
    cmp     rax, rcx
    jb      .corrupt
    mov     [rbx + 8], rax      ; store max
    er_ok
    pop     rbx
    ret
.corrupt:
    er_err  ERROR_CORRUPT
    pop     rbx
    ret
.error_pop:
    add     rsp, 8              ; discard saved min
.error:
    pop     rbx
    ret

; ==================================================================
; Helper: er_wasm_read_constant_i32
; Reads i32.const + value + end expression from [rsi].
; Returns value in rax. Updates rsi.
; ==================================================================
er_wasm_read_constant_i32:
    movzx   eax, byte [rsi]
    inc     rsi
    cmp     al, 0x41            ; i32.const
    jne     .unsupported
    er_call er_wasm_read_leb_i32, .error
    push    rax                 ; save value
    movzx   eax, byte [rsi]
    inc     rsi
    cmp     al, 0x0b            ; end
    jne     .unsupported
    pop     rax
    er_ok
    ret
.unsupported:
    er_err  ERROR_UNSUPPORTED
.error:
    ret

; ==================================================================
; Helper: eql (string comparison)
; Compares two bytes at [rdi] (len in rsi) and [rdx] (len in rcx).
; Returns 1 in rax if equal, 0 otherwise.
; Clobbers: rdi, rsi, rcx, r8
; ==================================================================
er_wasm_eql:
    er_frame_push
    ; rdi = ptr1, rsi = len1, rdx = ptr2, rcx = len2
    cmp     rsi, rcx
    jne     .not_equal
    er_check_zero rsi, .equal
    mov     r8, rsi
    mov     rsi, rdx
    mov     rcx, r8
    cld
    repe    cmpsb
    jne     .not_equal
.equal:
    mov     eax, 1
    pop     rbp
    ret
.not_equal:
    xor     eax, eax
    pop     rbp
    ret
