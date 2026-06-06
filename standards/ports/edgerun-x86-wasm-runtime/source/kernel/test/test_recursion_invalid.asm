; EdgeRun WASM recursion validation test — invalid (has cycle).
; Tests that a recursive call graph is rejected with ERROR_RECURSION.
;
; Build: yasm -f elf64 -I kernel -o test_recursion_invalid.o test_recursion_invalid.asm
; Link:  ld -T kernel/test/test_jit.ld -nostdlib -static -o test_recursion_invalid \
;          test_recursion_invalid.o kernel_build/runtime.o
; Run:   ./test_recursion_invalid

%define HAVE_ER_WASM_RUNTIME_PTR
%include "x86_64/wasm/wasm_interpreter.asm"
%include "test/test_macros.inc"

SECTION .data

; Dummy linear memory
dummy_mem:    times 256 db 0

; WASM bytecodes (not used directly)
body0:        db 0x10, 0x01, 0x0B     ; call 1, end
body1:        db 0x10, 0x00, 0x0B     ; call 0, end  ← cycle!

SECTION .bss
test_fail: resq 1

; Stub for er_wasm_runtime_ptr — referenced by wasm_run.asm
global er_wasm_runtime_ptr
er_wasm_runtime_ptr: resq 1

SECTION .text
global _start
_start:

    mov     qword [rel test_fail], 0

    ; --- Common runtime memory setup ---
    lea     rax, [rel dummy_mem]
    mov     [rel runtime_memory_ptr], rax
    mov     qword [rel runtime_memory_len], 256
    mov     qword [rel executor_memory_limit], 256

    mov     qword [rel import_count], 0
    mov     qword [rel function_count], 2

    ; --- Type table: 2 types ---
    mov     qword [rel types_buf + FUNC_TYPE_PARAM_COUNT_OFF], 0
    mov     qword [rel types_buf + FUNC_TYPE_RESULT_COUNT_OFF], 1
    mov     qword [rel types_buf + 1*FUNC_TYPE_SIZE + FUNC_TYPE_PARAM_COUNT_OFF], 0
    mov     qword [rel types_buf + 1*FUNC_TYPE_SIZE + FUNC_TYPE_RESULT_COUNT_OFF], 1

    ; --- Functions table ---
    mov     qword [rel functions_buf], 0        ; func 0: type_index = 0
    mov     qword [rel functions_buf + 8], 0    ; func 0: code_index = 0
    mov     qword [rel functions_buf + 16], 1   ; func 1: type_index = 1
    mov     qword [rel functions_buf + 24], 1   ; func 1: code_index = 1

    ; --- Code table ---
    ; Func 0: call 1, end
    lea     rax, [rel body0]
    mov     [rel code_buf], rax
    mov     qword [rel code_buf + 8], 3
    mov     qword [rel code_buf + 16], 0
    mov     qword [rel code_buf + 24], 0
    mov     qword [rel code_buf + 32], 2

    ; Func 1: call 0, end (creates cycle!)
    lea     rax, [rel body1]
    mov     [rel code_buf + 64], rax
    mov     qword [rel code_buf + 72], 3
    mov     qword [rel code_buf + 80], 0
    mov     qword [rel code_buf + 88], 2
    mov     qword [rel code_buf + 96], 2

    ; --- Decoded ops array ---
    ; Slot 0: call(0x10), imm0=1 (target func 1)
    mov     dword [rel decoded_ops + 0],  0
    mov     dword [rel decoded_ops + 4],  2
    mov     byte  [rel decoded_ops + 8],  0x10
    mov     dword [rel decoded_ops + 12], 1

    ; Slot 1: end(0x0B)
    mov     dword [rel decoded_ops + 32], 2
    mov     dword [rel decoded_ops + 36], 3
    mov     byte  [rel decoded_ops + 40], 0x0B

    ; Slot 2: call(0x10), imm0=0 (target func 0 — back-edge cycle)
    mov     dword [rel decoded_ops + 64],  0
    mov     dword [rel decoded_ops + 68],  2
    mov     byte  [rel decoded_ops + 72],  0x10
    mov     dword [rel decoded_ops + 76], 0

    ; Slot 3: end(0x0B)
    mov     dword [rel decoded_ops + 96],  2
    mov     dword [rel decoded_ops + 100], 3
    mov     byte  [rel decoded_ops + 104], 0x0B

    mov     qword [rel decoded_op_count], 4

    ; =================================================================
    ; Test: validate a recursive call graph (func0 -> func1 -> func0)
    ; Expected: rdx = ERROR_RECURSION (31)
    ; =================================================================

    call    er_wasm_validate_no_recursion

    cmp     rdx, ERROR_RECURSION
    jne     .fail

    ; --- PASS ---
    TEST_EXIT 0

.fail:
    TEST_EXIT 1
