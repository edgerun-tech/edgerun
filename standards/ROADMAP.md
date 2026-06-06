# EdgeRun Consolidation Roadmap

## Vision

A single WAT module `edgerun.wat` that is everything at once:

- **Library** — exports every function from every module
- **Build orchestrator** — self-hosting, can build itself
- **App** — runs on any WASM host
- **UI framework** — renders its own graphical interface
- **OS kernel** — boots on bare metal via virtio, handles interrupts, schedules processes
- **Pipeline engine** — every function is a pipeline stage, stages compose arbitrarily

Every function is a pipeline step. Every step can be SIMD-accelerated. The module owns one canonical `(memory ...)`. All sub-modules are source fragments concatenated into a single `(module ...)` block.

---

## Phase 1: Foundation — Single Module, Single Memory

**Goal**: One memory, one module, everything concatenated without conflicts.

| Step | File | Change |
|------|------|--------|
| 1.1 | `runtime/memory-map.wat` | **Create** — canonical address space layout, single source of truth for all offsets |
| 1.2 | `runtime/edgerun-core.wat` | Convert to fragment (no `(module)` or `(memory)`), reference memory-map, own the canonical `(memory)` declaration |
| 1.3 | `tools/build_wat.mjs` | **Create** — dependency-order concatenation → single `.wat` output |
| 1.4 | `edgerun.wat` | **Create** — top-level `(module ...)` that includes all fragments |
| 1.5 | All `runtime/`, `pipeline/` files | Convert to fragment format (remove `(module)`, `(memory)`, `(import)` where redundant) |
| 1.6 | `runtime/edgerun-core.wat` | Absorb `math-utils.wat` functions (single math namespace) |
| 1.7 | `pipeline/pipeline-core.wat` | Populate `stage_table` with all registered stage wrappers |

**Milestone**: `npm run build` produces `edgerun.wasm` — one file, all exports.

---

## Phase 2: Full Stage Table Population

**Goal**: Every module function has a pipeline stage wrapper. The stage table is fully populated.

| Slot | Stage | Source Module |
|------|-------|-------------|
| 0 | `passthrough` | Built-in |
| 1 | `hex_encode` | codec/hex-compat |
| 2 | `hex_decode` | codec/hex-compat |
| 3 | `b64_encode` | codec/encoding-base64 |
| 4 | `b64_decode` | codec/encoding-base64 |
| 5 | `b64url_encode` | codec/encoding-base64url |
| 6 | `b64url_decode` | codec/encoding-base64url |
| 7 | `b32hex_encode` | codec/encoding-base32hex |
| 8 | `b32hex_decode` | codec/encoding-base32hex |
| 9 | `json_parse` | codec/json-value-core |
| 10 | `json_emit` | codec/json-emit |
| 11 | `toml_scan` | codec/toml-scan |
| 12 | `yaml_scan` | codec/yaml-scan |
| 13 | `pem_parse` | codec/pem-rfc7468 |
| 14 | `deflate` | codec/deflate-inflate |
| 15 | `gunzip` | codec/gzip-member + deflate |
| 16 | `sha256` | crypto/crypto-sha256 |
| 17 | `hmac_sha256` | crypto/crypto-hmac-sha256 |
| 18 | `aes_encrypt` | crypto/crypto-aes-* |
| 19 | `aes_decrypt` | crypto/crypto-aes-* |
| 20 | `rsa_sign` | crypto/crypto-rsa-* |
| 21 | `rsa_verify` | crypto/crypto-rsa-* |
| 22 | `ed25519_sign` | crypto/crypto-ed25519-* |
| 23 | `ed25519_verify` | crypto/crypto-ed25519-* |
| 24 | `x25519` | crypto/crypto-x25519-* |
| 25 | `ecdsa` | crypto/crypto-ecdsa-* |
| 26 | `hkdf` | crypto/crypto-hkdf-* |
| 27 | `http1_parse` | protocol/http1-scan |
| 28 | `http1_emit` | protocol/http1-lines |
| 29 | `http2_frame` | protocol/http2-frame |
| 30 | `http3_frame` | protocol/http3-frame |
| 31 | `ws_frame` | protocol/ws-frame |
| 32 | `ws_encode` | protocol/ws-frame |
| 33 | `ws_decode` | protocol/ws-frame |
| 34 | `tls_decode` | protocol/tls-frame |
| 35 | `tls_encode` | protocol/tls-frame |
| 36 | `dns_parse` | protocol/dns-core-records |
| 37 | `dns_emit` | protocol/dns-message-header |
| 38 | `dhcp_parse` | protocol/dhcp-message-core |
| 39 | `dhcp_emit` | protocol/dhcp-message-core |
| 40 | `quic_parse` | protocol/quic-core-state |
| 41 | `der_parse` | protocol/der-asn1-basic |
| 42 | `ipv4_route` | protocol/ipv4-net |
| 43 | `wasm_exec` | compiler/interpreter-core |
| 44 | `wasm_compile` | compiler/compiler-x86_64 |
| 45 | `wat_parse` | compiler/interpreter-wat |
| 46 | `frame_pacer` | pipeline/frame-pacer |
| 47 | `mux_static` | pipeline/mux-core |
| 48 | `demux_static` | pipeline/mux-core |
| 49-63 | reserved | — |

**Milestone**: Any pipeline descriptor with any combination of these stages runs without errors. `pipeline_run` is the universal entry point.

---

## Phase 3: Streaming Parser Stages

**Goal**: Every parser stage supports streaming input.

All batch parsers get a streaming wrapper that:
- Accepts partial input via `STATUS_MORE` yield
- Maintains state between calls (internal buffer, parse position)
- Resumes from exactly where it left off

**Files modified**: Each protocol/codec `*-stage.wat` file gets a streaming entry point.

**Milestone**: Chunked TCP input flows through `[TLS_DECODE, HTTP1_PARSE, WS_FRAME, JSON_PARSE]` without buffering the entire stream.

---

## Phase 4: Pipeline Runtime / CLI

**Goal**: Inspect, debug, hot-reload pipelines.

| File | Purpose |
|------|---------|
| `pipeline/pipeline-serialize.wat` | Pipeline descriptor ↔ JSON |
| `pipeline/pipeline-metrics.wat` | Per-stage counters (bytes, calls, latency) |
| `pipeline/pipeline-diff.wat` | Compare two descriptors |
| `pipeline/pipeline-hot-reload.wat` | Swap a single stage at runtime |
| `tools/pipe-run.wat` | CLI runner: reads pipeline desc + input, runs, writes output |

**Milestone**: A pipeline is serializable to JSON, hot-reloadable, and measurable.

---

## Phase 5: Visualization Bridge

**Goal**: Pipeline graphs render in real-time through the UI framework.

| File | Purpose |
|------|---------|
| `ui/110-pipeline-graph.wat` | Pipeline descriptor → UI render tree (nodes + edges) |
| `ui/111-stage-inspector.wat` | Click stage → detail panel (config, metrics, errors) |
| `ui/112-pipeline-animator.wat` | Data flow animation along connectors |
| `ui/113-pipeline-editor.wat` | Drag-and-drop stage composition |
| `ui/114-pipeline-debugger.wat` | Step-through debugging |

**Milestone**: `pipeline_run` output renders as an interactive DAG in the UI.

---

## Phase 6: JIT Compiler as Pipeline Stage

**Goal**: Dynamic WASM compilation and execution in the pipeline.

| File | Purpose |
|------|---------|
| `pipeline/jit-compile-stage.wat` | Takes WASM binary, outputs native code |
| `pipeline/elf-emit-stage.wat` | Takes native code, outputs ELF64 binary |
| `compiler/hotpath-detect.wat` | Profile interpreter → trigger JIT after threshold |
| `pipeline/wasm-exec-stage.wat` | Extended: interpreter first, JIT after hot threshold |

**Milestone**: Feed WAT → `[WAT_PARSE, JIT_COMPILE, ELF_EMIT]` → native executable.

---

## Phase 7: Unikernel Pipeline

**Goal**: Bootable pipeline that processes network packets at native speed.

| File | Purpose |
|------|---------|
| `pipeline/virtio-net-stage.wat` | Reads packets from virtio ring |
| `pipeline/virtio-blk-stage.wat` | Reads/writes block device |
| `net-stack.pipe` | `[VIRTIO_NET, IPV4_ROUTE, UDP, DNS, HTTP1, ...]` |
| `boot.pipe` | `[VIRTIO_BLK, EXT4_READ, CONFIG_LOAD, INIT_STAGES]` |
| `system/kernel-pipeline.wat` | The kernel is a pipeline descriptor |
| `pipeline/elf-load-stage.wat` | Loads ELF segments into memory |

**Milestone**: A VM boots with this pipeline as the kernel, serves HTTP, runs at JIT-compiled native speed.

---

## Build System

`tools/build_wat.mjs` — concatenates all `.wat` source fragments in dependency order into a single `edgerun.wat` module.

```
build_wat.mjs --out edgerun.wat modules/*/
```

Dependency order:
1. `runtime/memory-map.wat` — address space constants
2. `runtime/edgerun-core.wat` — memory, helpers, status codes
3. `runtime/math-utils.wat` — math functions
4. `pipeline/pipe-core.wat` — data transport
5. `pipeline/pipeline-core.wat` — dispatch, descriptors
6. `compiler/interpreter-core.wat` — interpreter engine
7. All protocol/codec/crypto modules (alphabetical)
8. All pipeline stage wrappers
9. `compiler/interpreter.wat` — WAT parser, exports
10. `ui/*.wat` — UI framework
11. `system/*.wat` — system layer, compositor
12. `device/*.wat` — device abstractions

Output: single `edgerun.wasm` with ~300 exports, 64-slot stage table, unified memory.

---

## SIMD Acceleration Strategy

Every pipeline stage should have a SIMD fast path:

- **memcpy/memset**: 16-byte or 32-byte vector copies
- **base64 encode/decode**: 4→3 or 3→4 byte vector shuffles
- **SHA-256**: message schedule computed via `i32x4` adds
- **JSON/TOML/YAML scanning**: vectorized whitespace/token detection
- **Font rendering**: `f32x4` bezier evaluation
- **Icon rendering**: `i8x16` alpha blending
- **Protocol headers**: `i32x4` pattern matching

The `stage_fn` signature receives an aligned scratch buffer — stages with SIMD fast paths can detect alignment and dispatch accordingly.

---

## Implementation Priority

```
Phase 1 ─────── critical path (30 files, ~2000 lines)
  └─► Phase 2 ── high value (30 wrappers, ~1500 lines)
       ├─► Phase 3 ── streaming (15 files, ~1000 lines)
       ├─► Phase 4 ── tooling (5 files, ~800 lines)
       ├─► Phase 5 ── visualization (5 files, ~2000 lines)
       ├─► Phase 6 ── JIT stages (4 files, ~500 lines)
       └─► Phase 7 ── unikernel (6 files, ~1000 lines)
```

Phase 1 unlocks everything. Phase 2 unlocks maximum utility. Phase 5 makes it visible. Phase 7 is the capstone.
