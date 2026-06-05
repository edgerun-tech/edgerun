# Codec Primitive WAT Modules

This directory contains small deterministic WAT modules for encoder, decoder,
scanner, and framing primitives. They are external-standard/conformance
components, not an alternate EdgeRun internal wire protocol.

Each module must export:

```text
memory
proto_abi_version() -> i32
proto_standard_id() -> i32
```

`proto_abi_version` is `2`.

Most functions use fixed caller-provided buffers and return either a status
code or a packed `i64`:

```text
low32  = status
high32 = bytes_read | bytes_written | scalar result
```

Shared status values:

```text
0 ok
1 input_short
2 output_short
3 invalid
4 overflow_or_too_large
5 truncated
6 too_long_or_too_deep
```

Structured outputs are fixed-width records written to caller-provided memory.
Modules should return input offsets and lengths instead of copying payloads
where that keeps behavior simpler and more auditable.

Current module ids:

```text
300001 encoding-core
300002 encoding-text
300003 http1-scan
300004 ws-frame
300005 der-tlv
300006 tls-frame
300007 json-tape
300008 http1-lines
300009 http1-body
300010 http2-frame
300011 http-prefix-int
300012 http3-frame
300013 tls-vector
300014 tls-name
300015 pem-rfc7468
300016 json-scalar
300017 toml-scan
300018 hpack-huffman
300019 hpack-string
300020 qpack-string
300021 der-oid
300022 der-asn1-basic
300023 tls-clienthello
300024 tls-certificate-list
300025 dns-name
300026 dns-message-header
300027 percent-url-form
300071 sixel-decode
300073 integer-decimal
```

Current modules:

- `encoding-core.wat`: byte order, LEB128, QUIC varint, CRC32, Adler32.
- `encoding-text.wat`: strict hex and base64url no-pad byte codecs.
- `http1-scan.wat`: HTTP/1 token, header, CRLF, and chunk scanners.
- `http1-lines.wat`: HTTP/1 request, status, and header-line scanners.
- `http1-body.wat`: HTTP/1 content-length and transfer-coding body framing classifier.
- `http2-frame.wat`: HTTP/2 9-byte frame header encode/decode and type classification.
- `http-prefix-int.wat`: HPACK/QPACK prefix integer encode/decode.
- `hpack-huffman.wat`: HPACK Huffman byte-string decode and validation.
- `hpack-string.wat`: HPACK raw/Huffman string literal scanner and decoder.
- `qpack-string.wat`: QPACK raw/Huffman prefix-string scanner and decoder.
- `http3-frame.wat`: HTTP/3 frame header encode/decode/classification.
- `ws-frame.wat`: WebSocket frame-prefix, payload-length, header format/parse,
  mask, and close-payload helpers.
- `der-tlv.wat`: DER TLV tag/length helpers.
- `der-oid.wat`: DER OBJECT IDENTIFIER root, arc iteration, and small arc encoding helpers.
- `der-asn1-basic.wat`: DER INTEGER, BIT STRING, OCTET STRING, NULL, and SEQUENCE span scanners.
- `tls-frame.wat`: TLS record and handshake header helpers.
- `tls-vector.wat`: TLS u8/u16/u24 vectors, extension iteration, and ALPN iteration.
- `tls-name.wat`: TLS DNS name normalization and single-label wildcard matching.
- `tls-clienthello.wat`: TLS ClientHello body scan, extension lookup, SNI, and ALPN helpers.
- `tls-certificate-list.wat`: TLS 1.3 Certificate body and CertificateEntry scanners.
- `pem-rfc7468.wat`: PEM boundary, label, and base64 body scanners.
- `json-tape.wat`: minimal JSON token-tape scanner.
- `json-scalar.wat`: strict JSON string, number, unescape, and integer scalar helpers.
- `toml-scan.wat`: TOML line, key/value, table header, and scalar scanners.
- `dns-name.wat`: DNS uncompressed wire-name scanning and lowercase dotted-name emission.
- `dns-message-header.wat`: DNS 12-byte message header encode/decode and flag classification.
- `percent-url-form.wat`: strict percent encoding/decoding, form-url-encoded pair spans, and URI path/query/fragment scanning.
- `sixel-decode.wat`: SIXEL DCS payload decode to bounded RGBA raster bytes with repeat, color, movement, and dimension handling.
- `integer-decimal.wat`: bounded u64, i64, and split-limb u128 decimal ASCII formatting.
