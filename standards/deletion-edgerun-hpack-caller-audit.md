# edgerun-hpack caller retirement audit

This audit covers remaining callers and dependents of
`crates/utility/edgerun-hpack` after HPACK byte-level behavior moved into WAT
codec primitives.

Current WAT coverage:

- `standards/build/wasm/codec-primitives/http-prefix-int.wat`: HPACK and QPACK
  prefixed integer decode/encode.
- `standards/build/wasm/codec-primitives/hpack-huffman.wat`: HPACK Huffman
  decode and validation.
- `standards/build/wasm/codec-primitives/hpack-string.wat`: HPACK string
  literal scan/decode.
- `standards/build/wasm/codec-primitives/hpack-header-block.wat`: HPACK header
  block instruction and block scan.

## Delete compatibility reexport

These files only expose `edgerun-hpack` through compatibility layers. They do
not own HPACK behavior and should disappear once the crate is retired.

- `crates/utility/edgerun-encoding/src/lib.rs`
  - `#[cfg(feature = "hpack")] pub use edgerun_hpack::{Decoder, DecoderError,
    Encoder, HuffmanDecoder};`
  - Rationale: `edgerun-encoding` already documents that HPACK primitives are
    owned by WAT. This reexport keeps the retired crate reachable through an
    old compatibility surface.

- `crates/protocol/edgerun-protocols/src/http/http2/hpack.rs`
  - `pub use edgerun_hpack::{Decoder, Encoder};`
  - Rationale: this is the canonical HTTP/2 HPACK boundary today, but the file
    is only a reexport. Replace it with a WAT-backed HTTP/2 HPACK adapter or
    delete it after the HTTP runtime no longer needs Rust `Encoder`/`Decoder`.

- `crates/protocol/edgerun-protocols/src/http/http2/mod.rs`
  - `pub mod hpack;`
  - `pub use hpack::{Decoder, Encoder};`
  - Rationale: public compatibility exposure of the Rust stateful HPACK API.
    Keep HTTP/2 module ownership, but stop exporting retired crate types.

- `crates/node/edgerun-node/src/http/http2/mod.rs`
  - `pub mod hpack { pub use edgerun_protocols::http::http2::hpack::*; }`
  - `pub use hpack::{Decoder, Encoder};`
  - Rationale: node-local mirror of the protocol reexport. It should collapse
    when `edgerun-protocols` exposes a WAT-backed adapter or no public HPACK
    types.

- `crates/utility/edgerun-http-client/src/http/http2/mod.rs`
  - `pub mod hpack { pub use edgerun_protocols::http::http2::hpack::*; }`
  - `pub use hpack::{Decoder, Encoder};`
  - Rationale: HTTP client-local mirror of the protocol reexport. It should be
    deleted with the same replacement as the node HTTP/2 wrapper.

## Replace with WAT pipeline

These references exercise stateless HPACK byte codecs and are directly
replaceable by existing WAT primitives.

- `standards/runners/rust-parity-http-ws.js`
  - Rust oracle imports
    `edgerun_hpack::{encoder::encode_integer_into, huffman::{encode as
    hpack_huffman_encode, HuffmanDecoder}}`.
  - Rationale: this runner should switch to the established deleted-oracle
    pattern used by other retired crates. WAT already proves prefix integer,
    Huffman decode/validate, and string literal decode behavior.

- `standards/components/manifests/codec-primitives/http-prefix-int.toml`
  - `source_refs` include
    `crates/utility/edgerun-hpack/src/decoder.rs` and
    `crates/utility/edgerun-hpack/src/encoder.rs`.
  - Rationale: keep as historical parity refs until deletion note is committed,
    then point at `standards/deletion-edgerun-hpack.md` or the WAT runner.

- `standards/components/manifests/codec-primitives/hpack-huffman.toml`
  - `source_refs` include `crates/utility/edgerun-hpack/src/huffman.rs`.
  - Rationale: behavior is already in `hpack-huffman.wat`; manifest can become
    WAT-owned after crate retirement.

- `standards/components/manifests/codec-primitives/hpack-string.toml`
  - `source_refs` include
    `crates/utility/edgerun-hpack/src/decoder.rs` and
    `crates/utility/edgerun-hpack/src/huffman.rs`.
  - Rationale: behavior is already in `hpack-string.wat` plus
    `hpack-huffman.wat`; manifest can become WAT-owned after crate retirement.

- `standards/protocols/hpack.toml`
  - `implementation_crates = ["crates/utility/edgerun-hpack"]`
  - primitive sections list `implementation = ["crates/utility/edgerun-hpack"]`.
  - Rationale: update standards metadata to name the WAT modules for primitive
    behavior. Leave dynamic-table sections marked as HTTP runtime/table state
    until the stateful adapter is replaced.

## Leave for later HTTP runtime/table state

These callers depend on stateful `Decoder`/`Encoder` behavior: dynamic table
state, static table references, indexed/literal representation handling,
header-list construction, and table size updates. Existing WAT covers the byte
instructions but not the long-lived HTTP/2 compression context API used here.

- `crates/protocol/edgerun-protocols/src/http/http2/server/headers.rs`
  - Uses `Decoder` to decode request HEADERS, CONTINUATION, trailers, and full
    header blocks.
  - Uses `Encoder` through response helpers passed into complete-header
    processing.
  - Rationale: replace with an HTTP/2 compression-context adapter that feeds
    `hpack-header-block.wat`, resolves static/dynamic table entries, updates
    table state, and returns the same header-list shape expected by validators.

- `crates/protocol/edgerun-protocols/src/http/http2/server/response.rs`
  - Uses `Encoder::encode` for fixed response headers.
  - Rationale: can move to WAT header block emission or a minimal static-table
    response encoder once the HTTP runtime owns HPACK state.

- `crates/protocol/edgerun-protocols/src/http/http2/server/data.rs`
  - Accepts `&mut Encoder` only to emit a response after END_STREAM.
  - Rationale: indirect runtime dependency through `respond_with_200`; remove
    when response header encoding is WAT-backed.

- `crates/node/edgerun-node/src/http/server.rs`
  - Imports node-local `Decoder` and `Encoder` for HTTP/2 serving.
  - Rationale: leave until node HTTP/2 server is wired to the protocol-level
    WAT-backed compression context.

- `crates/node/edgerun-node/src/http/http2/client.rs`
  - Imports node-local `Decoder` and `Encoder` for HTTP/2 client state.
  - Rationale: leave until client request/response header compression state is
    moved behind the same WAT-backed adapter.

- `crates/node/edgerun-node/src/http/http2/mod.rs`
  - `impl From<edgerun_hpack::DecoderError> for Http2Error`.
  - Rationale: error conversion is tied to the Rust decoder type; replace with
    explicit HPACK/WAT status mapping when the adapter lands.

- `crates/utility/edgerun-http-client/src/http/server.rs`
  - Imports client-crate `Decoder` and `Encoder` for HTTP/2 serving.
  - Rationale: same runtime table-state dependency as `edgerun-node`.

- `crates/utility/edgerun-http-client/src/http/http2/client.rs`
  - Imports client-crate `Decoder` and `Encoder` for HTTP/2 client state.
  - Rationale: same runtime table-state dependency as `edgerun-node`.

- `crates/utility/edgerun-http-client/src/http/http2/mod.rs`
  - `impl From<edgerun_hpack::DecoderError> for Http2Error`.
  - Rationale: replace with adapter status mapping after Rust decoder removal.

## Manifest cleanup

These manifest references keep `edgerun-hpack` in the workspace or feature
graph. Remove them after the HTTP/2 runtime no longer imports Rust
`Encoder`/`Decoder`.

- `Cargo.toml`
  - Workspace member: `"crates/utility/edgerun-hpack"`.
  - Rationale: remove when the crate is tombstoned or deleted.

- `crates/utility/edgerun-hpack/Cargo.toml`
  - Package manifest for the retiring crate.
  - Rationale: delete or convert to a tombstone depending on the current crate
    retirement convention.

- `crates/utility/edgerun-encoding/Cargo.toml`
  - `hpack = ["dep:edgerun-hpack"]`
  - `edgerun-hpack = { version = "0.1", path = "../edgerun-hpack", optional =
    true }`
  - Rationale: compatibility feature should disappear with the reexport.

- `crates/protocol/edgerun-protocols/Cargo.toml`
  - `edgerun-hpack = { version = "0.1", path =
    "../../utility/edgerun-hpack", optional = true }`
  - `http2-server = ["http", "dep:edgerun-hpack"]`
  - Rationale: `http2-server` should depend on HTTP/2 runtime adapter code, not
    the retired crate.

- `crates/node/edgerun-node/Cargo.toml`
  - `http2 = ["http", "runtime", "edgerun-protocols/http2-server",
    "dep:edgerun-hpack"]`
  - `edgerun-hpack = { version = "0.1", path =
    "../../utility/edgerun-hpack", optional = true }`
  - Rationale: direct dependency is only needed for `DecoderError`; remove once
    errors are mapped through the protocol/runtime adapter.

- `crates/utility/edgerun-http-client/Cargo.toml`
  - `http2 = ["http", "edgerun-protocols/http2-server",
    "dep:edgerun-hpack"]`
  - `edgerun-hpack = { version = "0.1", path = "../edgerun-hpack", optional =
    true }`
  - Rationale: direct dependency is only needed for `DecoderError`; remove once
    errors are mapped through the protocol/runtime adapter.

## Reference counts

Collected with:

```bash
rg -l "edgerun_hpack|edgerun-hpack" crates Cargo.toml --glob '*.rs' --glob 'Cargo.toml' | wc -l
rg -l "crate::http::http2::hpack|http2::hpack|\\bhpack::|\\bhpack\\b" crates/protocol/edgerun-protocols crates/node/edgerun-node crates/utility/edgerun-http-client crates/utility/edgerun-encoding --glob '*.rs' --glob 'Cargo.toml' | wc -l
rg -l "edgerun_hpack|edgerun-hpack|crates/utility/edgerun-hpack" standards Cargo.toml crates --glob '!target' | wc -l
```
