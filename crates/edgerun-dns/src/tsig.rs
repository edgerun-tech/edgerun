//! TSIG runtime adapter.
//!
//! The TSIG protocol core lives in `edgerun-protocols::dns::tsig`.
//! This module only supplies runtime time defaults for the legacy DNS crate API.

use alloc::vec::Vec;

use crate::message::DnsMessage;
use crate::std::io;
use crate::std::time::{SystemTime, UNIX_EPOCH};

pub use edgerun_protocols::dns::tsig::{
    decode_base64, TsigAlgorithm, TsigError, TsigKey, TsigRdata, TSIG_TYPE_CODE,
};

/// TSIG signer with runtime clock convenience methods.
pub struct TsigSigner {
    inner: edgerun_protocols::dns::tsig::TsigSigner,
}

impl TsigSigner {
    /// Create a signer using HMAC-SHA256.
    pub fn new(key_name: &str, key_bytes: &[u8]) -> Self {
        Self {
            inner: edgerun_protocols::dns::tsig::TsigSigner::new(key_name, key_bytes),
        }
    }

    /// Create a signer with a specific algorithm.
    pub fn with_algorithm(key_name: &str, key: TsigKey) -> Self {
        Self {
            inner: edgerun_protocols::dns::tsig::TsigSigner::with_algorithm(key_name, key),
        }
    }

    /// Set time fudge (seconds). Default 300.
    pub fn set_fudge(&mut self, fudge: u16) {
        self.inner.set_fudge(fudge);
    }

    /// Sign with the runtime clock.
    pub fn sign(&self, msg: &DnsMessage, request_mac: &[u8]) -> Result<DnsMessage, io::Error> {
        self.inner
            .sign_at(msg, request_mac, current_time())
            .map_err(tsig_io_error)
    }

    /// Sign raw wire format with the runtime clock.
    pub fn sign_wire(&self, wire: &[u8], request_mac: &[u8]) -> Result<Vec<u8>, io::Error> {
        self.inner
            .sign_wire_at(wire, request_mac, current_time())
            .map_err(tsig_io_error)
    }

    /// Sign with explicit time for deterministic tests and replay.
    pub fn sign_at(
        &self,
        msg: &DnsMessage,
        request_mac: &[u8],
        time_signed: u64,
    ) -> Result<DnsMessage, TsigError> {
        self.inner.sign_at(msg, request_mac, time_signed)
    }
}

/// TSIG verifier with runtime clock convenience methods.
pub struct TsigVerifier {
    inner: edgerun_protocols::dns::tsig::TsigVerifier,
}

impl TsigVerifier {
    /// Create a verifier using HMAC-SHA256.
    pub fn new(key_name: &str, key_bytes: &[u8]) -> Self {
        Self {
            inner: edgerun_protocols::dns::tsig::TsigVerifier::new(key_name, key_bytes),
        }
    }

    /// Verify with the runtime clock.
    pub fn verify(
        &self,
        wire: &[u8],
        request_mac: &[u8],
    ) -> Result<(TsigRdata, Vec<u8>, Vec<u8>), TsigError> {
        self.inner.verify(wire, request_mac, current_time())
    }

    /// Verify with explicit time for deterministic tests and replay.
    pub fn verify_at(
        &self,
        wire: &[u8],
        request_mac: &[u8],
        now: u64,
    ) -> Result<(TsigRdata, Vec<u8>, Vec<u8>), TsigError> {
        self.inner.verify(wire, request_mac, now)
    }
}

fn current_time() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn tsig_io_error(error: TsigError) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, error)
}
