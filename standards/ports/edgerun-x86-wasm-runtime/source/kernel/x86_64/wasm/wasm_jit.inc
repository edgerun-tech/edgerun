%ifndef WASM_JIT_INC
%define WASM_JIT_INC

; ==================================================================
; JIT compiler constants — shared by all JIT sub-modules
; ==================================================================

; Code cache
JIT_CACHE_SIZE      equ 262144   ; 256KB total cache
JIT_SLOT_SIZE       equ 65536    ; 64KB per function slot
JIT_MAX_COMPILED    equ 4        ; max concurrently compiled functions
JIT_CACHE_MAGIC     equ 0x4A494321 ; "JIT!"

; Register allocation constants
JIT_NUM_VALUE_REGS  equ 8        ; rax, rcx, rdx, rbx, r8-r11
JIT_SPILL_NONE      equ -1       ; sentinel: value not in register

; Register IDs for allocation tracker
JIT_REG_RAX equ 0
JIT_REG_RCX equ 1
JIT_REG_RDX equ 2
JIT_REG_RBX equ 3
JIT_REG_R8  equ 4
JIT_REG_R9  equ 5
JIT_REG_R10 equ 6
JIT_REG_R11 equ 7

; Label types for branch fixup
JIT_LABEL_BLOCK  equ 0
JIT_LABEL_LOOP   equ 1
JIT_LABEL_IF     equ 2
JIT_LABEL_ELSE   equ 3

; Max nesting depth for labels (matches MAX_CONTROL_DEPTH=256)
JIT_MAX_LABELS   equ 256

; ==================================================================
; JitGlobals — runtime state accessible to JIT'd code via r15
; ==================================================================
struc JitGlobals
    .locals:           resq 1  ; 0  — exec_locals base addr
    .mem_ptr:          resq 1  ; 8  — runtime_memory_ptr value
    .mem_len:          resq 1  ; 16 — runtime_memory_len value
    .mem_limit:        resq 1  ; 24 — executor_memory_limit value
    .types_buf:        resq 1  ; 32 — types_buf base addr
    .code_buf:         resq 1  ; 40 — code_buf base addr
    .imports_buf:      resq 1  ; 48 — imports_buf base addr
    .import_count:     resq 1  ; 56 — import_count value
    .globals_buf:      resq 1  ; 64 — globals_buf base addr
    .global_count:     resq 1  ; 72 — global_count value
    .result_values:    resq 1  ; 80 — exec_result_values addr
    .result_count_ptr: resq 1  ; 88 — &exec_result_count
    .table_entries:    resq 1  ; 96 — table_entries addr
    .table_min:        resq 1  ; 104 — table_min value
    .local_count_ptr:  resq 1  ; 112 — &exec_local_count
    .functions_buf:    resq 1  ; 128 — functions_buf addr
    .function_count:   resq 1  ; 136 — function_count value
    .mem_grow_hook:    resq 1  ; 144 — host memory grow callback or 0
    .mem_grow_ctx:     resq 1  ; 152 — context for grow hook
    .memory_pages:     resq 1  ; 160 — executor_memory_pages value
endstruc

; ==================================================================
; JIT compile-time state (per function compilation)
; ==================================================================
struc JitState
    .code_ptr:         resq 1  ; current write position in code cache
    .cache_base:       resq 1  ; base of current function's cache slot
    .cache_end:        resq 1  ; end of current function's cache slot
    .func_idx:         resq 1  ; function being compiled
    .stack_depth:      resq 1  ; current WASM stack depth
    .max_stack:        resq 1  ; max WASM stack depth seen
    .reg_map:          resb JIT_NUM_VALUE_REGS ; which WASM slot each reg holds (-1 = free)
    .slot_reg:         resb 8  ; which reg holds each WASM slot (JIT_SPILL_NONE = spilled)
    .label_depth:      resq 1  ; current label nesting depth
    .label_offsets:    resq JIT_MAX_LABELS ; code address for each label (loop start, block end, etc.)
    .label_kinds:      resb JIT_MAX_LABELS ; kind of each label (BLOCK/LOOP/IF)
    .label_if_jz:      resq JIT_MAX_LABELS ; for IF labels, address of jz displacement to patch at else or end
    .fixup_count:      resq 1  ; number of pending forward fixups
    .fixup_label:      resq 256 ; label depth for each fixup
    .fixup_offset:     resq 256 ; code address where displacement field needs patching
    .return_emitted:   resq 1  ; flag: function epilogue already emitted
endstruc
; JitState is ~4KB — allocated in BSS for one at a time

; ==================================================================
; JIT public API function declarations
; ==================================================================
; er_wasm_jit_compile(func_idx=rdi) → rax=code_ptr, rdx=0 on success
; er_wasm_jit_exec(func_idx=rdi, args=rsi, args_count=rdx) → rax=result, rdx=error
; er_wasm_jit_init() — one-time init of globals and cache

%endif
