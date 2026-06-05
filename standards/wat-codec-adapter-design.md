# WAT Codec Host Adapter Design

This note scopes the thinnest Rust host adapter pattern for invoking the
codec-primitive WAT modules under `standards/build/wasm/codec-primitives`.

The adapter is a host-side execution boundary. It does not change EdgeRun wire
records, does not make WAT modules import each other, and does not move protocol
policy into the host. The host copies bytes into one module's exported memory,
calls an exported codec function, decodes the scalar status/result, and copies
output bytes or fixed records back out.

## Existing Runtime Facilities Inspected

Files and areas inspected:

- `Cargo.toml`: workspace members and workspace dependencies.
- `crates/edgerun-sdk/Cargo.toml`: SDK dependencies and features.
- `crates/edgerun-sdk/src/package.rs`: system `wasmtime` benchmarking helpers.
- `crates/node/edgerun-node/src/wasm.rs`: browser-node WASM ABI exported from
  Rust to JS.
- `crates/protocol/edgerun-work/src/wasm_worker_node.rs`: Rust-side WASM worker
  naming, but not an embedded WASM runtime.
- `workspaces/browser-core/browser/host-adapter.mjs`: browser JS memory-copy
  host adapter pattern.
- `standards/abi/standard-module-v1.md`: standard module ABI rules.
- `standards/build/wasm/codec-primitives/README.md`: codec primitive status and
  packed-result conventions.
- `standards/components/manifests/codec-primitives/encoding-core.toml`:
  manifest shape for exports, WAT path, ABI version, and standard id.
- `standards/runners/*smoke.js` and `standards/runners/rust-parity-*.js`:
  current Node-based compile/validate/instantiate pattern.

Conclusion:

- The repo has local `wasm-bindgen` compatibility crates for Rust compiled to
  browser WASM.
- The browser path uses JS `WebAssembly.instantiate` plus explicit memory-copy
  helpers.
- The SDK has a benchmark path that shells out to the `wasmtime` CLI.
- I did not find an existing embeddable Rust runtime dependency such as
  `wasmtime`, `wasmer`, or `wasmparser` in the workspace manifests.
- Adding an embeddable runtime crate would be a new dependency and requires
  explicit approval under the repo rules.

## Adapter Boundary

The portable API should live as types and traits that do not depend on any
runtime. Runtime-specific execution stays behind a `std` feature or a native
adapter crate.

Recommended layering:

```text
no_std + alloc facade
  - CodecModuleId
  - CodecStatus
  - PackedResult
  - CodecError
  - typed output record decoders
  - trait CodecExecutor

std host adapter
  - loads compiled .wasm bytes
  - instantiates a module using an approved runtime
  - resolves exported memory and functions
  - copies input/output buffers
  - maps traps/export errors/status codes

browser host adapter
  - JS WebAssembly implementation already matches this shape
  - Rust callers compiled to browser WASM should call browser-provided host
    functions or use JS glue, not embed a second runtime inside WASM
```

The facade can be implemented before selecting a Rust runtime. The first native
runtime implementation should be a narrow, feature-gated module after dependency
approval.

## Proposed Rust API

Core types:

```rust
pub const CODEC_ABI_VERSION: u32 = 2;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CodecModuleId {
    pub standard_id: u32,
    pub abi_version: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CodecStatus {
    Ok,
    InputShort,
    OutputShort,
    Invalid,
    OverflowOrTooLarge,
    Truncated,
    TooLongOrTooDeep,
    ModuleDefined(u32),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PackedResult {
    pub status: CodecStatus,
    pub high: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CodecError {
    MissingExport(&'static str),
    InvalidAbi { expected: u32, actual: u32 },
    InvalidStandardId { expected: u32, actual: u32 },
    MissingMemory,
    MemoryTooSmall,
    FunctionTrap,
    OutputTooSmall { needed_or_written: u32 },
    InputRejected(CodecStatus),
    RuntimeUnavailable,
}
```

The packed return decoder is shared by all callers:

```rust
pub fn decode_packed_i64(raw: i64) -> PackedResult {
    let value = raw as u64;
    PackedResult {
        status: CodecStatus::from_u32((value & 0xffff_ffff) as u32),
        high: (value >> 32) as u32,
    }
}
```

Runtime-independent executor trait:

```rust
pub trait CodecExecutor {
    fn module_id(&mut self) -> Result<CodecModuleId, CodecError>;

    fn call_i32_0(&mut self, name: &'static str) -> Result<i32, CodecError>;

    fn call_packed_i64(
        &mut self,
        name: &'static str,
        args: &[i64],
    ) -> Result<PackedResult, CodecError>;

    fn call_i32(
        &mut self,
        name: &'static str,
        args: &[i64],
    ) -> Result<i32, CodecError>;

    fn write_memory(&mut self, ptr: u32, bytes: &[u8]) -> Result<(), CodecError>;

    fn read_memory(&mut self, ptr: u32, len: u32) -> Result<Vec<u8>, CodecError>;
}
```

Thin typed wrapper example:

```rust
pub struct CodecModule<E> {
    exec: E,
}

impl<E: CodecExecutor> CodecModule<E> {
    pub fn verify_identity(
        &mut self,
        expected_standard_id: u32,
    ) -> Result<(), CodecError> {
        let id = self.exec.module_id()?;
        if id.abi_version != CODEC_ABI_VERSION {
            return Err(CodecError::InvalidAbi {
                expected: CODEC_ABI_VERSION,
                actual: id.abi_version,
            });
        }
        if id.standard_id != expected_standard_id {
            return Err(CodecError::InvalidStandardId {
                expected: expected_standard_id,
                actual: id.standard_id,
            });
        }
        Ok(())
    }
}
```

The API deliberately does not expose raw runtime handles to protocol crates.
Protocol crates should receive typed wrappers such as `Http2FrameCodec`,
`DerTlvCodec`, or `TlsNameCodec`, each with explicit function names and output
record decoders.

## Module Loading

Native std path:

```rust
pub struct WasmCodecStore {
    // runtime engine/cache, once an embeddable runtime is approved
}

impl WasmCodecStore {
    pub fn load_bytes(
        &self,
        wasm: &[u8],
        expected_standard_id: u32,
    ) -> Result<WasmCodecModule, CodecError>;
}
```

Loading steps:

1. Compile or instantiate the supplied `.wasm` bytes.
2. Require zero imports for current codec modules.
3. Resolve exports `memory`, `proto_abi_version`, and `proto_standard_id`.
4. Call identity exports and compare with the manifest.
5. Resolve standard-specific function exports lazily or at construction.
6. Keep one instance per use if mutable global state exists, especially
   `json-tape.wat`; otherwise instances may be pooled.

Browser path:

- Use the same logical API through JS `WebAssembly.instantiate`.
- Follow `workspaces/browser-core/browser/host-adapter.mjs`: allocate/copy
  input into exported memory, call, copy output bytes, and free only when the
  module exports allocator/free functions. Codec primitive modules currently
  use fixed host-chosen offsets and do not export allocators.

Build-time path:

- The adapter should consume `.wasm`, not `.wat`.
- `wat2wasm` remains a standards/test build step.
- Manifests should eventually include `wasm` path and hash so runtime loading
  does not compile WAT on the fly.

## Memory Copy Pattern

Current codec primitives export memory and accept caller-provided pointers.
They do not export alloc/free. The host therefore owns a scratch layout inside
the module memory:

```text
0x0000..0x3fff  input scratch
0x4000..0x7fff  output scratch
0x8000..0x8fff  fixed record scratch
```

The adapter should not assume these exact offsets globally. It should allocate
a simple monotonic scratch plan per call:

```rust
pub struct ScratchPlan {
    pub input_ptr: u32,
    pub input_len: u32,
    pub output_ptr: u32,
    pub output_cap: u32,
    pub record_ptr: u32,
    pub record_cap: u32,
}
```

For the first implementation, use fixed offsets and require the exported memory
to be large enough. If a module needs larger buffers, reject with
`CodecError::MemoryTooSmall` until memory growth policy is designed.

Recommended minimum first scratch layout:

```text
input_ptr  = 0
input_cap  = 64 KiB
output_ptr = 64 KiB
output_cap = 64 KiB
record_ptr = 128 KiB
record_cap = 4 KiB
```

If the module exports only one 64 KiB page, use a smaller default:

```text
input_ptr  = 0
input_cap  = 24 KiB
output_ptr = 24 KiB
output_cap = 24 KiB
record_ptr = 48 KiB
record_cap = 8 KiB
```

The adapter must bounds-check every write and read against current memory size
before touching memory.

## Function Call Shapes

The current WAT modules use a small set of function shapes:

```text
proto_abi_version() -> i32
proto_standard_id() -> i32
scalar/read helpers(...) -> i64 packed as status/high
validator/scanner(...) -> i32 status or classification
record scanners(ptr,len,out_ptr) -> i32 status
copying encoders(ptr,len,out_ptr,out_cap,...) -> i64 packed status/written
```

The host should support only these shapes first:

```rust
pub enum CodecCall<'a> {
    I32 { name: &'static str, args: &'a [i64] },
    PackedI64 { name: &'static str, args: &'a [i64] },
}
```

Typed wrappers map Rust-friendly APIs to these shapes. Example:

```rust
impl<E: CodecExecutor> Http2FrameCodec<E> {
    pub fn decode_header(
        &mut self,
        bytes: &[u8],
        max_frame_size: u32,
    ) -> Result<Http2FrameHeader, CodecError> {
        let scratch = self.copy_input_and_reserve_record(bytes, 32)?;
        let status = self.exec.call_i32(
            "http2_frame_header_decode",
            &[
                scratch.input_ptr.into(),
                bytes.len() as i64,
                scratch.record_ptr.into(),
                max_frame_size.into(),
            ],
        )?;
        map_status_i32(status)?;
        let record = self.exec.read_memory(scratch.record_ptr, 32)?;
        Http2FrameHeader::decode_record(&record)
    }
}
```

## Status And Error Mapping

Shared codec status mapping:

```text
0 -> Ok
1 -> InputShort
2 -> OutputShort
3 -> Invalid
4 -> OverflowOrTooLarge
5 -> Truncated
6 -> TooLongOrTooDeep
other -> ModuleDefined(code)
```

Rules:

- `Ok` returns the decoded high value, output bytes, or decoded fixed record.
- `OutputShort` should be returned to caller with `high` when the module uses
  it to report partial/needed bytes.
- `Invalid`, `OverflowOrTooLarge`, `Truncated`, and `TooLongOrTooDeep` map to
  `CodecError::InputRejected(status)`.
- Runtime traps, missing exports, bad ABI, and memory errors are host errors,
  not codec validation errors.
- WAT-stricter parity mismatches must be represented in typed wrappers as
  different APIs, for example `percent_decode_strict`, not hidden behind an
  old permissive Rust name.

## no_std And std Boundary

The adapter facade should be `no_std + alloc`:

- status enums
- packed result decoding
- record decoders
- typed output structs
- traits

The execution backend is `std`:

- reading `.wasm` files
- runtime engine/store/module/instance handling
- memory access through runtime APIs
- error formatting for host failures

Protocol crates that must remain `no_std + alloc` should not depend on a
native WASM runtime. They can depend on the facade traits and continue to offer
native Rust implementations as the default. Runtime selection belongs in a
std/native host, test harness, node adapter, or browser host.

## Dependency Conclusion

There is no approved embeddable Rust WASM runtime dependency currently visible
in the workspace manifests. The only `wasmtime` usage found is a system command
benchmark in `crates/edgerun-sdk/src/package.rs`.

This is a statement about the Rust workspace dependency graph, not about all
EdgeRun runtime assets. The sibling `/home/ken/edgerun-c` tree has an
EdgeRun-owned WASM parser/executor/JIT path with `er_fn_load` and
`er_fn_call_args`; see `standards/wat-codec-edgerun-c-runtime-bridge.md`.
That path changes the blocker from "no runtime exists" to "no Rust/EdgeRun
bridge exists yet for loading codec `.wasm` bytes, copying module memory, and
calling codec exports through that ABI."

Therefore:

- Do not add `wasmtime`, `wasmer`, `wasmparser`, or `wat` without explicit
  approval.
- Do not shell out to `wasmtime --invoke` for production codec calls. That path
  is useful for benchmarking and diagnostics only.
- The first checked-in implementation can be the runtime-independent facade and
  JS/browser runner parity. Native Rust execution requires either dependency
  approval or a bridge to the existing `edgerun-c` runtime.

If approval is granted, the thinnest native implementation should use one
runtime crate behind a feature such as `std-wasm-runtime`, with no protocol
crate depending on that feature by default.

## Minimal First Implementation Path

1. Add a small workspace-local adapter crate or module, likely under
   `crates/utility/edgerun-codec-wasm-host` if a crate is desired.
2. Implement the `no_std + alloc` facade first:
   `CodecStatus`, `PackedResult`, `CodecError`, fixed-record decoders, and
   typed wrappers that depend only on `CodecExecutor`.
3. Add tests using a fake in-memory `CodecExecutor` to prove status decoding,
   error mapping, and fixed-record parsing without a WASM runtime.
4. Add component-manifest loading in a std-only helper, but only for metadata;
   do not execute WASM yet.
5. Prove the `edgerun-c` runtime bridge on one fixed-header codec, or request
   approval for an embeddable Rust runtime dependency if that bridge is not the
   selected path.
6. Implement one backend and wire only one low-risk surface:
   `http2-frame` or `dns-message-header`.
7. Add Rust-backed parity tests that call the typed adapter and compare against
   the existing Rust implementation.
8. Only then begin call-site replacement. Do not delete Rust source until the
   replacement path has corpus, parity, and composition coverage.

Best first replacement surface:

- `http2-frame` or `dns-message-header`, because they are fixed-size header
  codecs with no dynamic table, no crypto, no compression traversal, and no
  permissive/strict policy mismatch.

Avoid as first replacement:

- JSON tape, TOML scan, DNS compressed names, permissive form parsing, TLS
  certificate validation, HPACK/QPACK dynamic tables, and any crypto verifier.

## Risks

- Runtime dependency size and auditability: `wasmtime` or `wasmer` is a
  substantial dependency and must be approved deliberately.
- Performance: copying into WASM memory for small scalar codecs may cost more
  than native Rust. Use WAT replacement first for conformance and portability,
  not assumed speed.
- Strictness drift: some WAT modules intentionally reject inputs that current
  Rust accepts. Wrappers must expose strict APIs honestly.
- Stateful modules: `json-tape.wat` uses parser globals. Use per-call instances
  or explicit reset discipline before pooling.
- Memory growth and aliasing: fixed scratch offsets are simple but need clear
  bounds checks and no shared concurrent instance use.
- Source deletion risk: existing Rust may still be needed for `no_std` builds,
  browser builds without an embedded runtime, or permissive compatibility
  behavior.

## Decision

The adapter should be designed now, but Rust source deletion should wait. The
repo currently has enough evidence to build a facade and composition tests, but
not enough infrastructure to replace Rust implementations natively without a
runtime dependency decision.
