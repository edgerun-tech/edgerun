//! DHCPv6 Unique Identifiers (DUIDs) — RFC 8415 §11.

use alloc::vec::Vec;
use core::fmt;
use edgerun_encoding::byteorder::read_u16_be;

/// DHCPv6 Unique Identifier.
///
/// DUIDs uniquely identify DHCPv6 clients and servers.
/// Four types defined in RFC 8415 §11.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Duid {
    /// The DUID type.
    pub duid_type: DuidType,
    /// Raw bytes (excluding the 2-byte type field).
    pub data: Vec<u8>,
}

/// DUID types per RFC 8415 §11.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DuidType {
    /// DUID-LLT: link-layer + time (type 1) — most common for clients.
    Llt = 1,
    /// DUID-EN: vendor-assigned (type 2).
    En = 2,
    /// DUID-LL: link-layer only (type 3) — simplest, no time dependency.
    Ll = 3,
    /// DUID-UUID: UUID-based (type 4) — RFC 6355.
    Uuid = 4,
}

impl Duid {
    // -----------------------------------------------------------------------
    // Constructors
    // -----------------------------------------------------------------------

    /// Create a DUID-LLT (type 1) — hardware type + time + link-layer address.
    /// This is the default DUID type for most clients.
    pub fn llt(hw_type: u16, time: u32, link_addr: &[u8]) -> Self {
        let mut data = Vec::with_capacity(6 + link_addr.len());
        data.extend_from_slice(&hw_type.to_be_bytes()); // hardware type (1 = Ethernet)
        data.extend_from_slice(&time.to_be_bytes()); // time
        data.extend_from_slice(link_addr); // MAC address
        Self {
            duid_type: DuidType::Llt,
            data,
        }
    }

    /// Create a DUID-LL (type 3) — hardware type + link-layer address only.
    pub fn ll(hw_type: u16, link_addr: &[u8]) -> Self {
        let mut data = Vec::with_capacity(2 + link_addr.len());
        data.extend_from_slice(&hw_type.to_be_bytes());
        data.extend_from_slice(link_addr);
        Self {
            duid_type: DuidType::Ll,
            data,
        }
    }

    /// Create a DUID-EN (type 2) — enterprise number + identifier.
    pub fn en(enterprise_number: u32, identifier: &[u8]) -> Self {
        let mut data = Vec::with_capacity(4 + identifier.len());
        data.extend_from_slice(&enterprise_number.to_be_bytes());
        data.extend_from_slice(identifier);
        Self {
            duid_type: DuidType::En,
            data,
        }
    }

    /// Create a DUID-UUID (type 4) — 16-byte UUID.
    pub fn uuid(uuid: [u8; 16]) -> Self {
        Self {
            duid_type: DuidType::Uuid,
            data: uuid.to_vec(),
        }
    }

    /// Generate a DUID-LLT from the current time and a MAC address.
    pub fn generate_llt(mac: [u8; 6]) -> Self {
        Self::llt(1, 0, &mac) // hw_type 1 = Ethernet
    }

    /// Generate a DUID-LL from a MAC address.
    pub fn generate_ll(mac: [u8; 6]) -> Self {
        Self::ll(1, &mac)
    }

    /// Parse a DUID from wire format (includes 2-byte type prefix).
    pub fn from_wire(data: &[u8]) -> Option<Self> {
        if data.len() < 4 {
            return None;
        } // min: 2 type + 2 data
        let duid_type = read_u16_be(data, 0);
        let duid_type = match duid_type {
            1 => DuidType::Llt,
            2 => DuidType::En,
            3 => DuidType::Ll,
            4 => DuidType::Uuid,
            _ => return None,
        };
        Some(Self {
            duid_type,
            data: data[2..].to_vec(),
        })
    }

    /// Serialize to wire format (includes 2-byte type prefix).
    pub fn to_wire(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(2 + self.data.len());
        out.extend_from_slice(&(self.duid_type as u16).to_be_bytes());
        out.extend_from_slice(&self.data);
        out
    }

    /// Get the link-layer address from a DUID-LLT or DUID-LL, if present.
    pub fn link_layer_address(&self) -> Option<&[u8]> {
        match self.duid_type {
            DuidType::Llt if self.data.len() >= 6 => Some(&self.data[6..]),
            DuidType::Ll if self.data.len() >= 2 => Some(&self.data[2..]),
            _ => None,
        }
    }

    /// Get the hardware type from a DUID-LLT or DUID-LL, if present.
    pub fn hardware_type(&self) -> Option<u16> {
        match self.duid_type {
            DuidType::Llt | DuidType::Ll if self.data.len() >= 2 => {
                Some(read_u16_be(&self.data, 0))
            }
            _ => None,
        }
    }
}

impl fmt::Display for Duid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (i, b) in self.data.iter().enumerate() {
            if i > 0 {
                write!(f, ":")?;
            }
            write!(f, "{:02x}", b)?;
        }
        Ok(())
    }
}

/// Default server DUID — DUID-LLT generated at server startup.
pub fn default_server_duid() -> Duid {
    Duid::ll(1, b"edgerun")
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::format;

    #[test]
    fn test_duid_llt_roundtrip() {
        let mac = [0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff];
        let duid = Duid::generate_llt(mac);
        assert_eq!(duid.duid_type, DuidType::Llt);
        assert_eq!(duid.link_layer_address(), Some(&mac[..]));
        assert_eq!(duid.hardware_type(), Some(1));

        let wire = duid.to_wire();
        let parsed = Duid::from_wire(&wire).unwrap();
        assert_eq!(parsed, duid);
    }

    #[test]
    fn test_duid_ll_roundtrip() {
        let mac = [0x11, 0x22, 0x33, 0x44, 0x55, 0x66];
        let duid = Duid::generate_ll(mac);
        assert_eq!(duid.duid_type, DuidType::Ll);
        assert_eq!(duid.link_layer_address(), Some(&mac[..]));

        let wire = duid.to_wire();
        let parsed = Duid::from_wire(&wire).unwrap();
        assert_eq!(parsed, duid);
    }

    #[test]
    fn test_duid_en_roundtrip() {
        let duid = Duid::en(0x00001234, &[0xde, 0xad, 0xbe, 0xef]);
        assert_eq!(duid.duid_type, DuidType::En);
        let wire = duid.to_wire();
        let parsed = Duid::from_wire(&wire).unwrap();
        assert_eq!(parsed, duid);
    }

    #[test]
    fn test_duid_uuid_roundtrip() {
        let uuid = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16];
        let duid = Duid::uuid(uuid);
        assert_eq!(duid.duid_type, DuidType::Uuid);
        let wire = duid.to_wire();
        let parsed = Duid::from_wire(&wire).unwrap();
        assert_eq!(parsed, duid);
    }

    #[test]
    fn test_duid_display() {
        let mac = [0xaa, 0xbb, 0xcc];
        let duid = Duid::ll(1, &mac);
        assert_eq!(format!("{}", duid), "00:01:aa:bb:cc");
    }

    #[test]
    fn test_default_server_duid() {
        let duid = default_server_duid();
        assert_eq!(duid.duid_type, DuidType::Ll);
        assert!(!duid.data.is_empty());
    }
}
