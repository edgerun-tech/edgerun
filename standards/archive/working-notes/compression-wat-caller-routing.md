# compression WAT caller routing

`edgerun-encoding/src/compression.rs` was a Rust facade over behavior that is
now covered by WAT primitives. The facade is deleted. Do not preserve the Rust
API shape and do not restore `edgerun-miniz-oxide`; route each caller to the
compact format primitives through a real host-side WASM invocation boundary.

Cargo health is not an acceptance gate for this queue.

## Current WAT primitives

- `gzip-member.wat` (`300064`): gzip header/trailer scan and emit.
- `zlib-wrapper.wat` (`300065`): zlib header/trailer scan and emit.
- `deflate-stored.wat` (`300066`): raw stored-block encode and scan.
- `deflate-inflate.wat` (`300067`): bounded raw inflate for stored, fixed
  Huffman, and dynamic Huffman blocks.
- `encoding-core.wat`: CRC32 and Adler32 checksum proof helpers.

## Replacement sequences

### VFS raw DEFLATE

Caller:

- `crates/authority/edgerun-vfs/src/packet.rs`
  - `prepare_object_seal_request_stored_deflate`
  - `unsealed_payload_to_object`
  - `unsealed_payload_to_object_with_deflate`

Deleted calls:

- `edgerun_encoding::compression::deflate_raw_compress(bytes, 6)`
- `edgerun_encoding::compression::deflate_raw_decompress_with_limit(payload, limit)`

Current status:

- The default VFS write path emits uncompressed objects through
  `prepare_object_seal_request`.
- Raw DEFLATE reads still fail closed through `unsealed_payload_to_object`.
- `VfsRawDeflateWatAdapter` is the owner-local host boundary for the future WAT
  invocation. Its write method must call `deflate-stored.wat`; its read method
  must call `deflate-inflate.wat`.
- `VfsRawDeflateWatUnavailable` is the default no-adapter implementation and
  returns `CompressFailed` / `DecompressFailed`.

Route:

1. For `prepare_object_seal_request_stored_deflate`, call
   `deflate-stored.wat::deflate_stored_encode`.
2. Keep the existing VFS transform/object hash checks as the authority layer.
3. For `unsealed_payload_to_object_with_deflate`, call
   `deflate-inflate.wat::deflate_inflate_raw(payload, out_cap = limit,
   out_limit = limit)`.
4. Reject unless status is `ok`, written byte count equals
   `transform.plaintext_len`, and `vfs_object_id(out)` matches
   `transform.plaintext_object_id`.

Deletion gate:

- Stored-block inflate is already enough for EdgeRun-produced VFS bytes if all
  new VFS compression uses `deflate_stored_encode`.
- Fixed/dynamic Huffman are only needed if VFS must keep accepting older or
  external raw-DEFLATE payloads.

### OCI gzip read/write

Callers:

- `crates/edgerun-oci/src/registry/tar_push.rs`
  - `gzip_bytes`
- `crates/edgerun-oci/src/tar_compression.rs`
  - `decompress_gzip_layer`
- Indirect read callers:
  - `crates/edgerun-oci/src/tar_layer.rs`
  - `crates/edgerun-oci/src/registry/layer.rs`
  - `crates/edgerun-oci/src/lib.rs`

Deleted calls:

- `edgerun_encoding::compression::gzip_compress(data, 6)`
- `edgerun_encoding::compression::gzip_decompress(data)`

Current write status:

- `crates/edgerun-oci/src/registry/tar_push.rs` no longer calls the deleted
  compression facade.
- `OciGzipWatAdapter` is the owner-local write boundary. It exposes exact hooks
  for `encoding-core.wat::crc32`,
  `gzip-member.wat::gzip_member_write_header`,
  `deflate-stored.wat::deflate_stored_encode`, and
  `gzip-member.wat::gzip_member_write_trailer`.
- `OciGzipWatUnavailable` is the default adapter and returns
  `io::ErrorKind::Unsupported`. OCI push therefore fails closed until the host
  supplies real WAT calls; it does not fake gzip output.

Write route:

1. Compute CRC32 over tar bytes with `encoding-core.wat::crc32`.
2. Emit gzip header with `gzip-member.wat::gzip_member_write_header`.
3. Emit raw stored DEFLATE with `deflate-stored.wat::deflate_stored_encode`.
4. Emit gzip trailer with `gzip-member.wat::gzip_member_write_trailer(crc32,
   isize)`.

Read route:

1. Scan with `gzip-member.wat::gzip_member_scan`.
2. Inflate the deflate span with `deflate-inflate.wat::deflate_inflate_raw`.
3. Compare the returned CRC32 and written size with the gzip trailer fields.
4. Map scan/inflate/checksum failures to `TarLayerApplyError::Decompress`.

Deletion gate:

- OCI write can move now with stored-block output.
- OCI read waits for dynamic Huffman because registry gzip layers commonly use
  dynamic Huffman blocks.

### HTTP compression

Caller:

- `crates/protocol/edgerun-protocols/src/http/http1/compression.rs`
  - `compress_body`
  - `decompress_body`
  - `compress_gzip`
  - `decompress_gzip`
  - `compress_deflate`
  - `decompress_deflate`

Indirect users:

- `crates/utility/edgerun-http-client/src/http/*`
- `crates/node/edgerun-node/src/http/*`

Current calls:

- `compression::gzip_compress`
- `compression::gzip_decompress`
- `compression::zlib_compress`
- `compression::zlib_decompress`

Gzip write route:

1. `gzip_member_write_header`.
2. `deflate_stored_encode`.
3. `gzip_member_write_trailer(crc32(body), body.len())`.

Gzip read route:

1. `gzip_member_scan`.
2. `deflate_inflate_raw` on the deflate span.
3. Verify CRC32 and ISIZE against the scan record.

Deflate write route:

1. `zlib_write_header(level)`.
2. `deflate_stored_encode`.
3. `zlib_write_trailer(adler32(body))`.

Deflate read route:

1. `zlib_member_scan`.
2. `deflate_inflate_raw` on the deflate span.
3. Verify Adler32 against the scan record.

Deletion gate:

- HTTP response compression can move now with stored-block output. Do not claim
  compression-ratio parity.
- HTTP response decompression waits for fixed and dynamic Huffman. Fixed covers
  common small payloads; dynamic is required for general public-web responses.

### Codelyzer APK ZIP method 8

Caller:

- `crates/edgerun-codelyzer/src/bin/apk_api_calls.rs`
  - `extract_zip_entry`

Current call:

- `edgerun_encoding::compression::deflate_raw_decompress_with_limit(compressed,
  entry.uncompressed_size)`

Route:

1. ZIP local/central-directory parsing stays local to the codelyzer.
2. For method `8`, call `deflate_inflate_raw` on the raw DEFLATE entry with
   `out_cap = entry.uncompressed_size` and `out_limit =
   entry.uncompressed_size`.
3. Reject unless status is `ok`, written byte count equals
   `entry.uncompressed_size`, and consumed byte count equals
   `entry.compressed_size`.

Deletion gate:

- Wait for dynamic Huffman before deleting the current method-8 path. APKs from
  normal toolchains are not guaranteed stored or fixed Huffman.

### tor bench gzip/zlib

Caller:

- `crates/apps/edgerun-tor-bench/src/tor.rs`
  - `decompress_body`

Current calls:

- `miniz_oxide::inflate::decompress_to_vec_zlib`
- `miniz_oxide::inflate::decompress_to_vec`

Route:

1. Keep content-encoding and magic-byte detection local to the bench.
2. For zlib, run `zlib_member_scan`, then `deflate_inflate_raw`, then Adler32
   verification.
3. For gzip, replace the manual `10..len-8` and partial `FEXTRA` slicing with
   `gzip_member_scan`, then `deflate_inflate_raw`, then CRC32/ISIZE
   verification.

Deletion gate:

- This is benchmark code, but it is useful as an external response fixture. Do
  not delete it yet. Route it after fixed/dynamic inflate so it exercises real
  public Tor directory gzip/zlib bodies without retaining miniz.

## Delete-or-keep decision

No localized product caller path was deleted in this pass.

- VFS and OCI are product/storage paths.
- HTTP compression is a shared protocol path.
- APK method 8 is useful analysis behavior.
- tor bench is demo/bench code, but its compression path is useful external
  fixture coverage for the WAT inflater.

The shared Rust facade and miniz crate are gone. Remaining caller edits are
adapter work, not compatibility-crate preservation work.

## Final deletion status

Done:

1. `deflate-inflate.wat` supports stored, fixed Huffman, and dynamic Huffman
   blocks with output-cap and output-limit enforcement.
2. gzip composition verifies `gzip_member_scan` + `deflate_inflate_raw` +
   CRC32/ISIZE.
3. zlib composition verifies `zlib_member_scan` + `deflate_inflate_raw` +
   Adler32.
4. `crates/utility/edgerun-encoding/src/compression.rs` is deleted and the
   `compression` feature no longer pulls `miniz_oxide`.
5. `crates/utility/edgerun-miniz-oxide` source and metadata are deleted, and the
   root workspace/dependency/patch entries were removed.

Still intentional:

- VFS no longer points at deleted Rust compression APIs. It has an explicit
  `VfsRawDeflateWatAdapter` boundary, emits uncompressed bytes by default, and
  rejects raw DEFLATE without a host WAT adapter.
- OCI, HTTP compression, APK ZIP method 8, and tor bench no longer point at
  deleted Rust compression APIs. They still fail closed or emit uncompressed
  bytes at the existing owner-local boundary until their host-side WAT
  invocation adapters land.

## Evidence

Command:

```bash
rg -n "edgerun_encoding::compression|miniz_oxide::|edgerun-miniz-oxide|miniz-oxide" crates standards --glob '*.rs' --glob 'Cargo.toml' --glob '*.md'
```

Before this doc was added, direct code references were:

```text
crates/edgerun-codelyzer/src/bin/apk_api_calls.rs:1399:            let out = edgerun_encoding::compression::deflate_raw_decompress_with_limit(
crates/authority/edgerun-vfs/src/packet.rs:571:            edgerun_encoding::compression::deflate_raw_decompress_with_limit(payload, limit)
crates/authority/edgerun-vfs/src/packet.rs:770:    edgerun_encoding::compression::deflate_raw_compress(bytes, 6)
crates/utility/edgerun-miniz-oxide/src/lib.rs:13:use miniz_oxide::inflate::decompress_to_vec;
crates/utility/edgerun-miniz-oxide/src/lib.rs:14:use miniz_oxide::deflate::compress_to_vec;
crates/edgerun-oci/src/tar_compression.rs:10:    edgerun_encoding::compression::gzip_decompress(data)
crates/apps/edgerun-tor-bench/src/tor.rs:944:        Some("deflate") | None if is_zlib => miniz_oxide::inflate::decompress_to_vec_zlib(data)
crates/apps/edgerun-tor-bench/src/tor.rs:952:                return miniz_oxide::inflate::decompress_to_vec(&data[12 + xlen..data.len() - 8])
crates/apps/edgerun-tor-bench/src/tor.rs:955:            miniz_oxide::inflate::decompress_to_vec(&data[10..data.len() - 8])
crates/apps/edgerun-tor-bench/src/tor.rs:963:                miniz_oxide::inflate::decompress_to_vec_zlib(data)
crates/apps/edgerun-tor-bench/src/tor.rs:969:                miniz_oxide::inflate::decompress_to_vec(&data[10..data.len() - 8])
crates/utility/edgerun-encoding/src/compression.rs:19:    miniz_oxide::deflate::compress_to_vec(data, level)
crates/utility/edgerun-encoding/src/compression.rs:23:    miniz_oxide::inflate::decompress_to_vec(data).map_err(|_| CompressionError::InvalidDeflate)
crates/utility/edgerun-encoding/src/compression.rs:30:    miniz_oxide::inflate::decompress_to_vec_with_limit(data, limit).map_err(|error| {
crates/utility/edgerun-encoding/src/compression.rs:32:            miniz_oxide::inflate::TINFLStatus::HasMoreOutput => CompressionError::LimitExceeded,
crates/utility/edgerun-encoding/src/compression.rs:39:    miniz_oxide::deflate::compress_to_vec_zlib(data, level)
crates/utility/edgerun-encoding/src/compression.rs:43:    miniz_oxide::inflate::decompress_to_vec_zlib(data).map_err(|_| CompressionError::InvalidDeflate)
crates/edgerun-oci/src/registry/tar_push.rs:187:    Ok(edgerun_encoding::compression::gzip_compress(data, 6))
```

After the final deletion pass, central facade/core references and product source
call expressions are gone. Remaining documentation references are intentional
historical evidence and adapter-boundary notes for the next host-side WAT
invocation work.
