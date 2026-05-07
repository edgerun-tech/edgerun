//! Email authentication protocol logic.
//!
//! This module owns SPF, DKIM, and DMARC parsing, canonicalization, signing,
//! and verification helpers. It does not own DNS transports, sockets, policy
//! decisions, queues, or message storage.

pub mod dkim;
pub mod dmarc;
pub mod sign;
pub mod spf;
pub mod std;

pub use dkim::{DkimResult, DkimSignature, DkimStatus};
pub use dmarc::{DmarcPolicy, DmarcResult, DmarcStatus};
pub use sign::{DkimKeyStore, DkimSigner};
pub use spf::SpfResult;

use alloc::string::String;
use alloc::vec::Vec;
use std::io;

/// DNS queries needed by SPF, DKIM, and DMARC evaluation.
///
/// The protocol crate defines the required API but does not decide how those
/// TXT lookups are transported or cached.
pub trait DnsQuery {
    fn query_txt(
        &mut self,
        name: &str,
    ) -> impl std::future::Future<Output = io::Result<Vec<String>>> + Send;
}
