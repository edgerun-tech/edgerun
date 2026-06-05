# edgerun-hpack caller deletion queue

`crates/utility/edgerun-hpack` is deleted. Its byte-level behavior is owned by
the WAT codec primitives:

- `http-prefix-int.wat`
- `hpack-string.wat`
- `hpack-huffman.wat`
- `hpack-header-block.wat`
- `hpack-table-core.wat`

The removed crate had no Rust source left; it was only a tombstone manifest,
license, README, workspace member, and optional dependency target. Cargo health
is not an acceptance gate for this deletion pass.

## Evidence before deletion

`find crates/utility/edgerun-hpack -maxdepth 3 -type f | sort` returned only:

```text
crates/utility/edgerun-hpack/Cargo.toml
crates/utility/edgerun-hpack/LICENSE
crates/utility/edgerun-hpack/README.md
```

`rg -n "edgerun_hpack|edgerun-hpack" crates Cargo.toml standards` showed the
remaining live code references were direct compatibility imports, feature
gates, and HTTP/2 `Decoder`/`Encoder` reexports.

## Deleted direct crate hooks

- Root workspace member: `crates/utility/edgerun-hpack`.
- `edgerun-encoding` feature/dependency: `hpack = ["dep:edgerun-hpack"]`.
- `edgerun-protocols` optional dependency and `http2-server` dependency edge.
- `edgerun-http-client` optional dependency and `http2` dependency edge.
- `edgerun-node` optional dependency and `http2` dependency edge.
- Direct `From<edgerun_hpack::DecoderError>` conversion shims in node and HTTP
  client mirrors.
- Standards catalog/protocol implementation ownership now points at the WAT
  HPACK family instead of the deleted Rust crate.

## Remaining caller work

These paths still expect a Rust `Decoder`/`Encoder` surface and should be moved
to a runtime adapter that executes the WAT HPACK records:

```text
crates/protocol/edgerun-protocols/src/http/http2/server/headers.rs
crates/protocol/edgerun-protocols/src/http/http2/server/data.rs
crates/protocol/edgerun-protocols/src/http/http2/server/response.rs
crates/utility/edgerun-http-client/src/http/http2/mod.rs
crates/utility/edgerun-http-client/src/http/http2/client.rs
crates/utility/edgerun-http-client/src/http/server.rs
crates/node/edgerun-node/src/http/http2/mod.rs
crates/node/edgerun-node/src/http/http2/client.rs
crates/node/edgerun-node/src/http/server.rs
```

## Runtime adapter path

Create the adapter in `crates/protocol/edgerun-protocols/src/http/http2/`, not
as a resurrected utility crate. It should own HTTP/2 dynamic table state and use
the WAT primitives as record kernels:

1. `http-prefix-int.wat` for prefix integer decode/encode.
2. `hpack-string.wat` and `hpack-huffman.wat` for string and Huffman handling.
3. `hpack-header-block.wat` for header block structural scan records.
4. `hpack-table-core.wat` for dynamic-table size/accounting arithmetic.

The adapter should expose protocol-owned request/response header records, not
the old `edgerun_hpack::{Decoder, Encoder}` compatibility API.

## Next agent packets

1. Replace `server/headers.rs` HPACK imports with protocol-owned header block
   record structs and a WAT adapter boundary.
2. Replace response/data encoder call sites with WAT-backed header emit records.
3. Remove node and HTTP client mirror `hpack` reexports after the protocol
   adapter lands.
4. Update standards manifests and old parity runners whose historical
   `parity_sources` still point at deleted `crates/utility/edgerun-hpack/src/*`
   files.
