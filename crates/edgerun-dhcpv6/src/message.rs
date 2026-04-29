//! DHCPv6 message types — RFC 8415 §7.

use crate::std::fmt;
use crate::std::io;
use crate::std::prelude::v1::*;
use edgerun_encoding::byteorder::{read_u16_be, read_u32_be};

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

pub const DHCPV6_SERVER_PORT: u16 = 547;
pub const DHCPV6_CLIENT_PORT: u16 = 546;

/// All_DHCP_Relay_Agents_and_Servers multicast address.
pub const ALL_DHCP_RELAY_AND_SERVERS: &str = "ff02::1:2";
/// All_DHCP_Servers (site-local scope).
pub const ALL_DHCP_SERVERS_SITE: &str = "ff05::1:3";

// ---------------------------------------------------------------------------
// Message types (RFC 8415 §7.3)
// ---------------------------------------------------------------------------

/// DHCPv6 message type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Dhcpv6MsgType {
    /// Client → Server: requesting configuration.
    Solicit = 1,
    /// Server → Client: offering configuration.
    Advertise = 2,
    /// Client → Server: requesting offered addresses/options.
    Request = 3,
    /// Client → Server: confirming addresses on new link.
    Confirm = 4,
    /// Client → Server: extending address lifetimes.
    Renew = 5,
    /// Client → Server: extending lifetimes when server unreachable.
    Rebind = 6,
    /// Server → Client: response to Solicit/Request/Renew/Rebind/Confirm.
    Reply = 7,
    /// Client → Server: releasing addresses.
    Release = 8,
    /// Client → Server: reporting address conflict.
    Decline = 9,
    /// Server → Client: trigger client to reconfigure.
    Reconfigure = 10,
    /// Client → Server: requesting options only (stateless).
    InformationRequest = 11,
    /// Relay → Relay/Server: forwarding client message.
    RelayForw = 12,
    /// Relay/Server → Relay: response to RelayForw.
    RelayRepl = 13,
}

impl Dhcpv6MsgType {
    pub fn from_u8(v: u8) -> Option<Self> {
        match v {
            1 => Some(Self::Solicit),
            2 => Some(Self::Advertise),
            3 => Some(Self::Request),
            4 => Some(Self::Confirm),
            5 => Some(Self::Renew),
            6 => Some(Self::Rebind),
            7 => Some(Self::Reply),
            8 => Some(Self::Release),
            9 => Some(Self::Decline),
            10 => Some(Self::Reconfigure),
            11 => Some(Self::InformationRequest),
            12 => Some(Self::RelayForw),
            13 => Some(Self::RelayRepl),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Solicit => "SOLICIT",
            Self::Advertise => "ADVERTISE",
            Self::Request => "REQUEST",
            Self::Confirm => "CONFIRM",
            Self::Renew => "RENEW",
            Self::Rebind => "REBIND",
            Self::Reply => "REPLY",
            Self::Release => "RELEASE",
            Self::Decline => "DECLINE",
            Self::Reconfigure => "RECONFIGURE",
            Self::InformationRequest => "INFORMATION-REQUEST",
            Self::RelayForw => "RELAY-FORW",
            Self::RelayRepl => "RELAY-REPL",
        }
    }
}

impl fmt::Display for Dhcpv6MsgType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

// ---------------------------------------------------------------------------
// Transaction ID
// ---------------------------------------------------------------------------

/// 3-byte transaction ID — matches client transaction across messages.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TransactionId(pub [u8; 3]);

impl TransactionId {
    pub fn random() -> Self {
        use edgerun_crypto::RngCore;
        let mut bytes = [0u8; 3];
        edgerun_crypto::OsRng.fill_bytes(&mut bytes);
        Self(bytes)
    }

    pub fn from_u32(v: u32) -> Self {
        Self([(v >> 16) as u8, (v >> 8) as u8, v as u8])
    }

    pub fn as_u32(self) -> u32 {
        ((self.0[0] as u32) << 16) | ((self.0[1] as u32) << 8) | (self.0[2] as u32)
    }
}

impl fmt::Display for TransactionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "0x{:02x}{:02x}{:02x}", self.0[0], self.0[1], self.0[2])
    }
}

// ---------------------------------------------------------------------------
// DHCPv6 Message
// ---------------------------------------------------------------------------

/// A complete DHCPv6 message (client/server messages only, not relay).
///
/// Wire format (RFC 8415 §6):
/// - msg-type (1 byte)
/// - transaction-id (3 bytes)
/// - options (variable, option-code 2 bytes + option-len 2 bytes + option-data)
#[derive(Debug, Clone)]
pub struct Dhcpv6Message {
    pub msg_type: Dhcpv6MsgType,
    pub transaction_id: TransactionId,
    pub options: Vec<super::options::Dhcpv6Option>,
}

impl Dhcpv6Message {
    /// Create a Solicit message with IA_NA options.
    pub fn solicit(iaid: u32, client_duid: &[u8], elapsed_ms: u16) -> Self {
        let mut options = Vec::new();

        // IA_NA option
        let mut ia_na_data = Vec::new();
        ia_na_data.extend_from_slice(&iaid.to_be_bytes());
        ia_na_data.extend_from_slice(&0u32.to_be_bytes()); // T1 (server fills in)
        ia_na_data.extend_from_slice(&0u32.to_be_bytes()); // T2 (server fills in)
        options.push(super::options::Dhcpv6Option::from_raw(
            super::options::OPT_IA_NA,
            ia_na_data,
        ));

        // Client Identifier option
        options.push(super::options::Dhcpv6Option::from_raw(
            super::options::OPT_CLIENTID,
            client_duid.to_vec(),
        ));

        // Elapsed Time option
        options.push(super::options::Dhcpv6Option::from_raw(
            super::options::OPT_ELAPSED_TIME,
            elapsed_ms.to_be_bytes().to_vec(),
        ));

        // Option Request List — what we want (each option code is 2 bytes, big-endian)
        let mut oro_data = Vec::new();
        for &code in &[
            super::options::OPT_DNS_SERVERS,
            super::options::OPT_DOMAIN_LIST,
            super::options::OPT_SNTP_SERVERS,
            super::options::OPT_NTP_SERVER,
        ] {
            oro_data.extend_from_slice(&code.to_be_bytes());
        }
        options.push(super::options::Dhcpv6Option::from_raw(
            super::options::OPT_ORO,
            oro_data,
        ));

        Self {
            msg_type: Dhcpv6MsgType::Solicit,
            transaction_id: TransactionId::random(),
            options,
        }
    }

    /// Create a Request message.
    pub fn request(iaid: u32, client_duid: &[u8], server_duid: &[u8]) -> Self {
        let mut options = Vec::new();

        // IA_NA option
        let mut ia_na_data = Vec::new();
        ia_na_data.extend_from_slice(&iaid.to_be_bytes());
        ia_na_data.extend_from_slice(&0u32.to_be_bytes());
        ia_na_data.extend_from_slice(&0u32.to_be_bytes());
        options.push(super::options::Dhcpv6Option::from_raw(
            super::options::OPT_IA_NA,
            ia_na_data,
        ));

        // Client Identifier
        options.push(super::options::Dhcpv6Option::from_raw(
            super::options::OPT_CLIENTID,
            client_duid.to_vec(),
        ));

        // Server Identifier
        options.push(super::options::Dhcpv6Option::from_raw(
            super::options::OPT_SERVERID,
            server_duid.to_vec(),
        ));

        Self {
            msg_type: Dhcpv6MsgType::Request,
            transaction_id: TransactionId::random(),
            options,
        }
    }

    /// Create a Renew message.
    pub fn renew(iaid: u32, client_duid: &[u8], server_duid: &[u8]) -> Self {
        let mut options = Vec::new();

        let mut ia_na_data = Vec::new();
        ia_na_data.extend_from_slice(&iaid.to_be_bytes());
        ia_na_data.extend_from_slice(&0u32.to_be_bytes());
        ia_na_data.extend_from_slice(&0u32.to_be_bytes());
        options.push(super::options::Dhcpv6Option::from_raw(
            super::options::OPT_IA_NA,
            ia_na_data,
        ));

        options.push(super::options::Dhcpv6Option::from_raw(
            super::options::OPT_CLIENTID,
            client_duid.to_vec(),
        ));
        options.push(super::options::Dhcpv6Option::from_raw(
            super::options::OPT_SERVERID,
            server_duid.to_vec(),
        ));

        Self {
            msg_type: Dhcpv6MsgType::Renew,
            transaction_id: TransactionId::random(),
            options,
        }
    }

    /// Create an Information-Request message (stateless).
    pub fn information_request(client_duid: &[u8]) -> Self {
        let mut options = Vec::new();
        options.push(super::options::Dhcpv6Option::from_raw(
            super::options::OPT_CLIENTID,
            client_duid.to_vec(),
        ));
        let mut oro_data = Vec::new();
        for &code in &[
            super::options::OPT_DNS_SERVERS,
            super::options::OPT_DOMAIN_LIST,
            super::options::OPT_SNTP_SERVERS,
        ] {
            oro_data.extend_from_slice(&code.to_be_bytes());
        }
        options.push(super::options::Dhcpv6Option::from_raw(
            super::options::OPT_ORO,
            oro_data,
        ));

        Self {
            msg_type: Dhcpv6MsgType::InformationRequest,
            transaction_id: TransactionId::random(),
            options,
        }
    }

    /// Create a Release message.
    pub fn release(iaid: u32, client_duid: &[u8], server_duid: &[u8]) -> Self {
        let mut options = Vec::new();

        let mut ia_na_data = Vec::new();
        ia_na_data.extend_from_slice(&iaid.to_be_bytes());
        ia_na_data.extend_from_slice(&0u32.to_be_bytes());
        ia_na_data.extend_from_slice(&0u32.to_be_bytes());
        options.push(super::options::Dhcpv6Option::from_raw(
            super::options::OPT_IA_NA,
            ia_na_data,
        ));

        options.push(super::options::Dhcpv6Option::from_raw(
            super::options::OPT_CLIENTID,
            client_duid.to_vec(),
        ));
        options.push(super::options::Dhcpv6Option::from_raw(
            super::options::OPT_SERVERID,
            server_duid.to_vec(),
        ));

        Self {
            msg_type: Dhcpv6MsgType::Release,
            transaction_id: TransactionId::random(),
            options,
        }
    }

    /// Serialize to wire format.
    pub fn to_wire(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.push(self.msg_type as u8);
        buf.extend_from_slice(&self.transaction_id.0);
        for opt in &self.options {
            opt.to_wire(&mut buf);
        }
        buf
    }

    /// Parse from wire format.
    pub fn from_wire(data: &[u8]) -> Result<Self, io::Error> {
        if data.len() < 4 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "DHCPv6 message too short",
            ));
        }

        let msg_type = Dhcpv6MsgType::from_u8(data[0]).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Unknown DHCPv6 type: {}", data[0]),
            )
        })?;

        let transaction_id = TransactionId([data[1], data[2], data[3]]);

        let options = super::options::Dhcpv6Option::parse_all(&data[4..])?;

        Ok(Self {
            msg_type,
            transaction_id,
            options,
        })
    }

    /// Get the Client DUID from options, if present.
    pub fn client_duid(&self) -> Option<Vec<u8>> {
        self.options
            .iter()
            .find(|o| o.code == super::options::OPT_CLIENTID)
            .map(|o| o.data.clone())
    }

    /// Get the Server DUID from options, if present.
    pub fn server_duid(&self) -> Option<Vec<u8>> {
        self.options
            .iter()
            .find(|o| o.code == super::options::OPT_SERVERID)
            .map(|o| o.data.clone())
    }

    /// Get IA_NA options from the message.
    pub fn ia_na_options(&self) -> Vec<&super::options::Dhcpv6Option> {
        self.options
            .iter()
            .filter(|o| o.code == super::options::OPT_IA_NA)
            .collect()
    }

    /// Get the IAID from the first IA_NA option.
    pub fn ia_id(&self) -> Option<u32> {
        self.ia_na_options().first().and_then(|opt| {
            if opt.data.len() >= 12 {
                Some(read_u32_be(&opt.data, 0))
            } else {
                None
            }
        })
    }

    /// Get the elapsed time option in milliseconds.
    pub fn elapsed_time_ms(&self) -> Option<u16> {
        self.options
            .iter()
            .find(|o| o.code == super::options::OPT_ELAPSED_TIME)
            .and_then(|o| {
                if o.data.len() >= 2 {
                    Some(read_u16_be(&o.data, 0))
                } else {
                    None
                }
            })
    }

    /// Get the Option Request List.
    pub fn option_request_list(&self) -> Vec<u16> {
        self.options
            .iter()
            .find(|o| o.code == super::options::OPT_ORO)
            .map(|o| {
                (0..o.data.len() / 2)
                    .map(|i| read_u16_be(&o.data, i * 2))
                    .collect()
            })
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_solicit_wire_roundtrip() {
        let client_duid = vec![0, 1, 0, 1, 0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff];
        let msg = Dhcpv6Message::solicit(0x12345678, &client_duid, 0);

        let wire = msg.to_wire();
        assert_eq!(wire[0], 1); // SOLICIT
        assert!(wire.len() > 4);

        let parsed = Dhcpv6Message::from_wire(&wire).unwrap();
        assert_eq!(parsed.msg_type, Dhcpv6MsgType::Solicit);
        assert_eq!(parsed.client_duid(), Some(client_duid));
        assert_eq!(parsed.ia_id(), Some(0x12345678));
    }

    #[test]
    fn test_request_wire_roundtrip() {
        let client_duid = vec![0, 3, 0, 1, 0xaa, 0xbb, 0xcc, 0xdd];
        let server_duid = vec![0, 1, 0, 1, 0x11, 0x22, 0x33, 0x44];
        let msg = Dhcpv6Message::request(1, &client_duid, &server_duid);

        let wire = msg.to_wire();
        let parsed = Dhcpv6Message::from_wire(&wire).unwrap();
        assert_eq!(parsed.msg_type, Dhcpv6MsgType::Request);
        assert_eq!(parsed.server_duid(), Some(server_duid));
    }

    #[test]
    fn test_transaction_id_random() {
        let t1 = TransactionId::random();
        let t2 = TransactionId::random();
        assert_ne!(t1, t2);
    }

    #[test]
    fn test_transaction_id_roundtrip() {
        let t = TransactionId::from_u32(0x010203);
        assert_eq!(t.as_u32(), 0x010203);
        assert_eq!(t.0, [0x01, 0x02, 0x03]);
    }

    #[test]
    fn test_msg_type_display() {
        assert_eq!(Dhcpv6MsgType::Solicit.to_string(), "SOLICIT");
        assert_eq!(
            Dhcpv6MsgType::InformationRequest.to_string(),
            "INFORMATION-REQUEST"
        );
    }

    #[test]
    fn test_information_request() {
        let client_duid = vec![0, 3, 0, 1, 0xaa, 0xbb, 0xcc, 0xdd];
        let msg = Dhcpv6Message::information_request(&client_duid);
        let wire = msg.to_wire();
        let parsed = Dhcpv6Message::from_wire(&wire).unwrap();
        assert_eq!(parsed.msg_type, Dhcpv6MsgType::InformationRequest);
        assert!(!parsed.option_request_list().is_empty());
    }
}
