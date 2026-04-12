//! # edgerun-oauth
//!
//! OAuth 2.0 + OpenID Connect (OIDC) client and server library with device flow support.
//! Zero external dependencies — uses only edgerun-* crates.
//!
//! ## Features
//! - OAuth 2.0 Device Authorization Grant (RFC 8628) with PKCE
//! - OAuth 2.0 Authorization Code Flow with PKCE (RFC 6749 + RFC 7636)
//! - OpenID Connect (OIDC) ID Token parsing and verification
//! - OIDC Discovery (RFC 8414)
//! - JWKS endpoint for public key distribution
//! - Token introspection (RFC 7662)
//! - File-based token storage with atomic writes
//! - Async-first API (edgerun-rt runtime)
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
//! # edgerun_rt::block_on(async {
//! let config = ClientConfig::device_flow(
//!     "https://provider.example.com",
//!     "my-client-id",
//! );
//! let client = OAuthClient::new(config);
//! let creds = client.device_flow(&MyCallback).await.unwrap();
//! println!("Access token: {}", creds.access_token);
//! # });
//! ```
//!
//! ## Server Quick Start
//! ```no_run
//! use edgerun_oauth::{OAuthServer, ServerConfig, Issuer};
//!
//! # edgerun_rt::block_on(async {
//! let config = ServerConfig::new("https://auth.example.com", "my-issuer");
//! let server = OAuthServer::new(config).unwrap();
//!
//! // Register a new client
//! server.register_client("my-app", vec!["openid".into(), "email".into()]).await;
//!
//! // The server exposes these endpoints:
//! // GET  /.well-known/openid-configuration
//! // GET  /.well-known/jwks.json
//! // POST /oauth2/device/code
//! // POST /oauth2/token
//! // POST /oauth2/introspect
//! # });
//! ```

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(not(feature = "std"))]
extern crate alloc;

mod base64url;
mod client;
mod device_state;
mod discovery;
mod errors;
mod jwt;
mod pkce;
mod server;
mod token_store;
mod types;

pub use base64url::{base64url_encode, base64url_decode, base64url_nopad_encode};
pub use client::{OAuthClient, DeviceFlowCallback};
pub use discovery::{OidcDiscoveryDocument, JwksDocument, Jwk};
pub use errors::{OAuthError, OAuthResult, DeviceError};
pub use jwt::{IdToken, JwtHeader, JwtPayload, JwtVerifier};
pub use pkce::PkcePair;
pub use server::{OAuthServer, ServerConfig, ClientRegistration, PendingDeviceGrant};
pub use token_store::{TokenStore, default_token_path};
pub use types::{
    ClientConfig,
    Credentials,
    GrantType,
    OAuthCallback,
    OAuthRequest,
    OAuthResponse,
    Scope,
    TokenRequest,
    TokenResponse,
    DeviceAuthorizationRequest,
    DeviceAuthorizationResponse,
};
