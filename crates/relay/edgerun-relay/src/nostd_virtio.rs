use alloc::collections::BTreeMap;
use alloc::vec;
use alloc::vec::Vec;

use edgerun_protocols::ethernet_ipv4::{
    ARP_OP_REQUEST, ETH_TYPE_IPV4, IP_PROTO_TCP, IpAddr, IpStack, Network, ParsedPacket,
    TCP_FLAG_ACK, TCP_FLAG_FIN, TCP_FLAG_PSH, TCP_FLAG_RST, TCP_FLAG_SYN, TcpHeader, checksum,
};
use edgerun_wire::RelayMessage;

use crate::nostd_relay::{RelayEngine, RelayOutput};
use crate::{MAX_FRAME_LEN, decode_packet, encode_packet};

const TCP_MSS: usize = 1200;
const TCP_INITIAL_SEQ: u32 = 0xED6E_0001;
const TCP_RETRANSMIT_AFTER_MS: u64 = 1000;
const TCP_MAX_RETRANSMITS: u8 = 4;
const TCP_CONNECTION_LIFETIME_MS: u64 = 60_000;

#[derive(Clone, PartialEq)]
pub enum EthernetRelayPeer {
    Udp { mac: [u8; 6], ip: IpAddr, port: u16 },
    Tcp { ip: [u8; 4], port: u16 },
}

pub struct EthernetRelay {
    engine: RelayEngine<EthernetRelayPeer>,
    stack: IpStack,
    tcp_sessions: BTreeMap<TcpKey, TcpSession>,
    listen_port: u16,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct TcpKey {
    ip: [u8; 4],
    port: u16,
}

struct TcpSession {
    peer_mac: [u8; 6],
    send_seq: u32,
    recv_next: u32,
    read_buf: Vec<u8>,
    unacked: Vec<TcpUnacked>,
    opened_ms: u64,
}

struct TcpUnacked {
    seq: u32,
    len: u32,
    flags: u8,
    payload: Vec<u8>,
    sent_ms: u64,
    attempts: u8,
}

impl EthernetRelay {
    pub fn new(ip: IpAddr, netmask: IpAddr, gateway: IpAddr, mac: [u8; 6], port: u16) -> Self {
        let mut stack = IpStack::new();
        stack.configure(ip, netmask, gateway, mac);
        Self {
            engine: RelayEngine::new(),
            stack,
            tcp_sessions: BTreeMap::new(),
            listen_port: port,
        }
    }

    pub fn registered_len(&self) -> usize {
        self.engine.registered_len()
    }

    pub fn pending_len(&self) -> usize {
        self.engine.pending_len()
    }

    pub fn remove_peer(&mut self, peer: &EthernetRelayPeer) {
        self.engine.remove_peer(peer);
        if let EthernetRelayPeer::Tcp { ip, port } = peer {
            self.tcp_sessions.remove(&TcpKey {
                ip: *ip,
                port: *port,
            });
        }
    }

    pub fn handle_frame(&mut self, frame: &[u8]) -> Vec<Vec<u8>> {
        self.handle_frame_at(frame, 0)
    }

    pub fn handle_frame_at(&mut self, frame: &[u8], now_ms: u64) -> Vec<Vec<u8>> {
        let mut out = Vec::new();
        let mut network = Network::new(&mut self.stack);
        let Some(parsed) = network.recv(frame) else {
            return out;
        };

        match parsed {
            ParsedPacket::Arp { header, .. } => {
                if header.oper == ARP_OP_REQUEST && header.tpa == *network.stack.ip.as_bytes() {
                    if let Some(reply) = network.send_arp_reply(&header) {
                        out.push(reply.to_vec());
                    }
                }
            }
            ParsedPacket::Udp {
                eth,
                ip,
                header,
                payload,
            } if header.dst_port == self.listen_port => {
                let peer = EthernetRelayPeer::Udp {
                    mac: eth.src,
                    ip: IpAddr::from_slice(&ip.src),
                    port: header.src_port,
                };
                let Ok(message) = decode_packet(payload) else {
                    return out;
                };
                drop(network);
                for output in self.engine.handle_message(peer, message) {
                    self.push_output(output, now_ms, &mut out);
                }
            }
            ParsedPacket::Tcp {
                eth,
                ip,
                header,
                payload,
            } if header.dst_port == self.listen_port => {
                drop(network);
                self.handle_tcp_packet(eth.src, ip.src, header, payload, now_ms, &mut out);
            }
            _ => {}
        }

        out
    }

    pub fn poll_tcp(&mut self, now_ms: u64) -> Vec<Vec<u8>> {
        let mut out = Vec::new();
        let expired = self
            .tcp_sessions
            .iter()
            .filter_map(|(key, session)| {
                (now_ms.saturating_sub(session.opened_ms) >= TCP_CONNECTION_LIFETIME_MS)
                    .then_some(*key)
            })
            .collect::<Vec<_>>();
        for key in expired {
            if let Some(frame) = self.send_tcp_control(key, TCP_FLAG_FIN | TCP_FLAG_ACK, now_ms) {
                out.push(frame);
            }
            self.close_tcp(key);
        }

        let mut retransmits = Vec::new();
        let mut close = Vec::new();
        for (key, session) in self.tcp_sessions.iter_mut() {
            for segment in &mut session.unacked {
                if now_ms.saturating_sub(segment.sent_ms) < TCP_RETRANSMIT_AFTER_MS {
                    continue;
                }
                if segment.attempts >= TCP_MAX_RETRANSMITS {
                    close.push(*key);
                    break;
                }
                segment.attempts += 1;
                segment.sent_ms = now_ms;
                retransmits.push((
                    *key,
                    session.peer_mac,
                    segment.seq,
                    session.recv_next,
                    segment.flags,
                    segment.payload.clone(),
                ));
            }
        }
        for (key, mac, seq, ack, flags, payload) in retransmits {
            if let Some(frame) = self.build_tcp_segment(key, mac, seq, ack, flags, &payload) {
                out.push(frame);
            }
        }
        for key in close {
            self.close_tcp(key);
        }
        out
    }

    fn push_output(
        &mut self,
        output: RelayOutput<EthernetRelayPeer>,
        now_ms: u64,
        out: &mut Vec<Vec<u8>>,
    ) {
        match output {
            RelayOutput::Ack { peer, ack } => {
                self.push_message_to_peer(peer, RelayMessage::Ack(ack), now_ms, out);
            }
            RelayOutput::Message { peer, message } => {
                self.push_message_to_peer(peer, message, now_ms, out);
            }
        }
    }

    fn push_message_to_peer(
        &mut self,
        peer: EthernetRelayPeer,
        message: RelayMessage,
        now_ms: u64,
        out: &mut Vec<Vec<u8>>,
    ) {
        let Ok(bytes) = encode_packet(&message) else {
            return;
        };
        match peer {
            EthernetRelayPeer::Udp { mac, ip, port } => {
                let mut network = Network::new(&mut self.stack);
                if let Some(frame) = network.send_udp_eth(mac, ip, self.listen_port, port, &bytes) {
                    out.push(frame.to_vec());
                }
            }
            EthernetRelayPeer::Tcp { ip, port } => {
                out.extend(self.write_tcp_message_at(TcpKey { ip, port }, &bytes, now_ms));
            }
        }
    }

    fn handle_tcp_packet(
        &mut self,
        peer_mac: [u8; 6],
        peer_ip: [u8; 4],
        header: TcpHeader,
        payload: &[u8],
        now_ms: u64,
        out: &mut Vec<Vec<u8>>,
    ) {
        let key = TcpKey {
            ip: peer_ip,
            port: header.src_port,
        };

        if header.flags & TCP_FLAG_RST != 0 {
            self.close_tcp(key);
            return;
        }

        if header.flags & TCP_FLAG_SYN != 0 {
            self.tcp_sessions.insert(
                key,
                TcpSession {
                    peer_mac,
                    send_seq: TCP_INITIAL_SEQ,
                    recv_next: header.seq.wrapping_add(1),
                    read_buf: Vec::new(),
                    unacked: Vec::new(),
                    opened_ms: now_ms,
                },
            );
            if let Some(frame) = self.send_tcp_control(key, TCP_FLAG_SYN | TCP_FLAG_ACK, now_ms) {
                out.push(frame);
            }
            return;
        }

        if header.flags & TCP_FLAG_FIN != 0 {
            if let Some(session) = self.tcp_sessions.get_mut(&key) {
                session.recv_next = header.seq.wrapping_add(1);
            }
            if let Some(frame) = self.send_tcp_control(key, TCP_FLAG_ACK, now_ms) {
                out.push(frame);
            }
            self.close_tcp(key);
            return;
        }

        let mut messages = Vec::new();
        if let Some(session) = self.tcp_sessions.get_mut(&key) {
            if header.flags & TCP_FLAG_ACK != 0 {
                prune_acked(session, header.ack);
            }
            session.peer_mac = peer_mac;
            if !payload.is_empty() && header.seq == session.recv_next {
                session.recv_next = session.recv_next.wrapping_add(payload.len() as u32);
                if session.read_buf.len().saturating_add(payload.len()) > MAX_FRAME_LEN + 4 {
                    if let Some(frame) = self.send_tcp_control(key, TCP_FLAG_RST, now_ms) {
                        out.push(frame);
                    }
                    self.close_tcp(key);
                    return;
                }
                session.read_buf.extend_from_slice(payload);
                messages = drain_tcp_messages(&mut session.read_buf);
            }
        } else {
            return;
        }

        if let Some(frame) = self.send_tcp_control(key, TCP_FLAG_ACK, now_ms) {
            out.push(frame);
        }
        let peer = EthernetRelayPeer::Tcp {
            ip: key.ip,
            port: key.port,
        };
        for message in messages {
            for output in self.engine.handle_message(peer.clone(), message) {
                self.push_output(output, now_ms, out);
            }
        }
    }

    fn write_tcp_message_at(&mut self, key: TcpKey, bytes: &[u8], now_ms: u64) -> Vec<Vec<u8>> {
        let mut frame = Vec::with_capacity(4 + bytes.len());
        frame.extend_from_slice(&(bytes.len() as u32).to_be_bytes());
        frame.extend_from_slice(bytes);
        let mut out = Vec::new();
        for chunk in frame.chunks(TCP_MSS) {
            if let Some(frame) = self.send_tcp_data(key, chunk, now_ms) {
                out.push(frame);
            }
        }
        out
    }

    fn send_tcp_data(&mut self, key: TcpKey, payload: &[u8], now_ms: u64) -> Option<Vec<u8>> {
        let (mac, seq, ack) = {
            let session = self.tcp_sessions.get_mut(&key)?;
            let seq = session.send_seq;
            session.send_seq = session.send_seq.wrapping_add(payload.len() as u32);
            (session.peer_mac, seq, session.recv_next)
        };
        self.send_tcp_segment_tracked(
            key,
            mac,
            seq,
            ack,
            TCP_FLAG_ACK | TCP_FLAG_PSH,
            payload,
            now_ms,
        )
    }

    fn send_tcp_control(&mut self, key: TcpKey, flags: u8, now_ms: u64) -> Option<Vec<u8>> {
        let (mac, seq, ack, tracked) = {
            let session = self.tcp_sessions.get_mut(&key)?;
            let seq = session.send_seq;
            let tracked = flags & (TCP_FLAG_SYN | TCP_FLAG_FIN) != 0;
            if tracked {
                session.send_seq = session.send_seq.wrapping_add(1);
            }
            (session.peer_mac, seq, session.recv_next, tracked)
        };
        if tracked {
            self.send_tcp_segment_tracked(key, mac, seq, ack, flags, &[], now_ms)
        } else {
            self.build_tcp_segment(key, mac, seq, ack, flags, &[])
        }
    }

    fn send_tcp_segment_tracked(
        &mut self,
        key: TcpKey,
        mac: [u8; 6],
        seq: u32,
        ack: u32,
        flags: u8,
        payload: &[u8],
        now_ms: u64,
    ) -> Option<Vec<u8>> {
        let frame = self.build_tcp_segment(key, mac, seq, ack, flags, payload)?;
        let len = payload.len() as u32
            + u32::from(flags & TCP_FLAG_SYN != 0)
            + u32::from(flags & TCP_FLAG_FIN != 0);
        if len != 0 {
            if let Some(session) = self.tcp_sessions.get_mut(&key) {
                session.unacked.push(TcpUnacked {
                    seq,
                    len,
                    flags,
                    payload: payload.to_vec(),
                    sent_ms: now_ms,
                    attempts: 0,
                });
            }
        }
        Some(frame)
    }

    fn build_tcp_segment(
        &self,
        key: TcpKey,
        peer_mac: [u8; 6],
        seq: u32,
        ack: u32,
        flags: u8,
        payload: &[u8],
    ) -> Option<Vec<u8>> {
        build_tcp_segment(
            self.stack.mac,
            *self.stack.ip.as_bytes(),
            self.listen_port,
            key.ip,
            key.port,
            peer_mac,
            seq,
            ack,
            flags,
            payload,
        )
    }

    fn close_tcp(&mut self, key: TcpKey) {
        self.tcp_sessions.remove(&key);
        self.engine.remove_peer(&EthernetRelayPeer::Tcp {
            ip: key.ip,
            port: key.port,
        });
    }
}

fn drain_tcp_messages(buf: &mut Vec<u8>) -> Vec<RelayMessage> {
    let mut messages = Vec::new();
    let mut offset = 0usize;
    while buf.len().saturating_sub(offset) >= 4 {
        let len = u32::from_be_bytes([
            buf[offset],
            buf[offset + 1],
            buf[offset + 2],
            buf[offset + 3],
        ]) as usize;
        if len == 0 || len > MAX_FRAME_LEN {
            buf.clear();
            return messages;
        }
        if buf.len().saturating_sub(offset + 4) < len {
            break;
        }
        if let Ok(message) = decode_packet(&buf[offset + 4..offset + 4 + len]) {
            messages.push(message);
        } else {
            buf.clear();
            return messages;
        }
        offset += 4 + len;
    }
    if offset != 0 {
        buf.drain(0..offset);
    }
    messages
}

fn prune_acked(session: &mut TcpSession, ack: u32) {
    session
        .unacked
        .retain(|segment| !tcp_seq_le(segment.seq.wrapping_add(segment.len), ack));
}

fn tcp_seq_le(a: u32, b: u32) -> bool {
    a == b || b.wrapping_sub(a) < (1 << 31)
}

#[allow(clippy::too_many_arguments)]
fn build_tcp_segment(
    src_mac: [u8; 6],
    src_ip: [u8; 4],
    src_port: u16,
    dst_ip: [u8; 4],
    dst_port: u16,
    dst_mac: [u8; 6],
    seq: u32,
    ack: u32,
    flags: u8,
    payload: &[u8],
) -> Option<Vec<u8>> {
    let tcp_len = 20usize.checked_add(payload.len())?;
    let ip_len = 20usize.checked_add(tcp_len)?;
    let frame_len = 14usize.checked_add(ip_len)?;
    if frame_len > 1514 || ip_len > u16::MAX as usize {
        return None;
    }

    let mut frame = vec![0u8; frame_len];
    frame[0..6].copy_from_slice(&dst_mac);
    frame[6..12].copy_from_slice(&src_mac);
    write_u16(&mut frame, 12, ETH_TYPE_IPV4);

    let ip_start = 14;
    frame[ip_start] = 0x45;
    write_u16(&mut frame, ip_start + 2, ip_len as u16);
    frame[ip_start + 8] = 64;
    frame[ip_start + 9] = IP_PROTO_TCP;
    frame[ip_start + 12..ip_start + 16].copy_from_slice(&src_ip);
    frame[ip_start + 16..ip_start + 20].copy_from_slice(&dst_ip);
    let ip_sum = checksum(&frame[ip_start..ip_start + 20]);
    write_u16(&mut frame, ip_start + 10, ip_sum);

    let tcp_start = ip_start + 20;
    write_u16(&mut frame, tcp_start, src_port);
    write_u16(&mut frame, tcp_start + 2, dst_port);
    write_u32(&mut frame, tcp_start + 4, seq);
    write_u32(&mut frame, tcp_start + 8, ack);
    frame[tcp_start + 12] = 5 << 4;
    frame[tcp_start + 13] = flags;
    write_u16(&mut frame, tcp_start + 14, 64240);
    frame[tcp_start + 20..tcp_start + 20 + payload.len()].copy_from_slice(payload);
    let tcp_sum = tcp_checksum(&src_ip, &dst_ip, &frame[tcp_start..tcp_start + tcp_len]);
    write_u16(&mut frame, tcp_start + 16, tcp_sum);
    Some(frame)
}

fn tcp_checksum(src_ip: &[u8; 4], dst_ip: &[u8; 4], tcp_segment: &[u8]) -> u16 {
    let mut pseudo = Vec::with_capacity(12 + tcp_segment.len());
    pseudo.extend_from_slice(src_ip);
    pseudo.extend_from_slice(dst_ip);
    pseudo.push(0);
    pseudo.push(IP_PROTO_TCP);
    pseudo.extend_from_slice(&(tcp_segment.len() as u16).to_be_bytes());
    pseudo.extend_from_slice(tcp_segment);
    checksum(&pseudo)
}

fn write_u16(out: &mut [u8], offset: usize, value: u16) {
    out[offset..offset + 2].copy_from_slice(&value.to_be_bytes());
}

fn write_u32(out: &mut [u8], offset: usize, value: u32) {
    out[offset..offset + 4].copy_from_slice(&value.to_be_bytes());
}
