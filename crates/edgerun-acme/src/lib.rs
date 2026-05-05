//! edgerun-acme — ACME protocol client (RFC 8555) for Let's Encrypt integration.
//!
//! Supports:
//! - HTTP-01 challenge (via edgerun-http Handler)
//! - DNS-01 challenge (via edgerun-dns zone injection)
//! - TLS-ALPN-01 challenge (via edgerun-tls ALPN extension)
//! - Automatic certificate provisioning and renewal
//! - Encrypted storage of private keys via edgerun-secret-service

#![no_std]

extern crate alloc;
#[cfg(target_os = "none")]
extern crate self as std;
#[cfg(not(target_os = "none"))]
extern crate std;

pub mod prelude {
    pub mod v1 {
        pub use alloc::borrow::ToOwned;
        pub use alloc::boxed::Box;
        pub use alloc::format;
        pub use alloc::string::{String, ToString};
        pub use alloc::vec;
        pub use alloc::vec::Vec;
        pub use core::prelude::rust_2021::*;
    }
}

pub mod boxed {
    pub use alloc::boxed::Box;
}

pub mod string {
    pub use alloc::string::{String, ToString};
}

pub mod vec {
    pub use alloc::vec::Vec;
}

pub mod future {
    pub use core::future::*;
}

pub mod pin {
    pub use core::pin::*;
}

#[cfg(target_os = "none")]
pub use core::{cmp, convert, fmt, mem, option, result, slice, str};

#[cfg(target_os = "none")]
pub mod collections {
    pub use alloc::collections::{BTreeMap, BTreeSet, VecDeque};

    pub type HashMap<K, V> = alloc::collections::BTreeMap<K, V>;
    pub type HashSet<T> = alloc::collections::BTreeSet<T>;
}

#[cfg(target_os = "none")]
pub mod path {
    pub use edgerun_secret_service::path::{Path, PathBuf};
}

#[cfg(target_os = "none")]
pub mod sync {
    pub use alloc::sync::Arc;
    pub use edgerun_rt::{Mutex, MutexGuard, RwLock, RwLockReadGuard, RwLockWriteGuard};
}

mod account;
mod cert_store;
mod challenge;
pub mod challenge_material;
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
