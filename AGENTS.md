# EdgeRun Agent Guide

This repository ports the EdgeRun C/assembly kernel (`~/edgerun-c/`) to WAT
(WebAssembly Text Format) for browser-node compatibility. There is no Rust
workspace.

**Source**: `~/edgerun-c/kernel/x86_64/` — original ASM/C reference

## WAT Ports (standards/ports/)

WAT projects are standalone — NOT Rust workspace members. Build with
`wat2wasm`, test via the interpreter.

### wasm-tooling

Porting the x86_64 WASM compiler runtime from assembly to WAT.

- **Source**: `~/edgerun-c/kernel/x86_64/wasm/` (wasm_exec.asm,
  wasm_interpreter.asm.erobj, etc.)
- **Port**: `standards/ports/wasm-tooling/wasm-interpreter/`
  - `compiler-x86_64.wat` — WASM → x86_64 compiler (standalone, monolithic)
  - `compiler-aarch64.wat` — WASM → AArch64 compiler (concatenated from parts)
  - `compiler-arm32.wat` — WASM → ARM32 compiler (concatenated from parts)
  - `interpreter.wat` — WASM interpreter, shares linear memory with compiler

**Build**:
```bash
wat2wasm compiler-x86_64.wat -o compiler-x86_64.wasm
bash build.sh aarch64    # builds compiler-aarch64.wasm
bash build.sh arm32      # builds compiler-arm32.wasm
wat2wasm interpreter.wat -o interpreter.wasm 2>/dev/null; true
```

**ELF / Flat binary output** (x86-64 only):
- `compiler-x86_64.compile_to_elf(func_idx)` → returns `(buffer_addr, size)` of a
  self-contained ELF64 executable that runs the compiled WASM function and
  exits with its return value
- `compiler.compile_to_bin(func_idx)` → returns `(buffer_addr, size)` of a
  flat x86-64 binary (bare-metal, no headers) that runs the compiled WASM and
  stores the result at address `0x500`, then halts
- ELF runtime stub sets up `r15 = JitGlobals` (in BSS at `0x200000`), WASM
  linear memory (64KB), locals, globals, and table entries — all in BSS
- Flat binary uses the same JitGlobals/BSS layout; no OS dependencies

**Key architecture**:
- Two-pass: interpreter decodes WASM bytecode into decoded_op buffer (now 32
  bytes per op: opcode + pad + imm0 + offset + imm2 + v128_imm); compiler reads
  decoded ops and emits native machine code.
- Compiler and interpreter share the same linear memory.
- Compiler output goes to code cache at 0x100000.

**Style rules** (WAT):
- Emit helpers write instruction bytes via `$emit_byte`, `$emit_dword`,
  `$emit_modrm`
- Template functions implement one WASM opcode pattern using emit helpers
- The compile loop in `$jit_compile` dispatches by opcode via if/else chain
- 0xFC prefix ops (trunc_sat) dispatch on `$imm0` sub-opcode
- 0xFD prefix ops (SIMD) dispatch on `$imm0` sub-opcode
- AArch64/ARM32 compilers are built by concatenating parts via `build.sh`:
  `base.wat` + `emit.wat` + `templates/all.wat` + `templates/simd-${ARCH}.wat` + `dispatch.wat`
- x86-64 compiler is monolithic (`compiler-x86_64.wat`)

## Engineering Rules

- Always read existing code before writing.
- No third-party dependencies — use only wat2wasm.
- Run tests before claiming correctness.
- Keep changes scoped to one concept.
