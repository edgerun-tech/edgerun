# Final `edgerun-encoding` Deletion Callers

Status: crossed out.

`crates/utility/edgerun-encoding` has no remaining useful source. The final
empty shell is deleted:

- `crates/utility/edgerun-encoding/Cargo.toml`
- `crates/utility/edgerun-encoding/src/lib.rs`

The root workspace member and root `[workspace.dependencies]` entry are also
deleted. Cargo health is not an acceptance gate for this branch. Remaining
`edgerun_encoding::*` imports are caller-demolition targets against APIs whose
portable behavior already moved to WAT or whose Rust-only glue moved to owning
callers.

## What Was Drained

Portable behavior now lives in WAT:

- base64/base64url: `encoding-text.wat`, `encoding-base64.wat`
- base32hex: `encoding-base32hex.wat`
- byteorder fixed reads/writes: `endian-fixed.wat`
- C strings and C multi-strings: `c-string.wat`
- CRC32, Adler32, varint, QUIC varint: `encoding-core.wat`
- percent/form/query scanning: `percent-url-form.wat`
- chunked/body framing: `http1-chunk-stream.wat`, `http1-body.wat`
- HPACK prefix/string/huffman/header/table: HPACK WAT family
- hex/MAC/BDADDR compatibility scans: `hex-compat.wat`
- IPv4 network arithmetic: `ipv4-net.wat`
- host/port scan: `host-port.wat`
- KV/list/tag scans: `tag-list.wat`, `kv-scan.wat`
- quoted printable: `mime-quoted-printable.wat`
- RFC 2822/RFC 3339 dates: `rfc2822-date.wat`, `time-rfc3339.wat`
- length fields/frames and TLV: `length-field.wat`, `length-frame.wat`,
  `generic-tlv.wat`
- gzip/zlib/raw DEFLATE: `gzip-member.wat`, `zlib-wrapper.wat`,
  `deflate-stored.wat`, `deflate-inflate.wat`

Rust-only glue was not ported:

- `buf.rs`: QPACK-local cursor/trait scaffolding.
- `io.rs`: IMAP/SMTP/virtual-disk-local error/read/write scaffolding.

## Remaining Live Caller Queues

These are the highest-value remaining caller families found by the final
`rg -n "edgerun-encoding|edgerun_encoding"` scan. They should be removed by
WAT adapters, owner-local fixed records, or direct deletion of obsolete paths.
Do not recreate `edgerun-encoding`.

### Manifest hooks

Many crate manifests still name `edgerun-encoding = { workspace = true }`.
Because the root workspace dependency is gone, those are intentionally broken
until each owner removes or reroutes its imports. Start with crates whose Rust
imports were already cleared by prior agents, then move by domain.

### Base64 and base64url

Primary owners:

- ACME/OAuth/JWT and Codex debug/API payloads.
- TLS cert/PEM helpers.
- SMTP auth/message builder and terminal OSC payloads.
- WebSocket accept paths and tungstenite compatibility.

Use `encoding-base64.wat`, `encoding-text.wat`, `pem-rfc7468.wat`, and
`ws-accept.wat`. Keep protocol-specific semantics in the owning protocol.

### Byteorder

Primary owners:

- storage block/FAT parsing.
- TLS, QUIC, TFTP, HTTP/2 frame, Bluetooth, TPM/YubiKey, OCI ELF, and node
  hardware adapters.

Use `endian-fixed.wat` at host/WAT boundaries or move tiny endian reads into the
owning protocol while caller routing is still in progress. Do not restore a
shared Rust byteorder crate.

### Hex and hardware identifiers

Primary owners:

- Bluetooth GATT/MGMT, Linux sysfs/wifi/evdev, TPM/core crypto, OCI layer
  accounting, node hardware reports.

Use `hex-compat.wat` or owner-local identifier formatting policy.

### C strings and TLV

Primary owners:

- V4L2/ALSA/CEC/Goodix/Bluetooth hardware adapters.
- YubiKey and TPM wire helpers.

Use `c-string.wat`, `generic-tlv.wat`, or caller-owned fixed parsing.

### Dates and time

Primary owners:

- deleted `edgerun-time` callers
- `edgerun-core` utility reexports
- email/date surfaces

Use `time-rfc3339.wat` and `rfc2822-date.wat`; do not keep `edgerun-time` as a
thin reexport of a deleted Rust compatibility crate.

### Compression adapters

Primary owners:

- VFS, OCI gzip read/write, HTTP compression, APK ZIP method 8, tor bench.

`edgerun_encoding::compression` is gone. These paths must call
`gzip-member.wat`, `zlib-wrapper.wat`, `deflate-stored.wat`, and
`deflate-inflate.wat` through real host-side WASM invocation or fail closed at
the owner boundary.

## Next Agent Packets

1. Remove manifest hooks for crates whose `edgerun_encoding` imports are already
   gone.
2. Keep `edgerun-time` deleted. Replace remaining caller imports with
   `time-rfc3339.wat` ownership or owner-local host clock/date helpers.
3. Convert TLS/HTTP2/TFTP/QUIC byteorder call sites to `endian-fixed.wat`
   adapters or owner-local fixed reads.
4. Convert hardware C-string/hex/TLV call sites to `c-string.wat`,
   `hex-compat.wat`, and `generic-tlv.wat`.
5. Convert base64-heavy protocol paths: WebSocket accept, PEM/certs, SMTP auth,
   terminal OSC, Codex payloads.
6. Finish compression caller adapters for OCI, HTTP, APK ZIP, and tor bench.

## Evidence Command

```bash
rg -n "edgerun-encoding|edgerun_encoding" crates Cargo.toml standards --glob '*.rs' --glob 'Cargo.toml' --glob '*.md'
```

Expected shape:

- no root workspace member for `crates/utility/edgerun-encoding`;
- no root workspace dependency named `edgerun-encoding`;
- no `crates/utility/edgerun-encoding` source file hits;
- live Rust hits only in caller-demolition queues;
- standards hits documenting deleted behavior and routing plans.
