# WAT Codec Bridge To edgerun-c Runtime

This note corrects the earlier host-runtime conclusion for the codec WAT batch.
Inside `/home/ken/edgerun` I did not find an embeddable Rust WASM runtime
dependency. The user pointed out the owned runtime in `/home/ken/edgerun-c`;
that repository does have an EdgeRun-owned WASM parser/executor/JIT surface.

The conclusion is narrower now: native Rust replacement is not blocked on a
third-party runtime in principle, but it is still blocked on a bridge from
EdgeRun Rust protocol code to the `edgerun-c` runtime ABI, plus proof that the
runtime supports the exact codec module shapes.

## Runtime Surfaces Found

`/home/ken/edgerun-c/kernel/x86_64/wasm/wasm_core.er` names the current WASM
authority family and points at the legacy/current parser, module, interpreter,
and test object artifacts. It explicitly says the replacement direction is to
replace WASM constants, module, parser, interpreter, and run artifacts with
ER-owned WASM core authority and receipts.

`/home/ken/edgerun-c/kernel/x86_64/wasm/wasm_exec.er` names the execution
authority family and ties it to `wasm_exec.asm`, `wasm_exec_entry.asm`,
`wasm_exec_handlers.asm`, and `wasm_exec_helpers.asm`.

`/home/ken/edgerun-c/kernel/x86_64/wasm/wasm_decode.er` names the decoder
authority family and ties it to module, section, body, LEB, and util decoders.

`/home/ken/edgerun-c/kernel/x86_64/wasm/wasm_run.asm` exposes the most relevant
entry points:

- `er_fn_run`: load, validate, resolve one export, and call it with no args.
- `er_fn_run_args`: same shape, but with a pointer to 64-bit argument slots and
  an argument count.
- `er_fn_load`: parse and prepare a module for repeated execution.
- `er_fn_load_trusted`: trusted load path that skips the no-recursion policy
  validator.
- `er_fn_call`: call a no-arg export on a loaded module.
- `er_fn_call_args`: call an export on a loaded module using 64-bit argument
  slots.

The load path calls `er_wasm_parse_module`, `er_wasm_resolve_imports`, and
`er_wasm_validate_no_recursion`; the trusted path skips the recursion validator.
Both mark the module valid before execution. The module primer validates initial
memory pages, runs the start function once if present, and applies active data
segments to runtime memory.

`/home/ken/edgerun-c/kernel/x86_64/wasm/wasm_exec_entry.asm` shows the executor
ABI:

- `er_fn_exec(function_index, args_ptr, args_count)` initializes locals from
  64-bit argument slots.
- Results are returned in `rax`, with an error code in `rdx`.
- Imported function dispatch exists, but currently documents support for only
  zero, one, or two imported arguments before returning unsupported for wider
  imports.

`/home/ken/edgerun-c/kernel/x86_64/wasm/wasm_jit.asm` exposes
`er_wasm_jit_exec(function_index, args_ptr, args_count)`, which can compile on
demand and execute a function through the JIT path. This looks like an internal
execution optimization rather than a stable external codec-host API.

`/home/ken/edgerun-c/kernel/x86_64/wasm/wasm_constants.inc` defines runtime
state for module memory:

- `runtime_memory_ptr`
- `runtime_memory_len`
- `runtime_imports_ptr`
- `runtime_imports_len`
- `runtime_initial_pages`
- `executor_memory_pages`
- `executor_memory_limit`

`/home/ken/edgerun-c/kernel/x86_64/wasm_defines.inc` defines the resource limits
and base WASM constants. Notable limits for codec modules are 1024 functions,
128 types, 32 params, 4 results, 256 exports, 128 locals, 4096 value-stack
slots, 16 imports, and 64 KiB WASM pages.

There is also an application-side UI WASM ABI:

- `/home/ken/edgerun-c/app/src/ui/wasm.er` exports UI component functions and an
  exported `memory`-style pointer API for a WASM build.
- `/home/ken/edgerun-c/app/src/er/ui/wasm.er` is the inverse shape: it exports
  the same UI-facing ABI while importing host functions under `"er_ui"`.

Those UI files prove an ER/WASM ABI style, but they are not the codec execution
runtime. The codec bridge should be based on the kernel WASM runtime surfaces.

## Codec Export Shape To Support

The current codec WAT modules use the same simple host pattern already used by
the Node runners: exported memory, exported identity functions, scalar args,
and fixed scratch offsets chosen by the host.

For the first HTTP proof:

- `http2-frame.wat` exports `memory`, `proto_abi_version`,
  `proto_standard_id`, `http2_frame_type_classify`,
  `http2_frame_header_decode`, and `http2_frame_header_encode`.
- `http2_frame_header_decode(in_ptr, in_len, max_frame_size, out_ptr) -> i32`
  writes a 28-byte little-endian record into module memory.
- `http2_frame_header_encode(payload_len, frame_type, flags, stream_id,
  out_ptr, out_cap) -> i64` returns low32 status and high32 written length.
- `http3-frame.wat` exports `memory`, `proto_abi_version`,
  `proto_standard_id`, `http3_frame_type_classify`,
  `http3_frame_header_decode`, and `http3_frame_header_encode`.
- `http3_frame_header_decode(in_ptr, in_len, out_ptr) -> i32` writes a 24-byte
  little-endian record.
- `http3_frame_header_encode(frame_type, payload_len, out_ptr, out_cap) -> i64`
  returns low32 status and high32 written length.

The ER runtime call surface can pass the scalar arguments using `er_fn_call_args`
or `er_fn_run_args`. The missing bridge piece is not scalar calling; it is
controlled access to module memory before and after each call.

## Needed Bridge API Shape

A Rust or EdgeRun host bridge should expose a small `CodecExecutor`-like API
over the `edgerun-c` runtime:

```text
load(wasm_bytes, runtime_memory) -> LoadedCodec
call_i32(export_name, args64[]) -> Result<i32, RuntimeError>
call_i64(export_name, args64[]) -> Result<i64, RuntimeError>
write_memory(offset, bytes) -> Result<(), RuntimeError>
read_memory(offset, len) -> Result<Vec<u8>, RuntimeError>
module_id() -> { abi_version, standard_id }
```

For `http2_frame_header_decode`, the bridge would:

1. allocate or provide a runtime memory buffer of at least one WASM page;
2. load the compiled `.wasm` bytes through `er_fn_load` or a test-only trusted
   load if the module is checked into the standards tree;
3. call `proto_abi_version` and `proto_standard_id`;
4. copy the input header bytes into runtime memory at `in_ptr`;
5. call `http2_frame_header_decode` through `er_fn_call_args` with
   `[in_ptr, in_len, max_frame_size, out_ptr]`;
6. read the 28-byte output record from `out_ptr`;
7. decode the record into the existing Rust `Frame` header fields.

For `http3_frame_header_decode`, the same bridge calls
`http3_frame_header_decode` with `[in_ptr, in_len, out_ptr]` and reads the
24-byte record.

The bridge should consume compiled `.wasm`, not `.wat`. `wat2wasm` should remain
a standards build/test step until there is an ER-owned WAT-to-WASM compiler path
ready for these modules.

## Blockers

The runtime exists, but I did not find a ready Rust-facing adapter in
`/home/ken/edgerun` that can call `er_fn_load`/`er_fn_call_args` or copy bytes
into and out of the runtime memory. That adapter must be designed before any
Rust source deletion.

The visible runtime ABI uses process/kernel assembly entry points and global
module/executor state. The first bridge must define ownership and concurrency:
one loaded module per runtime memory arena, one call at a time per instance, or
explicit synchronization at the host boundary.

The codec modules need direct memory writes and reads. `wasm_run.asm` exposes
runtime memory fields and the module primer applies data segments into
`runtime_memory_ptr`, but I did not find a public `write_memory` or `read_memory`
function for host callers. A bridge can own the runtime memory buffer and copy
directly if the ABI contract permits it; otherwise `edgerun-c` needs explicit
memory access helpers.

The runtime resource limits look sufficient for the small frame codecs, but this
needs proof. The first proof should record function count, type count, export
count, memory pages, decoded op count, and any unsupported opcode errors for the
actual compiled `http2-frame.wasm`.

The runtime import path supports imports, but current codec primitives should
not need imports. The first bridge should require zero imports and reject modules
that import functions, memory, tables, or globals.

The standards docs in `/home/ken/edgerun` still correctly say no embeddable
Rust dependency was found inside that workspace. They should not be read as a
claim that EdgeRun owns no WASM runtime anywhere. This bridge note supersedes
that part for the cross-repo path through `/home/ken/edgerun-c`.

## First Proof

The first proof should be HTTP/2 frame header decode only.

Use `standards/build/wasm/codec-primitives/http2-frame.wat`, compile it to
`.wasm`, and execute it through the `edgerun-c` runtime with one or two cases:

- a 9-byte DATA frame with zero payload;
- a SETTINGS frame with a nonzero payload length and enough payload bytes
  present;
- a truncated frame that must return status `1`.

The proof should compare:

- `proto_abi_version() == 2`;
- `proto_standard_id() == 300010`;
- `http2_frame_header_decode(...) == 0` for valid inputs;
- the 28-byte output record against the existing Node proof and Rust oracle;
- status `1` for the truncated input.

Only after that proof should we attempt a Rust-side bridge. The first Rust patch
should keep Rust frame structs and semantics, route only fixed header
decode/encode through a feature-gated WAT runtime adapter, and keep existing
Rust implementations as the default until corpus and composition parity are
green.
