# EdgeRun Unified Pipeline SDK

Single-module WAT runtime for massively parallelizable, platform-independent
data transformation pipelines. Zero dependencies. Self-hosted tooling only.

## Design

All code lives in a single `(module ...)` block in `edgerun.wat`. The module
owns linear memory, exports a pipeline runner, and exposes every stage through
a 64-slot dispatch table. Pipelines are arrays of stage IDs + config pointers;
`pipeline_run` drives them sequentially through intermediate pipes.

### Core Principle

No new code. Existing fragments are integrated, deduplicated, and recombined
into pipeline stages. The result is fewer lines, fewer modules, zero
duplication, and no loss of meaning.

### Architecture

```
edgerun.wat (single module)
├── Memory + LUTs           (char classification, lowercase maps)
├── Shared globals          (status codes, heap bounds, pipe offsets, syscalls)
├── Core helpers            (memcpy, pack, bounds_check, char class, SIMD)
├── Pipeline infrastructure (pipe-core, frame-core, mux-core, pipeline-core)
├── Encoding stages         (hex, base64, varint, crc32, adler32, utf8, etc.)
├── Crypto stages           (SHA-1, SHA-256, SHA-512, HMAC, AES, etc.)
├── Protocol stages         (HTTP/1.1, HTTP/2, HTTP/3, DNS, TLS, WS, HPACK, etc.)
├── Tor stages              (cell codec, ntor, hidden service, directory, relay)
├── Application stages      (wallet, codex, mesh, OCI, auth, oauth, etc.)
├── System infrastructure   (allocator, event loop, telemetry, tokenizer, etc.)
├── Device interfaces       (BLE, BT, ESP32, TPM, virtio, etc.)
├── Data processing stages  (glob, html-strip, uuid, color, etc.)
├── Networking stages       (socket, session, BT frame codec)
├── UI framework            (92 composable UI fragments)
├── WASM compiler/interpreter stages (x86-64, aarch64, arm32 backends)
└── Test/validation         (ABI reject tests, tor receipts, shape constants)
```

### Pipeline Model

```
pipeline_run(desc, input_pipe, output_pipe, scratch, scap) → status
```

Drives N stages sequentially. Each stage reads from an input pipe, writes to
an output pipe. Intermediate pipes are created/destroyed by the runtime.

### Stage Interface

```
(type $stage_fn (func (param i32 i32 i32 i32 i32 i32 i32) (result i32)))
;; params: input_pipe, output_pipe, config_ptr, config_len, scratch_ptr, scratch_cap, state_ptr
```

### 64-Slot Dispatch Table

| Slot | Name | Type | Description |
|------|------|------|-------------|
| 0 | passthrough | batch | Pipe drain input → output |
| 1 | hex_encode | batch | Hex lowercase encode |
| 2 | hex_decode | batch | Hex strict decode |
| 3 | b64_encode | batch | Base64URL no-pad encode |
| 4 | b64_decode | batch | Base64URL no-pad decode |
| 5 | transport | streaming | Socket I/O: send+recv with timeout |
| 6 | mux_static | batch | Round-robin drain, epoch batching |
| 7 | demux_static | batch | Frame read → route by stream_id |
| 8 | mux_dynamic | batch | Linked-list mux, delegates |
| 9 | demux_dynamic | batch | Hash-table demux, delegates |
| 10 | ws_frame | streaming | Bidirectional WS framing (socket) |
| 11 | ws_encode | batch | Payload → WS frame (pure transform) |
| 12 | ws_decode | streaming | WS frame → payload (pure transform) |
| 13 | exec | batch | WASM load + call |
| 14-63 | (empty) | — | Available for new stages |

### Memory Zones

| Zone | Address | Purpose |
|------|---------|---------|
| LUTs | 0x01000–0x02FFF | Char classification, lowercase maps |
| HDR scratch | 0x3FFF0–0x3FFFF | Frame header I/O (16 bytes) |
| Pipe heap | 0x40000–0x80000 | User pipes, socket structs, configs |
| Stage heap | 0x80000–0x8FFFF | Persistent state blocks, decode buffers |
| Decoded ops | 0xA0000+ | WASM bytecode decode buffer |

### Reduction Targets

- **75% fewer lines**: eliminate duplication, remove dead modules, merge
  redundant implementations
- **100x speedup**: SIMD kernels, zero-copy pipe reads, frame-aligned
  intermediate pipes, tick-driven batching
- **Zero duplication**: single implementation per concept, shared through
  pipeline stages
- **Platform independent**: pure WAT, no host imports, no external deps

## Integration Status

| Phase | Module | Status | Lines |
|-------|--------|--------|-------|
| 1 | Core runtime (edgerun-core, shared-core) | Pending | ~940 |
| 2 | Pipeline infrastructure (pipe, frame, mux, pipeline) | Pending | ~900 |
| 3 | Encoding core + text codecs | Pending | ~2800 |
| 4 | Crypto / hash | Pending | ~3800 |
| 5 | Protocol stages | Pending | ~8000 |
| 6 | Tor | Pending | ~5000 |
| 7 | System / app / data / device / net | Pending | ~4000 |
| 8 | UI framework | Pending | ~12000 |
| 9 | WASM compiler / interpreter | Pending | ~4000 |
| 10 | Tests (validation only, no runtime test harness) | Pending | ~3000 |

**Total estimated**: ~45,000 lines → target: ~11,000 lines (75% reduction)

---

## Build System

The build is orchestrated via `package.json` scripts at the repo root.
All generated files (`compiler/gen/*.wat`, `ui/fragments/*`, `edgerun.wat`,
`edgerun.wasm`) are gitignored and should not be edited directly.

### Scripts

| Command | Description |
|---------|-------------|
| `bun run all` | Full build: generate JIT dispatch tables → pipeline stages → assemble `edgerun.wasm` |
| `bun run gen` | Generate JIT dispatch tables (`compiler/gen/jit-dispatch-*.wat`) and full JIT files (`compiler/gen/jit-full-*.wat`) |
| `bun run build` | Assemble `edgerun.wat` + `edgerun.wasm` from manifest fragments (standard profile) |
| `bun run build:full` | Assemble with all 3 JIT backends (x86-64, ARM32, AArch64) with renamed exports |
| `bun run build:ui` | Regenerate `ui/fragments/` from `ui/src/` and link `ui_framework.wat` (only needed when UI fragments change) |
| `bun run pipeline-stages` | Generate pipeline stage WAT files from `pipeline/registry.json` |
| `bun run build:cli -- <file.wat>` | Build a standalone CLI ELF from a WAT fragment |
| `bun run clean` | Remove all build artifacts |

### Build Scripts

All build scripts live in `tools/` and share `build-lib.mjs`:

- **`tools/build.mjs`** — Manifest-based module builder. Reads `manifest.json`,
  concatenates fragments in order, wraps in a `(module)`, compiles to WASM.
  Used by `bun run build`.

- **`tools/build_wat.mjs`** — Full build with all 3 JIT backends. Handles
  symbol renaming to avoid collisions. Used by `bun run build:full`.

- **`tools/build-lib.mjs`** — Shared library: fragment concatenation, WAT
  compilation (wasm-tools → wat2wasm fallback), WASM stripping/optimization.

- **`tools/er.mjs`** — `er` CLI tool for view/edit/build/commit workflows.
  Uses `compiler/er-tools.wasm` for self-hosting source management.

### Other Scripts

- **`cli/build.mjs`** — Links a user WAT fragment with CLI runtime
  libs and compiles via `wasm2elf` to produce a standalone ELF.

- **`ui/build_wat.mjs`** — UI framework builder. Generates
  cross-fragment import/export wrappers and links `ui_framework.wat`.

- **`pipeline/gen-stages.js`** — Generates pipeline stage `.wat`
  files from `pipeline/registry.json`.

- **`compiler/tools/gen_compiler.js`** — JIT compiler generator.
  Reads architecture JSON templates and emits dispatch/full JIT WAT files.

### Prerequisites

- **Bun** (for ES module scripts using `import.meta.dirname`)
- **Node.js** (for CommonJS scripts)
- **wat2wasm** or **wasm-tools** (for WAT → WASM compilation)
- **wasm2elf** (for CLI builds, from `compiler/tools/wasm2elf.js`)
