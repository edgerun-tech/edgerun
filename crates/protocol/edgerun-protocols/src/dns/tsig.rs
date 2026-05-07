use alloc::{string::String, vec::Vec};

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
        buf.extend_from_slice(&((self.time_signed >> 16) as u32).to_be_bytes());
        buf.extend_from_slice(&((self.time_signed & 0xFFFF) as u16).to_be_bytes());
        buf.extend_from_slice(&self.fudge.to_be_bytes());
        buf.extend_from_slice(&self.mac_size.to_be_bytes());
        buf.extend_from_slice(&self.orig_id.to_be_bytes());
        buf.extend_from_slice(&self.error.to_be_bytes());
        buf.extend_from_slice(&self.other_len.to_be_bytes());
        buf
    }

    /// Full wire format including MAC.
    pub fn to_wire(&self) -> Vec<u8> {
        let mut buf = self.to_wire_without_mac();
        buf.extend_from_slice(&(self.mac.len() as u16).to_be_bytes());
        buf.extend_from_slice(&self.mac);
        buf.extend_from_slice(&self.orig_id.to_be_bytes());
        buf.extend_from_slice(&self.error.to_be_bytes());
        buf.extend_from_slice(&self.other_len.to_be_bytes());
        buf.extend_from_slice(&self.other_data);
        buf
    }
}
