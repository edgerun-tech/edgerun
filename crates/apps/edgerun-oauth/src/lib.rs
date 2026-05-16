//! # edgerun-oauth
//!
//! OAuth 2.0 + OpenID Connect (OIDC) client and middleware helpers.
//! Zero external dependencies — uses only edgerun-* crates.
//!
//! ## Features
//! - OAuth 2.0 Device Authorization Grant (RFC 8628) with PKCE
//! - OAuth 2.0 Authorization Code Flow with PKCE (RFC 6749 + RFC 7636)
//! - OpenID Connect (OIDC) ID Token parsing and verification
//! - OIDC Discovery (RFC 8414)
//! - File-based token storage with atomic writes
//! - Async-first API (edgerun-node runtime)
//!
//! ## Client Quick Start
//! ```no_run
//! use edgerun_oauth::{OAuthClient, DeviceFlowCallback, ClientConfig};
//!
//! struct MyCallback;
//! impl DeviceFlowCallback for MyCallback {
//!     fn display_verification(&self, uri: &str, user_code: &str) {
//!         println!("Go to: {uri}");
//!         println!("Enter code: {user_code}");
//!     }
//! }
//!
//! # edgerun_node::rt::block_on(async {
//! let config = ClientConfig::device_flow(
//!     "https://provider.example.com",
//!     "my-client-id",
//! );
//! let client = OAuthClient::new(config);
//! let creds = client.device_flow(&MyCallback).await.unwrap();
//! if let Some(token) = creds.bearer_token() {
//!     println!("Access token: {token}");
//! }
//! # });
//! ```
//!
#![no_std]

extern crate alloc;
#[cfg(target_os = "none")]
extern crate self as std;
#[cfg(not(target_os = "none"))]
extern crate std;

pub mod prelude {
    pub use alloc::boxed::Box;
    pub use alloc::format;
    pub use alloc::string::{String, ToString};
    pub use alloc::vec;
    pub use alloc::vec::Vec;
    pub use core::option::Option::{self, None, Some};
    pub use core::prelude::rust_2024::*;
    pub use core::result::Result::{self, Err, Ok};
    pub use core::write;
}

#[cfg(target_os = "none")]
pub mod collections {
    pub use alloc::collections::{BTreeMap as HashMap, BTreeSet as HashSet};
}

#[cfg(target_os = "none")]
pub mod future {
    pub use core::future::*;
}

#[cfg(target_os = "none")]
pub mod pin {
    pub use core::pin::*;
}

#[cfg(target_os = "none")]
pub mod result {
    pub use core::result::*;
}

#[cfg(target_os = "none")]
pub mod io {
    pub use edgerun_secret_service::io::*;
}

#[cfg(target_os = "none")]
pub mod path {
    pub use edgerun_secret_service::path::{Path, PathBuf};
}

#[cfg(target_os = "none")]
pub mod env {
    use crate::io;
    use alloc::string::String;

    pub fn var(_key: &str) -> io::Result<String> {
        Err(io::Error::other("environment is unavailable"))
    }
}

#[cfg(target_os = "none")]
pub mod sync {
    pub use edgerun_node::rt::{RwLockReadGuard, RwLockWriteGuard};
    pub use edgerun_secret_service::sync::{Arc, Mutex, RwLock};
}

#[cfg(target_os = "none")]
pub mod error {
    pub use core::error::*;
}

#[cfg(target_os = "none")]
pub mod time {
    pub use edgerun_node::http::time::{Duration, Instant};
}

mod client;
mod discovery;
mod errors;
mod jwt;
mod oauth_client;
mod pkce;
mod token_store;
mod types;

pub fn runtime_now_secs() -> u64 {
    edgerun_node::rt::timer::ticks_to_us(edgerun_node::rt::now()) / 1_000_000
}

pub use client::{DeviceFlowCallback, OAuthClient};
pub use discovery::{Jwk, JwksDocument, OidcDiscoveryDocument};
pub use edgerun_encoding::base64::{base64url_decode, base64url_encode, base64url_nopad_encode};
pub use errors::{DeviceError, OAuthError};
pub use jwt::{IdToken, JwtHeader, JwtPayload, JwtVerifier, verifier_from_jwk};
pub use oauth_client::{AutoRefreshMiddleware, BearerTokenMiddleware, OAuthClientBuilder};
pub use pkce::PkcePair;
pub use token_store::{TokenStore, default_token_path};

// Re-export edgerun-node http middleware types for convenience
pub use edgerun_node::http::client_middleware::{
    Chain as ClientChain, Client, ClientExtensions, ClientMiddleware, ClientNext, ClientRequest,
    ClientTransport, client_middleware_fn,
};
pub use types::{
    ClientConfig, Credentials, DeviceAuthorizationRequest, DeviceAuthorizationResponse, GrantType,
    OAuthCallback, OAuthRequest, OAuthResponse, Scope, TokenRequest, TokenResponse,
};
