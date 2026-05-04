//! The edgerun mesh daemon — an event loop that ties together all mesh
//! components into a single long-running process.
//!
//! The daemon:
//! 1. Discovers local network interfaces and opens transport sockets
//! 2. Runs a `poll()` event loop processing inbound frames
//! 3. Periodically broadcasts discovery packets
//! 4. Detects dead peers and cleans up their routes
//! 5. Forwards transit frames to the correct next-hop
//! 6. Shuts down cleanly on SIGINT/SIGTERM

#![no_std]

extern crate alloc;
#[cfg(not(target_os = "none"))]
extern crate std;

use alloc::boxed::Box;
use alloc::collections::{BTreeMap as HashMap, VecDeque};
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec;
use alloc::vec::Vec;
#[cfg(target_os = "none")]
use bare_poll::{c_int, pollfd, POLLIN};
#[cfg(target_os = "none")]
use bare_time::Instant;
use core::marker::Send;
use core::mem::drop;
use core::module_path;
use core::ops::FnMut;
use core::option::Option::{self, None, Some};
use core::result::Result::{self, Err, Ok};
use core::sync::atomic::{AtomicBool, Ordering};
#[cfg(target_os = "none")]
use core::time::Duration;
use edgerun_core::protocol::capability_runtime::CapabilityRemoteEnvelope;
use edgerun_hardware_signing::HardwareSigningError;
use edgerun_hardware_signing::{MeshSigner, NodeID};
use edgerun_mesh::router::MeshRouter;
use edgerun_mesh::{FrameType, LocalNode, MeshFrame, MeshFrameHeader};
use edgerun_mesh_capability::{
    sync::{Arc, Mutex},
    MeshCapabilityServer, MeshEnvelopeDispatcher,
};
#[cfg(target_os = "none")]
use edgerun_mesh_link::io;
use edgerun_mesh_link::MeshLink;
use edgerun_mesh_session::{HandshakeAccept, HandshakeInit, SessionError, SessionManager};
use edgerun_remote_capability::RemoteCapabilityProvider;
#[cfg(not(target_os = "none"))]
use libc::{c_int, pollfd, POLLIN};
#[cfg(not(target_os = "none"))]
use std::io;
#[cfg(not(target_os = "none"))]
use std::time::{Duration, Instant};

#[cfg(target_os = "none")]
mod bare_poll {
    #[allow(non_camel_case_types)]
    pub type c_int = i32;

    #[allow(non_camel_case_types)]
    #[derive(Clone, Copy, Debug, Default)]
    pub struct pollfd {
        pub fd: c_int,
        pub events: i16,
        pub revents: i16,
    }

    pub const POLLIN: i16 = 0x0001;

    pub unsafe fn poll(_fds: *mut pollfd, _nfds: usize, _timeout_ms: i32) -> c_int {
        0
    }
}

#[cfg(target_os = "none")]
mod bare_time {
    use core::ops::Sub;
    use core::time::Duration;

    #[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
    pub struct Instant;

    impl Instant {
        #[must_use]
        pub const fn now() -> Self {
            Self
        }

        #[must_use]
        pub const fn elapsed(&self) -> Duration {
            Duration::from_secs(0)
        }

        #[must_use]
        pub const fn duration_since(&self, _earlier: Instant) -> Duration {
            Duration::from_secs(0)
        }
    }

    impl Sub<Duration> for Instant {
        type Output = Instant;

        fn sub(self, _rhs: Duration) -> Self::Output {
            self
        }
    }
}

// Re-export types needed by integrators
pub use edgerun_mesh_capability::OutboundQueue;

/// Callback type for handling decrypted frames that are not capability envelopes.
/// Receives (sender NodeID, decrypted payload bytes).
pub type CommandHandler = Box<dyn FnMut(NodeID, Vec<u8>) + Send>;

/// Callback type for handling metrics frames from mesh peers.
/// Receives (sender NodeID, frame_type, payload bytes).
pub type MetricsHandler = Box<dyn FnMut(NodeID, FrameType, Vec<u8>) + Send>;

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
    pending_handshakes: HashMap<NodeID, edgerun_mesh_session::EphemeralSecret>,
    /// Optional callback for decrypted frames that are not capability envelopes.
    /// Used by the node to receive CommandEnvelope payloads over mesh.
    command_handler: Option<CommandHandler>,
    /// Optional callback for metrics frames (MetricsReport, MigrationOrder, MigrationComplete).
    metrics_handler: Option<MetricsHandler>,
    /// Shared stop flag — allows stopping the daemon from another thread.
    stop_signal: Arc<AtomicBool>,
}

impl<P: RemoteCapabilityProvider> MeshDaemon<P> {
    /// Creates a new mesh daemon with the given identity, configuration, and
    /// a capability provider that handles inbound capability requests.
    pub fn new(node_id: NodeID, config: MeshDaemonConfig, provider: P) -> Self {
        let mut router = MeshRouter::new(LocalNode::new(node_id));
        router.set_default_ttl(config.default_ttl);
        let heartbeat_interval = config.heartbeat_interval;
        let mut link = MeshLink::new();
        link.set_local_node_id(node_id);

        Self {
            link,
            router,
            server: MeshCapabilityServer::new(provider),
            sessions: SessionManager::new(node_id),
            config,
            last_heartbeat: Instant::now() - heartbeat_interval,
            running: false,
            signer: None,
            outbound: Arc::new(Mutex::new(VecDeque::new())),
            pending_handshakes: HashMap::new(),
            command_handler: None,
            metrics_handler: None,
            stop_signal: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Returns a shared stop flag that can be used to stop the daemon from
    /// another thread. Set it to `true` to signal the daemon to exit.
    pub fn stop_signal(&self) -> Arc<AtomicBool> {
        self.stop_signal.clone()
    }

    /// Replace the stop signal with a shared one, returning the previous one.
    /// Useful for sharing shutdown control with external code.
    pub fn with_stop_signal(mut self, stop_signal: Arc<AtomicBool>) -> Self {
        self.stop_signal = stop_signal;
        self
    }

    /// Sets a callback for decrypted frames that are not capability envelopes.
    ///
    /// This is used by the node to receive `CommandEnvelope` and `QueryRequest`
    /// payloads over mesh. The callback receives the sender's NodeID and the
    /// raw decrypted bytes.
    pub fn with_command_handler<F>(mut self, handler: F) -> Self
    where
        F: FnMut(NodeID, Vec<u8>) + Send + 'static,
    {
        self.command_handler = Some(Box::new(handler));
        self
    }

    /// Sets a callback for mesh metrics frames (MetricsReport, MigrationOrder, MigrationComplete).
    ///
    /// This is used by the scheduler to receive provider metrics reported over mesh.
    pub fn with_metrics_handler<F>(mut self, handler: F) -> Self
    where
        F: FnMut(NodeID, FrameType, Vec<u8>) + Send + 'static,
    {
        self.metrics_handler = Some(Box::new(handler));
        self
    }

    /// Returns the shared outbound queue for creating `MeshCapabilityTransport` instances.
    pub fn outbound_queue(&self) -> OutboundQueue {
        Arc::clone(&self.outbound)
    }

    /// Enables UDP multicast discovery so this node can find and be found by
    /// peers on the local network.
    pub fn enable_udp_broadcast(&mut self) -> Result<(), io::Error> {
        self.link.enable_udp_broadcast()
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
        self.signer
            .as_ref()
            .map(|s| s.node_id())
            .unwrap_or(NodeID([0u8; 64]))
    }

    // -----------------------------------------------------------------------
    // Interface setup
    // -----------------------------------------------------------------------

    /// Opens raw Ethernet sockets on all discovered UP interfaces.
    ///
    /// This enumerates interfaces from `/sys/class/net/` and opens
    /// a raw socket on each one that is UP (administratively).
    pub fn discover_and_open_interfaces(&mut self) -> Result<Vec<String>, io::Error> {
        #[cfg(target_os = "none")]
        {
            return Ok(Vec::new());
        }
        #[cfg(not(target_os = "none"))]
        {
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
                    edgerun_log::warn!("failed to open raw socket on {name}: {e}");
                    continue;
                }
                opened.push(name);
            }
            Ok(opened)
        }
    }

    /// Opens multicast discovery sockets on interfaces with configured IPv4 addresses.
    pub fn open_multicast_sockets(&mut self) -> Result<Vec<String>, io::Error> {
        let mut opened = Vec::new();
        for (&ifindex, &ipv4) in &self.config.interface_ipv4 {
            if let Err(e) = self.link.add_multicast(ipv4) {
                edgerun_log::warn!("failed to open multicast on ifindex {ifindex}: {e}");
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

            // Step 2: sign the exact mesh-frame signature input verified by MeshFrame.
            sign_mesh_frame_with_signer(signer.as_ref(), &mut frame)?;

            signed.push(frame);
        }
        Ok(signed)
    }

    // -----------------------------------------------------------------------
    // Event loop
    // -----------------------------------------------------------------------

    /// Runs the daemon's event loop until `stop()` is called or the shared
    /// stop signal is set to `true`.
    pub fn run(&mut self) -> Result<(), io::Error> {
        self.running = true;
        while self.running && !self.stop_signal.load(Ordering::Relaxed) {
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
        // No fds to poll — nothing to wait for, return immediately.
        // Caller controls the polling cadence.
        if fds.is_empty() {
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
        #[cfg(not(target_os = "none"))]
        let nfds = unsafe { libc::poll(poll_fds.as_mut_ptr(), poll_fds.len() as _, timeout_ms) };
        #[cfg(target_os = "none")]
        let nfds = unsafe { bare_poll::poll(poll_fds.as_mut_ptr(), poll_fds.len(), timeout_ms) };

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
                        if !handshake_init_matches_frame(&frame, &init) {
                            edgerun_log::warn!(
                                "dropping handshake init with mismatched frame source"
                            );
                            continue;
                        }
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
                                    signature: [0u8;
                                        edgerun_hardware_signing::MESH_SIGNATURE_LENGTH],
                                };
                                self.link.queue_frame(accept_frame);
                                self.drain_and_encrypt_buffered(&peer)?;
                                processed_any = true;
                            }
                            Err(e) => {
                                edgerun_log::warn!("handshake error: {e}");
                            }
                        }
                    }
                }
                FrameType::HandshakeAccept => {
                    if let Some(accept) = HandshakeAccept::decode(&frame.payload) {
                        if !handshake_accept_matches_frame(&frame, &accept) {
                            edgerun_log::warn!(
                                "dropping handshake accept with mismatched frame source"
                            );
                            continue;
                        }
                        let peer = accept.responder;
                        if let Some(secret) = self.pending_handshakes.remove(&peer) {
                            if self
                                .sessions
                                .complete_handshake_initiator(&accept, &secret)
                                .is_ok()
                            {
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
                            // Try decoding as capability envelope first
                            if let Ok(envelope) =
                                decode_capability_remote_envelope(decrypted.as_slice())
                            {
                                if let Some(inboxes) =
                                    self.server.dispatcher().inboxes_mut().get_mut(&sender)
                                {
                                    if let Some(inbox) = inboxes.first_mut() {
                                        inbox.push(envelope);
                                        processed_any = true;
                                    }
                                }
                            } else if let Some(ref mut handler) = self.command_handler {
                                // Not a capability envelope — dispatch to command handler
                                // (used by the node to receive CommandEnvelope/QueryRequest over mesh)
                                handler(sender, decrypted);
                                processed_any = true;
                            }
                        }
                        Err(SessionError::NoActiveSession) => {
                            // Peer sent encrypted data but we have no session — drop
                        }
                        Err(e) => {
                            edgerun_log::warn!("decrypt error from {sender:?}: {e}");
                        }
                    }
                }
                FrameType::Discovery | FrameType::RouteAdv => {
                    // Already processed by link.pump()
                }
                FrameType::MetricsReport
                | FrameType::MigrationOrder
                | FrameType::MigrationComplete => {
                    if let Some(ref mut handler) = self.metrics_handler {
                        let src = frame.header.src;
                        let ft = frame.header.frame_type;
                        handler(src, ft, frame.payload);
                        processed_any = true;
                    }
                }
                FrameType::Unknown(_) => {}
            }
        }

        // 3. Dispatch inbound capability envelopes through the server
        let dispatched = match self.server.serve_one(&mut self.link) {
            Ok(d) => d,
            Err(e) => {
                edgerun_log::warn!("capability dispatch error: {e}");
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
                edgerun_log::error!("signing error: {e}");
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
        let payloads: Vec<(NodeID, Vec<u8>)> = self.outbound.lock().unwrap().drain(..).collect();
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
                        signature: [0u8; edgerun_hardware_signing::MESH_SIGNATURE_LENGTH],
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
                    edgerun_log::warn!("encrypt error for {peer:?}: {e}");
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
                    signature: [0u8; edgerun_hardware_signing::MESH_SIGNATURE_LENGTH],
                };
                self.pending_handshakes.insert(peer, secret);
                self.link.queue_frame(handshake_frame);
            }
            // Buffer payloads for after handshake completes
            for payload in payloads {
                self.outbound.lock().unwrap().push_back((peer, payload));
            }
        }

        Ok(())
    }

    /// Drains buffered payloads for the given peer from the outbound queue
    /// and encrypts/sends them. Called after a handshake completes.
    fn drain_and_encrypt_buffered(&mut self, peer: &NodeID) -> Result<(), io::Error> {
        // Separate matching payloads from non-matching
        let mut queue = self.outbound.lock().unwrap();
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
                    signature: [0u8; edgerun_hardware_signing::MESH_SIGNATURE_LENGTH],
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
        let frame = self.router.build_discovery_frame();
        self.link.queue_frame(frame);

        let signed_frames = self
            .sign_pending_frames()
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;
        for frame in signed_frames {
            self.link.queue_frame(frame);
        }

        self.link.drain_pending_frames(&mut self.router).map(|_| ())
    }

    /// Ticks the heartbeat — increments missed counters and removes dead peers.
    pub fn tick_heartbeat(&mut self) {
        let dead = self.router.tick_heartbeat();
        for peer_id in &dead {
            edgerun_log::info!("peer {peer_id:?} is dead, removing routes");
            self.server.dispatcher().remove_peer(peer_id);
        }
    }

    /// Returns whether the daemon is currently running.
    #[must_use]
    pub fn is_running(&self) -> bool {
        self.running
    }
}

fn sign_mesh_frame_with_signer(
    signer: &dyn MeshSigner,
    frame: &mut MeshFrame,
) -> Result<(), HardwareSigningError> {
    let record_hash = edgerun_core::crypto::sha256(&frame.signed_preimage());
    let sig_input = edgerun_core::crypto::signature_input(
        edgerun_core::crypto::SIG_DOMAIN_MESH_FRAME,
        &record_hash,
    );
    frame.signature = signer.sign_message_var(&sig_input)?;
    Ok(())
}

fn handshake_init_matches_frame(frame: &MeshFrame, init: &HandshakeInit) -> bool {
    frame.header.src == init.initiator
}

fn handshake_accept_matches_frame(frame: &MeshFrame, accept: &HandshakeAccept) -> bool {
    frame.header.src == accept.responder
}

// -----------------------------------------------------------------------
// Helper: interface name → ifindex
// -----------------------------------------------------------------------

fn interface_name_to_ifindex(name: &str) -> Result<c_int, io::Error> {
    #[cfg(target_os = "none")]
    {
        let _ = name;
        return Err(io::Error::new(io::ErrorKind::Other));
    }
    #[cfg(not(target_os = "none"))]
    {
        let c_name = std::ffi::CString::new(name).map_err(|_| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "interface name contains null byte",
            )
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use edgerun_capabilities::CapabilityError;
    use edgerun_crypto::p256::ecdsa::signature::hazmat::PrehashSigner;
    use edgerun_crypto::p256::ecdsa::{Signature, SigningKey};
    use edgerun_crypto::p256::elliptic_curve::sec1::ToEncodedPoint;
    use edgerun_proto::edgerun::v0::capability::{
        CapabilityDescriptor, CapabilityGrant, CapabilityInvocation, CapabilityRequest,
        CapabilityRevocation,
    };
    use edgerun_proto::edgerun::v0::capability_runtime::{
        CapabilitySessionAccept, CapabilitySessionClose, CapabilitySessionOpen,
    };
    use edgerun_remote_capability::{RemoteCapabilityProvider, RemoteInvocationResult};

    struct TestProvider;
    impl RemoteCapabilityProvider for TestProvider {
        fn descriptor(&self) -> CapabilityDescriptor {
            CapabilityDescriptor::default()
        }
        fn open_session(
            &mut self,
            _open: &CapabilitySessionOpen,
        ) -> Result<CapabilitySessionAccept, CapabilityError> {
            Err(CapabilityError::Unsupported("test".into()))
        }
        fn invoke(
            &mut self,
            _: &[u8],
            _: &CapabilityInvocation,
            _: Option<&[u8]>,
        ) -> Result<RemoteInvocationResult, CapabilityError> {
            Err(CapabilityError::Unsupported("test".into()))
        }
        fn close_session(&mut self, _: &CapabilitySessionClose) -> Result<(), CapabilityError> {
            Ok(())
        }
        fn handle_request(
            &mut self,
            _: &CapabilityRequest,
        ) -> Result<Option<CapabilityGrant>, CapabilityError> {
            Ok(None)
        }
        fn handle_grant(&mut self, _: &CapabilityGrant) -> Result<(), CapabilityError> {
            Ok(())
        }
        fn handle_revocation(&mut self, _: &CapabilityRevocation) -> Result<(), CapabilityError> {
            Ok(())
        }
    }

    type TestDaemon = MeshDaemon<TestProvider>;

    fn node_id(v: u8) -> NodeID {
        let mut bytes = [0u8; 64];
        bytes[0] = v;
        NodeID(bytes)
    }

    struct TestMeshSigner {
        node_id: NodeID,
        signing_key: SigningKey,
    }

    impl TestMeshSigner {
        fn new() -> Self {
            let mut bytes = [0u8; 32];
            edgerun_crypto::fill_random(&mut bytes).expect("random generation failed");
            let signing_key = SigningKey::from_bytes(&bytes.into()).unwrap();
            let encoded = signing_key.verifying_key().to_encoded_point(false);
            let mut node_bytes = [0u8; 64];
            node_bytes.copy_from_slice(&encoded.as_bytes()[1..65]);
            Self {
                node_id: NodeID(node_bytes),
                signing_key,
            }
        }
    }

    impl MeshSigner for TestMeshSigner {
        fn node_id(&self) -> NodeID {
            self.node_id
        }

        fn sign_digest(
            &self,
            digest: &[u8; 32],
        ) -> Result<[u8; edgerun_hardware_signing::MESH_SIGNATURE_LENGTH], HardwareSigningError>
        {
            let sig: Signature = self
                .signing_key
                .sign_prehash(digest)
                .map_err(|_| HardwareSigningError::Provider("test signing failed".into()))?;
            let mut out = [0u8; edgerun_hardware_signing::MESH_SIGNATURE_LENGTH];
            out.copy_from_slice(&sig.to_bytes());
            Ok(out)
        }
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

    // --- MeshDaemonConfig ---

    #[test]
    fn config_default_heartbeat_interval() {
        let config = MeshDaemonConfig::default();
        assert_eq!(config.heartbeat_interval, Duration::from_secs(5));
    }

    #[test]
    fn config_default_ttl() {
        let config = MeshDaemonConfig::default();
        assert_eq!(config.default_ttl, 16);
    }

    #[test]
    fn config_default_interface_ipv4_is_empty() {
        let config = MeshDaemonConfig::default();
        assert!(config.interface_ipv4.is_empty());
    }

    #[test]
    fn config_clone() {
        let mut config = MeshDaemonConfig::default();
        config.interface_ipv4.insert(1, [192, 168, 1, 1]);
        let cloned = config.clone();
        assert_eq!(config.heartbeat_interval, cloned.heartbeat_interval);
        assert_eq!(config.default_ttl, cloned.default_ttl);
        assert_eq!(config.interface_ipv4, cloned.interface_ipv4);
    }

    #[test]
    fn config_debug_output() {
        let config = MeshDaemonConfig::default();
        let debug = format!("{:?}", config);
        assert!(debug.contains("MeshDaemonConfig"));
    }

    // --- Daemon creation with custom config ---

    #[test]
    fn daemon_with_custom_ttl() {
        let mut config = MeshDaemonConfig::default();
        config.default_ttl = 64;
        let daemon = TestDaemon::new(node_id(1), config, TestProvider);
        assert_eq!(daemon.router().default_ttl(), 64);
    }

    #[test]
    fn daemon_with_interface_ipv4() {
        let mut config = MeshDaemonConfig::default();
        config.interface_ipv4.insert(2, [10, 0, 0, 1]);
        let daemon = TestDaemon::new(node_id(1), config, TestProvider);
        assert_eq!(daemon.config.interface_ipv4.len(), 1);
        assert_eq!(daemon.config.interface_ipv4[&2], [10, 0, 0, 1]);
    }

    // --- Daemon lifecycle ---

    #[test]
    fn daemon_initially_not_running() {
        let daemon = TestDaemon::new(node_id(1), MeshDaemonConfig::default(), TestProvider);
        assert!(!daemon.is_running());
    }

    #[test]
    fn daemon_node_id_without_signer_is_zeros() {
        let daemon = TestDaemon::new(node_id(1), MeshDaemonConfig::default(), TestProvider);
        assert_eq!(daemon.node_id(), NodeID([0u8; 64]));
    }

    #[test]
    fn daemon_initializes_link_identity_from_configured_node() {
        let local = node_id(0x42);
        let daemon = TestDaemon::new(local, MeshDaemonConfig::default(), TestProvider);

        assert_eq!(daemon.link.local_node_id(), local);
    }

    #[test]
    fn daemon_outbound_queue_is_shared() {
        let daemon = TestDaemon::new(node_id(1), MeshDaemonConfig::default(), TestProvider);
        let q1 = daemon.outbound_queue();
        let q2 = daemon.outbound_queue();
        // Both point to the same Arc
        assert!(Arc::ptr_eq(&q1, &q2));
    }

    // --- Accessors ---

    #[test]
    fn daemon_router_mut_can_modify_ttl() {
        let mut daemon = TestDaemon::new(node_id(1), MeshDaemonConfig::default(), TestProvider);
        daemon.router_mut().set_default_ttl(42);
        assert_eq!(daemon.router().default_ttl(), 42);
    }

    #[test]
    fn daemon_dispatcher_is_accessible() {
        let mut daemon = TestDaemon::new(node_id(1), MeshDaemonConfig::default(), TestProvider);
        let _ = daemon.dispatcher();
    }

    #[test]
    fn daemon_server_mut_is_accessible() {
        let mut daemon = TestDaemon::new(node_id(1), MeshDaemonConfig::default(), TestProvider);
        let _ = daemon.server_mut();
    }

    #[test]
    fn daemon_sessions_mut_is_accessible() {
        let mut daemon = TestDaemon::new(node_id(1), MeshDaemonConfig::default(), TestProvider);
        let _ = daemon.sessions_mut();
    }

    // --- run_once with no fds ---

    #[test]
    fn daemon_run_once_with_no_fds_returns_false() {
        let mut daemon = TestDaemon::new(node_id(1), MeshDaemonConfig::default(), TestProvider);
        // No interfaces opened, so no fds
        let result = daemon.run_once(Duration::from_millis(10));
        assert!(result.is_ok());
        assert!(!result.unwrap());
    }

    // --- heartbeat ---

    #[test]
    fn daemon_heartbeat_toggles_running() {
        let mut daemon = TestDaemon::new(node_id(1), MeshDaemonConfig::default(), TestProvider);
        assert!(!daemon.is_running());
        daemon.running = true;
        daemon.stop();
        assert!(!daemon.is_running());
    }

    // --- broadcast_discovery (fails gracefully without sockets) ---

    #[test]
    fn daemon_broadcast_discovery_without_sockets() {
        let mut daemon = TestDaemon::new(node_id(1), MeshDaemonConfig::default(), TestProvider);
        // Without any sockets opened, broadcast_discovery may fail, but should not panic
        let result = daemon.broadcast_discovery();
        // On a system without the required capabilities, this will be an error; that's fine
        let _ = result;
    }

    #[test]
    fn sign_pending_frames_produces_verifiable_mesh_signature() {
        let signer = TestMeshSigner::new();
        let local_id = signer.node_id();
        let mut daemon = TestDaemon::new(local_id, MeshDaemonConfig::default(), TestProvider)
            .with_signer(Box::new(signer));
        let frame = MeshFrame {
            header: MeshFrameHeader {
                dest: node_id(0xBB),
                src: NodeID([0u8; 64]),
                ttl: 16,
                frame_type: FrameType::Data,
            },
            payload: b"payload".to_vec(),
            signature: [0u8; edgerun_hardware_signing::MESH_SIGNATURE_LENGTH],
        };
        daemon.link.queue_frame(frame);

        let signed = daemon.sign_pending_frames().unwrap();

        assert_eq!(signed.len(), 1);
        assert_eq!(signed[0].header.src, local_id);
        assert!(signed[0].verify_signature());
    }

    #[test]
    fn handshake_init_must_match_signed_frame_source() {
        let peer = node_id(0xAA);
        let other = node_id(0xBB);
        let init = HandshakeInit {
            initiator: peer,
            ephemeral_pub: [1u8; edgerun_mesh_session::ECDH_PUBLIC_KEY_SIZE],
        };
        let matching = MeshFrame {
            header: MeshFrameHeader {
                dest: node_id(0xCC),
                src: peer,
                ttl: 16,
                frame_type: FrameType::HandshakeInit,
            },
            payload: init.encode().to_vec(),
            signature: [0u8; edgerun_hardware_signing::MESH_SIGNATURE_LENGTH],
        };
        let mismatched = MeshFrame {
            header: MeshFrameHeader {
                src: other,
                ..matching.header
            },
            ..matching.clone()
        };

        assert!(handshake_init_matches_frame(&matching, &init));
        assert!(!handshake_init_matches_frame(&mismatched, &init));
    }

    #[test]
    fn handshake_accept_must_match_signed_frame_source() {
        let peer = node_id(0xAA);
        let other = node_id(0xBB);
        let accept = HandshakeAccept {
            responder: peer,
            ephemeral_pub: [2u8; edgerun_mesh_session::ECDH_PUBLIC_KEY_SIZE],
        };
        let matching = MeshFrame {
            header: MeshFrameHeader {
                dest: node_id(0xCC),
                src: peer,
                ttl: 16,
                frame_type: FrameType::HandshakeAccept,
            },
            payload: accept.encode().to_vec(),
            signature: [0u8; edgerun_hardware_signing::MESH_SIGNATURE_LENGTH],
        };
        let mismatched = MeshFrame {
            header: MeshFrameHeader {
                src: other,
                ..matching.header
            },
            ..matching.clone()
        };

        assert!(handshake_accept_matches_frame(&matching, &accept));
        assert!(!handshake_accept_matches_frame(&mismatched, &accept));
    }

    // --- tick_heartbeat ---

    #[test]
    fn daemon_tick_heartbeat_no_crash() {
        let mut daemon = TestDaemon::new(node_id(1), MeshDaemonConfig::default(), TestProvider);
        daemon.tick_heartbeat();
        // Should not panic even with no peers
    }

    // --- Error cases ---

    #[test]
    fn interface_name_to_ifindex_empty_name() {
        let err = interface_name_to_ifindex("").unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::NotFound);
    }

    #[test]
    fn interface_name_to_ifindex_very_long_name() {
        let long_name = "a".repeat(256);
        let err = interface_name_to_ifindex(&long_name).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::NotFound);
    }

    // --- TestProvider ---

    #[test]
    fn test_provider_descriptor_is_default() {
        let provider = TestProvider;
        let desc = provider.descriptor();
        // Default descriptor should have empty/zero fields
        assert!(desc.capability_id.is_empty());
    }

    #[test]
    fn test_provider_open_session_returns_unsupported() {
        use edgerun_proto::edgerun::v0::capability_runtime::CapabilitySessionOpen;
        let mut provider = TestProvider;
        let open = CapabilitySessionOpen::default();
        let err = provider.open_session(&open).unwrap_err();
        assert!(matches!(err, CapabilityError::Unsupported(_)));
    }

    #[test]
    fn test_provider_invoke_returns_unsupported() {
        use edgerun_proto::edgerun::v0::capability::CapabilityInvocation;
        let mut provider = TestProvider;
        let invocation = CapabilityInvocation::default();
        let result = provider.invoke(&[], &invocation, None);
        assert!(result.is_err());
    }

    #[test]
    fn test_provider_close_session_returns_ok() {
        use edgerun_proto::edgerun::v0::capability_runtime::CapabilitySessionClose;
        let mut provider = TestProvider;
        let close = CapabilitySessionClose::default();
        assert!(provider.close_session(&close).is_ok());
    }

    #[test]
    fn test_provider_handle_request_returns_none() {
        use edgerun_proto::edgerun::v0::capability::CapabilityRequest;
        let mut provider = TestProvider;
        let req = CapabilityRequest::default();
        let result = provider.handle_request(&req).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_provider_handle_grant_returns_ok() {
        use edgerun_proto::edgerun::v0::capability::CapabilityGrant;
        let mut provider = TestProvider;
        let grant = CapabilityGrant::default();
        assert!(provider.handle_grant(&grant).is_ok());
    }

    #[test]
    fn test_provider_handle_revocation_returns_ok() {
        use edgerun_proto::edgerun::v0::capability::CapabilityRevocation;
        let mut provider = TestProvider;
        let rev = CapabilityRevocation::default();
        assert!(provider.handle_revocation(&rev).is_ok());
    }

    // --- node_id helper ---

    #[test]
    fn node_id_helper_sets_first_byte() {
        let id = node_id(42);
        assert_eq!(id.0[0], 42);
        for i in 1..64 {
            assert_eq!(id.0[i], 0);
        }
    }

    #[test]
    fn node_id_helper_different_values() {
        let a = node_id(1);
        let b = node_id(2);
        assert_ne!(a, b);
    }

    // --- Config with populated interface_ipv4 ---

    #[test]
    fn config_with_multiple_interfaces() {
        let mut config = MeshDaemonConfig::default();
        config.interface_ipv4.insert(1, [192, 168, 0, 1]);
        config.interface_ipv4.insert(2, [10, 0, 0, 1]);
        config.interface_ipv4.insert(3, [172, 16, 0, 1]);
        assert_eq!(config.interface_ipv4.len(), 3);
    }
}

trait NativeCapabilityRemoteEnvelopeEncode {
    fn native_encode_to_vec(&self) -> Vec<u8>;
}

impl NativeCapabilityRemoteEnvelopeEncode for CapabilityRemoteEnvelope {
    fn native_encode_to_vec(&self) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(b"ERMD");
        out.push(if self.message.is_some() { 1 } else { 0 });
        out
    }
}

fn decode_capability_remote_envelope(
    _bytes: &[u8],
) -> Result<CapabilityRemoteEnvelope, &'static str> {
    Ok(CapabilityRemoteEnvelope { message: None })
}
