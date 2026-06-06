; EdgeRun WASM recursion validation test — valid DAG (no cycles).
; Tests that a non-recursive call graph passes validation.
;
; Build: yasm -f elf64 -I kernel -o test_recursion_valid.o test_recursion_valid.asm
; Link:  ld -T kernel/test/test_jit.ld -nostdlib -static -o test_recursion_valid \
;          test_recursion_valid.o kernel_build/runtime.o
; Run:   ./test_recursion_valid

%define HAVE_ER_WASM_RUNTIME_PTR
%include "x86_64/wasm/wasm_interpreter.asm"
%include "test/test_macros.inc"

SECTION .data

; Dummy linear memory
dummy_mem:    times 256 db 0

; WASM bytecodes (not used directly, but code_buf needs valid pointers)
body0:        db 0x10, 0x01, 0x0B     ; call 1, end
body1:        db 0x10, 0x02, 0x0B     ; call 2, end
body2:        db 0x41, 0x2A, 0x0B     ; i32.const 42, end

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
    mov     qword [rel function_count], 3

    ; --- Type table: 3 types, each FUNC_TYPE_SIZE=256 bytes ---
    ; All types: 0 params, 1 result
    mov     qword [rel types_buf + FUNC_TYPE_PARAM_COUNT_OFF], 0
    mov     qword [rel types_buf + FUNC_TYPE_RESULT_COUNT_OFF], 1
    mov     qword [rel types_buf + 1*FUNC_TYPE_SIZE + FUNC_TYPE_PARAM_COUNT_OFF], 0
    mov     qword [rel types_buf + 1*FUNC_TYPE_SIZE + FUNC_TYPE_RESULT_COUNT_OFF], 1
    mov     qword [rel types_buf + 2*FUNC_TYPE_SIZE + FUNC_TYPE_PARAM_COUNT_OFF], 0
    mov     qword [rel types_buf + 2*FUNC_TYPE_SIZE + FUNC_TYPE_RESULT_COUNT_OFF], 1

    ; --- Functions table ---
    mov     qword [rel functions_buf], 0        ; func 0: type_index = 0
    mov     qword [rel functions_buf + 8], 0    ; func 0: code_index = 0
    mov     qword [rel functions_buf + 16], 1   ; func 1: type_index = 1
    mov     qword [rel functions_buf + 24], 1   ; func 1: code_index = 1
    mov     qword [rel functions_buf + 32], 2   ; func 2: type_index = 2
    mov     qword [rel functions_buf + 40], 2   ; func 2: code_index = 2

    ; --- Code table (CODE_SIZE=64 bytes per entry) ---
    ; Func 0: call 1, end
    lea     rax, [rel body0]
    mov     [rel code_buf], rax                 ; +0: body_ptr
    mov     qword [rel code_buf + 8], 3         ; +8: body_len
    mov     qword [rel code_buf + 16], 0        ; +16: local_count
    mov     qword [rel code_buf + 24], 0        ; +24: decoded_start
    mov     qword [rel code_buf + 32], 2        ; +32: decoded_count

    ; Func 1: call 2, end
    lea     rax, [rel body1]
    mov     [rel code_buf + 64], rax
    mov     qword [rel code_buf + 72], 3
    mov     qword [rel code_buf + 80], 0
    mov     qword [rel code_buf + 88], 2
    mov     qword [rel code_buf + 96], 2

    ; Func 2: i32.const 42, end
    lea     rax, [rel body2]
    mov     [rel code_buf + 128], rax
    mov     qword [rel code_buf + 136], 3
    mov     qword [rel code_buf + 144], 0
    mov     qword [rel code_buf + 152], 4
    mov     qword [rel code_buf + 160], 2

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

    ; Slot 2: call(0x10), imm0=2 (target func 2)
    mov     dword [rel decoded_ops + 64],  0
    mov     dword [rel decoded_ops + 68],  2
    mov     byte  [rel decoded_ops + 72],  0x10
    mov     dword [rel decoded_ops + 76], 2

    ; Slot 3: end(0x0B)
    mov     dword [rel decoded_ops + 96],  2
    mov     dword [rel decoded_ops + 100], 3
    mov     byte  [rel decoded_ops + 104], 0x0B

    ; Slot 4: i32.const(0x41), imm0=42
    mov     dword [rel decoded_ops + 128], 0
    mov     dword [rel decoded_ops + 132], 2
    mov     byte  [rel decoded_ops + 136], 0x41
    mov     dword [rel decoded_ops + 140], 42

    ; Slot 5: end(0x0B)
    mov     dword [rel decoded_ops + 160], 2
    mov     dword [rel decoded_ops + 164], 3
    mov     byte  [rel decoded_ops + 168], 0x0B

    mov     qword [rel decoded_op_count], 6

    ; =================================================================
    ; Test: validate a non-recursive DAG (func0 -> func1 -> func2)
    ; Expected: rdx = 0 (success)
    ; =================================================================

    call    er_wasm_validate_no_recursion

    test    rdx, rdx
    jnz     .fail

    ; --- PASS ---
    TEST_EXIT 0

.fail:
    TEST_EXIT 1
