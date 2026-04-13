# Public API Audit & DevEx Improvement Proposals

## Executive Summary

Analysis of **120+ crates** in the edgerun workspace reveals systemic API design issues that hurt developer experience, increase bug surface area, and violate the project's own design guidelines. Below are the findings organized by severity, with concrete proposed improvements.

---

## P0 — Critical (Fix Immediately)

### 1. `Result<T, String>` Error Types

**Affected crates:** `edgerun-crypto`, `edgerun-node`, `edgerun-linux-sysfs`, `edgerun-config`

**Problem:** Functions return `Result<T, String>` instead of proper error enums. Callers cannot match on error variants, cannot use `?` with different error types, and lose all structured error information.

**Current:**
```rust
// edgerun-crypto/src/lib.rs
pub fn aes256_gcm_encrypt(key: &[u8], nonce: &[u8], data: &[u8]) -> Result<Vec<u8>, String>
pub fn pem_decode(pem: &str) -> Result<(String, Vec<u8>), String>
```

**Proposed:**
```rust
// edgerun-crypto/src/error.rs
#[derive(Debug, thiserror::Error)]
pub enum CryptoError {
    #[error("invalid key length: expected {expected}, got {actual}")]
    InvalidKeyLength { expected: usize, actual: usize },
    #[error("AES-GCM encryption failed")]
    EncryptionFailed,
    #[error("AES-GCM decryption failed")]
    DecryptionFailed,
    #[error("invalid PEM format: {0}")]
    InvalidPem(String),
    #[error("invalid DER encoding: {0}")]
    InvalidDer(String),
    #[error("unsupported algorithm: {0}")]
    UnsupportedAlgorithm(String),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

pub fn aes256_gcm_encrypt(key: &[u8], nonce: &[u8], data: &[u8]) -> Result<Vec<u8>, CryptoError>
```

**Impact:** Enables `match err { CryptoError::DecryptionFailed => ..., _ => ... }`, proper `?` chaining, and testable error assertions.

---

### 2. `edgerun-crypto` Manual PKCS#8 DER Construction

**Problem:** `p256_signing_key_to_pem` manually pushes raw bytes to construct PKCS#8 DER. This is a correctness risk — one wrong length byte produces silently-invalid keys.

**Current (simplified):**
```rust
pub fn p256_signing_key_to_pem(key: &SigningKey) -> Result<String, String> {
    let mut der = Vec::new();
    der.push(0x30); // SEQUENCE
    der.push(0x84); // 4-byte length
    der.extend_from_slice(&len_bytes);
    // ... 50+ more raw byte pushes ...
}
```

**Proposed:** Use the `der` crate (already in the dependency tree via `p256`) or `pkcs8` crate's `EncodePrivateKey` trait:
```rust
use pkcs8::EncodePrivateKey;

pub fn p256_signing_key_to_pem(key: &SigningKey) -> Result<String, CryptoError> {
    let der = key.to_pkcs8_der().map_err(|_| CryptoError::InvalidDer("failed to encode".into()))?;
    let pem = pem_rfc7468::encode("PRIVATE KEY", der.as_bytes());
    Ok(pem)
}
```

---

### 3. `edgerun-crypto` — God Crate Exposing Entire RustCrypto Ecosystem

**Problem:** 982 lines, 25+ `pub use` re-exports of external crates. Violates the "single dependency boundary" goal — consumers can bypass edgerun-crypto's API and access raw RustCrypto internals through it.

**Current:**
```rust
pub use digest;
pub use sha2;
pub use p256;
pub use rsa;
pub use aes;
pub use aes_gcm;
// ... 20+ more ...
```

**Proposed:** Make re-exports `pub(crate)` and expose only the edgerun-curated API:
```rust
// Internal — not part of public API
pub(crate) use digest;
pub(crate) use sha2;
pub(crate) use p256;

// Public — curated, stable API
pub use error::CryptoError;
pub use hash::{sha256, sha384, sha512};
pub use hmac::{hmac_sha256, hmac_sha384};
pub use kdf::{hkdf_sha256, hkdf_sha384};
pub use aead::{AesGcmCipher, AesGcmNonce, AesGcmKey};
pub use p256_api::{P256SigningKey, P256PublicKey, P256Signature};
pub use x509::{X509Cert, SelfSignedCertBuilder};
pub use pem::{PemDecode, PemEncode};
```

---

## P1 — High (Fix This Sprint)

### 4. `RequestInit` — 15 Fields, No Builder, Stringly-Typed

**File:** `crates/edgerun-fetch/src/lib.rs`

**Current:**
```rust
pub struct RequestInit {
    pub method: String,
    pub url: String,
    pub headers: Vec<(String, String)>,
    pub body: Option<Vec<u8>>,
    pub referrer: String,           // Should be Option<Url>
    pub referrer_policy: String,    // Stringly-typed! ReferrerPolicy enum exists
    pub mode: RequestMode,
    pub credentials: RequestCredentials,
    pub cache: RequestCache,
    pub redirect: RequestRedirect,
    pub integrity: String,          // Should be Option<String>
    pub keep_alive: bool,
    pub signal: Option<()>,         // Unit type placeholder
    pub priority: RequestPriority,
    pub duplex: String,             // Stringly-typed!
}

// Usage: 15 lines of boilerplate
let init = RequestInit {
    method: "POST".into(),
    url: "https://api.example.com/data".into(),
    headers: vec![("Content-Type".into(), "application/json".into())],
    body: Some(json_bytes),
    referrer: "".into(),
    referrer_policy: "strict-origin-when-cross-origin".into(),
    mode: RequestMode::Cors,
    credentials: RequestCredentials::SameOrigin,
    cache: RequestCache::NoStore,
    redirect: RequestRedirect::Follow,
    integrity: "".into(),
    keep_alive: true,
    signal: None,
    priority: RequestPriority::Auto,
    duplex: "half".into(),
};
```

**Proposed:**
```rust
/// Builder for HTTP requests.
///
/// # Example
/// ```
/// let req = Request::builder("POST", "https://api.example.com/data")
///     .header("Content-Type", "application/json")
///     .body(json_bytes)
///     .cache(RequestCache::NoStore)
///     .redirect(RequestRedirect::Follow)
///     .build();
/// ```
pub struct Request {
    method: Method,
    url: Url,
    headers: HeaderMap,
    body: Option<Body>,
    mode: RequestMode,
    credentials: RequestCredentials,
    cache: RequestCache,
    redirect: RequestRedirect,
    priority: RequestPriority,
}

pub struct RequestBuilder {
    request: Request,
}

impl Request {
    pub fn builder(method: impl Into<Method>, url: impl TryInto<Url>) -> RequestBuilder { ... }
}

impl RequestBuilder {
    pub fn header(self, key: impl Into<HeaderName>, value: impl Into<HeaderValue>) -> Self { ... }
    pub fn headers(self, headers: HeaderMap) -> Self { ... }
    pub fn body(self, body: impl Into<Body>) -> Self { ... }
    pub fn cache(self, cache: RequestCache) -> Self { ... }
    pub fn redirect(self, redirect: RequestRedirect) -> Self { ... }
    pub fn credentials(self, credentials: RequestCredentials) -> Self { ... }
    pub fn priority(self, priority: RequestPriority) -> Self { ... }
    pub fn build(self) -> Result<Request, RequestBuildError> { ... }
}

// Sensible defaults — GET with no body, CORS mode, follow redirects
impl Default for RequestBuilder {
    fn default() -> Self {
        RequestBuilder {
            request: Request {
                method: Method::Get,
                url: Url::parse("about:blank").unwrap(),
                headers: HeaderMap::new(),
                body: None,
                mode: RequestMode::Cors,
                credentials: RequestCredentials::SameOrigin,
                cache: RequestCache::Default,
                redirect: RequestRedirect::Follow,
                priority: RequestPriority::Auto,
            },
        }
    }
}
```

**Also fix:** `Response.headers: ()` → `Response.headers: HeaderMap` and `Response.body: Option<Option<Vec<u8>>>` → `Response.body: Option<Body>` where `Body` has `async fn bytes(&mut self) -> Result<Vec<u8>>` and `async fn text(&mut self) -> Result<String>`.

---

### 5. Hardware Capability Crates — Repetitive Boilerplate

**Affected crates:** `edgerun-bluetooth`, `edgerun-nfc`, `edgerun-usb`, `edgerun-pci`, `edgerun-power`, `edgerun-display`, `edgerun-gpu`, `edgerun-input`, `edgerun-microphone`, `edgerun-speaker`, `edgerun-npu`

**Problem:** Every crate duplicates the identical pattern:
```rust
pub struct FooDeviceInfo {
    pub provider: String,
    pub instance_id: String,
    // ... 5-10 device-specific fields ...
}

pub fn default_foo_descriptor(provider: &str, instance_id: &str) -> CapabilityDescriptor { ... }
pub fn validate_foo_request(req: &FooRequest) -> Result<(), CapabilityError> { ... }

pub trait FooInventory: CapabilityProvider {
    fn list_foos(&self) -> Result<Vec<FooDeviceInfo>, CapabilityError>;
}
```

**Proposed:** Create a shared `edgerun-capability-inventory` crate with generic abstractions:

```rust
// crates/edgerun-capability-inventory/src/lib.rs

/// Generic device info trait — every hardware device implements this.
pub trait DeviceInfo: Clone + Debug + Send + Sync + 'static {
    fn provider(&self) -> &str;
    fn instance_id(&self) -> &str;
    fn descriptor(&self) -> CapabilityDescriptor;
}

/// Generic inventory trait — implemented by capability providers.
pub trait DeviceInventory<D: DeviceInfo>: CapabilityProvider {
    fn list_devices(&self) -> Result<Vec<D>, CapabilityError>;
    
    fn get_device(&self, instance_id: &str) -> Result<Option<D>, CapabilityError> {
        self.list_devices()?
            .into_iter()
            .find(|d| d.instance_id() == instance_id)
            .ok_or_else(|| CapabilityError::DeviceNotFound(instance_id.into()))
    }
}

/// Macro to generate boilerplate device info struct fields.
#[macro_export]
macro_rules! device_info_base {
    ($name:ident) => {
        pub struct $name {
            pub provider: String,
            pub instance_id: String,
        }

        impl $crate::DeviceInfo for $name {
            fn provider(&self) -> &str { &self.provider }
            fn instance_id(&self) -> &str { &self.instance_id }
        }
    };
}
```

Then each hardware crate becomes:
```rust
// edgerun-bluetooth/src/lib.rs — BEFORE: ~50 lines of boilerplate
edgerun_capability_inventory::device_info_base!(BluetoothDeviceInfo);

impl BluetoothDeviceInfo {
    pub fn add_fields(&mut self, name: String, address: MacAddr, ...) { ... }
}

// That's it. The macro handles provider/instance_id/descriptor.
```

---

### 6. `edgerun-rt::spawn()` — Hidden Global State

**File:** `crates/edgerun-rt/src/lib.rs`

**Problem:** `spawn()` is a free function that relies on hidden global/thread-local runtime state. Violates explicit state passing guideline.

**Current:**
```rust
pub fn spawn<F>(future: F) -> JoinHandle<F::Output>
where F: Future + Send + 'static, F::Output: Send + 'static { ... }
```

**Proposed:**
```rust
/// Explicit runtime handle — passed explicitly, not global.
#[derive(Clone)]
pub struct RuntimeHandle {
    inner: Arc<RuntimeInner>,
}

impl RuntimeHandle {
    pub fn spawn<F>(&self, future: F) -> JoinHandle<F::Output>
    where F: Future + Send + 'static, F::Output: Send + 'static { ... }
    
    pub fn spawn_blocking<F, R>(&self, f: F) -> JoinHandle<R>
    where F: FnOnce() -> R + Send + 'static, R: Send + 'static { ... }
}

/// Builder — explicit construction, no globals.
pub struct Builder {
    // ...
}

impl Builder {
    pub fn build(self) -> Result<Runtime, BuildError> { ... }
}

pub struct Runtime {
    handle: RuntimeHandle,
}

impl Runtime {
    pub fn handle(&self) -> &RuntimeHandle { &self.handle }
    pub fn block_on<F>(&self, future: F) -> F::Output { ... }
}

// Usage:
// let rt = Builder::new().build()?;
// let handle = rt.handle();
// handle.spawn(async { ... });
```

If a global default is still needed for ergonomics, provide it explicitly:
```rust
/// Get the current default runtime handle. Panics if no runtime has been set.
/// 
/// For explicit control, use `Runtime::handle()` instead.
pub fn current_handle() -> RuntimeHandle {
    CURRENT_HANDLE.with(|h| h.borrow().clone().expect("no runtime set"))
}

pub fn spawn<F>(future: F) -> JoinHandle<F::Output>
where F: Future + Send + 'static, F::Output: Send + 'static {
    current_handle().spawn(future)
}
```

---

### 7. `edgerun-fetch` — `Response` Has Broken Placeholder Types

**Current:**
```rust
pub struct Response {
    pub ok: bool,
    pub status: u16,
    pub status_text: String,
    pub headers: (),                    // Unit type!
    pub url: String,
    pub response_type: ResponseType,
    pub redirected: bool,
    pub body: Option<Option<Vec<u8>>>,  // Double Option — what does each mean?
}
```

**Proposed:**
```rust
pub struct Response {
    pub status: StatusCode,
    pub headers: HeaderMap,
    pub url: Url,
    pub response_type: ResponseType,
    pub redirected: bool,
    body: Option<Body>,  // Private — consumed via methods
}

impl Response {
    pub fn ok(&self) -> bool { self.status.is_success() }
    pub fn status_text(&self) -> &str { self.status.canonical_reason().unwrap_or("Unknown") }
    
    /// Consume the body as bytes. Returns error if body was already consumed.
    pub async fn bytes(mut self) -> Result<Vec<u8>, BodyError> { ... }
    
    /// Consume the body as UTF-8 text.
    pub async fn text(mut self) -> Result<String, BodyError> { ... }
    
    /// Consume and deserialize as JSON.
    pub async fn json<T: DeserializeOwned>(mut self) -> Result<T, FetchError> { ... }
}

/// Streaming body — can only be consumed once.
pub struct Body {
    inner: BodyInner,
}
```

---

### 8. Inconsistent Audio Sample Format Enums

**edgerun-microphone:**
```rust
pub enum MicrophoneSampleFormat {
    PcmS16Le, PcmS24Le, PcmS32Le, Float32Le,
}
```

**edgerun-speaker:**
```rust
pub enum SpeakerSampleFormat {
    PcmS16Le, PcmS24Le, PcmFloat32Le,  // Missing PcmS32Le, inconsistent naming
}
```

**Proposed:** Shared type in a common crate (e.g., `edgerun-audio` or `edgerun-core`):
```rust
/// PCM audio sample format — shared across microphone, speaker, and stream crates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AudioSampleFormat {
    /// Signed 16-bit little-endian PCM
    PcmS16Le,
    /// Signed 24-bit little-endian PCM (packed in 3 bytes)
    PcmS24Le,
    /// Signed 32-bit little-endian PCM
    PcmS32Le,
    /// 32-bit IEEE floating-point little-endian
    F32Le,
    /// 16-bit unsigned little-endian PCM
    PcmU16Le,
}

impl AudioSampleFormat {
    pub fn bytes_per_sample(self) -> usize {
        match self {
            Self::PcmS16Le | Self::PcmU16Le => 2,
            Self::PcmS24Le => 3,
            Self::PcmS32Le | Self::F32Le => 4,
        }
    }
    
    pub fn bits_per_sample(self) -> usize {
        self.bytes_per_sample() * 8
    }
}
```

---

## P2 — Important (Fix This Quarter)

### 9. God Types: `NodeStore`, `TlsStream`, `GpuInfo`

**`edgerun-storage::NodeStore`** manages: event log, blob store, SQLite indexes, credential store, object store, fetch queue, command replay cache, event-with-payload resolution, index rebuilding.

**Proposed split:**
```rust
pub struct EventLog { /* append-only file */ }
pub struct BlobStore { /* encrypted blobs */ }
pub struct ObjectStore { /* versioned objects */ }
pub struct CredentialStore { /* encrypted credentials */ }

/// High-level facade — composes the above.
pub struct NodeStore {
    event_log: EventLog,
    blob_store: BlobStore,
    object_store: ObjectStore,
    credential_store: CredentialStore,
    db: Arc<SqlitePool>,
}

impl NodeStore {
    // Convenience delegations — still available but clearly delegated
    pub fn events(&self) -> &EventLog { &self.event_log }
    pub fn blobs(&self) -> &BlobStore { &self.blob_store }
    pub fn objects(&self) -> &ObjectStore { &self.object_store }
    pub fn credentials(&self) -> &CredentialStore { &self.credential_store }
}
```

**`edgerun-gpu::GpuInfo`** — 25+ fields.

**Proposed split:**
```rust
pub struct GpuInfo {
    pub identity: GpuIdentity,    // vendor, device_id, vendor_id, name
    pub displays: Vec<GpuDisplay>, // DRM connector info
    pub capabilities: GpuCapabilities, // VRAM, clock, features
}

pub struct GpuIdentity {
    pub provider: String,
    pub instance_id: String,
    pub vendor: GpuVendor,
    pub vendor_id: u16,
    pub device_id: u16,
    pub device_name: String,
}

pub struct GpuCapabilities {
    pub vram_bytes: u64,
    pub max_clock_mhz: u32,
    pub supported_features: GpuFeatures, // bitflags
}
```

---

### 10. Glob Reexports Everywhere

**Problem:** `pub use submodule::*` makes it impossible to determine the public API by reading `lib.rs`. You must read every submodule.

**Affected crates:** `edgerun-config`, `edgerun-tpm`, `edgerun-storage`, and many more.

**Proposed:** Explicit re-exports only for items that are part of the public API:
```rust
// BAD
pub use types::*;
pub use parser::*;
pub use projector::*;
pub use importers::*;

// GOOD — lib.rs shows the complete public API at a glance
pub use types::{EdgerunConfig, NetworkConfig, WifiConfig, SecurityLevel};
pub use parser::parse_config;
pub use importers::{import_from_yaml, import_from_json};
// projector is internal — not re-exported
```

---

### 11. `edgerun-url` — Purely Stringly-Typed Data Bag

**Current:**
```rust
pub struct Url {
    pub href: String,
    pub origin: String,
    pub protocol: String,
    pub username: String,
    pub password: String,
    pub host: String,
    pub hostname: String,
    pub port: Option<String>,
    pub pathname: String,
    pub search: String,
    pub hash: String,
}
// No parsing. No validation. No construction methods.
```

**Proposed:**
```rust
/// A parsed, validated URL.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct Url {
    scheme: Scheme,
    username: String,       // Already validated as valid percent-encoded
    password: Option<String>,
    host: Option<Host>,     // Host::Domain(String) or Host::Ipv4/Ipv6
    port: Option<u16>,
    path: Vec<String>,      // Already validated path segments
    query: Option<QueryString>,
    fragment: Option<String>,
}

impl Url {
    /// Parse a URL string per WHATWG URL Standard.
    pub fn parse(input: &str) -> Result<Self, UrlParseError> { ... }
    
    /// Parse with a base URL for relative references.
    pub fn parse_with_base(input: &str, base: &Url) -> Result<Self, UrlParseError> { ... }
    
    /// Serialize back to a string.
    pub fn href(&self) -> String { ... }
    
    // Getters — no raw mutable access to internals
    pub fn scheme(&self) -> &Scheme { &self.scheme }
    pub fn host(&self) -> Option<&Host> { self.host.as_ref() }
    pub fn port(&self) -> Option<u16> { self.port }
    pub fn path(&self) -> &[String] { &self.path }
    pub fn query(&self) -> Option<&str> { self.query.as_ref().map(|q| q.as_str()) }
}
```

---

### 12. `edgerun-log` — No Structured Logging

**Current:**
```rust
log!(Info, "user {} logged in from {}", user_id, ip);
// Formats to flat string via format!()
```

**Proposed:**
```rust
// Structured key-value logging
log::info!("user logged in", user_id = %user_id, source_ip = %ip);
// Produces: {"level":"info","msg":"user logged in","user_id":"abc123","source_ip":"10.0.0.1"}

// Macro internals: parse key=value pairs, format structured output
// Keep the existing format!() syntax as a fallback for quick debugging
```

---

### 13. `edgerun-http` — Handler Trait Forces Boxing

**Current:**
```rust
pub trait Handler {
    fn handle(&self, req: Request) 
        -> std::pin::Pin<Box<dyn std::future::Future<Output = Response> + Send + '_>>;
}
```

**Proposed:** If `edgerun-rt` provides an async trait attribute (or uses `async-trait`):
```rust
#[async_trait]
pub trait Handler: Send + Sync {
    async fn handle(&self, req: Request) -> Response;
}

// Or with edgerun-rt's own async trait mechanism
```

---

### 14. Exposed Test/Benchmark Modules

**File:** `crates/edgerun-mesh/src/lib.rs`

```rust
pub mod router_tests;
pub mod router_benchmark;
```

**Proposed:**
```rust
#[cfg(test)]
mod router_tests;

#[cfg(test)]  
mod router_benchmark;
```

---

## P3 — Nice to Have

### 15. Missing `Default` on Request/Config Structs

All hardware request structs (`DisplayUpdateRequest`, `NpuWorkloadRequest`, `AudioPlaybackRequest`, `AudioCaptureRequest`) and config structs (`FlexContainer`, `GridContainer`) force the user to specify every field.

**Proposed:**
```rust
#[derive(Default)]
pub struct DisplayUpdateRequest {
    pub connector_id: u32,
    pub mode: Option<DisplayMode>,  // None = no change
    pub brightness: Option<u8>,     // None = no change
    pub enabled: Option<bool>,      // None = no change
}
```

---

### 16. `thread::sleep` Polling in v4l2 Camera

**Current:**
```rust
loop {
    thread::sleep(Duration::from_millis(10));
    // poll for frame
}
```

**Proposed:** Use `epoll`/`poll` on the V4L2 file descriptor with proper async integration:
```rust
use edgerun_rt::io::AsyncFd;

pub async fn capture_frame(&self) -> Result<Frame, CameraError> {
    let fd = AsyncFd::new(self.device_fd)?;
    fd.readable().await?;
    // dequeue buffer
}
```

---

### 17. `eprintln!` Debug Statements in TLS Code

**File:** `crates/edgerun-tls/src/lib.rs`

```rust
eprintln!("[CLIENT] transcript_hash: {:02x?}", hash);
```

**Proposed:** Use `edgerun-log` with debug/trace level, or remove entirely.

---

### 18. Proto Enums Cast to Raw `i32`

**File:** `crates/edgerun-capabilities/src/lib.rs`

All helper functions convert Rust enums to `i32` via `*v as i32`. Consumers must cast back manually.

**Proposed:** Keep the type-safe enum in the public API and only convert to `i32` at the proto serialization boundary:
```rust
pub struct CapabilityDescriptor {
    pub role: CapabilityRole,       // Type-safe enum, not i32
    pub modalities: Vec<Modality>,  // Type-safe enums, not Vec<i32>
    // ...
}

impl CapabilityDescriptor {
    pub fn to_proto(&self) -> proto::CapabilityDescriptor {
        proto::CapabilityDescriptor {
            role: self.role as i32,  // Only convert here
            modalities: self.modalities.iter().map(|m| *m as i32).collect(),
        }
    }
}
```

---

## Quick-Win Checklist

| Priority | Task | Effort |
|----------|------|--------|
| P0 | Add `CryptoError` enum to `edgerun-crypto` | Medium |
| P0 | Replace manual PKCS#8 DER with `pkcs8` crate | Medium |
| P0 | Add proper error enums to `edgerun-node`, `edgerun-config` | Small |
| P1 | `Request::builder()` pattern for `edgerun-fetch` | Medium |
| P1 | Fix `Response.headers: ()` and `body: Option<Option<>>` | Small |
| P1 | Create `AudioSampleFormat` shared type | Small |
| P1 | `RuntimeHandle` for explicit runtime passing in `edgerun-rt` | Medium |
| P2 | Split `NodeStore` into focused types | Large |
| P2 | Replace glob re-exports with explicit `pub use` | Medium (many crates) |
| P2 | `#[cfg(test)]` on test modules | Trivial |
| P2 | Split `GpuInfo` into identity/capabilities/displays | Small |
| P3 | Add `Default` to request/config structs | Small |
| P3 | Remove `eprintln!` from TLS | Trivial |
| P3 | Replace `thread::sleep` polling with async | Medium |

---

## Design Principles for Future APIs

Based on this audit, these principles should guide all new API design:

1. **No `Result<T, String>`** — Every crate with error conditions gets a proper `enum FooError`.
2. **No stringly-typed fields** — If a field has a known set of values, it's an `enum`.
3. **Builder pattern for >4 fields** — `Foo::builder().a(x).b(y).build()`.
4. **`Default` for configuration** — Every config/request struct implements `Default`.
5. **No global state** — Explicit handles, explicit construction, explicit passing.
6. **No god types** — Max 500 lines per struct impl, max 10 modules per crate.
7. **No glob re-exports** — `lib.rs` is the complete public API table.
8. **No `pub mod test`** — Tests are `#[cfg(test)]` and private.
9. **No `eprintln!`/`println!`** — Use structured logging via `edgerun-log`.
10. **No manual binary encoding** — Use `der`, `prost`, proper encoding crates.
11. **Extract shared patterns** — If 3+ crates duplicate a pattern, it's a shared crate.
12. **Type-safe at boundaries** — Proto `i32` conversions happen at serialization, not in the public API.
