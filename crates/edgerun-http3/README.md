# edgerun-http3

A complete, dependency-free HTTP/3 implementation with QUIC transport, QPACK header compression, and stream multiplexing — built exclusively with Rust's standard library.

## Features

- **HTTP/3 Protocol** (RFC 9114) — Request/response semantics over QUIC
- **QUIC Transport** (RFC 9000) — UDP-based reliable transport with congestion control, loss recovery, connection migration
- **QPACK Header Compression** (RFC 9204) — Improved HPACK designed for QUIC's out-of-order delivery, with dynamic table blocking prevention
- **Stream Multiplexing** — Unidirectional and bidirectional streams with HTTP/3 stream types
- **HTTP/3 Frames** — DATA, HEADERS, CANCEL_PUSH, MAX_PUSH_ID, GOAWAY, SETTINGS, PUSH_PROMISE
- **0-RTT Connection** — Early data support for reduced latency
- **Connection Migration** — Seamless handover across network changes
- **Zero External Dependencies** — Only `std`

## Architecture

```
edgerun-http3/
├── lib.rs            # Error types, HTTP/3 constants, version negotiation
├── quic/
│   ├── mod.rs        # QUIC connection management
│   ├── packet.rs     # QUIC packet format (Initial, Handshake, 0-RTT, 1-RTT)
│   ├── frame.rs      # QUIC frames (STREAM, ACK, CRYPTO, PING, PADDING, etc.)
│   ├── transport.rs  # Loss recovery, congestion control, flow control
│   └── crypto.rs     # QUIC crypto handshake (TLS 1.3 over QUIC)
├── qpack/
│   ├── mod.rs        # QPACK encoder/decoder
│   ├── static_table.rs # 99-entry static table
│   ├── encoder.rs    # QPACK encoder with dynamic table management
│   ├── decoder.rs    # QPACK decoder with instruction stream
│   └── instructions.rs # QPACK instructions (Indexed, Literal, etc.)
├── http3/
│   ├── mod.rs        # HTTP/3 connection and request/response
│   ├── frame.rs      # HTTP/3 frame types (DATA, HEADERS, SETTINGS, etc.)
│   ├── stream.rs     # HTTP/3 stream types (control, request, push, unidirectional)
│   └── settings.rs   # HTTP/3 settings (MAX_TABLE_CAPACITY, etc.)
└── connection.rs     # Top-level HTTP/3 connection (QUIC + HTTP/3 + QPACK)
```

## Usage

```rust
use edgerun_http3::{Http3Connection, QpackEncoder, QpackDecoder};
use edgerun_http3::http3::frame::Http3FrameType;
use edgerun_http3::quic::QuicConnection;

// Create QPACK encoder/decoder
let mut qpack_encoder = QpackEncoder::new();
let mut qpack_decoder = QpackDecoder::new();

// Encode request headers
let header_block = qpack_encoder.encode(&[
    (":method", "GET"),
    (":scheme", "https"),
    (":path", "/api/data"),
    (":authority", "example.com"),
])?;

// Create HTTP/3 connection over UDP socket
// let socket = std::net::UdpSocket::bind("0.0.0.0:0")?;
// let mut conn = Http3Connection::connect(socket, "example.com:443")?;

// Send request
// let stream_id = conn.send_request(header_block, None)?;

// Receive response
// while let Some(frame) = conn.poll_stream(stream_id)? {
//     match frame.frame_type {
//         Http3FrameType::Headers => { /* response headers */ }
//         Http3FrameType::Data => { /* response body */ }
//         _ => {}
//     }
// }
```

## HTTP/3 Frame Types

| Type | Name | Purpose |
|------|------|---------|
| 0x00 | DATA | Request/response body |
| 0x01 | HEADERS | Request/response headers (QPACK-encoded) |
| 0x03 | CANCEL_PUSH | Cancel a promised push |
| 0x04 | SETTINGS | Configuration parameters |
| 0x05 | PUSH_PROMISE | Server push notification |
| 0x07 | MAX_PUSH_ID | Maximum push ID |
| 0x08 | GOAWAY | Graceful shutdown |
| 0x09 | STREAMS_BLOCKED | Stream limit reached |

## QPACK Static Table

The static table contains 99 common header fields including:
- Pseudo-headers (`:method`, `:path`, `:scheme`, `:status`)
- Common methods (`GET`, `POST`, `HEAD`, `PUT`, `DELETE`, etc.)
- Common status codes (`200`, `201`, `204`, `301`, `302`, `400`, `404`, `500`, etc.)
- Common headers (`content-type`, `user-agent`, `cookie`, `accept`, etc.)

## QUIC Packet Types

| Type | Value | Purpose |
|------|-------|---------|
| Initial | 0x00 | Connection establishment, TLS handshake |
| 0-RTT | 0x01 | Early data (if previously connected) |
| Handshake | 0x02 | TLS handshake completion |
| Retry | 0x03 | Server retry (address validation) |
| 1-RTT | N/A | Encrypted application data |

## License

MIT
