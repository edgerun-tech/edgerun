; x86_64 assembly macros for EdgeRun
; NASM/yasm syntax — our own assembler is the long-term goal
;
; Usage:
;   %include "x86_64/macros.inc"

%ifndef MACROS_INC
%define MACROS_INC

[BITS 64]

; ------------------------------------------------------------------
; Register aliases — System V AMD64 ABI
; ------------------------------------------------------------------
; Argument registers (integer)
%define arg1    rdi
%define arg2    rsi
%define arg3    rdx
%define arg4    rcx
%define arg5    r8
%define arg6    r9

; Return register
%define retval  rax

; Callee-saved GPRs (must preserve)
%define saved1  rbx
%define saved2  rbp
%define saved3  r12
%define saved4  r13
%define saved5  r14
%define saved6  r15

; ------------------------------------------------------------------
; Useful IEEE 754 single-precision constants (as bit patterns)
; ------------------------------------------------------------------
%define FLOAT_SIGN_BIT    0x80000000
%define FLOAT_ABS_MASK    0x7fffffff
%define FLOAT_NAN         0x7fc00000
%define FLOAT_POS_INF     0x7f800000
%define FLOAT_NEG_INF     0xff800000
%define FLOAT_ONE         0x3f800000
%define FLOAT_HALF        0x3f000000
%define FLOAT_TWO         0x40000000
%define FLOAT_ZERO        0x00000000

; Double-precision constants
%define DOUBLE_SIGN_BIT   0x8000000000000000
%define DOUBLE_ABS_MASK   0x7fffffffffffffff

; Integer sizes
%define WORD_SIZE     8
%define DWORD_SIZE    4
%define CACHE_LINE    64
%define PAGE_SIZE     4096

; ------------------------------------------------------------------
; Function prologue / epilogue
; ------------------------------------------------------------------
; Begin a function with standard cfi + 16-byte alignment
%macro er_fn 1
    SECTION .text
    align 16
    global %1
    %1:
%endm

; Return from function
%macro er_ret 0
    ret
%endm

; Set success (rdx = 0, no error)
%macro er_ok 0
    xor     edx, edx
%endm

; Set error code in rdx
%macro er_err 1
    mov     edx, %1
%endm

; ------------------------------------------------------------------
; Call and check error (two-register convention)
; Calls function, then checks rdx for error; jumps to fail_label if non-zero
; ------------------------------------------------------------------
%macro er_call 2
    call    %1
    test    edx, edx
    jnz     %2
%endm

; ------------------------------------------------------------------
; Stack frame macros
; ------------------------------------------------------------------
; Push frame pointer, set up new frame
%macro er_frame_push 0
    push    rbp
    mov     rbp, rsp
%endm

; Pop frame pointer
%macro er_frame_pop 0
    pop     rbp
%endm

; Push callee-saved registers (list subsets, e.g. er_push rbx, r12, r13)
%macro er_push 1-*
    %rep %0
        push    %1
        %rotate 1
    %endrep
%endm

; Push frame pointer, set it, then save registers.
%macro er_frame_push_regs 1-*
    push    rbp
    mov     rbp, rsp
    %rep %0
        push    %1
        %rotate 1
    %endrep
%endm

; Pop callee-saved registers (reverse order)
%macro er_pop 1-*
    %rep %0
        %rotate -1
        pop     %1
    %endrep
%endm

; Restore registers and return.
%macro er_pop_ret 1-*
    %rep %0
        %rotate -1
        pop     %1
    %endrep
    ret
%endm

; Push all callee-saved GPRs
%macro er_push_all 0
    er_push rbx, rbp, r12, r13, r14, r15
%endm

; Pop all callee-saved GPRs
%macro er_pop_all 0
    er_pop  rbx, rbp, r12, r13, r14, r15
%endm

; Allocate N bytes on the stack (N should be multiple of 16)
%macro er_stack_alloc 1
    sub     rsp, %1
%endm

; Deallocate N bytes from the stack
%macro er_stack_free 1
    add     rsp, %1
%endm

; Align RSP to 16 bytes
%macro er_align_stack 0
    and     rsp, -16
%endm

; ------------------------------------------------------------------
; Indexed fixed-size struct field helpers
; ------------------------------------------------------------------
; These preserve r10 while using it as the address scratch register.
%macro er_load_struct_qword 5
    push    r10
    mov     r10, %3
    imul    r10, %4
    mov     %1, qword [%2 + r10 + %5]
    pop     r10
%endm

%macro er_load_struct_byte 5
    push    r10
    mov     r10, %3
    imul    r10, %4
    mov     %1, byte [%2 + r10 + %5]
    pop     r10
%endm

%macro er_store_struct_qword 5
    push    r10
    mov     r10, %2
    imul    r10, %3
    mov     qword [%1 + r10 + %4], %5
    pop     r10
%endm

%macro er_store_struct_byte 5
    push    r10
    mov     r10, %2
    imul    r10, %3
    mov     byte [%1 + r10 + %4], %5
    pop     r10
%endm

; ------------------------------------------------------------------
; Combined prologue / epilogue — balanced push/pop pairs
; ------------------------------------------------------------------
; Standard function prologue: frame pointer + all callee-saved regs
%macro er_prologue 0
    er_frame_push
    er_push_all
%endm

; Standard function epilogue: restore callee-saved regs + frame + return
%macro er_epilogue 0
    er_pop_all
    er_frame_pop
    ret
%endm

; ------------------------------------------------------------------
; Null / zero check helpers
; ------------------------------------------------------------------
; Jump to label if register is zero
%macro er_check_zero 2
    test    %1, %1
    jz      %2
%endm

; Jump to label if register is non-zero
%macro er_check_nonzero 2
    test    %1, %1
    jnz     %2
%endm

; ------------------------------------------------------------------
; MMIO helpers (32-bit)
; ------------------------------------------------------------------
; Read 32-bit MMIO at [base + offset] into eax
%macro er_read32 2
    mov     eax, [%1 + %2]
%endm

; Write 32-bit value to MMIO at [base + offset]
%macro er_write32 3
    mov     [%1 + %2], %3
%endm

%endif ; MACROS_INC
