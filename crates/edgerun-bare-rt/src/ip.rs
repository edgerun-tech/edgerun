//! IP Stack - IPv4, ARP, ICMP, UDP, TCP

#![allow(dead_code)]

pub const ETH_TYPE_IPV4: u16 = 0x0800;
pub const ETH_TYPE_ARP: u16 = 0x0806;

pub const IP_PROTO_ICMP: u8 = 1;
pub const IP_PROTO_TCP: u8 = 6;
pub const IP_PROTO_UDP: u8 = 17;

pub const DHCP_SERVER_PORT: u16 = 67;
pub const DHCP_CLIENT_PORT: u16 = 68;

pub const ARP_OP_REQUEST: u16 = 1;
pub const ARP_OP_REPLY: u16 = 2;

pub const ICMP_ECHO_REQUEST: u8 = 8;
pub const ICMP_ECHO_REPLY: u8 = 0;
pub const ICMP_DEST_UNREACHABLE: u8 = 3;
pub const ICMP_SOURCE_QUENCH: u8 = 4;
pub const ICMP_REDIRECT: u8 = 5;
pub const ICMP_ECHO: u8 = 8;
pub const ICMP_ROUTER_ADVERT: u8 = 9;
pub const ICMP_ROUTER_SOLICIT: u8 = 10;
pub const ICMP_TIME_EXCEEDED: u8 = 11;
pub const ICMP_PARAMETER_PROBLEM: u8 = 12;

#[derive(Clone, Copy)]
pub struct IcmpHeader {
    pub icmp_type: u8,
    pub code: u8,
    pub checksum: u16,
    pub identifier: u16,
    pub sequence: u16,
}

impl IcmpHeader {
    pub fn from_slice(data: &[u8]) -> Self {
        Self {
            icmp_type: data[0],
            code: data[1],
            checksum: u16::from_be_bytes([data[2], data[3]]),
            identifier: u16::from_be_bytes([data[4], data[5]]),
            sequence: u16::from_be_bytes([data[6], data[7]]),
        }
    }

    pub fn to_slice(&self, data: &mut [u8]) {
        data[0] = self.icmp_type;
        data[1] = self.code;
        data[2..4].copy_from_slice(&self.checksum.to_be_bytes());
        data[4..6].copy_from_slice(&self.identifier.to_be_bytes());
        data[6..8].copy_from_slice(&self.sequence.to_be_bytes());
    }
}

pub fn _ping(_mac: [u8; 6], _ip: IpAddr, id: u16, seq: u16) -> [u8; 64] {
    let mut packet = [0u8; 64];
    packet[0] = ICMP_ECHO_REQUEST;
    packet[1] = 0;
    packet[2] = 0;
    packet[3] = 0;
    packet[4..6].copy_from_slice(&id.to_be_bytes());
    packet[6..8].copy_from_slice(&seq.to_be_bytes());
    packet
}

pub fn checksum(data: &[u8]) -> u16 {
    let mut sum: u32 = 0;
    for i in (0..data.len()).step_by(2) {
        let word = if i + 1 < data.len() {
            ((data[i] as u32) << 8) | (data[i + 1] as u32)
        } else {
            (data[i] as u32) << 8
        };
        sum += word;
    }
    while sum >> 16 != 0 {
        sum = (sum & 0xFFFF) + (sum >> 16);
    }
    !(sum as u16)
}

pub fn echo_reply(req: &IcmpHeader, seq: u16) -> IcmpHeader {
    IcmpHeader {
        icmp_type: ICMP_ECHO_REPLY,
        code: 0,
        checksum: 0,
        identifier: req.identifier,
        sequence: seq,
    }
}

#[derive(Clone, Copy)]
pub struct TcpHeader {
    pub src_port: u16,
    pub dst_port: u16,
    pub seq: u32,
    pub ack: u32,
    pub flags: u8,
    pub window: u16,
    pub checksum: u16,
    pub urgent: u16,
}

impl TcpHeader {
    pub fn from_slice(data: &[u8]) -> Self {
        Self {
            src_port: u16::from_be_bytes([data[0], data[1]]),
            dst_port: u16::from_be_bytes([data[2], data[3]]),
            seq: u32::from_be_bytes([data[4], data[5], data[6], data[7]]),
            ack: u32::from_be_bytes([data[8], data[9], data[10], data[11]]),
            flags: data[12],
            window: u16::from_be_bytes([data[14], data[15]]),
            checksum: u16::from_be_bytes([data[16], data[17]]),
            urgent: u16::from_be_bytes([data[18], data[19]]),
        }
    }

    pub fn to_slice(&self, data: &mut [u8]) {
        data[0..2].copy_from_slice(&self.src_port.to_be_bytes());
        data[2..4].copy_from_slice(&self.dst_port.to_be_bytes());
        data[4..8].copy_from_slice(&self.seq.to_be_bytes());
        data[8..12].copy_from_slice(&self.ack.to_be_bytes());
        data[12] = self.flags;
        data[13] = 0;
        data[14..16].copy_from_slice(&self.window.to_be_bytes());
        data[16..18].copy_from_slice(&self.checksum.to_be_bytes());
        data[18..20].copy_from_slice(&self.urgent.to_be_bytes());
    }
}

pub const TCP_FLAG_FIN: u8 = 1;
pub const TCP_FLAG_SYN: u8 = 2;
pub const TCP_FLAG_RST: u8 = 4;
pub const TCP_FLAG_PSH: u8 = 8;
pub const TCP_FLAG_ACK: u8 = 16;

#[derive(Clone, Copy)]
pub struct EthHeader {
    pub dst: [u8; 6],
    pub src: [u8; 6],
    pub ethertype: u16,
}

impl EthHeader {
    pub fn from_slice(data: &[u8]) -> Self {
        let mut dst = [0u8; 6];
        let mut src = [0u8; 6];
        dst.copy_from_slice(&data[0..6]);
        src.copy_from_slice(&data[6..12]);
        Self { dst, src, ethertype: u16::from_be_bytes([data[12], data[13]]) }
    }

    pub fn to_slice(&self, data: &mut [u8]) {
        data[0..6].copy_from_slice(&self.dst);
        data[6..12].copy_from_slice(&self.src);
        data[12..14].copy_from_slice(&self.ethertype.to_be_bytes());
    }
}

#[derive(Clone, Copy)]
pub struct IpHeader {
    pub ver_ihl: u8,
    pub tos: u8,
    pub len: u16,
    pub ttl: u8,
    pub proto: u8,
    pub checksum: u16,
    pub src: [u8; 4],
    pub dst: [u8; 4],
}

impl IpHeader {
    pub fn from_slice(data: &[u8]) -> Self {
        let mut src = [0u8; 4];
        let mut dst = [0u8; 4];
        src.copy_from_slice(&data[12..16]);
        dst.copy_from_slice(&data[16..20]);
        let len = u16::from_be_bytes([data[2], data[3]]);
        Self {
            ver_ihl: data[0],
            tos: data[1],
            len,
            ttl: data[8],
            proto: data[9],
            checksum: u16::from_be_bytes([data[10], data[11]]),
            src,
            dst,
        }
    }

    pub fn to_slice(&self, data: &mut [u8]) {
        data[0] = self.ver_ihl;
        data[1] = self.tos;
        data[2..4].copy_from_slice(&self.len.to_be_bytes());
        data[4] = 0;
        data[5] = 0;
        data[6] = 0;
        data[7] = 0;
        data[8] = self.ttl;
        data[9] = self.proto;
        data[10..12].copy_from_slice(&self.checksum.to_be_bytes());
        data[12..16].copy_from_slice(&self.src);
        data[16..20].copy_from_slice(&self.dst);
    }
}

pub fn ip_checksum(data: &[u8]) -> u16 {
    checksum(data)
}

pub fn parse_packet(data: &[u8]) -> Option<(EthHeader, IpHeader)> {
    if data.len() < 34 {
        return None;
    }
    let eth = EthHeader::from_slice(data);
    if eth.ethertype != ETH_TYPE_IPV4 {
        return None;
    }
    let ip = IpHeader::from_slice(&data[14..]);
    Some((eth, ip))
}

#[derive(Clone, Copy)]
pub struct ArpHeader {
    pub oper: u16,
    pub sha: [u8; 6],
    pub spa: [u8; 4],
    pub tha: [u8; 6],
    pub tpa: [u8; 4],
}

impl ArpHeader {
    pub fn from_slice(data: &[u8]) -> Self {
        let mut sha = [0u8; 6];
        let mut spa = [0u8; 4];
        let mut tha = [0u8; 6];
        let mut tpa = [0u8; 4];
        sha.copy_from_slice(&data[8..14]);
        spa.copy_from_slice(&data[14..18]);
        tha.copy_from_slice(&data[18..24]);
        tpa.copy_from_slice(&data[24..28]);
        Self {
            oper: u16::from_be_bytes([data[6], data[7]]),
            sha, spa, tha, tpa,
        }
    }

    pub fn to_slice(&self, data: &mut [u8]) {
        data[0..2].copy_from_slice(&[0, 1]);
        data[2..4].copy_from_slice(&[1, 1]);
        data[4..6].copy_from_slice(&[6, 0]);
        data[6..8].copy_from_slice(&self.oper.to_be_bytes());
        data[8..14].copy_from_slice(&self.sha);
        data[14..18].copy_from_slice(&self.spa);
        data[18..24].copy_from_slice(&self.tha);
        data[24..28].copy_from_slice(&self.tpa);
    }
}

#[derive(Clone, Copy)]
pub struct UdpHeader {
    pub src_port: u16,
    pub dst_port: u16,
    pub len: u16,
    pub checksum: u16,
}

impl UdpHeader {
    pub fn from_slice(data: &[u8]) -> Self {
        Self {
            src_port: u16::from_be_bytes([data[0], data[1]]),
            dst_port: u16::from_be_bytes([data[2], data[3]]),
            len: u16::from_be_bytes([data[4], data[5]]),
            checksum: u16::from_be_bytes([data[6], data[7]]),
        }
    }

    pub fn to_slice(&self, data: &mut [u8]) {
        data[0..2].copy_from_slice(&self.src_port.to_be_bytes());
        data[2..4].copy_from_slice(&self.dst_port.to_be_bytes());
        data[4..6].copy_from_slice(&self.len.to_be_bytes());
        data[6..8].copy_from_slice(&self.checksum.to_be_bytes());
    }
}

#[derive(Clone, Copy, PartialEq)]
pub struct IpAddr([u8; 4]);

impl IpAddr {
    pub const fn new(a: u8, b: u8, c: u8, d: u8) -> Self {
        Self([a, b, c, d])
    }

    pub fn from_slice(data: &[u8]) -> Self {
        Self([data[0], data[1], data[2], data[3]])
    }

    pub const fn zero() -> Self {
        Self([0, 0, 0, 0])
    }

    pub const fn loopback() -> Self {
        Self([127, 0, 0, 1])
    }

    pub fn as_u32(&self) -> u32 {
        (self.0[0] as u32) << 24 | (self.0[1] as u32) << 16 | (self.0[2] as u32) << 8 | (self.0[3] as u32)
    }

    pub fn is_private(&self) -> bool {
        self.0[0] == 10 || (self.0[0] == 172 && self.0[1] >= 16 && self.0[1] < 32) || (self.0[0] == 192 && self.0[1] == 168)
    }

    pub fn as_bytes(&self) -> &[u8; 4] {
        &self.0
    }
}

pub struct IpStack {
    pub ip: IpAddr,
    pub netmask: IpAddr,
    pub gateway: IpAddr,
    pub mac: [u8; 6],
    pub arp: ArpCache,
}

impl IpStack {
    pub const fn new() -> Self {
        Self { ip: IpAddr::zero(), netmask: IpAddr::new(255, 255, 255, 0), gateway: IpAddr::zero(), mac: [0; 6], arp: ArpCache::new() }
    }

    pub fn configure(&mut self, ip: IpAddr, netmask: IpAddr, gateway: IpAddr, mac: [u8; 6]) {
        self.ip = ip;
        self.netmask = netmask;
        self.gateway = gateway;
        self.mac = mac;
    }

    pub fn route(&self, dst: IpAddr) -> IpAddr {
        let dst_bytes = dst.as_bytes();
        if dst_bytes[0] == self.ip.as_bytes()[0] {
            dst
        } else {
            self.gateway
        }
    }

    pub fn eth_header(&self, dst_mac: [u8; 6], proto: u16) -> EthHeader {
        EthHeader { dst: dst_mac, src: self.mac, ethertype: proto }
    }

    pub fn ip_header(&self, dst: IpAddr, proto: u8, len: u16) -> IpHeader {
        IpHeader {
            ver_ihl: 0x45,
            tos: 0,
            len,
            ttl: 64,
            proto,
            checksum: 0,
            src: *self.ip.as_bytes(),
            dst: *dst.as_bytes(),
        }
    }

    pub fn arp_request(&self, target: IpAddr, target_mac: [u8; 6]) -> ArpHeader {
        ArpHeader {
            oper: ARP_OP_REPLY,
            sha: self.mac,
            spa: *self.ip.as_bytes(),
            tha: target_mac,
            tpa: *target.as_bytes(),
        }
    }
}

pub struct ArpCache {
    entries: [(IpAddr, [u8; 6]); 8],
    count: usize,
}

impl ArpCache {
    pub const fn new() -> Self {
        Self { entries: [(IpAddr::zero(), [0; 6]); 8], count: 0 }
    }

    pub fn insert(&mut self, ip: IpAddr, mac: [u8; 6]) {
        if self.count < 8 {
            self.entries[self.count] = (ip, mac);
            self.count += 1;
        }
    }

    pub fn lookup(&self, ip: IpAddr) -> Option<[u8; 6]> {
        for i in 0..self.count {
            if self.entries[i].0 == ip {
                return Some(self.entries[i].1);
            }
        }
        None
    }
}

pub struct DhcpStateMachine {
    pub xid: u32,
    pub mac: [u8; 6],
    pub ip: IpAddr,
    pub netmask: IpAddr,
    pub gateway: IpAddr,
    pub retries: u8,
}

impl DhcpStateMachine {
    pub const fn new(mac: [u8; 6]) -> Self {
        Self {
            xid: 0x12345678,
            mac,
            ip: IpAddr::zero(),
            netmask: IpAddr::new(255, 255, 255, 0),
            gateway: IpAddr::zero(),
            retries: 0,
        }
    }

    pub fn discover(&self) -> [u8; 300] {
        let mut pkt = [0u8; 300];
        pkt[0] = 1;
        pkt[1] = 1;
        pkt[2] = 6;
        pkt[3] = 0;
        pkt[4..8].copy_from_slice(&self.xid.to_be_bytes());
        pkt[10..12].copy_from_slice(&0x8000u16.to_be_bytes());
        pkt[28..34].copy_from_slice(&self.mac);
        pkt[236..240].copy_from_slice(&0x63825363u32.to_be_bytes());
        pkt[240] = 53;
        pkt[241] = 1;
        pkt[242] = 1;
        pkt[243] = 55;
        pkt[244] = 3;
        pkt[245] = 1;
        pkt[246] = 3;
        pkt[247] = 6;
        pkt[248] = 255;
        pkt
    }

    pub fn request(&self, server: IpAddr) -> [u8; 300] {
        let mut pkt = [0u8; 300];
        pkt[0] = 1;
        pkt[1] = 1;
        pkt[2] = 6;
        pkt[3] = 0;
        pkt[4..8].copy_from_slice(&self.xid.to_be_bytes());
        pkt[10..12].copy_from_slice(&0x8000u16.to_be_bytes());
        pkt[28..34].copy_from_slice(&self.mac);
        pkt[236..240].copy_from_slice(&0x63825363u32.to_be_bytes());
        pkt[240] = 53;
        pkt[241] = 1;
        pkt[242] = 3;
        pkt[243] = 50;
        pkt[244] = 4;
        pkt[245..249].copy_from_slice(self.ip.as_bytes());
        pkt[249] = 54;
        pkt[250] = 4;
        pkt[251..255].copy_from_slice(server.as_bytes());
        pkt[255] = 255;
        pkt
    }

    pub fn parse(&mut self, pkt: &[u8]) -> bool {
        if pkt.len() < 240 {
            return false;
        }
        let cookie = u32::from_be_bytes([pkt[236], pkt[237], pkt[238], pkt[239]]);
        if cookie != 0x63825363 {
            return false;
        }
        
        let mut i = 240;
        while i < pkt.len() - 2 && i < 540 {
            let code = pkt[i];
            if code == 255 {
                break;
            }
            if code == 53 && pkt[i+2] == 5 {
                break;
            }
            if code == 1 {
                self.netmask = IpAddr::from_slice(&pkt[i+2..i+6]);
            } else if code == 3 {
                self.gateway = IpAddr::from_slice(&pkt[i+2..i+6]);
            }
            i += 2 + pkt[i+1] as usize;
        }
        
        self.ip = IpAddr::from_slice(&pkt[16..20]);
        self.ip != IpAddr::zero()
    }
}

pub struct Network<'a> {
    pub stack: &'a mut IpStack,
    packet: [u8; 1514],
    packet_len: usize,
}

impl<'a> Network<'a> {
    pub const fn new(stack: &'a mut IpStack) -> Self {
        Self { stack, packet: [0u8; 1514], packet_len: 0 }
    }

    pub fn send_ip(&mut self, dst: IpAddr, proto: u8, data: &[u8]) -> Option<&[u8]> {
        let routed = self.stack.route(dst);
        let dst_mac = if let Some(mac) = self.stack.arp.lookup(routed) {
            mac
        } else {
            [0xff; 6]
        };
        self.send_eth(dst_mac, dst, proto, data)
    }

    pub fn send_udp(&mut self, dst: IpAddr, src_port: u16, dst_port: u16, data: &[u8]) -> Option<&[u8]> {
        let routed = self.stack.route(dst);
        let dst_mac = if let Some(mac) = self.stack.arp.lookup(routed) {
            mac
        } else {
            [0xff; 6]
        };
        self.send_udp_eth(dst_mac, dst, src_port, dst_port, data)
    }

    pub fn send_udp_eth(&mut self, dst_mac: [u8; 6], dst: IpAddr, src_port: u16, dst_port: u16, data: &[u8]) -> Option<&[u8]> {
        let udp_len = 8usize.checked_add(data.len())?;
        let ip_len = 20usize.checked_add(udp_len)?;
        let packet_len = 14usize.checked_add(ip_len)?;
        if packet_len > self.packet.len() || udp_len > u16::MAX as usize || ip_len > u16::MAX as usize {
            return None;
        }

        self.packet = [0u8; 1514];
        let eth = self.stack.eth_header(dst_mac, ETH_TYPE_IPV4);
        eth.to_slice(&mut self.packet);

        let mut ip = self.stack.ip_header(dst, IP_PROTO_UDP, ip_len as u16);
        ip.to_slice(&mut self.packet[14..34]);
        self.packet[14 + 10] = 0;
        self.packet[14 + 11] = 0;
        ip.checksum = ip_checksum(&self.packet[14..34]);
        self.packet[14 + 10..14 + 12].copy_from_slice(&ip.checksum.to_be_bytes());

        let udp = UdpHeader {
            src_port,
            dst_port,
            len: udp_len as u16,
            checksum: 0,
        };
        udp.to_slice(&mut self.packet[34..42]);
        self.packet[42..42 + data.len()].copy_from_slice(data);
        self.packet_len = packet_len;
        Some(&self.packet[..self.packet_len])
    }

    pub fn send_eth(&mut self, dst_mac: [u8; 6], dst: IpAddr, proto: u8, data: &[u8]) -> Option<&[u8]> {
        let ip_len = (20 + data.len()) as u16;
        let packet_len = 34usize.checked_add(data.len())?;
        if packet_len > self.packet.len() {
            return None;
        }
        self.packet = [0u8; 1514];
        let eth = self.stack.eth_header(dst_mac, ETH_TYPE_IPV4);
        eth.to_slice(&mut self.packet);
        let mut ip = self.stack.ip_header(dst, proto, ip_len);
        ip.to_slice(&mut self.packet[14..34]);
        self.packet[14 + 10] = 0;
        self.packet[14 + 11] = 0;
        ip.checksum = ip_checksum(&self.packet[14..34]);
        self.packet[14 + 10..14 + 12].copy_from_slice(&ip.checksum.to_be_bytes());
        let payload_start = 34;
        self.packet_len = packet_len;
        self.packet[payload_start..payload_start + data.len()].copy_from_slice(data);
        Some(&self.packet[..self.packet_len])
    }

    pub fn packet(&self) -> &[u8] {
        &self.packet[..self.packet_len]
    }

    pub fn len(&self) -> usize {
        self.packet_len
    }

    pub fn recv<'packet>(&mut self, data: &'packet [u8]) -> Option<ParsedPacket<'packet>> {
        if data.len() < 34 {
            return None;
        }
        let eth = EthHeader::from_slice(data);
        if eth.ethertype != ETH_TYPE_IPV4 {
            return None;
        }
        if data.len() < 14 + 20 {
            return None;
        }
        let ip = IpHeader::from_slice(&data[14..]);
        let ip_header_len = 4 * (ip.ver_ihl & 0x0f) as usize;
        let payload_start = 14 + ip_header_len;
        if ip_header_len < 20 || data.len() < payload_start {
            return None;
        }
        let payload = &data[payload_start..];
        match ip.proto {
            IP_PROTO_UDP if payload.len() >= 8 => {
                let udp = UdpHeader::from_slice(payload);
                let udp_len = udp.len as usize;
                if udp_len < 8 || udp_len > payload.len() {
                    return None;
                }
                Some(ParsedPacket::Udp {
                    header: udp,
                    payload: &payload[8..udp_len],
                })
            }
            IP_PROTO_TCP if payload.len() >= 20 => Some(ParsedPacket::Tcp {
                header: TcpHeader::from_slice(payload),
                payload: &payload[20..],
            }),
            IP_PROTO_ICMP if payload.len() >= 8 => Some(ParsedPacket::Icmp {
                header: IcmpHeader::from_slice(payload),
                payload: &payload[8..],
            }),
            _ => None,
        }
    }
}

pub enum ParsedPacket<'a> {
    Udp { header: UdpHeader, payload: &'a [u8] },
    Tcp { header: TcpHeader, payload: &'a [u8] },
    Icmp { header: IcmpHeader, payload: &'a [u8] },
}

impl<'a> ParsedPacket<'a> {
    pub fn is_udp(&self) -> bool {
        matches!(self, ParsedPacket::Udp { .. })
    }
    
    pub fn is_tcp(&self) -> bool {
        matches!(self, ParsedPacket::Tcp { .. })
    }
    
    pub fn is_icmp(&self) -> bool {
        matches!(self, ParsedPacket::Icmp { .. })
    }

    pub fn udp_payload(&self) -> Option<&'a [u8]> {
        match self {
            ParsedPacket::Udp { payload, .. } => Some(payload),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn udp_packet_round_trips_payload() {
        let mut stack = IpStack::new();
        stack.configure(
            IpAddr::new(192, 168, 1, 12),
            IpAddr::new(255, 255, 255, 0),
            IpAddr::zero(),
            [0x52, 0x54, 0x00, 0x12, 0x34, 0x56],
        );

        let mut network = Network::new(&mut stack);
        let payload = [1, 2, 3, 4];
        let packet_src = network
            .send_udp(IpAddr::new(255, 255, 255, 255), DHCP_CLIENT_PORT, DHCP_SERVER_PORT, &payload)
            .unwrap();
        let mut packet = [0u8; 1514];
        let packet_len = packet_src.len();
        packet[..packet_len].copy_from_slice(packet_src);

        let parsed = network.recv(&packet[..packet_len]).unwrap();
        match parsed {
            ParsedPacket::Udp { header, payload } => {
                assert_eq!(header.src_port, DHCP_CLIENT_PORT);
                assert_eq!(header.dst_port, DHCP_SERVER_PORT);
                assert_eq!(header.len, 12);
                assert_eq!(payload, &[1, 2, 3, 4]);
            }
            _ => panic!("expected udp packet"),
        }
    }

    #[test]
    fn dhcp_discover_has_cookie_and_udp_ports() {
        let mac = [0x52, 0x54, 0x00, 0x12, 0x34, 0x56];
        let dhcp = DhcpStateMachine::new(mac);
        let discover = dhcp.discover();
        assert_eq!(&discover[236..240], &0x63825363u32.to_be_bytes());
        assert_eq!(&discover[28..34], &mac);

        let mut stack = IpStack::new();
        stack.configure(IpAddr::zero(), IpAddr::new(255, 255, 255, 0), IpAddr::zero(), mac);
        let mut network = Network::new(&mut stack);
        let packet_src = network
            .send_udp(
                IpAddr::new(255, 255, 255, 255),
                DHCP_CLIENT_PORT,
                DHCP_SERVER_PORT,
                &discover,
            )
            .unwrap();
        let mut packet = [0u8; 1514];
        let packet_len = packet_src.len();
        packet[..packet_len].copy_from_slice(packet_src);

        let parsed = network.recv(&packet[..packet_len]).unwrap();
        match parsed {
            ParsedPacket::Udp { header, payload } => {
                assert_eq!(header.src_port, DHCP_CLIENT_PORT);
                assert_eq!(header.dst_port, DHCP_SERVER_PORT);
                assert_eq!(&payload[236..240], &0x63825363u32.to_be_bytes());
            }
            _ => panic!("expected udp packet"),
        }
    }
}
