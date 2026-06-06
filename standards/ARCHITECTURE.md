# EdgeRun WASM Pipeline Architecture

## Core Principle: Decoded Op Buffer as Pipeline Contract

All WASM execution flows through a shared decoded-op buffer — a flat array of
32-byte records (`DEC_SZ = 32`). The decoder (or WAT parser) normalizes any
input into canonical operations; backends consume only canonical ops.

```
┌──────────┐    raw bytes    ┌───────────┐   canonical 32-byte ops   ┌──────────────┐
│ WASM bin │ ──────────────→ │  Decoder  │ ─────────────────────────→ │  Backend(s)  │
├──────────┤                 ├───────────┤                           ├──────────────┤
│ WAT src  │ ──────────────→ │  Parser   │                           │ x86-64 SSE   │
└──────────┘                 └───────────┘                           │ AArch64 NEON │
                                                                     │ ARM32 NEON   │
                                                                     │ AVX-512 (TBD)│
                                                                     │ GPU (TBD)    │
                                                                     └──────────────┘
```

The decoded op record layout (32 bytes):

```
Offset  Size  Field
────────────────────────────────
  0       4    opcode          — top-level WASM opcode (0x00-0xFF)
  4       4    imm0            — canonical SIMD operation ID (for 0xFD prefix)
  8       4    imm1 / offset   — lane index, alignment, or memory offset
 12       4    imm2 / align    — alignment or second immediate
 16      16    v128_imm        — 16 bytes of v128 constant data
```

Backends **never interpret raw WASM bytes**. They iterate decoded ops and emit
native code based on the canonical imm0 values.

## Canonical SIMD Opcodes

Each WAT SIMD operation gets a unique integer ID (0x00-0xFF range). Backend
dispatch tables compare `imm0` against these canonical IDs, never against
encoding-specific sub-opcodes.

The decoder translates raw sub-opcodes (from any WASM binary encoding) to
canonical IDs before storing them in the decoded op buffer:

```wat
;; After reading raw LEB128 sub-opcode into $raw:
(if (i32.eq (local.get $raw) (i32.const 0x2D))  ;; old encoding
  (i32.store (base+4) (i32.const OP_i8x16_splat)))
(if (i32.eq (local.get $raw) (i32.const 0x0F))  ;; modern wabt
  (i32.store (base+4) (i32.const OP_i8x16_splat)))
```

### Canonical ID Table (0x00-0x9F)

Grouped by shape, then by operation type:

#### v128 (0x00-0x0F)
| ID   | Op             | ID   | Op               |
|------|----------------|------|------------------|
| 0x00 | v128.load      | 0x08 | v128.load64_splat|
| 0x01 | v128.load8x8_s | 0x09 | v128.store       |
| 0x02 | v128.load8x8_u | 0x0A | v128.const       |
| 0x03 | v128.load16x4_s| 0x0B | v128.and         |
| 0x04 | v128.load16x4_u| 0x0C | v128.or          |
| 0x05 | v128.load32x2_s| 0x0D | v128.xor         |
| 0x06 | v128.load32x2_u| 0x0E | v128.not         |
| 0x07 | v128.load8_splat | 0x0F | v128.bitselect |

#### i8x16 (0x10-0x2F)
| ID   | Op                     | ID   | Op                    |
|------|------------------------|------|-----------------------|
| 0x10 | i8x16.splat            | 0x20 | i8x16.max_s           |
| 0x11 | i8x16.extract_lane_s   | 0x21 | i8x16.max_u           |
| 0x12 | i8x16.extract_lane_u   | 0x22 | i8x16.avgr_u          |
| 0x13 | i8x16.replace_lane     | 0x23 | i8x16.neg             |
| 0x14 | i8x16.eq               | 0x24 | i8x16.abs             |
| 0x15 | i8x16.ne               | 0x25 | i8x16.shl             |
| 0x16 | i8x16.lt_s             | 0x26 | i8x16.shr_s           |
| 0x17 | i8x16.lt_u             | 0x27 | i8x16.shr_u           |
| 0x18 | i8x16.gt_s             | 0x28 | i8x16.swizzle         |
| 0x19 | i8x16.gt_u             | 0x29 | i8x16.narrow_i16x8_s  |
| 0x1A | i8x16.le_s             | 0x2A | i8x16.narrow_i16x8_u  |
| 0x1B | i8x16.le_u             |      |                       |
| 0x1C | i8x16.ge_s             |      |                       |
| 0x1D | i8x16.ge_u             |      |                       |
| 0x1E | i8x16.add              |      |                       |
| 0x1F | i8x16.sub              |      |                       |
| 0x20 | i8x16.min_s            |      |                       |
| 0x21 | i8x16.min_u            |      |                       |

Wait, this overlaps. Let me use a cleaner grouping. The 0x10 and 0x20 overlap for min_s/min_u/max_s/max_u/neg/abs etc.

Let me just do sequential numbering clearly. Actually, let me not try to be clever — I'll just use a flat sequential scheme ordered by WAT operation name:

| ID   | Op                          | ID   | Op                           |
|------|-----------------------------|------|------------------------------|
| 0x00 | f32x4.abs                   | 0x60 | i16x8.extmul_low_i8x16_s    |
| 0x01 | f32x4.add                   | 0x61 | i16x8.extmul_low_i8x16_u    |
| 0x02 | f32x4.convert_i32x4_s       | 0x62 | i16x8.extmul_high_i8x16_s   |
| 0x03 | f32x4.convert_i32x4_u       | 0x63 | i16x8.extmul_high_i8x16_u   |
| ... |                             |      |                              |

This is getting too complex for a WAT literal. Let me simplify.

I'll use the AArch64 dispatch.wat opcodes as the canonical IDs, since they're already sequential-ish and the AArch64 backend uses them. For ops not in the old encoding (like min_u, max_u, etc.), I'll add them after the last old-encoding opcode.

The last old-encoding opcode in the AArch64 dispatch is f64x2.ge at 0xA5. So I'll assign new ops starting from 0xA6.

Actually wait — the old encoding opcodes go up to 0xA5. The canonical IDs should start fresh. Let me just use the actual old-encoding values as-is. The decoder translates any input encoding to these canonical values. The compilers dispatch on these values. This is the minimal delta — no need to renumber everything.

**Decision: Use the AArch64 old-encoding opcodes as canonical IDs.**

The decoder:
- For old-encoding WASM: pass-through (raw = canonical)
- For modern wabt: translate (raw → canonical)
- For new future encodings: add translation rule

This means the decoder translates modern encoding (the wat2wasm encoding I decoded from the binary) to old encoding values. The compilers never change.

## Decoder: Sub-Opcode Normalization

For 0xFD SIMD prefix, the decoder:

1. Reads LEB128 sub-opcode from WASM binary
2. Determines extra operand bytes (memory align+offset, lane index, v128.const 16 bytes)
3. Stores canonical opcode ID at `decoded_op + 4` (imm0 field)
4. For old-encoding input: canonical ID = raw sub-opcode (passthrough)
5. For modern-encoding input: translation table maps raw → canonical

## Backend Contract

Each backend (compiler):
- Reads `opcode` from `decoded_op + 0`
- For `opcode == 0xFD`, reads canonical SIMD ID from `decoded_op + 4`
- Dispatches by canonical ID via if/else chain
- Emits native code for each canonical op
- Unknown canonical IDs → JIT_ERROR / ud2

Backends never change when a new WASM binary encoding appears. Only the
decoder's translation table changes.

## Future: AVX-512 and GPU

AVX-512 and GPU backends follow the same contract. The decoded op buffer is
the handoff point.

- **AVX-512**: Runtime CPU feature detection selects EVEX encoding; receive
  uint64_t slots still work (512-bit via two consecutive pushes).
- **GPU**: The flat decoded op array can be DMA'd to GPU memory. A GPU kernel
  reads each 32-byte record and executes the corresponding operation on warp
  lanes instead of vector registers.

```bash
# Building the pipeline
wat2wasm compiler-x86_64.wat -o compiler-x86_64.wasm
bash build.sh aarch64
bash build.sh arm32

# Write this CPU cap info into linear memory for runtime dispatch
```

## Opcode Name Convention

All template functions and canonical IDs use WAT lowercase names with dots and
hyphens replaced by underscores:

```
i8x16.extract_lane_s  →  OP_i8x16_extract_lane_s
f32x4.convert_i32x4_s →  OP_f32x4_convert_i32x4_s
v128.load8_splat      →  OP_v128_load8_splat
```

## Pipeline Infrastructure (`build/wasm/`)

Composable WAT modules for in-memory data processing. Data flows through shared
linear memory pipes (`pipe-core`). Stages are wired into a 64-slot dispatch
table (`pipeline-core`) and registered via `stage-registry`.

### Module Dependency DAG

```
edgerun-core (system/runtime/)
  ├─ pipe-core        — byte pipes + bump allocators
  │    ├─ frame-core  — framed I/O (8-byte headers)
  │    ├─ mux-core    — static/dynamic mux + demux stages
  │    ├─ pipeline-core — pipeline_run + 64-slot dispatch table
  │    └─ socket-core — abstract socket I/O stage
  ├─ encoding-text    — hex/base64 encode/decode stages
  ├─ ws-stage         — WebSocket frame transform stages
  ├─ wasm-interpreter — pipeline-local copy of the WASM interpreter
  │    └─ wasm-exec-stage — process_wasm_exec stage_fn wrapper
  └─ stage-registry   — wires all stages into dispatch table slots
```

### Stage Registry (64-slot dispatch table)

| Slot | Name | Module | Type | Description |
|------|------|--------|------|-------------|
| 0 | passthrough | pipeline-core | batch | Pipe drain input → output |
| 1 | hex_encode | encoding-text | batch | Hex lowercase encode |
| 2 | hex_decode | encoding-text | batch | Hex strict decode |
| 3 | b64_encode | encoding-text | batch | Base64URL no-pad encode |
| 4 | b64_decode | encoding-text | batch | Base64URL no-pad decode |
| 5 | transport | socket-core | streaming | Socket I/O: send+recv with timeout |
| 6 | mux_static | mux-core | batch | Round-robin drain, epoch batching |
| 7 | demux_static | mux-core | batch | Frame read → route by stream_id |
| 8 | mux_dynamic | mux-core | batch | Linked-list mux |
| 9 | demux_dynamic | mux-core | batch | Hash-table demux |
| 10 | ws_frame | ws-stage | streaming | Bidirectional WS framing |
| 11 | **wasm_exec** | **wasm-exec-stage** | batch | **Load + execute WASM binary** |
| 12–63 | (empty) | — | — | Available |

### wasm_exec Stage

Slot 11 in the dispatch table. Config format (variable length):

```
+0:  func_idx  i32  (function index to call; -1 = function 0)
+4:  arg_count i32  (number of i32 arguments)
+8:  args[]    i32  (inline argument values)
```

**Flow:**
1. Read WASM binary from input pipe into scratch buffer
2. Call `interpreter.load(scratch, len)` to parse + decode all sections
3. Call `interpreter.call(func_idx, args_ptr, arg_count)`
4. Read first result via `get_result_value(0)`, write 4 bytes to output pipe

**Example pipeline (hex → wasm_exec):**
```
hex_decode → wasm_exec → transport
```
Feed hex-encoded WASM over the wire, decode to binary, execute, send result back.

**Linking:** The runtime instantiates `interpreter.wasm` under module name
`"wasm-interpreter"`. The stage imports `load`, `call`, `get_result_value`,
and `get_result_count` from it. Both share linear memory via `edgerun-core`.

### Building

```bash
# Full pipeline build (from build/wasm/pipeline/)
./build.sh

# Or individual modules:
wasm-tools wat2wasm wasm-exec-stage.wat -o wasm-exec-stage.wasm
wasm-tools wat2wasm stage-registry.wat -o stage-registry.wasm
```

### Testing

```bash
# Integration test: WASM module → pipe → pipeline_run → result
node test-pipeline-interpreter.js
```
