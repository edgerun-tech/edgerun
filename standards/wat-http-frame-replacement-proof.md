# HTTP Frame WAT Replacement Proof

This proof checks whether the current HTTP/2 and HTTP/3 Rust frame-header paths
can be replaced by WAT codec primitives without changing behavior.

## Result

Header encode/decode parity is proven by:

```bash
node standards/runners/http-frame-wat-adapter-proof.js
```

The runner builds a temporary Rust oracle against local `edgerun-protocols`,
compiles and validates `http2-frame.wat` and `http3-frame.wat`, then compares
Rust frame output with WAT header output.

Actual Rust replacement is not landed yet. Inside this Rust workspace there is
no Rust-side WASM interpreter/runtime dependency such as `wasmtime`, `wasmer`,
or `wasmi`; however the sibling `/home/ken/edgerun-c` tree has an
EdgeRun-owned WASM runtime path. The next blocker is a bridge that loads these
codec `.wasm` modules through that runtime, copies input/output module memory,
and calls exports such as `http2_frame_header_decode`.

## Replacement Targets

HTTP/2:

- `crates/protocol/edgerun-protocols/src/http/http2/frame/mod.rs`
- `Frame::to_bytes`: replace the 9-byte frame header construction.
- `Frame::from_bytes`: replace the 9-byte header scan, frame-size check, and
  payload-availability check.

HTTP/3:

- `crates/protocol/edgerun-protocols/src/http/http3/frame.rs`
- `Http3Frame::to_bytes`: replace frame type and payload-length QUIC varint
  header construction.
- `Http3Frame::from_bytes`: replace frame type and payload-length varint scan,
  header-length calculation, classification, and payload-availability check.

## Adapter Notes

`http2-frame.wat` reports the HTTP/2 stream-id reserved bit separately from the
cleared stream id. Current Rust `Frame::from_bytes` stores the raw 32-bit stream
id, including the reserved bit. An exact adapter must reconstruct:

```text
raw_stream_id = (reserved << 31) | stream_id_cleared
```

or deliberately normalize the parsed stream id and update tests intentionally.

The WAT modules replace only framing. Payload-specific Rust enum construction,
HTTP/2 semantic validation, HTTP/3 SETTINGS entry parsing, and HTTP/3
payload-varint parsing remain Rust-owned.

## Runtime Decision

No Rust source was changed in this proof. A real replacement needs one of:

- a bridge to the existing `edgerun-c` WASM runtime ABI;
- an approved Rust-side WASM runtime adapter;
- a generated Rust shim from the WAT logic;
- compiling these primitives into the host through an existing build step.

Until then, the WAT modules are validated conformance components and JS-hosted
proof artifacts, not in-process Rust replacements.
