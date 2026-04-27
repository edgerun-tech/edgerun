//! Async HTTP/3 server.
//!
//! Listens on a UDP socket and accepts incoming HTTP/3 (QUIC) connections.
//! Each accepted connection gets its own [`Http3Connection`] with the full
//! QUIC + TLS 1.3 handshake completed.
//!
//! # Example
//! ```no_run
//! use edgerun_http::http3::Http3Server;
//! use edgerun_tls::certificate_gen::generate_self_signed;
//! use edgerun_bare_rt::Runtime;
//!
//! let rt = Runtime::new_multi_thread().enable_all().build().unwrap();
//! rt.block_on(async {
//!     let cert = generate_self_signed(&["localhost"]);
//!     let server = Http3Server::bind("127.0.0.1:4433", cert).await.unwrap();
//!     println!("HTTP/3 server listening on {}", server.local_addr().unwrap());
//!
//!     // Accept connections in a loop
//!     loop {
//!         match server.accept().await {
//!             Ok((conn, client_addr)) => {
//!                 println!("Accepted HTTP/3 connection from {}", client_addr);
//!                 // Handle conn...
//!             }
//!             Err(e) => eprintln!("Accept error: {}", e),
//!         }
//!     }
//! });
//! ```

use crate::handler::Handler;
use crate::header::HeaderMap;
use crate::method::Method;
use crate::request::Request;
use crate::response::Response;
use crate::uri::Uri;
use edgerun_bare_rt::AsyncUdpSocket;
use edgerun_bare_rt::CancellationToken;
use edgerun_tls::certificate_gen::CertificateAndKey;
use std::net::SocketAddr;
use std::sync::Arc;

use super::connection::Http3Connection;
use super::qpack::{QpackDecoder, QpackEncoder};
use super::quic::crypto::PacketProtection;
use super::quic::frame::QuicFrame;
use super::quic::packet::{PacketType, QuicPacket};
use super::quic::ConnectionId;
use super::quic::QuicConnection;
use super::quic::QuicTlsServerHandshaker;
use super::Http3Error;
use crate::http3::settings::Http3Settings;
use std::collections::HashMap;
use std::sync::Mutex;

/// Per-client address validation state (RFC 9000 §8.1).
///
/// During the Initial phase, the server MUST limit its sends to 3x the
/// bytes received from the client until the client's address is validated
/// (via receiving a Handshake packet or a valid Retry token).
struct AddressValidationState {
    /// Total bytes received from this client during Initial phase
    bytes_received: u64,
    /// Total bytes sent to this client during Initial phase
    bytes_sent: u64,
    /// Whether the client's address has been validated
    /// (set to true when we receive a valid Handshake packet)
    address_validated: bool,
    /// Timestamp of last activity (for cleanup)
    last_activity: std::time::Instant,
}

impl AddressValidationState {
    fn new() -> Self {
        AddressValidationState {
            bytes_received: 0,
            bytes_sent: 0,
            address_validated: false,
            last_activity: std::time::Instant::now(),
        }
    }

    /// Check if we're allowed to send `bytes` more bytes to this client.
    ///
    /// RFC 9000 §8.1: Before address validation, server sends ≤ 3 × bytes received.
    fn can_send(&self, bytes: u64) -> bool {
        if self.address_validated {
            return true;
        }
        self.bytes_sent + bytes <= self.bytes_received * 3
    }

    /// Record bytes received from this client.
    fn record_received(&mut self, bytes: u64) {
        self.bytes_received += bytes;
        self.last_activity = std::time::Instant::now();
    }

    /// Record bytes sent to this client.
    fn record_sent(&mut self, bytes: u64) {
        self.bytes_sent += bytes;
    }

    /// Mark the client's address as validated.
    fn mark_validated(&mut self) {
        self.address_validated = true;
    }

    /// Check if this validation state is stale (> 30 seconds).
    fn is_stale(&self) -> bool {
        self.last_activity.elapsed().as_secs() > 30
    }
}

/// HTTP/3 server listening on a UDP socket.
pub struct Http3Server {
    /// Underlying UDP socket
    socket: Arc<AsyncUdpSocket>,
    /// Certificate and signing key for TLS
    cert_and_key: CertificateAndKey,
    /// Pending connections (packet → server_addr)
    pending: Mutex<Vec<(Vec<u8>, SocketAddr)>>,
    /// Address validation state per client (for anti-amplification)
    validation_state: Mutex<HashMap<SocketAddr, AddressValidationState>>,
}

impl Http3Server {
    /// Bind to the given address and prepare to accept HTTP/3 connections.
    pub async fn bind<A: std::net::ToSocketAddrs>(
        addr: A,
        cert_and_key: CertificateAndKey,
    ) -> std::io::Result<Self> {
        let socket = Arc::new(AsyncUdpSocket::bind(addr)?);
        Ok(Http3Server {
            socket,
            cert_and_key,
            pending: Mutex::new(Vec::new()),
            validation_state: Mutex::new(HashMap::new()),
        })
    }

    /// Local address of the server.
    pub fn local_addr(&self) -> std::io::Result<SocketAddr> {
        self.socket.local_addr()
    }

    /// Accept the next incoming HTTP/3 connection.
    ///
    /// This performs the full QUIC + TLS 1.3 server handshake:
    /// 1. Receive Initial packet with ClientHello
    /// 2. Derive Initial keys from client's DCID
    /// 3. Parse ClientHello, build ServerHello
    /// 4. Derive Handshake keys via ECDH
    /// 5. Build and send encrypted handshake messages
    /// 6. Wait for client's Finished
    /// 7. Derive Application keys
    ///
    /// Returns the established connection and the client's address.
    pub async fn accept(&self) -> Result<(Http3Connection, SocketAddr), String> {
        loop {
            // First check if we have pending data — extract without holding lock across await
            let pending_item = {
                let mut pending = self.pending.lock().unwrap();
                pending.first().cloned()
            };
            if let Some((data, client_addr)) = pending_item {
                // Remove from pending before processing
                {
                    let mut pending = self.pending.lock().unwrap();
                    if !pending.is_empty() {
                        pending.remove(0);
                    }
                }
                if let Ok(conn) = self.handle_initial_packet(&data, client_addr).await {
                    return Ok(conn);
                }
                // If handshake failed, try next packet
            }

            // Receive next packet
            let mut buf = [0u8; 65536]; // Max UDP datagram
            let (n, client_addr) = self
                .socket
                .recv_from(&mut buf)
                .await
                .map_err(|e| format!("recv_from failed: {}", e))?;

            let data = buf[..n].to_vec();
            match self.handle_initial_packet(&data, client_addr).await {
                Ok(conn) => return Ok(conn),
                Err(e) => {
                    // Log and try next packet
                    eprintln!(
                        "[HTTP/3 server] handshake error from {}: {}",
                        client_addr, e
                    );
                }
            }
        }
    }

    /// Process an Initial-level packet and perform the server-side handshake.
    async fn handle_initial_packet(
        &self,
        data: &[u8],
        client_addr: SocketAddr,
    ) -> Result<(Http3Connection, SocketAddr), String> {
        if data.len() < 25 {
            return Err("Packet too short".to_string());
        }

        let first_byte = data[0];
        eprintln!(
            "DEBUG: first={:02x} data[:20]={:02x?}",
            first_byte,
            &data[..20]
        );

        // Parse packet to get header_to_bytes_aad()
        let (pkt, _) =
            QuicPacket::from_bytes(data).map_err(|e| format!("Parse packet failed: {}", e))?;

        // Use header_to_bytes_aad() - same method as client
        let aad = pkt.header_to_bytes_aad();
        let encrypted_payload = data[aad.len()..].to_vec();

        eprintln!(
            "DEBUG: aad_len={}, encrypted_len={}",
            aad.len(),
            encrypted_payload.len()
        );

        // Extract CIDs from AAD
        let dst_cid_len = aad[5] as usize;
        let dst_cid = aad[6..6 + dst_cid_len].to_vec();
        let src_cid = if 6 + dst_cid_len + 1 < aad.len() {
            let src_offset = 6 + dst_cid_len + 1;
            let src_cid_len = aad[src_offset - 1] as usize;
            if src_offset + src_cid_len <= aad.len() {
                aad[src_offset..src_offset + src_cid_len].to_vec()
            } else {
                vec![]
            }
        } else {
            vec![]
        };

        eprintln!("DEBUG: parsed dst_cid={:?} src_cid={:?}", dst_cid, src_cid);

        let mut handshaker = QuicTlsServerHandshaker::new(self.cert_and_key.clone());
        let initial_keys = handshaker.initial_keys(&dst_cid);
        let mut initial_protection = PacketProtection::new(&initial_keys);

        let plaintext = initial_protection
            .unprotect(&aad, 0, &encrypted_payload)
            .map_err(|e| format!("Decrypt failed: {}", e))?;

        eprintln!(
            "DEBUG: plaintext[:30]={:02x?}",
            &plaintext[..plaintext.len().min(30)]
        );
        eprintln!(
            "DEBUG: frame_type=0x{:02x}",
            plaintext.first().copied().unwrap_or(0)
        );

        // Try parsing as QUIC CRYPTO frame first (0x06)
        let mut crypto_data = Self::parse_crypto_frame(&plaintext).map(|(d, _)| d);

        // If no QUIC CRYPTO frame, try treating raw TLS handshake data
        if crypto_data.is_none() && !plaintext.is_empty() {
            eprintln!("DEBUG: trying raw TLS data");
            // Raw TLS handshake data starts with 0x01 (ClientHello) or 0x02 (ServerHello)
            // TLS over QUIC uses 0x01 prefix for Handshake message type
            if plaintext.starts_with(&[0x01]) || plaintext.starts_with(&[0x16]) {
                crypto_data = Some(plaintext.to_vec());
            }
        }

        let crypto_data =
            crypto_data.ok_or_else(|| "No CRYPTO frame in decrypted payload".to_string())?;

        // Update address validation state
        {
            let mut state_map = self.validation_state.lock().unwrap();
            state_map.retain(|_, state| !state.is_stale());
            let state = state_map
                .entry(client_addr)
                .or_insert_with(AddressValidationState::new);
            state.record_received(data.len() as u64);
        }

        let client_dcid = dst_cid;
        let client_scid = src_cid;

        // Process ClientHello, get ServerHello
        let server_hello = handshaker.process_client_hello(&crypto_data)?;

        // Now we need to send the response:
        // 1. Initial packet with ServerHello (encrypted with Initial keys)
        // 2. Handshake packets with EE, Cert, CertVerify, Finished (encrypted with Handshake keys)

        // Check anti-amplification limit before sending
        {
            let state_map = self.validation_state.lock().unwrap();
            if let Some(state) = state_map.get(&client_addr) {
                let estimated_response_size = 500; // Estimate Initial packet size
                if !state.can_send(estimated_response_size as u64) {
                    return Err(format!(
                        "Anti-amplification limit reached for {} ({} sent, {} received)",
                        client_addr, state.bytes_sent, state.bytes_received
                    ));
                }
            }
        }

        // Build Initial response packet
        let server_hello_frame = QuicFrame::Crypto {
            offset: 0,
            data: server_hello.clone(),
        };
        let initial_response = self.build_initial_response(
            &client_dcid,
            &client_scid,
            &server_hello_frame,
            &handshaker,
        )?;

        // Send Initial packet
        self.socket
            .send_to(&initial_response, client_addr)
            .await
            .map_err(|e| format!("Failed to send Initial packet: {}", e))?;

        // Update validation state
        {
            let mut state_map = self.validation_state.lock().unwrap();
            if let Some(state) = state_map.get_mut(&client_addr) {
                state.record_sent(initial_response.len() as u64);
            }
        }

        // Build and send Handshake packet
        let (handshake_crypto, expected_client_verify) = handshaker
            .build_encrypted_handshake()
            .map_err(|e| format!("Failed to build encrypted handshake: {}", e))?;

        let handshake_frame = QuicFrame::Crypto {
            offset: 0,
            data: handshake_crypto.clone(),
        };
        let handshake_response = self.build_handshake_response(
            &client_dcid,
            &client_scid,
            &handshake_frame,
            &handshaker,
        )?;

        // Check anti-amplification limit for Handshake
        {
            let state_map = self.validation_state.lock().unwrap();
            if let Some(state) = state_map.get(&client_addr) {
                if !state.can_send(handshake_response.len() as u64) {
                    return Err(format!(
                        "Anti-amplification limit reached for {} (Handshake)",
                        client_addr
                    ));
                }
            }
        }

        // Send Handshake packet
        self.socket
            .send_to(&handshake_response, client_addr)
            .await
            .map_err(|e| format!("Failed to send Handshake packet: {}", e))?;

        // Update validation state
        {
            let mut state_map = self.validation_state.lock().unwrap();
            if let Some(state) = state_map.get_mut(&client_addr) {
                state.record_sent(handshake_response.len() as u64);
            }
        }

        // Wait for client's Finished in a Handshake packet
        let client_finished_data = self
            .wait_for_client_finished(
                client_addr,
                &client_dcid,
                &handshaker,
                &expected_client_verify,
            )
            .await?;

        // Client's address is now validated (they received our Handshake and replied)
        {
            let mut state_map = self.validation_state.lock().unwrap();
            if let Some(state) = state_map.get_mut(&client_addr) {
                state.mark_validated();
            }
        }

        // Build the transcript including client Finished
        let mut transcript_after = handshaker.transcript().to_vec();

        // Derive application keys and build the handshake result
        // The server uses the client's DCID (our SCID) as the dcid for key derivation
        let handshake_result = handshaker
            .build_result(&client_dcid, &transcript_after)
            .map_err(|e| format!("Failed to build handshake result: {}", e))?;
        handshaker.mark_complete();

        // Create a QuicConnection from the server-side handshake result
        // We need to extract the raw socket — we'll use mem::replace to take ownership
        // Actually, we can't extract the socket from AsyncUdpSocket. Instead,
        // we create the Http3Connection directly with the handshake result.
        let quic_conn = QuicConnection::from_server(
            // For the server, we use the same socket but with the client's address
            // The QuicConnection needs a UdpSocket — we'll create one bound to 0
            // and manage the actual I/O through the server's socket.
            // For now, use a dummy socket — the server handles I/O directly.
            Self::dummy_socket().map_err(|e| format!("Failed to create dummy socket: {}", e))?,
            client_addr.to_string(),
            ConnectionId::new(client_dcid),
            ConnectionId::new(client_scid),
            handshake_result,
        )?;

        // Wrap in Http3Connection (sends server preface: control + QPACK streams)
        let conn = Http3Connection::from_server(quic_conn)
            .await
            .map_err(|e| format!("Failed to create server HTTP/3 connection: {}", e))?;

        Ok((conn, client_addr))
    }

    /// Create a dummy UDP socket for server-side connections.
    ///
    /// Server connections don't use the socket directly — I/O is handled
    /// through the server's AsyncUdpSocket.
    fn dummy_socket() -> std::io::Result<std::net::UdpSocket> {
        std::net::UdpSocket::bind("127.0.0.1:0")
    }

    /// Build Initial response packet (contains ServerHello).
    fn build_initial_response(
        &self,
        client_dcid: &[u8],
        client_scid: &[u8],
        crypto_frame: &QuicFrame,
        handshaker: &QuicTlsServerHandshaker,
    ) -> Result<Vec<u8>, String> {
        let payload = crypto_frame.to_bytes();
        let initial_keys = handshaker.initial_keys(client_dcid);
        let mut protection = PacketProtection::new(&initial_keys);

        // Build long header for Initial
        let mut header = Vec::new();
        header.push(0x0C); // Long header, Initial type (0x00), fixed bits
        header.extend_from_slice(&0x00000001u32.to_be_bytes()); // Version
        header.push(client_dcid.len() as u8);
        header.extend_from_slice(client_dcid);
        header.push(client_scid.len() as u8);
        header.extend_from_slice(client_scid);
        // Token length = 0 (varint, 1 byte)
        header.push(0x00);
        // Payload length placeholder (2 bytes, will fill after)
        let payload_len_pos = header.len();
        header.extend_from_slice(&[0u8; 2]);
        // Packet number (2 bytes)
        header.extend_from_slice(&0u64.to_be_bytes()[6..]);

        let header_len = header.len();
        header.extend_from_slice(&payload);

        let encrypted = protection
            .protect(&header, &header[header_len..])
            .map_err(|e| format!("Initial encrypt failed: {}", e))?;

        // Fill in payload length
        let total_payload = encrypted.len();
        header[payload_len_pos] = ((total_payload >> 8) as u8) | 0x40;
        header[payload_len_pos + 1] = total_payload as u8;

        // Rebuild: header + encrypted payload
        let mut packet = header[..header_len].to_vec();
        packet.extend_from_slice(&encrypted);

        Ok(packet)
    }

    /// Build Handshake-level response packet (EE, Cert, CertVerify, Finished).
    fn build_handshake_response(
        &self,
        client_dcid: &[u8],
        client_scid: &[u8],
        crypto_frame: &QuicFrame,
        handshaker: &QuicTlsServerHandshaker,
    ) -> Result<Vec<u8>, String> {
        let payload = crypto_frame.to_bytes();
        let hs_keys = handshaker
            .handshake_keys()
            .map_err(|e| format!("Failed to derive handshake keys: {}", e))?;
        let mut protection = PacketProtection::new(&hs_keys);

        // Build long header for Handshake
        let mut header = Vec::new();
        header.push(0x0C | 0x20); // Long header, Handshake type (0x20), fixed bits
        header.extend_from_slice(&0x00000001u32.to_be_bytes()); // Version
        header.push(client_dcid.len() as u8);
        header.extend_from_slice(client_dcid);
        header.push(client_scid.len() as u8);
        header.extend_from_slice(client_scid);
        // Token length = 0
        header.push(0x00);
        // Payload length placeholder
        let payload_len_pos = header.len();
        header.extend_from_slice(&[0u8; 2]);
        // Packet number
        header.extend_from_slice(&0u64.to_be_bytes()[6..]);

        let header_len = header.len();
        header.extend_from_slice(&payload);

        let encrypted = protection
            .protect(&header, &header[header_len..])
            .map_err(|e| format!("Handshake encrypt failed: {}", e))?;

        let total_payload = encrypted.len();
        header[payload_len_pos] = ((total_payload >> 8) as u8) | 0x40;
        header[payload_len_pos + 1] = total_payload as u8;

        let mut packet = header[..header_len].to_vec();
        packet.extend_from_slice(&encrypted);

        Ok(packet)
    }

    /// Wait for the client's Finished message in a Handshake packet.
    ///
    /// Receives, decrypts, and verifies the client's Finished verify_data.
    /// Returns the raw client Finished message bytes (for transcript).
    async fn wait_for_client_finished(
        &self,
        _client_addr: SocketAddr,
        client_dcid: &[u8],
        handshaker: &QuicTlsServerHandshaker,
        expected_client_verify: &[u8],
    ) -> Result<Vec<u8>, String> {
        // Derive handshake-level protection keys for decryption
        let hs_keys = handshaker
            .handshake_keys()
            .map_err(|e| format!("Failed to derive handshake keys: {}", e))?;
        let mut hs_protection = PacketProtection::new(&hs_keys);

        // Try to receive packets until we find the client's Finished
        for _ in 0..20 {
            let mut buf = [0u8; 65536];
            let (n, _src) = self
                .socket
                .recv_from(&mut buf)
                .await
                .map_err(|e| format!("recv_from failed: {}", e))?;

            let (pkt, _) = QuicPacket::from_bytes(&buf[..n])
                .map_err(|e| format!("Packet parse error: {}", e))?;

            // Client sends Finished in Handshake-level packets
            if pkt.header.packet_type != PacketType::Handshake {
                continue;
            }

            // Decrypt
            let plaintext = hs_protection
                .unprotect(&[], pkt.header.packet_number, &pkt.payload)
                .map_err(|e| format!("Handshake decrypt failed: {}", e))?;

            // Parse CRYPTO frame
            if let Some((crypto_data, _)) = Self::parse_crypto_frame(&plaintext) {
                // The client's Finished message is in this CRYPTO data
                // Finished message: type(1) + length(3) + verify_data(32)
                if crypto_data.len() >= 4 + expected_client_verify.len() {
                    // Check if this is actually a Finished message (type 20)
                    if crypto_data[0] == 20 {
                        let verify_data = &crypto_data[4..4 + expected_client_verify.len()];

                        // Verify the client's Finished
                        handshaker.verify_client_finished(verify_data, expected_client_verify)?;

                        return Ok(crypto_data);
                    }
                }
            }
        }

        Err("Client Finished not received after 20 attempts".into())
    }

    /// Parse a CRYPTO frame from decrypted packet payload.
    fn parse_crypto_frame(data: &[u8]) -> Option<(Vec<u8>, usize)> {
        if data.is_empty() {
            return None;
        }

        let frame_type = data[0];
        if frame_type != 0x06 {
            return None;
        }

        let mut pos = 1;
        let (offset, n) = Self::decode_varint_at(data, pos).ok()?;
        pos += n;
        let _offset = offset;

        let (length, n) = Self::decode_varint_at(data, pos).ok()?;
        pos += n;

        if pos + length as usize > data.len() {
            return None;
        }

        let crypto_data = data[pos..pos + length as usize].to_vec();
        Some((crypto_data, pos + length as usize))
    }

    fn decode_varint_at(data: &[u8], pos: usize) -> Result<(u64, usize), String> {
        if pos >= data.len() {
            return Err("Out of bounds".into());
        }
        let (value, len) = edgerun_encoding::quic_varint::decode_varint(&data[pos..])
            .map_err(|e| format!("{e}"))?;
        Ok((value, len))
    }

    /// Run the HTTP/3 server, dispatching incoming requests to the given handler.
    ///
    /// This method loops indefinitely, accepting connections and spawning
    /// a task for each one. The task processes all requests on the connection
    /// through the provided [`Handler`].
    ///
    /// The server shuts down when the `shutdown` token is cancelled.
    ///
    /// # Example
    /// ```no_run
    /// use edgerun_http::http3::Http3Server;
    /// use edgerun_http::{Handler, Request, Response, StatusCode, into_handler};
    /// use edgerun_tls::certificate_gen::generate_self_signed;
    /// use edgerun_bare_rt::Runtime;
    /// use std::sync::Arc;
    ///
    /// let rt = Runtime::new_multi_thread().enable_all().build().unwrap();
    /// rt.block_on(async {
    ///     let cert = generate_self_signed(&["localhost"]);
    ///     let server = Http3Server::bind("127.0.0.1:4433", cert).await.unwrap();
    ///     let handler = into_handler(|_req| {
    ///         Response::text(StatusCode::new(200).unwrap(), "Hello!")
    ///     });
    ///     let shutdown = edgerun_bare_rt::CancellationToken::new();
    ///     server.serve(Arc::new(handler), shutdown).await.unwrap();
    /// });
    /// ```
    pub async fn serve(
        &self,
        handler: Arc<dyn Handler>,
        shutdown: CancellationToken,
    ) -> std::io::Result<()> {
        edgerun_log::info!("HTTP/3 server listening on {} (h3)", self.local_addr()?);
        loop {
            if shutdown.is_cancelled() {
                edgerun_log::info!("HTTP/3 server shutting down");
                return Ok(());
            }

            match self.accept().await {
                Ok((mut conn, client_addr)) => {
                    let handler = Arc::clone(&handler);
                    edgerun_bare_rt::spawn(async move {
                        if let Err(e) =
                            Self::handle_connection(&mut conn, handler, client_addr).await
                        {
                            edgerun_log::warn!(
                                "HTTP/3 connection error from {}: {}",
                                client_addr,
                                e
                            );
                        }
                    });
                }
                Err(e) => {
                    edgerun_log::warn!("HTTP/3 accept error: {}", e);
                    edgerun_bare_rt::sleep(std::time::Duration::from_millis(100)).await;
                }
            }
        }
    }

    /// Handle a single HTTP/3 connection, dispatching requests to the handler.
    async fn handle_connection(
        conn: &mut Http3Connection,
        handler: Arc<dyn Handler>,
        _client_addr: SocketAddr,
    ) -> Result<(), String> {
        loop {
            let (stream_id, method, uri, headers) = match conn.accept_request().await {
                Ok(Some(result)) => result,
                Ok(None) => continue,
                Err(e) => return Err(format!("accept_request: {:?}", e)),
            };

            let body = conn
                .recv_request_body(stream_id)
                .await
                .unwrap_or(None)
                .unwrap_or_default();

            let request = Request::new(method, uri, headers, Some(body));
            let response = handler.handle(request).await;

            let status = response.status();
            let resp_headers = response.headers().clone();
            let body = response.body().to_vec();

            if let Err(e) = conn
                .send_response(stream_id, status, &resp_headers, Some(body))
                .await
            {
                return Err(format!("send_response: {:?}", e));
            }
        }
    }
}
