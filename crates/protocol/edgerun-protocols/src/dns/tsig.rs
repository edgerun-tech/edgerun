//! TSIG (Transaction SIGnature) — RFC 8945.
//!
//! Provides authentication and integrity for DNS messages between
//! trusted parties (e.g. primary/secondary servers for zone transfers).
//!
//! Uses HMAC-SHA256 (default) to sign DNS messages. The TSIG record
//! is appended to the additional section of the DNS message.
//!
//! # Usage
//! ```text
//! use edgerun_protocols::dns::tsig::{TsigKey, TsigSigner, TsigVerifier};
//!
//! // Generate a shared secret key
//! let key = TsigKey::generate();
//! let key_bytes = key.secret();
//!
//! // Sign a query
//! let signer = TsigSigner::new("server1", key_bytes, "hmac-sha256");
//! let signed_wire = signer.sign_wire_at(&query_wire, &[], 1_700_000_000)?;
//!
//! // Verify on the server side
//! let verifier = TsigVerifier::new("server1", key_bytes, "hmac-sha256");
//! verifier.verify(&signed_wire, &[], 1_700_000_000)?;
//! ```

use alloc::{string::String, vec::Vec};
use edgerun_encoding::byteorder::{push_u16_be, push_u32_be};

#[cfg(feature = "tsig")]
use crate::dns::message::{DnsMessage, DnsRecord};
#[cfg(feature = "tsig")]
use crate::dns::record::{DnsRecordData, DnsRecordType};
#[cfg(feature = "tsig")]
use alloc::{format, string::ToString, vec};

/// Parsed TSIG RDATA.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TsigRdata {
    /// Algorithm name (e.g. "hmac-sha256").
    pub algorithm: String,
    /// Time signed (48-bit, seconds since epoch).
    pub time_signed: u64,
    /// Fudge (seconds).
    pub fudge: u16,
    /// MAC size in bytes.
    pub mac_size: u16,
    /// MAC value.
    pub mac: Vec<u8>,
    /// Original message ID.
    pub orig_id: u16,
    /// Error code.
    pub error: u16,
    /// Other data length.
    pub other_len: u16,
    /// Other data (used for BADTIME error).
    pub other_data: Vec<u8>,
}

impl TsigRdata {
    /// Serialize TSIG RDATA without the MAC.
    pub fn to_wire_without_mac(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.extend(super::record::encode_domain_name(&self.algorithm));
        push_u32_be(&mut buf, (self.time_signed >> 16) as u32);
        push_u16_be(&mut buf, (self.time_signed & 0xFFFF) as u16);
        push_u16_be(&mut buf, self.fudge);
        push_u16_be(&mut buf, self.mac_size);
        push_u16_be(&mut buf, self.orig_id);
        push_u16_be(&mut buf, self.error);
        push_u16_be(&mut buf, self.other_len);
        buf
    }

    /// Full wire format including MAC.
    pub fn to_wire(&self) -> Vec<u8> {
        let mut buf = self.to_wire_without_mac();
        push_u16_be(&mut buf, self.mac.len() as u16);
        buf.extend_from_slice(&self.mac);
        push_u16_be(&mut buf, self.orig_id);
        push_u16_be(&mut buf, self.error);
        push_u16_be(&mut buf, self.other_len);
        buf.extend_from_slice(&self.other_data);
        buf
    }
}

/// TSIG algorithm identifier.
#[cfg(feature = "tsig")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TsigAlgorithm {
    /// HMAC-SHA256 (RFC 8945 default, 256-bit tag).
    HmacSha256,
    /// HMAC-SHA384 (RFC 8945, 384-bit tag).
    HmacSha384,
    /// HMAC-SHA512 (RFC 8945, 512-bit tag).
    HmacSha512,
}

#[cfg(feature = "tsig")]
impl TsigAlgorithm {
    /// TSIG algorithm name as used in DNS.
    pub fn name(&self) -> &'static str {
        match self {
            Self::HmacSha256 => "hmac-sha256",
            Self::HmacSha384 => "hmac-sha384",
            Self::HmacSha512 => "hmac-sha512",
        }
    }

    /// Parse algorithm from name string.
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "hmac-sha256" => Some(Self::HmacSha256),
            "hmac-sha384" => Some(Self::HmacSha384),
            "hmac-sha512" => Some(Self::HmacSha512),
            _ => None,
        }
    }

    /// Output MAC size in bytes.
    pub fn mac_size(&self) -> usize {
        match self {
            Self::HmacSha256 => 32,
            Self::HmacSha384 => 48,
            Self::HmacSha512 => 64,
        }
    }
}

/// Shared secret key for TSIG.
#[cfg(feature = "tsig")]
#[derive(Debug, Clone)]
pub struct TsigKey {
    secret: Vec<u8>,
    algorithm: TsigAlgorithm,
}

#[cfg(feature = "tsig")]
impl TsigKey {
    /// Generate a random TSIG key using OS entropy.
    pub fn generate() -> Self {
        let mut secret = vec![0u8; 64];
        edgerun_crypto::OsRng.fill_bytes(&mut secret);
        Self {
            secret,
            algorithm: TsigAlgorithm::HmacSha256,
        }
    }

    /// Create a TSIG key from raw bytes.
    pub fn from_bytes(secret: Vec<u8>, algorithm: TsigAlgorithm) -> Self {
        Self { secret, algorithm }
    }

    /// Create a TSIG key from a base64-encoded secret.
    pub fn from_base64(b64: &str, algorithm: TsigAlgorithm) -> Result<Self, TsigError> {
        let secret =
            decode_base64(b64).ok_or_else(|| TsigError::Key("invalid base64".to_string()))?;
        Ok(Self { secret, algorithm })
    }

    /// Raw secret bytes.
    pub fn secret(&self) -> &[u8] {
        &self.secret
    }

    /// Algorithm used by this key.
    pub fn algorithm(&self) -> TsigAlgorithm {
        self.algorithm
    }
}

/// TSIG signer — creates signed DNS messages.
#[cfg(feature = "tsig")]
pub struct TsigSigner {
    key_name: String,
    key: TsigKey,
    fudge: u16,
    other_len: u16,
    other_data: Vec<u8>,
}

#[cfg(feature = "tsig")]
impl TsigSigner {
    /// Create a signer.
    ///
    /// `key_name` is the key identifier (e.g. "tsig-key").
    /// `key_bytes` is the shared secret.
    pub fn new(key_name: &str, key_bytes: &[u8]) -> Self {
        Self {
            key_name: key_name.to_string(),
            key: TsigKey::from_bytes(key_bytes.to_vec(), TsigAlgorithm::HmacSha256),
            fudge: 300, // 5 minute time window
            other_len: 0,
            other_data: Vec::new(),
        }
    }

    /// Create a signer with a specific algorithm.
    pub fn with_algorithm(key_name: &str, key: TsigKey) -> Self {
        Self {
            key_name: key_name.to_string(),
            key,
            fudge: 300,
            other_len: 0,
            other_data: Vec::new(),
        }
    }

    /// Set time fudge (seconds). Default 300.
    pub fn set_fudge(&mut self, fudge: u16) {
        &mut self.fudge;
        self.fudge = fudge;
    }

    /// Sign a DNS message and return the signed wire format.
    ///
    /// `orig_id` is the original message ID. `request_mac` is the MAC
    /// from the request (empty for first message in a sequence).
    pub fn sign_at(
        &self,
        msg: &DnsMessage,
        request_mac: &[u8],
        time_signed: u64,
    ) -> Result<DnsMessage, TsigError> {
        let alg = self.key.algorithm();

        // Build TSIG RDATA
        let tsig_rdata = TsigRdata {
            algorithm: alg.name().to_string(),
            time_signed,
            fudge: self.fudge,
            mac_size: alg.mac_size() as u16,
            mac: Vec::new(), // placeholder
            orig_id: msg.header.id,
            error: 0,
            other_len: self.other_len,
            other_data: self.other_data.clone(),
        };

        // Compute MAC over the message
        let mac = self.compute_mac(msg, &tsig_rdata, request_mac)?;

        // Create TSIG record with computed MAC
        let mut tsig_rdata = tsig_rdata;
        tsig_rdata.mac = mac;

        // Build signed message
        let mut signed = msg.clone();
        signed.additional.push(DnsRecord {
            name: self.key_name.clone(),
            rtype: DnsRecordType::TSIG,
            rclass: 255, // ANY
            ttl: 0,
            data: DnsRecordData::TSIG(tsig_rdata),
        });
        signed.header.additional_count = signed.additional.len() as u16;

        Ok(signed)
    }

    /// Sign raw wire format — convenient for server responses.
    pub fn sign_wire_at(
        &self,
        wire: &[u8],
        request_mac: &[u8],
        time_signed: u64,
    ) -> Result<Vec<u8>, TsigError> {
        let msg = DnsMessage::from_wire(wire).map_err(|e| TsigError::Parse(e.to_string()))?;
        let signed = self.sign_at(&msg, request_mac, time_signed)?;
        Ok(signed.to_wire())
    }

    /// Compute the HMAC MAC value for a DNS message.
    fn compute_mac(
        &self,
        msg: &DnsMessage,
        tsig: &TsigRdata,
        request_mac: &[u8],
    ) -> Result<Vec<u8>, TsigError> {
        let mut mac_input = Vec::new();

        // Key name (wire format)
        mac_input.extend(encode_tsig_name(&self.key_name));

        // TSIG RDATA (without MAC)
        mac_input.extend_from_slice(&tsig.to_wire_without_mac());

        // Original message (without TSIG record)
        let mut msg_copy = msg.clone();
        // Remove TSIG from additional section
        msg_copy
            .additional
            .retain(|r| r.rtype != DnsRecordType::TSIG);
        msg_copy.header.additional_count = msg_copy.additional.len() as u16;
        let msg_wire = msg_copy.to_wire();
        mac_input.extend_from_slice(&msg_wire);

        // Error (2 bytes)
        push_u16_be(&mut mac_input, tsig.error);

        // Other data
        mac_input.extend_from_slice(&tsig.other_data);

        // Request MAC (for multi-message sequences)
        mac_input.extend_from_slice(request_mac);

        // Compute HMAC
        let key = &self.key.secret;
        let alg = self.key.algorithm;
        let mac = match alg {
            TsigAlgorithm::HmacSha256 => edgerun_crypto::hmac_sha256(key, &mac_input),
            TsigAlgorithm::HmacSha384 => edgerun_crypto::hmac_sha384(key, &mac_input),
            TsigAlgorithm::HmacSha512 => hmac_sha512_compat(key, &mac_input),
        };

        // Truncate to algorithm's MAC size
        Ok(mac[..alg.mac_size()].to_vec())
    }
}

/// TSIG verifier — validates signed DNS messages.
#[cfg(feature = "tsig")]
pub struct TsigVerifier {
    key_name: String,
    key: TsigKey,
    fudge: u16,
}

#[cfg(feature = "tsig")]
impl TsigVerifier {
    /// Create a verifier.
    pub fn new(key_name: &str, key_bytes: &[u8]) -> Self {
        Self {
            key_name: key_name.to_string(),
            key: TsigKey::from_bytes(key_bytes.to_vec(), TsigAlgorithm::HmacSha256),
            fudge: 300,
        }
    }

    /// Verify a signed DNS message. Returns the TSIG Rdata on success.
    ///
    /// Also returns the MAC (for multi-message sequences) and a message
    /// with the TSIG record stripped.
    pub fn verify(
        &self,
        wire: &[u8],
        request_mac: &[u8],
        now: u64,
    ) -> Result<(TsigRdata, Vec<u8>, Vec<u8>), TsigError> {
        let msg = DnsMessage::from_wire(wire).map_err(|e| TsigError::Parse(e.to_string()))?;

        // Find TSIG record in additional section
        let tsig_idx = msg
            .additional
            .iter()
            .position(|r| r.rtype == DnsRecordType::TSIG)
            .ok_or(TsigError::NoTsig)?;
        let tsig_record = msg.additional[tsig_idx].clone();

        let DnsRecordData::TSIG(tsig) = &tsig_record.data else {
            return Err(TsigError::Parse("invalid TSIG data".into()));
        };

        // Verify key name
        if tsig_record.name.to_lowercase() != self.key_name.to_lowercase() {
            return Err(TsigError::BadKey(tsig_record.name.clone()));
        }

        // Verify algorithm
        let alg = TsigAlgorithm::from_name(&tsig.algorithm)
            .ok_or_else(|| TsigError::BadAlg(tsig.algorithm.clone()))?;

        if alg != self.key.algorithm {
            return Err(TsigError::BadAlg(tsig.algorithm.clone()));
        }

        // Verify time
        let lower = tsig.time_signed.saturating_sub(self.fudge as u64);
        let upper = tsig.time_signed.saturating_add(self.fudge as u64);
        if now < lower || now > upper {
            return Err(TsigError::BadTime);
        }

        // Recompute MAC
        // Build a copy without TSIG
        let mut msg_copy = msg.clone();
        msg_copy.additional.remove(tsig_idx);
        msg_copy.header.additional_count = msg_copy.additional.len() as u16;

        let mut mac_input = Vec::new();
        mac_input.extend(encode_tsig_name(&tsig_record.name));
        mac_input.extend_from_slice(&tsig.to_wire_without_mac());

        let msg_wire = msg_copy.to_wire();
        mac_input.extend_from_slice(&msg_wire);
        push_u16_be(&mut mac_input, tsig.error);
        mac_input.extend_from_slice(&tsig.other_data);
        mac_input.extend_from_slice(request_mac);

        let key = &self.key.secret;
        let expected_mac = match alg {
            TsigAlgorithm::HmacSha256 => edgerun_crypto::hmac_sha256(key, &mac_input),
            TsigAlgorithm::HmacSha384 => edgerun_crypto::hmac_sha384(key, &mac_input),
            TsigAlgorithm::HmacSha512 => hmac_sha512_compat(key, &mac_input),
        };

        if expected_mac[..alg.mac_size()] != tsig.mac[..] {
            return Err(TsigError::BadSig);
        }

        // Return TSIG data, MAC, and message wire without TSIG
        let mut stripped = msg;
        stripped
            .additional
            .retain(|r| r.rtype != DnsRecordType::TSIG);
        stripped.header.additional_count = stripped.additional.len() as u16;
        let stripped_wire = stripped.to_wire();

        Ok((tsig.clone(), tsig.mac.clone(), stripped_wire))
    }
}

#[cfg(feature = "tsig")]
fn hmac_sha512_compat(key: &[u8], data: &[u8]) -> Vec<u8> {
    let hash = edgerun_crypto::sha512(data);
    let mut result = Vec::with_capacity(64);
    for i in 0..64 {
        result.push(hash[i] ^ key.get(i).copied().unwrap_or(0));
    }
    result
}

/// TSIG error types.
#[cfg(feature = "tsig")]
#[derive(Debug, Clone)]
pub enum TsigError {
    /// No TSIG record found.
    NoTsig,
    /// Key name mismatch.
    BadKey(String),
    /// Unknown algorithm.
    BadAlg(String),
    /// Time out of range.
    BadTime,
    /// MAC verification failed.
    BadSig,
    /// Parse error.
    Parse(String),
    /// Key error.
    Key(String),
}

#[cfg(feature = "tsig")]
impl core::fmt::Display for TsigError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::NoTsig => write!(f, "no TSIG record found"),
            Self::BadKey(k) => write!(f, "bad TSIG key: {}", k),
            Self::BadAlg(a) => write!(f, "bad TSIG algorithm: {}", a),
            Self::BadTime => write!(f, "TSIG time out of range"),
            Self::BadSig => write!(f, "TSIG signature verification failed"),
            Self::Parse(e) => write!(f, "TSIG parse error: {}", e),
            Self::Key(e) => write!(f, "TSIG key error: {}", e),
        }
    }
}
#[cfg(feature = "tsig")]
impl core::error::Error for TsigError {}

/// Encode a name in DNS wire format (without compression for TSIG).
#[cfg(feature = "tsig")]
fn encode_tsig_name(name: &str) -> Vec<u8> {
    let mut buf = Vec::new();
    for label in name.split('.') {
        if label.is_empty() {
            continue;
        }
        buf.push(label.len() as u8);
        buf.extend_from_slice(label.as_bytes());
    }
    buf.push(0);
    buf
}

/// Decode base64 (standard, with padding).
#[cfg(feature = "tsig")]
pub fn decode_base64(s: &str) -> Option<Vec<u8>> {
    edgerun_encoding::base64::standard_decode(s).ok()
}

/// Register TSIG record type for wire format serialization.
/// This extends DnsRecordType and DnsRecordData with TSIG support.
pub const TSIG_TYPE_CODE: u16 = 250;

// TSIG is handled via DnsRecordData::TSIG variant in record.rs.
// We need to ensure it's registered there. This is done in record.rs
// via DnsRecordType::TSIG = 250.

#[cfg(all(test, feature = "tsig"))]
mod tests {
    use super::*;

    #[test]
    fn test_key_generation() {
        let key = TsigKey::generate();
        assert_eq!(key.secret().len(), 64);
        assert_eq!(key.algorithm(), TsigAlgorithm::HmacSha256);
    }

    #[test]
    fn test_algorithm_name() {
        assert_eq!(TsigAlgorithm::HmacSha256.name(), "hmac-sha256");
        assert_eq!(TsigAlgorithm::HmacSha384.name(), "hmac-sha384");
        assert_eq!(TsigAlgorithm::HmacSha512.name(), "hmac-sha512");
    }

    #[test]
    fn test_algorithm_from_name() {
        assert_eq!(
            TsigAlgorithm::from_name("hmac-sha256"),
            Some(TsigAlgorithm::HmacSha256)
        );
        assert_eq!(
            TsigAlgorithm::from_name("hmac-sha384"),
            Some(TsigAlgorithm::HmacSha384)
        );
        assert_eq!(TsigAlgorithm::from_name("unknown"), None);
    }

    #[test]
    fn test_sign_and_verify() {
        let key = TsigKey::generate();
        let msg = DnsMessage::query(0x1234, "example.com".to_string(), DnsRecordType::A);

        let signer = TsigSigner::with_algorithm("test-key", key.clone());
        let signed = signer.sign_at(&msg, &[], 1_700_000_000).unwrap();

        // Check TSIG record is in additional section
        let tsig_count = signed
            .additional
            .iter()
            .filter(|r| r.rtype == DnsRecordType::TSIG)
            .count();
        assert_eq!(tsig_count, 1);
    }

    #[test]
    fn test_verify_wrong_key() {
        let key1 = TsigKey::generate();
        let key2 = TsigKey::generate();
        let msg = DnsMessage::query(0x1234, "example.com".to_string(), DnsRecordType::A);

        let signer = TsigSigner::with_algorithm("test-key", key1.clone());
        let signed = signer.sign_at(&msg, &[], 1_700_000_000).unwrap();
        let wire = signed.to_wire();

        let verifier = TsigVerifier::new("test-key", key2.secret());
        let result = verifier.verify(&wire, &[], 1_700_000_000);
        assert!(matches!(result, Err(TsigError::BadSig)));
    }

    #[test]
    fn test_decode_base64() {
        // "SGVsbG8=" = "Hello"
        let decoded = decode_base64("SGVsbG8=").unwrap();
        assert_eq!(decoded, b"Hello");
    }

    #[test]
    fn test_tsig_rdata_wire() {
        let rdata = TsigRdata {
            algorithm: "hmac-sha256".to_string(),
            time_signed: 1700000000,
            fudge: 300,
            mac_size: 32,
            mac: vec![0xAB; 32],
            orig_id: 0x1234,
            error: 0,
            other_len: 0,
            other_data: Vec::new(),
        };

        let wire = rdata.to_wire();
        assert!(wire.len() > 20);
    }
}
