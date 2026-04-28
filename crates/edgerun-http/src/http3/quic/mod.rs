//! QUIC transport protocol (RFC 9000)

pub mod crypto {
    pub use edgerun_quic::crypto::*;
}

pub mod frame {
    pub use edgerun_quic::frame::*;
}

pub mod handshake {
    pub use edgerun_quic::handshake::*;
}

pub mod handshake_unified {
    pub use edgerun_quic::handshake_unified::*;
}

pub mod packet {
    pub use edgerun_quic::packet::*;
}

pub mod server_handshake {
    pub use edgerun_quic::server_handshake::*;
}

pub mod transport {
    pub use edgerun_quic::transport::*;
}

pub mod types {
    pub use edgerun_quic::types::*;
}

pub use edgerun_quic::types::INITIAL_SALT_V1;
pub use edgerun_quic::{
    get_long_header_payload_offset, ConnectionId, HandshakeResult, PacketNumberSpace,
    PacketProtection, PacketType, ProtectionKeys, QuicCrypto, QuicFrame, QuicPacket,
    QuicTlsHandshaker, QuicTlsServerHandshaker, QuicTransport, ServerHandshakeResult,
    TransportParameters, QUIC_VERSION_V1,
};

use alloc::borrow::ToOwned;
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec;
use alloc::vec::Vec;
use crypto::{CryptoPhase, ProtectionKeys as ProtKeys};

use crate::runtime::net::{IpAddr, SocketAddr, ToSocketAddrs, UdpSocket};
use crate::runtime::sync::Arc;
use crate::runtime::AsyncUdpSocket;
use edgerun_crypto::CipherSuite;

/// Client-side QUIC connection options.
#[derive(Debug, Clone, Copy, Default)]
pub struct QuicConnectOptions {
    /// Permit the current incomplete QUIC certificate verifier.
    ///
    /// This is only appropriate for same-stack tests or local development with
    /// explicitly trusted endpoints.
    pub accept_invalid_certs: bool,
}

/// QUIC connection
pub struct QuicConnection {
    /// UDP socket — async for client path, dummy/wrapped for tests
    socket: Arc<AsyncUdpSocket>,
    /// Server address (hostname:port)
    server_addr: String,
    /// Transport layer
    transport: QuicTransport,
    /// Crypto layer
    crypto: QuicCrypto,
    /// Packet protection (application-level, after handshake)
    protection: Option<crypto::PacketProtection>,
    /// Handshake-level protection (valid during handshake)
    hs_protection: Option<crypto::PacketProtection>,
    /// Initial-level protection (valid during handshake)
    initial_protection: Option<crypto::PacketProtection>,
    /// 0-RTT early data protection (for early data sending)
    early_data_protection: Option<crypto::PacketProtection>,
    /// Connection established
    established: bool,
    /// Early data (0-RTT) was sent on this connection
    early_data_sent: bool,
    /// Receive buffer
    recv_buffer: Vec<u8>,
    /// Offset into recv_buffer for partial reads
    recv_offset: usize,
    /// Server destination connection ID (used for Initial key derivation)
    server_dcid: ConnectionId,
    /// Current key phase (0 or 1, toggles on each update — RFC 9001 §6)
    key_phase: bool,
    /// Current peer key phase for incoming 1-RTT packets.
    peer_key_phase: bool,
    /// Previous protection keys (for decrypting in-flight packets during key transition)
    prev_protection: Option<crypto::PacketProtection>,
    /// Client application traffic secret (for key updates — RFC 9001 §6)
    client_app_traffic_secret: Vec<u8>,
    /// Server application traffic secret (for key updates — RFC 9001 §6)
    server_app_traffic_secret: Vec<u8>,
    /// Hash algorithm matching the cipher suite (for HKDF-Expand-Label)
    cipher_suite_hash: edgerun_tls::prf::Hasher,
    /// Next send offset per stream (for STREAM frame fragmentation)
    stream_send_offset: alloc::collections::BTreeMap<u64, u64>,
    /// Active path for connection migration (local_addr, remote_addr)
    /// Set when the first packet is received from the peer
    active_path: Option<(
        crate::runtime::net::SocketAddr,
        crate::runtime::net::SocketAddr,
    )>,
    /// Pending migration path challenges (data → deadline)
    pending_path_challenges: alloc::collections::BTreeMap<[u8; 8], crate::runtime::time::Instant>,
    /// Captured sent packets (for integration testing)
    sent_packets_buffer: Vec<Vec<u8>>,
}

impl QuicConnection {
    /// Create a client connection and perform full QUIC + TLS 1.3 handshake.
    ///
    /// This performs the complete handshake:
    /// 1. Derive Initial keys from well-known salt + server DCID
    /// 2. Send ClientHello in CRYPTO frame (Initial level)
    /// 3. Receive ServerHello + encrypted handshake messages
    /// 4. Verify server's Finished
    /// 5. Send client Finished
    /// 6. Derive application traffic keys
    ///
    /// Resolves `server` hostname via DNS, binds a UDP socket to a random
    /// local port, and returns the connection ready for HTTP/3 data transfer.
    pub async fn connect(server: &str) -> Result<Self, String> {
        Self::connect_with_options(server, QuicConnectOptions::default()).await
    }

    pub async fn connect_with_options(
        server: &str,
        options: QuicConnectOptions,
    ) -> Result<Self, String> {
        edgerun_log::debug!("CLIENT QUIC: connect({})", server);
        // Resolve hostname to IP address
        let (server_host, server_port) = if let Ok(addr) = server.parse::<SocketAddr>() {
            edgerun_log::debug!("CLIENT QUIC: parsed as SocketAddr");
            (addr.ip().to_owned(), addr.port())
        } else if let Ok(ip) = server.parse::<IpAddr>() {
            edgerun_log::debug!("CLIENT QUIC: parsed as IpAddr");
            (ip, 443)
        } else {
            edgerun_log::debug!("CLIENT QUIC: resolving {}...", server);
            let resolved = Self::resolve_host(server).await?;
            edgerun_log::debug!("CLIENT QUIC: resolved to {}", resolved);
            (resolved, 443)
        };

        // Bind UDP socket
        let socket = Arc::new(
            crate::runtime::bind_udp_socket("0.0.0.0:0")
                .map_err(|e| format!("Failed to bind UDP socket: {}", e))?,
        );

        edgerun_log::debug!("CLIENT QUIC: Bound UDP socket");

        let local_cid = ConnectionId::random();
        let remote_cid = ConnectionId::random();
        let transport = QuicTransport::new(local_cid.clone(), remote_cid.clone());
        let crypto = QuicCrypto::new();

        let remote_addr = SocketAddr::new(server_host, server_port);

        let mut conn = QuicConnection {
            socket,
            server_addr: remote_addr.to_string(),
            transport,
            crypto,
            protection: None,
            hs_protection: None,
            initial_protection: None,
            early_data_protection: None,
            early_data_sent: false,
            established: false,
            recv_buffer: Vec::new(),
            recv_offset: 0,
            server_dcid: remote_cid.clone(),
            key_phase: false,
            peer_key_phase: false,
            prev_protection: None,
            client_app_traffic_secret: Vec::new(),
            server_app_traffic_secret: Vec::new(),
            cipher_suite_hash: edgerun_tls::prf::Hasher::Sha256,
            stream_send_offset: alloc::collections::BTreeMap::new(),
            active_path: None,
            pending_path_challenges: alloc::collections::BTreeMap::new(),
            sent_packets_buffer: Vec::new(),
        };

        conn.do_handshake(options).await?;

        // Initialize active path
        if let Ok(local) = conn.socket.local_addr() {
            conn.active_path = Some((local, remote_addr));
        }

        Ok(conn)
    }

    /// Resolve hostname to IP address.
    async fn resolve_host(host: &str) -> Result<IpAddr, String> {
        if let Ok(ip) = host.parse::<IpAddr>() {
            return Ok(ip);
        }
        if let Ok(addrs) = (host, 443).to_socket_addrs() {
            let mut first_ip = None;
            for addr in addrs {
                let ip = addr.ip();
                if ip.is_ipv4() {
                    return Ok(ip);
                }
                first_ip.get_or_insert(ip);
            }
            if let Some(ip) = first_ip {
                return Ok(ip);
            }
        }
        Err(format!("DNS resolution failed for {}", host))
    }

    /// Perform the QUIC + TLS 1.3 handshake.
    async fn do_handshake(&mut self, options: QuicConnectOptions) -> Result<(), String> {
        let server_name = self
            .server_addr
            .split(':')
            .next()
            .unwrap_or(&self.server_addr);
        let mut handshaker = handshake::QuicTlsHandshaker::new(server_name);
        handshaker.allow_unverified_certificates(options.accept_invalid_certs);

        // ── Step 1: Derive Initial keys ──────────────────────────────
        let dcid = self.server_dcid.as_bytes().to_vec();
        let initial_keys = handshaker.initial_keys(&dcid);

        let initial_protection_write = crypto::PacketProtection::new(&initial_keys);
        self.initial_protection = Some(initial_protection_write);

        // ── Step 2: Send ClientHello in Initial packet ───────────────
        let crypto_data = handshaker.initial_crypto_data();
        let crypto_frame = QuicFrame::Crypto {
            offset: 0,
            data: crypto_data.to_vec(),
        };
        edgerun_log::debug!("CLIENT: Sending ClientHello...");
        self.send_initial_frame(crypto_frame).await?;
        edgerun_log::debug!("CLIENT: Waiting for server Initial...");

        // ── Step 3: Receive server's Initial packet ──────────────────
        let server_initial = self.recv_packet().await?;
        edgerun_log::debug!("CLIENT: Got server packet, decrypting...");
        let decrypted_initial = self.decrypt_packet_initial(&server_initial)?;
        edgerun_log::debug!("CLIENT: Decrypted, parsing CRYPTO frame...");

        // Parse CRYPTO frame from decrypted payload
        let (server_crypto_data, _) =
            super::crypto_frame::parse_crypto_frame(&decrypted_initial)
                .ok_or_else(|| "No CRYPTO frame in server Initial packet".to_string())?;

        // Feed ServerHello to handshaker
        handshaker.process_initial_crypto(&server_crypto_data)?;

        // ── Step 4: Derive Handshake keys ────────────────────────────
        let hs_keys = handshaker.handshake_keys()?;
        let hs_protection = crypto::PacketProtection::new(&hs_keys);
        self.hs_protection = Some(hs_protection);

        // ── Step 5: Receive Handshake-level packets ──────────────────
        // Server sends EncryptedExtensions, Certificate, CertificateVerify, Finished
        // These may come in one or multiple packets.
        let mut all_handshake_crypto = Vec::new();

        // Try to receive handshake data (with async I/O)
        for _ in 0..10 {
            match self.recv_packet().await {
                Ok(pkt) => {
                    let decrypted = self.decrypt_packet_handshake(&pkt)?;
                    if let Some((data, _)) = super::crypto_frame::parse_crypto_frame(&decrypted) {
                        all_handshake_crypto.extend_from_slice(&data);
                        break;
                    }
                }
                Err(e) if e.contains("would block") || e.contains("no data") => {
                    if !all_handshake_crypto.is_empty() {
                        break;
                    }
                    crate::runtime::sleep(crate::runtime::time::Duration::from_millis(10)).await;
                }
                Err(e) => return Err(e),
            }
        }

        if all_handshake_crypto.is_empty() {
            return Err("No handshake CRYPTO data received from server".into());
        }

        // Process handshake messages (verify Finished, build client Finished)
        let client_finished = handshaker.process_handshake_crypto(&all_handshake_crypto)?;

        // ── Step 6: Send client Finished ─────────────────────────────
        let client_finished_frame = QuicFrame::Crypto {
            offset: 0,
            data: client_finished.clone(),
        };
        self.send_handshake_frame(client_finished_frame).await?;

        // Update transcript with client Finished (needed for app key derivation)
        let mut transcript_after_finished = handshaker.transcript().to_vec();
        transcript_after_finished.extend_from_slice(&client_finished);

        // ── Step 7: Derive Application keys ──────────────────────────
        let app_keys = handshaker.app_keys(&transcript_after_finished);
        let app_protection = crypto::PacketProtection::new(&app_keys);
        self.protection = Some(app_protection);

        // Store app traffic secrets for future key updates (RFC 9001 §6)
        self.client_app_traffic_secret =
            handshaker.client_app_traffic_secret(&transcript_after_finished);
        self.server_app_traffic_secret =
            handshaker.server_app_traffic_secret(&transcript_after_finished);
        self.cipher_suite_hash = handshaker.hasher().clone();

        handshaker.mark_complete();
        self.established = true;

        // Initialize the active path for connection migration (RFC 9000 §9)
        self.initialize_active_path();

        // Enable 0-RTT early data if keys were derived
        if let Some(early_keys) = handshaker.early_data_keys() {
            self.enable_early_data(early_keys);
        }

        Ok(())
    }

    /// Send a CRYPTO frame in an Initial packet (unprotected header + encrypted payload).
    async fn send_initial_frame(&mut self, frame: QuicFrame) -> Result<(), String> {
        let payload = frame.to_bytes();

        let pn = self
            .transport
            .next_packet_number(PacketNumberSpace::Initial);

        let mut pkt = QuicPacket::initial(
            QUIC_VERSION_V1,
            self.transport.remote_cid.as_bytes().to_vec(),
            self.transport.local_cid.as_bytes().to_vec(),
            vec![],
            pn,
            payload,
        );

        pad_initial_datagram(&mut pkt);

        let encrypted = pkt.payload.clone();
        let aad = pkt.header_to_bytes_aad_with_payload_len(encrypted.len() + 16);

        let send_bytes = if let Some(ref mut prot) = self.initial_protection {
            prot.protect_with_packet_number(pkt.header.packet_number, &aad, &encrypted)
                .map_err(|e| format!("Initial encrypt failed: {}", e))?
        } else {
            return Err("No Initial protection keys".into());
        };

        let mut full_packet = aad;
        full_packet.extend_from_slice(&send_bytes);
        let pn_offset =
            packet::get_packet_number_offset(&full_packet, self.transport.remote_cid.len())?;
        if let Some(ref prot) = self.initial_protection {
            prot.protect_header(&mut full_packet, pn_offset, pkt.header.pn_length)
                .map_err(|e| format!("Initial header protection failed: {}", e))?;
        }

        let addr: SocketAddr = self
            .server_addr
            .parse()
            .map_err(|e| format!("Invalid server address: {}", e))?;
        self.socket
            .send_to(&full_packet, addr)
            .await
            .map_err(|e| format!("UDP send failed: {}", e))?;

        self.transport.record_packet_sent(
            PacketNumberSpace::Initial,
            pkt.header.packet_number,
            full_packet.len(),
            true,
        );
        self.sent_packets_buffer.push(full_packet);
        self.transport.update_activity();
        Ok(())
    }

    /// Send a CRYPTO frame in a Handshake-level packet.
    async fn send_handshake_frame(&mut self, frame: QuicFrame) -> Result<(), String> {
        let payload = frame.to_bytes();
        let pn = self
            .transport
            .next_packet_number(PacketNumberSpace::Handshake);

        let pkt = QuicPacket {
            header: packet::QuicPacketHeader {
                packet_type: PacketType::Handshake,
                version: QUIC_VERSION_V1,
                dst_cid: self.transport.remote_cid.as_bytes().to_vec(),
                src_cid: self.transport.local_cid.as_bytes().to_vec(),
                token: Vec::new(),
                pn_length: 4,
                packet_number: pn,
                key_phase: false,
                payload_length: payload.len(),
            },
            payload,
        };

        let aad = pkt.header_to_bytes_aad_with_payload_len(pkt.payload.len() + 16);

        let send_bytes = if let Some(ref mut prot) = self.hs_protection {
            prot.protect_with_packet_number(pkt.header.packet_number, &aad, &pkt.payload)
                .map_err(|e| format!("Handshake encrypt failed: {}", e))?
        } else {
            return Err("No Handshake protection keys".into());
        };

        let mut full_packet = aad;
        full_packet.extend_from_slice(&send_bytes);
        let pn_offset =
            packet::get_packet_number_offset(&full_packet, self.transport.remote_cid.len())?;
        if let Some(ref prot) = self.hs_protection {
            prot.protect_header(&mut full_packet, pn_offset, pkt.header.pn_length)
                .map_err(|e| format!("Handshake header protection failed: {}", e))?;
        }

        let addr: SocketAddr = self
            .server_addr
            .parse()
            .map_err(|e| format!("Invalid server address: {}", e))?;
        self.socket
            .send_to(&full_packet, addr)
            .await
            .map_err(|e| format!("UDP send failed: {}", e))?;

        self.transport.record_packet_sent(
            PacketNumberSpace::Handshake,
            pkt.header.packet_number,
            full_packet.len(),
            true,
        );
        self.sent_packets_buffer.push(full_packet);
        self.transport.update_activity();
        Ok(())
    }

    /// Receive a raw QUIC packet from the UDP socket.
    async fn recv_packet(&mut self) -> Result<QuicPacket, String> {
        let mut buf = [0u8; 4096];
        let (n, _) = self
            .socket
            .recv_from(&mut buf)
            .await
            .map_err(|e| format!("UDP recv failed: {}", e))?;
        self.recv_buffer = buf[..n].to_vec();
        self.recv_offset = 0;

        let recv_buffer = self.recv_buffer.clone();
        let packet_bytes = self.unprotect_received_header(&recv_buffer)?;
        let (packet, _consumed) = QuicPacket::from_bytes(&packet_bytes)
            .map_err(|e| format!("Packet parse error: {}", e))?;

        Ok(packet)
    }

    /// Decrypt an Initial-level packet payload.
    fn decrypt_packet_initial(&mut self, pkt: &QuicPacket) -> Result<Vec<u8>, String> {
        if let Some(ref mut prot) = self.initial_protection {
            // AAD = unprotected packet header (RFC 9001 §5.2)
            let aad = pkt.header_to_bytes_aad();
            prot.unprotect(&aad, pkt.header.packet_number, &pkt.payload)
                .map_err(|e| format!("Initial decrypt failed: {}", e))
        } else {
            Err("No Initial protection keys for decryption".into())
        }
    }

    /// Decrypt a Handshake-level packet payload.
    fn decrypt_packet_handshake(&mut self, pkt: &QuicPacket) -> Result<Vec<u8>, String> {
        if let Some(ref mut prot) = self.hs_protection {
            // AAD = unprotected packet header (RFC 9001 §5.2)
            let aad = pkt.header_to_bytes_aad();
            prot.unprotect(&aad, pkt.header.packet_number, &pkt.payload)
                .map_err(|e| format!("Handshake decrypt failed: {}", e))
        } else {
            Err("No Handshake protection keys for decryption".into())
        }
    }

    /// Create a dummy connection for testing
    pub fn dummy() -> Self {
        let udp = UdpSocket::bind("127.0.0.1:0").expect("Cannot bind test socket");
        let socket =
            Arc::new(crate::runtime::wrap_udp_socket(udp).expect("Cannot wrap test socket"));
        let local_cid = ConnectionId::random();
        let remote_cid = ConnectionId::random();
        let transport = QuicTransport::new(local_cid.clone(), remote_cid.clone());

        QuicConnection {
            socket,
            server_addr: "dummy".to_string(),
            transport,
            crypto: QuicCrypto::new(),
            protection: None,
            hs_protection: None,
            initial_protection: None,
            established: false,
            recv_buffer: Vec::new(),
            recv_offset: 0,
            server_dcid: remote_cid,
            early_data_protection: None,
            early_data_sent: false,
            key_phase: false,
            peer_key_phase: false,
            prev_protection: None,
            client_app_traffic_secret: Vec::new(),
            server_app_traffic_secret: Vec::new(),
            cipher_suite_hash: edgerun_tls::prf::Hasher::Sha256,
            stream_send_offset: alloc::collections::BTreeMap::new(),
            active_path: None,
            pending_path_challenges: alloc::collections::BTreeMap::new(),
            sent_packets_buffer: Vec::new(),
        }
    }

    /// Check if connection is established
    pub fn is_established(&self) -> bool {
        self.established
    }

    /// Create a QUIC connection for testing with a pre-bound socket.
    ///
    /// The connection is marked as `established` with no protection keys,
    /// suitable for testing 1-RTT packet construction without encryption.
    pub fn from_established_test(
        socket: UdpSocket,
        target: crate::runtime::net::SocketAddr,
    ) -> Self {
        let socket =
            Arc::new(crate::runtime::wrap_udp_socket(socket).expect("Cannot wrap test socket"));
        let local_cid = ConnectionId::random();
        let remote_cid = ConnectionId::random();
        let transport = QuicTransport::new(local_cid.clone(), remote_cid.clone());

        QuicConnection {
            socket,
            server_addr: target.to_string(),
            transport,
            crypto: QuicCrypto::new(),
            protection: None,
            hs_protection: None,
            initial_protection: None,
            established: true,
            recv_buffer: Vec::new(),
            recv_offset: 0,
            server_dcid: remote_cid,
            early_data_protection: None,
            early_data_sent: false,
            key_phase: false,
            peer_key_phase: false,
            prev_protection: None,
            client_app_traffic_secret: Vec::new(),
            server_app_traffic_secret: Vec::new(),
            cipher_suite_hash: edgerun_tls::prf::Hasher::Sha256,
            stream_send_offset: alloc::collections::BTreeMap::new(),
            active_path: None,
            pending_path_challenges: alloc::collections::BTreeMap::new(),
            sent_packets_buffer: Vec::new(),
        }
    }

    /// Create a server-side connection from an established handshake result.
    ///
    /// Called after the server-side TLS handshake completes successfully.
    /// The `handshake_result` contains all derived protection keys.
    pub fn from_server(
        socket: crate::runtime::net::UdpSocket,
        client_addr: String,
        client_dcid: ConnectionId,
        client_scid: ConnectionId,
        handshake_result: crate::http3::quic::server_handshake::ServerHandshakeResult,
    ) -> Result<Self, String> {
        let socket = Arc::new(
            crate::runtime::wrap_udp_socket(socket)
                .map_err(|e| format!("Cannot wrap socket: {}", e))?,
        );
        Self::from_server_socket(
            socket,
            client_addr,
            client_dcid,
            client_scid,
            handshake_result,
        )
    }

    /// Create a server-side connection using an already-bound async UDP socket.
    pub fn from_server_socket(
        socket: Arc<AsyncUdpSocket>,
        client_addr: String,
        client_dcid: ConnectionId,
        client_scid: ConnectionId,
        handshake_result: crate::http3::quic::server_handshake::ServerHandshakeResult,
    ) -> Result<Self, String> {
        let transport = QuicTransport::new(client_dcid.clone(), client_scid.clone());

        let mut conn = QuicConnection {
            socket,
            server_addr: client_addr,
            transport,
            crypto: QuicCrypto::new(),
            protection: None,
            hs_protection: None,
            initial_protection: None,
            established: true,
            recv_buffer: Vec::new(),
            recv_offset: 0,
            server_dcid: client_dcid,
            early_data_protection: None,
            early_data_sent: false,
            key_phase: false,
            peer_key_phase: false,
            prev_protection: None,
            client_app_traffic_secret: Vec::new(),
            server_app_traffic_secret: Vec::new(),
            cipher_suite_hash: edgerun_tls::prf::Hasher::Sha256,
            stream_send_offset: alloc::collections::BTreeMap::new(),
            active_path: None,
            pending_path_challenges: alloc::collections::BTreeMap::new(),
            sent_packets_buffer: Vec::new(),
        };

        // Set up application-level protection keys
        let app_protection = crypto::PacketProtection::new(&handshake_result.app_keys);
        conn.protection = Some(app_protection);

        Ok(conn)
    }

    /// Get server name
    pub fn server_name(&self) -> &str {
        &self.server_addr
    }

    /// Send data on a stream with automatic fragmentation.
    ///
    /// If `data` exceeds the MTU, it is split into multiple STREAM frames
    /// with proper offsets. Only the final chunk (or the only chunk) sets
    /// the `fin` bit. Each chunk is sent in a separate QUIC packet.
    ///
    /// The stream offset is tracked per stream across calls.
    pub async fn send_stream_data(
        &mut self,
        stream_id: u64,
        data: &[u8],
        fin: bool,
    ) -> Result<(), String> {
        if data.is_empty() && !fin {
            return Ok(());
        }

        const MAX_STREAM_PAYLOAD: usize = 1170;
        let mut offset = self
            .stream_send_offset
            .get(&stream_id)
            .copied()
            .unwrap_or(0);
        let total_len = data.len();
        let mut chunks_sent = 0;
        while chunks_sent * MAX_STREAM_PAYLOAD < total_len {
            let start = chunks_sent * MAX_STREAM_PAYLOAD;
            let end = (start + MAX_STREAM_PAYLOAD).min(total_len);
            let chunk = &data[start..end];
            let is_last = end == total_len && fin;

            let frame = QuicFrame::Stream {
                stream_id,
                offset,
                fin: is_last,
                data: chunk.to_vec(),
            };
            self.send_frame(frame).await?;

            offset += chunk.len() as u64;
            chunks_sent += 1;
        }

        self.stream_send_offset.insert(stream_id, offset);
        Ok(())
    }

    /// Send RESET_STREAM to abort a stream (RFC 9000 §4.5).
    pub async fn send_reset_stream(
        &mut self,
        stream_id: u64,
        error_code: u64,
    ) -> Result<(), String> {
        let frame = QuicFrame::ResetStream {
            stream_id,
            error_code,
            final_size: 0,
        };
        self.send_frame(frame).await
    }

    /// Send STOP_SENDING to tell peer to stop sending (RFC 9000 §4.6).
    pub async fn send_stop_sending(
        &mut self,
        stream_id: u64,
        error_code: u64,
    ) -> Result<(), String> {
        let frame = QuicFrame::StopSending {
            stream_id,
            error_code,
        };
        self.send_frame(frame).await
    }

    /// Send a single QUIC frame
    ///
    /// Builds the appropriate packet header based on connection state:
    /// - Before handshake: long header (Initial) — used during handshake
    /// - After handshake: short header (1-RTT) — used for application data
    pub async fn send_frame(&mut self, frame: QuicFrame) -> Result<(), String> {
        let pn = self
            .transport
            .next_packet_number(PacketNumberSpace::ApplicationData);
        let payload = frame.to_bytes();

        let mut packet = if self.established {
            QuicPacket::one_rtt_with_key_phase(
                self.transport.remote_cid.as_bytes().to_vec(),
                pn,
                self.key_phase,
                payload,
            )
        } else {
            QuicPacket::initial(
                QUIC_VERSION_V1,
                self.transport.remote_cid.as_bytes().to_vec(),
                self.transport.local_cid.as_bytes().to_vec(),
                vec![],
                pn,
                payload,
            )
        };
        if self.protection.is_some() {
            packet.header.pn_length = 4;
        }

        let send_bytes = if let Some(ref mut prot) = self.protection {
            let aad = packet.header_to_bytes_aad();
            let encrypted = prot
                .protect_with_packet_number(packet.header.packet_number, &aad, &packet.payload)
                .map_err(|e| format!("Packet protection failed: {}", e))?;
            let mut packet_bytes = aad;
            packet_bytes.extend_from_slice(&encrypted);
            let pn_offset =
                packet::get_packet_number_offset(&packet_bytes, self.transport.remote_cid.len())?;
            prot.protect_header(&mut packet_bytes, pn_offset, packet.header.pn_length)
                .map_err(|e| format!("Header protection failed: {}", e))?;
            packet_bytes
        } else {
            packet.to_bytes()
        };

        self.sent_packets_buffer.push(send_bytes.clone());

        let addr: SocketAddr = self.server_addr.parse().unwrap_or_else(|_| {
            format!("{}:443", self.server_addr)
                .parse()
                .expect("server_addr must be parseable as SocketAddr")
        });
        self.socket
            .send_to(&send_bytes, addr)
            .await
            .map_err(|e| format!("UDP send failed: {}", e))?;

        self.transport.record_packet_sent(
            packet_number_space(packet.header.packet_type),
            packet.header.packet_number,
            send_bytes.len(),
            false,
        );
        self.transport.update_activity();
        Ok(())
    }

    /// Receive data, returning (stream_id, data, fin)
    ///
    /// Reads from the UDP socket, decrypts 1-RTT packets, parses QUIC frames,
    /// and returns the first STREAM frame found.
    pub async fn recv_stream_data(&mut self) -> Result<Option<(u64, Vec<u8>, bool)>, String> {
        loop {
            if self.recv_buffer.is_empty() || self.recv_offset >= self.recv_buffer.len() {
                let mut buf = [0u8; 65536];
                let (n, _) = self
                    .socket
                    .recv_from(&mut buf)
                    .await
                    .map_err(|e| format!("UDP recv failed: {}", e))?;
                self.recv_buffer = buf[..n].to_vec();
                self.recv_offset = 0;
                self.transport.update_activity();
            }

            if let Some(stream_data) = self.recv_from_buffer()? {
                return Ok(Some(stream_data));
            }

            self.recv_buffer.clear();
            self.recv_offset = 0;
        }
    }

    /// Inject a raw received packet (for testing without UDP sockets).
    /// The packet should already be encrypted with application traffic keys.
    pub fn inject_packet(&mut self, data: Vec<u8>) {
        self.recv_buffer = data;
        self.recv_offset = 0;
    }

    /// Get sent data for a stream (for testing — returns last sent stream payload).
    ///
    /// In a real UDP flow, this data would have been sent over the network.
    /// For integration tests, we capture it here to inject into the peer.
    pub fn get_sent_data(&self, _stream_id: u64) -> Result<Vec<u8>, String> {
        if self.sent_packets_buffer.is_empty() {
            return Err("No sent data available".into());
        }
        // Return the last sent packet (most recent)
        Ok(self.sent_packets_buffer.last().cloned().unwrap())
    }

    /// Get sent control stream data (for testing GOAWAY flow).
    pub fn get_sent_control_data(&self) -> Result<Vec<u8>, String> {
        if self.sent_packets_buffer.is_empty() {
            return Err("No control data available".into());
        }
        Ok(self.sent_packets_buffer.last().cloned().unwrap())
    }

    /// Get the number of packets sent (for testing fragmentation).
    pub fn get_sent_packet_count(&self) -> usize {
        self.sent_packets_buffer.len()
    }

    /// Clear the sent packets buffer (call between test steps).
    pub fn clear_sent_packets(&mut self) {
        self.sent_packets_buffer.clear();
    }

    fn unprotect_received_header(&self, data: &[u8]) -> Result<Vec<u8>, String> {
        if data.is_empty() {
            return Err("Empty packet".to_string());
        }

        let packet_type =
            PacketType::from_byte(data[0]).ok_or_else(|| "Invalid packet type".to_string())?;
        if packet_type == PacketType::Retry {
            return Ok(data.to_vec());
        }

        let mut packet = data.to_vec();
        let pn_offset = packet::get_packet_number_offset(&packet, self.transport.local_cid.len())?;

        let protection = match packet_type {
            PacketType::Initial => self.initial_protection.as_ref(),
            PacketType::Handshake => self.hs_protection.as_ref(),
            PacketType::ZeroRtt => self.early_data_protection.as_ref(),
            PacketType::OneRtt => self.protection.as_ref(),
            PacketType::Retry => None,
        };

        if let Some(protection) = protection {
            protection
                .unprotect_header(&mut packet, pn_offset)
                .map_err(|e| format!("Header protection removal failed: {}", e))?;
        }

        Ok(packet)
    }

    /// Parse and decrypt packets from the receive buffer.
    /// Returns the first STREAM frame found.
    fn recv_from_buffer(&mut self) -> Result<Option<(u64, Vec<u8>, bool)>, String> {
        while self.recv_offset < self.recv_buffer.len() {
            let data = self.recv_buffer[self.recv_offset..].to_vec();
            let packet_bytes = self.unprotect_received_header(&data)?;

            match QuicPacket::from_bytes_with_short_dcid_len(
                &packet_bytes,
                self.transport.local_cid.len(),
            ) {
                Ok((mut packet, consumed)) => {
                    self.recv_offset += consumed;

                    if packet.header.packet_type != PacketType::OneRtt {
                        continue;
                    }

                    let packet_number = self.transport.expand_packet_number(
                        PacketNumberSpace::ApplicationData,
                        packet.header.packet_number,
                        packet.header.pn_length,
                    );
                    packet.header.packet_number = packet_number;

                    let aad = packet.header_to_bytes_aad();
                    let plaintext = if packet.header.key_phase == self.peer_key_phase {
                        if let Some(ref mut prot) = self.protection {
                            prot.unprotect(&aad, packet.header.packet_number, &packet.payload)
                                .map_err(|e| format!("Packet decryption failed: {}", e))?
                        } else {
                            packet.payload.clone()
                        }
                    } else {
                        let previous_result = self.prev_protection.as_mut().and_then(|prev| {
                            prev.unprotect(&aad, packet.header.packet_number, &packet.payload)
                                .ok()
                        });

                        if let Some(plaintext) = previous_result {
                            plaintext
                        } else {
                            self.install_peer_key_update(packet.header.key_phase)?;
                            self.protection
                                .as_mut()
                                .ok_or_else(|| "No packet protection keys".to_string())?
                                .unprotect(&aad, packet.header.packet_number, &packet.payload)
                                .map_err(|e| {
                                    format!("Packet decryption after peer key update failed: {}", e)
                                })?
                        }
                    };

                    self.transport.update_activity();
                    self.transport
                        .record_received_packet(PacketNumberSpace::ApplicationData, packet_number);

                    // Parse frames from the decrypted payload
                    if let Some(stream_data) = self.parse_stream_frames(&plaintext) {
                        return Ok(Some(stream_data));
                    }
                }
                Err(_) => {
                    // Failed to parse — skip this byte and try again
                    self.recv_offset += 1;
                }
            }
        }

        Ok(None)
    }

    /// Parse QUIC frames from decrypted payload, returning the first STREAM frame.
    fn parse_stream_frames(&mut self, data: &[u8]) -> Option<(u64, Vec<u8>, bool)> {
        let mut pos = 0;
        while pos < data.len() {
            match QuicFrame::from_bytes(&data[pos..]) {
                Ok((frame, consumed)) => {
                    pos += consumed;
                    self.transport.process_frame(&frame);
                    if let QuicFrame::Stream {
                        stream_id,
                        offset: _,
                        fin,
                        data: frame_data,
                    } = frame
                    {
                        return Some((stream_id, frame_data, fin));
                    }
                    // Skip non-STREAM frames (ACK, PADDING, etc.)
                }
                Err(_) => {
                    // If frame parsing fails, stop processing this packet
                    break;
                }
            }
        }
        None
    }

    /// Get mutable crypto
    pub fn crypto_mut(&mut self) -> &mut QuicCrypto {
        &mut self.crypto
    }

    /// Set protection keys after handshake
    pub fn set_protection_keys(&mut self, keys: &ProtKeys) {
        self.crypto.set_keys(CryptoPhase::Application, keys.clone());
        self.protection = Some(PacketProtection::new(keys));
    }

    /// Set non-blocking mode — no-op for async sockets (always non-blocking).
    pub fn set_nonblocking(&self, _nonblocking: bool) -> Result<(), String> {
        Ok(())
    }

    // ------------------------------------------------------------------
    // 0-RTT / Early Data (RFC 9001 §4.6, RFC 9114 §4.3)
    // ------------------------------------------------------------------

    /// Enable 0-RTT early data with pre-derived protection keys.
    ///
    /// The keys must be derived from the TLS early secret + ClientHello hash
    /// (RFC 8446 §7.1, "c e traffic" label). Call this after constructing
    /// the connection but before sending any data.
    ///
    /// NOTE: 0-RTT data is vulnerable to replay attacks. Only use for idempotent
    /// requests (GET, HEAD, OPTIONS).
    pub fn enable_early_data(&mut self, keys: crypto::ProtectionKeys) {
        self.early_data_protection = Some(crypto::PacketProtection::new(&keys));
        self.early_data_sent = false;
    }

    /// Send early data (0-RTT) before the handshake completes.
    ///
    /// This sends data encrypted with 0-RTT keys, allowing the client to
    /// send HTTP/3 requests in the first flight.
    pub async fn send_early_data(
        &mut self,
        stream_id: u64,
        data: &[u8],
        fin: bool,
    ) -> Result<(), String> {
        if self.early_data_protection.is_none() {
            return Err("Early data not enabled".to_string());
        }

        let frame = self
            .transport
            .create_stream_frame(stream_id, data.to_vec(), fin);
        let pn = self
            .transport
            .next_packet_number(PacketNumberSpace::ApplicationData);

        let payload = frame.to_bytes();
        let packet = QuicPacket {
            header: packet::QuicPacketHeader {
                packet_type: PacketType::ZeroRtt,
                version: QUIC_VERSION_V1,
                dst_cid: self.transport.remote_cid.as_bytes().to_vec(),
                src_cid: self.transport.local_cid.as_bytes().to_vec(),
                token: Vec::new(),
                pn_length: 4,
                packet_number: pn,
                key_phase: false,
                payload_length: payload.len(),
            },
            payload,
        };
        let aad = packet.header_to_bytes_aad_with_payload_len(packet.payload.len() + 16);

        if let Some(ref mut prot) = self.early_data_protection {
            let encrypted = prot
                .protect_with_packet_number(packet.header.packet_number, &aad, &packet.payload)
                .map_err(|e| format!("0-RTT encrypt failed: {}", e))?;

            let mut packet_bytes = aad;
            packet_bytes.extend_from_slice(&encrypted);
            let pn_offset =
                packet::get_packet_number_offset(&packet_bytes, self.transport.remote_cid.len())?;
            prot.protect_header(&mut packet_bytes, pn_offset, packet.header.pn_length)
                .map_err(|e| format!("0-RTT header protection failed: {}", e))?;

            let addr: SocketAddr = self.server_addr.parse().unwrap_or_else(|_| {
                format!("{}:443", self.server_addr)
                    .parse()
                    .expect("server_addr must be parseable as SocketAddr")
            });
            self.socket
                .send_to(&packet_bytes, addr)
                .await
                .map_err(|e| format!("UDP send failed: {}", e))?;
            self.sent_packets_buffer.push(packet_bytes);
        }

        self.early_data_sent = true;
        self.transport.update_activity();
        Ok(())
    }

    /// Check if early data was sent on this connection.
    pub fn early_data_was_sent(&self) -> bool {
        self.early_data_sent
    }

    // ------------------------------------------------------------------
    // Connection Migration (RFC 9000 §9)
    // ------------------------------------------------------------------

    /// Get the currently active path (local_addr, remote_addr).
    ///
    /// Returns `None` if no path has been validated yet.
    pub fn active_path(
        &self,
    ) -> Option<(
        crate::runtime::net::SocketAddr,
        crate::runtime::net::SocketAddr,
    )> {
        self.active_path
    }

    /// Set the active path after successful path validation.
    ///
    /// This is called after receiving a valid PATH_RESPONSE for a
    /// previously sent PATH_CHALLENGE (RFC 9000 §9.3).
    pub fn set_active_path(
        &mut self,
        local: crate::runtime::net::SocketAddr,
        remote: crate::runtime::net::SocketAddr,
    ) {
        self.active_path = Some((local, remote));
    }

    /// Initialize the active path from the socket's local address and the server address.
    ///
    /// Called after handshake completion to establish the initial path.
    pub fn initialize_active_path(&mut self) {
        if self.active_path.is_none() {
            if let Ok(local) = self.socket.local_addr() {
                if let Ok(remote) = self.server_addr.parse::<crate::runtime::net::SocketAddr>() {
                    self.active_path = Some((local, remote));
                }
            }
        }
    }

    /// Send a PATH_CHALLENGE to probe a new path (RFC 9000 §9.1).
    ///
    /// Used to validate a new path when the client's address changes.
    pub async fn send_path_challenge(&mut self, data: [u8; 8]) -> Result<(), String> {
        let frame = QuicFrame::PathChallenge { data };
        self.pending_path_challenges.insert(
            data,
            crate::runtime::time::Instant::now() + crate::runtime::time::Duration::from_secs(3),
        );
        self.send_frame(frame).await
    }

    /// Process a received PATH_CHALLENGE and send PATH_RESPONSE.
    ///
    /// When we receive a PATH_CHALLENGE, we must respond with a PATH_RESPONSE
    /// containing the same data to validate the path.
    pub async fn respond_to_path_challenge(&mut self, data: [u8; 8]) -> Result<(), String> {
        let frame = QuicFrame::PathResponse { data };
        self.send_frame(frame).await
    }

    /// Check for expired PATH_CHALLENGE probes.
    ///
    /// Returns the list of expired challenge data.
    pub fn check_expired_path_challenges(&mut self) -> Vec<[u8; 8]> {
        let now = crate::runtime::time::Instant::now();
        let expired: Vec<[u8; 8]> = self
            .pending_path_challenges
            .iter()
            .filter_map(|(data, deadline)| if now > *deadline { Some(*data) } else { None })
            .collect();
        for data in &expired {
            self.pending_path_challenges.remove(data);
        }
        expired
    }

    // ------------------------------------------------------------------
    // Key Update (RFC 9001 §6)
    // ------------------------------------------------------------------

    /// Initiate a key update for 1-RTT traffic keys (RFC 9001 §6).
    ///
    /// When the AEAD key usage limit is approaching, the endpoint initiates
    /// a key update by deriving new traffic secrets and updating protection keys.
    /// The Key Phase bit is toggled in subsequent 1-RTT packets.
    ///
    /// # RFC 9001 §6 Key Update Process
    /// 1. Derive next secret: `next_secret = HKDF-Expand-Label(current_secret, "traffic upd", "", Hash.length)`
    /// 2. Derive new keys from next secret
    /// 3. Update protection with new keys
    /// 4. Store old protection for in-flight packet decryption
    /// 5. Toggle key phase bit
    pub fn initiate_key_update(&mut self) -> Result<(), String> {
        if self.client_app_traffic_secret.is_empty() {
            return Err(
                "Cannot initiate key update: no application traffic secret available".into(),
            );
        }
        if self.server_app_traffic_secret.is_empty() {
            return Err(
                "Cannot initiate key update: no peer application traffic secret available".into(),
            );
        }

        // Derive next client application traffic secret (RFC 8446 §7.2)
        let next_secret = self.cipher_suite_hash.expand_label(
            &self.client_app_traffic_secret,
            "traffic upd",
            &[],
            self.cipher_suite_hash.len(),
        );

        // Derive new keys from the next secret
        let key_len = 16; // AES-128-GCM
        let iv_len = 12;
        let next_write_key =
            self.cipher_suite_hash
                .quic_expand_label(&next_secret, "key", &[], key_len);
        let next_write_iv =
            self.cipher_suite_hash
                .quic_expand_label(&next_secret, "iv", &[], iv_len);
        let next_write_hp =
            self.cipher_suite_hash
                .quic_expand_label(&next_secret, "hp", &[], key_len);
        let current_read_key = self.cipher_suite_hash.quic_expand_label(
            &self.server_app_traffic_secret,
            "key",
            &[],
            key_len,
        );
        let current_read_iv = self.cipher_suite_hash.quic_expand_label(
            &self.server_app_traffic_secret,
            "iv",
            &[],
            iv_len,
        );
        let current_read_hp = self.cipher_suite_hash.quic_expand_label(
            &self.server_app_traffic_secret,
            "hp",
            &[],
            key_len,
        );

        // Store old protection for in-flight packet decryption
        if let Some(old_protection) = self.protection.take() {
            self.prev_protection = Some(old_protection);
        }

        // Create new protection with updated keys
        let new_keys = crypto::ProtectionKeys::new(
            CipherSuite::TLS_AES_128_GCM_SHA256,
            next_write_key,
            next_write_iv,
            current_read_key,
            current_read_iv,
        )
        .with_header_protection(next_write_hp, current_read_hp);
        self.protection = Some(crypto::PacketProtection::new(&new_keys));

        // Update the stored secret for future key updates
        self.client_app_traffic_secret = next_secret;

        // Toggle key phase bit
        self.key_phase = !self.key_phase;

        Ok(())
    }

    fn install_peer_key_update(&mut self, peer_key_phase: bool) -> Result<(), String> {
        if self.client_app_traffic_secret.is_empty() || self.server_app_traffic_secret.is_empty() {
            return Err("Cannot process peer key update without application traffic secrets".into());
        }

        let key_len = 16;
        let iv_len = 12;
        let next_peer_secret = self.cipher_suite_hash.expand_label(
            &self.server_app_traffic_secret,
            "traffic upd",
            &[],
            self.cipher_suite_hash.len(),
        );

        let current_write_key = self.cipher_suite_hash.quic_expand_label(
            &self.client_app_traffic_secret,
            "key",
            &[],
            key_len,
        );
        let current_write_iv = self.cipher_suite_hash.quic_expand_label(
            &self.client_app_traffic_secret,
            "iv",
            &[],
            iv_len,
        );
        let current_write_hp = self.cipher_suite_hash.quic_expand_label(
            &self.client_app_traffic_secret,
            "hp",
            &[],
            key_len,
        );
        let next_read_key =
            self.cipher_suite_hash
                .quic_expand_label(&next_peer_secret, "key", &[], key_len);
        let next_read_iv =
            self.cipher_suite_hash
                .quic_expand_label(&next_peer_secret, "iv", &[], iv_len);
        let next_read_hp =
            self.cipher_suite_hash
                .quic_expand_label(&next_peer_secret, "hp", &[], key_len);

        if let Some(old_protection) = self.protection.take() {
            self.prev_protection = Some(old_protection);
        }

        let new_keys = crypto::ProtectionKeys::new(
            CipherSuite::TLS_AES_128_GCM_SHA256,
            current_write_key,
            current_write_iv,
            next_read_key,
            next_read_iv,
        )
        .with_header_protection(current_write_hp, next_read_hp);
        self.protection = Some(crypto::PacketProtection::new(&new_keys));
        self.server_app_traffic_secret = next_peer_secret;
        self.peer_key_phase = peer_key_phase;

        Ok(())
    }

    /// Get the current key phase (for setting the Key Phase bit in packet headers).
    pub fn key_phase(&self) -> bool {
        self.key_phase
    }

    /// Clear previous protection keys (safe to call after confirming peer has received
    /// packets encrypted with the new keys — RFC 9001 §6.1).
    pub fn discard_old_keys(&mut self) {
        self.prev_protection = None;
    }

    // ------------------------------------------------------------------
    // Idle Timeout (RFC 9000 §10.1)
    // ------------------------------------------------------------------

    /// Check if the connection has exceeded the idle timeout.
    ///
    /// Returns true if no packets have been received within the configured
    /// max_idle_timeout (or a default of 30 seconds).
    pub fn is_idle_timeout(&self) -> bool {
        let timeout = crate::runtime::time::Duration::from_millis(
            self.transport.params.max_idle_timeout.max(30000),
        );
        self.transport.last_activity().elapsed() > timeout
    }

    /// Get time until idle timeout fires.
    pub fn time_until_idle_timeout(&self) -> crate::runtime::time::Duration {
        let timeout = crate::runtime::time::Duration::from_millis(
            self.transport.params.max_idle_timeout.max(30000),
        );
        let elapsed = self.transport.last_activity().elapsed();
        if elapsed >= timeout {
            crate::runtime::time::Duration::ZERO
        } else {
            timeout - elapsed
        }
    }
}

fn pad_initial_datagram(pkt: &mut QuicPacket) {
    const MIN_INITIAL_DATAGRAM: usize = 1200;
    loop {
        let datagram_len = pkt
            .header_to_bytes_aad_with_payload_len(pkt.payload.len() + 16)
            .len()
            + pkt.payload.len()
            + 16;
        if datagram_len >= MIN_INITIAL_DATAGRAM {
            break;
        }
        pkt.payload.push(0);
    }
}

fn packet_number_space(packet_type: PacketType) -> PacketNumberSpace {
    match packet_type {
        PacketType::Initial => PacketNumberSpace::Initial,
        PacketType::Handshake => PacketNumberSpace::Handshake,
        PacketType::OneRtt | PacketType::ZeroRtt | PacketType::Retry => {
            PacketNumberSpace::ApplicationData
        }
    }
}

#[cfg(test)]
mod tests;
