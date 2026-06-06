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

### build/wasm/ — Pipeline Infrastructure

Composable WAT modules for in-memory data processing — no host I/O, all data
flows through shared linear memory pipes. 24 tests passing across 2 test files.

**Module DAG** (indentation = import dependency):
```
edgerun-core        — shared memory, status codes, LUTs, pack()
  ├─ pipe-core      — byte pipes + bump allocator (0x40000–0x80000)
  │  ├─ frame-core  — framed I/O (8-byte header [stream_id][payload_len])
  │  ├─ pipeline-core — 64-slot dispatch table + pipeline_run
  │  ├─ encoding-core — binary I/O, varint, crc32/adler32
  │  ├─ socket-core   — send/recv pipe abstraction
  │  └─ hash/crypto-sha1 — SHA-1 (workspace at 65536)
  ├─ encoding-text   — hex/base64 encode/decode + process_* stages
  ├─ mux-core        — static/dynamic mux + demux + process_* stages
  ├─ stage-registry  — wires modules into dispatch table (10 of 64 slots)
  ├─ deflate-inflate — imports crc32/adler32 from encoding-core
  └─ ws-accept       — imports sha1 from crypto-sha1
```

**Stage dispatch** (pipeline-core call_indirect, indices 0–9):
| idx | Name             | Module         |
|-----|------------------|----------------|
| 0   | passthrough      | pipeline-core  |
| 1   | hex_encode       | encoding-text  |
| 2   | hex_decode       | encoding-text  |
| 3   | b64_encode       | encoding-text  |
| 4   | b64_decode       | encoding-text  |
| 5   | transport        | socket-core    |
| 6   | mux_static       | mux-core       |
| 7   | demux_static     | mux-core       |
| 8   | mux_dynamic      | mux-core       |
| 9   | demux_dynamic    | mux-core       |

**Frame format**: `[stream_id:u32_le][payload_len:u32_le][payload]` — 8-byte
header, no varint. `frame_read` stores payload metadata at scratch[0..4].

**Mux strategies**:
- Static: fixed array of stream pipes, round-robin drain. Config = `[count][pipe_0]...`
- Dynamic: linked list of stream nodes, O(1) add/remove, O(n) iteration

**Demux strategies**:
- Static: direct array indexing by stream_id. Config = `[count][pipe_0]...`
- Dynamic: hash table with linear probing (identity hash `stream_id & mask`)

**Shared HDR scratch at 0x3FFF0**: safe since stages run sequentially.
**Pipe bump heap at 0x40000–0x80000**: pipes are one-shot transfer buffers,
auto-reset on full drain.

## Engineering Rules

- Always read existing code before writing.
- No third-party dependencies — use only wat2wasm.
- Run tests before claiming correctness.
- Keep changes scoped to one concept.
