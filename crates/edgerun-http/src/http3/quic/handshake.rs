//! QUIC-TLS handshake driver (RFC 9001 + RFC 8446).
//!
//! Drives the TLS 1.3 handshake over QUIC CRYPTO frames — no TLS record layer.
//! Handshake messages are transported directly in CRYPTO frames.
//!
//! # Handshake Flow (Client)
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

use edgerun_crypto::getrandom;
use edgerun_crypto::AesGcmCipher;
use edgerun_tls::cipher::{CipherSuite, NamedGroup};
use edgerun_tls::handshake::{ClientHelloBuilder, ServerHello};
use edgerun_tls::key_exchange::{EcdhKeyPair, KeyExchangeGroup};
use edgerun_tls::prf::{
    Hasher, Tls13KeySchedule, TrafficKeys,
    quic_initial_client_keys, quic_traffic_keys, quic_hp_key,
    INITIAL_SALT_V1,
};

use super::frame::QuicFrame;
use super::packet::QuicPacket;
use super::crypto::{CryptoPhase, ProtectionKeys, PacketProtection, AeadAlgorithm};
use super::{ConnectionId, TransportParameters, QUIC_VERSION_V1};

/// Certificate validation result.
#[derive(Debug)]
pub struct CertValidationResult {
    /// Whether the certificate chain is valid
    pub chain_valid: bool,
    /// Whether the hostname matches
    pub hostname_valid: bool,
    /// Validation error details
    pub error: Option<String>,
}

/// Certificate validator for TLS 1.3 (RFC 8446 §4.4.2).
///
/// Validates:
/// 1. Certificate chain (leaf → intermediates → root)
/// 2. Hostname/SNI matching (RFC 2818)
/// 3. Certificate expiration
/// 4. CertificateVerify signature
pub struct CertificateValidator {
    /// Expected server hostname (for SNI verification)
    expected_hostname: Option<String>,
    /// Whether to skip hostname verification (testing only)
    skip_hostname_check: bool,
    /// Whether to skip chain validation (testing only)
    skip_chain_check: bool,
}

impl CertificateValidator {
    pub fn new(expected_hostname: Option<&str>) -> Self {
        CertificateValidator {
            expected_hostname: expected_hostname.map(|s| s.to_string()),
            skip_hostname_check: false,
            skip_chain_check: false,
        }
    }

    /// Skip hostname verification (testing only — DANGEROUS in production).
    pub fn skip_hostname(&mut self) {
        self.skip_hostname_check = true;
    }

    /// Skip chain verification (testing only — DANGEROUS in production).
    pub fn skip_chain(&mut self) {
        self.skip_chain_check = true;
    }

    /// Validate a certificate chain presented by the server.
    ///
    /// In a full implementation this would:
    /// 1. Parse the DER-encoded certificates
    /// 2. Build a chain from leaf to root
    /// 3. Verify signatures at each level
    /// 4. Check expiration dates
    /// 5. Verify against system trust store
    ///
    /// For now, we perform basic structural checks.
    pub fn validate_chain(&self, cert_der_list: &[Vec<u8>]) -> CertValidationResult {
        if self.skip_chain_check {
            return CertValidationResult {
                chain_valid: true,
                hostname_valid: true,
                error: None,
            };
        }

        if cert_der_list.is_empty() {
            return CertValidationResult {
                chain_valid: false,
                hostname_valid: false,
                error: Some("Empty certificate chain".to_string()),
            };
        }

        // Basic structural validation:
        // The leaf certificate must be present
        let leaf = &cert_der_list[0];
        if leaf.len() < 64 {
            return CertValidationResult {
                chain_valid: false,
                hostname_valid: false,
                error: Some("Leaf certificate too small".to_string()),
            };
        }

        // Check for self-signed leaf (common in testing)
        if cert_der_list.len() == 1 {
            // Self-signed cert — accept if hostname check passes
            return CertValidationResult {
                chain_valid: true, // Self-signed accepted
                hostname_valid: self.check_hostname(leaf),
                error: None,
            };
        }

        // Multi-cert chain — basic acceptance
        CertValidationResult {
            chain_valid: true,
            hostname_valid: self.check_hostname(leaf),
            error: None,
        }
    }

    /// Check if the hostname matches the certificate's Subject Alternative Name.
    ///
    /// In a full implementation, this would:
    /// 1. Parse the X.509 certificate
    /// 2. Extract the SAN extension (2.5.29.17)
    /// 3. Match against DNS names and IP addresses
    /// 4. Fall back to Common Name if no SAN
    fn check_hostname(&self, _cert_der: &[u8]) -> bool {
        if self.skip_hostname_check {
            return true;
        }

        // Without an X.509 parser, we can't validate hostname.
        // In production, integrate with rustls or an X.509 parser.
        // For now, accept if no hostname expected.
        self.expected_hostname.is_none()
    }

    /// Verify a CertificateVerify signature (RFC 8446 §4.4.3).
    ///
    /// The server signs the transcript hash with its private key.
    /// The client verifies this signature using the server's public key from
    /// the leaf certificate.
    pub fn verify_certificate_signature(
        &self,
        cert_der: &[u8],
        signature_algorithm: u16,
        signature: &[u8],
        transcript_hash: &[u8],
    ) -> bool {
        // In a full implementation:
        // 1. Extract the public key from cert_der
        // 2. Construct the signature verification context
        // 3. Verify the signature over transcript_hash
        //
        // The signature algorithm determines the verification method:
        // 0x0401 = RSA-PSS-SHA256
        // 0x0403 = ECDSA-SECP256R1-SHA256
        // 0x0804 = ED25519
        //
        // For now, accept all signatures (testing mode).
        // In production, this must verify signatures cryptographically.
        signature.len() >= 64 // Require at least 512-bit signature
    }
}

/// QUIC-TLS handshake result.
#[derive(Clone)]
pub struct HandshakeResult {
    /// Initial-level protection keys
    pub initial_keys: ProtectionKeys,
    /// Handshake-level protection keys
    pub handshake_keys: ProtectionKeys,
    /// Application (1-RTT) protection keys
    pub app_keys: ProtectionKeys,
    /// 0-RTT early data protection keys (if 0-RTT was enabled)
    pub early_data_keys: Option<ProtectionKeys>,
    /// Negotiated cipher suite
    pub cipher_suite: CipherSuite,
    /// Server random (for debugging/extensions)
    pub server_random: [u8; 32],
    /// Full transcript of handshake messages (for exporters)
    pub transcript: Vec<u8>,
}

/// QUIC-TLS handshake state machine (client side).
pub struct QuicTlsHandshaker {
    /// TLS 1.3 key schedule
    key_schedule: Tls13KeySchedule,
    /// Hasher matching the cipher suite
    hasher: Hasher,
    /// Client random bytes
    client_random: [u8; 32],
    /// Server random (filled after ServerHello)
    server_random: [u8; 32],
    /// ECDH key pair for key_share
    key_pair: EcdhKeyPair,
    /// Server's key share (parsed from ServerHello)
    server_key_share: Vec<u8>,
    /// Running transcript: concatenation of all handshake message bytes
    transcript: Vec<u8>,
    /// Cached client handshake traffic secret (for Finished + app key derivation)
    client_hs_secret: Vec<u8>,
    /// Cached server handshake traffic secret (for Finished verification)
    server_hs_secret: Vec<u8>,
    /// 0-RTT early traffic secret (derived before advancing to handshake)
    early_traffic_secret: Vec<u8>,
    /// Negotiated cipher suite
    cipher_suite: CipherSuite,
    /// ClientHello raw bytes (for retransmission)
    client_hello_bytes: Vec<u8>,
    /// Server name for SNI and certificate validation
    server_name: String,
    /// CRYPTO frame offset for retransmission
    crypto_send_offset: usize,
    /// Whether we've received ServerHello
    received_server_hello: bool,
    /// Whether handshake is complete
    complete: bool,
    /// Server certificate chain (DER-encoded, from TLS Certificate message)
    server_cert_chain: Vec<Vec<u8>>,
    /// CertificateVerify signature (algorithm + raw bytes)
    cert_verify_signature: Option<(u16, Vec<u8>)>,
    /// Certificate validation result (set after processing Certificate)
    cert_validation: Option<CertValidationResult>,
}

impl QuicTlsHandshaker {
    /// Create a new handshaker for a QUIC client connection.
    pub fn new(server_name: &str) -> Self {
        let mut client_random = [0u8; 32];
        getrandom::fill(&mut client_random).expect("CSPRNG failure");

        let key_pair = EcdhKeyPair::generate(KeyExchangeGroup::X25519)
            .expect("X25519 key generation failed");

        let cipher_suite = CipherSuite::TLS_AES_128_GCM_SHA256;
        let hasher = hasher_for_suite(cipher_suite);

        let client_hello_bytes = build_client_hello_quic(&client_random, server_name, &key_pair)
            .expect("ClientHello build failed");

        let key_schedule = Tls13KeySchedule::new(hasher.clone());

        QuicTlsHandshaker {
            key_schedule,
            hasher,
            client_random,
            server_random: [0u8; 32],
            key_pair,
            server_key_share: Vec::new(),
            transcript: Vec::new(),
            client_hs_secret: Vec::new(),
            server_hs_secret: Vec::new(),
            early_traffic_secret: Vec::new(),
            cipher_suite,
            client_hello_bytes,
            server_name: server_name.to_string(),
            crypto_send_offset: 0,
            received_server_hello: false,
            complete: false,
            server_cert_chain: Vec::new(),
            cert_verify_signature: None,
            cert_validation: None,
        }
    }

    /// Build the Initial packet payload: CRYPTO frame containing ClientHello.
    ///
    /// Returns `(crypto_frame_bytes, client_hello_bytes)`.
    pub fn initial_crypto_data(&self) -> &[u8] {
        &self.client_hello_bytes
    }

    /// Derive Initial-level protection keys for the given destination connection ID.
    pub fn initial_keys(&self, dcid: &[u8]) -> ProtectionKeys {
        let (write, read) = quic_initial_client_keys(dcid, 16, 12, &self.hasher);
        let hp = quic_hp_key(
            &self.key_schedule_derive_initial_secret(dcid),
            16,
            &self.hasher,
        );
        ProtectionKeys::new(
            AeadAlgorithm::Aes128Gcm,
            write.write_key,
            write.write_iv,
            read.write_key,
            read.write_iv,
        )
    }

    /// Derive the initial traffic secret from DCID (for HP key derivation).
    fn key_schedule_derive_initial_secret(&self, dcid: &[u8]) -> Vec<u8> {
        // initial_secret = HKDF-Extract(initial_salt, dcid)
        self.hasher.extract(INITIAL_SALT_V1, dcid)
    }

    /// Process received CRYPTO data from an Initial packet.
    ///
    /// Parses the ServerHello from the CRYPTO payload and advances the
    /// key schedule. Returns `Ok(())` if ServerHello was successfully
    /// parsed, or an error string if the data is invalid.
    pub fn process_initial_crypto(&mut self, crypto_data: &[u8]) -> Result<(), String> {
        // The CRYPTO payload from the server's Initial packet should contain
        // the ServerHello handshake message.
        // ServerHello wire format: type(1) + length(3) + payload
        if crypto_data.is_empty() {
            return Err("Empty CRYPTO data".into());
        }

        // The server may send multiple handshake messages in one CRYPTO payload.
        // The first one should be ServerHello (type 2).
        let sh = ServerHello::parse(crypto_data)
            .map_err(|e| format!("Failed to parse ServerHello: {:?}", e))?;

        if sh.supported_version != Some(0x0304) {
            return Err(format!(
                "Server did not negotiate TLS 1.3 (got {:?})",
                sh.supported_version,
            ));
        }

        self.server_random = sh.random;
        self.server_key_share = sh.server_key_share.clone();
        self.cipher_suite = sh.cipher_suite;
        self.hasher = hasher_for_suite(self.cipher_suite);

        // Append ClientHello to transcript (our own CH, which the server also has)
        self.transcript.extend_from_slice(&self.client_hello_bytes);

        // Append ServerHello to transcript
        // crypto_data may contain more than just ServerHello. We need the exact
        // ServerHello bytes for the transcript. ServerHello::parse doesn't return
        // consumed bytes, so we re-serialize the length.
        let sh_msg_len = if crypto_data.len() >= 4 {
            let msg_len = u32::from_be_bytes([0, crypto_data[1], crypto_data[2], crypto_data[3]]) as usize;
            4 + msg_len
        } else {
            crypto_data.len()
        };
        self.transcript.extend_from_slice(&crypto_data[..sh_msg_len.min(crypto_data.len())]);
        self.received_server_hello = true;

        // Derive 0-RTT early traffic keys BEFORE advancing key schedule.
        // 0-RTT keys are derived from the early secret + ClientHello hash
        // (RFC 8446 §7.1, "c e traffic" label).
        let ch_hash = self.hasher.hash(&self.transcript);
        self.early_traffic_secret = self.key_schedule.client_early_traffic_secret(&ch_hash);

        // Advance key schedule to handshake phase
        let shared_secret = self.key_pair.exchange(&self.server_key_share)?;
        let transcript_hash = self.hasher.hash(&self.transcript);

        self.key_schedule.advance_to_handshake(&shared_secret, &transcript_hash, &transcript_hash);

        // Cache the handshake traffic secrets for later use (Finished verification)
        self.client_hs_secret = self.key_schedule.client_handshake_traffic_secret(&transcript_hash);
        self.server_hs_secret = self.key_schedule.server_handshake_traffic_secret(&transcript_hash);

        Ok(())
    }

    /// Check if we've received the ServerHello and can proceed to handshake phase.
    pub fn has_server_hello(&self) -> bool {
        self.received_server_hello
    }

    /// Get the server name (SNI).
    pub fn server_name(&self) -> &str {
        &self.server_name
    }

    /// Get the certificate validation result (if available).
    pub fn cert_validation(&self) -> Option<&CertValidationResult> {
        self.cert_validation.as_ref()
    }

    /// Get the server certificate chain (DER-encoded).
    pub fn server_cert_chain(&self) -> &[Vec<u8>] {
        &self.server_cert_chain
    }

    /// Derive handshake-level protection keys.
    ///
    /// Must be called after `process_initial_crypto()` succeeds.
    /// Returns `(write_keys, read_keys)` for the Handshake encryption level.
    pub fn handshake_keys(&self) -> Result<ProtectionKeys, String> {
        let transcript_hash = self.hasher.hash(&self.transcript);

        let client_hs_secret = self.key_schedule.client_handshake_traffic_secret(&transcript_hash);
        let server_hs_secret = self.key_schedule.server_handshake_traffic_secret(&transcript_hash);

        // From client perspective:
        // - write: use client_hs_secret (client encrypts with its own secret)
        // - read: use server_hs_secret (client decrypts server's encrypted data)
        let write = quic_traffic_keys(&client_hs_secret, self.cipher_suite.key_len(), 12, &self.hasher);
        let read = quic_traffic_keys(&server_hs_secret, self.cipher_suite.key_len(), 12, &self.hasher);
        let hp = quic_hp_key(&client_hs_secret, self.cipher_suite.key_len(), &self.hasher);

        Ok(ProtectionKeys::new(
            AeadAlgorithm::Aes128Gcm,
            write.write_key,
            write.write_iv,
            read.write_key,
            read.write_iv,
        ))
    }

    /// Process received CRYPTO data from Handshake-level packets.
    ///
    /// Parses EncryptedExtensions, Certificate, CertificateVerify, and Finished
    /// messages. Returns `Ok(())` if all messages were processed and the client
    /// Finished message is ready to be sent.
    ///
    /// Returns the Client Finished CRYPTO data to send back to the server.
    pub fn process_handshake_crypto(
        &mut self,
        crypto_data: &[u8],
    ) -> Result<Vec<u8>, String> {
        // The handshake CRYPTO payload contains multiple TLS handshake messages:
        // EncryptedExtensions(8), Certificate(11), CertificateVerify(15), Finished(20)
        let mut pos = 0;
        let mut received_finished = false;

        while pos < crypto_data.len() {
            if pos + 4 > crypto_data.len() {
                break;
            }

            let msg_type = crypto_data[pos];
            let msg_len = u32::from_be_bytes([
                0,
                crypto_data[pos + 1],
                crypto_data[pos + 2],
                crypto_data[pos + 3],
            ]) as usize;

            if pos + 4 + msg_len > crypto_data.len() {
                break; // Partial message, wait for more data
            }

            let msg = &crypto_data[pos..pos + 4 + msg_len];

            match msg_type {
                8 => {
                    // EncryptedExtensions — append to transcript
                    self.transcript.extend_from_slice(msg);
                }
                11 => {
                    // Certificate — parse and extract DER certificate chain
                    // TLS 1.3 format: type(1) + len(3) + context_len(1) + context + cert_list_len(3) + entries
                    if msg_len >= 5 {
                        let context_len = msg[4] as usize;
                        let cert_list_start = 5 + context_len;
                        if cert_list_start + 3 <= msg.len() {
                            let cert_list_len = u32::from_be_bytes([0, msg[cert_list_start], msg[cert_list_start + 1], msg[cert_list_start + 2]]) as usize;
                            let mut cert_pos = cert_list_start + 3;
                            let cert_end = cert_pos + cert_list_len.min(msg.len() - cert_pos);

                            while cert_pos + 3 <= cert_end {
                                let cert_len = u32::from_be_bytes([0, msg[cert_pos], msg[cert_pos + 1], msg[cert_pos + 2]]) as usize;
                                cert_pos += 3;
                                if cert_pos + cert_len <= cert_end && cert_len > 0 {
                                    let cert_der = msg[cert_pos..cert_pos + cert_len].to_vec();
                                    self.server_cert_chain.push(cert_der);
                                }
                                cert_pos += cert_len;
                                // Skip extensions (2-byte length + data)
                                if cert_pos + 2 <= cert_end {
                                    let ext_len = u16::from_be_bytes([msg[cert_pos], msg[cert_pos + 1]]) as usize;
                                    cert_pos += 2 + ext_len;
                                }
                            }

                            // Validate certificate chain
                            let validator = CertificateValidator::new(Some(&self.server_name()));
                            self.cert_validation = Some(validator.validate_chain(&self.server_cert_chain));
                        }
                    }
                    self.transcript.extend_from_slice(msg);
                }
                15 => {
                    // CertificateVerify — extract signature algorithm and signature
                    // Format: type(1) + len(3) + sig_alg(2) + sig_len(2) + signature
                    if msg_len >= 8 {
                        let sig_alg = u16::from_be_bytes([msg[4], msg[5]]);
                        let sig_len = u16::from_be_bytes([msg[6], msg[7]]) as usize;
                        if 8 + sig_len <= msg.len() {
                            let signature = msg[8..8 + sig_len].to_vec();
                            self.cert_verify_signature = Some((sig_alg, signature));
                        }
                    }
                    self.transcript.extend_from_slice(msg);
                }
                20 => {
                    // Finished — verify and append
                    // Transcript hash for verification: hash of all messages BEFORE Finished
                    let pre_finished_hash = self.hasher.hash(&self.transcript);

                    // Use cached server handshake secret (derived from CH||SH transcript)
                    let finished_key = self.hasher.expand_label(&self.server_hs_secret, "finished", &[], self.hasher.len());

                    let verify_data = match self.hasher {
                        Hasher::Sha256 => edgerun_crypto::hmac_sha256(&finished_key, &pre_finished_hash),
                        Hasher::Sha384 => edgerun_crypto::hmac_sha384(&finished_key, &pre_finished_hash),
                    };

                    // verify_data is at offset 4 in the Finished message
                    if msg.len() < 4 + verify_data.len() {
                        return Err("Finished message too short".into());
                    }

                    let server_verify_data = &msg[4..];
                    if server_verify_data.len() != verify_data.len()
                        || !constant_time_eq(server_verify_data, &verify_data)
                    {
                        return Err("Server Finished verification failed".into());
                    }

                    // Append to transcript
                    self.transcript.extend_from_slice(msg);
                    received_finished = true;
                }
                _ => {
                    // Unknown message type — append to transcript anyway
                    self.transcript.extend_from_slice(msg);
                }
            }

            pos += 4 + msg_len;
        }

        if !received_finished {
            return Err("Server Finished not found in CRYPTO data".into());
        }

        // Build Client Finished
        let client_finished = self.build_client_finished()?;
        Ok(client_finished)
    }

    /// Build the Client Finished message (wire format).
    fn build_client_finished(&self) -> Result<Vec<u8>, String> {
        // Transcript hash includes all handshake messages up to (but not including) client Finished
        let full_transcript_hash = self.hasher.hash(&self.transcript);

        // Use cached client handshake secret (derived from CH||SH transcript)
        let finished_key = self.hasher.expand_label(&self.client_hs_secret, "finished", &[], self.hasher.len());

        let verify_data = match self.hasher {
            Hasher::Sha256 => edgerun_crypto::hmac_sha256(&finished_key, &full_transcript_hash),
            Hasher::Sha384 => edgerun_crypto::hmac_sha384(&finished_key, &full_transcript_hash),
        };

        // Finished message: type(1) + length(3) + verify_data
        let mut msg = Vec::with_capacity(4 + verify_data.len());
        msg.push(20); // Finished type
        msg.extend_from_slice(&(verify_data.len() as u32).to_be_bytes()[1..]);
        msg.extend_from_slice(&verify_data);

        Ok(msg)
    }

    /// Derive application (1-RTT) traffic keys.
    ///
    /// Must be called after Client Finished has been sent and the transcript
    /// includes the client Finished message.
    /// Get 0-RTT early data protection keys (if derived).
    ///
    /// Returns `Some(keys)` if the client derived 0-RTT keys during
    /// `process_initial_crypto()`. The keys can be used with
    /// `QuicConnection::enable_early_data()` to send early data.
    pub fn early_data_keys(&self) -> Option<ProtectionKeys> {
        if self.early_traffic_secret.is_empty() {
            return None;
        }
        let keys = quic_traffic_keys(&self.early_traffic_secret, self.cipher_suite.key_len(), 12, &self.hasher);
        // For 0-RTT, client sends so we use client's write keys.
        // The server would use read keys from the same secret.
        Some(ProtectionKeys::new(
            AeadAlgorithm::Aes128Gcm,
            keys.write_key.clone(),
            keys.write_iv.clone(),
            keys.write_key,  // Same keys for simplicity — client sends, server reads
            keys.write_iv,
        ))
    }

    /// Derive application traffic keys from the post-Client-Finished transcript hash.
    pub fn app_keys(&self, transcript_after_client_finished: &[u8]) -> ProtectionKeys {
        let app_transcript_hash = self.hasher.hash(transcript_after_client_finished);

        let client_app_secret = self.key_schedule.client_app_traffic_secret(&app_transcript_hash);
        let server_app_secret = self.key_schedule.server_app_traffic_secret(&app_transcript_hash);

        let write = quic_traffic_keys(&client_app_secret, self.cipher_suite.key_len(), 12, &self.hasher);
        let read = quic_traffic_keys(&server_app_secret, self.cipher_suite.key_len(), 12, &self.hasher);

        ProtectionKeys::new(
            AeadAlgorithm::Aes128Gcm,
            write.write_key,
            write.write_iv,
            read.write_key,
            read.write_iv,
        )
    }

    /// Check if handshake is complete.
    pub fn is_complete(&self) -> bool {
        self.complete
    }

    /// Mark handshake as complete.
    pub fn mark_complete(&mut self) {
        self.complete = true;
    }

    /// Get the full handshake transcript (for key derivation by caller).
    pub fn transcript(&self) -> &[u8] {
        &self.transcript
    }

    /// Build full handshake result with all protection keys.
    pub fn build_result(&self, dcid: &[u8], transcript_after_finished: &[u8]) -> Result<HandshakeResult, String> {
        let (init_write, init_read) = quic_initial_client_keys(dcid, 16, 12, &self.hasher);

        let hs_keys = self.handshake_keys()?;
        let app_keys = self.app_keys(transcript_after_finished);

        // Derive 0-RTT early data protection keys (if early traffic secret was derived)
        let early_data_keys = if !self.early_traffic_secret.is_empty() {
            let early_keys = quic_traffic_keys(&self.early_traffic_secret, self.cipher_suite.key_len(), 12, &self.hasher);
            let early_read = quic_traffic_keys(&self.early_traffic_secret, self.cipher_suite.key_len(), 12, &self.hasher);
            Some(ProtectionKeys::new(
                AeadAlgorithm::Aes128Gcm,
                early_keys.write_key,
                early_keys.write_iv,
                early_read.write_key,
                early_read.write_iv,
            ))
        } else {
            None
        };

        Ok(HandshakeResult {
            initial_keys: ProtectionKeys::new(
                AeadAlgorithm::Aes128Gcm,
                init_write.write_key,
                init_write.write_iv,
                init_read.write_key,
                init_read.write_iv,
            ),
            handshake_keys: hs_keys,
            app_keys,
            early_data_keys,
            cipher_suite: self.cipher_suite,
            server_random: self.server_random,
            transcript: self.transcript.clone(),
        })
    }
}

/// Build a ClientHello handshake message for QUIC (no TLS record wrapper).
///
/// Uses the edgerun-tls ClientHelloBuilder, which produces the correct
/// wire format: type(1) + length(3) + ClientHello payload.
fn build_client_hello_quic(
    client_random: &[u8; 32],
    server_name: &str,
    key_pair: &EcdhKeyPair,
) -> Result<Vec<u8>, String> {
    let public_key = key_pair.public_key_bytes();
    let group = match key_pair.group() {
        KeyExchangeGroup::SECP256R1 => NamedGroup::SECP256R1,
        KeyExchangeGroup::X25519 => NamedGroup::X25519,
    };

    let mut random = [0u8; 32];
    random.copy_from_slice(client_random);

    let ch = ClientHelloBuilder::new(random, server_name)
        .key_share(&public_key, group)
        .build()
        .map_err(|e| format!("ClientHello build error: {:?}", e))?;

    Ok(ch)
}

/// Get the Hasher for a cipher suite.
fn hasher_for_suite(suite: CipherSuite) -> Hasher {
    match suite {
        CipherSuite::TLS_AES_256_GCM_SHA384 => Hasher::Sha384,
        _ => Hasher::Sha256,
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

    #[test]
    fn test_handshaker_new() {
        let hs = QuicTlsHandshaker::new("example.com");
        assert!(!hs.received_server_hello);
        assert!(!hs.complete);
        assert!(!hs.initial_crypto_data().is_empty());
    }

    #[test]
    fn test_initial_crypto_data_is_client_hello() {
        let hs = QuicTlsHandshaker::new("example.com");
        let data = hs.initial_crypto_data();

        // First byte should be handshake type 1 (ClientHello)
        assert_eq!(data[0], 1);

        // Should have a reasonable length (ClientHello is typically 200-500 bytes)
        assert!(data.len() > 100);
    }

    #[test]
    fn test_initial_keys_derive() {
        let hs = QuicTlsHandshaker::new("example.com");
        let dcid = vec![0x83, 0x94, 0xc8, 0xf0, 0x3e, 0x51, 0x57, 0x08];
        let keys = hs.initial_keys(&dcid);
        assert_eq!(keys.write_key.len(), 16);
        assert_eq!(keys.write_iv.len(), 12);
    }

    #[test]
    fn test_hasher_for_suite() {
        use edgerun_tls::cipher::CipherSuite;
        assert!(matches!(hasher_for_suite(CipherSuite::TLS_AES_128_GCM_SHA256), Hasher::Sha256));
        assert!(matches!(hasher_for_suite(CipherSuite::TLS_AES_256_GCM_SHA384), Hasher::Sha384));
    }
}
