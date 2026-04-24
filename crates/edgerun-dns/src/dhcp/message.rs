//! DHCPv4 message parser/serializer — RFC 2131 wire format.

use std::io;
use std::net::Ipv4Addr;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

pub const DHCP_SERVER_PORT: u16 = 67;
pub const DHCP_CLIENT_PORT: u16 = 68;

const BOOTREQUEST: u8 = 1;
const BOOTREPLY: u8 = 2;

const HTYPE_ETHER: u8 = 1;
const HLEN_ETHER: u8 = 6;

const DHCP_COOKIE: [u8; 4] = [99, 130, 83, 99]; // 0x63825363

// DHCP option codes
pub const OPT_SUBNET_MASK: u8 = 1;
pub const OPT_ROUTER: u8 = 3;
pub const OPT_DNS_SERVER: u8 = 6;
pub const OPT_HOST_NAME: u8 = 12;
pub const OPT_REQUESTED_IP: u8 = 50;
pub const OPT_LEASE_TIME: u8 = 51;
pub const OPT_MESSAGE_TYPE: u8 = 53;
pub const OPT_SERVER_ID: u8 = 54;
pub const OPT_PARAM_REQUEST: u8 = 55;
pub const OPT_RENEWAL_TIME: u8 = 58;
pub const OPT_REBIND_TIME: u8 = 59;
pub const OPT_CLIENT_ID: u8 = 61;
/// TFTP Server Name (PXE)
pub const OPT_TFTP_SERVER_NAME: u8 = 66;
/// Bootfile Name (PXE)
pub const OPT_BOOTFILE_NAME: u8 = 67;
/// Vendor-Specific Information (PXE sub-options in option 43)
pub const OPT_VENDOR_ENCAP: u8 = 43;
/// Client System Architecture Type (PXE)
pub const OPT_CLIENT_ARCH: u8 = 93;
/// Client Network Interface Identifier / UNDI version (PXE)
pub const OPT_CLIENT_NDI: u8 = 94;
/// Client Machine Identifier / UUID-GUID (PXE)
pub const OPT_CLIENT_MACHINE_ID: u8 = 97;
/// Domain Name (DNS domain, e.g. "example.com")
pub const OPT_DOMAIN_NAME: u8 = 15;
/// Broadcast Address
pub const OPT_BROADCAST_ADDR: u8 = 28;
/// Interface MTU
pub const OPT_INTERFACE_MTU: u8 = 26;
/// NTP Servers
pub const OPT_NTP_SERVERS: u8 = 42;
/// Maximum DHCP Message Size
pub const OPT_MAX_MSG_SIZE: u8 = 57;
/// Option Overload (sname/file fields carry options)
pub const OPT_OPTION_OVERLOAD: u8 = 52;
/// Classless Static Routes (RFC 3442)
pub const OPT_CLASSLESS_STATIC_ROUTES: u8 = 121;
/// Domain Search (RFC 3397)
pub const OPT_DOMAIN_SEARCH: u8 = 119;
/// Client FQDN (RFC 4702)
pub const OPT_CLIENT_FQDN: u8 = 81;
/// Vendor Class Identifier
pub const OPT_VENDOR_CLASS: u8 = 60;
/// Rapid Commit (RFC 4039) — 2-message DORA exchange
pub const OPT_RAPID_COMMIT: u8 = 80;
pub const OPT_END: u8 = 255;
pub const OPT_PAD: u8 = 0;

// DHCP message types
const MSG_DISCOVER: u8 = 1;
const MSG_OFFER: u8 = 2;
const MSG_REQUEST: u8 = 3;
const MSG_DECLINE: u8 = 4;
const MSG_ACK: u8 = 5;
const MSG_NAK: u8 = 6;
const MSG_RELEASE: u8 = 7;
const MSG_INFORM: u8 = 8;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// DHCP message type (op field).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DhcpOp {
    /// BOOTREQUEST — client → server
    Request = 1,
    /// BOOTREPLY — server → client
    Reply = 2,
}

/// DHCP message type (option 53).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DhcpMessageType {
    Discover = 1,
    Offer = 2,
    Request = 3,
    Decline = 4,
    Ack = 5,
    Nak = 6,
    Release = 7,
    Inform = 8,
}

/// A complete DHCP message on the wire.
#[derive(Debug, Clone)]
pub struct DhcpMessage {
    pub op: DhcpOp,
    pub htype: u8,
    pub hlen: u8,
    pub hops: u8,
    pub xid: u32,
    pub secs: u16,
    pub broadcast: bool,
    pub ciaddr: Ipv4Addr,
    pub yiaddr: Ipv4Addr,
    pub siaddr: Ipv4Addr,
    pub giaddr: Ipv4Addr,
    pub chaddr: [u8; 16],
    pub sname: [u8; 64],
    pub file: [u8; 128],
    pub options: DhcpOptions,
    /// True if this is a BOOTP message (no DHCP magic cookie).
    pub is_bootp: bool,
}

/// Parsed DHCP options.
#[derive(Debug, Clone, Default)]
pub struct DhcpOptions {
    pub message_type: Option<DhcpMessageType>,
    pub subnet_mask: Option<Ipv4Addr>,
    pub router: Option<Ipv4Addr>,
    pub dns_servers: Vec<Ipv4Addr>,
    pub requested_ip: Option<Ipv4Addr>,
    pub lease_time: Option<u32>,
    pub server_id: Option<Ipv4Addr>,
    pub renewal_time: Option<u32>,
    pub rebind_time: Option<u32>,
    pub client_id: Option<Vec<u8>>,
    pub param_request_list: Vec<u8>,
    // --- Common options ---
    /// DNS domain name (e.g. "example.com") — option 15.
    pub domain_name: Option<String>,
    /// Broadcast address — option 28.
    pub broadcast_addr: Option<Ipv4Addr>,
    /// Interface MTU — option 26.
    pub interface_mtu: Option<u16>,
    /// NTP servers — option 42.
    pub ntp_servers: Vec<Ipv4Addr>,
    /// Maximum DHCP message size — option 57.
    pub max_msg_size: Option<u16>,
    /// Option overload — option 52 (1=sname, 2=file, 3=both).
    pub option_overload: Option<u8>,
    /// Classless static routes — option 121 (RFC 3442).
    /// Each entry: (prefix_len, prefix_bytes, gateway).
    pub classless_routes: Vec<(u8, Vec<u8>, Ipv4Addr)>,
    /// DNS domain search list — option 119 (RFC 3397, compressed DNS format).
    pub domain_search: Option<Vec<u8>>,
    /// Client FQDN — option 81 (RFC 4702).
    pub client_fqdn: Option<Vec<u8>>,
    /// Vendor class identifier — option 60.
    pub vendor_class: Option<String>,
    // --- PXE Boot options ---
    /// TFTP server hostname or IP (option 66).
    pub tftp_server_name: Option<String>,
    /// Bootfile name/path (option 67).
    pub bootfile_name: Option<String>,
    /// Client system architecture type (option 93).
    pub client_arch: Option<PxeClientArch>,
    /// UNDI version (option 94): (type, major, minor).
    pub client_undi: Option<(u8, u8, u8)>,
    /// Client machine UUID-GUID (option 97).
    pub client_machine_id: Option<[u8; 17]>,
    /// Vendor-encapsulated options (option 43) — raw PXE sub-options.
    pub vendor_encap: Option<Vec<u8>>,
    /// Parsed vendor sub-options from option 43.
    pub vendor_sub_options: Vec<(u8, Vec<u8>)>,
    /// Hostname (option 12).
    pub host_name: Option<String>,
    /// Raw unparsed options (for extensions we don't understand).
    pub raw: Vec<(u8, Vec<u8>)>,
}

/// PXE client architecture type (option 93).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum PxeClientArch {
    IntelX86PC = 0,
    NecPC98 = 1,
    Itanium = 2,
    DecAlpha = 3,
    ArcX86 = 4,
    IntelLeanClient = 5,
    Ia32EFI = 6,
    EfiBC = 7,
    XscaleEFI = 8,
    X86_64EFI = 9,
    Arm32EFI = 10,
    Arm64EFI = 11,
    PowerPCOpenFirmware = 12,
    PowerOPEPDRIVER = 13,
    VirtEFI = 14,
    SalSystem32EFI = 15,
    SalSystem64EFI = 16,
    RiscVEfi = 19,
    RiscVEfi32 = 20,
    RiscVEfi64 = 21,
    RiscVEfi128 = 22,
    LoongArchEfi = 25,
    LoongArchEfi32 = 26,
    LoongArchEfi64 = 27,
    LoongArchEfi128 = 28,
}

impl PxeClientArch {
    pub fn from_u16(v: u16) -> Option<Self> {
        match v {
            0 => Some(Self::IntelX86PC),
            1 => Some(Self::NecPC98),
            2 => Some(Self::Itanium),
            3 => Some(Self::DecAlpha),
            4 => Some(Self::ArcX86),
            5 => Some(Self::IntelLeanClient),
            6 => Some(Self::Ia32EFI),
            7 => Some(Self::EfiBC),
            8 => Some(Self::XscaleEFI),
            9 => Some(Self::X86_64EFI),
            10 => Some(Self::Arm32EFI),
            11 => Some(Self::Arm64EFI),
            12 => Some(Self::PowerPCOpenFirmware),
            13 => Some(Self::PowerOPEPDRIVER),
            14 => Some(Self::VirtEFI),
            15 => Some(Self::SalSystem32EFI),
            16 => Some(Self::SalSystem64EFI),
            19 => Some(Self::RiscVEfi),
            20 => Some(Self::RiscVEfi32),
            21 => Some(Self::RiscVEfi64),
            22 => Some(Self::RiscVEfi128),
            25 => Some(Self::LoongArchEfi),
            26 => Some(Self::LoongArchEfi32),
            27 => Some(Self::LoongArchEfi64),
            28 => Some(Self::LoongArchEfi128),
            _ => None,
        }
    }

    pub fn as_u16(self) -> u16 {
        self as u16
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::IntelX86PC => "Intel x86PC",
            Self::NecPC98 => "NEC PC-98",
            Self::Itanium => "Itanium",
            Self::DecAlpha => "DEC Alpha",
            Self::ArcX86 => "ARC x86",
            Self::IntelLeanClient => "Intel Lean Client",
            Self::Ia32EFI => "IA32 EFI",
            Self::EfiBC => "EFI BC",
            Self::XscaleEFI => "Xscale EFI",
            Self::X86_64EFI => "x86-64 EFI",
            Self::Arm32EFI => "ARM32 EFI",
            Self::Arm64EFI => "ARM64 EFI",
            Self::PowerPCOpenFirmware => "PowerPC Open Firmware",
            Self::PowerOPEPDRIVER => "PowerOP EPD Driver",
            Self::VirtEFI => "Virt EFI",
            Self::SalSystem32EFI => "SAL System 32-bit EFI",
            Self::SalSystem64EFI => "SAL System 64-bit EFI",
            Self::RiscVEfi => "RISC-V EFI",
            Self::RiscVEfi32 => "RISC-V EFI 32-bit",
            Self::RiscVEfi64 => "RISC-V EFI 64-bit",
            Self::RiscVEfi128 => "RISC-V EFI 128-bit",
            Self::LoongArchEfi => "LoongArch EFI",
            Self::LoongArchEfi32 => "LoongArch EFI 32-bit",
            Self::LoongArchEfi64 => "LoongArch EFI 64-bit",
            Self::LoongArchEfi128 => "LoongArch EFI 128-bit",
        }
    }

    /// PXE boot file prefix for this architecture.
    pub fn default_bootfile(&self) -> &'static str {
        match self {
            Self::IntelX86PC => "pxelinux.0",
            Self::Ia32EFI => "bootia32.efi",
            Self::X86_64EFI => "bootx64.efi",
            Self::Arm32EFI => "bootarm.efi",
            Self::Arm64EFI => "bootaa64.efi",
            _ => "pxelinux.0",
        }
    }
}

/// Builder for DHCP network configuration parameters.
pub struct NetworkConfig {
    pub server_id: Ipv4Addr,
    pub subnet_mask: Ipv4Addr,
    pub router: Ipv4Addr,
    pub dns_servers: Vec<Ipv4Addr>,
    pub lease_time: u32,
    pub tftp_server: Option<String>,
    pub bootfile: Option<String>,
}

impl DhcpMessage {
    /// Minimum size of a DHCP message header (236 bytes fixed + 4 byte cookie).
    pub const HEADER_SIZE: usize = 240;

    /// Serialize this DHCP message to wire format.
    pub fn to_wire(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(Self::HEADER_SIZE + 64);

        // Fixed header (236 bytes)
        buf.push(self.op as u8);
        buf.push(self.htype);
        buf.push(self.hlen);
        buf.push(self.hops);
        buf.extend_from_slice(&self.xid.to_be_bytes());
        buf.extend_from_slice(&self.secs.to_be_bytes());
        let flags: u16 = if self.broadcast { 0x8000 } else { 0 };
        buf.extend_from_slice(&flags.to_be_bytes());
        buf.extend_from_slice(&self.ciaddr.octets());
        buf.extend_from_slice(&self.yiaddr.octets());
        buf.extend_from_slice(&self.siaddr.octets());
        buf.extend_from_slice(&self.giaddr.octets());
        buf.extend_from_slice(&self.chaddr);
        buf.extend_from_slice(&self.sname);
        buf.extend_from_slice(&self.file);

        // Magic cookie and options (omit for BOOTP)
        if !self.is_bootp {
            buf.extend_from_slice(&DHCP_COOKIE);
            self.options.serialize(&mut buf);
        }

        buf
    }

    /// Parse a DHCP message from wire format.
    pub fn from_wire(data: &[u8]) -> Result<Self, io::Error> {
        // Minimum: 236 bytes for BOOTP (no magic cookie), 240 for DHCP
        if data.len() < 236 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("DHCP/BOOTP message too short: {} bytes (min 236)", data.len()),
            ));
        }

        let op = match data[0] {
            BOOTREQUEST => DhcpOp::Request,
            BOOTREPLY => DhcpOp::Reply,
            v => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("Invalid DHCP op: {}", v),
                ))
            }
        };

        let htype = data[1];
        let hlen = data[2];
        let hops = data[3];
        let xid = u32::from_be_bytes([data[4], data[5], data[6], data[7]]);
        let secs = u16::from_be_bytes([data[8], data[9]]);
        let flags = u16::from_be_bytes([data[10], data[11]]);
        let broadcast = (flags & 0x8000) != 0;

        let ciaddr = Ipv4Addr::new(data[12], data[13], data[14], data[15]);
        let yiaddr = Ipv4Addr::new(data[16], data[17], data[18], data[19]);
        let siaddr = Ipv4Addr::new(data[20], data[21], data[22], data[23]);
        let giaddr = Ipv4Addr::new(data[24], data[25], data[26], data[27]);

        let mut chaddr = [0u8; 16];
        chaddr.copy_from_slice(&data[28..44]);

        let mut sname = [0u8; 64];
        sname.copy_from_slice(&data[44..108]);

        let mut file = [0u8; 128];
        if data.len() >= 236 {
            file.copy_from_slice(&data[108..236]);
        }

        // Check for DHCP magic cookie — if absent or message too short, this is BOOTP (RFC 951)
        let is_bootp = data.len() < 240 || data[236..240] != DHCP_COOKIE;

        let options = if is_bootp {
            // BOOTP: no options, use sname/file fields
            DhcpOptions::default()
        } else {
            DhcpOptions::parse(&data[240..])?
        };

        Ok(Self {
            op,
            htype,
            hlen,
            hops,
            xid,
            secs,
            broadcast,
            ciaddr,
            yiaddr,
            siaddr,
            giaddr,
            chaddr,
            sname,
            file,
            options,
            is_bootp,
        })
    }

    /// Create a new DHCP DISCOVER message.
    pub fn discover(xid: u32, mac: [u8; 6]) -> Self {
        Self {
            op: DhcpOp::Request,
            htype: HTYPE_ETHER,
            hlen: HLEN_ETHER,
            hops: 0,
            xid,
            secs: 0,
            broadcast: true,
            ciaddr: Ipv4Addr::UNSPECIFIED,
            yiaddr: Ipv4Addr::UNSPECIFIED,
            siaddr: Ipv4Addr::UNSPECIFIED,
            giaddr: Ipv4Addr::UNSPECIFIED,
            chaddr: mac_to_chaddr(mac),
            sname: [0; 64],
            file: [0; 128],
            is_bootp: false,
            options: DhcpOptions {
                message_type: Some(DhcpMessageType::Discover),
                param_request_list: vec![
                    OPT_SUBNET_MASK,
                    OPT_ROUTER,
                    OPT_DNS_SERVER,
                    OPT_LEASE_TIME,
                    OPT_RENEWAL_TIME,
                    OPT_REBIND_TIME,
                ],
                ..Default::default()
            },
        }
    }

    /// Create a DHCP REQUEST message.
    pub fn request(xid: u32, mac: [u8; 6], requested_ip: Ipv4Addr, server_id: Ipv4Addr) -> Self {
        Self {
            op: DhcpOp::Request,
            htype: HTYPE_ETHER,
            hlen: HLEN_ETHER,
            hops: 0,
            xid,
            secs: 0,
            broadcast: true,
            ciaddr: Ipv4Addr::UNSPECIFIED,
            yiaddr: Ipv4Addr::UNSPECIFIED,
            siaddr: Ipv4Addr::UNSPECIFIED,
            giaddr: Ipv4Addr::UNSPECIFIED,
            chaddr: mac_to_chaddr(mac),
            sname: [0; 64],
            file: [0; 128],
            is_bootp: false,
            options: DhcpOptions {
                message_type: Some(DhcpMessageType::Request),
                requested_ip: Some(requested_ip),
                server_id: Some(server_id),
                param_request_list: vec![
                    OPT_SUBNET_MASK,
                    OPT_ROUTER,
                    OPT_DNS_SERVER,
                    OPT_LEASE_TIME,
                    OPT_RENEWAL_TIME,
                    OPT_REBIND_TIME,
                ],
                ..Default::default()
            },
        }
    }

    /// Create a DHCP RELEASE message.
    pub fn release(xid: u32, mac: [u8; 6], ciaddr: Ipv4Addr, server_id: Ipv4Addr) -> Self {
        Self {
            op: DhcpOp::Request,
            htype: HTYPE_ETHER,
            hlen: HLEN_ETHER,
            hops: 0,
            xid,
            secs: 0,
            broadcast: false,
            ciaddr,
            yiaddr: Ipv4Addr::UNSPECIFIED,
            siaddr: Ipv4Addr::UNSPECIFIED,
            giaddr: Ipv4Addr::UNSPECIFIED,
            chaddr: mac_to_chaddr(mac),
            sname: [0; 64],
            file: [0; 128],
            is_bootp: false,
            options: DhcpOptions {
                message_type: Some(DhcpMessageType::Release),
                server_id: Some(server_id),
                ..Default::default()
            },
        }
    }

    /// Create a DHCP OFFER (server → client).
    pub fn offer(
        xid: u32,
        mac: [u8; 6],
        yiaddr: Ipv4Addr,
        net: NetworkConfig,
    ) -> Self {
        Self {
            op: DhcpOp::Reply,
            htype: HTYPE_ETHER,
            hlen: HLEN_ETHER,
            hops: 0,
            xid,
            secs: 0,
            broadcast: false,
            ciaddr: Ipv4Addr::UNSPECIFIED,
            yiaddr,
            siaddr: Ipv4Addr::UNSPECIFIED,
            giaddr: Ipv4Addr::UNSPECIFIED,
            chaddr: mac_to_chaddr(mac),
            sname: [0; 64],
            file: [0; 128],
            is_bootp: false,
            options: DhcpOptions {
                message_type: Some(DhcpMessageType::Offer),
                subnet_mask: Some(net.subnet_mask),
                router: Some(net.router),
                dns_servers: net.dns_servers,
                server_id: Some(net.server_id),
                lease_time: Some(net.lease_time),
                renewal_time: Some(net.lease_time / 2),
                rebind_time: Some(net.lease_time * 7 / 8),
                tftp_server_name: net.tftp_server,
                bootfile_name: net.bootfile,
                ..Default::default()
            },
        }
    }

    /// Create a DHCP ACK (server → client).
    pub fn ack(
        xid: u32,
        mac: [u8; 6],
        yiaddr: Ipv4Addr,
        net: NetworkConfig,
    ) -> Self {
        Self {
            op: DhcpOp::Reply,
            htype: HTYPE_ETHER,
            hlen: HLEN_ETHER,
            hops: 0,
            xid,
            secs: 0,
            broadcast: false,
            ciaddr: Ipv4Addr::UNSPECIFIED,
            yiaddr,
            siaddr: Ipv4Addr::UNSPECIFIED,
            giaddr: Ipv4Addr::UNSPECIFIED,
            chaddr: mac_to_chaddr(mac),
            sname: [0; 64],
            file: [0; 128],
            is_bootp: false,
            options: DhcpOptions {
                message_type: Some(DhcpMessageType::Ack),
                subnet_mask: Some(net.subnet_mask),
                router: Some(net.router),
                dns_servers: net.dns_servers,
                server_id: Some(net.server_id),
                lease_time: Some(net.lease_time),
                renewal_time: Some(net.lease_time / 2),
                rebind_time: Some(net.lease_time * 7 / 8),
                tftp_server_name: net.tftp_server,
                bootfile_name: net.bootfile,
                ..Default::default()
            },
        }
    }

    /// Create a DHCP NAK (server → client).
    /// Per RFC 2131 §4.3.2, NAK MUST be broadcast and include the client's chaddr.
    pub fn nak(xid: u32, server_id: Ipv4Addr, client_mac: [u8; 6]) -> Self {
        Self {
            op: DhcpOp::Reply,
            htype: HTYPE_ETHER,
            hlen: HLEN_ETHER,
            hops: 0,
            xid,
            secs: 0,
            broadcast: true,
            ciaddr: Ipv4Addr::UNSPECIFIED,
            yiaddr: Ipv4Addr::UNSPECIFIED,
            siaddr: Ipv4Addr::UNSPECIFIED,
            giaddr: Ipv4Addr::UNSPECIFIED,
            chaddr: mac_to_chaddr(client_mac),
            sname: [0; 64],
            file: [0; 128],
            is_bootp: false,
            options: DhcpOptions {
                message_type: Some(DhcpMessageType::Nak),
                server_id: Some(server_id),
                ..Default::default()
            },
        }
    }

    /// Extract the MAC address from the chaddr field.
    pub fn client_mac(&self) -> [u8; 6] {
        let mut mac = [0u8; 6];
        mac.copy_from_slice(&self.chaddr[..6]);
        mac
    }
}

// ---------------------------------------------------------------------------
// Options parsing / serialization
// ---------------------------------------------------------------------------

impl DhcpOptions {
    fn parse(data: &[u8]) -> Result<Self, io::Error> {
        let mut opts = Self::default();
        let mut i = 0;
        while i < data.len() {
            let code = data[i];
            if code == OPT_END {
                break;
            }
            if code == OPT_PAD {
                i += 1;
                continue;
            }
            i += 1;
            if i >= data.len() {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "Truncated DHCP option",
                ));
            }
            let len = data[i] as usize;
            i += 1;
            if i + len > data.len() {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "Truncated DHCP option data",
                ));
            }
            let value = &data[i..i + len];
            i += len;

            match code {
                OPT_MESSAGE_TYPE => {
                    if !value.is_empty() {
                        opts.message_type = match value[0] {
                            MSG_DISCOVER => Some(DhcpMessageType::Discover),
                            MSG_OFFER => Some(DhcpMessageType::Offer),
                            MSG_REQUEST => Some(DhcpMessageType::Request),
                            MSG_DECLINE => Some(DhcpMessageType::Decline),
                            MSG_ACK => Some(DhcpMessageType::Ack),
                            MSG_NAK => Some(DhcpMessageType::Nak),
                            MSG_RELEASE => Some(DhcpMessageType::Release),
                            MSG_INFORM => Some(DhcpMessageType::Inform),
                            v => {
                                opts.raw.push((code, value.to_vec()));
                                eprintln!("Unknown DHCP message type: {}", v);
                                None
                            }
                        };
                    }
                }
                OPT_SUBNET_MASK => {
                    if value.len() == 4 {
                        opts.subnet_mask = Some(Ipv4Addr::new(value[0], value[1], value[2], value[3]));
                    }
                }
                OPT_ROUTER => {
                    if value.len() >= 4 {
                        opts.router = Some(Ipv4Addr::new(value[0], value[1], value[2], value[3]));
                    }
                }
                OPT_DNS_SERVER => {
                    for chunk in value.chunks(4) {
                        if chunk.len() == 4 {
                            opts.dns_servers.push(Ipv4Addr::new(chunk[0], chunk[1], chunk[2], chunk[3]));
                        }
                    }
                }
                OPT_REQUESTED_IP => {
                    if value.len() == 4 {
                        opts.requested_ip = Some(Ipv4Addr::new(value[0], value[1], value[2], value[3]));
                    }
                }
                OPT_LEASE_TIME => {
                    if value.len() == 4 {
                        opts.lease_time = Some(u32::from_be_bytes([value[0], value[1], value[2], value[3]]));
                    }
                }
                OPT_SERVER_ID => {
                    if value.len() == 4 {
                        opts.server_id = Some(Ipv4Addr::new(value[0], value[1], value[2], value[3]));
                    }
                }
                OPT_RENEWAL_TIME => {
                    if value.len() == 4 {
                        opts.renewal_time = Some(u32::from_be_bytes([value[0], value[1], value[2], value[3]]));
                    }
                }
                OPT_REBIND_TIME => {
                    if value.len() == 4 {
                        opts.rebind_time = Some(u32::from_be_bytes([value[0], value[1], value[2], value[3]]));
                    }
                }
                OPT_CLIENT_ID => {
                    opts.client_id = Some(value.to_vec());
                }
                OPT_PARAM_REQUEST => {
                    opts.param_request_list = value.to_vec();
                }
                OPT_TFTP_SERVER_NAME => {
                    opts.tftp_server_name = Some(String::from_utf8_lossy(value).to_string());
                }
                OPT_BOOTFILE_NAME => {
                    opts.bootfile_name = Some(String::from_utf8_lossy(value).to_string());
                }
                OPT_CLIENT_ARCH => {
                    if value.len() >= 2 {
                        let arch = u16::from_be_bytes([value[0], value[1]]);
                        opts.client_arch = PxeClientArch::from_u16(arch);
                    }
                }
                OPT_CLIENT_NDI => {
                    if value.len() >= 3 {
                        opts.client_undi = Some((value[0], value[1], value[2]));
                    }
                }
                OPT_CLIENT_MACHINE_ID => {
                    if value.len() == 17 {
                        let mut guid = [0u8; 17];
                        guid.copy_from_slice(value);
                        opts.client_machine_id = Some(guid);
                    }
                }
                OPT_VENDOR_ENCAP => {
                    opts.vendor_encap = Some(value.to_vec());
                    // Parse vendor sub-options (RFC 4578 PXE format)
                    let mut j = 0;
                    while j < value.len() {
                        let sub_code = value[j];
                        if sub_code == 255 { break; } // End
                        if sub_code == 0 { j += 1; continue; } // Pad
                        j += 1;
                        if j >= value.len() { break; }
                        let sub_len = value[j] as usize;
                        j += 1;
                        if j + sub_len > value.len() { break; }
                        opts.vendor_sub_options.push((sub_code, value[j..j+sub_len].to_vec()));
                        j += sub_len;
                    }
                }
                OPT_HOST_NAME => {
                    opts.host_name = Some(String::from_utf8_lossy(value).to_string());
                }
                OPT_DOMAIN_NAME => {
                    opts.domain_name = Some(String::from_utf8_lossy(value).to_string());
                }
                OPT_BROADCAST_ADDR => {
                    if value.len() == 4 {
                        opts.broadcast_addr = Some(Ipv4Addr::new(value[0], value[1], value[2], value[3]));
                    }
                }
                OPT_INTERFACE_MTU => {
                    if value.len() == 2 {
                        opts.interface_mtu = Some(u16::from_be_bytes([value[0], value[1]]));
                    }
                }
                OPT_NTP_SERVERS => {
                    for chunk in value.chunks(4) {
                        if chunk.len() == 4 {
                            opts.ntp_servers.push(Ipv4Addr::new(chunk[0], chunk[1], chunk[2], chunk[3]));
                        }
                    }
                }
                OPT_MAX_MSG_SIZE => {
                    if value.len() == 2 {
                        opts.max_msg_size = Some(u16::from_be_bytes([value[0], value[1]]));
                    }
                }
                OPT_OPTION_OVERLOAD => {
                    if !value.is_empty() {
                        opts.option_overload = Some(value[0]);
                    }
                }
                OPT_CLASSLESS_STATIC_ROUTES => {
                    // RFC 3442: dest-len(1) | dest-prefix(n) | gateway(4)
                    let mut j = 0;
                    while j + 5 <= value.len() {
                        let prefix_len = value[j];
                        let prefix_bytes = prefix_len.div_ceil(8); // ceiling division
                        j += 1;
                        if j + prefix_bytes as usize + 4 > value.len() { break; }
                        let prefix = value[j..j + prefix_bytes as usize].to_vec();
                        j += prefix_bytes as usize;
                        let gateway = Ipv4Addr::new(value[j], value[j+1], value[j+2], value[j+3]);
                        j += 4;
                        opts.classless_routes.push((prefix_len, prefix, gateway));
                    }
                }
                OPT_DOMAIN_SEARCH => {
                    opts.domain_search = Some(value.to_vec());
                }
                OPT_CLIENT_FQDN => {
                    opts.client_fqdn = Some(value.to_vec());
                }
                OPT_VENDOR_CLASS => {
                    opts.vendor_class = Some(String::from_utf8_lossy(value).to_string());
                }
                _ => {
                    opts.raw.push((code, value.to_vec()));
                }
            }
        }
        Ok(opts)
    }

    fn serialize(&self, buf: &mut Vec<u8>) {
        // Message type is required
        if let Some(mt) = self.message_type {
            buf.push(OPT_MESSAGE_TYPE);
            buf.push(1);
            buf.push(match mt {
                DhcpMessageType::Discover => MSG_DISCOVER,
                DhcpMessageType::Offer => MSG_OFFER,
                DhcpMessageType::Request => MSG_REQUEST,
                DhcpMessageType::Decline => MSG_DECLINE,
                DhcpMessageType::Ack => MSG_ACK,
                DhcpMessageType::Nak => MSG_NAK,
                DhcpMessageType::Release => MSG_RELEASE,
                DhcpMessageType::Inform => MSG_INFORM,
            });
        }

        if let Some(ip) = self.subnet_mask {
            buf.push(OPT_SUBNET_MASK);
            buf.push(4);
            buf.extend_from_slice(&ip.octets());
        }

        if let Some(ip) = self.router {
            buf.push(OPT_ROUTER);
            buf.push(4);
            buf.extend_from_slice(&ip.octets());
        }

        if !self.dns_servers.is_empty() {
            let dns_len = self.dns_servers.len() * 4;
            buf.push(OPT_DNS_SERVER);
            buf.push(dns_len.min(255) as u8);
            for s in &self.dns_servers[..self.dns_servers.len().min(63)] {
                buf.extend_from_slice(&s.octets());
            }
        }

        if let Some(name) = &self.domain_name {
            let bytes = name.as_bytes();
            buf.push(OPT_DOMAIN_NAME);
            buf.push(bytes.len().min(255) as u8);
            buf.extend_from_slice(&bytes[..bytes.len().min(255)]);
        }

        if let Some(ip) = self.broadcast_addr {
            buf.push(OPT_BROADCAST_ADDR);
            buf.push(4);
            buf.extend_from_slice(&ip.octets());
        }

        if let Some(mtu) = self.interface_mtu {
            buf.push(OPT_INTERFACE_MTU);
            buf.push(2);
            buf.extend_from_slice(&mtu.to_be_bytes());
        }

        if !self.ntp_servers.is_empty() {
            let ntp_len = (self.ntp_servers.len() * 4).min(255);
            buf.push(OPT_NTP_SERVERS);
            buf.push(ntp_len as u8);
            for s in &self.ntp_servers[..self.ntp_servers.len().min(63)] {
                buf.extend_from_slice(&s.octets());
            }
        }

        if let Some(size) = self.max_msg_size {
            buf.push(OPT_MAX_MSG_SIZE);
            buf.push(2);
            buf.extend_from_slice(&size.to_be_bytes());
        }

        if !self.classless_routes.is_empty() {
            let mut route_buf = Vec::new();
            for (prefix_len, prefix, gateway) in &self.classless_routes {
                route_buf.push(*prefix_len);
                route_buf.extend_from_slice(prefix);
                route_buf.extend_from_slice(&gateway.octets());
            }
            if route_buf.len() <= 255 {
                buf.push(OPT_CLASSLESS_STATIC_ROUTES);
                buf.push(route_buf.len() as u8);
                buf.extend_from_slice(&route_buf);
            }
        }

        if let Some(search) = &self.domain_search {
            if search.len() <= 255 {
                buf.push(OPT_DOMAIN_SEARCH);
                buf.push(search.len() as u8);
                buf.extend_from_slice(search);
            }
        }

        if let Some(fqdn) = &self.client_fqdn {
            if fqdn.len() <= 255 {
                buf.push(OPT_CLIENT_FQDN);
                buf.push(fqdn.len() as u8);
                buf.extend_from_slice(fqdn);
            }
        }

        if let Some(vc) = &self.vendor_class {
            let bytes = vc.as_bytes();
            if bytes.len() <= 255 {
                buf.push(OPT_VENDOR_CLASS);
                buf.push(bytes.len() as u8);
                buf.extend_from_slice(bytes);
            }
        }

        if let Some(ip) = self.requested_ip {
            buf.push(OPT_REQUESTED_IP);
            buf.push(4);
            buf.extend_from_slice(&ip.octets());
        }

        if let Some(t) = self.lease_time {
            buf.push(OPT_LEASE_TIME);
            buf.push(4);
            buf.extend_from_slice(&t.to_be_bytes());
        }

        if let Some(ip) = self.server_id {
            buf.push(OPT_SERVER_ID);
            buf.push(4);
            buf.extend_from_slice(&ip.octets());
        }

        if let Some(t) = self.renewal_time {
            buf.push(OPT_RENEWAL_TIME);
            buf.push(4);
            buf.extend_from_slice(&t.to_be_bytes());
        }

        if let Some(t) = self.rebind_time {
            buf.push(OPT_REBIND_TIME);
            buf.push(4);
            buf.extend_from_slice(&t.to_be_bytes());
        }

        if let Some(cid) = &self.client_id {
            buf.push(OPT_CLIENT_ID);
            buf.push(cid.len() as u8);
            buf.extend_from_slice(cid);
        }

        if !self.param_request_list.is_empty() {
            buf.push(OPT_PARAM_REQUEST);
            buf.push(self.param_request_list.len() as u8);
            buf.extend_from_slice(&self.param_request_list);
        }

        // --- PXE options ---
        if let Some(name) = &self.tftp_server_name {
            let bytes = name.as_bytes();
            buf.push(OPT_TFTP_SERVER_NAME);
            buf.push(bytes.len() as u8);
            buf.extend_from_slice(bytes);
        }

        if let Some(name) = &self.bootfile_name {
            let bytes = name.as_bytes();
            buf.push(OPT_BOOTFILE_NAME);
            buf.push(bytes.len() as u8);
            buf.extend_from_slice(bytes);
        }

        if let Some(arch) = self.client_arch {
            buf.push(OPT_CLIENT_ARCH);
            buf.push(2);
            buf.extend_from_slice(&arch.as_u16().to_be_bytes());
        }

        if let Some((typ, major, minor)) = self.client_undi {
            buf.push(OPT_CLIENT_NDI);
            buf.push(3);
            buf.push(typ);
            buf.push(major);
            buf.push(minor);
        }

        if let Some(guid) = self.client_machine_id {
            buf.push(OPT_CLIENT_MACHINE_ID);
            buf.push(17);
            buf.extend_from_slice(&guid);
        }

        if let Some(encap) = &self.vendor_encap {
            buf.push(OPT_VENDOR_ENCAP);
            buf.push(encap.len() as u8);
            buf.extend_from_slice(encap);
        }

        if let Some(name) = &self.host_name {
            let bytes = name.as_bytes();
            buf.push(OPT_HOST_NAME);
            buf.push(bytes.len() as u8);
            buf.extend_from_slice(bytes);
        }

        for (code, value) in &self.raw {
            buf.push(*code);
            buf.push(value.len() as u8);
            buf.extend_from_slice(value);
        }

        // End option
        buf.push(OPT_END);
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn mac_to_chaddr(mac: [u8; 6]) -> [u8; 16] {
    let mut chaddr = [0u8; 16];
    chaddr[..6].copy_from_slice(&mac);
    chaddr
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_discover_roundtrip() {
        let mac = [0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff];
        let msg = DhcpMessage::discover(0x12345678, mac);
        let wire = msg.to_wire();
        let parsed = DhcpMessage::from_wire(&wire).unwrap();

        assert_eq!(parsed.op, DhcpOp::Request);
        assert_eq!(parsed.xid, 0x12345678);
        assert_eq!(parsed.client_mac(), mac);
        assert_eq!(parsed.options.message_type, Some(DhcpMessageType::Discover));
        assert!(parsed.broadcast);
    }

    #[test]
    fn test_offer_roundtrip() {
        let mac = [0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff];
        let server = Ipv4Addr::new(192, 168, 1, 1);
        let net = NetworkConfig {
            server_id: server,
            subnet_mask: Ipv4Addr::new(255, 255, 255, 0),
            router: server,
            dns_servers: vec![Ipv4Addr::new(8, 8, 8, 8)],
            lease_time: 86400,
            tftp_server: Some("192.168.1.1".to_string()),
            bootfile: Some("bootx64.efi".to_string()),
        };
        let msg = DhcpMessage::offer(
            0x12345678,
            mac,
            Ipv4Addr::new(192, 168, 1, 100),
            net,
        );
        let wire = msg.to_wire();
        let parsed = DhcpMessage::from_wire(&wire).unwrap();

        assert_eq!(parsed.op, DhcpOp::Reply);
        assert_eq!(parsed.options.message_type, Some(DhcpMessageType::Offer));
        assert_eq!(parsed.yiaddr, Ipv4Addr::new(192, 168, 1, 100));
        assert_eq!(parsed.options.server_id, Some(server));
        assert_eq!(parsed.options.lease_time, Some(86400));
        assert_eq!(parsed.options.dns_servers, vec![Ipv4Addr::new(8, 8, 8, 8)]);
        assert_eq!(parsed.options.tftp_server_name, Some("192.168.1.1".to_string()));
        assert_eq!(parsed.options.bootfile_name, Some("bootx64.efi".to_string()));
    }

    #[test]
    fn test_ack_roundtrip() {
        let mac = [0x11, 0x22, 0x33, 0x44, 0x55, 0x66];
        let msg = DhcpMessage::ack(
            0xabcdef00,
            mac,
            Ipv4Addr::new(10, 0, 0, 50),
            NetworkConfig {
                server_id: Ipv4Addr::new(10, 0, 0, 1),
                subnet_mask: Ipv4Addr::new(255, 0, 0, 0),
                router: Ipv4Addr::new(10, 0, 0, 1),
                dns_servers: vec![Ipv4Addr::new(1, 1, 1, 1)],
                lease_time: 3600,
                tftp_server: Some("10.0.0.1".to_string()),
                bootfile: Some("bootx64.efi".to_string()),
            },
        );
        let wire = msg.to_wire();
        let parsed = DhcpMessage::from_wire(&wire).unwrap();

        assert_eq!(parsed.options.message_type, Some(DhcpMessageType::Ack));
        assert_eq!(parsed.options.lease_time, Some(3600));
        assert_eq!(parsed.options.renewal_time, Some(1800));
        assert_eq!(parsed.options.rebind_time, Some(3150));
        assert_eq!(parsed.options.tftp_server_name, Some("10.0.0.1".to_string()));
        assert_eq!(parsed.options.bootfile_name, Some("bootx64.efi".to_string()));
    }

    #[test]
    fn test_nak_roundtrip() {
        let msg = DhcpMessage::nak(0xdeadbeef, Ipv4Addr::new(192, 168, 1, 1), [0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF]);
        let wire = msg.to_wire();
        let parsed = DhcpMessage::from_wire(&wire).unwrap();

        assert_eq!(parsed.options.message_type, Some(DhcpMessageType::Nak));
        assert_eq!(parsed.options.server_id, Some(Ipv4Addr::new(192, 168, 1, 1)));
    }

    #[test]
    fn test_request_roundtrip() {
        let mac = [0x01, 0x02, 0x03, 0x04, 0x05, 0x06];
        let msg = DhcpMessage::request(
            0x11223344,
            mac,
            Ipv4Addr::new(172, 16, 0, 100),
            Ipv4Addr::new(172, 16, 0, 1),
        );
        let wire = msg.to_wire();
        let parsed = DhcpMessage::from_wire(&wire).unwrap();

        assert_eq!(parsed.options.message_type, Some(DhcpMessageType::Request));
        assert_eq!(parsed.options.requested_ip, Some(Ipv4Addr::new(172, 16, 0, 100)));
        assert_eq!(parsed.options.server_id, Some(Ipv4Addr::new(172, 16, 0, 1)));
    }

    #[test]
    fn test_release_roundtrip() {
        let mac = [0x01, 0x02, 0x03, 0x04, 0x05, 0x06];
        let msg = DhcpMessage::release(
            0x55667788,
            mac,
            Ipv4Addr::new(192, 168, 0, 50),
            Ipv4Addr::new(192, 168, 0, 1),
        );
        let wire = msg.to_wire();
        let parsed = DhcpMessage::from_wire(&wire).unwrap();

        assert_eq!(parsed.options.message_type, Some(DhcpMessageType::Release));
        assert_eq!(parsed.ciaddr, Ipv4Addr::new(192, 168, 0, 50));
    }

    #[test]
    fn test_invalid_wire_format() {
        // Too short (less than 236 bytes — minimum for BOOTP header)
        assert!(DhcpMessage::from_wire(&[0u8; 100]).is_err());
        // Valid BOOTP message (no magic cookie, 236 bytes) — should succeed
        let mut buf = vec![0u8; 236];
        buf[0] = BOOTREQUEST;
        let msg = DhcpMessage::from_wire(&buf).unwrap();
        assert!(msg.is_bootp);
    }

    #[test]
    fn test_message_type_display() {
        assert_eq!(DhcpMessageType::Discover as u8, MSG_DISCOVER);
        assert_eq!(DhcpMessageType::Offer as u8, MSG_OFFER);
        assert_eq!(DhcpMessageType::Request as u8, MSG_REQUEST);
        assert_eq!(DhcpMessageType::Ack as u8, MSG_ACK);
        assert_eq!(DhcpMessageType::Nak as u8, MSG_NAK);
    }

    #[test]
    fn test_min_message_size() {
        assert_eq!(DhcpMessage::HEADER_SIZE, 240);
    }

    #[test]
    fn test_bootp_message() {
        // Construct a BOOTP message (no DHCP magic cookie)
        let mut buf = vec![0u8; 240]; // minimum BOOTP size
        buf[0] = BOOTREQUEST; // op = BOOTREQUEST
        buf[1] = 1; // htype = Ethernet
        buf[2] = 6; // hlen = 6
        buf[3] = 0; // hops
        buf[4..8].copy_from_slice(&0x12345678u32.to_be_bytes()); // xid
        // sname is 64 bytes at offset 44
        let sname_bytes = b"bootserver";
        buf[44..44+sname_bytes.len()].copy_from_slice(sname_bytes);
        // file is 128 bytes at offset 108
        let file_bytes = b"pxelinux.0";
        buf[108..108+file_bytes.len()].copy_from_slice(file_bytes);

        let msg = DhcpMessage::from_wire(&buf).unwrap();
        assert!(msg.is_bootp);
        assert_eq!(msg.op, DhcpOp::Request);
        assert_eq!(msg.xid, 0x12345678);
        // sname should have data
        assert!(msg.sname.iter().any(|&b| b != 0));
    }

    #[test]
    fn test_bootp_response_serialization() {
        let mut msg = DhcpMessage::offer(
            0x12345678,
            [0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff],
            Ipv4Addr::new(192, 168, 1, 100),
            NetworkConfig {
                server_id: Ipv4Addr::new(192, 168, 1, 1),
                subnet_mask: Ipv4Addr::new(255, 255, 255, 0),
                router: Ipv4Addr::new(192, 168, 1, 1),
                dns_servers: vec![],
                lease_time: 86400,
                tftp_server: Some("192.168.1.1".to_string()),
                bootfile: Some("pxelinux.0".to_string()),
            },
        );

        // Convert to BOOTP response
        msg.is_bootp = true;
        msg.siaddr = Ipv4Addr::new(192, 168, 1, 1);
        let bootfile = b"pxelinux.0";
        msg.file[..bootfile.len()].copy_from_slice(bootfile);

        let wire = msg.to_wire();
        // BOOTP response should be 236 bytes (240 header minus 4 magic cookie, no options)
        assert_eq!(wire.len(), 236);
        // BOOTP should NOT have the DHCP magic cookie at bytes 232-235
        assert_ne!(&wire[232..236], &[99, 130, 83, 99]);

        // Parse it back
        let parsed = DhcpMessage::from_wire(&wire).unwrap();
        assert!(parsed.is_bootp);
        assert_eq!(parsed.siaddr, Ipv4Addr::new(192, 168, 1, 1));
    }
}
