; EdgeRun WASM JIT self-hosted test runner — x86_64 assembly
; Tests JIT compilation + execution with debug dump output.
;
; Build: yasm -f elf64 -I kernel -o test_wasm_jit_self.o test_wasm_jit_self.asm
; Link:  ld -T kernel/test/test_jit.ld -nostdlib -static -o test_wasm_jit \
;          test_wasm_jit_self.o kernel_build/runtime.o
; Run:   ./test_wasm_jit

%define HAVE_ER_WASM_RUNTIME_PTR
%include "x86_64/wasm/wasm_interpreter.asm"
%include "x86_64/wasm/wasm_jit_debug.asm"
%include "test/test_macros.inc"

LINUX_SYS_MPROTECT equ 10
LINUX_PROT_READ    equ 1
LINUX_PROT_WRITE   equ 2
LINUX_PROT_EXEC    equ 4
LINUX_PAGE_SIZE    equ 4096
LINUX_PAGE_MASK    equ -LINUX_PAGE_SIZE

; ==================================================================
; Data
; =================================================================+
SECTION .data

; Dummy linear memory (prevents null deref in templates)
dummy_mem:    times 256 db 0

; WASM bytecode for func 0: block i32.const 42 end → result = 42
body0:        db 0x02, 0x7F, 0x41, 0x2A, 0x0B
BODY0_LEN     equ 5

; WASM bytecode for func 1: call(0) end → calls func 0, returns its result
body1:        db 0x10, 0x00, 0x0B
BODY1_LEN     equ 3
missing_export_name: db "missing"
MISSING_EXPORT_NAME_LEN equ 7

SECTION .bss
saved_rax: resq 1
saved_rdx: resq 1
runtime_a: resb RUNTIME_SIZE
runtime_b: resb RUNTIME_SIZE

; er_wasm_runtime_ptr provided here so wasm_run.asm doesn't extern it
global er_wasm_runtime_ptr
er_wasm_runtime_ptr: resq 1

SECTION .text
global _start
_start:

    call    jit_debug_init

    ; er_fn_call requires a successfully loaded module for the same runtime.
    mov     byte [rel exec_storage_module_valid], 0
    lea     rdi, [rel runtime_a]
    lea     rsi, [rel missing_export_name]
    mov     edx, MISSING_EXPORT_NAME_LEN
    call    er_fn_call
    cmp     rdx, ERROR_BAD_ARGUMENT
    jne     .fail

    mov     byte [rel exec_storage_module_valid], 1
    lea     rax, [rel runtime_a]
    mov     [rel executor_runtime_ptr], rax
    lea     rdi, [rel runtime_b]
    lea     rsi, [rel missing_export_name]
    mov     edx, MISSING_EXPORT_NAME_LEN
    call    er_fn_call
    cmp     rdx, ERROR_BAD_ARGUMENT
    jne     .fail
    mov     byte [rel exec_storage_module_valid], 0
    mov     qword [rel executor_runtime_ptr], 0

    lea     rdi, [rel jit_code_cache]
    and     rdi, LINUX_PAGE_MASK
    lea     rsi, [rel jit_code_cache + JIT_CACHE_SIZE + LINUX_PAGE_SIZE - 1]
    and     rsi, LINUX_PAGE_MASK
    sub     rsi, rdi
    mov     edx, LINUX_PROT_READ | LINUX_PROT_WRITE | LINUX_PROT_EXEC
    mov     eax, LINUX_SYS_MPROTECT
    syscall
    test    rax, rax
    jnz     .fail

    ; --- Common runtime memory setup ---
    lea     rax, [rel dummy_mem]
    mov     [rel runtime_memory_ptr], rax
    mov     qword [rel runtime_memory_len], 256
    mov     qword [rel executor_memory_limit], 256

    mov     qword [rel import_count], 0
    mov     qword [rel function_count], 2

    ; --- Type table: 2 types, each FUNC_TYPE_SIZE=256 bytes ---
    ; Type 0: 0 params, 1 result
    mov     qword [rel types_buf + FUNC_TYPE_PARAM_COUNT_OFF], 0
    mov     qword [rel types_buf + FUNC_TYPE_RESULT_COUNT_OFF], 1
    ; Type 1: 0 params, 1 result
    mov     qword [rel types_buf + 1*FUNC_TYPE_SIZE + FUNC_TYPE_PARAM_COUNT_OFF], 0
    mov     qword [rel types_buf + 1*FUNC_TYPE_SIZE + FUNC_TYPE_RESULT_COUNT_OFF], 1

    ; --- Functions table: maps func_idx → type_index, code_index ---
    mov     qword [rel functions_buf], 0        ; func 0: type_index = 0
    mov     qword [rel functions_buf + 8], 0    ; func 0: code_index = 0
    mov     qword [rel functions_buf + 16], 1   ; func 1: type_index = 1
    mov     qword [rel functions_buf + 24], 1   ; func 1: code_index = 1

    ; --- Code table: maps code_index → body, len, locals, decoded_ops range ---
    ; Each entry is CODE_SIZE=64 bytes.

    ; Entries for func 0 (code index 0, at code_buf + 0*64)
    lea     rax, [rel body0]
    mov     [rel code_buf], rax                 ; +0: body_ptr
    mov     qword [rel code_buf + 8], BODY0_LEN ; +8: body_len
    mov     qword [rel code_buf + 16], 0        ; +16: local_count
    mov     qword [rel code_buf + 24], 0        ; +24: decoded_start
    mov     qword [rel code_buf + 32], 3        ; +32: decoded_count
    ; Total decoded ops for func 0: 3 (block, i32.const, end)

    ; Entries for func 1 (code index 1, at code_buf + 1*64)
    lea     rax, [rel body1]
    mov     [rel code_buf + 64], rax            ; +64+0: body_ptr
    mov     qword [rel code_buf + 72], BODY1_LEN ; +64+8: body_len
    mov     qword [rel code_buf + 80], 0        ; +64+16: local_count
    mov     qword [rel code_buf + 88], 3        ; +64+24: decoded_start
    mov     qword [rel code_buf + 96], 2        ; +64+32: decoded_count
    ; Total decoded ops for func 1: 2 (call, end), starting at slot 3

    ; --- Decoded ops array ---
    ; Each op: DECODED_OP_SIZE=32 bytes.
    ; Fields: offset(4) + next_offset(4) + opcode(1) + pad(3) + imm0(4) + imm1(4) + pad(12)

    ; Slot 0: block(0x02), imm0 = 0x7F (i32 block type)
    mov     dword [rel decoded_ops + 0],  0     ; offset=0
    mov     dword [rel decoded_ops + 4],  2     ; next_offset=2
    mov     byte  [rel decoded_ops + 8],  0x02
    mov     dword [rel decoded_ops + 12], 0x7F

    ; Slot 1: i32.const(0x41), imm0=42
    mov     dword [rel decoded_ops + 32], 2
    mov     dword [rel decoded_ops + 36], 4
    mov     byte  [rel decoded_ops + 40], 0x41
    mov     dword [rel decoded_ops + 44], 42

    ; Slot 2: end(0x0B)
    mov     dword [rel decoded_ops + 64], 4
    mov     dword [rel decoded_ops + 68], 5
    mov     byte  [rel decoded_ops + 72], 0x0B

    ; Slot 3: call(0x10), imm0=0 (func index 0)
    mov     dword [rel decoded_ops + 96],  0
    mov     dword [rel decoded_ops + 100], 2
    mov     byte  [rel decoded_ops + 104], 0x10
    mov     dword [rel decoded_ops + 108], 0

    ; Slot 4: end(0x0B)
    mov     dword [rel decoded_ops + 128], 2
    mov     dword [rel decoded_ops + 132], 3
    mov     byte  [rel decoded_ops + 136], 0x0B

    mov     qword [rel decoded_op_count], 5

    ; =================================================================
    ; Test 1: JIT compile + execute func 0 (block i32.const 42 end)
    ; =================================================================
    lea     rdi, [rel .str_test1]
    call    jit_debug_print_str
    call    jit_debug_newline

    xor     edi, edi
    call    er_wasm_jit_compile
    test    rdx, rdx
    jnz     .fail

    ; Dump compiled code
    mov     rdi, 0
    mov     rsi, rax
    mov     rdx, [rel jit_state.code_ptr]
    sub     rdx, [rel jit_state.cache_base]
    call    jit_debug_dump_header

    mov     rdi, rax                    ; code_ptr
    mov     rsi, [rel jit_state.code_ptr]
    sub     rsi, [rel jit_state.cache_base]
    call    jit_debug_dump_code

    ; Execute
    lea     r15, [rel jit_globals]
    mov     rax, [rel jit_table + 0]
    call    rax

    mov     rdx, 0
    cmp     rax, 42
    cmovne  rdx, rax
    call    jit_debug_dump_result
    cmp     rax, 42
    jne     .fail

    ; =================================================================
    ; Test 1.5: Call func 0 through the explicit JIT executor
    ; =================================================================
    lea     rdi, [rel .str_test1b]
    call    jit_debug_print_str
    call    jit_debug_newline

    xor     edi, edi            ; func_idx = 0
    xor     esi, esi            ; args = NULL (0 params)
    xor     edx, edx            ; arg_count = 0
    call    er_wasm_jit_exec

    mov     rdx, 0
    cmp     rax, 42
    cmovne  rdx, rax
    call    jit_debug_dump_result
    cmp     rax, 42
    jne     .fail

    ; =================================================================
    ; Test 1.6: Same but after clearing jit_table
    ; =================================================================
    lea     rdi, [rel .str_test1c]
    call    jit_debug_print_str
    call    jit_debug_newline

    mov     qword [rel jit_table + 0], 0
    mov     qword [rel jit_table + 8], 0
    mov     qword [rel jit_table + 16], 0
    mov     qword [rel jit_table + 24], 0

    xor     edi, edi            ; func_idx = 0
    xor     esi, esi            ; args = NULL (0 params)
    xor     edx, edx            ; arg_count = 0
    call    er_wasm_jit_exec

    mov     rdx, 0
    cmp     rax, 42
    cmovne  rdx, rax
    call    jit_debug_dump_result
    cmp     rax, 42
    jne     .fail

    ; =================================================================
    ; Test 2: JIT compile + execute func 1 (call(0) end) → should get 42
    ; =================================================================
    lea     rdi, [rel .str_test2]
    call    jit_debug_print_str
    call    jit_debug_newline

    ; Clear jit_table slots so func 1 won't see stale pointers from func 0
    ; (Different func_idx may map to same slot index via func_idx & 3)
    xor     eax, eax
    mov     qword [rel jit_table + 0], rax
    mov     qword [rel jit_table + 8], rax
    mov     qword [rel jit_table + 16], rax
    mov     qword [rel jit_table + 24], rax

    mov     edi, 1
    call    er_wasm_jit_compile
    test    rdx, rdx
    jnz     .fail

    ; Dump compiled code
    mov     rdi, 1
    mov     rsi, rax
    mov     rdx, [rel jit_state.code_ptr]
    sub     rdx, [rel jit_state.cache_base]
    call    jit_debug_dump_header

    mov     rdi, rax                    ; code_ptr
    mov     rsi, [rel jit_state.code_ptr]
    sub     rsi, [rel jit_state.cache_base]
    call    jit_debug_dump_code

    ; Execute func 1 via the explicit JIT executor.
    xor     edx, edx
    xor     esi, esi
    mov     edi, 1
    call    er_wasm_jit_exec

    ; SAVE raw result before any debug output
    mov     qword [rel saved_rax], rax
    mov     qword [rel saved_rdx], rdx

    lea     rdi, [rel .str_raw]
    call    jit_debug_print_str
    mov     rax, qword [rel saved_rax]
    call    jit_debug_print_hex64
    lea     rdi, [rel .str_comma]
    call    jit_debug_print_str
    mov     rax, qword [rel saved_rdx]
    call    jit_debug_print_hex64
    call    jit_debug_newline

    mov     rax, qword [rel saved_rax]
    mov     rdx, qword [rel saved_rdx]
    cmp     rax, 42
    cmovne  rdx, rax
    call    jit_debug_dump_result
    mov     rax, qword [rel saved_rax]
    cmp     rax, 42
    jne     .fail

    ; =================================================================
    ; All tests passed
    ; =================================================================
    lea     rdi, [rel .str_pass]
    call    jit_debug_print_str
    call    jit_debug_newline
    TEST_EXIT 0

.fail:
    lea     rdi, [rel .str_fail]
    call    jit_debug_print_str
    call    jit_debug_newline
    TEST_EXIT 1

.str_test1:  db "=== Test 1: func 0 (block i32.const 42 end) ===", 0
.str_test1b: db "=== Test 1.5: func 0 via er_wasm_jit_exec ===", 0
.str_test1c: db "=== Test 1.6: func 0 via er_wasm_jit_exec (jit_table cleared) ===", 0
.str_test2:  db "=== Test 2: func 1 (call(0) end) ===", 0
.str_raw:    db "raw rax=", 0
.str_comma:  db ", rdx=", 0
.str_pass:  db "PASS", 0
.str_fail:  db "FAIL", 0
