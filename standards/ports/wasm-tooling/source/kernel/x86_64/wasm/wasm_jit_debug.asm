; ==================================================================
; wasm_jit_debug.asm — JIT development debug utilities
;
; Uses Linux x86_64 syscalls for stdout output (userspace test only).
; Not included in kernel builds — include in test files as needed.
;
; Public API:
;   jit_debug_init() — one-time init (must call before other functions)
;   jit_debug_dump_code(rdi=code_ptr, rsi=byte_count)
;       Prints hex dump of compiled JIT code to stdout
;   jit_debug_dump_result(rax=result_value, rdx=error_code)
;       Prints "Result: <hex> (<signed>)" line
;   jit_debug_dump_header(rdi=func_idx, rsi=code_ptr, rdx=byte_count)
;       Prints "JIT compiled func N: M bytes at 0xADDR"
;   jit_debug_print_str(rdi=NUL-terminated string)
;       Prints a string to stdout
;   jit_debug_print_hex64(rax=value)
;       Prints "0x" + 16 hex digits
;   jit_debug_print_dec64(rax=value)
;       Prints signed decimal representation
;   jit_debug_newline()
;       Prints a newline to stdout
; =================================================================+

SECTION .bss
align 8
jit_debug_buf: resb 80  ; scratch buffer for number formatting

SECTION .text

; ------------------------------------------------------------------
; Init — must be called once before other debug functions
; (currently a no-op, reserved for future setup)
; -----------------------------------------------------------------+
er_fn jit_debug_init
    ret

; ------------------------------------------------------------------
; Write buffer to stdout via Linux syscall
; Called with: rdi = buf, rsi = len
; Syscall needs: rdi = fd, rsi = buf, rdx = len
; -----------------------------------------------------------------+
er_fn jit_debug_write
    mov     eax, 1          ; SYS_write
    mov     rdx, rsi        ; rdx = len
    mov     rsi, rdi        ; rsi = buf
    mov     edi, 1          ; fd = stdout
    syscall
    ret

; ------------------------------------------------------------------
; Print a single character to stdout
; al = char
; -----------------------------------------------------------------+
er_fn jit_debug_putchar
    sub     rsp, 16
    mov     [rsp], al
    lea     rdi, [rsp]
    mov     esi, 1
    call    jit_debug_write
    add     rsp, 16
    ret

; ------------------------------------------------------------------
; Print a newline
; -----------------------------------------------------------------+
er_fn jit_debug_newline
    mov     al, 0x0A
    call    jit_debug_putchar
    ret

; ------------------------------------------------------------------
; Print a space character
; -----------------------------------------------------------------+
er_fn jit_debug_space
    mov     al, 0x20
    call    jit_debug_putchar
    ret

; ------------------------------------------------------------------
; Print a NUL-terminated string
; rdi = string
; -----------------------------------------------------------------+
er_fn jit_debug_print_str
    er_push rdi, rcx
    mov     rcx, -1
    xor     eax, eax
    repne   scasb
    not     rcx
    dec     rcx              ; rcx = string length
    pop     rdi
    push    rcx
    pop     rsi              ; rsi = length
    pop     rdi              ; rdi = string
    call    jit_debug_write
    ret

; ------------------------------------------------------------------
; Print "0x" prefix
; -----------------------------------------------------------------+
er_fn jit_debug_print_0x
    mov     al, '0'
    call    jit_debug_putchar
    mov     al, 'x'
    call    jit_debug_putchar
    ret

; ------------------------------------------------------------------
; Print a hex nibble (low 4 bits of al)
; -----------------------------------------------------------------+
er_fn jit_debug_print_nibble
    and     al, 0x0F
    cmp     al, 10
    jb      .digit
    add     al, 'a' - 10
    jmp     .out
.digit:
    add     al, '0'
.out:
    call    jit_debug_putchar
    ret

; ------------------------------------------------------------------
; Print a byte as two hex characters
; al = byte value
; -----------------------------------------------------------------+
er_fn jit_debug_print_hex8
    push    rax
    shr     al, 4
    call    jit_debug_print_nibble
    pop     rax
    call    jit_debug_print_nibble
    ret

; ------------------------------------------------------------------
; Print a 64-bit value as "0x" + 16 hex digits
; rax = value
; Note: syscall clobbers rcx, so we use r8 as loop counter
; -----------------------------------------------------------------+
er_fn jit_debug_print_hex64
    er_push r8, rax

    call    jit_debug_print_0x
    mov     r8, 16              ; loop counter in r8 (syscall-safe)
    pop     rax                 ; rax = original value to display
    push    rax                 ; keep copy for restoration

.loop64:
    rol     rax, 4
    push    rax
    and     al, 0x0F
    cmp     al, 10
    jb      .digit64
    add     al, 'a' - 10
    jmp     .put64
.digit64:
    add     al, '0'
.put64:
    call    jit_debug_putchar
    pop     rax
    dec     r8
    jnz     .loop64

    pop     rax                 ; restore original rax
    pop     r8
    ret

; ------------------------------------------------------------------
; Print a 64-bit value as signed decimal
; rax = value
; -----------------------------------------------------------------+
er_fn jit_debug_print_dec64
    er_push rax, rbx, rcx, rdx, rdi

    test    rax, rax
    jns     .positive
    push    rax
    mov     al, '-'
    call    jit_debug_putchar
    pop     rax
    neg     rax
.positive:
    mov     rdi, 63         ; end of buffer
    lea     rbx, [rel jit_debug_buf + 63]
    mov     byte [rbx], 0
    mov     rcx, 10
.conv_loop:
    dec     rbx
    xor     rdx, rdx
    div     rcx
    add     dl, '0'
    mov     [rbx], dl
    er_check_nonzero rax, .conv_loop

    mov     rdi, rbx
    call    jit_debug_print_str

    er_pop_ret rax, rbx, rcx, rdx, rdi

; ------------------------------------------------------------------
; Print hex dump of JIT compiled code
; rdi = code_ptr, rsi = byte_count
;
; Output format:
;   JIT code at 0xADDR (N bytes):
;     0x00: 55 48 89 e5 b8 2a ...
; -----------------------------------------------------------------+
er_fn jit_debug_dump_code
    er_push rax, rbx, rcx, rdx, rdi, rsi, r8

    mov     r8, rdi             ; r8 = code_ptr
    mov     rbx, rsi            ; rbx = remaining bytes

    ; Print "JIT code at "
    lea     rdi, [rel .str_prefix]
    call    jit_debug_print_str

    ; Print address
    mov     rax, r8
    call    jit_debug_print_hex64

    ; Print " ("
    mov     al, ' '
    call    jit_debug_putchar
    mov     al, '('
    call    jit_debug_putchar

    ; Print byte count
    mov     rax, rbx
    call    jit_debug_print_dec64

    ; Print " bytes):\n"
    lea     rdi, [rel .str_bytes]
    call    jit_debug_print_str

    ; Hex dump loop: 16 bytes per line
    xor     rcx, rcx            ; rcx = offset within dump
.dump_loop:
    cmp     rbx, 0
    jle     .dump_done

    ; Print offset prefix: "  OFFSET: "
    call    jit_debug_space
    call    jit_debug_space
    mov     rax, rcx
    call    jit_debug_print_hex64
    mov     al, ':'
    call    jit_debug_putchar
    call    jit_debug_space

    ; Print up to 16 bytes
    mov     rdx, rbx
    cmp     rdx, 16
    jle     .last_line
    mov     rdx, 16
.last_line:
    push    rcx
    xor     rcx, rcx
.byte_loop:
    cmp     rcx, rdx
    jge     .byte_done
    mov     al, [r8 + rcx]
    call    jit_debug_print_hex8
    mov     al, ' '
    call    jit_debug_putchar
    inc     rcx
    jmp     .byte_loop
.byte_done:
    pop     rcx

    ; Advance
    add     r8, rdx
    sub     rbx, rdx
    add     rcx, rdx

    call    jit_debug_newline
    jmp     .dump_loop
.dump_done:

    er_pop_ret rax, rbx, rcx, rdx, rdi, rsi, r8

.str_prefix: db "JIT code at ", 0
.str_bytes:   db " bytes):", 0x0A, 0

; ------------------------------------------------------------------
; Print a summary header for a JIT compilation
; rdi = func_idx, rsi = code_ptr, rdx = byte_count
; -----------------------------------------------------------------+
er_fn jit_debug_dump_header
    er_push rax, rdi, rsi, rdx

    ; Print "JIT compiled func "
    lea     rdi, [rel .str_func]
    call    jit_debug_print_str

    ; Print func_idx — saved in rdi (3rd slot from bottom)
    mov     rax, [rsp + 16]     ; rdi was 3rd push
    call    jit_debug_print_dec64

    ; Print ": "
    mov     al, ':'
    call    jit_debug_putchar
    call    jit_debug_space

    ; Print byte count — saved in rdx (top of stack)
    mov     rax, [rsp]          ; rdx is on top
    call    jit_debug_print_dec64

    ; Print " bytes at "
    lea     rdi, [rel .str_bytes_at]
    call    jit_debug_print_str

    ; Print code address — saved in rsi (2nd slot from top)
    mov     rax, [rsp + 8]      ; rsi is 2nd from top
    call    jit_debug_print_hex64

    call    jit_debug_newline

    er_pop_ret rax, rdi, rsi, rdx

.str_func:     db "JIT compiled func ", 0
.str_bytes_at: db " bytes at ", 0

; ------------------------------------------------------------------
; Print execution result
; rax = result value, rdx = error code
; Returns rax and rdx unchanged (caller can continue using them).
; -----------------------------------------------------------------+
er_fn jit_debug_dump_result
    er_push rax, rdx

    lea     rdi, [rel .str_result]
    call    jit_debug_print_str

    er_pop  rax, rdx
    er_push rax, rdx

    call    jit_debug_print_hex64

    lea     rdi, [rel .str_paren]
    call    jit_debug_print_str

    er_pop  rax, rdx
    er_push rax, rdx

    call    jit_debug_print_dec64

    mov     al, ')'
    call    jit_debug_putchar

    call    jit_debug_newline        ; print newline before restoring regs

    ; Restore rax and rdx — must come after newline since newline clobbers rax
    er_pop  rax, rdx
    er_check_zero rdx, .no_error
    ; error path: print " ERROR: <code>"
    er_push rax, rdx
    lea     rdi, [rel .str_error]
    call    jit_debug_print_str
    pop     rax
    call    jit_debug_print_dec64
    call    jit_debug_newline
    pop     rax
.no_error:
    ret

.str_result: db "Result: ", 0
.str_paren:  db " (", 0
.str_error:  db " ERROR: ", 0
