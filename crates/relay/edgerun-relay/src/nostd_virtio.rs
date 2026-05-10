use alloc::collections::BTreeMap;
use alloc::vec;
use alloc::vec::Vec;

use edgerun_network_driver::{FrameDevice, FrameDriverError};
use edgerun_protocols::ethernet_ipv4::{
    ARP_OP_REQUEST, ETH_TYPE_IPV4, IP_PROTO_TCP, IpAddr, IpStack, Network, ParsedPacket,
    TCP_FLAG_ACK, TCP_FLAG_FIN, TCP_FLAG_PSH, TCP_FLAG_RST, TCP_FLAG_SYN, TcpHeader, checksum,
};
use edgerun_wire::RelayMessage;

use crate::nostd_relay::{RelayEngine, RelayOutput};
use crate::{MAX_FRAME_LEN, decode_packet, encode_packet};
#[cfg(feature = "wss")]
use edgerun_rusttls::ServerConfig;

const TCP_MSS: usize = 1200;
const TCP_INITIAL_SEQ: u32 = 0xED6E_0001;
const TCP_RETRANSMIT_AFTER_MS: u64 = 1000;
const TCP_MAX_RETRANSMITS: u8 = 4;
const TCP_CONNECTION_LIFETIME_MS: u64 = 60_000;
const CUSTOM_QUIC_RETRANSMIT_AFTER_MS: u64 = 60_000;
const CUSTOM_QUIC_MAX_RETRANSMITS: u8 = 32;
const CUSTOM_QUIC_SESSION_LIFETIME_MS: u64 = 60_000;
const VIRTIO_RX_BUFFER_LEN: usize = 2048;
const VIRTIO_DEFAULT_POLL_BUDGET: usize = 64;

#[derive(Clone, PartialEq)]
pub enum EthernetRelayPeer {
    Udp {
        mac: [u8; 6],
        ip: IpAddr,
        port: u16,
    },
    CustomQuicUdp {
        mac: [u8; 6],
        ip: IpAddr,
        port: u16,
        connection_id: u64,
    },
    Tcp {
        ip: [u8; 4],
        port: u16,
    },
}

pub struct EthernetRelay {
    engine: RelayEngine<EthernetRelayPeer>,
    stack: IpStack,
    tcp_sessions: BTreeMap<TcpKey, TcpSession>,
    custom_quic_sessions: BTreeMap<CustomQuicKey, CustomQuicSession>,
    listen_port: u16,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EthernetRelayEventKind {
    TcpOpened,
    TcpClosed { reason: &'static str },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EthernetRelayEvent {
    pub kind: EthernetRelayEventKind,
    pub ip: [u8; 4],
    pub port: u16,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct VirtioRelayPoll {
    pub rx_frames: usize,
    pub tx_frames: usize,
    pub tx_errors: usize,
    pub dropped_rx_frames: usize,
}

pub struct VirtioRelay {
    relay: EthernetRelay,
    #[cfg(feature = "wss")]
    wss_config: Option<ServerConfig>,
    rx_buf: [u8; VIRTIO_RX_BUFFER_LEN],
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct CustomQuicKey {
    ip: [u8; 4],
    port: u16,
    connection_id: u64,
}

struct CustomQuicSession {
    peer_mac: [u8; 6],
    next_packet_number: u64,
    unacked: Vec<CustomQuicUnacked>,
    last_seen_ms: u64,
}

struct CustomQuicUnacked {
    packet_number: u64,
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
            custom_quic_sessions: BTreeMap::new(),
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
        match peer {
            EthernetRelayPeer::Tcp { ip, port } => {
                self.close_tcp(TcpKey {
                    ip: *ip,
                    port: *port,
                });
            }
            EthernetRelayPeer::CustomQuicUdp {
                ip,
                port,
                connection_id,
                ..
            } => {
                self.custom_quic_sessions.remove(&CustomQuicKey {
                    ip: *ip.as_bytes(),
                    port: *port,
                    connection_id: *connection_id,
                });
            }
            EthernetRelayPeer::Udp { .. } => {}
        }
    }

    pub fn handle_frame(&mut self, frame: &[u8]) -> Vec<Vec<u8>> {
        self.handle_frame_at(frame, 0)
    }

    pub fn handle_frame_at(&mut self, frame: &[u8], now_ms: u64) -> Vec<Vec<u8>> {
        let mut events = Vec::new();
        self.handle_frame_at_with_events(frame, now_ms, &mut events)
    }

    pub fn handle_frame_at_with_events(
        &mut self,
        frame: &[u8],
        now_ms: u64,
        events: &mut Vec<EthernetRelayEvent>,
    ) -> Vec<Vec<u8>> {
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
                let ip = IpAddr::from_slice(&ip.src);
                if let Some((connection_id, packet_number)) = custom_quic_decode_ack(payload) {
                    drop(network);
                    self.custom_quic_ack(
                        CustomQuicKey {
                            ip: *ip.as_bytes(),
                            port: header.src_port,
                            connection_id,
                        },
                        packet_number,
                        now_ms,
                    );
                    return out;
                }
                let (peer, messages, transport_ack) =
                    if let Some((connection_id, packet_number, relay_payload)) =
                        custom_quic_decode_data(payload)
                    {
                        let peer = EthernetRelayPeer::CustomQuicUdp {
                            mac: eth.src,
                            ip,
                            port: header.src_port,
                            connection_id,
                        };
                        let Some(messages) = custom_quic_decode_relay_messages(relay_payload)
                        else {
                            return out;
                        };
                        (
                            peer,
                            messages,
                            custom_quic_encode_ack(connection_id, packet_number),
                        )
                    } else {
                        let peer = EthernetRelayPeer::Udp {
                            mac: eth.src,
                            ip,
                            port: header.src_port,
                        };
                        let Ok(message) = decode_packet(payload) else {
                            return out;
                        };
                        (peer, vec![message], Vec::new())
                    };
                if !transport_ack.is_empty() {
                    if let Some(frame) = network.send_udp_eth(
                        eth.src,
                        ip,
                        self.listen_port,
                        header.src_port,
                        &transport_ack,
                    ) {
                        out.push(frame.to_vec());
                    }
                }
                drop(network);
                if let EthernetRelayPeer::CustomQuicUdp {
                    mac,
                    ip,
                    port,
                    connection_id,
                } = &peer
                {
                    self.custom_quic_seen(
                        CustomQuicKey {
                            ip: *ip.as_bytes(),
                            port: *port,
                            connection_id: *connection_id,
                        },
                        *mac,
                        now_ms,
                    );
                }
                let mut relay_outputs = Vec::new();
                for message in messages {
                    for output in self.engine.handle_message(peer.clone(), message) {
                        relay_outputs.push(output);
                    }
                }
                self.push_outputs(relay_outputs, now_ms, &mut out);
            }
            ParsedPacket::Tcp {
                eth,
                ip,
                header,
                payload,
            } if header.dst_port == self.listen_port => {
                drop(network);
                self.handle_tcp_packet(eth.src, ip.src, header, payload, now_ms, &mut out, events);
            }
            _ => {}
        }

        out
    }

    pub fn poll_tcp(&mut self, now_ms: u64) -> Vec<Vec<u8>> {
        let mut events = Vec::new();
        self.poll_tcp_with_events(now_ms, &mut events)
    }

    pub fn poll_tcp_with_events(
        &mut self,
        now_ms: u64,
        events: &mut Vec<EthernetRelayEvent>,
    ) -> Vec<Vec<u8>> {
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
            self.close_tcp_with_event(key, "lifetime_exceeded", events);
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
            self.close_tcp_with_event(key, "retransmit_exhausted", events);
        }
        self.poll_custom_quic(now_ms, &mut out);
        out
    }

    fn custom_quic_seen(&mut self, key: CustomQuicKey, peer_mac: [u8; 6], now_ms: u64) {
        let session = self
            .custom_quic_sessions
            .entry(key)
            .or_insert_with(|| CustomQuicSession {
                peer_mac,
                next_packet_number: 0,
                unacked: Vec::new(),
                last_seen_ms: now_ms,
            });
        session.peer_mac = peer_mac;
        session.last_seen_ms = now_ms;
    }

    fn custom_quic_ack(&mut self, key: CustomQuicKey, packet_number: u64, now_ms: u64) {
        if let Some(session) = self.custom_quic_sessions.get_mut(&key) {
            session.last_seen_ms = now_ms;
            session
                .unacked
                .retain(|packet| packet.packet_number != packet_number);
        }
    }

    fn poll_custom_quic(&mut self, now_ms: u64, out: &mut Vec<Vec<u8>>) {
        let expired = self
            .custom_quic_sessions
            .iter()
            .filter_map(|(key, session)| {
                (now_ms.saturating_sub(session.last_seen_ms) >= CUSTOM_QUIC_SESSION_LIFETIME_MS)
                    .then_some(*key)
            })
            .collect::<Vec<_>>();
        for key in expired {
            self.custom_quic_sessions.remove(&key);
        }

        let mut retransmits = Vec::new();
        for (key, session) in self.custom_quic_sessions.iter_mut() {
            session.unacked.retain_mut(|packet| {
                if now_ms.saturating_sub(packet.sent_ms) < CUSTOM_QUIC_RETRANSMIT_AFTER_MS {
                    return true;
                }
                if packet.attempts >= CUSTOM_QUIC_MAX_RETRANSMITS {
                    return false;
                }
                packet.attempts += 1;
                packet.sent_ms = now_ms;
                retransmits.push((*key, session.peer_mac, packet.payload.clone()));
                true
            });
        }

        let mut network = Network::new(&mut self.stack);
        for (key, mac, payload) in retransmits {
            if let Some(frame) = network.send_udp_eth(
                mac,
                IpAddr::from_slice(&key.ip),
                self.listen_port,
                key.port,
                &payload,
            ) {
                out.push(frame.to_vec());
            }
        }
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

    fn push_outputs(
        &mut self,
        outputs: Vec<RelayOutput<EthernetRelayPeer>>,
        now_ms: u64,
        out: &mut Vec<Vec<u8>>,
    ) {
        for output in outputs {
            self.push_output(output, now_ms, out);
        }
    }

    fn push_message_to_peer(
        &mut self,
        peer: EthernetRelayPeer,
        message: RelayMessage,
        now_ms: u64,
        out: &mut Vec<Vec<u8>>,
    ) {
        self.push_messages_to_peer(peer, &[message], now_ms, out);
    }

    fn push_messages_to_peer(
        &mut self,
        peer: EthernetRelayPeer,
        messages: &[RelayMessage],
        now_ms: u64,
        out: &mut Vec<Vec<u8>>,
    ) {
        if messages.is_empty() {
            return;
        }
        if messages.len() > 1 {
            if let EthernetRelayPeer::CustomQuicUdp {
                mac,
                ip,
                port,
                connection_id,
            } = peer
            {
                self.push_custom_quic_messages(mac, ip, port, connection_id, messages, now_ms, out);
                return;
            }
        }
        let message = messages[0].clone();
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
            EthernetRelayPeer::CustomQuicUdp {
                mac,
                ip,
                port,
                connection_id,
            } => {
                self.push_custom_quic_payload(mac, ip, port, connection_id, &bytes, now_ms, out);
            }
            EthernetRelayPeer::Tcp { ip, port } => {
                out.extend(self.write_tcp_message_at(TcpKey { ip, port }, &bytes, now_ms));
            }
        }
    }

    fn push_custom_quic_payload(
        &mut self,
        mac: [u8; 6],
        ip: IpAddr,
        port: u16,
        connection_id: u64,
        relay_payload: &[u8],
        now_ms: u64,
        out: &mut Vec<Vec<u8>>,
    ) {
        let key = CustomQuicKey {
            ip: *ip.as_bytes(),
            port,
            connection_id,
        };
        let packet_number = {
            let session =
                self.custom_quic_sessions
                    .entry(key)
                    .or_insert_with(|| CustomQuicSession {
                        peer_mac: mac,
                        next_packet_number: 0,
                        unacked: Vec::new(),
                        last_seen_ms: now_ms,
                    });
            session.peer_mac = mac;
            session.last_seen_ms = now_ms;
            let packet_number = session.next_packet_number;
            session.next_packet_number = session.next_packet_number.wrapping_add(1);
            packet_number
        };
        let payload = custom_quic_encode_data(connection_id, packet_number, relay_payload);
        if let Some(session) = self.custom_quic_sessions.get_mut(&key) {
            session.unacked.push(CustomQuicUnacked {
                packet_number,
                payload: payload.clone(),
                sent_ms: now_ms,
                attempts: 0,
            });
        }
        let mut network = Network::new(&mut self.stack);
        if let Some(frame) = network.send_udp_eth(mac, ip, self.listen_port, port, &payload) {
            out.push(frame.to_vec());
        }
    }

    fn push_custom_quic_messages(
        &mut self,
        mac: [u8; 6],
        ip: IpAddr,
        port: u16,
        connection_id: u64,
        messages: &[RelayMessage],
        now_ms: u64,
        out: &mut Vec<Vec<u8>>,
    ) {
        let mut chunk = Vec::new();
        let mut chunk_len = 6usize;
        for message in messages {
            let Ok(encoded) = encode_packet(message) else {
                continue;
            };
            let item_len = 4 + encoded.len();
            if !chunk.is_empty() && chunk_len + item_len > CUSTOM_QUIC_MAX_RELAY_PAYLOAD {
                let payload = custom_quic_encode_encoded_relay_batch(&chunk);
                self.push_custom_quic_payload(mac, ip, port, connection_id, &payload, now_ms, out);
                chunk.clear();
                chunk_len = 6;
            }
            chunk_len += item_len;
            chunk.push(encoded);
        }
        if !chunk.is_empty() {
            let payload = custom_quic_encode_encoded_relay_batch(&chunk);
            self.push_custom_quic_payload(mac, ip, port, connection_id, &payload, now_ms, out);
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
        events: &mut Vec<EthernetRelayEvent>,
    ) {
        let key = TcpKey {
            ip: peer_ip,
            port: header.src_port,
        };

        if header.flags & TCP_FLAG_RST != 0 {
            self.close_tcp_with_event(key, "rst", events);
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
            events.push(EthernetRelayEvent {
                kind: EthernetRelayEventKind::TcpOpened,
                ip: key.ip,
                port: key.port,
            });
            return;
        }

        if header.flags & TCP_FLAG_FIN != 0 {
            if let Some(session) = self.tcp_sessions.get_mut(&key) {
                session.recv_next = header.seq.wrapping_add(1);
            }
            if let Some(frame) = self.send_tcp_control(key, TCP_FLAG_ACK, now_ms) {
                out.push(frame);
            }
            self.close_tcp_with_event(key, "fin", events);
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
                    self.close_tcp_with_event(key, "frame_too_large", events);
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

    fn close_tcp_with_event(
        &mut self,
        key: TcpKey,
        reason: &'static str,
        events: &mut Vec<EthernetRelayEvent>,
    ) {
        let existed = self.tcp_sessions.remove(&key).is_some();
        self.engine.remove_peer(&EthernetRelayPeer::Tcp {
            ip: key.ip,
            port: key.port,
        });
        if existed {
            events.push(EthernetRelayEvent {
                kind: EthernetRelayEventKind::TcpClosed { reason },
                ip: key.ip,
                port: key.port,
            });
        }
    }
}

impl VirtioRelay {
    pub fn new(relay: EthernetRelay) -> Self {
        Self {
            relay,
            #[cfg(feature = "wss")]
            wss_config: None,
            rx_buf: [0; VIRTIO_RX_BUFFER_LEN],
        }
    }

    #[cfg(feature = "wss")]
    pub fn with_wss_config(relay: EthernetRelay, wss_config: ServerConfig) -> Self {
        Self {
            relay,
            wss_config: Some(wss_config),
            rx_buf: [0; VIRTIO_RX_BUFFER_LEN],
        }
    }

    pub fn relay(&self) -> &EthernetRelay {
        &self.relay
    }

    pub fn relay_mut(&mut self) -> &mut EthernetRelay {
        &mut self.relay
    }

    #[cfg(feature = "wss")]
    pub fn wss_config(&self) -> Option<&ServerConfig> {
        self.wss_config.as_ref()
    }

    #[cfg(feature = "wss")]
    pub fn set_wss_config(&mut self, wss_config: ServerConfig) {
        self.wss_config = Some(wss_config);
    }

    pub fn poll(
        &mut self,
        net: &mut impl FrameDevice,
        now_ms: u64,
        events: &mut Vec<EthernetRelayEvent>,
    ) -> VirtioRelayPoll {
        self.poll_with_budget(net, now_ms, VIRTIO_DEFAULT_POLL_BUDGET, events)
    }

    pub fn poll_with_budget(
        &mut self,
        net: &mut impl FrameDevice,
        now_ms: u64,
        budget: usize,
        events: &mut Vec<EthernetRelayEvent>,
    ) -> VirtioRelayPoll {
        let mut stats = VirtioRelayPoll::default();

        for frame in self.relay.poll_tcp_with_events(now_ms, events) {
            send_frame(net, &frame, &mut stats);
        }

        for _ in 0..budget {
            match net.try_recv_frame(&mut self.rx_buf) {
                Ok(Some(len)) => {
                    stats.rx_frames += 1;
                    let frames =
                        self.relay
                            .handle_frame_at_with_events(&self.rx_buf[..len], now_ms, events);
                    for frame in frames {
                        send_frame(net, &frame, &mut stats);
                    }
                }
                Ok(None) => break,
                Err(FrameDriverError::InvalidBufferLength) => {
                    stats.dropped_rx_frames += 1;
                    break;
                }
                Err(_) => {
                    stats.dropped_rx_frames += 1;
                    break;
                }
            }
        }

        stats
    }
}

fn send_frame(net: &mut impl FrameDevice, frame: &[u8], stats: &mut VirtioRelayPoll) {
    match net.try_send_frame(frame) {
        Ok(()) => stats.tx_frames += 1,
        Err(_) => stats.tx_errors += 1,
    }
}

const CUSTOM_QUIC_MAGIC: &[u8; 4] = b"ERQ0";
const CUSTOM_QUIC_DATA: u8 = 1;
const CUSTOM_QUIC_ACK: u8 = 2;
const CUSTOM_QUIC_HEADER_LEN: usize = 4 + 1 + 8 + 8 + 2;
const CUSTOM_QUIC_BATCH_MAGIC: &[u8; 4] = b"ERQB";
const CUSTOM_QUIC_MAX_RELAY_PAYLOAD: usize = 1100;

fn custom_quic_decode_data(payload: &[u8]) -> Option<(u64, u64, &[u8])> {
    if payload.len() < CUSTOM_QUIC_HEADER_LEN
        || &payload[..4] != CUSTOM_QUIC_MAGIC
        || payload[4] != CUSTOM_QUIC_DATA
    {
        return None;
    }
    let connection_id = u64::from_be_bytes(payload[5..13].try_into().ok()?);
    let packet_number = u64::from_be_bytes(payload[13..21].try_into().ok()?);
    let len = u16::from_be_bytes(payload[21..23].try_into().ok()?) as usize;
    let end = CUSTOM_QUIC_HEADER_LEN.checked_add(len)?;
    if end > payload.len() {
        return None;
    }
    Some((
        connection_id,
        packet_number,
        &payload[CUSTOM_QUIC_HEADER_LEN..end],
    ))
}

fn custom_quic_decode_ack(payload: &[u8]) -> Option<(u64, u64)> {
    if payload.len() < CUSTOM_QUIC_HEADER_LEN
        || &payload[..4] != CUSTOM_QUIC_MAGIC
        || payload[4] != CUSTOM_QUIC_ACK
    {
        return None;
    }
    let connection_id = u64::from_be_bytes(payload[5..13].try_into().ok()?);
    let packet_number = u64::from_be_bytes(payload[13..21].try_into().ok()?);
    Some((connection_id, packet_number))
}

fn custom_quic_decode_relay_messages(payload: &[u8]) -> Option<Vec<RelayMessage>> {
    if payload.len() < 6 || &payload[..4] != CUSTOM_QUIC_BATCH_MAGIC {
        return decode_packet(payload).ok().map(|message| vec![message]);
    }
    let count = u16::from_be_bytes(payload[4..6].try_into().ok()?) as usize;
    let mut offset = 6usize;
    let mut messages = Vec::with_capacity(count);
    for _ in 0..count {
        if payload.len().saturating_sub(offset) < 4 {
            return None;
        }
        let len = u32::from_be_bytes(payload[offset..offset + 4].try_into().ok()?) as usize;
        offset = offset.checked_add(4)?;
        let end = offset.checked_add(len)?;
        if end > payload.len() {
            return None;
        }
        messages.push(decode_packet(&payload[offset..end]).ok()?);
        offset = end;
    }
    Some(messages)
}

fn custom_quic_encode_relay_batch(messages: &[RelayMessage]) -> Vec<u8> {
    let encoded = messages
        .iter()
        .map(|message| encode_packet(message).ok())
        .collect::<Option<Vec<_>>>()
        .unwrap_or_default();
    custom_quic_encode_encoded_relay_batch(&encoded)
}

fn custom_quic_encode_encoded_relay_batch(encoded: &[Vec<u8>]) -> Vec<u8> {
    let len = encoded.iter().map(|item| 4 + item.len()).sum::<usize>();
    let mut out = Vec::with_capacity(6 + len);
    out.extend_from_slice(CUSTOM_QUIC_BATCH_MAGIC);
    out.extend_from_slice(&(encoded.len().min(u16::MAX as usize) as u16).to_be_bytes());
    for item in encoded {
        out.extend_from_slice(&(item.len() as u32).to_be_bytes());
        out.extend_from_slice(&item);
    }
    out
}

fn custom_quic_encode_data(
    connection_id: u64,
    packet_number: u64,
    relay_payload: &[u8],
) -> Vec<u8> {
    let len = relay_payload.len().min(u16::MAX as usize);
    let mut out = Vec::with_capacity(CUSTOM_QUIC_HEADER_LEN + len);
    out.extend_from_slice(CUSTOM_QUIC_MAGIC);
    out.push(CUSTOM_QUIC_DATA);
    out.extend_from_slice(&connection_id.to_be_bytes());
    out.extend_from_slice(&packet_number.to_be_bytes());
    out.extend_from_slice(&(len as u16).to_be_bytes());
    out.extend_from_slice(&relay_payload[..len]);
    out
}

fn custom_quic_encode_ack(connection_id: u64, packet_number: u64) -> Vec<u8> {
    let mut out = Vec::with_capacity(CUSTOM_QUIC_HEADER_LEN);
    out.extend_from_slice(CUSTOM_QUIC_MAGIC);
    out.push(CUSTOM_QUIC_ACK);
    out.extend_from_slice(&connection_id.to_be_bytes());
    out.extend_from_slice(&packet_number.to_be_bytes());
    out.extend_from_slice(&0u16.to_be_bytes());
    out
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
