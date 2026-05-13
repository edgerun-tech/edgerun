use std::collections::BTreeMap;
use std::io::{self, ErrorKind, Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream, ToSocketAddrs};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use edgerun_protocols::websocket::{
    WS_EXTENDED_64_LEN, WS_MASK_LEN, WebSocketError, WebSocketMessage, decode_client_message,
    decode_frame_prefix, decode_handshake_request, decode_payload_len, encode_server_binary,
    encode_server_control, encode_upgrade_response, handshake_complete,
};

use crate::channel::ChannelEnvelope;
use crate::frame_codec::{channel_envelope_bytes, channel_envelope_from_bytes};
use crate::protocol::{MAX_WORK_FRAME_LEN, NodeId};
use crate::std_runtime::threading::{drain_joined_threads, join_optional_thread};

const ACCEPT_POLL_MS: u64 = 10;
const CONNECTION_READ_TIMEOUT_MS: u64 = 250;
const DEFAULT_WS_PROTOCOL: &str = "edgerun-work-v1";

#[derive(Debug)]
pub struct WebSocketWorkHub {
    listen_addr: SocketAddr,
    inbox: Arc<Mutex<Vec<ChannelEnvelope>>>,
    peers: Arc<Mutex<BTreeMap<NodeId, TcpStream>>>,
    shutdown: Arc<AtomicBool>,
    accept_thread: Option<JoinHandle<()>>,
    worker_threads: Arc<Mutex<Vec<JoinHandle<()>>>>,
}

impl WebSocketWorkHub {
    pub fn bind<A: ToSocketAddrs>(addr: A) -> io::Result<Self> {
        Self::bind_with_protocol(addr, DEFAULT_WS_PROTOCOL)
    }

    pub fn bind_with_protocol<A: ToSocketAddrs>(
        addr: A,
        protocol: &'static str,
    ) -> io::Result<Self> {
        let listener = TcpListener::bind(addr)?;
        listener.set_nonblocking(true)?;
        let listen_addr = listener.local_addr()?;
        let inbox = Arc::new(Mutex::new(Vec::new()));
        let peers = Arc::new(Mutex::new(BTreeMap::new()));
        let shutdown = Arc::new(AtomicBool::new(false));
        let worker_threads = Arc::new(Mutex::new(Vec::new()));
        let inbox_for_thread = Arc::clone(&inbox);
        let peers_for_thread = Arc::clone(&peers);
        let shutdown_for_thread = Arc::clone(&shutdown);
        let workers_for_thread = Arc::clone(&worker_threads);
        let accept_thread = thread::spawn(move || {
            while !shutdown_for_thread.load(Ordering::Acquire) {
                match listener.accept() {
                    Ok((stream, _peer)) => {
                        let _ = stream.set_read_timeout(Some(Duration::from_millis(
                            CONNECTION_READ_TIMEOUT_MS,
                        )));
                        let inbox = Arc::clone(&inbox_for_thread);
                        let peers = Arc::clone(&peers_for_thread);
                        let shutdown = Arc::clone(&shutdown_for_thread);
                        let handle = thread::spawn(move || {
                            let _ =
                                serve_websocket_client(stream, protocol, inbox, peers, shutdown);
                        });
                        workers_for_thread
                            .lock()
                            .expect("websocket worker thread list poisoned")
                            .push(handle);
                    }
                    Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(ACCEPT_POLL_MS));
                    }
                    Err(_) => break,
                }
            }
        });
        Ok(Self {
            listen_addr,
            inbox,
            peers,
            shutdown,
            accept_thread: Some(accept_thread),
            worker_threads,
        })
    }

    pub fn listen_addr(&self) -> SocketAddr {
        self.listen_addr
    }

    pub fn drain_envelopes(&self) -> Vec<ChannelEnvelope> {
        self.inbox
            .lock()
            .expect("websocket hub inbox poisoned")
            .drain(..)
            .collect()
    }

    pub fn connected_peer_count(&self) -> usize {
        self.peers
            .lock()
            .expect("websocket peer map poisoned")
            .len()
    }

    pub fn send_envelope_to(&self, node_id: NodeId, envelope: &ChannelEnvelope) -> io::Result<()> {
        let payload = channel_envelope_bytes(envelope).map_err(invalid_ws_data)?;
        self.send_binary_to(node_id, &payload)
    }

    pub fn send_binary_to(&self, node_id: NodeId, payload: &[u8]) -> io::Result<()> {
        let frame = encode_server_binary(payload).map_err(invalid_ws_data)?;
        let mut peers = self.peers.lock().expect("websocket peer map poisoned");
        let stream = peers.get_mut(&node_id).ok_or_else(|| {
            io::Error::new(ErrorKind::NotConnected, "websocket peer is not connected")
        })?;
        stream.write_all(&frame)?;
        stream.flush()
    }

    pub fn shutdown(&mut self) -> io::Result<()> {
        self.shutdown.store(true, Ordering::Release);
        let _ = TcpStream::connect(self.listen_addr);
        join_optional_thread(&mut self.accept_thread, "websocket accept thread panicked")?;
        drain_joined_threads(
            &self.worker_threads,
            "websocket worker thread list poisoned",
            "websocket worker thread panicked",
        )
    }
}

impl Drop for WebSocketWorkHub {
    fn drop(&mut self) {
        let _ = self.shutdown();
    }
}

fn serve_websocket_client(
    mut stream: TcpStream,
    protocol: &'static str,
    inbox: Arc<Mutex<Vec<ChannelEnvelope>>>,
    peers: Arc<Mutex<BTreeMap<NodeId, TcpStream>>>,
    shutdown: Arc<AtomicBool>,
) -> io::Result<()> {
    complete_handshake(&mut stream, protocol)?;
    loop {
        if shutdown.load(Ordering::Acquire) {
            return Ok(());
        }
        match read_client_message(&mut stream) {
            Ok(WebSocketMessage::Binary(bytes)) => {
                let envelope = channel_envelope_from_bytes(&bytes).map_err(invalid_ws_data)?;
                if let Ok(peer_stream) = stream.try_clone() {
                    peers
                        .lock()
                        .expect("websocket peer map poisoned")
                        .insert(envelope.from, peer_stream);
                }
                forward_envelope_to_peer(&peers, envelope.to, &bytes);
                inbox
                    .lock()
                    .expect("websocket hub inbox poisoned")
                    .push(envelope);
            }
            Ok(WebSocketMessage::Ping(payload)) => {
                let frame = encode_server_control(0xA, &payload).map_err(invalid_ws_data)?;
                stream.write_all(&frame)?;
                stream.flush()?;
            }
            Ok(WebSocketMessage::Pong) => {}
            Ok(WebSocketMessage::Close) => return Ok(()),
            Err(error)
                if error.kind() == ErrorKind::WouldBlock || error.kind() == ErrorKind::TimedOut =>
            {
                continue;
            }
            Err(error) if error.kind() == ErrorKind::UnexpectedEof => return Ok(()),
            Err(error) => return Err(error),
        }
    }
}

fn forward_envelope_to_peer(
    peers: &Arc<Mutex<BTreeMap<NodeId, TcpStream>>>,
    node_id: NodeId,
    payload: &[u8],
) {
    let Ok(frame) = encode_server_binary(payload).map_err(invalid_ws_data) else {
        return;
    };
    let mut peers = peers.lock().expect("websocket peer map poisoned");
    let Some(peer) = peers.get_mut(&node_id) else {
        return;
    };
    if peer.write_all(&frame).and_then(|_| peer.flush()).is_err() {
        peers.remove(&node_id);
    }
}

fn complete_handshake(stream: &mut TcpStream, protocol: &str) -> io::Result<()> {
    let mut request = Vec::new();
    let mut byte = [0u8; 1];
    while request.len() <= edgerun_protocols::websocket::WS_MAX_HANDSHAKE_LEN {
        stream.read_exact(&mut byte)?;
        request.push(byte[0]);
        if handshake_complete(&request) {
            let request = decode_handshake_request(request).map_err(invalid_ws_data)?;
            let response = encode_upgrade_response(&request, protocol).map_err(invalid_ws_data)?;
            stream.write_all(response.as_bytes())?;
            stream.flush()?;
            return Ok(());
        }
    }
    Err(invalid_ws_data(WebSocketError::HandshakeTooLarge))
}

fn read_client_message(stream: &mut TcpStream) -> io::Result<WebSocketMessage> {
    let mut header = [0u8; 2];
    stream.read_exact(&mut header)?;
    let prefix = decode_frame_prefix(&header)
        .and_then(|prefix| prefix.require_client_mask())
        .map_err(invalid_ws_data)?;
    let mut extended = [0u8; WS_EXTENDED_64_LEN];
    let extended_len = prefix.extended_len_bytes();
    if extended_len != 0 {
        stream.read_exact(&mut extended[..extended_len])?;
    }
    let payload_len = decode_payload_len(prefix, &extended[..extended_len], MAX_WORK_FRAME_LEN)
        .map_err(invalid_ws_data)?;
    let mut mask = [0u8; WS_MASK_LEN];
    stream.read_exact(&mut mask)?;
    let mut payload = vec![0u8; payload_len];
    stream.read_exact(&mut payload)?;
    decode_client_message(prefix.fin, prefix.opcode, mask, payload).map_err(invalid_ws_data)
}

fn invalid_ws_data(error: impl core::fmt::Debug) -> io::Error {
    io::Error::new(
        ErrorKind::InvalidData,
        format!("invalid websocket data: {error:?}"),
    )
}
