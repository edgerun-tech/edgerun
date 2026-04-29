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
//! use edgerun_rt::Runtime;
//!
//! let rt = Runtime::new_multi_thread().enable_all().build().unwrap();
//! rt.block_on(async {
//!     let cert = generate_self_signed(&["localhost"]).unwrap();
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
use crate::runtime;
use crate::runtime::net::SocketAddr;
use crate::runtime::AsyncUdpSocket;
use crate::runtime::CancellationToken;
use crate::uri::Uri;
use alloc::collections::BTreeMap as HashMap;
use alloc::format;
use alloc::string::{String, ToString};
use alloc::sync::Arc;
use alloc::vec;
use alloc::vec::Vec;
use edgerun_tls::certificate_gen::CertificateAndKey;

use super::connection::Http3Connection;
use super::quic::crypto::PacketProtection;
use super::quic::frame::QuicFrame;
use super::quic::packet::{self, PacketType, QuicPacket, QuicPacketHeader};
use super::quic::ConnectionId;
use super::quic::QuicConnection;
use super::quic::QuicTlsServerHandshaker;
use super::quic::QUIC_VERSION_V1;
use super::Http3Error;
use crate::http3::settings::Http3Settings;
use crate::runtime::sync::Mutex;
use edgerun_qpack::{QpackDecoder, QpackEncoder};

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
    last_activity: crate::runtime::time::Instant,
}

impl AddressValidationState {
    fn new() -> Self {
        AddressValidationState {
            bytes_received: 0,
            bytes_sent: 0,
            address_validated: false,
            last_activity: crate::runtime::time::Instant::now(),
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
        self.last_activity = crate::runtime::time::Instant::now();
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

fn parse_long_header_dcid(data: &[u8]) -> Result<Vec<u8>, String> {
    if data.len() < 6 {
        return Err("Packet too short for long header DCID".to_string());
    }
    if data[0] & 0x80 == 0 {
        return Err("Expected long header packet".to_string());
    }

    let dcid_len = data[5] as usize;
    if dcid_len > 20 {
        return Err("DCID length exceeds QUIC maximum".to_string());
    }
    let start = 6;
    let end = start + dcid_len;
    if end > data.len() {
        return Err("DCID exceeds packet length".to_string());
    }
    Ok(data[start..end].to_vec())
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
    pub async fn bind<A: crate::runtime::net::ToSocketAddrs>(
        addr: A,
        cert_and_key: CertificateAndKey,
    ) -> crate::runtime::io::Result<Self> {
        let socket = Arc::new(runtime::bind_udp_socket(addr)?);
        Ok(Http3Server {
            socket,
            cert_and_key,
            pending: Mutex::new(Vec::new()),
            validation_state: Mutex::new(HashMap::new()),
        })
    }

    /// Local address of the server.
    pub fn local_addr(&self) -> crate::runtime::io::Result<SocketAddr> {
        self.socket
            .local_addr()
            .map_err(crate::runtime::io::Error::other)
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
                    edgerun_log::warn!(
                        "[HTTP/3 server] handshake error from {}: {}",
                        client_addr,
                        e
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

        let mut handshaker = QuicTlsServerHandshaker::new(self.cert_and_key.clone());
        let dst_cid = parse_long_header_dcid(data)?;
        let initial_keys = handshaker.initial_keys(&dst_cid);
        let mut initial_protection = PacketProtection::new(&initial_keys);
        let mut packet_bytes = data.to_vec();
        let pn_offset = packet::get_packet_number_offset(&packet_bytes, 0)?;
        initial_protection
            .unprotect_header(&mut packet_bytes, pn_offset)
            .map_err(|e| format!("Initial header protection failed: {}", e))?;

        // Parse packet to get header_to_bytes_aad()
        let (pkt, _) = QuicPacket::from_bytes(&packet_bytes)
            .map_err(|e| format!("Parse packet failed: {}", e))?;

        // Use header_to_bytes_aad() - same method as client
        let aad = pkt.header_to_bytes_aad();

        let dst_cid = pkt.header.dst_cid.clone();
        let src_cid = pkt.header.src_cid.clone();

        let plaintext = initial_protection
            .unprotect(&aad, pkt.header.packet_number, &pkt.payload)
            .map_err(|e| format!("Decrypt failed: {}", e))?;

        // Try parsing as QUIC CRYPTO frame first (0x06)
        let mut crypto_data = super::crypto_frame::parse_crypto_frame(&plaintext).map(|(d, _)| d);

        // If no QUIC CRYPTO frame, try treating raw TLS handshake data
        if crypto_data.is_none() && !plaintext.is_empty() {
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
        transcript_after.extend_from_slice(&handshake_crypto);
        transcript_after.extend_from_slice(&client_finished_data);

        // Derive application keys and build the handshake result
        // The server uses the client's DCID (our SCID) as the dcid for key derivation
        let handshake_result = handshaker
            .build_result(&client_dcid, &transcript_after)
            .map_err(|e| format!("Failed to build handshake result: {}", e))?;
        handshaker.mark_complete();

        let quic_conn = QuicConnection::from_server_socket(
            Arc::clone(&self.socket),
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

        let packet = QuicPacket::initial(
            QUIC_VERSION_V1,
            client_scid.to_vec(),
            client_dcid.to_vec(),
            Vec::new(),
            0,
            payload,
        );
        let aad = packet.header_to_bytes_aad_with_payload_len(packet.payload.len() + 16);

        let encrypted = protection
            .protect_with_packet_number(packet.header.packet_number, &aad, &packet.payload)
            .map_err(|e| format!("Initial encrypt failed: {}", e))?;

        let mut packet_bytes = aad;
        packet_bytes.extend_from_slice(&encrypted);
        let pn_offset = packet::get_packet_number_offset(&packet_bytes, 0)?;
        protection
            .protect_header(&mut packet_bytes, pn_offset, packet.header.pn_length)
            .map_err(|e| format!("Initial header protection failed: {}", e))?;

        Ok(packet_bytes)
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

        let packet = QuicPacket {
            header: QuicPacketHeader {
                packet_type: PacketType::Handshake,
                version: QUIC_VERSION_V1,
                dst_cid: client_scid.to_vec(),
                src_cid: client_dcid.to_vec(),
                token: Vec::new(),
                pn_length: 4,
                packet_number: 0,
                key_phase: false,
                payload_length: payload.len(),
            },
            payload,
        };
        let aad = packet.header_to_bytes_aad_with_payload_len(packet.payload.len() + 16);

        let encrypted = protection
            .protect_with_packet_number(packet.header.packet_number, &aad, &packet.payload)
            .map_err(|e| format!("Handshake encrypt failed: {}", e))?;

        let mut packet_bytes = aad;
        packet_bytes.extend_from_slice(&encrypted);
        let pn_offset = packet::get_packet_number_offset(&packet_bytes, 0)?;
        protection
            .protect_header(&mut packet_bytes, pn_offset, packet.header.pn_length)
            .map_err(|e| format!("Handshake header protection failed: {}", e))?;

        Ok(packet_bytes)
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

            let mut packet_bytes = buf[..n].to_vec();
            let pn_offset = packet::get_packet_number_offset(&packet_bytes, 0)?;
            hs_protection
                .unprotect_header(&mut packet_bytes, pn_offset)
                .map_err(|e| format!("Handshake header protection failed: {}", e))?;

            let (pkt, _) = QuicPacket::from_bytes(&packet_bytes)
                .map_err(|e| format!("Packet parse error: {}", e))?;

            // Client sends Finished in Handshake-level packets
            if pkt.header.packet_type != PacketType::Handshake {
                continue;
            }

            // Decrypt
            let aad = pkt.header_to_bytes_aad();
            let plaintext = hs_protection
                .unprotect(&aad, pkt.header.packet_number, &pkt.payload)
                .map_err(|e| format!("Handshake decrypt failed: {}", e))?;

            // Parse CRYPTO frame
            if let Some((crypto_data, _)) = super::crypto_frame::parse_crypto_frame(&plaintext) {
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
    /// use edgerun_rt::Runtime;
    /// use edgerun_http::runtime::sync::Arc;
    ///
    /// let rt = Runtime::new_multi_thread().enable_all().build().unwrap();
    /// rt.block_on(async {
    ///     let cert = generate_self_signed(&["localhost"]).unwrap();
    ///     let server = Http3Server::bind("127.0.0.1:4433", cert).await.unwrap();
    ///     let handler = into_handler(|_req| {
    ///         Response::text(StatusCode::new(200).unwrap(), "Hello!")
    ///     });
    ///     let shutdown = edgerun_rt::CancellationToken::new();
    ///     server.serve(Arc::new(handler), shutdown).await.unwrap();
    /// });
    /// ```
    pub async fn serve(
        &self,
        handler: Arc<dyn Handler>,
        shutdown: CancellationToken,
    ) -> crate::runtime::io::Result<()> {
        edgerun_log::info!("HTTP/3 server listening on {} (h3)", self.local_addr()?);
        loop {
            if shutdown.is_cancelled() {
                edgerun_log::info!("HTTP/3 server shutting down");
                return Ok(());
            }

            match runtime::timeout(
                crate::runtime::time::Duration::from_millis(100),
                self.accept(),
            )
            .await
            {
                Ok(Ok((mut conn, client_addr))) => {
                    if let Err(e) =
                        Self::handle_connection(&mut conn, Arc::clone(&handler), client_addr).await
                    {
                        edgerun_log::warn!("HTTP/3 connection error from {}: {}", client_addr, e);
                    }
                }
                Ok(Err(e)) => {
                    edgerun_log::warn!("HTTP/3 accept error: {}", e);
                    crate::runtime::sleep(crate::runtime::time::Duration::from_millis(100)).await;
                }
                Err(_) => {}
            }
        }
    }

    /// Handle a single HTTP/3 connection, dispatching requests to the handler.
    async fn handle_connection(
        conn: &mut Http3Connection,
        handler: Arc<dyn Handler>,
        _client_addr: SocketAddr,
    ) -> Result<(), String> {
        let (stream_id, method, uri, headers) = loop {
            match conn.accept_request().await {
                Ok(Some(result)) => break result,
                Ok(None) => continue,
                Err(e) => return Err(format!("accept_request: {:?}", e)),
            }
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

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::super::quic::QuicTlsHandshaker;
    use super::*;
    use edgerun_tls::certificate_gen::generate_self_signed;

    fn test_server(cert_and_key: CertificateAndKey) -> Http3Server {
        let socket = crate::runtime::net::UdpSocket::bind("127.0.0.1:0").expect("bind test socket");
        Http3Server {
            socket: Arc::new(crate::runtime::wrap_udp_socket(socket).expect("wrap test socket")),
            cert_and_key,
            pending: Mutex::new(Vec::new()),
            validation_state: Mutex::new(HashMap::new()),
        }
    }

    #[test]
    fn initial_response_uses_parseable_protected_long_header() {
        let cert = generate_self_signed(&["localhost"]).expect("generate cert");
        let server = test_server(cert.clone());
        let mut server_hs = QuicTlsServerHandshaker::new(cert);
        let client_hs = QuicTlsHandshaker::new("localhost");

        let client_dcid = vec![1, 2, 3, 4, 5, 6, 7, 8];
        let client_scid = vec![9, 10, 11, 12];
        let frame = QuicFrame::Crypto {
            offset: 0,
            data: b"server hello".to_vec(),
        };

        let packet_bytes = server
            .build_initial_response(&client_dcid, &client_scid, &frame, &server_hs)
            .expect("build initial response");
        let mut packet_bytes = packet_bytes;
        let mut protection = PacketProtection::new(&client_hs.initial_keys(&client_dcid));
        let pn_offset = packet::get_packet_number_offset(&packet_bytes, 0).expect("pn offset");
        protection
            .unprotect_header(&mut packet_bytes, pn_offset)
            .expect("unprotect header");
        let (packet, consumed) = QuicPacket::from_bytes(&packet_bytes).expect("parse packet");

        assert_eq!(consumed, packet_bytes.len());
        assert_eq!(packet.header.packet_type, PacketType::Initial);
        assert_eq!(packet.header.dst_cid, client_scid);
        assert_eq!(packet.header.src_cid, client_dcid);

        let plaintext = protection
            .unprotect(
                &packet.header_to_bytes_aad(),
                packet.header.packet_number,
                &packet.payload,
            )
            .expect("decrypt initial response");
        let (decoded, consumed) = QuicFrame::from_bytes(&plaintext).expect("parse crypto frame");
        assert_eq!(consumed, plaintext.len());
        match decoded {
            QuicFrame::Crypto { data, .. } => assert_eq!(data, b"server hello"),
            _ => panic!("expected CRYPTO frame"),
        }
    }

    #[test]
    fn server_initial_decrypt_uses_parsed_packet_number() {
        let cert = generate_self_signed(&["localhost"]).expect("generate cert");
        let client_hs = QuicTlsHandshaker::new("localhost");
        let server_hs = QuicTlsServerHandshaker::new(cert);
        let client_dcid = vec![1, 2, 3, 4, 5, 6, 7, 8];
        let client_scid = vec![9, 10, 11, 12];
        let frame = QuicFrame::Crypto {
            offset: 0,
            data: client_hs.initial_crypto_data().to_vec(),
        };
        let packet = QuicPacket::initial(
            QUIC_VERSION_V1,
            client_dcid.clone(),
            client_scid,
            Vec::new(),
            5,
            frame.to_bytes(),
        );
        let aad = packet.header_to_bytes_aad_with_payload_len(packet.payload.len() + 16);
        let client_keys = client_hs.initial_keys(&client_dcid);
        let mut client_protection = PacketProtection::new(&client_keys);
        let encrypted = client_protection
            .protect_with_packet_number(packet.header.packet_number, &aad, &packet.payload)
            .expect("encrypt initial");
        let mut packet_bytes = aad;
        packet_bytes.extend_from_slice(&encrypted);

        let (parsed, consumed) = QuicPacket::from_bytes(&packet_bytes).expect("parse packet");
        assert_eq!(consumed, packet_bytes.len());
        assert_eq!(parsed.header.packet_number, 5);

        let server_keys = server_hs.initial_keys(&client_dcid);
        let mut wrong_server_protection = PacketProtection::new(&server_keys);
        assert!(wrong_server_protection
            .unprotect(&parsed.header_to_bytes_aad(), 0, &parsed.payload)
            .is_err());

        let mut server_protection = PacketProtection::new(&server_keys);
        let plaintext = server_protection
            .unprotect(
                &parsed.header_to_bytes_aad(),
                parsed.header.packet_number,
                &parsed.payload,
            )
            .expect("decrypt initial");
        let (decoded, consumed) = QuicFrame::from_bytes(&plaintext).expect("parse crypto frame");
        assert_eq!(consumed, plaintext.len());
        match decoded {
            QuicFrame::Crypto { data, .. } => assert_eq!(data, client_hs.initial_crypto_data()),
            _ => panic!("expected CRYPTO frame"),
        }
    }

    #[test]
    fn handshake_response_uses_parseable_protected_long_header() {
        let cert = generate_self_signed(&["localhost"]).expect("generate cert");
        let server = test_server(cert.clone());
        let mut client_hs = QuicTlsHandshaker::new("localhost");
        let mut server_hs = QuicTlsServerHandshaker::new(cert);

        let client_dcid = vec![1, 2, 3, 4, 5, 6, 7, 8];
        let client_scid = vec![9, 10, 11, 12];
        let server_hello = server_hs
            .process_client_hello(client_hs.initial_crypto_data())
            .expect("server processes client hello");
        client_hs
            .process_initial_crypto(&server_hello)
            .expect("client processes server hello");

        let frame = QuicFrame::Crypto {
            offset: 0,
            data: b"encrypted extensions".to_vec(),
        };
        let packet_bytes = server
            .build_handshake_response(&client_dcid, &client_scid, &frame, &server_hs)
            .expect("build handshake response");
        let mut packet_bytes = packet_bytes;
        let mut protection =
            PacketProtection::new(&client_hs.handshake_keys().expect("client handshake keys"));
        let pn_offset = packet::get_packet_number_offset(&packet_bytes, 0).expect("pn offset");
        protection
            .unprotect_header(&mut packet_bytes, pn_offset)
            .expect("unprotect header");
        let (packet, consumed) = QuicPacket::from_bytes(&packet_bytes).expect("parse packet");

        assert_eq!(consumed, packet_bytes.len());
        assert_eq!(packet.header.packet_type, PacketType::Handshake);
        assert_eq!(packet.header.dst_cid, client_scid);
        assert_eq!(packet.header.src_cid, client_dcid);

        let plaintext = protection
            .unprotect(
                &packet.header_to_bytes_aad(),
                packet.header.packet_number,
                &packet.payload,
            )
            .expect("decrypt handshake response");
        let (decoded, consumed) = QuicFrame::from_bytes(&plaintext).expect("parse crypto frame");
        assert_eq!(consumed, plaintext.len());
        match decoded {
            QuicFrame::Crypto { data, .. } => assert_eq!(data, b"encrypted extensions"),
            _ => panic!("expected CRYPTO frame"),
        }
    }
}
