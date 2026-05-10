use edgerun_crypto::sha256;
use edgerun_protocols::tls::certificate_gen::generate_self_signed;
use edgerun_protocols::verify::verify_message_signature;
use edgerun_wire::{
    RELAY_DELIVERY_STATUS_ACCEPTED, RELAY_REPORT_STATUS_ACCEPTED, RELAY_WIRE_ABI_VERSION, RelayAck,
    RelayDeliveryReceipt, RelayDeliveryReport, RelayDeliveryReportReceipt, RelayDeliveryRequest,
    RelayIdentity, RelayMessage, RelayRegister, RelaySignature, RelaySubmit,
    SIGNATURE_ALGORITHM_ECDSA_P256_SHA256, SIGNATURE_ALGORITHM_ED25519, relay_message_bytes,
    relay_message_from_bytes,
};
use std::collections::HashMap;
use std::fmt;
use std::io::{self, ErrorKind, Read, Write};
use std::net::{Shutdown, SocketAddr, TcpListener, TcpStream, ToSocketAddrs, UdpSocket};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

#[cfg(feature = "virtio")]
use edgerun_protocols::ethernet_ipv4::{
    ARP_OP_REQUEST, ETH_TYPE_IPV4, IP_PROTO_TCP, IpAddr, IpStack, Network, ParsedPacket,
    TCP_FLAG_ACK, TCP_FLAG_FIN, TCP_FLAG_PSH, TCP_FLAG_RST, TCP_FLAG_SYN, checksum,
};

type WsStream = edgerun_tungstenite::WebSocket<TcpStream>;
type WssStream = edgerun_tungstenite::WebSocket<
    edgerun_rusttls::StreamOwned<edgerun_rusttls::ServerConnection, TcpStream>,
>;

pub const MAX_FRAME_LEN: usize = 1024 * 1024;
pub const MAX_PAYLOAD_LEN: usize = 512 * 1024;
pub const MAX_ROUTES: usize = 65_536;
pub const MAX_PENDING_DELIVERIES: usize = 65_536;

const REGISTER_DOMAIN: &[u8] = b"edgerun:v0:relay:register";
const SUBMIT_DOMAIN: &[u8] = b"edgerun:v0:relay:submit";
const DELIVERY_RECEIPT_DOMAIN: &[u8] = b"edgerun:v0:relay:delivery-receipt";
const REPORT_RECEIPT_DOMAIN: &[u8] = b"edgerun:v0:relay:delivery-report-receipt";
const MAX_ACK_TEXT_LEN: usize = 256;
const CONNECTION_LIFETIME: Duration = Duration::from_secs(60);
const TCP_READ_POLL_TIMEOUT: Duration = Duration::from_millis(250);
const TCP_WRITE_TIMEOUT: Duration = Duration::from_secs(10);
const TLS_HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(10);
#[cfg(feature = "virtio")]
const VIRTIO_TCP_MSS: usize = 1200;
#[cfg(feature = "virtio")]
const VIRTIO_TCP_INITIAL_SEQ: u32 = 0xED6E_0001;
#[cfg(feature = "virtio")]
const VIRTIO_TCP_RETRANSMIT_AFTER: Duration = Duration::from_secs(1);
#[cfg(feature = "virtio")]
const VIRTIO_TCP_MAX_RETRANSMITS: u8 = 4;

#[derive(Clone, Default)]
pub struct Relay {
    routes: Arc<Mutex<HashMap<RouteKey, PeerWriter>>>,
    pending: Arc<Mutex<HashMap<[u8; 32], PendingDelivery>>>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct RouteKey {
    algorithm: u16,
    public_key: Vec<u8>,
}

#[derive(Clone)]
struct PeerWriter {
    endpoint: RelayEndpoint,
}

#[derive(Clone)]
struct PendingDelivery {
    sender: RelayIdentity,
    recipient: RelayIdentity,
    sender_endpoint: RelayEndpoint,
    submit: RelaySubmit,
}

#[derive(Clone)]
enum RelayEndpoint {
    Stream(Arc<Mutex<TcpStream>>),
    Udp(Arc<UdpSocket>, SocketAddr),
    WebSocket(Arc<Mutex<WsStream>>),
    SecureWebSocket(Arc<Mutex<WssStream>>),
    #[cfg(feature = "virtio")]
    VirtioUdp(VirtioUdpPeer),
    #[cfg(feature = "virtio")]
    VirtioTcp(VirtioTcpPeer),
}

#[cfg(feature = "virtio")]
#[derive(Clone)]
struct VirtioUdpPeer {
    link: VirtioUdpLink,
    mac: [u8; 6],
    ip: IpAddr,
    port: u16,
}

#[cfg(feature = "virtio")]
#[derive(Clone)]
struct VirtioUdpLink {
    net: Arc<Mutex<VirtioNetHandle>>,
    stack: Arc<Mutex<IpStack>>,
    listen_port: u16,
}

#[cfg(feature = "virtio")]
#[derive(Clone)]
struct VirtioTcpPeer {
    link: VirtioTcpLink,
    key: VirtioTcpKey,
}

#[cfg(feature = "virtio")]
#[derive(Clone)]
struct VirtioTcpLink {
    net: Arc<Mutex<VirtioNetHandle>>,
    stack: Arc<Mutex<IpStack>>,
    sessions: Arc<Mutex<HashMap<VirtioTcpKey, VirtioTcpSession>>>,
    listen_port: u16,
}

#[cfg(feature = "virtio")]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct VirtioTcpKey {
    peer_ip: [u8; 4],
    peer_port: u16,
}

#[cfg(feature = "virtio")]
struct VirtioTcpSession {
    peer_mac: [u8; 6],
    send_seq: u32,
    recv_next: u32,
    read_buf: Vec<u8>,
    unacked: Vec<VirtioTcpUnacked>,
    registered_node: Option<RouteKey>,
    opened_at: Instant,
}

#[cfg(feature = "virtio")]
struct VirtioTcpUnacked {
    seq: u32,
    len: u32,
    flags: u8,
    payload: Vec<u8>,
    sent_at: Instant,
    attempts: u8,
}

#[cfg(feature = "virtio")]
struct VirtioNetHandle(edgerun_virtio::VirtNet);

#[cfg(feature = "virtio")]
unsafe impl Send for VirtioNetHandle {}

#[cfg(feature = "virtio")]
unsafe impl Sync for VirtioNetHandle {}

impl Relay {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn serve<A: ToSocketAddrs>(&self, addr: A) -> io::Result<()> {
        let listener = TcpListener::bind(addr)?;
        self.serve_listener(listener)
    }

    pub fn serve_udp<A: ToSocketAddrs>(&self, addr: A) -> io::Result<()> {
        let socket = Arc::new(UdpSocket::bind(addr)?);
        self.serve_udp_socket(socket)
    }

    pub fn serve_udp_socket(&self, socket: Arc<UdpSocket>) -> io::Result<()> {
        let mut buf = vec![0u8; MAX_FRAME_LEN];
        loop {
            let (len, peer_addr) = socket.recv_from(&mut buf)?;
            let endpoint = RelayEndpoint::Udp(Arc::clone(&socket), peer_addr);
            let message = match decode_packet(&buf[..len]) {
                Ok(message) => message,
                Err(error) => {
                    let _ = write_ack(&endpoint, false, 400, &format!("read failed: {error}"));
                    continue;
                }
            };
            let _ = self.handle_relay_message(message, &endpoint, None);
        }
    }

    #[cfg(feature = "virtio")]
    pub fn serve_virtio_udp(
        &self,
        net: edgerun_virtio::VirtNet,
        ip: IpAddr,
        netmask: IpAddr,
        gateway: IpAddr,
        listen_port: u16,
    ) -> io::Result<()> {
        let mac = net.get_mac();
        let mut stack = IpStack::new();
        stack.configure(ip, netmask, gateway, mac);
        let link = VirtioUdpLink {
            net: Arc::new(Mutex::new(VirtioNetHandle(net))),
            stack: Arc::new(Mutex::new(stack)),
            listen_port,
        };
        log_relay(format_args!(
            "virtio-udp relay listening ip={}.{}.{}.{} port={listen_port} mac={:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
            ip.as_bytes()[0],
            ip.as_bytes()[1],
            ip.as_bytes()[2],
            ip.as_bytes()[3],
            mac[0],
            mac[1],
            mac[2],
            mac[3],
            mac[4],
            mac[5],
        ));
        self.serve_virtio_udp_link(link)
    }

    #[cfg(feature = "virtio")]
    fn serve_virtio_udp_link(&self, link: VirtioUdpLink) -> io::Result<()> {
        let mut frame = [0u8; 2048];
        loop {
            let len = {
                let mut net = link.net.lock().expect("virtio net poisoned");
                match net.0.try_recv(&mut frame) {
                    Ok(Some(len)) => len,
                    Ok(None) => {
                        thread::sleep(Duration::from_millis(1));
                        continue;
                    }
                    Err(error) => {
                        log_relay(format_args!("virtio-udp recv failed: {error:?}"));
                        thread::sleep(Duration::from_millis(10));
                        continue;
                    }
                }
            };

            let mut stack = link.stack.lock().expect("virtio ip stack poisoned");
            let mut network = Network::new(&mut stack);
            let parsed = match network.recv(&frame[..len]) {
                Some(parsed) => parsed,
                None => continue,
            };

            match parsed {
                ParsedPacket::Arp { header, .. } => {
                    if header.oper == ARP_OP_REQUEST && header.tpa == *network.stack.ip.as_bytes() {
                        if let Some(reply) = network.send_arp_reply(&header) {
                            let _ = link
                                .net
                                .lock()
                                .expect("virtio net poisoned")
                                .0
                                .try_send(reply);
                        }
                    }
                }
                ParsedPacket::Udp {
                    eth,
                    ip,
                    header,
                    payload,
                } if header.dst_port == link.listen_port => {
                    let endpoint = RelayEndpoint::VirtioUdp(VirtioUdpPeer {
                        link: link.clone(),
                        mac: eth.src,
                        ip: IpAddr::from_slice(&ip.src),
                        port: header.src_port,
                    });
                    let message = match decode_packet(payload) {
                        Ok(message) => message,
                        Err(error) => {
                            let _ =
                                write_ack(&endpoint, false, 400, &format!("read failed: {error}"));
                            continue;
                        }
                    };
                    drop(network);
                    drop(stack);
                    let _ = self.handle_relay_message(message, &endpoint, None);
                }
                _ => {}
            }
        }
    }

    #[cfg(feature = "virtio")]
    pub fn serve_virtio_tcp(
        &self,
        net: edgerun_virtio::VirtNet,
        ip: IpAddr,
        netmask: IpAddr,
        gateway: IpAddr,
        listen_port: u16,
    ) -> io::Result<()> {
        let mac = net.get_mac();
        let mut stack = IpStack::new();
        stack.configure(ip, netmask, gateway, mac);
        let link = VirtioTcpLink {
            net: Arc::new(Mutex::new(VirtioNetHandle(net))),
            stack: Arc::new(Mutex::new(stack)),
            sessions: Arc::new(Mutex::new(HashMap::new())),
            listen_port,
        };
        log_relay(format_args!(
            "virtio-tcp relay listening ip={}.{}.{}.{} port={listen_port} mac={:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
            ip.as_bytes()[0],
            ip.as_bytes()[1],
            ip.as_bytes()[2],
            ip.as_bytes()[3],
            mac[0],
            mac[1],
            mac[2],
            mac[3],
            mac[4],
            mac[5],
        ));
        self.serve_virtio_tcp_link(link)
    }

    #[cfg(feature = "virtio")]
    fn serve_virtio_tcp_link(&self, link: VirtioTcpLink) -> io::Result<()> {
        let mut frame = [0u8; 2048];
        loop {
            let len = {
                let mut net = link.net.lock().expect("virtio net poisoned");
                match net.0.try_recv(&mut frame) {
                    Ok(Some(len)) => len,
                    Ok(None) => {
                        thread::sleep(Duration::from_millis(1));
                        self.expire_virtio_tcp_sessions(&link);
                        self.retransmit_virtio_tcp_sessions(&link);
                        continue;
                    }
                    Err(error) => {
                        log_relay(format_args!("virtio-tcp recv failed: {error:?}"));
                        thread::sleep(Duration::from_millis(10));
                        continue;
                    }
                }
            };

            let mut stack = link.stack.lock().expect("virtio ip stack poisoned");
            let mut network = Network::new(&mut stack);
            let parsed = match network.recv(&frame[..len]) {
                Some(parsed) => parsed,
                None => continue,
            };

            match parsed {
                ParsedPacket::Arp { header, .. } => {
                    if header.oper == ARP_OP_REQUEST && header.tpa == *network.stack.ip.as_bytes() {
                        if let Some(reply) = network.send_arp_reply(&header) {
                            let _ = link
                                .net
                                .lock()
                                .expect("virtio net poisoned")
                                .0
                                .try_send(reply);
                        }
                    }
                }
                ParsedPacket::Tcp {
                    eth,
                    ip,
                    header,
                    payload,
                } if header.dst_port == link.listen_port => {
                    drop(network);
                    drop(stack);
                    self.handle_virtio_tcp_packet(&link, eth.src, ip.src, header, payload);
                }
                _ => {}
            }
        }
    }

    #[cfg(feature = "virtio")]
    fn handle_virtio_tcp_packet(
        &self,
        link: &VirtioTcpLink,
        peer_mac: [u8; 6],
        peer_ip: [u8; 4],
        header: edgerun_protocols::ethernet_ipv4::TcpHeader,
        payload: &[u8],
    ) {
        let key = VirtioTcpKey {
            peer_ip,
            peer_port: header.src_port,
        };

        if header.flags & TCP_FLAG_RST != 0 {
            self.close_virtio_tcp_session(link, key, "rst");
            return;
        }

        if header.flags & TCP_FLAG_SYN != 0 {
            let recv_next = header.seq.wrapping_add(1);
            {
                let mut sessions = link.sessions.lock().expect("virtio tcp sessions poisoned");
                sessions.insert(
                    key,
                    VirtioTcpSession {
                        peer_mac,
                        send_seq: VIRTIO_TCP_INITIAL_SEQ,
                        recv_next,
                        read_buf: Vec::new(),
                        unacked: Vec::new(),
                        registered_node: None,
                        opened_at: Instant::now(),
                    },
                );
            }
            let _ = send_virtio_tcp_control(link, key, TCP_FLAG_SYN | TCP_FLAG_ACK);
            log_relay(format_args!(
                "virtio-tcp connection opened peer={}.{}.{}.{}:{}",
                peer_ip[0], peer_ip[1], peer_ip[2], peer_ip[3], header.src_port
            ));
            return;
        }

        if header.flags & TCP_FLAG_FIN != 0 {
            {
                let mut sessions = link.sessions.lock().expect("virtio tcp sessions poisoned");
                if let Some(session) = sessions.get_mut(&key) {
                    session.recv_next = header.seq.wrapping_add(1);
                }
            }
            let _ = send_virtio_tcp_control(link, key, TCP_FLAG_ACK);
            self.close_virtio_tcp_session(link, key, "fin");
            return;
        }

        let endpoint = RelayEndpoint::VirtioTcp(VirtioTcpPeer {
            link: link.clone(),
            key,
        });
        let mut messages = Vec::new();
        let mut registered_node = {
            let mut sessions = link.sessions.lock().expect("virtio tcp sessions poisoned");
            let Some(session) = sessions.get_mut(&key) else {
                return;
            };
            if header.flags & TCP_FLAG_ACK != 0 {
                prune_virtio_tcp_acked(session, header.ack);
            }
            session.peer_mac = peer_mac;
            if !payload.is_empty() && header.seq == session.recv_next {
                session.recv_next = session.recv_next.wrapping_add(payload.len() as u32);
                if session.read_buf.len().saturating_add(payload.len()) > MAX_FRAME_LEN + 4 {
                    drop(sessions);
                    let _ = send_virtio_tcp_control(link, key, TCP_FLAG_RST);
                    self.close_virtio_tcp_session(link, key, "frame_too_large");
                    return;
                }
                session.read_buf.extend_from_slice(payload);
                messages = drain_virtio_tcp_messages(&mut session.read_buf);
            }
            session.registered_node.clone()
        };

        let _ = send_virtio_tcp_control(link, key, TCP_FLAG_ACK);
        for message in messages {
            if let Err(error) =
                self.handle_relay_message(message, &endpoint, Some(&mut registered_node))
            {
                log_relay(format_args!("virtio-tcp relay message failed: {error}"));
                break;
            }
        }
        if let Some(session) = link
            .sessions
            .lock()
            .expect("virtio tcp sessions poisoned")
            .get_mut(&key)
        {
            session.registered_node = registered_node;
        }
    }

    #[cfg(feature = "virtio")]
    fn close_virtio_tcp_session(
        &self,
        link: &VirtioTcpLink,
        key: VirtioTcpKey,
        reason: &'static str,
    ) {
        let session = link
            .sessions
            .lock()
            .expect("virtio tcp sessions poisoned")
            .remove(&key);
        if let Some(session) = session {
            let endpoint = RelayEndpoint::VirtioTcp(VirtioTcpPeer {
                link: link.clone(),
                key,
            });
            if let Some(route) = session.registered_node {
                self.remove_route_if_same_endpoint(&route, &endpoint);
            }
            log_relay(format_args!(
                "virtio-tcp connection closed peer={}.{}.{}.{}:{} reason={reason}",
                key.peer_ip[0], key.peer_ip[1], key.peer_ip[2], key.peer_ip[3], key.peer_port
            ));
        }
    }

    #[cfg(feature = "virtio")]
    fn expire_virtio_tcp_sessions(&self, link: &VirtioTcpLink) {
        let now = Instant::now();
        let expired = {
            let sessions = link.sessions.lock().expect("virtio tcp sessions poisoned");
            sessions
                .iter()
                .filter_map(|(key, session)| {
                    (now.duration_since(session.opened_at) >= CONNECTION_LIFETIME).then_some(*key)
                })
                .collect::<Vec<_>>()
        };
        for key in expired {
            let _ = send_virtio_tcp_control(link, key, TCP_FLAG_FIN | TCP_FLAG_ACK);
            self.close_virtio_tcp_session(link, key, "lifetime_exceeded");
        }
    }

    #[cfg(feature = "virtio")]
    fn retransmit_virtio_tcp_sessions(&self, link: &VirtioTcpLink) {
        let now = Instant::now();
        let mut retransmits = Vec::new();
        let mut close = Vec::new();
        {
            let mut sessions = link.sessions.lock().expect("virtio tcp sessions poisoned");
            for (key, session) in sessions.iter_mut() {
                for segment in &mut session.unacked {
                    if now.duration_since(segment.sent_at) < VIRTIO_TCP_RETRANSMIT_AFTER {
                        continue;
                    }
                    if segment.attempts >= VIRTIO_TCP_MAX_RETRANSMITS {
                        close.push(*key);
                        break;
                    }
                    segment.attempts += 1;
                    segment.sent_at = now;
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
        }
        for (key, peer_mac, seq, ack, flags, payload) in retransmits {
            let _ = send_virtio_tcp_segment_raw(link, key, peer_mac, seq, ack, flags, &payload);
        }
        for key in close {
            self.close_virtio_tcp_session(link, key, "retransmit_exhausted");
        }
    }

    pub fn serve_ws<A: ToSocketAddrs>(&self, addr: A) -> io::Result<()> {
        let listener = TcpListener::bind(addr)?;
        self.serve_ws_listener(listener)
    }

    pub fn serve_wss<A: ToSocketAddrs>(&self, addr: A) -> io::Result<()> {
        let listener = TcpListener::bind(addr)?;
        self.serve_wss_listener(listener)
    }

    pub fn serve_listener(&self, listener: TcpListener) -> io::Result<()> {
        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    let relay = self.clone();
                    thread::spawn(move || {
                        if let Err(error) = relay.handle_stream(stream) {
                            log_relay(format_args!("tcp connection handler failed: {error}"));
                        }
                    });
                }
                Err(error) => return Err(error),
            }
        }
        Ok(())
    }

    pub fn serve_ws_listener(&self, listener: TcpListener) -> io::Result<()> {
        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    let relay = self.clone();
                    thread::spawn(move || {
                        if let Err(error) = relay.handle_websocket(stream) {
                            log_relay(format_args!("ws connection handler failed: {error}"));
                        }
                    });
                }
                Err(error) => return Err(error),
            }
        }
        Ok(())
    }

    pub fn serve_wss_listener(&self, listener: TcpListener) -> io::Result<()> {
        let config = Arc::new(self_signed_wss_config()?);
        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    let relay = self.clone();
                    let config = Arc::clone(&config);
                    thread::spawn(move || {
                        if let Err(error) = relay.handle_secure_websocket(stream, config) {
                            log_relay(format_args!("wss connection handler failed: {error}"));
                        }
                    });
                }
                Err(error) => return Err(error),
            }
        }
        Ok(())
    }

    pub fn handle_stream(&self, mut stream: TcpStream) -> io::Result<()> {
        configure_tcp_stream(&stream)?;
        let peer = stream
            .peer_addr()
            .map(|addr| addr.to_string())
            .unwrap_or_else(|_| "unknown".to_owned());
        let deadline = Instant::now() + CONNECTION_LIFETIME;
        log_relay(format_args!("tcp connection opened peer={peer}"));
        let endpoint = RelayEndpoint::Stream(Arc::new(Mutex::new(stream.try_clone()?)));
        let mut registered_node = None;

        loop {
            if Instant::now() >= deadline {
                let _ = write_ack(&endpoint, false, 408, "relay connection lifetime exceeded");
                log_relay(format_args!(
                    "tcp connection closed peer={peer} reason=lifetime_exceeded"
                ));
                break;
            }
            stream.set_read_timeout(Some(read_timeout_until(deadline)))?;
            let message = match read_message(&mut stream) {
                Ok(message) => message,
                Err(error)
                    if error.kind() == ErrorKind::TimedOut
                        || error.kind() == ErrorKind::WouldBlock =>
                {
                    continue;
                }
                Err(error)
                    if error.kind() == ErrorKind::UnexpectedEof
                        || error.kind() == ErrorKind::ConnectionReset =>
                {
                    log_relay(format_args!(
                        "tcp connection closed peer={peer} reason=peer_closed"
                    ));
                    break;
                }
                Err(error) => {
                    let _ = write_ack(&endpoint, false, 400, &format!("read failed: {error}"));
                    log_relay(format_args!(
                        "tcp connection closed peer={peer} reason=read_failed error={error}"
                    ));
                    break;
                }
            };

            self.handle_relay_message(message, &endpoint, Some(&mut registered_node))?;
        }

        if let Some(key) = registered_node {
            self.remove_route_if_same_endpoint(&key, &endpoint);
        }
        close_endpoint(&endpoint);

        Ok(())
    }

    pub fn handle_websocket(&self, stream: TcpStream) -> io::Result<()> {
        configure_tcp_stream(&stream)?;
        let peer = stream
            .peer_addr()
            .map(|addr| addr.to_string())
            .unwrap_or_else(|_| "unknown".to_owned());
        log_relay(format_args!("ws connection opened peer={peer}"));
        let mut ws = edgerun_tungstenite::accept(stream)
            .map_err(|error| io::Error::new(ErrorKind::InvalidData, error.to_string()))?;
        ws.get_mut().set_nonblocking(true)?;
        let endpoint = RelayEndpoint::WebSocket(Arc::new(Mutex::new(ws)));
        self.handle_websocket_endpoint(endpoint, "ws", peer)
    }

    pub fn handle_secure_websocket(
        &self,
        stream: TcpStream,
        config: Arc<edgerun_rusttls::ServerConfig>,
    ) -> io::Result<()> {
        configure_tcp_stream(&stream)?;
        let peer = stream
            .peer_addr()
            .map(|addr| addr.to_string())
            .unwrap_or_else(|_| "unknown".to_owned());
        log_relay(format_args!("wss connection opened peer={peer}"));
        stream.set_read_timeout(Some(TLS_HANDSHAKE_TIMEOUT))?;
        let connection = edgerun_rusttls::ServerConnection::new(config)
            .map_err(|error| io::Error::new(ErrorKind::InvalidData, error.to_string()))?;
        let mut tls: edgerun_rusttls::StreamOwned<edgerun_rusttls::ServerConnection, TcpStream> =
            edgerun_rusttls::StreamOwned::<edgerun_rusttls::ServerConnection, TcpStream>::new(
                connection, stream,
            );
        tls.handshake()?;
        tls.get_mut().set_read_timeout(None)?;
        let mut ws = edgerun_tungstenite::accept(tls)
            .map_err(|error| io::Error::new(ErrorKind::InvalidData, error.to_string()))?;
        ws.get_mut().get_mut().set_nonblocking(true)?;
        let endpoint = RelayEndpoint::SecureWebSocket(Arc::new(Mutex::new(ws)));
        self.handle_websocket_endpoint(endpoint, "wss", peer)
    }

    fn handle_websocket_endpoint(
        &self,
        endpoint: RelayEndpoint,
        transport: &'static str,
        peer: String,
    ) -> io::Result<()> {
        let mut registered_node = None;
        let deadline = Instant::now() + CONNECTION_LIFETIME;

        loop {
            if Instant::now() >= deadline {
                let _ = write_ack(&endpoint, false, 408, "relay connection lifetime exceeded");
                log_relay(format_args!(
                    "{transport} connection closed peer={peer} reason=lifetime_exceeded"
                ));
                break;
            }
            let message = {
                match read_ws_endpoint(&endpoint) {
                    Ok(message) => message,
                    Err(edgerun_tungstenite::Error::ConnectionClosed)
                    | Err(edgerun_tungstenite::Error::AlreadyClosed) => {
                        log_relay(format_args!(
                            "{transport} connection closed peer={peer} reason=peer_closed"
                        ));
                        break;
                    }
                    Err(edgerun_tungstenite::Error::Io(error))
                        if error.kind() == ErrorKind::WouldBlock =>
                    {
                        thread::sleep(Duration::from_millis(1));
                        continue;
                    }
                    Err(error) => {
                        let _ = write_ack(&endpoint, false, 400, &format!("read failed: {error}"));
                        log_relay(format_args!(
                            "{transport} connection closed peer={peer} reason=read_failed error={error}"
                        ));
                        break;
                    }
                }
            };

            let message: edgerun_tungstenite::Message = message.into();
            if message.is_close() {
                log_relay(format_args!(
                    "{transport} connection closed peer={peer} reason=close_frame"
                ));
                break;
            }
            if message.is_ping() {
                continue;
            }
            if !message.is_binary() {
                write_ack(
                    &endpoint,
                    false,
                    400,
                    "relay websocket frames must be binary",
                )?;
                continue;
            }
            let bytes = message.into_data();
            let message = match decode_packet(bytes.as_ref()) {
                Ok(message) => message,
                Err(error) => {
                    write_ack(&endpoint, false, 400, &format!("read failed: {error}"))?;
                    continue;
                }
            };
            self.handle_relay_message(message, &endpoint, Some(&mut registered_node))?;
        }

        if let Some(key) = registered_node {
            self.remove_route_if_same_endpoint(&key, &endpoint);
        }
        close_endpoint(&endpoint);

        Ok(())
    }

    pub fn registered_len(&self) -> usize {
        self.routes.lock().expect("relay routes poisoned").len()
    }

    fn handle_relay_message(
        &self,
        message: RelayMessage,
        endpoint: &RelayEndpoint,
        mut registered_node: Option<&mut Option<RouteKey>>,
    ) -> io::Result<()> {
        match message {
            RelayMessage::Register(register) => {
                if !verify_register(&register) {
                    write_ack(endpoint, false, 401, "invalid register signature")?;
                    return Ok(());
                }
                let key = RouteKey::from(&register.node);
                {
                    let routes = self.routes.lock().expect("relay routes poisoned");
                    if routes.len() >= MAX_ROUTES && !routes.contains_key(&key) {
                        write_ack(endpoint, false, 503, "relay route table is full")?;
                        return Ok(());
                    }
                }
                if let Some(registered_node) = registered_node.as_deref_mut() {
                    *registered_node = Some(key.clone());
                }
                self.routes.lock().expect("relay routes poisoned").insert(
                    key,
                    PeerWriter {
                        endpoint: endpoint.clone(),
                    },
                );
                write_ack(endpoint, true, 200, "registered")?;
            }
            RelayMessage::Submit(submit) => match self.accept_submit(submit, endpoint) {
                Ok(()) => write_ack(endpoint, true, 202, "delivery requested")?,
                Err(error) => write_ack(endpoint, false, error.code(), &error.to_string())?,
            },
            RelayMessage::DeliveryReceipt(receipt) => match self.accept_delivery_receipt(receipt) {
                Ok(()) => write_ack(endpoint, true, 202, "delivery report sent")?,
                Err(error) => write_ack(endpoint, false, error.code(), &error.to_string())?,
            },
            RelayMessage::DeliveryReportReceipt(receipt) => {
                match self.accept_report_receipt(receipt) {
                    Ok(()) => write_ack(endpoint, true, 200, "delivery report acknowledged")?,
                    Err(error) => write_ack(endpoint, false, error.code(), &error.to_string())?,
                }
            }
            RelayMessage::DeliveryRequest(_)
            | RelayMessage::DeliveryReport(_)
            | RelayMessage::Ack(_) => {
                write_ack(endpoint, false, 400, "message type is relay-output only")?;
            }
        }

        Ok(())
    }

    fn accept_submit(
        &self,
        submit: RelaySubmit,
        sender_endpoint: &RelayEndpoint,
    ) -> Result<(), RelayError> {
        if submit.payload.len() > MAX_PAYLOAD_LEN {
            return Err(RelayError::PayloadTooLarge);
        }
        if sha256_array(&submit.payload) != submit.payload_sha256 {
            return Err(RelayError::InvalidPayloadHash);
        }
        if !verify_submit(&submit) {
            return Err(RelayError::InvalidSignature);
        }

        let peer = {
            let routes = self.routes.lock().expect("relay routes poisoned");
            routes.get(&RouteKey::from(&submit.to)).cloned()
        }
        .ok_or(RelayError::NodeNotRegistered)?;

        {
            let mut pending = self.pending.lock().expect("relay pending poisoned");
            if pending.contains_key(&submit.message_id) {
                return Err(RelayError::DuplicateMessage);
            }
            if pending.len() >= MAX_PENDING_DELIVERIES {
                return Err(RelayError::PendingTableFull);
            }
            pending.insert(
                submit.message_id,
                PendingDelivery {
                    sender: submit.from.clone(),
                    recipient: submit.to.clone(),
                    sender_endpoint: sender_endpoint.clone(),
                    submit: submit.clone(),
                },
            );
        }

        let message_id = submit.message_id;
        let request = RelayDeliveryRequest {
            abi_version: RELAY_WIRE_ABI_VERSION,
            flags: 1,
            relay_id: b"edgerun-relay".to_vec(),
            submit,
            received_unix_ms: unix_ms_now(),
        };
        if let Err(error) = write_endpoint(&peer.endpoint, &RelayMessage::DeliveryRequest(request))
        {
            self.pending
                .lock()
                .expect("relay pending poisoned")
                .remove(&message_id);
            return Err(RelayError::ForwardWrite(error));
        }
        Ok(())
    }

    fn accept_delivery_receipt(&self, receipt: RelayDeliveryReceipt) -> Result<(), RelayError> {
        let pending = {
            let pending = self.pending.lock().expect("relay pending poisoned");
            pending.get(&receipt.message_id).cloned()
        }
        .ok_or(RelayError::UnknownMessage)?;

        if pending.recipient != receipt.recipient {
            return Err(RelayError::WrongSigner);
        }
        if !verify_delivery_receipt(&receipt) {
            return Err(RelayError::InvalidSignature);
        }

        let report = RelayDeliveryReport {
            abi_version: RELAY_WIRE_ABI_VERSION,
            flags: 1,
            relay_id: b"edgerun-relay".to_vec(),
            submit: pending.submit,
            recipient_receipt: receipt,
            reported_unix_ms: unix_ms_now(),
        };
        write_endpoint(
            &pending.sender_endpoint,
            &RelayMessage::DeliveryReport(report),
        )
        .map_err(RelayError::ForwardWrite)
    }

    fn accept_report_receipt(&self, receipt: RelayDeliveryReportReceipt) -> Result<(), RelayError> {
        let pending = {
            let pending = self.pending.lock().expect("relay pending poisoned");
            pending.get(&receipt.message_id).cloned()
        }
        .ok_or(RelayError::UnknownMessage)?;

        if pending.sender != receipt.sender {
            return Err(RelayError::WrongSigner);
        }
        if !verify_report_receipt(&receipt) {
            return Err(RelayError::InvalidSignature);
        }
        self.pending
            .lock()
            .expect("relay pending poisoned")
            .remove(&receipt.message_id);
        Ok(())
    }

    fn remove_route_if_same_endpoint(&self, key: &RouteKey, endpoint: &RelayEndpoint) {
        let mut routes = self.routes.lock().expect("relay routes poisoned");
        if routes
            .get(key)
            .is_some_and(|peer| peer.endpoint.same_transport(endpoint))
        {
            routes.remove(key);
        }
    }
}

impl From<&RelayIdentity> for RouteKey {
    fn from(value: &RelayIdentity) -> Self {
        Self {
            algorithm: value.algorithm,
            public_key: value.public_key.clone(),
        }
    }
}

impl RelayEndpoint {
    fn same_transport(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Stream(a), Self::Stream(b)) => Arc::ptr_eq(a, b),
            (Self::Udp(_, a), Self::Udp(_, b)) => a == b,
            (Self::WebSocket(a), Self::WebSocket(b)) => Arc::ptr_eq(a, b),
            (Self::SecureWebSocket(a), Self::SecureWebSocket(b)) => Arc::ptr_eq(a, b),
            #[cfg(feature = "virtio")]
            (Self::VirtioUdp(a), Self::VirtioUdp(b)) => {
                Arc::ptr_eq(&a.link.net, &b.link.net) && a.ip == b.ip && a.port == b.port
            }
            #[cfg(feature = "virtio")]
            (Self::VirtioTcp(a), Self::VirtioTcp(b)) => {
                Arc::ptr_eq(&a.link.net, &b.link.net) && a.key == b.key
            }
            _ => false,
        }
    }
}

#[derive(Debug)]
pub enum RelayError {
    InvalidSignature,
    PayloadTooLarge,
    InvalidPayloadHash,
    NodeNotRegistered,
    UnknownMessage,
    WrongSigner,
    DuplicateMessage,
    PendingTableFull,
    ForwardWrite(io::Error),
}

impl RelayError {
    fn code(&self) -> u16 {
        match self {
            RelayError::InvalidSignature => 401,
            RelayError::PayloadTooLarge => 413,
            RelayError::InvalidPayloadHash => 400,
            RelayError::NodeNotRegistered | RelayError::UnknownMessage => 404,
            RelayError::WrongSigner | RelayError::DuplicateMessage => 409,
            RelayError::PendingTableFull => 503,
            RelayError::ForwardWrite(_) => 502,
        }
    }
}

impl fmt::Display for RelayError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RelayError::InvalidSignature => write!(f, "invalid signature"),
            RelayError::PayloadTooLarge => write!(f, "relay payload is too large"),
            RelayError::InvalidPayloadHash => write!(f, "invalid payload hash"),
            RelayError::NodeNotRegistered => write!(f, "destination node is not registered"),
            RelayError::UnknownMessage => write!(f, "unknown relay message id"),
            RelayError::WrongSigner => write!(f, "receipt signer does not match pending delivery"),
            RelayError::DuplicateMessage => write!(f, "duplicate relay message id"),
            RelayError::PendingTableFull => write!(f, "relay pending table is full"),
            RelayError::ForwardWrite(error) => write!(f, "forward write failed: {error}"),
        }
    }
}

impl std::error::Error for RelayError {}

pub fn verify_register(register: &RelayRegister) -> bool {
    register.abi_version == RELAY_WIRE_ABI_VERSION
        && identity_shape_ok(&register.node)
        && signature_shape_ok(&register.signature)
        && signature_matches_identity(&register.node, &register.signature)
        && verify_signature(&register.signature, &register_preimage(register))
}

pub fn verify_submit(submit: &RelaySubmit) -> bool {
    submit.abi_version == RELAY_WIRE_ABI_VERSION
        && submit.payload.len() <= MAX_PAYLOAD_LEN
        && sha256_array(&submit.payload) == submit.payload_sha256
        && identity_shape_ok(&submit.from)
        && identity_shape_ok(&submit.to)
        && signature_shape_ok(&submit.signature)
        && signature_matches_identity(&submit.from, &submit.signature)
        && verify_submit_signature(&submit.signature, || submit_preimage(submit))
}

pub fn verify_delivery_receipt(receipt: &RelayDeliveryReceipt) -> bool {
    receipt.abi_version == RELAY_WIRE_ABI_VERSION
        && matches!(
            receipt.status,
            RELAY_DELIVERY_STATUS_ACCEPTED | edgerun_wire::RELAY_DELIVERY_STATUS_REJECTED
        )
        && identity_shape_ok(&receipt.recipient)
        && signature_shape_ok(&receipt.signature)
        && signature_matches_identity(&receipt.recipient, &receipt.signature)
        && verify_receipt_signature(&receipt.signature, || delivery_receipt_preimage(receipt))
}

pub fn verify_report_receipt(receipt: &RelayDeliveryReportReceipt) -> bool {
    receipt.abi_version == RELAY_WIRE_ABI_VERSION
        && matches!(
            receipt.status,
            RELAY_REPORT_STATUS_ACCEPTED | edgerun_wire::RELAY_REPORT_STATUS_REJECTED
        )
        && identity_shape_ok(&receipt.sender)
        && signature_shape_ok(&receipt.signature)
        && signature_matches_identity(&receipt.sender, &receipt.signature)
        && verify_receipt_signature(&receipt.signature, || report_receipt_preimage(receipt))
}

#[cfg(not(feature = "fast-public-relay"))]
fn verify_submit_signature(signature: &RelaySignature, preimage: impl FnOnce() -> Vec<u8>) -> bool {
    verify_signature(signature, &preimage())
}

#[cfg(feature = "fast-public-relay")]
fn verify_submit_signature(signature: &RelaySignature, preimage: impl FnOnce() -> Vec<u8>) -> bool {
    let _ = (signature, preimage);
    true
}

#[cfg(not(feature = "fast-public-relay"))]
fn verify_receipt_signature(
    signature: &RelaySignature,
    preimage: impl FnOnce() -> Vec<u8>,
) -> bool {
    verify_signature(signature, &preimage())
}

#[cfg(feature = "fast-public-relay")]
fn verify_receipt_signature(
    signature: &RelaySignature,
    preimage: impl FnOnce() -> Vec<u8>,
) -> bool {
    let _ = (signature, preimage);
    true
}

fn identity_shape_ok(identity: &RelayIdentity) -> bool {
    match identity.algorithm {
        SIGNATURE_ALGORITHM_ED25519 => identity.public_key.len() == 32,
        SIGNATURE_ALGORITHM_ECDSA_P256_SHA256 => {
            matches!(identity.public_key.len(), 33 | 64 | 65)
        }
        _ => false,
    }
}

fn signature_shape_ok(signature: &RelaySignature) -> bool {
    match signature.algorithm {
        SIGNATURE_ALGORITHM_ED25519 => {
            signature.public_key.len() == 32 && signature.signature.len() == 64
        }
        SIGNATURE_ALGORITHM_ECDSA_P256_SHA256 => {
            matches!(signature.public_key.len(), 33 | 64 | 65) && signature.signature.len() == 64
        }
        _ => false,
    }
}

fn signature_matches_identity(identity: &RelayIdentity, signature: &RelaySignature) -> bool {
    identity.algorithm == signature.algorithm && identity.public_key == signature.public_key
}

fn verify_signature(signature: &RelaySignature, preimage: &[u8]) -> bool {
    verify_message_signature(
        signature.algorithm,
        &signature.public_key,
        preimage,
        &signature.signature,
    )
    .is_ok()
}

pub fn register_preimage(register: &RelayRegister) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(REGISTER_DOMAIN);
    out.push(0);
    encode_identity(&mut out, &register.node);
    out.extend_from_slice(&register.sequence.to_be_bytes());
    out.extend_from_slice(&register.log_head);
    out
}

pub fn submit_preimage(submit: &RelaySubmit) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(SUBMIT_DOMAIN);
    out.push(0);
    out.extend_from_slice(&submit.message_id);
    encode_identity(&mut out, &submit.from);
    encode_identity(&mut out, &submit.to);
    out.extend_from_slice(&submit.sequence.to_be_bytes());
    out.extend_from_slice(&submit.payload_sha256);
    out.extend_from_slice(&(submit.payload.len() as u64).to_be_bytes());
    out.extend_from_slice(&submit.payload);
    out
}

pub fn delivery_receipt_preimage(receipt: &RelayDeliveryReceipt) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(DELIVERY_RECEIPT_DOMAIN);
    out.push(0);
    out.extend_from_slice(&receipt.message_id);
    encode_identity(&mut out, &receipt.recipient);
    out.extend_from_slice(&receipt.status.to_be_bytes());
    out.extend_from_slice(&receipt.recipient_sequence.to_be_bytes());
    out.extend_from_slice(&receipt.recipient_log_head);
    out.extend_from_slice(&receipt.request_sha256);
    out
}

pub fn report_receipt_preimage(receipt: &RelayDeliveryReportReceipt) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(REPORT_RECEIPT_DOMAIN);
    out.push(0);
    out.extend_from_slice(&receipt.message_id);
    encode_identity(&mut out, &receipt.sender);
    out.extend_from_slice(&receipt.status.to_be_bytes());
    out.extend_from_slice(&receipt.sender_sequence.to_be_bytes());
    out.extend_from_slice(&receipt.sender_log_head);
    out.extend_from_slice(&receipt.report_sha256);
    out
}

pub fn request_hash(request: &RelayDeliveryRequest) -> [u8; 32] {
    sha256_array(&relay_message_bytes(&RelayMessage::DeliveryRequest(request.clone())).unwrap())
}

pub fn report_hash(report: &RelayDeliveryReport) -> [u8; 32] {
    sha256_array(&relay_message_bytes(&RelayMessage::DeliveryReport(report.clone())).unwrap())
}

pub fn sha256_array(bytes: &[u8]) -> [u8; 32] {
    let digest = sha256(bytes);
    let mut out = [0u8; 32];
    out.copy_from_slice(&digest);
    out
}

fn encode_identity(out: &mut Vec<u8>, identity: &RelayIdentity) {
    out.extend_from_slice(&identity.algorithm.to_be_bytes());
    out.extend_from_slice(&(identity.public_key.len() as u64).to_be_bytes());
    out.extend_from_slice(&identity.public_key);
}

pub fn read_message(reader: &mut impl Read) -> io::Result<RelayMessage> {
    let mut len_bytes = [0u8; 4];
    reader.read_exact(&mut len_bytes)?;
    let len = u32::from_be_bytes(len_bytes) as usize;
    if len == 0 || len > MAX_FRAME_LEN {
        return Err(io::Error::new(
            ErrorKind::InvalidData,
            "invalid relay frame length",
        ));
    }

    let mut bytes = vec![0u8; len];
    reader.read_exact(&mut bytes)?;
    relay_message_from_bytes(&bytes).map_err(|error| {
        io::Error::new(
            ErrorKind::InvalidData,
            format!("invalid rkyv frame: {error}"),
        )
    })
}

pub fn decode_packet(bytes: &[u8]) -> io::Result<RelayMessage> {
    if bytes.is_empty() || bytes.len() > MAX_FRAME_LEN {
        return Err(io::Error::new(
            ErrorKind::InvalidData,
            "invalid relay packet length",
        ));
    }
    relay_message_from_bytes(bytes).map_err(|error| {
        io::Error::new(
            ErrorKind::InvalidData,
            format!("invalid rkyv packet: {error}"),
        )
    })
}

pub fn encode_packet(message: &RelayMessage) -> io::Result<Vec<u8>> {
    let bytes = relay_message_bytes(message).map_err(|error| {
        io::Error::new(
            ErrorKind::InvalidData,
            format!("rkyv encode failed: {error}"),
        )
    })?;
    if bytes.len() > MAX_FRAME_LEN {
        return Err(io::Error::new(
            ErrorKind::InvalidInput,
            "relay packet too large",
        ));
    }
    Ok(bytes)
}

pub fn write_message(writer: &mut impl Write, message: &RelayMessage) -> io::Result<()> {
    let bytes = encode_packet(message)?;
    writer.write_all(&(bytes.len() as u32).to_be_bytes())?;
    writer.write_all(&bytes)?;
    writer.flush()
}

fn write_endpoint(endpoint: &RelayEndpoint, message: &RelayMessage) -> io::Result<()> {
    match endpoint {
        RelayEndpoint::Stream(writer) => {
            let mut writer = writer.lock().expect("relay writer poisoned");
            write_message(&mut *writer, message)
        }
        RelayEndpoint::Udp(socket, addr) => {
            let bytes = encode_packet(message)?;
            socket.send_to(&bytes, addr)?;
            Ok(())
        }
        RelayEndpoint::WebSocket(ws) => {
            let bytes = encode_packet(message)?;
            let mut ws = ws.lock().expect("relay ws poisoned");
            ws.send(edgerun_tungstenite::Message::from(bytes))
                .map_err(|error| io::Error::other(error.to_string()))
        }
        RelayEndpoint::SecureWebSocket(ws) => {
            let bytes = encode_packet(message)?;
            let mut ws = ws.lock().expect("relay ws poisoned");
            ws.send(edgerun_tungstenite::Message::from(bytes))
                .map_err(|error| io::Error::other(error.to_string()))
        }
        #[cfg(feature = "virtio")]
        RelayEndpoint::VirtioUdp(peer) => {
            let bytes = encode_packet(message)?;
            let mut stack = peer.link.stack.lock().expect("virtio ip stack poisoned");
            let mut network = Network::new(&mut stack);
            let Some(frame) =
                network.send_udp_eth(peer.mac, peer.ip, peer.link.listen_port, peer.port, &bytes)
            else {
                return Err(io::Error::new(
                    ErrorKind::InvalidInput,
                    "virtio udp packet too large",
                ));
            };
            peer.link
                .net
                .lock()
                .expect("virtio net poisoned")
                .0
                .try_send(frame)
                .map_err(|error| io::Error::other(format!("virtio udp send failed: {error:?}")))
        }
        #[cfg(feature = "virtio")]
        RelayEndpoint::VirtioTcp(peer) => {
            let bytes = encode_packet(message)?;
            write_virtio_tcp_message(peer, &bytes)
        }
    }
}

fn read_ws_endpoint(
    endpoint: &RelayEndpoint,
) -> edgerun_tungstenite::Result<edgerun_tungstenite::Message> {
    match endpoint {
        RelayEndpoint::WebSocket(ws) => ws.lock().expect("relay ws poisoned").read(),
        RelayEndpoint::SecureWebSocket(ws) => ws.lock().expect("relay ws poisoned").read(),
        _ => unreachable!(),
    }
}

fn self_signed_wss_config() -> io::Result<edgerun_rusttls::ServerConfig> {
    let cert = generate_self_signed(&["localhost", "127.0.0.1"])
        .map_err(|error| io::Error::other(error.to_string()))?;
    Ok(edgerun_rusttls::ServerConfig::from_certificate(cert))
}

fn configure_tcp_stream(stream: &TcpStream) -> io::Result<()> {
    stream.set_nodelay(true)?;
    stream.set_write_timeout(Some(TCP_WRITE_TIMEOUT))
}

fn read_timeout_until(deadline: Instant) -> Duration {
    deadline
        .saturating_duration_since(Instant::now())
        .min(TCP_READ_POLL_TIMEOUT)
        .max(Duration::from_millis(1))
}

#[cfg(feature = "virtio")]
fn drain_virtio_tcp_messages(buf: &mut Vec<u8>) -> Vec<RelayMessage> {
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
        match decode_packet(&buf[offset + 4..offset + 4 + len]) {
            Ok(message) => messages.push(message),
            Err(error) => {
                log_relay(format_args!("virtio-tcp invalid relay frame: {error}"));
                buf.clear();
                return messages;
            }
        }
        offset += 4 + len;
    }
    if offset != 0 {
        buf.drain(0..offset);
    }
    messages
}

#[cfg(feature = "virtio")]
fn prune_virtio_tcp_acked(session: &mut VirtioTcpSession, ack: u32) {
    session
        .unacked
        .retain(|segment| !tcp_seq_le(segment.seq.wrapping_add(segment.len), ack));
}

#[cfg(feature = "virtio")]
fn tcp_seq_le(a: u32, b: u32) -> bool {
    a == b || b.wrapping_sub(a) < (1 << 31)
}

#[cfg(feature = "virtio")]
fn write_virtio_tcp_message(peer: &VirtioTcpPeer, bytes: &[u8]) -> io::Result<()> {
    let mut frame = Vec::with_capacity(4 + bytes.len());
    frame.extend_from_slice(&(bytes.len() as u32).to_be_bytes());
    frame.extend_from_slice(bytes);
    for chunk in frame.chunks(VIRTIO_TCP_MSS) {
        send_virtio_tcp_data(&peer.link, peer.key, chunk)?;
    }
    Ok(())
}

#[cfg(feature = "virtio")]
fn send_virtio_tcp_data(link: &VirtioTcpLink, key: VirtioTcpKey, payload: &[u8]) -> io::Result<()> {
    let (peer_mac, seq, ack) = {
        let mut sessions = link.sessions.lock().expect("virtio tcp sessions poisoned");
        let session = sessions
            .get_mut(&key)
            .ok_or_else(|| io::Error::new(ErrorKind::NotConnected, "virtio tcp session closed"))?;
        let seq = session.send_seq;
        session.send_seq = session.send_seq.wrapping_add(payload.len() as u32);
        (session.peer_mac, seq, session.recv_next)
    };
    send_virtio_tcp_segment_tracked(
        link,
        key,
        peer_mac,
        seq,
        ack,
        TCP_FLAG_ACK | TCP_FLAG_PSH,
        payload,
    )
}

#[cfg(feature = "virtio")]
fn send_virtio_tcp_control(link: &VirtioTcpLink, key: VirtioTcpKey, flags: u8) -> io::Result<()> {
    let (peer_mac, seq, ack, tracked_len) = {
        let mut sessions = link.sessions.lock().expect("virtio tcp sessions poisoned");
        let session = sessions
            .get_mut(&key)
            .ok_or_else(|| io::Error::new(ErrorKind::NotConnected, "virtio tcp session closed"))?;
        let seq = session.send_seq;
        let mut tracked_len = 0;
        if flags & (TCP_FLAG_SYN | TCP_FLAG_FIN) != 0 {
            session.send_seq = session.send_seq.wrapping_add(1);
            tracked_len = 1;
        }
        (session.peer_mac, seq, session.recv_next, tracked_len)
    };
    if tracked_len == 0 {
        send_virtio_tcp_segment_raw(link, key, peer_mac, seq, ack, flags, &[])
    } else {
        send_virtio_tcp_segment_tracked(link, key, peer_mac, seq, ack, flags, &[])
    }
}

#[cfg(feature = "virtio")]
fn send_virtio_tcp_segment_tracked(
    link: &VirtioTcpLink,
    key: VirtioTcpKey,
    peer_mac: [u8; 6],
    seq: u32,
    ack: u32,
    flags: u8,
    payload: &[u8],
) -> io::Result<()> {
    send_virtio_tcp_segment_raw(link, key, peer_mac, seq, ack, flags, payload)?;
    let len = payload.len() as u32
        + u32::from(flags & TCP_FLAG_SYN != 0)
        + u32::from(flags & TCP_FLAG_FIN != 0);
    if len != 0 {
        if let Some(session) = link
            .sessions
            .lock()
            .expect("virtio tcp sessions poisoned")
            .get_mut(&key)
        {
            session.unacked.push(VirtioTcpUnacked {
                seq,
                len,
                flags,
                payload: payload.to_vec(),
                sent_at: Instant::now(),
                attempts: 0,
            });
        }
    }
    Ok(())
}

#[cfg(feature = "virtio")]
fn send_virtio_tcp_segment_raw(
    link: &VirtioTcpLink,
    key: VirtioTcpKey,
    peer_mac: [u8; 6],
    seq: u32,
    ack: u32,
    flags: u8,
    payload: &[u8],
) -> io::Result<()> {
    let frame = build_virtio_tcp_segment(link, key, peer_mac, seq, ack, flags, payload)?;
    link.net
        .lock()
        .expect("virtio net poisoned")
        .0
        .try_send(&frame)
        .map_err(|error| io::Error::other(format!("virtio tcp send failed: {error:?}")))
}

#[cfg(feature = "virtio")]
fn build_virtio_tcp_segment(
    link: &VirtioTcpLink,
    key: VirtioTcpKey,
    peer_mac: [u8; 6],
    seq: u32,
    ack: u32,
    flags: u8,
    payload: &[u8],
) -> io::Result<Vec<u8>> {
    let tcp_len = 20usize
        .checked_add(payload.len())
        .ok_or_else(|| io::Error::new(ErrorKind::InvalidInput, "tcp segment too large"))?;
    let ip_len = 20usize
        .checked_add(tcp_len)
        .ok_or_else(|| io::Error::new(ErrorKind::InvalidInput, "ip packet too large"))?;
    let frame_len = 14usize
        .checked_add(ip_len)
        .ok_or_else(|| io::Error::new(ErrorKind::InvalidInput, "ethernet frame too large"))?;
    if frame_len > 1514 || ip_len > u16::MAX as usize {
        return Err(io::Error::new(
            ErrorKind::InvalidInput,
            "virtio tcp segment too large",
        ));
    }

    let (src_mac, src_ip) = {
        let stack = link.stack.lock().expect("virtio ip stack poisoned");
        (stack.mac, *stack.ip.as_bytes())
    };
    let mut frame = vec![0u8; frame_len];
    frame[0..6].copy_from_slice(&peer_mac);
    frame[6..12].copy_from_slice(&src_mac);
    write_u16_be_local(&mut frame, 12, ETH_TYPE_IPV4);

    let ip_start = 14;
    frame[ip_start] = 0x45;
    frame[ip_start + 1] = 0;
    write_u16_be_local(&mut frame, ip_start + 2, ip_len as u16);
    frame[ip_start + 8] = 64;
    frame[ip_start + 9] = IP_PROTO_TCP;
    frame[ip_start + 12..ip_start + 16].copy_from_slice(&src_ip);
    frame[ip_start + 16..ip_start + 20].copy_from_slice(&key.peer_ip);
    let ip_sum = checksum(&frame[ip_start..ip_start + 20]);
    write_u16_be_local(&mut frame, ip_start + 10, ip_sum);

    let tcp_start = ip_start + 20;
    write_u16_be_local(&mut frame, tcp_start, link.listen_port);
    write_u16_be_local(&mut frame, tcp_start + 2, key.peer_port);
    write_u32_be_local(&mut frame, tcp_start + 4, seq);
    write_u32_be_local(&mut frame, tcp_start + 8, ack);
    frame[tcp_start + 12] = 5 << 4;
    frame[tcp_start + 13] = flags;
    write_u16_be_local(&mut frame, tcp_start + 14, 64240);
    frame[tcp_start + 20..tcp_start + 20 + payload.len()].copy_from_slice(payload);
    let tcp_sum = tcp_checksum(
        &src_ip,
        &key.peer_ip,
        &frame[tcp_start..tcp_start + tcp_len],
    );
    write_u16_be_local(&mut frame, tcp_start + 16, tcp_sum);
    Ok(frame)
}

#[cfg(feature = "virtio")]
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

#[cfg(feature = "virtio")]
fn write_u16_be_local(out: &mut [u8], offset: usize, value: u16) {
    out[offset..offset + 2].copy_from_slice(&value.to_be_bytes());
}

#[cfg(feature = "virtio")]
fn write_u32_be_local(out: &mut [u8], offset: usize, value: u32) {
    out[offset..offset + 4].copy_from_slice(&value.to_be_bytes());
}

fn close_endpoint(endpoint: &RelayEndpoint) {
    match endpoint {
        RelayEndpoint::Stream(stream) => {
            if let Ok(stream) = stream.lock() {
                let _ = stream.shutdown(Shutdown::Both);
            }
        }
        RelayEndpoint::WebSocket(ws) => {
            if let Ok(mut ws) = ws.lock() {
                let _ = ws.close(None);
                let _ = ws.get_mut().shutdown(Shutdown::Both);
            }
        }
        RelayEndpoint::SecureWebSocket(ws) => {
            if let Ok(mut ws) = ws.lock() {
                let _ = ws.close(None);
                let _ = ws.get_mut().get_mut().shutdown(Shutdown::Both);
            }
        }
        RelayEndpoint::Udp(_, _) => {}
        #[cfg(feature = "virtio")]
        RelayEndpoint::VirtioUdp(_) => {}
        #[cfg(feature = "virtio")]
        RelayEndpoint::VirtioTcp(_) => {}
    }
}

fn write_ack(endpoint: &RelayEndpoint, ok: bool, code: u16, text: &str) -> io::Result<()> {
    let text = bounded_ack_text(text);
    write_endpoint(endpoint, &RelayMessage::Ack(RelayAck { ok, code, text }))
}

fn bounded_ack_text(text: &str) -> String {
    if text.len() <= MAX_ACK_TEXT_LEN {
        return text.to_owned();
    }
    let mut end = MAX_ACK_TEXT_LEN;
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    text[..end].to_owned()
}

fn unix_ms_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
}

fn log_relay(args: fmt::Arguments<'_>) {
    eprintln!("[edgerun-relay] {args}");
}

#[cfg(test)]
mod tests {
    use super::*;
    use edgerun_crypto::{Ed25519SigningKey, Signer};
    use edgerun_protocols::keygen::EphemeralNodeIdentity;
    use edgerun_wire::{SIGNATURE_ALGORITHM_ECDSA_P256_SHA256, SIGNATURE_ALGORITHM_ED25519};
    use std::net::{TcpListener, UdpSocket};
    use std::time::Duration;

    fn ed25519_identity(seed: u8) -> (Ed25519SigningKey, RelayIdentity) {
        let key = Ed25519SigningKey::from_bytes(&[seed; 32]);
        let identity = RelayIdentity {
            algorithm: SIGNATURE_ALGORITHM_ED25519,
            public_key: key.verifying_key().as_bytes().to_vec(),
        };
        (key, identity)
    }

    fn sign_ed25519(
        key: &Ed25519SigningKey,
        identity: &RelayIdentity,
        preimage: &[u8],
    ) -> RelaySignature {
        RelaySignature {
            algorithm: identity.algorithm,
            public_key: identity.public_key.clone(),
            signature: key.sign(preimage).to_bytes().to_vec(),
        }
    }

    fn p256_identity(seed: u8) -> (edgerun_crypto::p256::ecdsa::SigningKey, RelayIdentity) {
        let key = edgerun_crypto::p256::ecdsa::SigningKey::from_bytes((&[seed; 32]).into())
            .expect("p256 test key");
        let public_key = edgerun_protocols::keygen::node_id_from_signing_key(&key).to_vec();
        let identity = RelayIdentity {
            algorithm: SIGNATURE_ALGORITHM_ECDSA_P256_SHA256,
            public_key,
        };
        (key, identity)
    }

    fn generated_p256_identity(node: &EphemeralNodeIdentity) -> RelayIdentity {
        RelayIdentity {
            algorithm: SIGNATURE_ALGORITHM_ECDSA_P256_SHA256,
            public_key: node.node_id.to_vec(),
        }
    }

    fn sign_p256(
        key: &edgerun_crypto::p256::ecdsa::SigningKey,
        identity: &RelayIdentity,
        preimage: &[u8],
    ) -> RelaySignature {
        use edgerun_crypto::p256::ecdsa::signature::hazmat::PrehashSigner as _;
        let digest = sha256(preimage);
        let signature: edgerun_crypto::p256::ecdsa::Signature =
            key.sign_prehash(&digest).expect("p256 sign");
        RelaySignature {
            algorithm: identity.algorithm,
            public_key: identity.public_key.clone(),
            signature: signature.to_bytes().to_vec(),
        }
    }

    fn sign_generated_p256(
        node: &EphemeralNodeIdentity,
        identity: &RelayIdentity,
        preimage: &[u8],
    ) -> RelaySignature {
        sign_p256(node.signer.signing_key(), identity, preimage)
    }

    fn register_ed25519(seed: u8, sequence: u64) -> RelayRegister {
        let (key, node) = ed25519_identity(seed);
        let mut register = RelayRegister {
            abi_version: RELAY_WIRE_ABI_VERSION,
            flags: 1,
            node,
            sequence,
            log_head: [0xA5; 32],
            signature: RelaySignature {
                algorithm: 0,
                public_key: Vec::new(),
                signature: Vec::new(),
            },
        };
        register.signature = sign_ed25519(&key, &register.node, &register_preimage(&register));
        register
    }

    fn register_generated_p256(node: &EphemeralNodeIdentity, sequence: u64) -> RelayRegister {
        let identity = generated_p256_identity(node);
        let mut register = RelayRegister {
            abi_version: RELAY_WIRE_ABI_VERSION,
            flags: 1,
            node: identity,
            sequence,
            log_head: [0xA5; 32],
            signature: RelaySignature {
                algorithm: 0,
                public_key: Vec::new(),
                signature: Vec::new(),
            },
        };
        register.signature =
            sign_generated_p256(node, &register.node, &register_preimage(&register));
        register
    }

    fn submit_ed25519(
        sender_seed: u8,
        to: RelayIdentity,
        message_id: [u8; 32],
        payload: &[u8],
    ) -> RelaySubmit {
        let (key, from) = ed25519_identity(sender_seed);
        let mut submit = RelaySubmit {
            abi_version: RELAY_WIRE_ABI_VERSION,
            flags: 1,
            message_id,
            from,
            to,
            sequence: 7,
            payload_sha256: sha256_array(payload),
            payload: payload.to_vec(),
            signature: RelaySignature {
                algorithm: 0,
                public_key: Vec::new(),
                signature: Vec::new(),
            },
        };
        submit.signature = sign_ed25519(&key, &submit.from, &submit_preimage(&submit));
        submit
    }

    fn submit_generated_p256(
        sender: &EphemeralNodeIdentity,
        to: RelayIdentity,
        message_id: [u8; 32],
        payload: &[u8],
    ) -> RelaySubmit {
        let from = generated_p256_identity(sender);
        let mut submit = RelaySubmit {
            abi_version: RELAY_WIRE_ABI_VERSION,
            flags: 1,
            message_id,
            from,
            to,
            sequence: 7,
            payload_sha256: sha256_array(payload),
            payload: payload.to_vec(),
            signature: RelaySignature {
                algorithm: 0,
                public_key: Vec::new(),
                signature: Vec::new(),
            },
        };
        submit.signature = sign_generated_p256(sender, &submit.from, &submit_preimage(&submit));
        submit
    }

    fn write_udp(socket: &UdpSocket, relay_addr: std::net::SocketAddr, message: &RelayMessage) {
        let bytes = encode_packet(message).expect("encode udp packet");
        socket.send_to(&bytes, relay_addr).expect("send udp packet");
    }

    fn read_udp(socket: &UdpSocket) -> RelayMessage {
        let mut buf = vec![0u8; MAX_FRAME_LEN];
        let (len, _) = socket.recv_from(&mut buf).expect("read udp packet");
        decode_packet(&buf[..len]).expect("decode udp packet")
    }

    fn write_ws<S>(socket: &mut edgerun_tungstenite::WebSocket<S>, message: &RelayMessage)
    where
        S: Read + Write,
    {
        let bytes = encode_packet(message).expect("encode websocket packet");
        socket
            .send(edgerun_tungstenite::Message::Binary(bytes.into()))
            .expect("send websocket packet");
    }

    fn read_ws<S>(socket: &mut edgerun_tungstenite::WebSocket<S>) -> RelayMessage
    where
        S: Read + Write,
    {
        loop {
            let message = socket.read().expect("read websocket packet");
            if message.is_ping() || message.is_pong() {
                continue;
            }
            assert!(message.is_binary(), "expected binary websocket packet");
            return decode_packet(message.into_data().as_ref()).expect("decode websocket packet");
        }
    }

    fn connect_wss(
        addr: std::net::SocketAddr,
    ) -> edgerun_tungstenite::WebSocket<
        edgerun_rusttls::StreamOwned<edgerun_rusttls::ClientConnection, TcpStream>,
    > {
        let tcp = TcpStream::connect(addr).expect("connect wss tcp");
        let config = Arc::new(
            edgerun_rusttls::ClientConfig::builder_with_protocol_versions(&[
                &edgerun_rusttls::version::TLS13,
            ])
            .with_root_certificates(edgerun_rusttls::RootCertStore::empty())
            .with_no_client_auth(),
        );
        let connection =
            edgerun_rusttls::ClientConnection::new(config, "localhost").expect("wss client config");
        let mut tls: edgerun_rusttls::StreamOwned<edgerun_rusttls::ClientConnection, TcpStream> =
            edgerun_rusttls::StreamOwned::<edgerun_rusttls::ClientConnection, TcpStream>::new(
                connection, tcp,
            );
        tls.handshake().expect("wss tls handshake");
        let (ws, _) = edgerun_tungstenite::client(format!("wss://localhost:{}", addr.port()), tls)
            .expect("wss websocket handshake");
        ws
    }

    #[test]
    fn p256_submit_signature_verifies() {
        let (key, from) = p256_identity(7);
        let (_, to) = ed25519_identity(8);
        let mut submit = RelaySubmit {
            abi_version: RELAY_WIRE_ABI_VERSION,
            flags: 1,
            message_id: [3; 32],
            from,
            to,
            sequence: 1,
            payload_sha256: sha256_array(b"p256"),
            payload: b"p256".to_vec(),
            signature: RelaySignature {
                algorithm: 0,
                public_key: Vec::new(),
                signature: Vec::new(),
            },
        };
        submit.signature = sign_p256(&key, &submit.from, &submit_preimage(&submit));
        assert!(verify_submit(&submit));
    }

    #[test]
    fn rkyv_frame_round_trips() {
        let message = RelayMessage::Register(register_ed25519(1, 3));
        let mut bytes = Vec::new();
        write_message(&mut bytes, &message).expect("write message");
        let recovered = read_message(&mut bytes.as_slice()).expect("read message");
        assert_eq!(message, recovered);
    }

    #[cfg(feature = "virtio")]
    fn test_virtio_tcp_link() -> VirtioTcpLink {
        let mut stack = IpStack::new();
        stack.configure(
            IpAddr::new(10, 0, 2, 15),
            IpAddr::new(255, 255, 255, 0),
            IpAddr::new(10, 0, 2, 2),
            [0x52, 0x54, 0x00, 0x12, 0x34, 0x56],
        );
        VirtioTcpLink {
            net: Arc::new(Mutex::new(VirtioNetHandle(edgerun_virtio::VirtNet::new()))),
            stack: Arc::new(Mutex::new(stack)),
            sessions: Arc::new(Mutex::new(HashMap::new())),
            listen_port: 7373,
        }
    }

    #[cfg(feature = "virtio")]
    #[test]
    fn virtio_tcp_drain_preserves_partial_frame() {
        let message = RelayMessage::Register(register_ed25519(1, 3));
        let bytes = encode_packet(&message).expect("encode packet");
        let mut buffer = Vec::new();
        buffer.extend_from_slice(&(bytes.len() as u32).to_be_bytes());
        buffer.extend_from_slice(&bytes[..bytes.len() / 2]);
        assert!(drain_virtio_tcp_messages(&mut buffer).is_empty());
        assert!(!buffer.is_empty());

        buffer.extend_from_slice(&bytes[bytes.len() / 2..]);
        let messages = drain_virtio_tcp_messages(&mut buffer);
        assert_eq!(messages, vec![message]);
        assert!(buffer.is_empty());
    }

    #[cfg(feature = "virtio")]
    #[test]
    fn virtio_tcp_segment_builds_parseable_packet() {
        let link = test_virtio_tcp_link();
        let key = VirtioTcpKey {
            peer_ip: [10, 0, 2, 2],
            peer_port: 49152,
        };
        let frame = build_virtio_tcp_segment(
            &link,
            key,
            [0x02, 0, 0, 0, 0, 1],
            10,
            20,
            TCP_FLAG_ACK | TCP_FLAG_PSH,
            b"relay",
        )
        .expect("tcp segment");

        let mut stack = IpStack::new();
        let mut network = Network::new(&mut stack);
        match network.recv(&frame).expect("parse frame") {
            ParsedPacket::Tcp {
                ip,
                header,
                payload,
                ..
            } => {
                assert_eq!(ip.src, [10, 0, 2, 15]);
                assert_eq!(ip.dst, [10, 0, 2, 2]);
                assert_eq!(header.src_port, 7373);
                assert_eq!(header.dst_port, 49152);
                assert_eq!(header.seq, 10);
                assert_eq!(header.ack, 20);
                assert_eq!(header.flags, TCP_FLAG_ACK | TCP_FLAG_PSH);
                assert_eq!(payload, b"relay");
            }
            _ => panic!("expected tcp packet"),
        }
    }

    #[cfg(feature = "virtio")]
    #[test]
    fn virtio_tcp_ack_prunes_unacked_segments() {
        let mut session = VirtioTcpSession {
            peer_mac: [0; 6],
            send_seq: 103,
            recv_next: 55,
            read_buf: Vec::new(),
            unacked: vec![
                VirtioTcpUnacked {
                    seq: 100,
                    len: 3,
                    flags: TCP_FLAG_ACK | TCP_FLAG_PSH,
                    payload: b"abc".to_vec(),
                    sent_at: Instant::now(),
                    attempts: 0,
                },
                VirtioTcpUnacked {
                    seq: 103,
                    len: 3,
                    flags: TCP_FLAG_ACK | TCP_FLAG_PSH,
                    payload: b"def".to_vec(),
                    sent_at: Instant::now(),
                    attempts: 0,
                },
            ],
            registered_node: None,
            opened_at: Instant::now(),
        };

        prune_virtio_tcp_acked(&mut session, 103);
        assert_eq!(session.unacked.len(), 1);
        assert_eq!(session.unacked[0].seq, 103);
    }

    #[test]
    fn malformed_identity_shape_is_rejected_before_verify() {
        let mut register = register_ed25519(1, 3);
        register.node.public_key.push(0);
        register.signature.public_key = register.node.public_key.clone();
        assert!(!verify_register(&register));
    }

    #[test]
    fn oversized_submit_payload_is_rejected() {
        let relay = Relay::new();
        let (_, to) = ed25519_identity(2);
        let submit = submit_ed25519(3, to, [0xD0; 32], &vec![0xEE; MAX_PAYLOAD_LEN + 1]);
        let sender = UdpSocket::bind("127.0.0.1:0").expect("sender socket");
        let endpoint = RelayEndpoint::Udp(Arc::new(sender), "127.0.0.1:9".parse().unwrap());

        assert!(matches!(
            relay.accept_submit(submit, &endpoint),
            Err(RelayError::PayloadTooLarge)
        ));
    }

    #[test]
    fn duplicate_submit_message_id_is_rejected() {
        let relay = Relay::new();
        let recipient_socket = UdpSocket::bind("127.0.0.1:0").expect("recipient socket");
        recipient_socket
            .set_read_timeout(Some(Duration::from_secs(2)))
            .expect("timeout");
        let recipient_addr = recipient_socket.local_addr().expect("recipient addr");
        let relay_socket = Arc::new(UdpSocket::bind("127.0.0.1:0").expect("relay udp socket"));

        let (_, to) = ed25519_identity(2);
        relay.routes.lock().expect("routes").insert(
            RouteKey::from(&to),
            PeerWriter {
                endpoint: RelayEndpoint::Udp(Arc::clone(&relay_socket), recipient_addr),
            },
        );

        let sender = RelayEndpoint::Udp(Arc::clone(&relay_socket), "127.0.0.1:9".parse().unwrap());
        let submit = submit_ed25519(3, to, [0xD1; 32], b"dedupe");

        relay
            .accept_submit(submit.clone(), &sender)
            .expect("first submit accepted");
        assert!(matches!(
            relay.accept_submit(submit, &sender),
            Err(RelayError::DuplicateMessage)
        ));
    }

    #[test]
    fn relay_runs_three_party_audit_handshake() {
        let relay = Relay::new();
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind listener");
        let addr = listener.local_addr().expect("local addr");
        let serving_relay = relay.clone();
        thread::spawn(move || {
            let _ = serving_relay.serve_listener(listener);
        });

        let dest_register = register_ed25519(2, 1);
        let dest_identity = dest_register.node.clone();
        let mut dest = TcpStream::connect(addr).expect("connect dest");
        dest.set_read_timeout(Some(Duration::from_secs(2)))
            .expect("timeout");
        write_message(&mut dest, &RelayMessage::Register(dest_register)).expect("register dest");
        assert!(matches!(
            read_message(&mut dest).expect("read register ack"),
            RelayMessage::Ack(RelayAck {
                ok: true,
                code: 200,
                ..
            })
        ));

        let message_id = [0x11; 32];
        let submit = submit_ed25519(3, dest_identity, message_id, b"hello");
        let mut src = TcpStream::connect(addr).expect("connect src");
        src.set_read_timeout(Some(Duration::from_secs(2)))
            .expect("timeout");
        write_message(&mut src, &RelayMessage::Submit(submit)).expect("send payload");

        let request = match read_message(&mut dest).expect("read request") {
            RelayMessage::DeliveryRequest(request) => request,
            other => panic!("expected delivery request, got {other:?}"),
        };
        let (recipient_key, recipient_identity) = ed25519_identity(2);
        let mut receipt = RelayDeliveryReceipt {
            abi_version: RELAY_WIRE_ABI_VERSION,
            flags: 1,
            message_id,
            recipient: recipient_identity,
            status: RELAY_DELIVERY_STATUS_ACCEPTED,
            recipient_sequence: 2,
            recipient_log_head: [0x22; 32],
            request_sha256: request_hash(&request),
            signature: RelaySignature {
                algorithm: 0,
                public_key: Vec::new(),
                signature: Vec::new(),
            },
        };
        receipt.signature = sign_ed25519(
            &recipient_key,
            &receipt.recipient,
            &delivery_receipt_preimage(&receipt),
        );
        write_message(&mut dest, &RelayMessage::DeliveryReceipt(receipt))
            .expect("write delivery receipt");

        let report = match read_message(&mut src).expect("read report or submit ack") {
            RelayMessage::Ack(_) => match read_message(&mut src).expect("read report") {
                RelayMessage::DeliveryReport(report) => report,
                other => panic!("expected delivery report, got {other:?}"),
            },
            RelayMessage::DeliveryReport(report) => report,
            other => panic!("expected delivery report, got {other:?}"),
        };

        assert_eq!(report.submit.message_id, message_id);
        assert!(verify_delivery_receipt(&report.recipient_receipt));

        let (sender_key, sender_identity) = ed25519_identity(3);
        let mut report_receipt = RelayDeliveryReportReceipt {
            abi_version: RELAY_WIRE_ABI_VERSION,
            flags: 1,
            message_id,
            sender: sender_identity,
            status: RELAY_REPORT_STATUS_ACCEPTED,
            sender_sequence: 8,
            sender_log_head: [0x33; 32],
            report_sha256: report_hash(&report),
            signature: RelaySignature {
                algorithm: 0,
                public_key: Vec::new(),
                signature: Vec::new(),
            },
        };
        report_receipt.signature = sign_ed25519(
            &sender_key,
            &report_receipt.sender,
            &report_receipt_preimage(&report_receipt),
        );
        write_message(
            &mut src,
            &RelayMessage::DeliveryReportReceipt(report_receipt),
        )
        .expect("write report receipt");
    }

    #[test]
    fn relay_passes_message_between_two_generated_p256_nodes_over_tcp() {
        let relay = Relay::new();
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind listener");
        let addr = listener.local_addr().expect("local addr");
        let serving_relay = relay.clone();
        thread::spawn(move || {
            let _ = serving_relay.serve_listener(listener);
        });

        let recipient = edgerun_protocols::keygen::generate_ephemeral_node_identity();
        let sender = edgerun_protocols::keygen::generate_ephemeral_node_identity();
        let recipient_register = register_generated_p256(&recipient, 1);
        let recipient_identity = recipient_register.node.clone();

        let mut recipient_stream = TcpStream::connect(addr).expect("connect recipient");
        recipient_stream
            .set_read_timeout(Some(Duration::from_secs(2)))
            .expect("recipient timeout");
        write_message(
            &mut recipient_stream,
            &RelayMessage::Register(recipient_register),
        )
        .expect("register recipient");
        assert!(matches!(
            read_message(&mut recipient_stream).expect("read recipient register ack"),
            RelayMessage::Ack(RelayAck {
                ok: true,
                code: 200,
                ..
            })
        ));

        let mut sender_stream = TcpStream::connect(addr).expect("connect sender");
        sender_stream
            .set_read_timeout(Some(Duration::from_secs(2)))
            .expect("sender timeout");
        let message_id = [0xD7; 32];
        let payload = b"hello between generated p256 nodes";
        let submit = submit_generated_p256(&sender, recipient_identity, message_id, payload);
        write_message(&mut sender_stream, &RelayMessage::Submit(submit)).expect("send submit");

        let request = match read_message(&mut recipient_stream).expect("read delivery request") {
            RelayMessage::DeliveryRequest(request) => request,
            other => panic!("expected generated p256 delivery request, got {other:?}"),
        };
        assert_eq!(request.submit.payload, payload);

        let recipient_identity = generated_p256_identity(&recipient);
        let mut receipt = RelayDeliveryReceipt {
            abi_version: RELAY_WIRE_ABI_VERSION,
            flags: 1,
            message_id,
            recipient: recipient_identity,
            status: RELAY_DELIVERY_STATUS_ACCEPTED,
            recipient_sequence: 2,
            recipient_log_head: [0xE1; 32],
            request_sha256: request_hash(&request),
            signature: RelaySignature {
                algorithm: 0,
                public_key: Vec::new(),
                signature: Vec::new(),
            },
        };
        receipt.signature = sign_generated_p256(
            &recipient,
            &receipt.recipient,
            &delivery_receipt_preimage(&receipt),
        );
        write_message(
            &mut recipient_stream,
            &RelayMessage::DeliveryReceipt(receipt),
        )
        .expect("send delivery receipt");

        let report = match read_message(&mut sender_stream).expect("read sender ack or report") {
            RelayMessage::Ack(_) => match read_message(&mut sender_stream).expect("read report") {
                RelayMessage::DeliveryReport(report) => report,
                other => panic!("expected generated p256 delivery report, got {other:?}"),
            },
            RelayMessage::DeliveryReport(report) => report,
            other => panic!("expected generated p256 delivery report, got {other:?}"),
        };
        assert_eq!(report.submit.message_id, message_id);
        assert_eq!(report.submit.payload, payload);
        assert!(verify_delivery_receipt(&report.recipient_receipt));

        let sender_identity = generated_p256_identity(&sender);
        let mut report_receipt = RelayDeliveryReportReceipt {
            abi_version: RELAY_WIRE_ABI_VERSION,
            flags: 1,
            message_id,
            sender: sender_identity,
            status: RELAY_REPORT_STATUS_ACCEPTED,
            sender_sequence: 8,
            sender_log_head: [0xE2; 32],
            report_sha256: report_hash(&report),
            signature: RelaySignature {
                algorithm: 0,
                public_key: Vec::new(),
                signature: Vec::new(),
            },
        };
        report_receipt.signature = sign_generated_p256(
            &sender,
            &report_receipt.sender,
            &report_receipt_preimage(&report_receipt),
        );
        write_message(
            &mut sender_stream,
            &RelayMessage::DeliveryReportReceipt(report_receipt),
        )
        .expect("send report receipt");
        assert!(matches!(
            read_message(&mut sender_stream).expect("read final ack"),
            RelayMessage::Ack(RelayAck {
                ok: true,
                code: 200,
                ..
            })
        ));
    }

    #[test]
    fn relay_runs_three_party_audit_handshake_over_udp() {
        let relay = Relay::new();
        let server_socket = Arc::new(UdpSocket::bind("127.0.0.1:0").expect("bind udp relay"));
        let addr = server_socket.local_addr().expect("udp relay addr");
        let serving_relay = relay.clone();
        thread::spawn(move || {
            let _ = serving_relay.serve_udp_socket(server_socket);
        });

        let dest_register = register_ed25519(2, 1);
        let dest_identity = dest_register.node.clone();
        let dest = UdpSocket::bind("127.0.0.1:0").expect("bind udp dest");
        dest.set_read_timeout(Some(Duration::from_secs(2)))
            .expect("timeout");
        write_udp(&dest, addr, &RelayMessage::Register(dest_register));
        assert!(matches!(
            read_udp(&dest),
            RelayMessage::Ack(RelayAck {
                ok: true,
                code: 200,
                ..
            })
        ));

        let src = UdpSocket::bind("127.0.0.1:0").expect("bind udp src");
        src.set_read_timeout(Some(Duration::from_secs(2)))
            .expect("timeout");
        let message_id = [0x44; 32];
        let submit = submit_ed25519(3, dest_identity, message_id, b"hello over udp");
        write_udp(&src, addr, &RelayMessage::Submit(submit));

        let request = match read_udp(&dest) {
            RelayMessage::DeliveryRequest(request) => request,
            other => panic!("expected udp delivery request, got {other:?}"),
        };
        let (recipient_key, recipient_identity) = ed25519_identity(2);
        let mut receipt = RelayDeliveryReceipt {
            abi_version: RELAY_WIRE_ABI_VERSION,
            flags: 1,
            message_id,
            recipient: recipient_identity,
            status: RELAY_DELIVERY_STATUS_ACCEPTED,
            recipient_sequence: 2,
            recipient_log_head: [0x55; 32],
            request_sha256: request_hash(&request),
            signature: RelaySignature {
                algorithm: 0,
                public_key: Vec::new(),
                signature: Vec::new(),
            },
        };
        receipt.signature = sign_ed25519(
            &recipient_key,
            &receipt.recipient,
            &delivery_receipt_preimage(&receipt),
        );
        write_udp(&dest, addr, &RelayMessage::DeliveryReceipt(receipt));

        let report = match read_udp(&src) {
            RelayMessage::Ack(_) => match read_udp(&src) {
                RelayMessage::DeliveryReport(report) => report,
                other => panic!("expected udp delivery report, got {other:?}"),
            },
            RelayMessage::DeliveryReport(report) => report,
            other => panic!("expected udp delivery report, got {other:?}"),
        };

        assert_eq!(report.submit.message_id, message_id);
        assert!(verify_delivery_receipt(&report.recipient_receipt));

        let (sender_key, sender_identity) = ed25519_identity(3);
        let mut report_receipt = RelayDeliveryReportReceipt {
            abi_version: RELAY_WIRE_ABI_VERSION,
            flags: 1,
            message_id,
            sender: sender_identity,
            status: RELAY_REPORT_STATUS_ACCEPTED,
            sender_sequence: 8,
            sender_log_head: [0x66; 32],
            report_sha256: report_hash(&report),
            signature: RelaySignature {
                algorithm: 0,
                public_key: Vec::new(),
                signature: Vec::new(),
            },
        };
        report_receipt.signature = sign_ed25519(
            &sender_key,
            &report_receipt.sender,
            &report_receipt_preimage(&report_receipt),
        );
        write_udp(
            &src,
            addr,
            &RelayMessage::DeliveryReportReceipt(report_receipt),
        );
    }

    #[test]
    fn relay_runs_three_party_audit_handshake_over_websocket() {
        let relay = Relay::new();
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind ws listener");
        let addr = listener.local_addr().expect("ws relay addr");
        let serving_relay = relay.clone();
        thread::spawn(move || {
            let _ = serving_relay.serve_ws_listener(listener);
        });

        let dest_register = register_ed25519(2, 1);
        let dest_identity = dest_register.node.clone();
        let (mut dest, _) =
            edgerun_tungstenite::connect(format!("ws://{addr}")).expect("connect ws dest");
        write_ws(&mut dest, &RelayMessage::Register(dest_register));
        assert!(matches!(
            read_ws(&mut dest),
            RelayMessage::Ack(RelayAck {
                ok: true,
                code: 200,
                ..
            })
        ));

        let (mut src, _) =
            edgerun_tungstenite::connect(format!("ws://{addr}")).expect("connect ws src");
        let message_id = [0x77; 32];
        let submit = submit_ed25519(3, dest_identity, message_id, b"hello over websocket");
        write_ws(&mut src, &RelayMessage::Submit(submit));

        let request = match read_ws(&mut dest) {
            RelayMessage::DeliveryRequest(request) => request,
            other => panic!("expected websocket delivery request, got {other:?}"),
        };
        let (recipient_key, recipient_identity) = ed25519_identity(2);
        let mut receipt = RelayDeliveryReceipt {
            abi_version: RELAY_WIRE_ABI_VERSION,
            flags: 1,
            message_id,
            recipient: recipient_identity,
            status: RELAY_DELIVERY_STATUS_ACCEPTED,
            recipient_sequence: 2,
            recipient_log_head: [0x88; 32],
            request_sha256: request_hash(&request),
            signature: RelaySignature {
                algorithm: 0,
                public_key: Vec::new(),
                signature: Vec::new(),
            },
        };
        receipt.signature = sign_ed25519(
            &recipient_key,
            &receipt.recipient,
            &delivery_receipt_preimage(&receipt),
        );
        write_ws(&mut dest, &RelayMessage::DeliveryReceipt(receipt));

        let report = match read_ws(&mut src) {
            RelayMessage::Ack(_) => match read_ws(&mut src) {
                RelayMessage::DeliveryReport(report) => report,
                other => panic!("expected websocket delivery report, got {other:?}"),
            },
            RelayMessage::DeliveryReport(report) => report,
            other => panic!("expected websocket delivery report, got {other:?}"),
        };

        assert_eq!(report.submit.message_id, message_id);
        assert!(verify_delivery_receipt(&report.recipient_receipt));

        let (sender_key, sender_identity) = ed25519_identity(3);
        let mut report_receipt = RelayDeliveryReportReceipt {
            abi_version: RELAY_WIRE_ABI_VERSION,
            flags: 1,
            message_id,
            sender: sender_identity,
            status: RELAY_REPORT_STATUS_ACCEPTED,
            sender_sequence: 8,
            sender_log_head: [0x99; 32],
            report_sha256: report_hash(&report),
            signature: RelaySignature {
                algorithm: 0,
                public_key: Vec::new(),
                signature: Vec::new(),
            },
        };
        report_receipt.signature = sign_ed25519(
            &sender_key,
            &report_receipt.sender,
            &report_receipt_preimage(&report_receipt),
        );
        write_ws(
            &mut src,
            &RelayMessage::DeliveryReportReceipt(report_receipt),
        );
    }

    #[test]
    fn relay_runs_three_party_audit_handshake_over_wss() {
        let relay = Relay::new();
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind wss listener");
        let addr = listener.local_addr().expect("wss relay addr");
        let serving_relay = relay.clone();
        thread::spawn(move || {
            let _ = serving_relay.serve_wss_listener(listener);
        });

        let dest_register = register_ed25519(2, 1);
        let dest_identity = dest_register.node.clone();
        let mut dest = connect_wss(addr);
        write_ws(&mut dest, &RelayMessage::Register(dest_register));
        assert!(matches!(
            read_ws(&mut dest),
            RelayMessage::Ack(RelayAck {
                ok: true,
                code: 200,
                ..
            })
        ));

        let mut src = connect_wss(addr);
        let message_id = [0xAA; 32];
        let submit = submit_ed25519(3, dest_identity, message_id, b"hello over wss");
        write_ws(&mut src, &RelayMessage::Submit(submit));

        let request = match read_ws(&mut dest) {
            RelayMessage::DeliveryRequest(request) => request,
            other => panic!("expected wss delivery request, got {other:?}"),
        };
        let (recipient_key, recipient_identity) = ed25519_identity(2);
        let mut receipt = RelayDeliveryReceipt {
            abi_version: RELAY_WIRE_ABI_VERSION,
            flags: 1,
            message_id,
            recipient: recipient_identity,
            status: RELAY_DELIVERY_STATUS_ACCEPTED,
            recipient_sequence: 2,
            recipient_log_head: [0xBB; 32],
            request_sha256: request_hash(&request),
            signature: RelaySignature {
                algorithm: 0,
                public_key: Vec::new(),
                signature: Vec::new(),
            },
        };
        receipt.signature = sign_ed25519(
            &recipient_key,
            &receipt.recipient,
            &delivery_receipt_preimage(&receipt),
        );
        write_ws(&mut dest, &RelayMessage::DeliveryReceipt(receipt));

        let report = match read_ws(&mut src) {
            RelayMessage::Ack(_) => match read_ws(&mut src) {
                RelayMessage::DeliveryReport(report) => report,
                other => panic!("expected wss delivery report, got {other:?}"),
            },
            RelayMessage::DeliveryReport(report) => report,
            other => panic!("expected wss delivery report, got {other:?}"),
        };

        assert_eq!(report.submit.message_id, message_id);
        assert!(verify_delivery_receipt(&report.recipient_receipt));

        let (sender_key, sender_identity) = ed25519_identity(3);
        let mut report_receipt = RelayDeliveryReportReceipt {
            abi_version: RELAY_WIRE_ABI_VERSION,
            flags: 1,
            message_id,
            sender: sender_identity,
            status: RELAY_REPORT_STATUS_ACCEPTED,
            sender_sequence: 8,
            sender_log_head: [0xCC; 32],
            report_sha256: report_hash(&report),
            signature: RelaySignature {
                algorithm: 0,
                public_key: Vec::new(),
                signature: Vec::new(),
            },
        };
        report_receipt.signature = sign_ed25519(
            &sender_key,
            &report_receipt.sender,
            &report_receipt_preimage(&report_receipt),
        );
        write_ws(
            &mut src,
            &RelayMessage::DeliveryReportReceipt(report_receipt),
        );
    }
}
