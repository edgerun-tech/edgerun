# codex-client

Generic transport layer for Codex HTTP requests, retries, and streaming primitives.

`codex-client` is intentionally split into a portable core and an optional native transport adapter.

## Portable core

Build with `default-features = false` when compiling for browser/WASM or any runtime that provides its own HTTP implementation.

The portable core provides:

- `HttpTransport`, `Request`, `Response`, `StreamResponse`, and `ByteStream`.
- Retry utilities: `RetryPolicy`, `RetryOn`, `run_with_retry`, and `backoff`.
- `sse_stream`, which turns byte streams into raw SSE `data:` frames with idle timeout handling.
- `RequestTelemetry`, a lightweight trait that does not require OpenTelemetry propagation crates.

The portable core does not compile reqwest-shaped native client code, rustls, native root certificate loading, OpenTelemetry propagation, or process-global ChatGPT Cloudflare cookies.

```toml
codex-client = { workspace = true, default-features = false }
```

## Native transport

The default feature set enables `native-transport`, which exposes:

- `ReqwestTransport`
- `CodexHttpClient`
- `CodexRequestBuilder`
- custom CA helpers
- rustls provider initialization
- ChatGPT Cloudflare cookie-store support

```toml
codex-client = { workspace = true }
```

## Browser/WASM integration shape

Browser runtimes should implement `HttpTransport` using `fetch` and `ReadableStream`, then pass that transport into higher-level crates such as `codex-api` and `codex-model-provider`.

```rust
use std::sync::Arc;
use codex_client::{HttpTransport, Request, Response, StreamResponse, TransportError};

pub struct BrowserFetchTransport;

#[edgerun_async_trait::async_trait]
impl HttpTransport for BrowserFetchTransport {
    async fn execute(&self, request: Request) -> Result<Response, TransportError> {
        todo!("call browser fetch and collect the response body")
    }

    async fn stream(&self, request: Request) -> Result<StreamResponse, TransportError> {
        todo!("call browser fetch and adapt ReadableStream chunks")
    }
}

let transport: Arc<dyn HttpTransport> = Arc::new(BrowserFetchTransport);
```
