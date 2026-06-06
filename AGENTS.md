# EdgeRun Agent Guide

This repository ports the EdgeRun C/assembly kernel (`~/edgerun-c/`) to WAT
(WebAssembly Text Format) for browser-node compatibility. There is no Rust
workspace.

**Source**: `~/edgerun-c/kernel/x86_64/` — original ASM/C reference

## WAT Ports (standards/ports/)

WAT projects are standalone — NOT Rust workspace members. Build with
`wat2wasm`, test via the interpreter.

### edgerun-x86-wasm-runtime

Porting the x86_64 WASM compiler runtime from assembly to WAT.

- **Source**: `~/edgerun-c/kernel/x86_64/wasm/` (wasm_exec.asm,
  wasm_interpreter.asm.erobj, etc.)
- **Port**: `standards/ports/edgerun-x86-wasm-runtime/wasm-interpreter/`
  - `compiler.wat` — WASM → x86_64 compiler: decodes decoded_ops and emits
    x86_64 machine code into code cache at 0x100000
  - `interpreter.wat` — WASM interpreter, shares linear memory with compiler
  - `test-compiler-all.js` — 145 compiler tests
  - `test-interpreter.js` — 123 interpreter tests
  - `test-elf.js` — ELF binary emission test (compiles WASM → native binary)

**Build**:
```bash
wat2wasm compiler.wat -o compiler.wasm
wat2wasm interpreter.wat -o interpreter.wasm
```

**Test** (all tests load interpreter.wasm and compiler.wasm):
```bash
node test-compiler-all.js
node test-interpreter.js
node test-elf.js
```

**ELF / Flat binary output**:
- `compiler.compile_to_elf(func_idx)` → returns `(buffer_addr, size)` of a
  self-contained ELF64 executable that runs the compiled WASM function and
  exits with its return value
- `compiler.compile_to_bin(func_idx)` → returns `(buffer_addr, size)` of a
  flat x86-64 binary (bare-metal, no headers) that runs the compiled WASM and
  stores the result at address `0x500`, then halts
- ELF runtime stub sets up `r15 = JitGlobals` (in BSS at `0x200000`), WASM
  linear memory (64KB), locals, globals, and table entries — all in BSS
- Flat binary uses the same JitGlobals/BSS layout; no OS dependencies

**Key architecture**:
- Two-pass: interpreter decodes WASM bytecode into decoded_op buffer (16 bytes
  per op: opcode + pad + imm0 + offset + imm2). Compiler reads decoded ops and
  emits x86_64 machine code.
- Compiler and interpreter share the same linear memory.
- Compiler output goes to code cache at 0x100000.

**Style rules** (WAT):
- Emit helpers write x86_64 instruction bytes via `$emit_byte`, `$emit_dword`,
  `$emit_modrm`
- Template functions implement one WASM opcode pattern using emit helpers
- The compile loop in `$jit_compile` dispatches by opcode via if/else chain
- 0xFC prefix ops (trunc_sat) dispatch on `$imm0` sub-opcode
- Unsigned conversions use branch-free cmov/sar/and patterns
- Saturating truncations use rel8 conditional jumps with `$patch_rel8`

## Engineering Rules

- Always read existing code before writing.
- No third-party dependencies — use only wat2wasm.
- Run tests before claiming correctness.
- Keep changes scoped to one concept.
