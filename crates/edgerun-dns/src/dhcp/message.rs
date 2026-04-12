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

        // Magic cookie
        buf.extend_from_slice(&DHCP_COOKIE);

        // Options
        self.options.serialize(&mut buf);

        buf
    }

    /// Parse a DHCP message from wire format.
    pub fn from_wire(data: &[u8]) -> Result<Self, io::Error> {
        if data.len() < Self::HEADER_SIZE {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("DHCP message too short: {} bytes (min {})", data.len(), Self::HEADER_SIZE),
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
        file.copy_from_slice(&data[108..236]);

        // Verify magic cookie
        if data[236..240] != DHCP_COOKIE {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Invalid DHCP magic cookie",
            ));
        }

        let options = DhcpOptions::parse(&data[240..])?;

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
        server_id: Ipv4Addr,
        subnet_mask: Ipv4Addr,
        router: Ipv4Addr,
        dns_servers: Vec<Ipv4Addr>,
        lease_time: u32,
        tftp_server: Option<String>,
        bootfile: Option<String>,
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
            options: DhcpOptions {
                message_type: Some(DhcpMessageType::Offer),
                subnet_mask: Some(subnet_mask),
                router: Some(router),
                dns_servers,
                server_id: Some(server_id),
                lease_time: Some(lease_time),
                renewal_time: Some(lease_time / 2),
                rebind_time: Some(lease_time * 7 / 8),
                tftp_server_name: tftp_server,
                bootfile_name: bootfile,
                ..Default::default()
            },
        }
    }

    /// Create a DHCP ACK (server → client).
    pub fn ack(
        xid: u32,
        mac: [u8; 6],
        yiaddr: Ipv4Addr,
        server_id: Ipv4Addr,
        subnet_mask: Ipv4Addr,
        router: Ipv4Addr,
        dns_servers: Vec<Ipv4Addr>,
        lease_time: u32,
        tftp_server: Option<String>,
        bootfile: Option<String>,
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
            options: DhcpOptions {
                message_type: Some(DhcpMessageType::Ack),
                subnet_mask: Some(subnet_mask),
                router: Some(router),
                dns_servers,
                server_id: Some(server_id),
                lease_time: Some(lease_time),
                renewal_time: Some(lease_time / 2),
                rebind_time: Some(lease_time * 7 / 8),
                tftp_server_name: tftp_server,
                bootfile_name: bootfile,
                ..Default::default()
            },
        }
    }

    /// Create a DHCP NAK (server → client).
    pub fn nak(xid: u32, server_id: Ipv4Addr) -> Self {
        Self {
            op: DhcpOp::Reply,
            htype: HTYPE_ETHER,
            hlen: HLEN_ETHER,
            hops: 0,
            xid,
            secs: 0,
            broadcast: false,
            ciaddr: Ipv4Addr::UNSPECIFIED,
            yiaddr: Ipv4Addr::UNSPECIFIED,
            siaddr: Ipv4Addr::UNSPECIFIED,
            giaddr: Ipv4Addr::UNSPECIFIED,
            chaddr: [0; 16],
            sname: [0; 64],
            file: [0; 128],
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
                }
                OPT_HOST_NAME => {
                    opts.host_name = Some(String::from_utf8_lossy(value).to_string());
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
            buf.push(OPT_DNS_SERVER);
            buf.push((self.dns_servers.len() * 4) as u8);
            for s in &self.dns_servers {
                buf.extend_from_slice(&s.octets());
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
        let msg = DhcpMessage::offer(
            0x12345678,
            mac,
            Ipv4Addr::new(192, 168, 1, 100),
            server,
            Ipv4Addr::new(255, 255, 255, 0),
            Ipv4Addr::new(192, 168, 1, 1),
            vec![Ipv4Addr::new(8, 8, 8, 8)],
            86400,
            Some("192.168.1.1".to_string()),
            Some("bootx64.efi".to_string()),
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
            Ipv4Addr::new(10, 0, 0, 1),
            Ipv4Addr::new(255, 0, 0, 0),
            Ipv4Addr::new(10, 0, 0, 1),
            vec![Ipv4Addr::new(1, 1, 1, 1)],
            3600,
            Some("10.0.0.1".to_string()),
            Some("bootx64.efi".to_string()),
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
        let msg = DhcpMessage::nak(0xdeadbeef, Ipv4Addr::new(192, 168, 1, 1));
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
        // Too short
        assert!(DhcpMessage::from_wire(&[0u8; 100]).is_err());
        // Bad magic cookie
        let mut buf = vec![0u8; 240];
        buf[0] = BOOTREQUEST;
        assert!(DhcpMessage::from_wire(&buf).is_err());
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
}
