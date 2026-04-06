//! The Lifegraph mesh daemon — an event loop that ties together all mesh
//! components into a single long-running process.
//!
//! The daemon:
//! 1. Discovers local network interfaces and opens transport sockets
//! 2. Runs a `poll()` event loop processing inbound frames
//! 3. Periodically broadcasts discovery packets
//! 4. Detects dead peers and cleans up their routes
//! 5. Forwards transit frames to the correct next-hop
//! 6. Shuts down cleanly on SIGINT/SIGTERM

use lifegraph_hardware_signing::{MeshSigner, NodeID};
use lifegraph_mesh::{FrameType, LocalNode, MeshFrame, MeshFrameHeader};
use lifegraph_mesh_link::MeshLink;
use lifegraph_mesh_router::MeshRouter;
use lifegraph_mesh_capability::{MeshCapabilityServer, MeshEnvelopeDispatcher, OutboundQueue};
use lifegraph_mesh_session::{HandshakeAccept, HandshakeInit, SessionError, SessionManager};
use lifegraph_remote_capability::RemoteCapabilityProvider;
use lifegraph_hardware_signing::HardwareSigningError;
use lifegraph_proto::lifegraph::v0::capability_runtime::CapabilityRemoteEnvelope;
use prost::Message;
use sha2::{Digest, Sha256};
use libc::{c_int, pollfd, POLLIN};
use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
use std::io;
use std::rc::Rc;
use std::time::{Duration, Instant};

// ---------------------------------------------------------------------------
// Daemon configuration
// ---------------------------------------------------------------------------

/// Configuration for the mesh daemon.
#[derive(Clone, Debug)]
pub struct MeshDaemonConfig {
    /// How often to broadcast discovery packets (default: 5 seconds).
    pub heartbeat_interval: Duration,
    /// Default TTL for outbound data frames (default: 16).
    pub default_ttl: u8,
    /// Local IPv4 addresses for multicast on each interface (keyed by ifindex).
    pub interface_ipv4: HashMap<c_int, [u8; 4]>,
}

impl Default for MeshDaemonConfig {
    fn default() -> Self {
        Self {
            heartbeat_interval: Duration::from_secs(5),
            default_ttl: 16,
            interface_ipv4: HashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Daemon state
// ---------------------------------------------------------------------------

/// The running mesh daemon.
pub struct MeshDaemon<P: RemoteCapabilityProvider> {
    link: MeshLink,
    router: MeshRouter,
    server: MeshCapabilityServer<P>,
    sessions: SessionManager,
    config: MeshDaemonConfig,
    last_heartbeat: Instant,
    running: bool,
    /// Hardware-backed signer. The private key never leaves the hardware;
    /// the daemon only holds the public key (NodeID) and sends SHA-256
    /// digests to the hardware for signing.
    signer: Option<Box<dyn MeshSigner>>,
    /// Shared outbound queue for capability transports.
    outbound: OutboundQueue,
    /// Pending handshakes: peer → ECDH ephemeral secret.
    pending_handshakes: HashMap<NodeID, lifegraph_mesh_session::EphemeralSecret>,
}

impl<P: RemoteCapabilityProvider> MeshDaemon<P> {
    /// Creates a new mesh daemon with the given identity, configuration, and
    /// a capability provider that handles inbound capability requests.
    pub fn new(node_id: NodeID, config: MeshDaemonConfig, provider: P) -> Self {
        let mut router = MeshRouter::new(LocalNode::new(node_id));
        router.set_default_ttl(config.default_ttl);
        let heartbeat_interval = config.heartbeat_interval;

        Self {
            link: MeshLink::new(),
            router,
            server: MeshCapabilityServer::new(provider),
            sessions: SessionManager::new(node_id),
            config,
            last_heartbeat: Instant::now() - heartbeat_interval,
            running: false,
            signer: None,
            outbound: Rc::new(RefCell::new(VecDeque::new())),
            pending_handshakes: HashMap::new(),
        }
    }

    /// Returns the shared outbound queue for creating `MeshCapabilityTransport` instances.
    pub fn outbound_queue(&self) -> OutboundQueue {
        Rc::clone(&self.outbound)
    }

    /// Attaches a hardware-backed signer.
    ///
    /// The private key **never leaves the hardware**.  The daemon only:
    /// 1. Holds the public key (`NodeID`) from the signer
    /// 2. Hashes each outbound frame with SHA-256
    /// 3. Sends the 32-byte digest to hardware → gets 64-byte signature
    pub fn with_signer(mut self, signer: Box<dyn MeshSigner>) -> Self {
        let node_id = signer.node_id();
        self.link.set_local_node_id(node_id);
        self.signer = Some(signer);
        self
    }

    /// Returns the signer's NodeID, or zeros if no signer is attached.
    #[must_use]
    pub fn node_id(&self) -> NodeID {
        self.signer.as_ref().map(|s| s.node_id()).unwrap_or(NodeID([0u8; 64]))
    }

    // -----------------------------------------------------------------------
    // Interface setup
    // -----------------------------------------------------------------------

    /// Opens raw Ethernet sockets on all discovered UP interfaces.
    ///
    /// This enumerates interfaces from `/sys/class/net/` and opens
    /// a raw socket on each one that is UP (administratively).
    pub fn discover_and_open_interfaces(&mut self) -> Result<Vec<String>, io::Error> {
        let mut opened = Vec::new();
        let net_dir = std::path::Path::new("/sys/class/net");
        if !net_dir.exists() {
            return Ok(opened);
        }
        for entry in std::fs::read_dir(net_dir)? {
            let entry = entry?;
            let name = entry.file_name().to_string_lossy().to_string();
            // Skip loopback
            if name == "lo" {
                continue;
            }
            // Check if interface is UP via operstate
            let state_path = entry.path().join("operstate");
            let operstate = std::fs::read_to_string(&state_path).unwrap_or_default();
            if operstate.trim() != "up" {
                continue;
            }
            let ifindex = interface_name_to_ifindex(&name)?;
            if ifindex <= 0 {
                continue;
            }
            if let Err(e) = self.link.add_raw_ethernet(ifindex) {
                eprintln!("mesh-daemon: failed to open raw socket on {name}: {e}");
                continue;
            }
            opened.push(name);
        }
        Ok(opened)
    }

    /// Opens multicast discovery sockets on interfaces with configured IPv4 addresses.
    pub fn open_multicast_sockets(&mut self) -> Result<Vec<String>, io::Error> {
        let mut opened = Vec::new();
        for (&ifindex, &ipv4) in &self.config.interface_ipv4 {
            if let Err(e) = self.link.add_multicast(ipv4) {
                eprintln!("mesh-daemon: failed to open multicast on ifindex {ifindex}: {e}");
                continue;
            }
            opened.push(format!("ifindex={ifindex}"));
        }
        Ok(opened)
    }

    /// Adds an IP tunnel to a known peer at a specific IP address.
    pub fn add_tunnel(
        &mut self,
        peer_id: NodeID,
        peer_ip: [u8; 4],
        port: u16,
    ) -> Result<(), io::Error> {
        self.link.add_tunnel(peer_id, peer_ip, port)
    }

    // -----------------------------------------------------------------------
    // Routing and dispatching access
    // -----------------------------------------------------------------------

    #[must_use]
    pub fn router(&self) -> &MeshRouter {
        &self.router
    }

    #[must_use]
    pub fn router_mut(&mut self) -> &mut MeshRouter {
        &mut self.router
    }

    #[must_use]
    pub fn dispatcher(&mut self) -> &mut MeshEnvelopeDispatcher {
        self.server.dispatcher()
    }

    /// Access the capability server for direct envelope dispatch.
    pub fn server_mut(&mut self) -> &mut MeshCapabilityServer<P> {
        &mut self.server
    }

    /// Access the session manager for handshakes and session management.
    pub fn sessions_mut(&mut self) -> &mut SessionManager {
        &mut self.sessions
    }

    // -----------------------------------------------------------------------
    // Outbound frame signing
    // -----------------------------------------------------------------------

    /// Signs all pending outbound frames using the hardware signer.
    ///
    /// The private key **never leaves secure hardware**. For each frame:
    /// 1. Fill in `src` NodeID from the signer's public key
    /// 2. Hash `header_bytes || payload` with SHA-256 → 32-byte digest
    /// 3. Send digest to hardware → get 64-byte ECDSA signature
    /// 4. Fill in the signature
    fn sign_pending_frames(&mut self) -> Result<Vec<MeshFrame>, HardwareSigningError> {
        let mut signed = Vec::new();
        let pending = self.link.drain_pending_frames_raw();
        for mut frame in pending {
            let Some(ref signer) = self.signer else {
                // No signer — send unsigned (dev mode only)
                frame.header.src = self.router.node_id();
                signed.push(frame);
                continue;
            };

            // Step 1: fill in src NodeID (public key only, no secret material)
            frame.header.src = signer.node_id();

            // Step 2: hash the preimage (header + payload)
            let preimage = frame.signed_preimage();
            let digest = Sha256::digest(&preimage);
            let mut digest_bytes = [0u8; 32];
            digest_bytes.copy_from_slice(&digest);

            // Step 3: send digest to hardware → get 64-byte signature
            frame.signature = signer.sign_digest(&digest_bytes)?;

            signed.push(frame);
        }
        Ok(signed)
    }

    // -----------------------------------------------------------------------
    // Event loop
    // -----------------------------------------------------------------------

    /// Runs the daemon's event loop until `stop()` is called.
    pub fn run(&mut self) -> Result<(), io::Error> {
        self.running = true;
        while self.running {
            if !self.run_once(Duration::from_millis(100))? {
                // Timeout expired with no events — check heartbeat
                let now = Instant::now();
                if now.duration_since(self.last_heartbeat) >= self.config.heartbeat_interval {
                    let _ = self.broadcast_discovery();
                    self.tick_heartbeat();
                    self.last_heartbeat = now;
                }
            }
        }
        Ok(())
    }

    /// Stops the daemon's event loop.
    pub fn stop(&mut self) {
        self.running = false;
    }

    /// Processes one round of events with the given timeout.
    ///
    /// Returns `true` if at least one event was processed, `false` if
    /// the timeout expired with no events.
    pub fn run_once(&mut self, timeout: Duration) -> Result<bool, io::Error> {
        let fds = self.link.poll_fds();
        if fds.is_empty() {
            std::thread::sleep(timeout);
            return Ok(false);
        }

        let mut poll_fds: Vec<pollfd> = fds
            .iter()
            .map(|&fd| pollfd {
                fd,
                events: POLLIN,
                revents: 0,
            })
            .collect();

        let timeout_ms = timeout.as_millis().min(i32::MAX as u128) as i32;
        let nfds = unsafe { libc::poll(poll_fds.as_mut_ptr(), poll_fds.len() as _, timeout_ms) };

        if nfds < 0 {
            let err = io::Error::last_os_error();
            if err.kind() == io::ErrorKind::Interrupted {
                return Ok(false); // EINTR — retry
            }
            return Err(err);
        }
        if nfds == 0 {
            return Ok(false); // timeout
        }

        let mut processed_any = false;

        // 1. Read inbound frames, verify signatures, process discovery, forward
        for pfd in &poll_fds {
            if pfd.revents & POLLIN != 0 {
                let count = self.link.pump(&mut self.router)?;
                if count > 0 {
                    processed_any = true;
                }
            }
        }

        // 2. Process inbound data frames: handshake, session accept, or decrypt+dispatch
        let inbound_frames = self.link.drain_inbound_data_frames();
        for frame in inbound_frames {
            match frame.header.frame_type {
                FrameType::HandshakeInit => {
                    if let Some(init) = HandshakeInit::decode(&frame.payload) {
                        let peer = init.initiator;
                        match self.sessions.respond_to_handshake(&init) {
                            Ok((accept, _secret)) => {
                                let accept_frame = MeshFrame {
                                    header: MeshFrameHeader {
                                        dest: peer,
                                        src: NodeID([0u8; 64]),
                                        ttl: self.config.default_ttl,
                                        frame_type: FrameType::HandshakeAccept,
                                    },
                                    payload: accept.encode().to_vec(),
                                    signature: [0u8; lifegraph_hardware_signing::MESH_SIGNATURE_LENGTH],
                                };
                                self.link.queue_frame(accept_frame);
                                self.drain_and_encrypt_buffered(&peer)?;
                                processed_any = true;
                            }
                            Err(e) => {
                                eprintln!("mesh-daemon: handshake error: {e}");
                            }
                        }
                    }
                }
                FrameType::HandshakeAccept => {
                    if let Some(accept) = HandshakeAccept::decode(&frame.payload) {
                        let peer = accept.responder;
                        if let Some(secret) = self.pending_handshakes.remove(&peer) {
                            if self.sessions.complete_handshake_initiator(&accept, &secret).is_ok() {
                                self.drain_and_encrypt_buffered(&peer)?;
                            }
                            processed_any = true;
                        }
                    }
                }
                FrameType::Data => {
                    let sender = frame.header.src;
                    match self.sessions.decrypt_from(sender, &frame.payload) {
                        Ok(decrypted) => {
                            // Decode protobuf and deliver to the server
                            if let Ok(envelope) = CapabilityRemoteEnvelope::decode(decrypted.as_slice()) {
                                if let Some(inboxes) = self.server.dispatcher().inboxes_mut().get_mut(&sender) {
                                    if let Some(inbox) = inboxes.first_mut() {
                                        inbox.push(envelope);
                                        processed_any = true;
                                    }
                                }
                            }
                        }
                        Err(SessionError::NoActiveSession) => {
                            // Peer sent encrypted data but we have no session — drop
                        }
                        Err(e) => {
                            eprintln!("mesh-daemon: decrypt error from {sender:?}: {e}");
                        }
                    }
                }
                FrameType::Discovery | FrameType::RouteAdv => {
                    // Already processed by link.pump()
                }
                FrameType::Unknown(_) => {}
            }
        }

        // 3. Dispatch inbound capability envelopes through the server
        let dispatched = match self.server.serve_one(&mut self.link) {
            Ok(d) => d,
            Err(e) => {
                eprintln!("mesh-daemon: capability dispatch error: {e}");
                false
            }
        };
        if dispatched {
            processed_any = true;
        }

        // 4. Drain outbound queue from capability transports, encrypt, sign, send
        self.drain_and_encrypt_outbound()?;

        // 5. Sign and send pending outbound frames (handshakes, etc.)
        let signed_frames = match self.sign_pending_frames() {
            Ok(frames) => frames,
            Err(e) => {
                eprintln!("mesh-daemon: signing error: {e}");
                Vec::new()
            }
        };

        for frame in signed_frames {
            self.link.queue_frame(frame);
        }

        let sent = self.link.drain_pending_frames(&mut self.router)?;
        if sent > 0 {
            processed_any = true;
        }

        Ok(processed_any)
    }

    /// Drains the outbound queue from capability transports, encrypts each
    /// payload through the session manager, and queues the resulting mesh frames.
    ///
    /// If a session has expired, initiates a rekey handshake and re-queues
    /// the payloads for delivery after the handshake completes.
    fn drain_and_encrypt_outbound(&mut self) -> Result<(), io::Error> {
        let payloads: Vec<(NodeID, Vec<u8>)> = self.outbound.borrow_mut().drain(..).collect();
        let mut rekey_peers: HashMap<NodeID, Vec<Vec<u8>>> = HashMap::new();

        for (peer, payload) in payloads {
            match self.sessions.encrypt_for(peer, &payload) {
                Ok(encrypted) => {
                    let frame = MeshFrame {
                        header: MeshFrameHeader {
                            dest: peer,
                            src: NodeID([0u8; 64]),
                            ttl: self.config.default_ttl,
                            frame_type: FrameType::Data,
                        },
                        payload: encrypted,
                        signature: [0u8; lifegraph_hardware_signing::MESH_SIGNATURE_LENGTH],
                    };
                    self.link.queue_frame(frame);
                }
                Err(SessionError::SessionExpired) => {
                    // Session expired — buffer for rekey
                    rekey_peers.entry(peer).or_default().push(payload);
                }
                Err(SessionError::NoActiveSession) => {
                    // No session yet — shouldn't happen if transport only sends
                    // after handshake, but if it does, buffer for handshake.
                    rekey_peers.entry(peer).or_default().push(payload);
                }
                Err(e) => {
                    eprintln!("mesh-daemon: encrypt error for {peer:?}: {e}");
                }
            }
        }

        // Initiate rekey handshakes for expired sessions
        for (peer, payloads) in rekey_peers {
            if !self.pending_handshakes.contains_key(&peer) {
                let (init, secret) = self.sessions.initiate_handshake(peer);
                let handshake_frame = MeshFrame {
                    header: MeshFrameHeader {
                        dest: peer,
                        src: NodeID([0u8; 64]),
                        ttl: self.config.default_ttl,
                        frame_type: FrameType::HandshakeInit,
                    },
                    payload: init.encode().to_vec(),
                    signature: [0u8; lifegraph_hardware_signing::MESH_SIGNATURE_LENGTH],
                };
                self.pending_handshakes.insert(peer, secret);
                self.link.queue_frame(handshake_frame);
            }
            // Buffer payloads for after handshake completes
            for payload in payloads {
                self.outbound.borrow_mut().push_back((peer, payload));
            }
        }

        Ok(())
    }

    /// Drains buffered payloads for the given peer from the outbound queue
    /// and encrypts/sends them. Called after a handshake completes.
    fn drain_and_encrypt_buffered(&mut self, peer: &NodeID) -> Result<(), io::Error> {
        // Separate matching payloads from non-matching
        let mut queue = self.outbound.borrow_mut();
        let (matching, others): (Vec<_>, Vec<_>) = queue.drain(..).partition(|(p, _)| p == peer);
        // Put non-matching payloads back
        for item in others {
            queue.push_back(item);
        }
        drop(queue);

        for (peer_id, payload) in matching {
            if let Ok(encrypted) = self.sessions.encrypt_for(peer_id, &payload) {
                let frame = MeshFrame {
                    header: MeshFrameHeader {
                        dest: peer_id,
                        src: NodeID([0u8; 64]),
                        ttl: self.config.default_ttl,
                        frame_type: FrameType::Data,
                    },
                    payload: encrypted,
                    signature: [0u8; lifegraph_hardware_signing::MESH_SIGNATURE_LENGTH],
                };
                self.link.queue_frame(frame);
            }
        }
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Discovery and heartbeat
    // -----------------------------------------------------------------------

    /// Broadcasts a discovery frame on all multicast and raw Ethernet sockets.
    pub fn broadcast_discovery(&mut self) -> Result<(), io::Error> {
        self.link.broadcast_discovery(&mut self.router)
    }

    /// Ticks the heartbeat — increments missed counters and removes dead peers.
    pub fn tick_heartbeat(&mut self) {
        let dead = self.router.tick_heartbeat();
        for peer_id in &dead {
            eprintln!("mesh-daemon: peer {peer_id:?} is dead, removing routes");
            self.server.dispatcher().remove_peer(peer_id);
        }
    }

    /// Returns whether the daemon is currently running.
    #[must_use]
    pub fn is_running(&self) -> bool {
        self.running
    }
}

// -----------------------------------------------------------------------
// Helper: interface name → ifindex
// -----------------------------------------------------------------------

fn interface_name_to_ifindex(name: &str) -> Result<c_int, io::Error> {
    let c_name = std::ffi::CString::new(name).map_err(|_| {
        io::Error::new(io::ErrorKind::InvalidInput, "interface name contains null byte")
    })?;
    let ifindex = unsafe { libc::if_nametoindex(c_name.as_ptr()) };
    if ifindex == 0 {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("interface {name} not found"),
        ));
    }
    Ok(ifindex as c_int)
}

#[cfg(test)]
mod tests {
    use super::*;
    use lifegraph_remote_capability::{
        RemoteCapabilityProvider, RemoteInvocationResult,
    };
    use lifegraph_capabilities::CapabilityError;
    use lifegraph_proto::lifegraph::v0::capability::{
        CapabilityDescriptor, CapabilityGrant, CapabilityInvocation,
        CapabilityRequest, CapabilityRevocation,
    };
    use lifegraph_proto::lifegraph::v0::capability_runtime::{
        CapabilitySessionAccept, CapabilitySessionClose, CapabilitySessionOpen,
    };

    struct TestProvider;
    impl RemoteCapabilityProvider for TestProvider {
        fn descriptor(&self) -> CapabilityDescriptor { CapabilityDescriptor::default() }
        fn open_session(&mut self, _open: &CapabilitySessionOpen) -> Result<CapabilitySessionAccept, CapabilityError> {
            Err(CapabilityError::Unsupported("test".into()))
        }
        fn invoke(&mut self, _: &[u8], _: &CapabilityInvocation, _: Option<&[u8]>) -> Result<RemoteInvocationResult, CapabilityError> {
            Err(CapabilityError::Unsupported("test".into()))
        }
        fn close_session(&mut self, _: &CapabilitySessionClose) -> Result<(), CapabilityError> { Ok(()) }
        fn handle_request(&mut self, _: &CapabilityRequest) -> Result<Option<CapabilityGrant>, CapabilityError> { Ok(None) }
        fn handle_grant(&mut self, _: &CapabilityGrant) -> Result<(), CapabilityError> { Ok(()) }
        fn handle_revocation(&mut self, _: &CapabilityRevocation) -> Result<(), CapabilityError> { Ok(()) }
    }

    type TestDaemon = MeshDaemon<TestProvider>;

    fn node_id(v: u8) -> NodeID {
        let mut bytes = [0u8; 64];
        bytes[0] = v;
        NodeID(bytes)
    }

    #[test]
    fn daemon_creates_with_defaults() {
        let daemon = TestDaemon::new(node_id(0xAA), MeshDaemonConfig::default(), TestProvider);
        assert!(daemon.is_running() == false);
        assert_eq!(daemon.router().default_ttl(), 16);
    }

    #[test]
    fn daemon_stops_when_requested() {
        let mut daemon = TestDaemon::new(node_id(0xAA), MeshDaemonConfig::default(), TestProvider);
        daemon.running = true;
        daemon.stop();
        assert!(!daemon.is_running());
    }

    #[test]
    fn interface_name_to_ifindex_rejects_null_bytes() {
        let err = interface_name_to_ifindex("eth0\0bad").unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidInput);
    }

    #[test]
    fn config_custom_values() {
        let config = MeshDaemonConfig {
            heartbeat_interval: Duration::from_secs(10),
            default_ttl: 32,
            interface_ipv4: HashMap::new(),
        };
        let daemon = TestDaemon::new(node_id(0xBB), config, TestProvider);
        assert_eq!(daemon.router().default_ttl(), 32);
    }
}
