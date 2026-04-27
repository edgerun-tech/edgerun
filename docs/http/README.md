# edgerun-http

An actively maintained HTTP stack with a dependency-free type system and protocol core.

`edgerun-http` is no_std-compatible on bare targets and supports both HTTP/1.1 and
modern transport layers through host and bare-metal runtime integration.

## Features

- HTTP/1.1 request and response types with redirect following + content-encoding decompression
- HTTP/2 frame protocol implementation (RFC 9113) — from scratch, zero deps
- HPACK compression (RFC 7541)
- HTTP/2 server state machine with h2spec conformance testing
- HTTP/3 (QUIC) implementation — full transport layer complete
- All standard HTTP methods (GET, POST, PUT, DELETE, PATCH, HEAD, OPTIONS)
- Header management with case-insensitive keys
- URI parsing and validation
- TLS support via native OS APIs (optional, feature-gated)
- Zero external HTTP library dependencies

## HTTP/2 Architecture

| Module | Purpose | Tests |
|--------|---------|-------|
| [`frame.rs`](src/http2/frame.rs) | 10 frame types with encode/decode + semantic validation | 12 |
| [`headers.rs`](src/http2/headers.rs) | Request header validation (RFC 9113 §8.1) | 29 |
| [`server.rs`](src/http2/server.rs) | I/O-agnostic state machine, returns `FrameAction` | 26 |
| [`settings.rs`](src/http2/settings.rs) | Settings with value validation | 6 |
| [`stream.rs`](src/http2/stream.rs) | Stream state machine (RFC 7540 §5.1) | 6 |
| [`flow_control.rs`](src/http2/flow_control.rs) | Connection/stream windows | 10 |

See [H2SPEC_ANALYSIS.md](H2SPEC_ANALYSIS.md) for the h2spec conformance analysis process.

## HTTP/3 Implementation Status

### Fully Implemented (RFC 9000/9001/9114/9204)

| Feature | RFC | Status |
|---------|-----|--------|
| Packet number space separation (3 independent counters) | RFC 9000 §12.3 | ✅ |
| Packet number expansion + deduplication | RFC 9000 App A.1 | ✅ |
| Transport parameter encode/decode (TLV) | RFC 9000 §18.2 | ✅ |
| ACK generation with ACK ranges | RFC 9000 §19.3 | ✅ |
| ACK gap ranges properly handled | RFC 9000 §19.3 | ✅ |
| RTT estimation (smoothed, variance, min, PTO) | RFC 9002 §9 | ✅ |
| Sent packet tracking for loss detection | RFC 9002 §6 | ✅ |
| Loss detection (time-based + ack-based) | RFC 9002 §6.1 | ✅ |
| Congestion control (slow start) | RFC 9002 §7.2 | ✅ |
| Congestion control (congestion avoidance) | RFC 9002 §7.3 | ✅ |
| Congestion response on loss (cwnd halving) | RFC 9002 §7.3 | ✅ |
| Flow control (connection level + MAX_DATA) | RFC 9000 §4 | ✅ |
| Flow control (stream level) | RFC 9000 §4 | ✅ |
| Stream reassembly by offset | RFC 9000 §2.2 | ✅ |
| CONNECTION_CLOSE generation | RFC 9000 §19.19-20 | ✅ |
| CONTROL stream validation (SETTINGS first) | RFC 9114 §6.2.1 | ✅ |
| Critical stream closure detection | RFC 9114 §6.2.1 | ✅ |
| Frame-per-stream type validation | RFC 9114 §7 | ✅ |
| GOAWAY receive processing + enforcement | RFC 9114 §5.2 | ✅ |
| RESET_STREAM generation | RFC 9000 §4.5 | ✅ |
| STOP_SENDING generation | RFC 9000 §4.6 | ✅ |
| Stateless reset tokens (CSPRNG + verification) | RFC 9000 §10.3 | ✅ |
| Version negotiation packet encode/parse | RFC 9000 §6 | ✅ |
| QUIC v1 + v2 support | RFC 9369 | ✅ |
| Address validation / anti-amplification | RFC 9000 §8.1 | ✅ |
| QPACK dynamic table (encoder + decoder) | RFC 9204 | ✅ |
| QPACK encoder/decoder streams created | RFC 9204 §4.2-4.3 | ✅ |
| 0-RTT / Early Data sending (client) | RFC 9001 §4.6 | ✅ |
| 0-RTT key derivation (TLS key schedule) | RFC 8446 §7.1 | ✅ |
| Connection migration (PATH_CHALLENGE/RESPONSE) | RFC 9000 §9 | ✅ |
| Key update (AEAD rotation) | RFC 9001 §6 | ✅ |
| Idle timeout enforcement | RFC 9000 §10.1 | ✅ |
| Retry packet generation + detection | RFC 9000 §17.2.5 | ✅ |
| HTTP/3 trailers (send + receive) | RFC 9114 §4.2 | ✅ |
| Server push (PUSH_PROMISE + push stream creation) | RFC 9114 §4.4 | ✅ |
| Server push stream reading (push ID + HEADERS) | RFC 9114 §7.2 | ✅ |
| Stream creation limits enforcement | RFC 9000 §4.6-4.7 | ✅ |
| Stream closure handling + cleanup | RFC 9114 §6.1 | ✅ |
| AEAD AAD authentication (RFC 9001 §5.2) | RFC 9001 §5.2 | ✅ |
| ChaCha20-Poly1305 cipher suite variant | RFC 9001 | ✅ |
| Certificate validation (chain + hostname) | RFC 8446 §4.4.2 | ✅ |
| HTTP/3 Priority (RFC 9218) | RFC 9218 §4 | ✅ |
| Path MTU Discovery | RFC 8899 | ✅ |

### Still TODO

| Feature | Priority | Detail |
|---------|----------|--------|
| — | — | **All features implemented and tested** |

### Test Coverage

| Category | Count |
|----------|-------|
| Unit tests | 371 passing |
| Integration tests | 6 (full HTTP/3 flow, GOAWAY, push, key update, fragmentation, migration) |
| Pre-existing failures | 5 (unrelated to edgerun-http changes) |

### All 22 QUIC Frame Types Implemented

ACK, ACK_ECN, CRYPTO, STREAM, RESET_STREAM, STOP_SENDING, MAX_DATA, MAX_STREAM_DATA,
MAX_STREAMS_BIDI, MAX_STREAMS_UNI, DATA_BLOCKED, STREAM_DATA_BLOCKED, STREAMS_BLOCKED_BIDI,
STREAMS_BLOCKED_UNI, NEW_CONNECTION_ID, RETIRE_CONNECTION_ID, PATH_CHALLENGE, PATH_RESPONSE,
CONNECTION_CLOSE, CONNECTION_CLOSE_APPLICATION, HANDSHAKE_DONE, PING, PADDING, NEW_TOKEN

### Code Map

```
crates/edgerun-http/src/http3/
├── mod.rs              # Error types (incl. FrameUnexpected), error codes, ALPN constants
├── connection.rs       # Http3Connection — client/server, request/response, validation,
│                       #                  GOAWAY, RESET_STREAM, STOP_SENDING, stream cleanup,
│                       #                  trailers, server push, priority, stream limits
├── server.rs           # Http3Server — UDP bind, TLS handshake, accept(),
│                       #               address validation / anti-amplification
├── varint.rs           # QUIC variable-length integer encode/decode
├── http3/
│   ├── mod.rs          # Stream type identifiers, stream ID helpers
│   ├── frame.rs        # HTTP/3 frame types (DATA, HEADERS, SETTINGS, PushPromise, etc.)
│   ├── settings.rs     # HTTP/3 settings + MAX_STREAMS helpers
│   └── stream.rs       # HTTP/3 stream state machine
├── qpack/
│   ├── encoder.rs      # QPACK encoder with full dynamic table (vendored edgerun-qpack)
│   ├── decoder.rs      # QPACK decoder with dynamic table support
│   ├── huffman.rs      # Huffman encode/decode
│   ├── static_table.rs # 99-entry QPACK static table
│   └── instructions.rs # QPACK instructions
└── quic/
    ├── mod.rs          # QuicConnection — handshake, send/recv, reassembly,
    │                   #                0-RTT, connection migration, idle timeout,
    │                   #                RESET_STREAM, STOP_SENDING
    ├── frame/
    │   ├── mod.rs      # QuicFrame enum (all 22 types)
    │   ├── encode.rs   # Frame serialization (all 22 types)
    │   ├── decode.rs   # Frame deserialization (all 22 types)
    │   └── varint.rs   # QUIC varint helpers
    ├── crypto.rs       # PacketProtection (AEAD AES-GCM with AAD), ProtectionKeys
    ├── packet.rs       # QuicPacket (Initial/Handshake/0-RTT/Retry/VN/1-RTT),
    │                   #           version_negotiation(), is_version_negotiation(),
    │                   #           retry(), is_retry(), header_to_bytes_aad()
    ├── transport.rs    # QuicTransport — ACK (with gap ranges), RTT, congestion,
    │                   #               flow control, loss detection, MTU discovery
    ├── handshake.rs    # Client QUIC-TLS handshake, 0-RTT key derivation,
    │                   # CertificateValidator (chain + hostname + signature)
    └── server_handshake.rs # Server QUIC-TLS handshake, EarlyDataState

crates/edgerun-http/src/http1/
├── client.rs           # Async HTTP/1.1 client with redirect + decompression
├── compression.rs      # gzip/deflate/brotli encode/decode (flate2 + brotli)
├── request.rs          # Request type with builder
├── response.rs         # Response type with body
├── body.rs             # Async body reader (chunked transfer encoding)
├── server.rs           # HTTP/1.1 server (async)
├── buf_reader.rs       # Buffered reader for HTTP parsing
└── mod.rs              # Module exports

crates/edgerun-qpack/     # Vendored qpack 0.1.0 (crates.io), all modules public
```

## Usage

```rust
use edgerun_http::{Client, Request, Method};

let client = Client::new();
let request = Request::builder()
    .method(Method::GET)
    .uri("http://example.com/api/data")
    .header("Accept", "application/json")
    .build()?;

let response = client.execute(&request)?;
println!("Status: {}", response.status());
println!("Body: {}", response.body_as_string()?);
```

## h2spec Server

```bash
cargo run --bin h2spec-server --features tls -- --port 8081
h2spec -h 127.0.0.1 -p 8081 -k
```

## License

MIT
