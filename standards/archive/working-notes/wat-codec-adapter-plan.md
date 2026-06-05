# WAT Codec Rust Host Adapter Plan

This plan defines the first Rust host adapter shape for replacing checked-in WAT
codec primitives with Rust-built WebAssembly modules. The goal is a thin,
deterministic adapter over existing no-std codec logic, not a new protocol
implementation and not a new runtime dependency.

## Scope

Write the first replacement as a Rust crate or module that compiles to
`wasm32-unknown-unknown` and exports the same fixed ABI as the current WAT
module. Do not add host callbacks, allocator-dependent protocol behavior,
threads, clocks, sockets, files, or a second serialization format.

The first target should preserve this standard-module convention:

```text
proto_abi_version() -> i32
proto_standard_id() -> i32
memory
```

Byte-oriented functions should continue to use pointer/length/capacity values
and return either a status `i32` or packed `u64`:

```text
low32  = status
high32 = bytes_consumed | bytes_written | scalar result
```

Status values should remain compatible with
`standards/wat-encoder-decoder-roadmap.md`:

```text
0 ok
1 input_short
2 output_short
3 invalid
4 overflow
5 truncated
6 too_long
```

## Safest First Replacement Target

Replace `standards/build/wasm/codec-primitives/ws-frame.wat` first.

Reasons:

- It has a compact ABI with one fixed output record, one in-place byte helper,
  and one packed-return encoder.
- Its Rust source is already no-std-capable:
  `crates/protocol/edgerun-protocols/src/websocket.rs`.
- The required Cargo feature set is existing and local:
  `edgerun-protocols = { default-features = false, features = ["websocket"] }`.
- Existing tests already exercise the WAT and Rust parity path:
  `standards/runners/ws-frame-smoke.js`,
  `standards/runners/codec-parity-smoke.js`, and
  `standards/runners/rust-parity-http-ws.js`.

The first patch target should be a new Rust-built Wasm artifact for `ws-frame`,
not a behavioral rewrite of the WebSocket codec itself. Keep
`websocket.rs` as the canonical implementation and make the adapter only map the
ABI.

## Candidate Rust Source Surface

Canonical crate:

- `crates/protocol/edgerun-protocols`

Canonical file:

- `crates/protocol/edgerun-protocols/src/websocket.rs`

Candidate functions and constants:

- `decode_frame_prefix(header: &[u8]) -> Result<WebSocketFramePrefix, WebSocketError>`
- `decode_payload_len(prefix: WebSocketFramePrefix, extended: &[u8], max_len: usize) -> Result<usize, WebSocketError>`
- `decode_client_message(fin, opcode, mask, payload)` only as a reference for masking/opcode policy
- `encode_server_frame(opcode, payload)` only as a reference for header policy
- `WebSocketFramePrefix::extended_len_bytes()`
- `WS_OPCODE_BINARY`, `WS_OPCODE_CLOSE`, `WS_OPCODE_PING`, `WS_OPCODE_PONG`
- `WS_MAX_CONTROL_PAYLOAD_LEN`
- `WS_BASE_HEADER_LEN`, `WS_EXTENDED_16_LEN`, `WS_EXTENDED_64_LEN`

Important mismatch to handle deliberately: the WAT `ws_decode_prefix` rejects
reserved/unsupported opcodes, fragmented control frames, and extended control
payload lengths. The Rust `decode_frame_prefix` only decodes the two-byte prefix;
those WAT policy checks currently live outside that Rust function or are
implicit in later helpers. The adapter should either add tiny adapter-local
validation before writing the record, or first add a canonical no-std helper in
`websocket.rs` in a separate Rust-source patch. For the replacement path, the
lowest-risk adapter is an adapter-local validator that calls
`decode_frame_prefix` for byte extraction and mirrors the existing WAT policy
without changing crate behavior.

## Adapter Shape

Use one small Rust Wasm adapter crate per codec family at first. The adapter
should be `#![no_std]`, export C ABI functions, and depend only on workspace
crates already used by the canonical implementation.

Candidate location for a future implementation:

```text
crates/protocol/edgerun-protocols-wasm-adapters/
```

Candidate first module:

```text
src/ws_frame.rs
```

Candidate crate properties:

```toml
[lib]
crate-type = ["cdylib"]

[dependencies]
edgerun-protocols = { path = "../edgerun-protocols", default-features = false, features = ["websocket"] }
```

Do not add `wasm-bindgen`, `wasmtime`, `wasmi`, `wit-bindgen`, `serde`, or a WAT
parser dependency. The module only needs exported functions and linear memory.
Compilation can be done with `cargo build --target wasm32-unknown-unknown`.
Existing JS runners can instantiate the resulting Wasm with the standard
`WebAssembly` API.

Use `alloc` only if the canonical helper requires it. For `ws-frame`, the ABI
can avoid allocation entirely by encoding only frame headers, applying masks in
place, and writing fixed records. Do not call `encode_server_frame` in the
exported header helper because it allocates a `Vec` and appends payload bytes;
copy its header-size logic instead or factor a no-alloc canonical helper later.

## ABI Exports

The Rust adapter should export these names exactly for `ws-frame` parity:

```rust
#[unsafe(no_mangle)]
pub extern "C" fn proto_abi_version() -> i32;

#[unsafe(no_mangle)]
pub extern "C" fn proto_standard_id() -> i32;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ws_decode_prefix(ptr: u32, len: u32, out: u32) -> i32;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ws_decode_payload_len(
    ptr: u32,
    len: u32,
    payload_len_code: u32,
    max_len: u32,
    out: u32,
) -> i32;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ws_apply_mask_in_place(
    payload_ptr: u32,
    payload_len: u32,
    mask: u32,
) -> i32;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ws_write_server_frame_header(
    opcode: u32,
    payload_len_low: u32,
    payload_len_high: u32,
    out: u32,
    out_cap: u32,
) -> u64;
```

Return constants:

```rust
const ABI_VERSION: i32 = 2;
const STANDARD_ID_WS_FRAME: i32 = 300004;
```

Packing helper:

```rust
const fn pack_u64(status: u32, value: u32) -> u64 {
    ((value as u64) << 32) | status as u64
}
```

## Memory Helpers

Keep memory helpers tiny and local to the adapter. They should be the only
unsafe surface inside the adapter.

Recommended helpers:

```rust
unsafe fn input_slice<'a>(ptr: u32, len: u32) -> Option<&'a [u8]>;
unsafe fn output_slice<'a>(ptr: u32, cap: u32) -> Option<&'a mut [u8]>;
fn write_u32_le(out: &mut [u8], offset: usize, value: u32) -> Result<(), Status>;
fn write_u64_be(out: &mut [u8], offset: usize, value: u64) -> Result<(), Status>;
fn checked_range(ptr: u32, len: u32) -> Option<(*const u8, usize)>;
fn checked_range_mut(ptr: u32, len: u32) -> Option<(*mut u8, usize)>;
```

The helpers should reject:

- zero `out` pointer when the function must write a record;
- `ptr + len` overflow;
- `len` that cannot fit into `usize`;
- output capacity smaller than the fixed record or header length.

For wasm32, `u32` pointers map directly to linear memory addresses. Still keep
the conversion checked so the same adapter code is auditable and so accidental
native unit tests fail clearly.

For `ws_decode_prefix`, write a 20-byte little-endian record:

```text
fin: u32
opcode: u32
masked: u32
payload_len_code: u32
extended_len_bytes: u32
```

For `ws_decode_payload_len`, write a 12-byte little-endian record:

```text
payload_len_low: u32
payload_len_high: u32
consumed: u32
```

For `ws_apply_mask_in_place`, interpret `mask: u32` as the current WAT does:
the first payload byte is XORed with the most-significant mask byte, then the
next bytes use the remaining big-endian mask bytes in order.

## Error Mapping

Use explicit adapter status mapping. Do not expose Rust enum discriminants as
ABI values.

Recommended mapping:

```text
WebSocketError::TruncatedHeader        -> 1 input_short
WebSocketError::MessageTooLarge        -> 4 overflow
WebSocketError::ControlPayloadTooLarge -> 3 invalid
WebSocketError::UnsupportedOpcode      -> 3 invalid
WebSocketError::Fragmented             -> 3 invalid
WebSocketError::UnmaskedClientFrame    -> 3 invalid
all adapter pointer/range failures     -> 3 invalid
output capacity too small              -> 2 output_short
```

For `ws_decode_payload_len`, preserve the WAT-specific minimal-encoding checks:

- code `126` with decoded length `< 126` returns `3 invalid`;
- code `127` with decoded length `< 65536` returns `3 invalid`;
- code `127` with the high bit set returns `3 invalid`;
- decoded length greater than `max_len` returns `4 overflow`;
- malformed `payload_len_code` returns `3 invalid`.

For `ws_write_server_frame_header`, preserve these WAT-specific checks:

- opcode must be one of `0`, `1`, `2`, `8`, `9`, `10`;
- control opcodes `8`, `9`, `10` must have payload length `<= 125`;
- 64-bit payload length must not set the high bit;
- header lengths are `2`, `4`, or `10`;
- insufficient `out_cap` returns packed `{ status: 2, value: 0 }`.

## no_std/std Boundary

The adapter and canonical codec logic should remain `no_std`. Standard-library
code belongs only in tooling that builds, copies, or compares artifacts.

Recommended split:

- `crates/protocol/edgerun-protocols/src/websocket.rs`: canonical no-std
  protocol helpers.
- Future adapter crate: no-std C ABI wrapper over the canonical helpers.
- Existing JS runners: std host/test layer that compiles or instantiates Wasm.
- Any artifact-copy script: std-only build tooling, not a dependency of the
  adapter crate.

Do not put JS-runner behavior, `wat2wasm` invocation, filesystem paths, or
corpus loading into the Rust adapter.

## Replacement Patch Sequence

1. Add the Rust adapter crate or module with only `ws-frame` exports.
2. Build it for `wasm32-unknown-unknown`.
3. Compare the generated Wasm against the current WAT module through the
   existing runners without changing runner logic first.
4. Only after parity is proven, switch the `ws-frame` manifest or runner input
   to the Rust-built artifact in a separate patch.
5. Keep `standards/build/wasm/codec-primitives/ws-frame.wat` until the Rust
   artifact is proven by the same smoke and parity checks.

The first replacement patch should not touch `encoding-core`, `http1-scan`,
`http2-frame`, `http3-frame`, HPACK/QPACK, DER, TLS, JSON, or TOML. Those have
larger semantic surfaces or more canonical-helper gaps.

## Follow-On Targets

After `ws-frame`, the next safest adapters are:

- `encoding-core`: source in `crates/utility/edgerun-encoding/src/varint.rs`,
  `quic_varint.rs`, `crc32.rs`, and byteorder helpers; mostly scalar and
  fixed-buffer ABI.
- `encoding-text`: source in `crates/utility/edgerun-encoding/src/hex.rs` and
  `base64.rs`; requires careful output-capacity mapping.
- `http2-frame`: source in
  `crates/protocol/edgerun-protocols/src/http/http2/frame/mod.rs`; current
  Rust `Frame::from_bytes` allocates payload bytes, so a no-allocation header
  helper should be factored before using it as a thin adapter.

Avoid starting with HPACK/QPACK string codecs because the existing Rust helpers
allocate decoded `Vec<u8>` values. They are good parity oracles but not the
thinnest first adapter.

## Tests To Run

Before the replacement patch:

```bash
cargo test -p edgerun-protocols --features websocket websocket
node standards/runners/ws-frame-smoke.js
node standards/runners/codec-parity-smoke.js
node standards/runners/rust-parity-http-ws.js
```

After adding the Rust adapter:

```bash
cargo build -p <adapter-package> --target wasm32-unknown-unknown --release
cargo test -p edgerun-protocols --features websocket websocket
node standards/runners/ws-frame-smoke.js <path-to-rust-built-ws-frame.wasm-or-compatible-input>
node standards/runners/codec-parity-smoke.js
node standards/runners/rust-parity-http-ws.js
```

If the runners still require WAT input, add a temporary comparison command in
the implementation branch rather than changing the corpus or manifest in the
same patch. The replacement is ready only when the Rust-built Wasm returns the
same statuses, records, packed lengths, and header bytes as the current WAT for
the smoke and Rust parity cases.

## Open Cleanup Before Switching Authority

- `standards/components/manifests/codec-primitives/ws-frame.toml` currently
  lists `crates/protocol/edgerun-work/src/ws_channel.rs` as `parity_sources`;
  the canonical codec source for this target is actually
  `crates/protocol/edgerun-protocols/src/websocket.rs`.
- The current WAT validates more prefix policy than
  `decode_frame_prefix`; decide whether that policy belongs in a new canonical
  helper such as `validate_frame_prefix_for_codec_abi` or remains adapter-local.
- Keep the ABI version at `2` for this module. Do not reopen the older
  `standard-module-v1` wording mismatch during the first replacement patch.
