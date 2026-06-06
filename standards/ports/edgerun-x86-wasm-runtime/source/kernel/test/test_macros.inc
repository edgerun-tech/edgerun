; EdgeRun self-hosted test assertion macros
;
; Usage:
;   %include "test/test_macros.inc"
;   SECTION .bss
;   total:  resq 1
;   passed: resq 1
;   failed: resq 1
;
; Tests increment qword [rel passed] on success,
; and always increment qword [rel total] (or [rel failed]).

%ifdef TEST_FLAT_KERNEL
%define _start er_kernel_main
%define TEST_DEBUG_EXIT_PORT 0xf4
%endif

; ------------------------------------------------------------------
; Basic comparison macros (eax/edx, rax/rdx)
; Expects: qword [rel passed] and qword [rel total] in .bss
; ------------------------------------------------------------------

; Standard counter layouts for self-hosted tests.
%macro TEST_BSS_TOTAL_PASSED 0
SECTION .bss
total:  resq 1
passed: resq 1
%endmacro

%macro TEST_BSS_TOTAL_PASSED_FAILED 0
SECTION .bss
total:  resq 1
passed: resq 1
failed: resq 1
%endmacro

%macro TEST_BSS_PASSED_FAILED 0
SECTION .bss
passed: resq 1
failed: resq 1
%endmacro

%macro TEST_DATA_PASSED_FAILED 0
SECTION .data
passed: dq 0
failed: dq 0
%endmacro

%macro TEST_DATA_TOTAL_PASSED_FAILED 0
SECTION .data
total:  dq 0
passed: dq 0
failed: dq 0
%endmacro

; Emit a short debug label to stdout without disturbing caller registers.
%macro TEST_DEBUG_LABEL 1
%ifndef TEST_FLAT_KERNEL
    jmp     %%done
%%str: db %1, 10
%%end:
%%done:
    push    rcx
    push    r11
    push    rdi
    push    rsi
    push    rdx
    push    rax
    mov     rdi, 1
    lea     rsi, [rel %%str]
    mov     rdx, %%end - %%str
    mov     rax, 1
    syscall
    pop     rax
    pop     rdx
    pop     rsi
    pop     rdi
    pop     r11
    pop     rcx
%endif
%endmacro

; Pass if eax == edx (32-bit)
%macro TEST 0
    cmp     eax, edx
    jne     %%fail
    inc     qword [rel passed]
%%fail:
    inc     qword [rel total]
%endmacro

; Pass if rax == rdx (64-bit)
%macro TESTQ 0
    cmp     rax, rdx
    jne     %%fail
    inc     qword [rel passed]
%%fail:
    inc     qword [rel total]
%endmacro

; Pass if [rdi] == [rsi] for ecx bytes
; Clobbers rdi, rsi, ecx, eax
%macro TEST_MEM 0
    cld
    repe    cmpsb
    jne     %%fail
    inc     qword [rel passed]
%%fail:
    inc     qword [rel total]
%endmacro

; Pass if dword at %1 equals immediate %2.
%macro TEST_DWORD_IMM 2
    mov     eax, dword %1
    mov     edx, %2
    TEST
%endmacro

; Pass if dword at %1 equals dword at %2.
%macro TEST_DWORD_MEM 2
    mov     eax, dword %1
    mov     edx, dword %2
    TEST
%endmacro

; Call %1 with edi=%2. Pass if eax == %3.
; Expects: qword [rel passed] and qword [rel total].
%macro TEST_CALL_EAX 3
    mov     edi, %2
    call    %1
    cmp     eax, %3
    jne     %%fail
    inc     qword [rel passed]
%%fail:
    inc     qword [rel total]
%endmacro

; Compare two zero-terminated strings through er_strcmp.
; Expects: qword [rel passed], [rel failed], and [rel total].
%macro TEST_STREQ 2
    lea     rdi, %1
    lea     rsi, %2
    call    er_strcmp
    test    eax, eax
    jnz     %%fail
    inc     qword [rel passed]
    jmp     %%done
%%fail:
    inc     qword [rel failed]
%%done:
    inc     qword [rel total]
%endmacro

; ------------------------------------------------------------------
; Assert macros (reg, reg comparison)
; Expects: qword [rel passed] and qword [rel failed] in .bss
; ------------------------------------------------------------------

; Assert %1 is non-zero.
%macro ASSERT_TRUE 1
    test    %1, %1
    jz      %%fail
    inc     qword [rel passed]
    jmp     %%done
%%fail:
    inc     qword [rel failed]
%%done:
%endmacro

; Assert %1 is non-zero; exit immediately with %2 on failure.
%macro ASSERT_TRUE_EXIT 2
    test    %1, %1
    jz      %%fail
    inc     qword [rel passed]
    jmp     %%done
%%fail:
    inc     qword [rel failed]
    TEST_EXIT %2
%%done:
%endmacro

; Assert %1 == %2 (any same-size registers)
%macro ASSERT_EQ 2
    cmp     %1, %2
    jne     %%fail
    inc     qword [rel passed]
    jmp     %%done
%%fail:
    inc     qword [rel failed]
%%done:
%endmacro

; Assert dword at %1 equals %2.
%macro ASSERT_DWORD 2
    ASSERT_EQ dword %1, %2
%endmacro

; Assert qword at %1 equals %2.
%macro ASSERT_QWORD 2
    ASSERT_EQ qword %1, %2
%endmacro

; Assert byte at %1 equals %2.
%macro ASSERT_BYTE 2
    ASSERT_EQ byte %1, %2
%endmacro

; Assert byte at %1 masked by %2 equals %3. Clobbers eax.
%macro ASSERT_BYTE_MASK 3
    movzx   eax, byte %1
    and     eax, %2
    ASSERT_EQ eax, %3
%endmacro

; Assert word at %1 equals %2.
%macro ASSERT_WORD 2
    ASSERT_EQ word %1, %2
%endmacro

; Assert rax == immediate
%macro ASSERT_RAX 1
    cmp     rax, %1
    jne     %%fail
    inc     qword [rel passed]
    jmp     %%done
%%fail:
    inc     qword [rel failed]
%%done:
%endmacro

; Assert rdx == immediate
%macro ASSERT_RDX 1
    cmp     rdx, %1
    jne     %%fail
    inc     qword [rel passed]
    jmp     %%done
%%fail:
    inc     qword [rel failed]
%%done:
%endmacro

; Assert all bytes at [rdi] equal byte %1 for %2 bytes
; Clobbers rcx, rdi, al
%macro ASSERT_MEM_ALL 2
    push    rcx
    push    rdi
    push    rsi
    mov     rcx, %2
    mov     al, %1
%%loop:
    cmp     [rdi], al
    jne     %%fail
    inc     rdi
    dec     rcx
    jnz     %%loop
    inc     qword [rel passed]
    jmp     %%done
%%fail:
    inc     qword [rel failed]
%%done:
    pop     rsi
    pop     rdi
    pop     rcx
%endmacro

; Assert all %2 bytes at [rdi] are zero
; Clobbers rcx, rdi, eax
%macro ASSERT_MEM_ZERO 2
    push    rcx
    push    rdi
    push    rsi
    mov     rcx, %2
    xor     eax, eax
%%loop:
    cmp     [rdi], al
    jne     %%fail
    inc     rdi
    dec     rcx
    jnz     %%loop
    inc     qword [rel passed]
    jmp     %%done
%%fail:
    inc     qword [rel failed]
%%done:
    pop     rsi
    pop     rdi
    pop     rcx
%endmacro

; Assert u64 at [rdi] == %1; clobbers rax, rdi
%macro ASSERT_MEM_U64 1
    mov     rax, [rdi]
    cmp     rax, %1
    jne     %%fail
    inc     qword [rel passed]
    jmp     %%done
%%fail:
    inc     qword [rel failed]
%%done:
%endmacro

; Assert memory ranges are equal.
; Arguments are effective addresses and byte count, e.g. [rel want], [rel got], 32.
; Clobbers rdi, rsi, rcx.
%macro ASSERT_MEM_EQ 3
    push    rdi
    push    rsi
    push    rcx
    lea     rdi, %1
    lea     rsi, %2
    mov     rcx, %3
    cld
    repe    cmpsb
    jne     %%fail
    inc     qword [rel passed]
    jmp     %%done
%%fail:
    inc     qword [rel failed]
%%done:
    pop     rcx
    pop     rsi
    pop     rdi
%endmacro

; Assert memory ranges are equal; exit immediately with %4 on failure.
%macro ASSERT_MEM_EQ_EXIT 4
    push    rdi
    push    rsi
    push    rcx
    lea     rdi, %1
    lea     rsi, %2
    mov     rcx, %3
    cld
    repe    cmpsb
    jne     %%fail
    inc     qword [rel passed]
    jmp     %%done
%%fail:
    inc     qword [rel failed]
    TEST_EXIT %4
%%done:
    pop     rcx
    pop     rsi
    pop     rdi
%endmacro

; Branch to %2 unless eax equals %1. Does not update counters.
%macro EXPECT_EAX 2
    cmp     eax, %1
    jne     %2
%endmacro

; Branch to %3 unless qword at %1 equals %2. Clobbers rax.
%macro EXPECT_QWORD 3
    mov     rax, qword %1
    cmp     rax, %2
    jne     %3
%endmacro

; Branch to %3 unless dword at %1 equals %2. Clobbers eax.
%macro EXPECT_DWORD 3
    mov     eax, dword %1
    cmp     eax, %2
    jne     %3
%endmacro

; Mark one whole test case as passed.
%macro TEST_CASE_PASS 0
    inc     qword [rel passed]
%endmacro

; Mark one whole test case as failed.
%macro TEST_CASE_FAIL 0
    inc     qword [rel failed]
%endmacro

; Exit with the supplied status code.
%macro TEST_EXIT 1
%ifdef TEST_FLAT_KERNEL
    mov     dx, TEST_DEBUG_EXIT_PORT
    mov     eax, %1
    out     dx, eax
%%halt:
    cli
    hlt
    jmp     %%halt
%else
    mov     edi, %1
    mov     eax, 60
    syscall
%endif
%endmacro

; Exit with status already in edi.
%macro TEST_EXIT_EDI 0
%ifdef TEST_FLAT_KERNEL
    mov     dx, TEST_DEBUG_EXIT_PORT
    mov     eax, edi
    out     dx, eax
%%halt:
    cli
    hlt
    jmp     %%halt
%else
    mov     eax, 60
    syscall
%endif
%endmacro

; Exit with status 1 if failed is non-zero, else 0.
%macro TEST_EXIT_FAILED 0
    xor     edi, edi
    cmp     qword [rel failed], 0
    sete    dil
    xor     dil, 1
    TEST_EXIT_EDI
%endmacro

; Exit with status 0 when passed equals total, else 1.
%macro TEST_EXIT_PASSED_TOTAL 0
    mov     rax, [rel passed]
    mov     rdx, [rel total]
    cmp     rax, rdx
    sete    al
    xor     al, 1
    movzx   edi, al
    TEST_EXIT_EDI
%endmacro

; Exit with status 0 when eax is zero, else status 1.
%macro TEST_EXIT_EAX_ZERO 0
    xor     edi, edi
    test    eax, eax
    setnz   dil
    TEST_EXIT_EDI
%endmacro

; Exit with status 0 when qword at %1 is zero, else 1.
%macro TEST_EXIT_QWORD_ZERO 1
    xor     edi, edi
    cmp     qword %1, 0
    setne   dil
    TEST_EXIT_EDI
%endmacro

; Return eax=0 if failed is zero, else eax=-1.
%macro TEST_RETURN_FAILED 0
    xor     eax, eax
    cmp     qword [rel failed], 0
    je      %%done
    mov     eax, -1
%%done:
%endmacro
