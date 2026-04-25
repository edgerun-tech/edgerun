//! edgerun-acme — ACME protocol client (RFC 8555) for Let's Encrypt integration.
//!
//! Supports:
//! - HTTP-01 challenge (via edgerun-http Handler)
//! - DNS-01 challenge (via edgerun-dns zone injection)
//! - TLS-ALPN-01 challenge (via edgerun-tls ALPN extension)
//! - Automatic certificate provisioning and renewal
//! - Encrypted storage of private keys via edgerun-secret-service

mod account;
mod cert_store;
mod challenge;
mod client;
mod dns_challenge;
mod error;
mod http_challenge;
mod order;
mod tls_alpn_challenge;
pub mod types;

pub use account::AccountKey;
pub use cert_store::{CertInfo, CertStore, StoredCert};
pub use challenge::Challenge;
pub use client::{AcmeClient, AcmeConfig};
pub use dns_challenge::{DnsChallenge, DnsChallengeManager};
pub use error::AcmeError;
pub use http_challenge::{HttpChallengeHandler, HttpChallengeServer};
pub use order::Order;
pub use tls_alpn_challenge::{TlsAlpnChallenge, TlsAlpnManager};
pub use types::{ChallengeStatus, ChallengeType, Directory, DirectoryUrl, OrderStatus};
