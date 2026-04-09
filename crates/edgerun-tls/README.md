# edgerun-tls

A dependency-free TLS 1.2/1.3 client implementation built exclusively with Rust's standard library and workspace crypto primitives.

## Features

- TLS 1.2 and TLS 1.3 support
- Full handshake protocol implementation
- Record layer encryption
- Certificate validation (X.509 parsing)
- Cipher suite negotiation
- Secure random number generation
- Session resumption support
- Zero external crate dependencies (only `std` + workspace crypto)

## Supported Cipher Suites

### TLS 1.3
- `TLS_AES_128_GCM_SHA256`
- `TLS_AES_256_GCM_SHA384`
- `TLS_CHACHA20_POLY1305_SHA256`

### TLS 1.2
- `TLS_ECDHE_ECDSA_WITH_AES_128_GCM_SHA256`
- `TLS_ECDHE_RSA_WITH_AES_128_GCM_SHA256`
- `TLS_ECDHE_ECDSA_WITH_AES_256_GCM_SHA384`
- `TLS_ECDHE_RSA_WITH_AES_256_GCM_SHA384`

## Usage

```rust
use edgerun_tls::TlsStream;
use std::net::TcpStream;
use std::io::{Read, Write};

// Connect to an HTTPS server
let stream = TcpStream::connect("example.com:443")?;
let mut tls = TlsStream::client(stream, "example.com")?;

// Send and receive encrypted data
tls.write_all(b"GET / HTTP/1.1\r\nHost: example.com\r\n\r\n")?;
let mut response = Vec::new();
tls.read_to_end(&mut response)?;
```

## Architecture

The implementation is split into several modules:

- **Handshake** - TLS handshake protocol (ClientHello, ServerHello, key exchange)
- **Record** - TLS record layer (fragmentation, encryption, MAC)
- **Cipher** - Cipher suite implementations (AES-GCM, ChaCha20-Poly1305)
- **KeyExchange** - ECDHE key exchange (P-256, X25519)
- **Certificate** - X.509 certificate parsing and validation
- **Alert** - TLS alert protocol for error handling
- **Prf** - Pseudorandom function for key derivation

## Security Notes

- This implementation uses constant-time operations where possible
- Private keys are zeroized after use
- No support for deprecated/insecure cipher suites
- Certificate validation is strict (no bypass options)

## License

MIT
