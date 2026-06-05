# edgerun-encoding deletion pass

Status: crossed out.

The final empty crate shell is deleted:

- `crates/utility/edgerun-encoding/Cargo.toml`
- `crates/utility/edgerun-encoding/src/lib.rs`

The root workspace member and root `[workspace.dependencies]` entry are also
removed. Remaining `edgerun_encoding::*` imports are intentional caller
demolition queues against deleted APIs; do not add a compatibility shim.

Rust source was deleted from `crates/utility/edgerun-encoding` because
equivalent portable codec primitives now live in WAT:

- `src/base64.rs` -> `encoding-text.wat`
- `src/chunked.rs` -> `http1-chunk-stream.wat`
- `src/crc32.rs` -> `encoding-core.wat`
- `src/hpack.rs` -> `http-prefix-int.wat`, `hpack-string.wat`,
  `hpack-huffman.wat`, `hpack-header-block.wat`
- `src/percent.rs` -> `percent-url-form.wat`
- `src/prefix_varint.rs` -> `http-prefix-int.wat`
- `src/quic_varint.rs` -> `encoding-core.wat`
- `src/varint.rs` -> `encoding-core.wat`
- `src/base32hex.rs` -> `encoding-base32hex.wat`
- `src/byteorder.rs` -> `endian-fixed.wat`
- `src/cstring.rs` -> `c-string.wat`
- `src/frame.rs` -> `length-frame.wat`
- `src/hex.rs` -> `hex-compat.wat`
- `src/ip.rs` -> `ipv4-net.wat`
- `src/kv.rs` -> `tag-list.wat`, `kv-scan.wat`
- `src/net.rs` -> `host-port.wat`
- `src/quoted_printable.rs` -> `mime-quoted-printable.wat`
- `src/rfc2822.rs` -> `rfc2822-date.wat`
- `src/rfc3339.rs` -> `time-rfc3339.wat`
- `src/string_field.rs` -> `length-field.wat`
- `src/tlv.rs` -> `generic-tlv.wat`
- `src/compression.rs` -> `gzip-member.wat`, `zlib-wrapper.wat`,
  `deflate-stored.wat`, `deflate-inflate.wat`
- `src/buf.rs`, `src/io.rs` -> deleted as Rust trait/error glue, with tiny
  owner-local copies where a caller still needs host-language scaffolding

The crate is no longer the canonical source for these codec behaviors. The WAT
modules and their runners are the proof path.

Proof commands:

```bash
node standards/runners/encoding-core-smoke.js
node standards/runners/encoding-text-smoke.js
node standards/runners/percent-url-form-smoke.js
node standards/runners/http1-chunk-stream-smoke.js
node standards/runners/http-prefix-int-smoke.js
node standards/runners/hpack-string-smoke.js
node standards/runners/hpack-huffman-smoke.js
node standards/runners/hpack-header-block-smoke.js
node standards/runners/rust-parity-encoding.js
```

Final-shell evidence:

```bash
rg -n "edgerun-encoding|edgerun_encoding" crates Cargo.toml standards --glob '*.rs' --glob 'Cargo.toml' --glob '*.md'
```

Expected result: no root workspace member/dependency entry and no
`crates/utility/edgerun-encoding` source file hits. Live Rust hits are caller
demolition targets only.

Do not use `cargo check` as the acceptance criterion for this deletion pass.
The deleted Rust modules intentionally break old Rust import paths until the
remaining crates move to WAT/runtime consumption or are deleted in their own
crate pass.
