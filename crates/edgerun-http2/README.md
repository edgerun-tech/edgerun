# edgerun-http2

A complete, dependency-free HTTP/2 implementation with HPACK compression, frame protocol, stream multiplexing, and flow control — built exclusively with Rust's standard library.

## Features

- **HTTP/2 Frame Protocol** — All 10 frame types: DATA, HEADERS, PRIORITY, RST_STREAM, SETTINGS, PUSH_PROMISE, PING, GOAWAY, WINDOW_UPDATE, CONTINUATION
- **HPACK Header Compression** (RFC 7541) — Static table (61 entries), dynamic table with eviction, integer encoding, Huffman coding support
- **Stream Multiplexing** — Full state machine (Idle → Open → Half-Closed → Closed), priority dependencies, concurrent stream management
- **Flow Control** (RFC 7540 §5.2) — Connection and stream level, window management, automatic WINDOW_UPDATE
- **SETTINGS Negotiation** — All 6 standard settings with validation and acknowledgment
- **Connection Management** — Preface exchange, PING/pong, graceful shutdown (GOAWAY), error handling
- **Zero External Dependencies** — Only `std`

## Architecture

```
edgerun-http2/
├── lib.rs            # Error types, error codes, connection preface
├── frame.rs          # Frame protocol: 10 frame types with parse/serialize
├── hpack.rs          # HPACK: static/dynamic tables, encoder/decoder
├── settings.rs       # SETTINGS: 6 parameters with validation
├── flow_control.rs   # Flow control: window management per stream/connection
├── stream.rs         # Stream state machine, priority, multiplexing
└── connection.rs     # Connection: preface, frame dispatch, I/O
```

## Usage

```rust
use edgerun_http2::{Connection, Frame, Settings, Encoder, Decoder};
use edgerun_http2::frame::{HeadersFrame, DataFrame, SettingsFrame};

// Create HPACK encoder/decoder
let mut encoder = Encoder::new();
let mut decoder = Decoder::new();

// Encode request headers
let mut header_block = Vec::new();
header_block.extend(encoder.encode_header(":method", "GET")?);
header_block.extend(encoder.encode_header(":scheme", "https")?);
header_block.extend(encoder.encode_header(":path", "/api/data")?);
header_block.extend(encoder.encode_header(":authority", "example.com")?);

// Create connection (wraps TcpStream)
// let tcp = TcpStream::connect("example.com:443")?;
// let mut conn = Connection::client(tcp)?;

// Send request
// let stream_id = conn.send_request(header_block, None)?;

// Poll for frames
// while let Some(frame) = conn.poll()? {
//     match frame.frame_type {
//         FrameType::Headers => { /* response headers */ }
//         FrameType::Data => { /* response body */ }
//         FrameType::Goaway => { /* connection closing */ }
//         _ => {}
//     }
// }
```

## Frame Types

| Type | Value | Purpose |
|------|-------|---------|
| DATA | 0x0 | Request/response body |
| HEADERS | 0x1 | Request/response headers |
| PRIORITY | 0x2 | Stream priority updates |
| RST_STREAM | 0x3 | Stream termination |
| SETTINGS | 0x4 | Configuration parameters |
| PUSH_PROMISE | 0x5 | Server push notification |
| PING | 0x6 | Connection liveness check |
| GOAWAY | 0x7 | Graceful shutdown |
| WINDOW_UPDATE | 0x8 | Flow control |
| CONTINUATION | 0x9 | Header block continuation |

## HPACK Static Table

The static table contains 61 common header fields including:
- Pseudo-headers (`:method`, `:path`, `:scheme`, `:status`)
- Common methods (`GET`, `POST`)
- Common paths (`/`, `/index.html`)
- Common status codes (`200`, `204`, `404`, `500`)
- Common headers (`content-type`, `user-agent`, `cookie`, etc.)

## License

MIT
