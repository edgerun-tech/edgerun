; EdgeRun freestanding WASM interpreter — x86_64 assembly
; System V AMD64 ABI: arg1=rdi, arg2=rsi, arg3=rdx, arg4=rcx, arg5=r8, arg6=r9, retval=rax
; Canonical EdgeRun WASM runtime implementation.
; No libc, no external dependencies.

%include "x86_64/macros.inc"

default rel

; ------------------------------------------------------------------
; Opcode handler pattern macros (eliminate repeated boilerplate)
; Stack ops are inlined to avoid 3 function calls per binary op (~50 cycles saved)
; ------------------------------------------------------------------
; Binary op: pop right, pop left, compute, push
%macro _wasm_binop 2
    mov     r8, [exec_stack_len]
    cmp     r8, 2
    jb      .underflow_error
    sub     r8, 2
    mov     %1cx, [exec_stack + r8 * 8 + 8]
    mov     %1ax, [exec_stack + r8 * 8]
    %2      %1ax, %1cx
    mov     [exec_stack + r8 * 8], %1ax
    inc     r8
    mov     [exec_stack_len], r8
    jmp     .dispatch_next
%endm

; Shift/rotate op: pop shift, pop value, shift using cl, push
%macro _wasm_shiftop 2
    mov     r8, [exec_stack_len]
    cmp     r8, 2
    jb      .underflow_error
    sub     r8, 2
    mov     %1cx, [exec_stack + r8 * 8 + 8]
    mov     %1ax, [exec_stack + r8 * 8]
    %2      %1ax, cl
    mov     [exec_stack + r8 * 8], %1ax
    inc     r8
    mov     [exec_stack_len], r8
    jmp     .dispatch_next
%endm

; Compare op: pop right, pop left, cmp, setCC, push
%macro _wasm_cmpop 2
    mov     r8, [exec_stack_len]
    cmp     r8, 2
    jb      .underflow_error
    sub     r8, 2
    mov     %1cx, [exec_stack + r8 * 8 + 8]
    mov     %1ax, [exec_stack + r8 * 8]
    cmp     %1ax, %1cx
    %2      al
    movzx   eax, al
    mov     [exec_stack + r8 * 8], rax
    inc     r8
    mov     [exec_stack_len], r8
    jmp     .dispatch_next
%endm

; Unary op: pop value, compute, push
%macro _wasm_unop 2
    mov     r8, [exec_stack_len]
    test    r8, r8
    jz      .underflow_error
    dec     r8
    mov     %1ax, [exec_stack + r8 * 8]
    %2      %1ax, %1ax
    mov     [exec_stack + r8 * 8], %1ax
    jmp     .dispatch_next
%endm

; Zero-test op: pop value, test+setz, push
%macro _wasm_eqz 1
    mov     r8, [exec_stack_len]
    test    r8, r8
    jz      .underflow_error
    dec     r8
    mov     %1ax, [exec_stack + r8 * 8]
    test    %1ax, %1ax
    setz    al
    movzx   eax, al
    mov     [exec_stack + r8 * 8], rax
    jmp     .dispatch_next
%endm

; Float binary ops (SSE scalar)
; f32: pop right (low32), pop left (low32), compute, push result
%macro _wasm_f32_binop 1
    mov     r8, [exec_stack_len]
    cmp     r8, 2
    jb      .underflow_error
    sub     r8, 2
    movd    xmm1, [exec_stack + r8 * 8 + 8]
    movd    xmm0, [exec_stack + r8 * 8]
    %1      xmm0, xmm1
    movd    eax, xmm0
    mov     [exec_stack + r8 * 8], rax
    inc     r8
    mov     [exec_stack_len], r8
    jmp     .dispatch_next
%endm

; f64: pop right, pop left, compute, push result
%macro _wasm_f64_binop 1
    mov     r8, [exec_stack_len]
    cmp     r8, 2
    jb      .underflow_error
    sub     r8, 2
    movq    xmm1, [exec_stack + r8 * 8 + 8]
    movq    xmm0, [exec_stack + r8 * 8]
    %1      xmm0, xmm1
    movq    rax, xmm0
    mov     [exec_stack + r8 * 8], rax
    inc     r8
    mov     [exec_stack_len], r8
    jmp     .dispatch_next
%endm

; Float unary round ops: pop, compute via roundss/roundsd, push
; arg2 = rounding mode immediate (0=nearest, 1=floor, 2=ceil, 3=trunc)
%macro _wasm_f32_unop_round 2
    mov     r8, [exec_stack_len]
    test    r8, r8
    jz      .underflow_error
    dec     r8
    movd    xmm0, [exec_stack + r8 * 8]
    %1      xmm0, xmm0, %2
    movd    eax, xmm0
    mov     [exec_stack + r8 * 8], rax
    jmp     .dispatch_next
%endm

%macro _wasm_f64_unop_round 2
    mov     r8, [exec_stack_len]
    test    r8, r8
    jz      .underflow_error
    dec     r8
    movq    xmm0, [exec_stack + r8 * 8]
    %1      xmm0, xmm0, %2
    movq    rax, xmm0
    mov     [exec_stack + r8 * 8], rax
    jmp     .dispatch_next
%endm

; Float sqrt unary ops
%macro _wasm_f32_unop_sqrt 0
    mov     r8, [exec_stack_len]
    test    r8, r8
    jz      .underflow_error
    dec     r8
    movd    xmm0, [exec_stack + r8 * 8]
    sqrtss  xmm0, xmm0
    movd    eax, xmm0
    mov     [exec_stack + r8 * 8], rax
    jmp     .dispatch_next
%endm

%macro _wasm_f64_unop_sqrt 0
    mov     r8, [exec_stack_len]
    test    r8, r8
    jz      .underflow_error
    dec     r8
    movq    xmm0, [exec_stack + r8 * 8]
    sqrtsd  xmm0, xmm0
    movq    rax, xmm0
    mov     [exec_stack + r8 * 8], rax
    jmp     .dispatch_next
%endm

; Float comparison: returns 0 for NaN (eq, lt, gt, le, ge)
; arg = setCC instruction suffix (e.g., e, b, a, be, ae)
%macro _wasm_f32_cmp 1
    mov     r8, [exec_stack_len]
    cmp     r8, 2
    jb      .underflow_error
    sub     r8, 2
    movd    xmm1, [exec_stack + r8 * 8 + 8]
    movd    xmm0, [exec_stack + r8 * 8]
    ucomiss xmm0, xmm1
    setnp   cl
    set%1   al
    and     al, cl
    movzx   eax, al
    mov     [exec_stack + r8 * 8], rax
    inc     r8
    mov     [exec_stack_len], r8
    jmp     .dispatch_next
%endm

%macro _wasm_f64_cmp 1
    mov     r8, [exec_stack_len]
    cmp     r8, 2
    jb      .underflow_error
    sub     r8, 2
    movq    xmm1, [exec_stack + r8 * 8 + 8]
    movq    xmm0, [exec_stack + r8 * 8]
    ucomisd xmm0, xmm1
    setnp   cl
    set%1   al
    and     al, cl
    movzx   eax, al
    mov     [exec_stack + r8 * 8], rax
    inc     r8
    mov     [exec_stack_len], r8
    jmp     .dispatch_next
%endm

; Float ne comparison: returns 1 for NaN
%macro _wasm_f32_cmp_ne 0
    mov     r8, [exec_stack_len]
    cmp     r8, 2
    jb      .underflow_error
    sub     r8, 2
    movd    xmm1, [exec_stack + r8 * 8 + 8]
    movd    xmm0, [exec_stack + r8 * 8]
    ucomiss xmm0, xmm1
    setp    cl      ; 1 if NaN
    setne   al      ; 1 if not equal (ordered)
    or      al, cl  ; 1 if NaN or not equal
    movzx   eax, al
    mov     [exec_stack + r8 * 8], rax
    inc     r8
    mov     [exec_stack_len], r8
    jmp     .dispatch_next
%endm

%macro _wasm_f64_cmp_ne 0
    mov     r8, [exec_stack_len]
    cmp     r8, 2
    jb      .underflow_error
    sub     r8, 2
    movq    xmm1, [exec_stack + r8 * 8 + 8]
    movq    xmm0, [exec_stack + r8 * 8]
    ucomisd xmm0, xmm1
    setp    cl
    setne   al
    or      al, cl
    movzx   eax, al
    mov     [exec_stack + r8 * 8], rax
    inc     r8
    mov     [exec_stack_len], r8
    jmp     .dispatch_next
%endm

%include "x86_64/wasm_defines.inc"

; ==================================================================
; BSS — all interpreter state
; ==================================================================
SECTION .bss

; -- Value is 16 bytes: 8 bytes data, 4 bytes type tag, 4 bytes padding
; VALUE_size equ 16

; -- FuncType: params[128] + results[4] + param_count + result_count
; Layout: 128 bytes params + 4 bytes results + 8 bytes param_count + 8 bytes result_count = 148
; Padded to 256 for power-of-2 addressing
FUNC_TYPE_SIZE equ 256
FUNC_TYPE_PARAM_COUNT_OFF equ 132  ; offset of param_count field
FUNC_TYPE_RESULT_COUNT_OFF equ 140 ; offset of result_count field

types_buf:      resb MAX_TYPES * FUNC_TYPE_SIZE       ; 128 * 148 = 18944
type_count:     resq 1

; -- ImportedFunction layout (64 bytes):
;   +0: module_name_ptr(8) — pointer into names_buf
;   +8: module_name_len(8)
;  +16: func_name_ptr(8)   — pointer into names_buf
;  +24: func_name_len(8)
;  +32: type_index(8)
;  +40: resolved_module_id(8) — reserved; host import resolution uses +48
;  +48: resolved_func_index(8) — function index in the resolved module
;  +56: (unused padding)
IMPORTED_FUNC_SIZE equ 64
%define IMPORT_MODULE_NAME_PTR_OFF   0
%define IMPORT_MODULE_NAME_LEN_OFF   8
%define IMPORT_FUNC_NAME_PTR_OFF     16
%define IMPORT_FUNC_NAME_LEN_OFF     24
%define IMPORT_TYPE_INDEX_OFF        32
%define IMPORT_RESOLVED_MOD_ID_OFF   40
%define IMPORT_RESOLVED_FUNC_IDX_OFF 48
imports_buf:    resb MAX_IMPORTS * IMPORTED_FUNC_SIZE
import_count:   resq 1

; -- ImportedMemory: module fields + min_pages(8) + max_pages(8) + present flag(1) = 33
imported_memory_present: resb 1
imported_memory_min:     resq 1
imported_memory_max:     resq 1

; -- ImportedTable: module fields + min_entries(8) + max_entries(8) + present flag(1)
imported_table_present:  resb 1
imported_table_min:      resq 1
imported_table_max:      resq 1

; -- ImportedGlobal: module fields + global_index(8)
IMPORTED_GLOBAL_SIZE equ 24
imported_globals_buf:   resb MAX_GLOBALS * IMPORTED_GLOBAL_SIZE
imported_global_count:  resq 1

; -- HostImport struct layout is defined in wasm_defines.inc.
host_imports_buf:   resb MAX_HOST_IMPORTS * HOST_IMPORT_SIZE
host_import_count:  resq 1

; -- Function: type_index(8) + code_index(8) = 16
FUNCTION_SIZE equ 16
functions_buf:   resb MAX_FUNCTIONS * FUNCTION_SIZE
function_count:  resq 1

; -- Code: body_offset(8) + body_len(8) + local_count(8) + decoded_start(8) + decoded_count(8) = 40
; Padded to 64 for power-of-2 addressing
CODE_SIZE equ 64
code_buf:        resb MAX_FUNCTIONS * CODE_SIZE
code_count:      resq 1

; Local types for each function (stored contiguously)
; Each function has up to MAX_LOCALS types. Store as byte array.
; Index into this using: local_type_offset[func_idx] + local_idx
; We'll compute offsets on the fly from the Code struct
code_local_types: resb MAX_FUNCTIONS * MAX_LOCALS  ; 1024 * 1024 = 1MB (large!)

; -- Export: name_offset(8) + name_len(8) + kind(1) + padding(7) + index(8) = 32
EXPORT_SIZE equ 32
exports_buf:     resb MAX_FUNCTIONS * EXPORT_SIZE
export_count:    resq 1

; -- Global: value_type(1) + mutable(1) + padding(6) + value_data(8) + value_tag(4) + padding(4) = 24
; Padded to 32 for power-of-2 addressing
GLOBAL_SIZE equ 32
globals_buf:     resb MAX_GLOBALS * GLOBAL_SIZE
global_count:    resq 1

; -- Table entries (funcref array)
table_entries:    resq MAX_TABLE_ENTRIES  ; each is usize (8 bytes)
table_min:        resq 1
table_max:        resq 1
table_has:        resb 1

; -- Element segments: entries[32] * 8 + count(8) + passive(1) + dropped(1) = 266 per segment
; Padded to 512 for power-of-2 addressing
ELEMENT_SEGMENT_SIZE equ 512
element_segments: resb MAX_DATA_SEGMENTS * ELEMENT_SEGMENT_SIZE
element_segment_count: resq 1

; -- Data segments: offset(8) + byte_offset(8) + byte_len(8) + active(1) + dropped(1) = 26
;   bytes themselves stored in a workspace buffer
DATA_SEGMENT_SIZE equ 32
data_segments:    resb MAX_DATA_SEGMENTS * DATA_SEGMENT_SIZE
data_segment_count: resq 1
declared_data_count: resq 1  ; -1 = not declared

; -- Decoded op cache (global, reused across parses)
; DecodedOp: offset(4) + next_offset(4) + opcode_byte(1) + padding(3) + imm0(4) + imm1(4) = 20
; Padded to 32 for power-of-2 addressing
DECODED_OP_SIZE equ 32
decoded_ops:     resb MAX_DECODED_OPS * DECODED_OP_SIZE  ; ~10MB!
decoded_op_count: resq 1

; -- Runtime context (caller-provided)
runtime_memory_ptr:    resq 1
runtime_memory_len:    resq 1
runtime_ticks_ptr:     resq 1
runtime_imports_ptr:   resq 1
runtime_imports_len:   resq 1
runtime_memory_grow_fn:   resq 1
runtime_memory_grow_ctx:  resq 1
runtime_table_grow_fn:    resq 1
runtime_table_grow_ctx:   resq 1
runtime_initial_pages:    resq 1
runtime_has_initial_pages: resb 1

; -- Executor state (set up before execution)
executor_runtime_ptr:  resq 1
executor_module_ptr:   resq 1
executor_memory_pages: resq 1
executor_memory_limit: resq 1

MAX_MODULES equ 4

; -- Names buffer (for import/export name strings)
; Simple bump allocator — caller writes names here before parsing
names_buf:       resb 16384
names_offset:    resq 1

; -- Workspace for string comparison, temp storage
scratch_buf:     resb 256

; -- Execution storage (cached module + executor for repeated calls)
; Module parse cache
exec_storage_module_valid: resb 1
exec_storage_start_ran:    resb 1
opcode_table_populated:    resb 1  ; one-time flag for opcode dispatch table

; -- String compare temporary buffer for import/export name matching
strcmp_buf:      resb 256

; -- Execution engine state
; Current frame locals (each is 8 bytes, up to MAX_LOCALS)
exec_local_count: resq 1
exec_locals:    resb MAX_LOCALS * 8

; Current frame value stack (each entry is 8 bytes)
exec_stack_len: resq 1
exec_stack:     resb MAX_STACK * 8

; Control frame stack
; Each ControlFrame: kind(8) + next_offset(8) = 16 bytes
CONTROL_FRAME_SIZE equ 16
exec_control_len: resq 1
exec_control:   resb MAX_CONTROL_DEPTH * CONTROL_FRAME_SIZE

; Decoded op tracking
exec_decoded_index: resq 1
exec_decoded_end:   resq 1
exec_fast_cache_start: resq 1
exec_fast_cache_end:   resq 1
exec_fast_cache_state: resq 1  ; 0 unknown, 1 fast, 2 unsupported, 3 trace

; Frame save area (for WASM function calls)
; Stores locals + stack + control frames as a flat stack
; Layout per saved frame:
;   [locals data (local_count * 8 bytes)]
;   [stack data (stack_len * 8 bytes)]
;   [control data (control_len * 16 bytes)]
;   [metadata: local_count, stack_len, control_len, dec_idx, dec_end,
;              type_idx, body_ptr, body_len, reader_offset, result_count = 80 bytes]
FRAME_SAVE_SIZE equ 1024 * 1024  ; 1MB save area (enough for deep recursion with moderate locals)
exec_frame_save:  resb FRAME_SAVE_SIZE
exec_frame_save_ptr: resq 1  ; offset into save area (grows upward)
exec_call_depth: resq 1

; Current function info for dispatch
exec_function_type_ptr: resq 1  ; pointer to current FuncType
exec_type_index:       resq 1  ; type index of current function
exec_code_body_ptr:    resq 1  ; pointer to current code body
exec_code_body_len:    resq 1  ; remaining body length
exec_reader_offset:    resq 1  ; current reader offset within body

; Return value from function execution (as i64, + count in exec_result_count)
exec_result_values: resb MAX_TYPE_RESULTS * 8  ; up to 4 result values
exec_result_count:  resq 1

; ==================================================================
; .data section — lookup tables
; ==================================================================
SECTION .data

; Opcode dispatch table: 256 entries, each 8 bytes (function pointer)
; Uses a two-level approach: hot opcodes handled inline, cold ones via table
; For now we use a simple jump table for the main dispatch
opcode_table:
; Uninitialized — will be populated by init code
times 256 dq 0  ; will be filled at runtime with dispatch addresses

; Section order table (for validation)
section_order:
db 0   ; 0 (custom)
db 1   ; 1 type
db 2   ; 2 import
db 3   ; 3 function
db 4   ; 4 table
db 5   ; 5 memory
db 6   ; 6 global
db 7   ; 7 export
db 8   ; 8 start
db 9   ; 9 element
db 11  ; 10 code
db 12  ; 11 data
db 10  ; 12 data_count

; Float constants for saturating truncation
align 8
f32_sat_bound_0:    dd 0x00000000  ; 0.0f (positive zero)
f32_sat_bound_1:    dd 0x3f800000  ; 1.0f
f32_sat_2pow31:     dd 0x4F000000  ; 2147483648.0f = 2^31
f32_sat_minus_2pow31: dd 0xCF000000  ; -2147483648.0f
f32_sat_2pow32:     dd 0x4F800000  ; 4294967296.0f = 2^32
f32_sat_minus_1:    dd 0xBF800000  ; -1.0f
f32_sat_2pow63:     dd 0x5F000000  ; 9223372036854775808.0f = 2^63
f32_sat_minus_2pow63: dd 0xDF000000  ; -9223372036854775808.0f
f32_sat_2pow64:     dd 0x5F800000  ; 18446744073709551616.0f = 2^64
f64_sat_2pow63:     dq 0x43E0000000000000  ; 9223372036854775808.0 = 2^63
f64_sat_minus_2pow63: dq 0xC3E0000000000000  ; -9223372036854775808.0
f64_sat_2pow64:     dq 0x43F0000000000000  ; 18446744073709551616.0 = 2^64
f64_2pow31:         dq 0x41E0000000000000  ; 2147483648.0 = 2^31
f64_minus_2pow31:   dq 0xC1E0000000000000  ; -2147483648.0
f64_2pow32:         dq 0x41F0000000000000  ; 4294967296.0 = 2^32

SECTION .bss
frame_locals:    resb MAX_LOCALS * 16  ; 1024 Values * 16 bytes each = 16KB
frame_stack:     resb MAX_STACK * 16   ; 32 Values * 16 bytes = 512 bytes
frame_stack_len: resq 1
