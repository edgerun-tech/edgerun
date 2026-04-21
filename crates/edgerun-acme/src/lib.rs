//! edgerun-acme — ACME protocol client (RFC 8555) for Let's Encrypt integration.
//!
//! Supports:
//! - HTTP-01 challenge (via edgerun-http Handler)
//! - DNS-01 challenge (via edgerun-dns zone injection)
//! - TLS-ALPN-01 challenge (via edgerun-tls ALPN extension)
//! - Automatic certificate provisioning and renewal
//! - Encrypted storage of private keys via edgerun-secret-service

mod error;
mod client;
pub mod types;
mod account;
mod order;
mod challenge;
mod http_challenge;
mod dns_challenge;
mod tls_alpn_challenge;
mod cert_store;

pub use error::AcmeError;
pub use client::{AcmeClient, AcmeConfig};
pub use account::AccountKey;
pub use order::Order;
pub use challenge::Challenge;
pub use http_challenge::{HttpChallengeHandler, HttpChallengeServer};
pub use dns_challenge::{DnsChallenge, DnsChallengeManager};
pub use tls_alpn_challenge::{TlsAlpnChallenge, TlsAlpnManager};
pub use cert_store::{CertStore, CertInfo, StoredCert};
pub use types::{Directory, DirectoryUrl, OrderStatus, ChallengeType, ChallengeStatus};