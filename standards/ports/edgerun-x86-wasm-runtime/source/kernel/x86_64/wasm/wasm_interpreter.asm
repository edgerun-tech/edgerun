; EdgeRun freestanding WASM interpreter — x86_64 assembly
; System V AMD64 ABI: arg1=rdi, arg2=rsi, arg3=rdx, arg4=rcx, arg5=r8, arg6=r9, retval=rax
; Canonical EdgeRun WASM runtime implementation.
; No libc, no external dependencies.
;
; This file is the umbrella that includes all sub-modules:
;   wasm_constants.inc — %defines, BSS, and data sections
;   wasm_decode.asm    — LEB128 readers, module parsing, section parsers
;   wasm_exec.asm      — execution engine, opcode dispatch, stack ops
;   wasm_run.asm       — top-level entry point (er_fn_run) and init

%include "x86_64/macros.inc"
default rel

%include "x86_64/wasm/wasm_constants.inc"

SECTION .text
; Decode sub-modules (order: utilities → module → sections → body → helper utilities)
%include "x86_64/wasm/wasm_decode_leb.asm"
%include "x86_64/wasm/wasm_decode_module.asm"
%include "x86_64/wasm/wasm_decode_sections.asm"
%include "x86_64/wasm/wasm_decode_body.asm"
%include "x86_64/wasm/wasm_decode_util.asm"

; Exec sub-modules (order: helpers → entry → dispatch → handlers)
%include "x86_64/wasm/wasm_exec_helpers.asm"
%include "x86_64/wasm/wasm_exec_entry.asm"
%include "x86_64/wasm/wasm_exec.asm"
%include "x86_64/wasm/wasm_exec_handlers.asm"

; JIT sub-modules (no dispatch loop, emit native code directly)
%include "x86_64/wasm/wasm_jit.inc"
%include "x86_64/wasm/wasm_jit_emit.asm"
%include "x86_64/wasm/wasm_jit_templates.asm"
%include "x86_64/wasm/wasm_jit.asm"

%include "x86_64/wasm/wasm_run.asm"
