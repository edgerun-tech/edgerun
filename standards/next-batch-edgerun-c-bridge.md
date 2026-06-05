# Next Batch: edgerun-c WASM Runtime Bridge

This batch should prove that the checked-in WAT codec primitives can run through
the owned `/home/ken/edgerun-c` WASM runtime path. Integration can come later;
the immediate value is to replace "JS-hosted conformance artifact" with
"EdgeRun-owned runtime-executable codec kernel."

## Recommendation

Make the next highest-value batch the `edgerun-c` bridge proof, ahead of adding
more WAT codecs.

Reason: the current codec set is already broad enough to cover useful leaf
kernels for HTTP, TLS, DER, DNS, WebSocket, JSON, TOML, percent/form, HPACK, and
QPACK. More WAT increases coverage, but it does not move Rust deletion forward
until one in-process or owned-runtime execution path is proven. A small
`edgerun-c` bridge proof would unlock the next phase: feature-gated Rust
adapters that call WAT/WASM leaf kernels while keeping Rust object models and
protocol policy intact.

## First Proof Module

Run `http2-frame.wat` first.

Use:

```text
standards/build/wasm/codec-primitives/http2-frame.wat
```

Compile it to `.wasm` as a standards build step, then load that `.wasm` through
the `edgerun-c` runtime.

Why HTTP/2 first:

- It has the smallest high-value fixed-record ABI.
- It has no imports and one exported memory.
- `http2_frame_header_decode` uses only scalar args, byte loads, integer
  arithmetic, bounds checks, and fixed little-endian output stores.
- The existing proof runner already compares HTTP/2 frame header behavior
  against Rust `edgerun-protocols`.
- The Rust replacement target is narrow: header scan and header construction,
  not payload semantics or HTTP/2 stream/state validation.

Defer `http3-frame` until second. It is also valuable, but it uses `i64` frame
types and QUIC varints, so it adds more runtime surface. Defer `ws-frame` until
after HTTP/2 because it has multiple helpers, in-place mutation, and policy
strictness details around opcodes and control frames.

## Runtime Surfaces To Use

The relevant `edgerun-c` entry points are in:

```text
/home/ken/edgerun-c/kernel/x86_64/wasm/wasm_run.asm
/home/ken/edgerun-c/kernel/x86_64/wasm/wasm_exec_entry.asm
/home/ken/edgerun-c/kernel/x86_64/wasm_defines.inc
```

Use `er_fn_load` plus repeated `er_fn_call_args`, not only `er_fn_run_args`.
The repeated-call path proves the shape needed by protocol adapters.

Observed ABI:

```text
er_fn_load(runtime_ptr, wasm_bytes_ptr, wasm_bytes_len) -> rdx error
er_fn_call(runtime_ptr, export_name_ptr, export_name_len) -> rax result, rdx error
er_fn_call_args(runtime_ptr, export_name_ptr, export_name_len, args_ptr, args_count)
  -> rax result, rdx error
```

`RuntimeConfig` is 88 bytes and contains:

```text
memory_ptr, memory_len, ticks_ptr,
memory_grow_fn, memory_grow_ctx,
table_grow_fn, table_grow_ctx,
initial_pages, has_pages,
imports_ptr, imports_len
```

The bridge proof should use one runtime memory arena of at least one WASM page,
zero imports, no grow hooks for the first pass, and explicit initial pages if
needed.

## Exact Proof API

The proof harness should expose a small host-side bridge API. It can be C,
assembly test harness, ER source, or Rust FFI later; the first proof only needs
the contract to be real and executable.

Required calls:

```text
load_codec(wasm_bytes) -> loaded runtime
call0_i32("proto_abi_version") -> i32
call0_i32("proto_standard_id") -> i32
write_memory(offset, bytes)
call_i32("http2_frame_header_decode", [in_ptr, in_len, max_frame_size, out_ptr])
read_memory(out_ptr, 28) -> bytes
call_i64("http2_frame_header_encode", [payload_len, frame_type, flags, stream_id, out_ptr, out_cap])
read_memory(out_ptr, written_len) -> bytes
```

The minimum memory API is essential. The existing runtime uses
`runtime_memory_ptr` and `runtime_memory_len`; the proof must show that the host
can write the input header into module memory and read the fixed output record
back after execution.

## Required Test Cases

Decode tests:

```text
DATA length 0, stream 1
SETTINGS length 6, stream 0
PING length 8, opaque flags
reserved stream id bit set
input shorter than 9 bytes -> status 1
payload length larger than max_frame_size -> status 6
declared payload not present in input -> status 5
```

Encode tests:

```text
DATA length 0, stream 1 -> exact 9-byte header
SETTINGS length 6, stream 0 -> exact 9-byte header
payload length 0x01000000 -> status 4
out_cap < 9 -> status 2
reserved stream id high bit is masked or rejected exactly as the current WAT ABI says
```

Identity tests:

```text
proto_abi_version() == 2
proto_standard_id() == 300010
module has zero imports
module exports memory
```

The 28-byte decode record must be compared byte-for-byte against the existing
Node WAT proof and field-for-field against Rust `edgerun-protocols`.

## Success Criteria

This batch is successful when it proves all of the following:

- `edgerun-c` can parse, validate, load, and repeatedly call the compiled
  `http2-frame.wasm`.
- The runtime supports all opcodes used by `http2-frame.wat`.
- Host-managed runtime memory can be written before a call and read after a
  call.
- `er_fn_call_args` handles four scalar arguments for the decode export and six
  scalar arguments for the encode export.
- `i32` and packed `i64` results match the Node-hosted WAT runner.
- Valid and rejecting cases match the Rust oracle for the narrow header surface.
- The proof records runtime errors separately from codec status values.

This does not require editing Rust protocol code yet. It only proves that the
owned runtime is capable of executing the existing codec module shape.

## Blockers

The visible runtime API is global-state oriented. `er_fn_load` leaves one module
ready for `er_fn_call` / `er_fn_call_args`, and `er_fn_call_args` checks that the
loaded module matches the staged `runtime_ptr`. The first proof should treat one
runtime instance as single-threaded and non-reentrant.

The public host memory accessor is not obvious from the inspected files. The
bridge can probably own the backing memory passed through `RuntimeConfig` and
copy directly into that buffer, but the proof must make that contract explicit.
If direct access is not allowed, add tiny `edgerun-c` memory read/write helpers
before attempting Rust integration.

The first proof should reject modules with imports. Import resolution exists,
but codec primitives should not need imports, and avoiding host callbacks keeps
the proof small.

The proof should use `er_fn_load`, not `er_fn_load_trusted`, unless the test is
explicitly proving the trusted built-in module path. For deletion leverage we
need ordinary parse, resolve, and no-recursion validation.

The `.wat` to `.wasm` compiler is still outside the `edgerun-c` runtime proof.
That is acceptable for this batch. Treat compiled `.wasm` as the executable
artifact and keep WAT compilation in the standards build/proof layer.

## Does This Outrank More WAT Codecs?

Yes.

Adding more codecs is useful when it covers a missing external standard. But
the current bottleneck is replacement leverage, not corpus breadth. The existing
batch already has enough leaf kernels to delete or bypass meaningful Rust
scanner code if an owned runtime bridge works. Without the bridge, new WAT
modules remain conformance assets and JS-runner proofs.

Ranking for the next batch:

```text
1. edgerun-c HTTP/2 frame bridge proof
2. edgerun-c HTTP/3 frame bridge proof
3. edgerun-c WebSocket frame bridge proof
4. Rust feature-gated adapter facade over the proven bridge
5. More WAT codecs only where they feed a deletion target already audited
```

The first code replacement should still be conservative: keep Rust structs,
payload parsing, and protocol policy canonical; route only fixed frame header
decode/encode through a feature-gated WAT runtime adapter after the bridge proof
is green.

