# edgerun-http

A dependency-free HTTP client and type system built exclusively with Rust's standard library.

## Features

- HTTP/1.1 request and response types
- HTTP/2 frame protocol implementation (RFC 9113) — from scratch, zero deps
- HPACK compression (RFC 7541)
- HTTP/2 server state machine with h2spec conformance testing
- HTTP/3 (QUIC) implementation in progress
- All standard HTTP methods (GET, POST, PUT, DELETE, PATCH, HEAD, OPTIONS)
- Header management with case-insensitive keys
- URI parsing and validation
- TLS support via native OS APIs (optional, feature-gated)
- Zero external HTTP library dependencies

## HTTP/2 Architecture

The HTTP/2 implementation is modularized for testability:

| Module | Purpose | Tests |
|--------|---------|-------|
| [`frame.rs`](src/http2/frame.rs) | 10 frame types with encode/decode + semantic validation | 12 |
| [`headers.rs`](src/http2/headers.rs) | Request header validation (RFC 9113 §8.1) | 29 |
| [`server.rs`](src/http2/server.rs) | I/O-agnostic state machine, returns `FrameAction` | 26 |
| [`settings.rs`](src/http2/settings.rs) | Settings with value validation | 6 |
| [`stream.rs`](src/http2/stream.rs) | Stream state machine (RFC 7540 §5.1) | 6 |
| [`flow_control.rs`](src/http2/flow_control.rs) | Connection/stream windows | 10 |

See [H2SPEC_ANALYSIS.md](H2SPEC_ANALYSIS.md) for the h2spec conformance analysis process.

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
# Build and run
cargo run --bin h2spec-server --features tls -- --port 8081

# Test against it
h2spec -h 127.0.0.1 -p 8081 -k
```

## License

MIT
