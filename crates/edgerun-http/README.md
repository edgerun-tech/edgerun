# edgerun-http

A dependency-free HTTP client and type system built exclusively with Rust's standard library.

## Features

- HTTP/1.1 request and response types
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
| GOAWAY receive processing | RFC 9114 §5.2 | ✅ |
| RESET_STREAM generation | RFC 9000 §4.5 | ✅ |
| STOP_SENDING generation | RFC 9000 §4.6 | ✅ |
| Stateless reset tokens (CSPRNG + verification) | RFC 9000 §10.3 | ✅ |
| Version negotiation packet encode/parse | RFC 9000 §6 | ✅ |
| QUIC v1 + v2 support | RFC 9369 | ✅ |
| Address validation / anti-amplification | RFC 9000 §8.1 | ✅ |
| QPACK dynamic table support | RFC 9204 | ✅ |
| 0-RTT / Early Data sending | RFC 9001 §4.6 | ✅ |
| Connection migration (PATH_CHALLENGE/RESPONSE) | RFC 9000 §9 | ✅ |
| Key update (AEAD rotation) | RFC 9001 §6 | ✅ |
| Idle timeout enforcement | RFC 9000 §10.1 | ✅ |
| Retry packet generation + detection | RFC 9000 §17.2.5 | ✅ |
| HTTP/3 trailers (send + receive) | RFC 9114 §4.2 | ✅ |
| Server push (PUSH_PROMISE, MAX_PUSH_ID, CANCEL_PUSH) | RFC 9114 §4.4, §7.5-7.6 | ✅ |

### Still TODO

| Feature | Priority |
|---------|----------|
| 0-RTT / Early Data reception (server-side) | IMPORTANT |
| Full connection migration implementation | NICE |
| Path MTU Discovery | NICE |
| Key update (full TLS key schedule) | IMPORTANT |
| HTTP/3 Priority (RFC 9218) | NICE |
| Server push stream reading | IMPORTANT |
| Certificate validation (chain + hostname) | IMPORTANT |
| Multiple cipher suites (ChaCha20) | IMPORTANT |
| HTTP/3 trailers (full roundtrip) | NICE |
| CONNECT method tunneling | NICE |

### All 22 QUIC Frame Types Implemented

ACK, ACK_ECN, CRYPTO, STREAM, RESET_STREAM, STOP_SENDING, MAX_DATA, MAX_STREAM_DATA,
MAX_STREAMS_BIDI, MAX_STREAMS_UNI, DATA_BLOCKED, STREAM_DATA_BLOCKED, STREAMS_BLOCKED_BIDI,
STREAMS_BLOCKED_UNI, NEW_CONNECTION_ID, RETIRE_CONNECTION_ID, PATH_CHALLENGE, PATH_RESPONSE,
CONNECTION_CLOSE, CONNECTION_CLOSE_APPLICATION, HANDSHAKE_DONE, PING, PADDING, NEW_TOKEN

### Code Map

```
crates/edgerun-http/src/http3/
├── mod.rs              # Error types, error codes, ALPN constants
├── connection.rs       # Http3Connection — client/server, request/response, validation,
│                       #                  trailers, server push, GOAWAY, RESET_STREAM
├── server.rs           # Http3Server — UDP bind, TLS handshake, accept(),
│                       #               address validation / anti-amplification
├── varint.rs           # QUIC variable-length integer encode/decode
├── http3/
│   ├── mod.rs          # Stream type identifiers, stream ID helpers
│   ├── frame.rs        # HTTP/3 frame types (DATA, HEADERS, SETTINGS, etc.)
│   ├── settings.rs     # HTTP/3 settings + MAX_STREAMS helpers
│   └── stream.rs       # HTTP/3 stream state machine
├── qpack/
│   ├── encoder.rs      # QPACK encoder with dynamic table support + static table lookup
│   ├── decoder.rs      # QPACK decoder
│   ├── huffman.rs      # Huffman encode/decode
│   ├── static_table.rs # 99-entry QPACK static table
│   └── instructions.rs # QPACK instructions
└── quic/
    ├── mod.rs          # QuicConnection — handshake, send/recv, reassembly,
    │                   #                0-RTT, connection migration, idle timeout
    ├── frame/
    │   ├── mod.rs      # QuicFrame enum (all 22 types)
    │   ├── encode.rs   # Frame serialization (all 22 types)
    │   ├── decode.rs   # Frame deserialization (all 22 types)
    │   └── varint.rs   # QUIC varint helpers
    ├── crypto.rs       # PacketProtection (AEAD AES-GCM), ProtectionKeys
    ├── packet.rs       # QuicPacket (Initial/Handshake/0-RTT/Retry/VN/1-RTT),
    │                   #           version_negotiation(), is_version_negotiation(),
    │                   #           retry(), is_retry(), parse_supported_versions()
    ├── transport.rs    # QuicTransport — ACK, RTT, congestion, flow control,
    │                   #               loss detection, sent packet tracking
    ├── handshake.rs    # Client-side QUIC-TLS handshake
    └── server_handshake.rs # Server-side QUIC-TLS handshake
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
