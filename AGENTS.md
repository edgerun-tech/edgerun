# EdgeRun Agent Guide

This repository ports the EdgeRun C/assembly kernel (`~/edgerun-c/`) to WAT
for browser-node compatibility.

**Source**: `~/edgerun-c/kernel/x86_64/` — original ASM/C reference

## WAT Ports (standards/ports/)

WAT projects are standalone — build with `wat2wasm`, test via the interpreter.

### wasm-tooling

Porting the x86_64 WASM compiler runtime from assembly to WAT.

- **Source**: `~/edgerun-c/kernel/x86_64/wasm/`
- **Port**: `standards/ports/wasm-tooling/wasm-interpreter/`

Build: `wat2wasm compiler-x86_64.wat -o compiler-x86_64.wasm`

### build/wasm/ — Pipeline Infrastructure

Composable WAT modules for in-memory data processing — no host I/O, all data
flows through shared linear memory pipes. 32 tests passing across 2 test files.

---

## Pipeline Architecture

### Module Dependency DAG

```
edgerun-core ─── shared memory, status codes, epoch, pack(), LUTs
  └─ pipe-core ─── byte pipes + bump allocators
       ├─ frame-core     ─── framed I/O (8-byte header [stream_id][payload_len])
       ├─ pipeline-core  ─── 64-slot dispatch table + pipeline_run + tick
       ├─ encoding-core  ─── binary I/O, varint, crc32/adler32
       ├─ socket-core    ─── abstract socket + process_transport stage
       ├─ mux-core       ─── static/dynamic mux + demux + process_* stages
       ├─ ws-stage       ─── ws_encode + ws_decode pure transform stages
       └─ hash/          ─── SHA-1, SHA-256, HMAC, etc.
  ├─ encoding-text  ─── hex/base64 encode/decode + process_* stages
  ├─ stage-registry ─── wires modules into dispatch table (12 of 64 slots)
  ├─ deflate-inflate─── imports crc32/adler32 from encoding-core
  └─ ws-accept      ─── imports sha1 from crypto-sha1
```

### Pipeline Execution Model

`pipeline_run(desc, input_pipe, output_pipe, scratch, scap)` drives N stages
sequentially. Each stage reads from **its own input pipe** and writes to **its
own output pipe**. The runtime creates intermediate pipes between stages.

**Chain flow:**
```
pipeline_run(input, output):
  prev = input
  for each stage i:
    out = (i == last) ? output : pipe_create(pcap)
    stage_fn(prev, out, config, clen, scratch, scap, state)
    close(prev) if prev != input
    prev = out
  return last result
```

Stage 0 reads from `input`, writes to an intermediate pipe.
Stage 1 reads from that intermediate pipe, writes to the next (or `output`).

**Example — mux → demux:**
```
Pipeline: [mux_static → demux_static]
Run: pipeline_run(desc, dummy, muxOut, ...)

Stage 0 (mux_static):
  reads from stream pipes configured in cfg (s0, s1)
  writes framed data → intermediate_pipe

Stage 1 (demux_static):
  reads from intermediate_pipe
  routes payloads → dOut0, dOut1 (configured in cfg)
```

**Example — encode → transport:**
```
Pipeline: [ws_encode → transport]
Run: pipeline_run(desc, payload_pipe, raw_out, ...)

Stage 0 (ws_encode):
  reads payload from payload_pipe
  writes WS-framed bytes → intermediate_pipe

Stage 1 (transport):
  reads WS-framed bytes from intermediate_pipe
  sends via socket send pipe
  reads response from socket recv pipe
  writes response to raw_out (pipeline output)
```

### Stage Interface

```wat
(type $stage_fn (func (param i32 i32 i32 i32 i32 i32 i32) (result i32)))
;; params: input_pipe, output_pipe, config_ptr, config_len, scratch_ptr, scratch_cap, state_ptr
```

**Params:**
| Param    | Description |
|----------|-------------|
| input    | Pipe to read from (previous stage's output or user-provided input) |
| output   | Pipe to write to (next stage's input or user-provided output) |
| config   | Pointer to stage configuration blob in linear memory |
| clen     | Length of configuration blob |
| scratch  | Temporary buffer for work (shared, reused per pipeline_run call) |
| scap     | Capacity of scratch buffer |
| state    | Pointer to state block (0 if stage is stateless/batch) |

**Return values:**
| Value | Constant | Meaning |
|-------|----------|---------|
| >= 0  | OK (0)   | Stage completed, pipeline continues to next stage |
| < 0   | —        | Fatal error, pipeline aborts immediately |
| 7     | MORE     | Stage yielded (needs another pipeline_run call) |
| 8     | TIMEOUT  | Stage timed out (tick budget exceeded) |

**Stage categories:**
- **Batch** (state=0): reads all available input, processes, writes output. No
  yield. Examples: hex_encode, hex_decode, b64_encode, b64_decode, mux_static,
  demux_static.
- **Streaming** (state!=0): may yield MORE when waiting for slow input. State
  block preserves progress across calls. Examples: transport, ws_decode.

### Clock / Tick / Epoch

Time flows through pipelines as **logical ticks**, not wall clock:

- **Pipeline tick** (`desc+16`): incremented once per `pipeline_run` call.
  Written to `state[0]` before each stage invocation (if state != 0).
- **Global epoch** (`edgerun-core.epoch`): a shared mut i32 in linear memory.
  External schedulers increment it. Stages can read it for cross-pipeline
  synchronization.
- **Epoch batching** (mux_static): only drains stream pipes every N ticks.
- **Tick-based timeout** (transport): if `tick - start_tick >= timeout_ticks`,
  returns TIMEOUT.

Convention: `state[0]` always holds the current tick (RO, written by
pipeline_run or scheduler). Stage-private fields start at `state[4]`.

### Memory Zones

| Zone | Address Range | Owner | Purpose |
|------|-------------|-------|---------|
| LUTs | `0x01000–0x02FFF` | edgerun-core | Char classification, lowercase maps |
| HDR scratch | `0x3FFF0–0x3FFFF` | frame-core, mux-core | Frame header I/O (16 bytes) |
| Pipe heap | `0x40000–0x6FFFF` | pipe-core | User pipes, socket structs, configs |
| Pipeline scratch | `0x70000–0x7FFFF` | pipeline-core | Intermediate pipes (ephemeral per run) |
| Stage heap | `0x80000–0x8FFFF` | stages | Persistent state blocks, decode buffers |
| Decoded ops | `0xA0000+` | wasm-interp | WASM bytecode decode buffer |

### Stage Registry (64-slot dispatch table)

| Slot | Name | Module | Type | State | Description |
|------|------|--------|------|-------|-------------|
| 0 | passthrough | pipeline-core | batch | — | Pipe drain input → output |
| 1 | hex_encode | encoding-text | batch | — | Hex lowercase encode |
| 2 | hex_decode | encoding-text | batch | — | Hex strict decode |
| 3 | b64_encode | encoding-text | batch | — | Base64URL no-pad encode |
| 4 | b64_decode | encoding-text | batch | — | Base64URL no-pad decode |
| 5 | transport | socket-core | streaming | 16B | Socket I/O: send+recv with timeout |
| 6 | mux_static | mux-core | batch | 12B | Round-robin drain, epoch batching |
| 7 | demux_static | mux-core | batch | — | Frame read → route by stream_id |
| 8 | mux_dynamic | mux-core | batch | — | Linked-list mux, delegates |
| 9 | demux_dynamic | mux-core | batch | — | Hash-table demux, delegates |
| 10 | ws_frame | ws-stage | streaming | 148B | Bidirectional WS framing (socket) |
| 11 | ws_encode | ws-stage | batch | — | Payload → WS frame (pure transform) |
| 12 | ws_decode | ws-stage | streaming | 148B | WS frame → payload (pure transform) |
| 13–63 | (empty) | — | — | — | Available for new stages |

### State Block Layouts

**Transport** (16 bytes, `process_transport`):
```
+0:  tick          i32 (RO, written by pipeline_run)
+4:  phase         i32 (0=idle, 1=awaiting recv)
+8:  start_tick    i32 (tick when phase=1 entered)
+12: timeout_ticks i32 (0 = infinite)
```

**WS frame (bidirectional)** (148 bytes, `process_ws_frame` — socket mode):
```
+0:   tick          i32 (RO)
+4:   phase         i32 (0=idle, 1=awaiting)
+8:   start_tick    i32
+12:  timeout_ticks i32
+16:  dbuf_len      i32
+20:  dbuf[128]     decode buffer
```

**WS decode** (148 bytes, `process_ws_decode` — pure transform):
```
+0:   tick          i32 (RO)
+4:   (reserved)
+8:   dbuf_len      i32
+12:  dbuf[128]     decode buffer
```

**Mux static epoch** (12 bytes, `process_mux_static`):
```
+0:  tick          i32 (RO)
+4:  epoch_len     i32 (0 = no batching)
+8:  last_flush    i32
```

### Session Model

A network session uses **two coordinated pipelines**:

```
Upstream:   [app → mux_static → ws_encode → transport]   (data → network)
Downstream: [transport → ws_decode → demux_static → app] (network → data)
```

Both pipelines share:
- One **socket handle** (transport uses send/recv pipes)
- One **epoch counter** (shared tick source)
- One **session descriptor** (links up_desc + down_desc + socket + epoch + state)

The scheduler drives both pipelines with the same epoch value:

```javascript
function driveSession(session) {
  session.epoch++;
  // Upstream: process app data → network
  let r = pipeline_run(session.up_desc, appOutput, netInput, scratch, scap);
  // Downstream: process network → app
  r = pipeline_run(session.down_desc, netOutput, appInput, scratch, scap);
}
```

### Frame Format

```
[stream_id: u32_le][payload_len: u32_le][payload: bytes]
```

8-byte header, fixed-size (no varint). `frame_read` stores payload metadata at
`scratch[0..4]` (payload_len) and `scratch[4..]` (payload bytes).

### Pipe Semantics

- Created via `pipe_create(capacity)` from bump heap
- One-shot transfer: data written, read, then auto-resets on full drain
- `pipe_available(p)` → bytes available for reading
- `pipe_read(p, dst, max)` → read bytes into dst, advance cursor
- `pipe_write(p, src, len)` → write bytes from src, advance cursor
- Intermediate pipes (created by pipeline_run) are closed after the next stage
  consumes them

### Engineering Rules

- Always read existing code before writing.
- No third-party dependencies — use only wat2wasm.
- Run tests before claiming correctness.
- Keep changes scoped to one concept.
