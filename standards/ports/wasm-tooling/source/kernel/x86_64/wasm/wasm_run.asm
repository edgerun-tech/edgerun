; ==================================================================
; er_fn_run: Top-level entry point
; rdi = runtime_ptr (pointer to RuntimeConfig struct)
; rsi = wasm_bytes_ptr
; rdx = wasm_bytes_len
; rcx = export_name_ptr
; r8  = export_name_len
; Returns: rax = result value, rdx = error code
; =================================================================+
er_fn er_fn_run
    er_frame_push_regs rbx, r12, r13, r14, r15
    push    r8              ; save export_name_len

    mov     r12, rdi        ; runtime_ptr
    mov     r13, rsi        ; wasm_bytes
    mov     r14, rdx        ; wasm_len
    mov     r15, rcx        ; export_name

    mov     rdi, r12
    call    _er_wasm_stage_runtime_for_module

    mov     rdi, r13
    mov     rsi, r14
    call    _er_wasm_parse_resolve_validate
    er_check_nonzero rdx, .error

    mov     edi, 1
    call    _er_wasm_prime_loaded_module
    er_check_nonzero rdx, .error

    ; Resolve export to function index
    mov     rdi, r15
    mov     rsi, [rbp - 48]
    call    _er_wasm_resolve_export_function_index
    er_check_nonzero rdx, .error
    mov     r15, rax

    ; Execute exported function
    mov     rdi, r15
    xor     rsi, rsi        ; no args for now
    xor     rdx, rdx
    call    er_fn_exec
    er_check_nonzero rdx, .error

    ; Return result
    mov     rax, [exec_result_values]
    mov     rdx, [exec_result_count]
    jmp     .done

.error:
    mov     rax, -1
    mov     byte [exec_storage_module_valid], 0
    mov     qword [executor_runtime_ptr], 0
.done:
    pop     r8              ; restore export_name_len
    er_pop_ret rbp, rbx, r12, r13, r14, r15

; ==================================================================
; er_fn_run_args: Top-level entry point with WASM value arguments
; rdi = runtime_ptr
; rsi = wasm_bytes_ptr
; rdx = wasm_bytes_len
; rcx = export_name_ptr
; r8  = export_name_len
; r9  = args pointer
; [rbp+16] = args count
; Returns: rax = result value, rdx = error code
; =================================================================+
er_fn er_fn_run_args
    er_frame_push_regs rbx, r12, r13, r14, r15
    push    r8              ; save export_name_len
    push    r9              ; save args ptr

    mov     r12, rdi        ; runtime_ptr
    mov     r13, rsi        ; wasm_bytes
    mov     r14, rdx        ; wasm_len
    mov     r15, rcx        ; export_name

    mov     [rel er_wasm_runtime_ptr], r12
    mov     rdi, r12
    call    _er_wasm_stage_runtime_for_module

    mov     rdi, r13
    mov     rsi, r14
    call    _er_wasm_parse_resolve_validate
    er_check_nonzero rdx, .error

    mov     edi, 1
    call    _er_wasm_prime_loaded_module
    er_check_nonzero rdx, .error

    mov     rdi, r15
    mov     rsi, [rbp - 48]
    call    _er_wasm_resolve_export_function_index
    er_check_nonzero rdx, .error
    mov     r15, rax

    mov     rbx, [rbp + 16] ; args count
    er_check_zero rbx, .exec
    cmp     qword [rbp - 56], 0
    je      .bad_args
.exec:
    mov     rdi, r15
    mov     rsi, [rbp - 56]
    mov     rdx, rbx
    call    er_fn_exec
    er_check_nonzero rdx, .error

    mov     rax, [exec_result_values]
    mov     rdx, [exec_result_count]
    jmp     .done

.bad_args:
    mov     rax, -1
    er_err  ERROR_BAD_ARGUMENT
    jmp     .done
.error:
    mov     rax, -1
    mov     byte [exec_storage_module_valid], 0
    mov     qword [executor_runtime_ptr], 0
.done:
    er_pop_ret rbp, rbx, r12, r13, r14, r15, r8, r9

; ==================================================================
; Initialize the runtime context
; er_fn_init(memory_ptr=rdi, memory_len=rsi, ticks_ptr=rdx)
; =================================================================+
er_fn er_fn_init
    er_frame_push

    mov     [runtime_memory_ptr], rdi
    mov     [runtime_memory_len], rsi
    mov     [runtime_ticks_ptr], rdx
    mov     qword [runtime_imports_ptr], 0
    mov     qword [runtime_imports_len], 0
    mov     byte [runtime_has_initial_pages], 0

    xor     eax, eax
    pop     rbp
    ret

; ==================================================================
; er_fn_load — Parse and prepare a WASM module for repeated execution
; rdi = runtime_ptr, rsi = wasm_bytes_ptr, rdx = wasm_bytes_len
; Returns: rdx = error code (0 = OK)
;
; Like er_fn_run but stops after the start function and data segments.
; Does NOT find or call the export. Leaves the module ready for
; repeated calls via er_fn_call.
; =================================================================+
%ifndef HAVE_ER_WASM_RUNTIME_PTR
extern er_wasm_runtime_ptr
%endif
er_fn er_fn_load
    er_frame_push_regs rbx, r12, r13, r14, r15

    mov     r12, rdi        ; runtime_ptr
    mov     r13, rsi        ; wasm_bytes
    mov     r14, rdx        ; wasm_len

    ; Update runtime pointer for import wrappers
    mov     [rel er_wasm_runtime_ptr], r12

    mov     rdi, r12
    call    _er_wasm_stage_runtime_for_module

    mov     rdi, r13
    mov     rsi, r14
    call    _er_wasm_parse_resolve_validate
    er_check_nonzero rdx, .error

    xor     edi, edi
    call    _er_wasm_prime_loaded_module
    er_check_nonzero rdx, .error

    xor     edx, edx
    jmp     .done
.error:
    mov     rax, -1
    mov     byte [exec_storage_module_valid], 0
    mov     qword [executor_runtime_ptr], 0
.done:
    er_pop_ret rbp, rbx, r12, r13, r14, r15

; ==================================================================
; er_fn_load_trusted — Load a trusted internal WASM module.
; Same ABI as er_fn_load, but skips the no-recursion policy validator.
; Use for built-in modules that the kernel ships, not untrusted payloads.
; =================================================================+
er_fn er_fn_load_trusted
    er_frame_push_regs rbx, r12, r13, r14, r15

    mov     r12, rdi
    mov     r13, rsi
    mov     r14, rdx
    mov     [rel er_wasm_runtime_ptr], r12

    mov     rdi, r12
    call    _er_wasm_stage_runtime_for_module

    mov     rdi, r13
    mov     rsi, r14
    call    _er_wasm_parse_resolve_trusted
    er_check_nonzero rdx, .error

    xor     edi, edi
    call    _er_wasm_prime_loaded_module
    er_check_nonzero rdx, .error

    xor     edx, edx
    jmp     .done
.error:
    mov     rax, -1
    mov     byte [exec_storage_module_valid], 0
    mov     qword [executor_runtime_ptr], 0
.done:
    er_pop_ret rbp, rbx, r12, r13, r14, r15

; ==================================================================
; _er_wasm_validate_launch_pages
; rax = requested initial pages
; Returns: rax unchanged on success, rdx=0
;          rdx=ERROR_NO_MEMORY on policy/backing violation
; ==================================================================
_er_wasm_validate_launch_pages:
    mov     rcx, [memory_min_pages]
    cmp     rax, rcx
    jb      .no_memory

    mov     rcx, [memory_max_pages]
    er_check_zero rcx, .check_backing
    cmp     rax, rcx
    ja      .no_memory

.check_backing:
    mov     rdi, rax
    call    er_wasm_pages_to_bytes
    er_check_nonzero rdx, .no_memory
    cmp     rax, [runtime_memory_len]
    ja      .no_memory

    shr     rax, WASM_PAGE_SHIFT
    er_ok
    ret

.no_memory:
    er_err  ERROR_NO_MEMORY
    ret

; ==================================================================
; _er_wasm_stage_runtime_for_module
; rdi = runtime_ptr
; Initializes runtime globals and clears per-module execution state.
; ==================================================================
_er_wasm_stage_runtime_for_module:
    push    rbx
    mov     rbx, rdi
    mov     [executor_runtime_ptr], rbx

    ; Initialize runtime context memory/ticks
    mov     rdi, [rbx + RUNTIME_MEMORY_PTR_OFF]
    mov     rsi, [rbx + RUNTIME_MEMORY_LEN_OFF]
    mov     rdx, [rbx + RUNTIME_TICKS_PTR_OFF]
    call    er_fn_init

    ; Stage growth hooks and page policy inputs
    mov     rax, [rbx + RUNTIME_MEM_GROW_FN_OFF]
    mov     [runtime_memory_grow_fn], rax
    mov     rax, [rbx + RUNTIME_MEM_GROW_CTX_OFF]
    mov     [runtime_memory_grow_ctx], rax
    mov     rax, [rbx + RUNTIME_TABLE_GROW_FN_OFF]
    mov     [runtime_table_grow_fn], rax
    mov     rax, [rbx + RUNTIME_TABLE_GROW_CTX_OFF]
    mov     [runtime_table_grow_ctx], rax
    mov     rax, [rbx + RUNTIME_INITIAL_PAGES_OFF]
    mov     [runtime_initial_pages], rax
    movzx   eax, byte [rbx + RUNTIME_HAS_PAGES_OFF]
    mov     [runtime_has_initial_pages], al

    ; Stage host imports table
    mov     rax, [rbx + RUNTIME_IMPORTS_PTR_OFF]
    mov     [runtime_imports_ptr], rax
    mov     rax, [rbx + RUNTIME_IMPORTS_LEN_OFF]
    mov     [runtime_imports_len], rax

    ; Reset per-module parser/execution state
    mov     byte [exec_storage_module_valid], 0
    mov     byte [exec_storage_start_ran], 0
    mov     qword [exec_frame_save_ptr], 0
    mov     qword [exec_call_depth], 0
    cld
    lea     rdi, [rel jit_table]
    xor     eax, eax
    mov     ecx, MAX_FUNCTIONS
    rep stosq

    pop     rbx
    ret

; ==================================================================
; _er_wasm_parse_resolve_validate
; rdi = wasm_bytes_ptr, rsi = wasm_bytes_len
; Returns rdx = 0 on success, error otherwise.
; ==================================================================
_er_wasm_parse_resolve_validate:
    call    er_wasm_parse_module
    er_check_nonzero rdx, .done

    call    er_wasm_resolve_imports
    er_check_nonzero rdx, .done

    call    er_wasm_validate_no_mutable_globals
    er_check_nonzero rdx, .done

    call    er_wasm_validate_no_start_section
    er_check_nonzero rdx, .done

    call    er_wasm_validate_no_memory_grow
    er_check_nonzero rdx, .done

    call    er_wasm_validate_no_call_indirect
    er_check_nonzero rdx, .done

    call    er_wasm_validate_no_recursion
    er_check_nonzero rdx, .done

    mov     byte [exec_storage_module_valid], 1
    er_ok
.done:
    ret

; ==================================================================
; _er_wasm_parse_resolve_trusted
; rdi = wasm_bytes_ptr, rsi = wasm_bytes_len
; Returns rdx = 0 on success, error otherwise.
; ==================================================================
_er_wasm_parse_resolve_trusted:
    call    er_wasm_parse_module
    er_check_nonzero rdx, .done

    call    er_wasm_resolve_imports
    er_check_nonzero rdx, .done

    mov     byte [exec_storage_module_valid], 1
    er_ok
.done:
    ret

; ==================================================================
; _er_wasm_prime_loaded_module
; Validates initial pages, runs start once, applies data segments.
; Returns rdx = 0 on success, error otherwise.
; ==================================================================
_er_wasm_prime_loaded_module:
    cmp     byte [runtime_has_initial_pages], 0
    je      .use_module_memory_pages
    mov     rax, [runtime_initial_pages]
    jmp     .set_memory_pages
.use_module_memory_pages:
    mov     rax, [memory_min_pages]
.set_memory_pages:
    call    _er_wasm_validate_launch_pages
    er_check_nonzero rdx, .done
    mov     [executor_memory_pages], rax
    shl     rax, WASM_PAGE_SHIFT
    mov     [executor_memory_limit], rax

    cmp     byte [exec_storage_start_ran], 0
    jne     .apply_data

    cmp     qword [start_function_index], -1
    je      .mark_start_ran

    mov     rdi, [start_function_index]
    xor     rsi, rsi
    xor     rdx, rdx
    call    er_fn_exec
    er_check_nonzero rdx, .done
    cmp     qword [exec_result_count], 0
    je      .mark_start_ran
    er_err  ERROR_CORRUPT
    jmp     .done

.mark_start_ran:
    mov     byte [exec_storage_start_ran], 1

.apply_data:
    mov     rdi, [runtime_memory_ptr]
    mov     rsi, [executor_memory_pages]
    call    er_wasm_apply_data_segments
    er_ok
.done:
    ret

; ==================================================================
; _er_wasm_resolve_export_function_index
; rdi = export_name_ptr, rsi = export_name_len
; Returns: rax=function_index, rdx=0 on success
;          rdx=ERROR_MISSING_EXPORT on lookup/kind mismatch
; ==================================================================
_er_wasm_resolve_export_function_index:
    call    er_wasm_find_export
    er_check_nonzero rdx, .done

    mov     rbx, rax
    imul    rbx, EXPORT_SIZE
    movzx   eax, byte [exports_buf + rbx + 16]
    cmp     al, 0x00
    jne     .not_function
    mov     rax, [exports_buf + rbx + 24]
    er_ok
    ret

.not_function:
    er_err  ERROR_MISSING_EXPORT
.done:
    ret

; ==================================================================
; er_fn_call — Call an exported function of a loaded module
; rdi = runtime_ptr, rsi = export_name_ptr, rdx = export_name_len
; Returns: rax = result value, rdx = error code
;
; The module must have been loaded via er_fn_load first.
; Multiple calls are safe — execution state is reset each time.
; =================================================================+
er_fn er_fn_call
    er_frame_push_regs rbx, r12, r13, r14, r15

    mov     r12, rdi        ; runtime_ptr
    mov     r13, rsi        ; export_name_ptr
    mov     r14, rdx        ; export_name_len

    ; Update runtime pointer for import wrappers
    mov     [rel er_wasm_runtime_ptr], r12

    cmp     byte [exec_storage_module_valid], 1
    jne     .bad_loaded_state
    cmp     r12, [executor_runtime_ptr]
    jne     .bad_loaded_state

    ; Resolve export to function index
    mov     rdi, r13
    mov     rsi, r14
    call    _er_wasm_resolve_export_function_index
    er_check_nonzero rdx, .error
    mov     rbx, rax

    ; Execute the function
    mov     rdi, rbx
    xor     rsi, rsi        ; no args
    xor     rdx, rdx        ; 0 args
    call    er_fn_exec
    er_check_nonzero rdx, .error

    ; Return result
    mov     rax, [exec_result_values]
    xor     edx, edx
    jmp     .done

.error:
    xor     eax, eax
    ; rdx already set by failing call
    jmp     .done
.bad_loaded_state:
    xor     eax, eax
    er_err  ERROR_BAD_ARGUMENT
.done:
    er_pop_ret rbp, rbx, r12, r13, r14, r15

; ==================================================================
; er_fn_call_args — Call an exported function of a loaded module with args
; rdi = runtime_ptr, rsi = export_name_ptr, rdx = export_name_len
; rcx = args pointer, r8 = args count
; Returns: rax = result value, rdx = error code
;
; Args are 64-bit slots, one per WASM value, matching er_fn_exec.
; =================================================================+
er_fn er_fn_call_args
    er_frame_push_regs rbx, r12, r13, r14, r15

    mov     r12, rdi        ; runtime_ptr
    mov     r13, rsi        ; export_name_ptr
    mov     r14, rdx        ; export_name_len
    mov     r15, rcx        ; args ptr
    mov     rbx, r8         ; args count

    ; Update runtime pointer for import wrappers
    mov     [rel er_wasm_runtime_ptr], r12

    cmp     byte [exec_storage_module_valid], 1
    jne     .bad_loaded_state
    cmp     r12, [executor_runtime_ptr]
    jne     .bad_loaded_state
    er_check_zero rbx, .resolve
    er_check_zero r15, .bad_loaded_state

.resolve:
    mov     rdi, r13
    mov     rsi, r14
    call    _er_wasm_resolve_export_function_index
    er_check_nonzero rdx, .error

    mov     rdi, rax
    mov     rsi, r15
    mov     rdx, rbx
    call    er_fn_exec
    er_check_nonzero rdx, .error

    mov     rax, [exec_result_values]
    xor     edx, edx
    jmp     .done

.error:
    xor     eax, eax
    jmp     .done
.bad_loaded_state:
    xor     eax, eax
    er_err  ERROR_BAD_ARGUMENT
.done:
    er_pop_ret rbp, rbx, r12, r13, r14, r15

; ==================================================================
; Resolve all imports against host-provided import table
; For each import in imports_buf, scans runtime_imports for a match
; by module name and function name.
; Sets resolved_func_index (offset 48) to the host import index.
; Returns: rdx = 0 on success, ERROR_MISSING_IMPORT on failure
; =================================================================+
er_wasm_resolve_imports:
    er_frame_push_regs rbx, r12, r13, r14, r15

    xor     r12d, r12d
    mov     r13, [import_count]
    mov     r14, [runtime_imports_len]

    er_check_zero r13, .done_ok

    cmp     qword [runtime_imports_ptr], 0
    je      .not_found_all

    er_check_zero r14, .not_found_all

.loop:
    cmp     r12, r13
    jae     .done_ok

    mov     rbx, r12
    imul    rbx, IMPORTED_FUNC_SIZE

    xor     r15d, r15d
.scan:
    cmp     r15, r14
    jae     .not_found

    ; Compare module name
    ; er_wasm_eql(rdi=ptr1, rsi=len1, rdx=ptr2, rcx=len2)
    mov     rax, r15
    imul    rax, HOST_IMPORT_SIZE
    add     rax, [runtime_imports_ptr]
    mov     rdi, [rax + HOST_IMPORT_MODULE_PTR_OFF]
    mov     rsi, [rax + HOST_IMPORT_MODULE_LEN_OFF]
    mov     rdx, [imports_buf + rbx + IMPORT_MODULE_NAME_PTR_OFF]
    mov     rcx, [imports_buf + rbx + IMPORT_MODULE_NAME_LEN_OFF]
    call    er_wasm_eql
    er_check_zero eax, .scan_next

    ; Module name matches — compare function name
    mov     rax, r15
    imul    rax, HOST_IMPORT_SIZE
    add     rax, [runtime_imports_ptr]
    mov     rdi, [rax + HOST_IMPORT_NAME_PTR_OFF]
    mov     rsi, [rax + HOST_IMPORT_NAME_LEN_OFF]
    mov     rdx, [imports_buf + rbx + IMPORT_FUNC_NAME_PTR_OFF]
    mov     rcx, [imports_buf + rbx + IMPORT_FUNC_NAME_LEN_OFF]
    call    er_wasm_eql
    er_check_zero eax, .scan_next

    ; Both names match — store resolved index
    mov     [imports_buf + rbx + IMPORT_RESOLVED_FUNC_IDX_OFF], r15
    inc     r12
    jmp     .loop

.scan_next:
    inc     r15
    jmp     .scan

.not_found:
.not_found_all:
    er_err  ERROR_MISSING_IMPORT
    jmp     .done

.done_ok:
    xor     edx, edx
.done:
    er_pop_ret rbp, rbx, r12, r13, r14, r15

; ==================================================================
; Validate that app modules do not contain mutable globals.
; Durable app state must be explicit transition input/output, not hidden
; module-global mutation across calls.
; Returns: rdx = 0 on success, ERROR_UNSUPPORTED when a mutable global exists
; =================================================================+
er_wasm_validate_no_mutable_globals:
    er_frame_push_regs rbx, r12

    mov     r12, [global_count]
    xor     ebx, ebx
.loop:
    cmp     rbx, r12
    jae     .ok

    mov     rax, rbx
    imul    rax, GLOBAL_SIZE
    cmp     byte [globals_buf + rax + 1], 0
    jne     .found

    inc     rbx
    jmp     .loop

.found:
    er_err  ERROR_UNSUPPORTED
    jmp     .done

.ok:
    er_ok
.done:
    er_pop_ret rbp, rbx, r12

; ==================================================================
; Validate that app modules do not contain a start section.
; App lifecycle must enter through explicit ABI functions, not load-time code.
; Returns: rdx = 0 on success, ERROR_UNSUPPORTED when a start section is present
; =================================================================+
er_wasm_validate_no_start_section:
    cmp     qword [start_function_index], -1
    jne     .found
    er_ok
    ret
.found:
    er_err  ERROR_UNSUPPORTED
    ret

; ==================================================================
; Validate that app modules do not contain memory.grow.
; App memory is a launch/runtime grant owned by the user and kernel, not a
; mutable capability available to app bytecode.
; Returns: rdx = 0 on success, ERROR_MEMORY_GROWTH when opcode 0x40 is present
; =================================================================+
er_wasm_validate_no_memory_grow:
    er_frame_push_regs rbx, r12, r13, r14, r15

    mov     r15, [code_count]
    xor     r12d, r12d              ; code index
.code_loop:
    cmp     r12, r15
    jae     .ok

    imul    r10, r12, CODE_SIZE
    mov     r13, [code_buf + r10 + 24]  ; decoded_start
    mov     r14, [code_buf + r10 + 32]  ; decoded_count
    xor     ebx, ebx

.op_loop:
    cmp     rbx, r14
    jae     .next_code

    mov     r8, r13
    add     r8, rbx
    imul    r8, DECODED_OP_SIZE
    cmp     byte [decoded_ops + r8 + 8], 0x40
    je      .memory_grow_found

    inc     rbx
    jmp     .op_loop

.next_code:
    inc     r12
    jmp     .code_loop

.memory_grow_found:
    er_err  ERROR_MEMORY_GROWTH
    jmp     .done

.ok:
    er_ok
.done:
    er_pop_ret rbp, rbx, r12, r13, r14, r15

; ==================================================================
; Validate that app modules do not contain call_indirect.
; Indirect calls hide the call graph from the current no-recursion validator.
; Returns: rdx = 0 on success, ERROR_UNSUPPORTED when opcode 0x11 is present
; =================================================================+
er_wasm_validate_no_call_indirect:
    er_frame_push_regs rbx, r12, r13, r14, r15

    mov     r15, [code_count]
    xor     r12d, r12d              ; code index
.code_loop:
    cmp     r12, r15
    jae     .ok

    imul    r10, r12, CODE_SIZE
    mov     r13, [code_buf + r10 + 24]  ; decoded_start
    mov     r14, [code_buf + r10 + 32]  ; decoded_count
    xor     ebx, ebx

.op_loop:
    cmp     rbx, r14
    jae     .next_code

    mov     r8, r13
    add     r8, rbx
    imul    r8, DECODED_OP_SIZE
    cmp     byte [decoded_ops + r8 + 8], 0x11
    je      .call_indirect_found

    inc     rbx
    jmp     .op_loop

.next_code:
    inc     r12
    jmp     .code_loop

.call_indirect_found:
    er_err  ERROR_UNSUPPORTED
    jmp     .done

.ok:
    er_ok
.done:
    er_pop_ret rbp, rbx, r12, r13, r14, r15

; ==================================================================
; Validate that the call graph has no cycles (recursion is banned).
; Scans decoded ops for direct call(0x10) instructions and runs a
; DFS cycle-detection pass over all functions.
; Returns: rdx = 0 on success, ERROR_RECURSION on cycle
; =================================================================+
er_wasm_validate_no_recursion:
    push    rbp
    mov     rbp, rsp
    er_push rbx, r12, r13, r14, r15

    ; Stack-allocate visited[0..1023] + in_progress[1024..2047]
    sub     rsp, MAX_FUNCTIONS * 2

    ; Zero both arrays (2*1024/8 = 256 qwords)
    lea     rdi, [rsp]
    xor     eax, eax
    mov     ecx, MAX_FUNCTIONS * 2 / 8
    rep stosq

    mov     r15, [function_count]

    xor     r12d, r12d              ; f = 0
.check_loop:
    cmp     r12, r15
    jae     .check_done

    cmp     byte [rsp + r12], 0     ; visited[f]?
    jne     .check_next

    ; DFS from function f
    mov     rdi, r12
    lea     rsi, [rsp]              ; visited
    lea     rdx, [rsp + MAX_FUNCTIONS]  ; in_progress
    mov     rcx, r15                ; function_count
    call    .dfs_cycle
    er_check_nonzero rdx, .check_error

.check_next:
    inc     r12
    jmp     .check_loop

.check_done:
    add     rsp, MAX_FUNCTIONS * 2
    xor     edx, edx
    jmp     .check_out

.check_error:
    add     rsp, MAX_FUNCTIONS * 2
    ; rdx already set

.check_out:
    er_pop_ret rbp, rbx, r12, r13, r14, r15

; ── DFS cycle detection ──
; rdi = function index
; rsi = visited byte array
; rdx = in_progress byte array
; rcx = function_count
; Returns rdx = 0 on success, ERROR_RECURSION on cycle
.dfs_cycle:
    ; Mark in-progress
    mov     byte [rdx + rdi], 1
    mov     byte [rsi + rdi], 1

    er_push r12, r13, r14, r15
    push    rcx                 ; save function_count

    mov     r12, rdi            ; current function
    mov     r13, rsi            ; visited
    mov     r14, rdx            ; in_progress

    ; Get code info
    push    rdi
    call    er_wasm_code_index_for_function
    er_check_nonzero rdx, .dfs_ok
    pop     rdi

    imul    r10, rax, CODE_SIZE
    mov     r11, [code_buf + r10 + 24]  ; decoded_start
    mov     r15, [code_buf + r10 + 32]  ; decoded_count

    ; Iterate decoded ops
    xor     ecx, ecx
.dfs_op_loop:
    cmp     ecx, r15d
    jae     .dfs_op_done

    mov     r8, r11
    add     r8, rcx
    imul    r8, DECODED_OP_SIZE

    movzx   eax, byte [decoded_ops + r8 + 8]  ; opcode
    cmp     al, 0x10                            ; call
    jne     .dfs_op_next

    mov     r9, [decoded_ops + r8 + 12]         ; target func index
    mov     rax, [rsp]                          ; function_count
    cmp     r9, rax
    jae     .dfs_op_next                        ; skip invalid (caught elsewhere)

    ; Check for cycle
    cmp     byte [r14 + r9], 0
    jne     .dfs_cycle_found

    ; If not visited, recurse
    cmp     byte [r13 + r9], 0
    jne     .dfs_op_next

    mov     rdi, r9
    mov     rsi, r13
    mov     rdx, r14
    mov     rcx, [rsp]          ; function_count (on stack above our pushes)
    call    .dfs_cycle
    er_check_nonzero rdx, .dfs_error

.dfs_op_next:
    inc     ecx
    jmp     .dfs_op_loop

.dfs_op_done:
    xor     edx, edx
    jmp     .dfs_done

.dfs_cycle_found:
    mov     edx, ERROR_RECURSION
    jmp     .dfs_done

.dfs_error:
    ; rdx already set
.dfs_done:
    mov     byte [r14 + r12], 0  ; clear in_progress
    add     rsp, 8               ; discard saved function_count from stack
    er_pop_ret r12, r13, r14, r15

.dfs_ok:
    pop     rdi
    xor     edx, edx
    jmp     .dfs_done
