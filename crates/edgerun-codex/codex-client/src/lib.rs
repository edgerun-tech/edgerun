#[cfg(feature = "native-transport")]
mod chatgpt_cloudflare_cookies;
#[cfg(feature = "native-transport")]
mod chatgpt_hosts;
#[cfg(feature = "native-transport")]
#[path = "custom_ca_edgerun_tls.rs"]
mod custom_ca;
#[cfg(feature = "native-transport")]
mod default_client;
mod error;
mod request;
mod retry;
#[cfg(feature = "native-transport")]
pub mod rustls_provider;
mod sse;
mod telemetry;
mod transport;

#[cfg(feature = "native-transport")]
pub use crate::chatgpt_cloudflare_cookies::with_chatgpt_cloudflare_cookie_store;
#[cfg(feature = "native-transport")]
pub use crate::chatgpt_hosts::is_allowed_chatgpt_host;
#[cfg(feature = "native-transport")]
pub use crate::custom_ca::BuildCustomCaTransportError;
/// Test-only subprocess hook for custom CA coverage.
///
/// This stays public only so the `custom_ca_probe` binary target can reuse the shared helper. It
/// is hidden from normal docs because ordinary callers should use
/// [`build_reqwest_client_with_custom_ca`] instead.
#[cfg(feature = "native-transport")]
#[doc(hidden)]
pub use crate::custom_ca::build_reqwest_client_for_subprocess_tests;
#[cfg(feature = "native-transport")]
pub use crate::custom_ca::build_reqwest_client_with_custom_ca;
#[cfg(feature = "native-transport")]
pub use crate::custom_ca::maybe_build_edgerun_tls_client_config_with_custom_ca;
#[cfg(feature = "native-transport")]
pub use crate::custom_ca::maybe_build_rustls_client_config_with_custom_ca;
#[cfg(feature = "native-transport")]
pub use crate::default_client::CodexHttpClient;
#[cfg(feature = "native-transport")]
pub use crate::default_client::CodexRequestBuilder;
pub use crate::error::StreamError;
pub use crate::error::TransportError;
pub use crate::request::PreparedRequestBody;
pub use crate::request::Request;
pub use crate::request::RequestBody;
pub use crate::request::RequestCompression;
pub use crate::request::Response;
pub use crate::retry::RetryOn;
pub use crate::retry::RetryPolicy;
pub use crate::retry::backoff;
pub use crate::retry::run_with_retry;
pub use crate::sse::sse_stream;
pub use crate::telemetry::RequestTelemetry;
pub use crate::transport::ByteStream;
pub use crate::transport::HttpTransport;
#[cfg(feature = "native-transport")]
pub use crate::transport::ReqwestTransport;
pub use crate::transport::StreamResponse;
