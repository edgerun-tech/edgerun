# EdgeRun — Single-Module WAT Runtime

Single-module WAT runtime for massively parallelizable, platform-independent
data transformation pipelines. Zero dependencies. Self-hosted tooling only.

## Design

All code lives in a single `(module ...)` block in `out/edgerun.wat`. The module
owns linear memory, exports a pipeline runner, and exposes every stage through
a dispatch table. Pipelines are arrays of stage IDs + config pointers;
`pipeline_run` drives them sequentially through intermediate pipes.

### Architecture

```
out/edgerun.wat (single module)
├── Memory + LUTs           (char classification, lowercase maps)
├── Shared globals          (status codes, heap bounds, pipe offsets, syscalls)
├── Core helpers            (memcpy, pack, bounds_check, char class, SIMD)
├── Pipeline infrastructure (pipe, frame, mux, pipeline cores)
├── ~145 pipeline stages    (encoding, crypto, protocol, app, UI, etc.)
├── UI framework            (92 composable UI fragments)
├── WASM compiler/interpreter (x86-64, aarch64, arm32 backends)
└── Test/validation         (ABI tests, shape constants)
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

### Memory Zones

| Zone | Address | Purpose |
|------|---------|---------|
| LUTs | 0x01000–0x02FFF | Char classification, lowercase maps |
| Pipe heap | 0x40000–0x80000 | User pipes, socket structs, configs |
| Stage heap | 0x80000–0x8FFFF | Persistent state blocks, decode buffers |
| Decoded ops | 0xA0000+ | WASM bytecode decode buffer |

## Build System

The build is orchestrated via `package.json` scripts at the repo root.
All generated files go under `out/` and are gitignored. Notable outputs:
- `out/edgerun.wat` — assembled single-module WAT (~6 MB)
- `out/edgerun.wasm` — compiled WASM binary (~3.5 MB)
- `out/gen/jit-dispatch-*.wat` — JIT dispatch tables
- `out/gen/jit-full-*.wat` — full JIT modules per architecture
- `out/pipeline/pipeline-stages.wat` — generated pipeline stages
- `out/ui/fragments/` — UI framework fragment wrappers
- `out/ui/ui_framework.wat` — linked UI framework module
- `out/data/embedded-source.wat` — source payload for er-tools
- `out/compiler/er-tools.wasm` — compiled er-tools binary

### Scripts

| Command | Description |
|---------|-------------|
| `bun run all` | Full build: generate JIT tables → assemble `out/edgerun.wasm` |
| `bun run gen` | Generate JIT dispatch tables and full JIT modules from templates |
| `bun run build` | Assemble `out/edgerun.wat` + `out/edgerun.wasm` (standard profile) |
| `bun run build:full` | Assemble with all 3 JIT backends and renamed exports |
| `bun run build:ui` | Regenerate `out/ui/fragments/` from `ui/src/` and link `out/ui/ui_framework.wat` |
| `bun run pipeline-stages` | Generate pipeline stage WAT files from `edgerun.registry` in `package.json` |
| `bun run build:cli -- <file.wat>` | Build a standalone CLI ELF from a WAT fragment |
| `bun run clean` | Remove all build artifacts (`rm -rf out/`) |

### Build Scripts

All build scripts live in `tools/` and share `build-lib.mjs`:

- **`tools/build.mjs`** — Manifest-based module builder. Reads `edgerun.manifest` from `package.json`,
  concatenates fragments, wraps in a `(module)`, compiles to WASM.

- **`tools/build_wat.mjs`** — Full build with all 3 JIT backends. Handles
  symbol renaming to avoid collisions. Used by `bun run build:full`.

- **`tools/build-lib.mjs`** — Shared library: fragment concatenation, WAT
  compilation (wasm-tools → wat2wasm fallback), WASM stripping/optimization.

- **`tools/er.mjs`** — `er` CLI tool for view/edit/build/commit workflows.
  Uses `out/compiler/er-tools.wasm` for self-hosting source management.

- **`cli/build.mjs`** — Links a user WAT fragment with CLI runtime
  libs and compiles via `wasm2elf` to produce a standalone ELF.

- **`ui/build_wat.mjs`** — UI framework builder. Generates
  cross-fragment import/export wrappers and links `out/ui/ui_framework.wat`.

- **`pipeline/gen-stages.js`** — Generates pipeline stage WAT
  files from `edgerun.registry` in `package.json`.

- **`compiler/tools/gen_compiler.js`** — JIT compiler generator.
  Reads architecture JSON templates and emits dispatch/full JIT WAT files.

### Key Files

- **`package.json` → `edgerun.manifest`** — Defines every fragment in order, with build profile
  and backend tags. Source of truth for what goes into the module.
- **`package.json` → `edgerun.registry`** — Defines all pipeline stages (slot #, name,
  pattern, constraints). Used by `gen-stages.js` to generate stage WAT code.
- **`package.json` → `edgerun.templates`** — Architecture templates for JIT backends.
  Base template inherited by all arch-specific templates.

### Prerequisites

- **Bun** (for ES module scripts using `import.meta.dirname`)
- **Bun** (for all scripts)
- **wat2wasm** or **wasm-tools** (for WAT → WASM compilation)
- **wasm2elf** (for CLI builds, from `compiler/tools/wasm2elf.js`)
