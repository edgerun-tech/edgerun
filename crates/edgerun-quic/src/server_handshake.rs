//! QUIC-TLS handshake driver — server side (RFC 9001 + RFC 8446).
//!
//! Drives the TLS 1.3 server handshake over QUIC CRYPTO frames.
//!
//! # Handshake Flow (Server)
//! ```text
//! Client                                          Server
//! ------                                          ------
//! Initial: CRYPTO( ClientHello )                →
//!                                       ←  Initial: CRYPTO( ServerHello )
//!                                       ←  Handshake: CRYPTO( EncryptedExtensions )
//!                                       ←  Handshake: CRYPTO( Certificate )
//!                                       ←  Handshake: CRYPTO( CertificateVerify )
//!                                       ←  Handshake: CRYPTO( Finished )
//! Handshake: CRYPTO( ClientFinished )   →
//!
//! [1-RTT packets with HTTP/3 data]      ↔     [1-RTT packets]
//! ```

use alloc::{
    format,
    string::{String, ToString},
    sync::Arc,
    vec,
    vec::Vec,
};
use edgerun_crypto::getrandom;
use edgerun_crypto::CipherSuite;
use edgerun_tls::cipher::NamedGroup;
use edgerun_tls::key_exchange::{EcdhKeyPair, KeyExchangeGroup};
use edgerun_tls::prf::{
    quic_hp_key, quic_initial_server_keys, quic_traffic_keys, Hasher, Tls13KeySchedule,
    TrafficKeys, INITIAL_SALT_V1,
};
use edgerun_tls::server::client_hello::ClientHello;
use edgerun_tls::server::message_builder::{
    build_certificate_message, build_certificate_verify, build_encrypted_extensions,
    build_finished_message, build_server_hello, compute_server_finished_verify_data,
};

use super::crypto::ProtectionKeys;
use crate::ConnectionId;

#[derive(Clone)]
pub struct CertificateAndKey {
    pub cert_der: Vec<u8>,
    pub signing_key: Arc<edgerun_crypto::p256::ecdsa::SigningKey>,
}

impl CertificateAndKey {
    pub fn from_der(cert_der: Vec<u8>, signing_key: edgerun_crypto::p256::ecdsa::SigningKey) -> Self {
        Self {
            cert_der,
            signing_key: Arc::new(signing_key),
        }
    }
}

impl From<edgerun_tls::CertificateAndKey> for CertificateAndKey {
    fn from(value: edgerun_tls::CertificateAndKey) -> Self {
        Self {
            cert_der: value.cert_der,
            signing_key: value.signing_key,
        }
    }
}

/// 0-RTT early data state (RFC 9001 §4.6).
#[derive(Debug, Clone)]
pub struct EarlyDataState {
    /// Whether the server accepted 0-RTT
    pub accepted: bool,
    /// Maximum allowed 0-RTT data size
    pub max_early_data_size: u64,
    /// Actual 0-RTT data received (if any)
    pub data: Option<Vec<u8>>,
}

impl Default for EarlyDataState {
    fn default() -> Self {
        Self::new()
    }
}

impl EarlyDataState {
    pub fn new() -> Self {
        EarlyDataState {
            accepted: false,
            max_early_data_size: 0,
            data: None,
        }
    }

    pub fn with_max_size(max_size: u64) -> Self {
        EarlyDataState {
            accepted: true,
            max_early_data_size: max_size,
            data: None,
        }
    }
}

/// Server-side QUIC-TLS handshake result.
#[derive(Clone)]
pub struct ServerHandshakeResult {
    /// Initial-level protection keys
    pub initial_keys: ProtectionKeys,
    /// Handshake-level protection keys
    pub handshake_keys: ProtectionKeys,
    /// Application (1-RTT) protection keys
    pub app_keys: ProtectionKeys,
    /// 0-RTT early data protection keys (if client offered early data)
    pub early_data_keys: Option<ProtectionKeys>,
    /// Negotiated cipher suite
    pub cipher_suite: CipherSuite,
    /// Client random bytes
    pub client_random: [u8; 32],
    /// SNI server name (if provided by client)
    pub server_name: Option<String>,
    /// Negotiated ALPN protocol (e.g., Some(b"h3"))
    pub negotiated_alpn: Option<Vec<u8>>,
    /// 0-RTT early data state
    pub early_data: EarlyDataState,
    /// Full transcript of handshake messages
    pub transcript: Vec<u8>,
}

/// Server-side QUIC-TLS handshake state machine.
pub struct QuicTlsServerHandshaker {
    /// TLS 1.3 key schedule
    key_schedule: Option<Tls13KeySchedule>,
    /// Hasher matching the cipher suite
    hasher: Hasher,
    /// Server's ECDH key pair
    key_pair: EcdhKeyPair,
    /// Certificate and signing key
    cert_and_key: CertificateAndKey,
    /// Negotiated cipher suite
    cipher_suite: CipherSuite,
    /// Client's random bytes (from ClientHello)
    client_random: [u8; 32],
    /// Server's random bytes
    server_random: [u8; 32],
    /// Client's key share
    client_key_share: Vec<u8>,
    /// Client's key share group
    client_key_share_group: KeyExchangeGroup,
    /// Running transcript
    transcript: Vec<u8>,
    /// SNI server name
    server_name: Option<String>,
    /// Negotiated ALPN protocol (None if not yet negotiated)
    negotiated_alpn: Option<Vec<u8>>,
    /// Whether handshake is complete
    complete: bool,
    /// Session ID (echoed from ClientHello)
    session_id: Vec<u8>,
    /// 0-RTT early traffic secret (derived from ClientHello hash)
    server_early_traffic_secret: Option<Vec<u8>>,
}

impl QuicTlsServerHandshaker {
    /// Create a new server handshaker with the given certificate.
    pub fn new(cert_and_key: impl Into<CertificateAndKey>) -> Self {
        let mut server_random = [0u8; 32];
        getrandom(&mut server_random).expect("CSPRNG failure");

        let key_pair =
            EcdhKeyPair::generate(KeyExchangeGroup::X25519).expect("X25519 key generation failed");
        let cert_and_key = cert_and_key.into();

        QuicTlsServerHandshaker {
            key_schedule: None,
            hasher: Hasher::Sha256,
            key_pair,
            cert_and_key,
            cipher_suite: CipherSuite::TLS_AES_128_GCM_SHA256,
            client_random: [0u8; 32],
            server_random,
            client_key_share: Vec::new(),
            client_key_share_group: KeyExchangeGroup::X25519,
            transcript: Vec::new(),
            server_name: None,
            negotiated_alpn: None,
            complete: false,
            session_id: Vec::new(),
            server_early_traffic_secret: None,
        }
    }

    /// Derive Initial-level protection keys for the given client destination connection ID.
    pub fn initial_keys(&self, client_dcid: &[u8]) -> ProtectionKeys {
        let (write, read) = quic_initial_server_keys(client_dcid, 16, 12, &self.hasher);
        ProtectionKeys::new(
            CipherSuite::TLS_AES_128_GCM_SHA256,
            write.write_key,
            write.write_iv,
            read.write_key,
            read.write_iv,
        )
    }

    /// Process the client's Initial CRYPTO data (ClientHello).
    ///
    /// Returns the ServerHello bytes to send back in an Initial packet.
    pub fn process_client_hello(&mut self, crypto_data: &[u8]) -> Result<Vec<u8>, String> {
        let ch = ClientHello::parse(crypto_data)
            .map_err(|e| format!("Failed to parse ClientHello: {:?}", e))?;

        // Verify TLS 1.3 is supported
        if !ch.supported_versions.contains(&0x0304) {
            return Err("Client does not support TLS 1.3".into());
        }

        // Select cipher suite (first mutual preference)
        let client_suites = &ch.cipher_suites;
        let server_suites = CipherSuite::client_default();
        self.cipher_suite = server_suites
            .iter()
            .find(|s| client_suites.contains(s))
            .copied()
            .ok_or_else(|| "No mutual cipher suite".to_string())?;

        self.hasher = match self.cipher_suite {
            CipherSuite::TLS_AES_256_GCM_SHA384 => Hasher::Sha384,
            _ => Hasher::Sha256,
        };

        // Select key exchange group
        self.client_key_share_group = ch
            .supported_groups
            .iter()
            .find(|g| matches!(g, NamedGroup::X25519 | NamedGroup::SECP256R1))
            .map(|g| match g {
                NamedGroup::SECP256R1 => KeyExchangeGroup::SECP256R1,
                NamedGroup::X25519 => KeyExchangeGroup::X25519,
                NamedGroup::SECP384R1 => KeyExchangeGroup::SECP256R1, // fallback
            })
            .unwrap_or(KeyExchangeGroup::X25519);

        self.client_key_share = ch
            .client_key_share
            .clone()
            .ok_or_else(|| "No client key_share".to_string())?;

        self.client_random = ch.random;
        self.server_name = ch.server_name.clone();
        self.session_id = ch.session_id.clone();

        // Negotiate ALPN — server prefers "h3" for HTTP/3
        if !ch.alpn_protocols.is_empty() {
            // Server preferences: h3 first
            let server_prefs: &[&[u8]] = &[b"h3"];
            for preferred in server_prefs {
                if ch.alpn_protocols.iter().any(|p| p.as_slice() == *preferred) {
                    self.negotiated_alpn = Some(preferred.to_vec());
                    break;
                }
            }
        }

        // Build ServerHello
        let server_pub = self.key_pair.public_key_bytes();
        let sh_group = match self.client_key_share_group {
            KeyExchangeGroup::SECP256R1 => NamedGroup::SECP256R1,
            KeyExchangeGroup::X25519 => NamedGroup::X25519,
        };

        let sh_bytes = build_server_hello(
            self.server_random,
            &self.session_id,
            self.cipher_suite,
            &server_pub,
            sh_group,
        );

        // Append ClientHello to transcript
        self.transcript.extend_from_slice(crypto_data);
        // Append ServerHello to transcript
        self.transcript.extend_from_slice(&sh_bytes);

        // Advance key schedule
        let shared_secret = self.key_pair.exchange(&self.client_key_share)?;
        let transcript_hash = self.hasher.hash(&self.transcript);

        let mut ks = Tls13KeySchedule::new(self.hasher.clone());
        ks.advance_to_handshake(&shared_secret, &transcript_hash, &transcript_hash);

        // Derive server 0-RTT early traffic secret from ClientHello hash.
        // This allows the server to decrypt 0-RTT data from the client.
        // Must be derived BEFORE advancing past early secret to handshake.
        let ch_hash = self.hasher.hash(crypto_data);
        let early_secret = ks.client_early_traffic_secret(&ch_hash);

        self.key_schedule = Some(ks);
        self.server_early_traffic_secret = Some(early_secret);

        Ok(sh_bytes)
    }

    /// Derive handshake-level protection keys.
    ///
    /// Must be called after `process_client_hello()` succeeds.
    pub fn handshake_keys(&self) -> Result<ProtectionKeys, String> {
        let ks = self
            .key_schedule
            .as_ref()
            .ok_or_else(|| "Key schedule not initialized".to_string())?;

        let transcript_hash = self.hasher.hash(&self.transcript);

        let client_hs_secret = ks.client_handshake_traffic_secret(&transcript_hash);
        let server_hs_secret = ks.server_handshake_traffic_secret(&transcript_hash);

        // From server perspective:
        // - write: use server_hs_secret (server encrypts with its own secret)
        // - read: use client_hs_secret (server decrypts client's encrypted data)
        let write = quic_traffic_keys(
            &server_hs_secret,
            self.cipher_suite.key_len(),
            12,
            &self.hasher,
        );
        let read = quic_traffic_keys(
            &client_hs_secret,
            self.cipher_suite.key_len(),
            12,
            &self.hasher,
        );

        Ok(ProtectionKeys::new(
            CipherSuite::TLS_AES_128_GCM_SHA256,
            write.write_key,
            write.write_iv,
            read.write_key,
            read.write_iv,
        ))
    }

    /// Build the encrypted handshake messages (EE, Cert, CertVerify, Finished)
    /// to send in Handshake-level packets.
    ///
    /// Returns a tuple of:
    /// - `(handshake_crypto_data, client_finished_verify_data)` — the data to send and the expected client verify data
    pub fn build_encrypted_handshake(&self) -> Result<(Vec<u8>, Vec<u8>), String> {
        let ks = self.key_schedule.as_ref().ok_or_else(|| {
            "Key schedule not initialized — process_client_hello first".to_string()
        })?;

        let transcript_hash = self.hasher.hash(&self.transcript);

        let mut output = Vec::new();

        // 1. EncryptedExtensions
        let ee = build_encrypted_extensions(self.negotiated_alpn.as_deref());
        output.extend_from_slice(&ee);

        // 2. Certificate
        let cert_msg = build_certificate_message(&self.cert_and_key.cert_der);
        output.extend_from_slice(&cert_msg);

        // 3. CertificateVerify
        let cv = build_certificate_verify(
            &self.transcript,
            &self.cert_and_key.signing_key,
            &self.hasher,
        )
        .map_err(|e| format!("CertificateVerify build failed: {:?}", e))?;
        output.extend_from_slice(&cv);

        // 4. Finished
        let server_hs_secret = ks.server_handshake_traffic_secret(&transcript_hash);
        // The pre-finished transcript hash is hash(CH || SH || EE || Cert || CertVerify)
        let pre_finished_transcript = self.transcript_with_messages(&[&ee, &cert_msg, &cv]);
        let pre_finished_hash = self.hasher.hash(&pre_finished_transcript);

        // Derive finished_key from server_hs_secret (RFC 8446 §4.4.4)
        let server_finished_key =
            self.hasher
                .expand_label(&server_hs_secret, "finished", &[], self.hasher.len());

        let verify_data = match self.hasher {
            Hasher::Sha256 => edgerun_crypto::hmac_sha256(&server_finished_key, &pre_finished_hash),
            Hasher::Sha384 => edgerun_crypto::hmac_sha384(&server_finished_key, &pre_finished_hash),
        };

        // Build the server's Finished message first
        let finished_msg = build_finished_message(&verify_data);
        output.extend_from_slice(&finished_msg);

        // For client Finished verification, we need to return the expected verify data
        // The client's Finished verify_data is computed over the transcript INCLUDING
        // the server's Finished message (RFC 8446 §4.4.4).
        let client_hs_secret = ks.client_handshake_traffic_secret(&transcript_hash);
        let client_finished_key =
            self.hasher
                .expand_label(&client_hs_secret, "finished", &[], self.hasher.len());

        // Build the full transcript including server's Finished for client's verify_data
        let mut full_transcript = pre_finished_transcript.clone();
        full_transcript.extend_from_slice(&finished_msg);
        let full_transcript_hash = self.hasher.hash(&full_transcript);

        let client_verify_data = match self.hasher {
            Hasher::Sha256 => {
                edgerun_crypto::hmac_sha256(&client_finished_key, &full_transcript_hash)
            }
            Hasher::Sha384 => {
                edgerun_crypto::hmac_sha384(&client_finished_key, &full_transcript_hash)
            }
        };

        Ok((output, client_verify_data))
    }

    /// Verify the client's Finished message.
    ///
    /// Returns `Ok(())` if the verify_data matches, or an error string.
    pub fn verify_client_finished(
        &self,
        client_verify_data: &[u8],
        expected: &[u8],
    ) -> Result<(), String> {
        if client_verify_data.len() != expected.len() {
            return Err("Client Finished verify_data length mismatch".into());
        }
        if !constant_time_eq(client_verify_data, expected) {
            return Err("Client Finished verification failed".into());
        }
        Ok(())
    }

    /// Build the full handshake result with all protection keys.
    ///
    /// `client_dcid` is the client's destination connection ID (our source CID).
    /// `transcript_after_finished` includes the client's Finished message.
    pub fn build_result(
        &self,
        client_dcid: &[u8],
        transcript_after_finished: &[u8],
    ) -> Result<ServerHandshakeResult, String> {
        let ks = self
            .key_schedule
            .as_ref()
            .ok_or_else(|| "Key schedule not initialized".to_string())?;

        let (init_write, init_read) = quic_initial_server_keys(client_dcid, 16, 12, &self.hasher);

        let transcript_hash = self.hasher.hash(self.transcript_without_client_finished());
        let client_hs_secret = ks.client_handshake_traffic_secret(&transcript_hash);
        let server_hs_secret = ks.server_handshake_traffic_secret(&transcript_hash);

        let hs_write = quic_traffic_keys(
            &server_hs_secret,
            self.cipher_suite.key_len(),
            12,
            &self.hasher,
        );
        let hs_read = quic_traffic_keys(
            &client_hs_secret,
            self.cipher_suite.key_len(),
            12,
            &self.hasher,
        );

        let app_transcript_hash = self.hasher.hash(transcript_after_finished);
        let client_app_secret = ks.client_app_traffic_secret(&app_transcript_hash);
        let server_app_secret = ks.server_app_traffic_secret(&app_transcript_hash);

        let app_write = quic_traffic_keys(
            &server_app_secret,
            self.cipher_suite.key_len(),
            12,
            &self.hasher,
        );
        let app_read = quic_traffic_keys(
            &client_app_secret,
            self.cipher_suite.key_len(),
            12,
            &self.hasher,
        );

        Ok(ServerHandshakeResult {
            initial_keys: ProtectionKeys::new(
                CipherSuite::TLS_AES_128_GCM_SHA256,
                init_write.write_key,
                init_write.write_iv,
                init_read.write_key,
                init_read.write_iv,
            ),
            handshake_keys: ProtectionKeys::new(
                CipherSuite::TLS_AES_128_GCM_SHA256,
                hs_write.write_key,
                hs_write.write_iv,
                hs_read.write_key,
                hs_read.write_iv,
            ),
            app_keys: ProtectionKeys::new(
                CipherSuite::TLS_AES_128_GCM_SHA256,
                app_write.write_key,
                app_write.write_iv,
                app_read.write_key,
                app_read.write_iv,
            ),
            early_data_keys: self.server_early_traffic_secret.as_ref().map(|secret| {
                // Server reads 0-RTT data from client, so client's write keys are server's read keys
                let read_keys =
                    quic_traffic_keys(secret, self.cipher_suite.key_len(), 12, &self.hasher);
                // Server would write 0-RTT response using the same secret (same direction)
                ProtectionKeys::new(
                    CipherSuite::TLS_AES_128_GCM_SHA256,
                    read_keys.write_key.clone(),
                    read_keys.write_iv.clone(),
                    read_keys.write_key,
                    read_keys.write_iv,
                )
            }),
            cipher_suite: self.cipher_suite,
            client_random: self.client_random,
            server_name: self.server_name.clone(),
            negotiated_alpn: self.negotiated_alpn.clone(),
            early_data: EarlyDataState::new(),
            transcript: self.transcript.clone(),
        })
    }

    /// Get the current transcript (CH || SH).
    pub fn transcript(&self) -> &[u8] {
        &self.transcript
    }

    /// Check if handshake is complete.
    pub fn is_complete(&self) -> bool {
        self.complete
    }

    /// Check if client key share was received and processed.
    pub fn has_client_key_share(&self) -> bool {
        !self.client_key_share.is_empty()
    }

    /// Mark handshake as complete.
    pub fn mark_complete(&mut self) {
        self.complete = true;
    }

    /// Compute transcript with additional messages appended.
    fn transcript_with_messages(&self, messages: &[&[u8]]) -> Vec<u8> {
        let mut t = self.transcript.clone();
        for msg in messages {
            t.extend_from_slice(msg);
        }
        t
    }

    /// Get the transcript without the client's Finished (for server key derivation).
    fn transcript_without_client_finished(&self) -> &[u8] {
        &self.transcript
    }
}

/// Constant-time equality comparison for cryptographic data.
fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

#[cfg(test)]
mod tests {
    use super::*;
    use edgerun_tls::certificate_gen::generate_self_signed;

    #[test]
    fn test_server_handshaker_new() {
        let cert = edgerun_tls::generate_self_signed(&["localhost", "127.0.0.1"]).unwrap();
        let hs = QuicTlsServerHandshaker::new(cert);
        assert!(!hs.complete);
    }

    #[test]
    fn test_server_initial_keys_derive() {
        let cert = edgerun_tls::generate_self_signed(&["localhost", "127.0.0.1"]).unwrap();
        let hs = QuicTlsServerHandshaker::new(cert);
        let client_dcid = vec![0x83, 0x94, 0xc8, 0xf0, 0x3e, 0x51, 0x57, 0x08];
        let keys = hs.initial_keys(&client_dcid);
        assert_eq!(keys.write_key.len(), 16);
        assert_eq!(keys.write_iv.len(), 12);
    }

    #[test]
    fn test_server_process_client_hello() {
        use edgerun_tls::cipher::NamedGroup as TlsNamedGroup;
        use edgerun_tls::handshake::ClientHelloBuilder;
        use edgerun_tls::key_exchange::{EcdhKeyPair, KeyExchangeGroup};

        let cert = edgerun_tls::generate_self_signed(&["localhost", "127.0.0.1"]).unwrap();
        let mut hs = QuicTlsServerHandshaker::new(cert);

        // Build a real ClientHello
        let mut client_random = [0u8; 32];
        getrandom(&mut client_random).unwrap();
        let client_kp = EcdhKeyPair::generate(KeyExchangeGroup::X25519).unwrap();
        let ch_bytes = ClientHelloBuilder::new(client_random, "localhost")
            .key_share(&client_kp.public_key_bytes(), TlsNamedGroup::X25519)
            .build()
            .unwrap();

        let sh_bytes = hs.process_client_hello(&ch_bytes).unwrap();

        // ServerHello should start with type 2
        assert_eq!(sh_bytes[0], 2);
        assert_eq!(hs.cipher_suite, CipherSuite::TLS_AES_128_GCM_SHA256);
        assert_eq!(hs.server_name, Some("localhost".to_string()));
        assert!(!hs.client_key_share.is_empty());
    }

    #[test]
    fn test_server_full_handshake_flow() {
        use edgerun_tls::cipher::NamedGroup as TlsNamedGroup;
        use edgerun_tls::handshake::ClientHelloBuilder;
        use edgerun_tls::key_exchange::{EcdhKeyPair, KeyExchangeGroup};

        let cert = edgerun_tls::generate_self_signed(&["localhost", "127.0.0.1"]).unwrap();
        let mut hs = QuicTlsServerHandshaker::new(cert);

        let mut client_random = [0u8; 32];
        getrandom(&mut client_random).unwrap();
        let client_kp = EcdhKeyPair::generate(KeyExchangeGroup::X25519).unwrap();
        let ch_bytes = ClientHelloBuilder::new(client_random, "localhost")
            .key_share(&client_kp.public_key_bytes(), TlsNamedGroup::X25519)
            .build()
            .unwrap();

        let sh_bytes = hs.process_client_hello(&ch_bytes).unwrap();
        assert_eq!(sh_bytes[0], 2); // ServerHello type

        // Build encrypted handshake messages
        let (handshake_data, expected_client_verify) = hs.build_encrypted_handshake().unwrap();
        assert!(!handshake_data.is_empty());
        assert_eq!(expected_client_verify.len(), 32); // SHA-256 verify_data

        // Verify the transcript contains CH || SH
        assert!(hs.transcript().len() > ch_bytes.len());
    }
}
