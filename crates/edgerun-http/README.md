# edgerun-http

A dependency-free HTTP client and type system built exclusively with Rust's standard library.

## Features

- HTTP/1.1 request and response types
- All standard HTTP methods (GET, POST, PUT, DELETE, PATCH, HEAD, OPTIONS)
- Header management with case-insensitive keys
- URI parsing and validation
- Basic HTTP client with TCP connectivity
- TLS support via native OS APIs (optional, feature-gated)
- Zero external dependencies

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

## License

MIT
