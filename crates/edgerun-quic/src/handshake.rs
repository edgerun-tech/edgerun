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

use alloc::{
    format,
    string::{String, ToString},
    vec,
    vec::Vec,
};
use edgerun_crypto::fill_random;
use edgerun_crypto::CipherSuite;
use edgerun_encoding::byteorder::{read_u16_be, read_u24_be};
use edgerun_tls::certificate::Certificate;
use edgerun_tls::cipher::NamedGroup;
use edgerun_tls::handshake::{ClientHelloBuilder, ServerHello};
use edgerun_tls::key_exchange::{EcdhKeyPair, KeyExchangeGroup};
use edgerun_tls::prf::{
    quic_hp_key, quic_initial_client_keys, quic_traffic_keys, Hasher, Tls13KeySchedule,
    TrafficKeys, INITIAL_SALT_V1,
};

use super::crypto::{CryptoPhase, PacketProtection, ProtectionKeys};
use super::frame::QuicFrame;
use super::packet::QuicPacket;
use crate::{ConnectionId, TransportParameters, QUIC_VERSION_V1};

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

impl CertValidationResult {
    pub fn is_valid(&self) -> bool {
        self.chain_valid && self.hostname_valid && self.error.is_none()
    }
}

/// Certificate validator for TLS 1.3 (RFC 8446 §4.4.2).
///
/// Validates:
/// 1. Certificate chain signatures and validity periods
/// 2. Hostname/SNI matching (RFC 2818)
/// 3. CertificateVerify signature
///
/// Production root trust is intentionally not faked: chains without a
/// configured trusted root are rejected by strict validation.
pub struct CertificateValidator {
    /// Expected server hostname (for SNI verification)
    expected_hostname: Option<String>,
    /// Trusted root certificates, parsed from configured DER roots.
    trusted_roots: Vec<Certificate>,
    /// Whether to skip hostname verification (testing only)
    skip_hostname_check: bool,
    /// Whether to skip chain validation (testing only)
    skip_chain_check: bool,
}

impl CertificateValidator {
    pub fn new(expected_hostname: Option<&str>) -> Self {
        CertificateValidator {
            expected_hostname: expected_hostname.map(|s| s.to_string()),
            trusted_roots: Vec::new(),
            skip_hostname_check: false,
            skip_chain_check: false,
        }
    }

    /// Configure trust roots from DER-encoded X.509 certificates.
    pub fn with_trusted_roots_der(
        expected_hostname: Option<&str>,
        roots: &[Vec<u8>],
    ) -> Result<Self, String> {
        let trusted_roots = parse_trusted_roots_der(roots);
        Ok(CertificateValidator {
            expected_hostname: expected_hostname.map(|s| s.to_string()),
            trusted_roots,
            skip_hostname_check: false,
            skip_chain_check: false,
        })
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

        let certs = match parse_cert_der_list(cert_der_list) {
            Ok(certs) => certs,
            Err(err) => {
                return CertValidationResult {
                    chain_valid: false,
                    hostname_valid: false,
                    error: Some(err),
                };
            }
        };

        let leaf = &certs[0];
        let hostname_valid = self.check_hostname(leaf);
        if !leaf.is_valid_now() {
            return CertValidationResult {
                chain_valid: false,
                hostname_valid,
                error: Some("Leaf certificate is expired or not yet valid".to_string()),
            };
        }

        if certs.len() == 1 && self.trusted_roots.is_empty() {
            return CertValidationResult {
                chain_valid: false,
                hostname_valid,
                error: Some("Self-signed certificate is not trusted".to_string()),
            };
        }

        for pair in certs.windows(2) {
            let cert = &pair[0];
            let issuer = &pair[1];
            if !issuer.is_valid_now() {
                return CertValidationResult {
                    chain_valid: false,
                    hostname_valid,
                    error: Some("Issuer certificate is expired or not yet valid".to_string()),
                };
            }
            if let Err(err) = cert.verify_signature(issuer) {
                return CertValidationResult {
                    chain_valid: false,
                    hostname_valid,
                    error: Some(err),
                };
            }
        }

        let chain_anchor = certs.last().expect("nonempty certificate chain");
        for trusted_root in &self.trusted_roots {
            if !trusted_root.is_valid_now() {
                continue;
            }
            if chain_anchor.verify_signature(trusted_root).is_ok() {
                return CertValidationResult {
                    chain_valid: true,
                    hostname_valid,
                    error: if hostname_valid {
                        None
                    } else {
                        Some("Hostname does not match certificate".to_string())
                    },
                };
            }
        }

        if self.trusted_roots.is_empty() {
            if let Err(err) = chain_anchor.verify_signature(chain_anchor) {
                return CertValidationResult {
                    chain_valid: false,
                    hostname_valid,
                    error: Some(format!("Root certificate is not self-signed: {err}")),
                };
            }
        }

        CertValidationResult {
            chain_valid: false,
            hostname_valid,
            error: Some("Certificate chain has no configured trusted root".to_string()),
        }
    }

    /// Check if the hostname matches the certificate's Subject Alternative Name.
    ///
    /// In a full implementation, this would:
    /// 1. Parse the X.509 certificate
    /// 2. Extract the SAN extension (2.5.29.17)
    /// 3. Match against DNS names and IP addresses
    /// 4. Fall back to Common Name if no SAN
    fn check_hostname(&self, cert: &Certificate) -> bool {
        if self.skip_hostname_check {
            return true;
        }

        match self.expected_hostname.as_deref() {
            Some(hostname) => cert.matches_hostname(hostname),
            None => true,
        }
    }

    /// Verify a CertificateVerify signature (RFC 8446 §4.4.3).
    ///
    /// The server signs the RFC 8446 context string plus transcript hash with
    /// its private key.
    /// The client verifies this signature using the server's public key from
    /// the leaf certificate.
    ///
    /// Supported algorithms:
    /// - 0x0403: ECDSA-SECP256R1-SHA256 (P-256)
    /// - 0x0804/0x0805/0x0806: RSA-PSS-RSAE with SHA-256/SHA-384/SHA-512
    /// - 0x0807: Ed25519
    /// - 0x0809/0x080a/0x080b: RSA-PSS-PSS with SHA-256/SHA-384/SHA-512
    pub fn verify_certificate_signature(
        &self,
        cert_der: &[u8],
        signature_algorithm: u16,
        signature: &[u8],
        transcript: &[u8],
        hasher: &Hasher,
    ) -> bool {
        match signature_algorithm {
            0x0403 => {
                // ECDSA-SECP256R1-SHA256 (P-256)
                self.verify_ecdsa_p256(cert_der, signature, transcript, hasher)
            }
            0x0807 => {
                // Ed25519
                self.verify_ed25519(cert_der, signature, transcript, hasher)
            }
            0x0804 | 0x0805 | 0x0806 | 0x0809 | 0x080a | 0x080b => {
                self.verify_rsa_pss(cert_der, signature_algorithm, signature, transcript, hasher)
            }
            _ => false,
        }
    }

    /// Verify an ED25519 signature over the transcript hash.
    fn verify_ed25519(
        &self,
        cert_der: &[u8],
        signature: &[u8],
        transcript: &[u8],
        hasher: &Hasher,
    ) -> bool {
        let cert = match Certificate::from_der(cert_der) {
            Ok(cert) => cert,
            Err(_) => return false,
        };
        let public_key: [u8; 32] = match cert.subject_public_key.as_slice().try_into() {
            Ok(public_key) => public_key,
            Err(_) => return false,
        };
        let verifying_key =
            match edgerun_crypto::ed25519_dalek::VerifyingKey::from_bytes(&public_key) {
                Ok(key) => key,
                Err(_) => return false,
            };
        let signature = match edgerun_crypto::ed25519_dalek::Signature::from_slice(signature) {
            Ok(signature) => signature,
            Err(_) => return false,
        };
        let signed_input = certificate_verify_signed_input(transcript, hasher);

        use edgerun_crypto::ed25519_dalek::Verifier;
        verifying_key.verify(&signed_input, &signature).is_ok()
    }

    /// Verify an RSA-PSS CertificateVerify signature.
    fn verify_rsa_pss(
        &self,
        cert_der: &[u8],
        signature_algorithm: u16,
        signature: &[u8],
        transcript: &[u8],
        hasher: &Hasher,
    ) -> bool {
        use edgerun_crypto::rsa::pkcs1::DecodeRsaPublicKey;

        let cert = match Certificate::from_der(cert_der) {
            Ok(cert) => cert,
            Err(_) => return false,
        };
        let public_key =
            match edgerun_crypto::rsa::RsaPublicKey::from_pkcs1_der(&cert.subject_public_key) {
                Ok(key) => key,
                Err(_) => return false,
            };
        let signature = match edgerun_crypto::rsa::pss::Signature::try_from(signature) {
            Ok(signature) => signature,
            Err(_) => return false,
        };
        let signed_input = certificate_verify_signed_input(transcript, hasher);

        match signature_algorithm {
            0x0804 | 0x0809 => {
                let key =
                    edgerun_crypto::rsa::pss::VerifyingKey::<edgerun_crypto::sha2::Sha256>::new(
                        public_key,
                    );
                use edgerun_crypto::rsa::signature::Verifier;
                key.verify(&signed_input, &signature).is_ok()
            }
            0x0805 | 0x080a => {
                let key =
                    edgerun_crypto::rsa::pss::VerifyingKey::<edgerun_crypto::sha2::Sha384>::new(
                        public_key,
                    );
                use edgerun_crypto::rsa::signature::Verifier;
                key.verify(&signed_input, &signature).is_ok()
            }
            0x0806 | 0x080b => {
                let key =
                    edgerun_crypto::rsa::pss::VerifyingKey::<edgerun_crypto::sha2::Sha512>::new(
                        public_key,
                    );
                use edgerun_crypto::rsa::signature::Verifier;
                key.verify(&signed_input, &signature).is_ok()
            }
            _ => false,
        }
    }

    /// Verify an ECDSA P-256 CertificateVerify signature.
    fn verify_ecdsa_p256(
        &self,
        cert_der: &[u8],
        signature: &[u8],
        transcript: &[u8],
        hasher: &Hasher,
    ) -> bool {
        let cert = match Certificate::from_der(cert_der) {
            Ok(cert) => cert,
            Err(_) => return false,
        };
        let verifying_key = match edgerun_crypto::p256::ecdsa::VerifyingKey::from_sec1_bytes(
            cert.subject_public_key.as_slice(),
        ) {
            Ok(key) => key,
            Err(_) => return false,
        };
        let signature = match edgerun_crypto::p256::ecdsa::Signature::from_der(signature) {
            Ok(signature) => signature,
            Err(_) => return false,
        };

        let signed_input = certificate_verify_signed_input(transcript, hasher);

        use edgerun_crypto::p256::ecdsa::signature::Verifier;
        verifying_key.verify(&signed_input, &signature).is_ok()
    }
}

fn certificate_verify_signed_input(transcript: &[u8], hasher: &Hasher) -> Vec<u8> {
    let mut signed_input = vec![0x20u8; 64];
    signed_input.extend_from_slice(b"TLS 1.3, server CertificateVerify");
    signed_input.push(0x00);
    signed_input.extend_from_slice(&hasher.hash(transcript));
    signed_input
}

fn parse_cert_der_list(cert_der_list: &[Vec<u8>]) -> Result<Vec<Certificate>, String> {
    cert_der_list
        .iter()
        .map(|cert| Certificate::from_der(cert))
        .collect::<Result<Vec<_>, _>>()
}

fn parse_trusted_roots_der(cert_der_list: &[Vec<u8>]) -> Vec<Certificate> {
    cert_der_list
        .iter()
        .filter_map(|cert| Certificate::from_der(cert).ok())
        .collect()
}

/// Common host CA bundle paths for Linux distributions.
#[cfg(feature = "std")]
pub const LINUX_CA_BUNDLE_PATHS: &[&str] = &[
    "/etc/ssl/certs/ca-certificates.crt",
    "/etc/ca-certificates/extracted/tls-ca-bundle.pem",
    "/etc/pki/tls/certs/ca-bundle.crt",
    "/etc/pki/ca-trust/extracted/pem/tls-ca-bundle.pem",
    "/etc/ssl/cert.pem",
    "/etc/ssl/ca-bundle.pem",
];

/// Load DER-encoded trust roots from the host Linux CA bundle.
///
/// Implemented only for host builds with the `std` feature. Bare targets must
/// supply roots explicitly with `QuicTlsHandshaker::set_trusted_roots_der`.
#[cfg(feature = "std")]
pub fn load_linux_trust_roots_der() -> Result<Vec<Vec<u8>>, String> {
    for path in LINUX_CA_BUNDLE_PATHS {
        let Ok(pem) = real_std::fs::read_to_string(path) else {
            continue;
        };
        let roots = parse_pem_certificates(&pem);
        if !roots.is_empty() {
            return Ok(roots);
        }
    }
    Err(format!(
        "No readable Linux CA bundle found in {}",
        LINUX_CA_BUNDLE_PATHS.join(", ")
    ))
}

#[cfg(feature = "std")]
fn parse_pem_certificates(pem: &str) -> Vec<Vec<u8>> {
    let begin = "-----BEGIN CERTIFICATE-----";
    let end = "-----END CERTIFICATE-----";
    let mut roots = Vec::new();
    let mut rest = pem;

    while let Some(begin_pos) = rest.find(begin) {
        let block_start = begin_pos;
        let after_begin = begin_pos + begin.len();
        let Some(end_rel) = rest[after_begin..].find(end) else {
            break;
        };
        let block_end = after_begin + end_rel + end.len();
        if let Some(der) = edgerun_crypto::x509_cert_from_pem(&rest[block_start..block_end]) {
            roots.push(der);
        }
        rest = &rest[block_end..];
    }

    roots
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
    /// Configured trusted root certificates, DER-encoded.
    trusted_roots_der: Vec<Vec<u8>>,
    /// Explicit local-test mode for same-stack QUIC without X.509 trust.
    allow_unverified_certificates: bool,
}

impl QuicTlsHandshaker {
    /// Create a new handshaker for a QUIC client connection.
    pub fn new(server_name: &str) -> Self {
        let mut client_random = [0u8; 32];
        fill_random(&mut client_random).expect("CSPRNG failure");

        let key_pair =
            EcdhKeyPair::generate(KeyExchangeGroup::X25519).expect("X25519 key generation failed");

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
            trusted_roots_der: Vec::new(),
            allow_unverified_certificates: false,
        }
    }

    /// Allow unverified server certificates.
    ///
    /// This is intended only for same-stack tests and local development.
    /// Production callers should leave this disabled and configure trust roots.
    pub fn allow_unverified_certificates(&mut self, allow: bool) {
        self.allow_unverified_certificates = allow;
    }

    /// Configure DER-encoded X.509 trust roots for strict certificate validation.
    pub fn set_trusted_roots_der(&mut self, roots: Vec<Vec<u8>>) {
        self.trusted_roots_der = roots;
    }

    /// Load trust roots from the host Linux CA bundle paths.
    ///
    /// This is available only for host builds with the `std` feature.
    #[cfg(feature = "std")]
    pub fn load_linux_trust_roots(&mut self) -> Result<usize, String> {
        let roots = load_linux_trust_roots_der()?;
        let count = roots.len();
        self.set_trusted_roots_der(roots);
        Ok(count)
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
        let initial_secret = self.key_schedule_derive_initial_secret(dcid);
        let client_in_secret =
            self.hasher
                .quic_expand_label(&initial_secret, "client in", &[], self.hasher.len());
        let server_in_secret =
            self.hasher
                .quic_expand_label(&initial_secret, "server in", &[], self.hasher.len());
        let write_hp = quic_hp_key(&server_in_secret, 16, &self.hasher);
        let read_hp = quic_hp_key(&client_in_secret, 16, &self.hasher);
        ProtectionKeys::new(
            CipherSuite::TLS_AES_128_GCM_SHA256,
            write.write_key,
            write.write_iv,
            read.write_key,
            read.write_iv,
        )
        .with_header_protection(write_hp, read_hp)
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
            let msg_len = read_u24_be(crypto_data, 1) as usize;
            4 + msg_len
        } else {
            crypto_data.len()
        };
        self.transcript
            .extend_from_slice(&crypto_data[..sh_msg_len.min(crypto_data.len())]);
        self.received_server_hello = true;

        // Derive 0-RTT early traffic keys BEFORE advancing key schedule.
        // 0-RTT keys are derived from the early secret + ClientHello hash
        // (RFC 8446 §7.1, "c e traffic" label).
        let ch_hash = self.hasher.hash(&self.transcript);
        self.early_traffic_secret = self.key_schedule.client_early_traffic_secret(&ch_hash);

        // Advance key schedule to handshake phase
        let shared_secret = self.key_pair.exchange(&self.server_key_share)?;
        let transcript_hash = self.hasher.hash(&self.transcript);

        self.key_schedule
            .advance_to_handshake(&shared_secret, &transcript_hash, &transcript_hash);

        // Cache the handshake traffic secrets for later use (Finished verification)
        self.client_hs_secret = self
            .key_schedule
            .client_handshake_traffic_secret(&transcript_hash);
        self.server_hs_secret = self
            .key_schedule
            .server_handshake_traffic_secret(&transcript_hash);

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

        let client_hs_secret = self
            .key_schedule
            .client_handshake_traffic_secret(&transcript_hash);
        let server_hs_secret = self
            .key_schedule
            .server_handshake_traffic_secret(&transcript_hash);

        // From client perspective:
        // - write: use client_hs_secret (client encrypts with its own secret)
        // - read: use server_hs_secret (client decrypts server's encrypted data)
        let write = quic_traffic_keys(
            &client_hs_secret,
            self.cipher_suite.key_len(),
            12,
            &self.hasher,
        );
        let read = quic_traffic_keys(
            &server_hs_secret,
            self.cipher_suite.key_len(),
            12,
            &self.hasher,
        );
        let write_hp = quic_hp_key(&client_hs_secret, self.cipher_suite.key_len(), &self.hasher);
        let read_hp = quic_hp_key(&server_hs_secret, self.cipher_suite.key_len(), &self.hasher);

        Ok(ProtectionKeys::new(
            self.cipher_suite,
            write.write_key,
            write.write_iv,
            read.write_key,
            read.write_iv,
        ))
        .map(|keys| keys.with_header_protection(write_hp, read_hp))
    }

    /// Process received CRYPTO data from Handshake-level packets.
    ///
    /// Parses EncryptedExtensions, Certificate, CertificateVerify, and Finished
    /// messages. Returns `Ok(())` if all messages were processed and the client
    /// Finished message is ready to be sent.
    ///
    /// Returns the Client Finished CRYPTO data to send back to the server.
    pub fn process_handshake_crypto(&mut self, crypto_data: &[u8]) -> Result<Vec<u8>, String> {
        // The handshake CRYPTO payload contains multiple TLS handshake messages:
        // EncryptedExtensions(8), Certificate(11), CertificateVerify(15), Finished(20)
        let mut pos = 0;
        let mut received_finished = false;

        while pos < crypto_data.len() {
            if pos + 4 > crypto_data.len() {
                break;
            }

            let msg_type = crypto_data[pos];
            let msg_len = read_u24_be(crypto_data, pos + 1) as usize;

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
                            let cert_list_len = read_u24_be(msg, cert_list_start) as usize;
                            let mut cert_pos = cert_list_start + 3;
                            let cert_end = cert_pos + cert_list_len.min(msg.len() - cert_pos);

                            while cert_pos + 3 <= cert_end {
                                let cert_len = read_u24_be(msg, cert_pos) as usize;
                                cert_pos += 3;
                                if cert_pos + cert_len <= cert_end && cert_len > 0 {
                                    let cert_der = msg[cert_pos..cert_pos + cert_len].to_vec();
                                    self.server_cert_chain.push(cert_der);
                                }
                                cert_pos += cert_len;
                                // Skip extensions (2-byte length + data)
                                if cert_pos + 2 <= cert_end {
                                    let ext_len = read_u16_be(msg, cert_pos) as usize;
                                    cert_pos += 2 + ext_len;
                                }
                            }

                            // Validate certificate chain
                            let validator = CertificateValidator::with_trusted_roots_der(
                                Some(self.server_name()),
                                &self.trusted_roots_der,
                            )?;
                            self.cert_validation =
                                Some(validator.validate_chain(&self.server_cert_chain));
                            if !self.allow_unverified_certificates {
                                let validation =
                                    self.cert_validation.as_ref().ok_or_else(|| {
                                        "Server certificate validation was not recorded".to_string()
                                    })?;
                                if !validation.is_valid() {
                                    return Err(format!(
                                        "Server certificate validation failed: {}",
                                        validation
                                            .error
                                            .as_deref()
                                            .unwrap_or("hostname or trust chain is invalid")
                                    ));
                                }
                            }
                        }
                    }
                    self.transcript.extend_from_slice(msg);
                }
                15 => {
                    // CertificateVerify — extract signature algorithm and signature
                    // Format: type(1) + len(3) + sig_alg(2) + sig_len(2) + signature
                    if msg_len < 8 {
                        return Err("Malformed CertificateVerify message".to_string());
                    }
                    let sig_alg = read_u16_be(msg, 4);
                    let sig_len = read_u16_be(msg, 6) as usize;
                    if 8 + sig_len > msg.len() {
                        return Err("Malformed CertificateVerify signature length".to_string());
                    }
                    let signature = msg[8..8 + sig_len].to_vec();
                    self.cert_verify_signature = Some((sig_alg, signature.clone()));

                    if !self.allow_unverified_certificates {
                        let leaf_cert = self.server_cert_chain.first().ok_or_else(|| {
                            "CertificateVerify received before Certificate".to_string()
                        })?;
                        let validator = CertificateValidator::with_trusted_roots_der(
                            Some(self.server_name()),
                            &self.trusted_roots_der,
                        )?;
                        if !validator.verify_certificate_signature(
                            leaf_cert,
                            sig_alg,
                            &signature,
                            &self.transcript,
                            &self.hasher,
                        ) {
                            return Err(
                                "Server CertificateVerify signature validation failed".to_string()
                            );
                        }
                    }
                    self.transcript.extend_from_slice(msg);
                }
                20 => {
                    if !self.allow_unverified_certificates
                        && (self.server_cert_chain.is_empty()
                            || self.cert_verify_signature.is_none())
                    {
                        return Err(
                            "Strict QUIC certificate validation requires Certificate and CertificateVerify"
                                .to_string(),
                        );
                    }
                    // Finished — verify and append
                    // Transcript hash for verification: hash of all messages BEFORE Finished
                    let pre_finished_hash = self.hasher.hash(&self.transcript);

                    // Use cached server handshake secret (derived from CH||SH transcript)
                    let finished_key = self.hasher.expand_label(
                        &self.server_hs_secret,
                        "finished",
                        &[],
                        self.hasher.len(),
                    );

                    let verify_data = match self.hasher {
                        Hasher::Sha256 => {
                            edgerun_crypto::hmac_sha256(&finished_key, &pre_finished_hash)
                        }
                        Hasher::Sha384 => {
                            edgerun_crypto::hmac_sha384(&finished_key, &pre_finished_hash)
                        }
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
        let finished_key =
            self.hasher
                .expand_label(&self.client_hs_secret, "finished", &[], self.hasher.len());

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
        let keys = quic_traffic_keys(
            &self.early_traffic_secret,
            self.cipher_suite.key_len(),
            12,
            &self.hasher,
        );
        let key_len = self.cipher_suite.key_len();
        let iv_len = 12;
        let write_hp = quic_hp_key(&self.early_traffic_secret, key_len, &self.hasher);
        // 0-RTT is client-to-server only. Do not mirror write keys into
        // the read side or tests can accidentally decrypt with client state.
        Some(ProtectionKeys::new(
            self.cipher_suite,
            keys.write_key,
            keys.write_iv,
            vec![0u8; key_len],
            vec![0u8; iv_len],
        ))
        .map(|keys| keys.with_header_protection(write_hp, vec![0u8; key_len]))
    }

    /// Build server-side keys for reading client 0-RTT data from the same
    /// early traffic secret.
    pub fn server_early_data_read_keys(&self) -> Option<ProtectionKeys> {
        if self.early_traffic_secret.is_empty() {
            return None;
        }
        let keys = quic_traffic_keys(
            &self.early_traffic_secret,
            self.cipher_suite.key_len(),
            12,
            &self.hasher,
        );
        let key_len = self.cipher_suite.key_len();
        let iv_len = 12;
        let read_hp = quic_hp_key(&self.early_traffic_secret, key_len, &self.hasher);
        Some(ProtectionKeys::new(
            self.cipher_suite,
            vec![0u8; key_len],
            vec![0u8; iv_len],
            keys.write_key,
            keys.write_iv,
        ))
        .map(|keys| keys.with_header_protection(vec![0u8; key_len], read_hp))
    }

    /// Derive application traffic keys from the post-Client-Finished transcript hash.
    pub fn app_keys(&self, transcript_after_client_finished: &[u8]) -> ProtectionKeys {
        let app_transcript_hash = self.hasher.hash(transcript_after_client_finished);

        let client_app_secret = self
            .key_schedule
            .client_app_traffic_secret(&app_transcript_hash);
        let server_app_secret = self
            .key_schedule
            .server_app_traffic_secret(&app_transcript_hash);

        let write = quic_traffic_keys(
            &client_app_secret,
            self.cipher_suite.key_len(),
            12,
            &self.hasher,
        );
        let read = quic_traffic_keys(
            &server_app_secret,
            self.cipher_suite.key_len(),
            12,
            &self.hasher,
        );
        let write_hp = quic_hp_key(
            &client_app_secret,
            self.cipher_suite.key_len(),
            &self.hasher,
        );
        let read_hp = quic_hp_key(
            &server_app_secret,
            self.cipher_suite.key_len(),
            &self.hasher,
        );

        ProtectionKeys::new(
            self.cipher_suite,
            write.write_key,
            write.write_iv,
            read.write_key,
            read.write_iv,
        )
        .with_header_protection(write_hp, read_hp)
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

    /// Derive client application traffic secret (for key updates).
    pub fn client_app_traffic_secret(&self, transcript_after_finished: &[u8]) -> Vec<u8> {
        let hash = self.hasher.hash(transcript_after_finished);
        self.key_schedule.client_app_traffic_secret(&hash)
    }

    /// Derive server application traffic secret (for key updates).
    pub fn server_app_traffic_secret(&self, transcript_after_finished: &[u8]) -> Vec<u8> {
        let hash = self.hasher.hash(transcript_after_finished);
        self.key_schedule.server_app_traffic_secret(&hash)
    }

    /// Get the hasher (for HKDF operations in key updates).
    pub fn hasher(&self) -> &edgerun_tls::prf::Hasher {
        &self.hasher
    }

    /// Build full handshake result with all protection keys.
    pub fn build_result(
        &self,
        dcid: &[u8],
        transcript_after_finished: &[u8],
    ) -> Result<HandshakeResult, String> {
        let initial_keys = self.initial_keys(dcid);

        let hs_keys = self.handshake_keys()?;
        let app_keys = self.app_keys(transcript_after_finished);

        let early_data_keys = self.early_data_keys();

        Ok(HandshakeResult {
            initial_keys,
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
        assert!(matches!(
            hasher_for_suite(CipherSuite::TLS_AES_128_GCM_SHA256),
            Hasher::Sha256
        ));
        assert!(matches!(
            hasher_for_suite(CipherSuite::TLS_AES_256_GCM_SHA384),
            Hasher::Sha384
        ));
    }

    #[test]
    fn early_data_keys_are_directional() {
        let mut hs = QuicTlsHandshaker::new("example.com");
        hs.early_traffic_secret = vec![0x11; hs.hasher.len()];

        let client_keys = hs.early_data_keys().unwrap();
        let server_keys = hs.server_early_data_read_keys().unwrap();
        assert_ne!(client_keys.write_key, client_keys.read_key);
        assert_eq!(client_keys.write_key, server_keys.read_key);

        let header = b"0rtt header";
        let plaintext = b"GET /";
        let mut client = PacketProtection::new(&client_keys);
        let encrypted = client
            .protect_with_packet_number(7, header, plaintext)
            .unwrap();

        let mut wrong_direction = PacketProtection::new(&client_keys);
        assert!(wrong_direction.unprotect(header, 7, &encrypted).is_err());

        let mut server = PacketProtection::new(&server_keys);
        let decrypted = server.unprotect(header, 7, &encrypted).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    fn certificate_message(cert_der: &[u8]) -> Vec<u8> {
        let mut body = Vec::new();
        body.push(0); // certificate_request_context length

        let cert_list_len = cert_der.len() + 5;
        body.extend_from_slice(&(cert_list_len as u32).to_be_bytes()[1..]);
        body.extend_from_slice(&(cert_der.len() as u32).to_be_bytes()[1..]);
        body.extend_from_slice(cert_der);
        body.extend_from_slice(&0u16.to_be_bytes()); // certificate extensions length

        let mut msg = Vec::new();
        msg.push(11);
        msg.extend_from_slice(&(body.len() as u32).to_be_bytes()[1..]);
        msg.extend_from_slice(&body);
        msg
    }

    fn der_len(len: usize) -> Vec<u8> {
        if len < 128 {
            return vec![len as u8];
        }
        let mut bytes = Vec::new();
        let mut value = len;
        while value > 0 {
            bytes.push(value as u8);
            value >>= 8;
        }
        bytes.reverse();
        let mut out = vec![0x80 | bytes.len() as u8];
        out.extend_from_slice(&bytes);
        out
    }

    fn der(tag: u8, value: &[u8]) -> Vec<u8> {
        let mut out = vec![tag];
        out.extend_from_slice(&der_len(value.len()));
        out.extend_from_slice(value);
        out
    }

    fn der_seq(parts: &[Vec<u8>]) -> Vec<u8> {
        let mut value = Vec::new();
        for part in parts {
            value.extend_from_slice(part);
        }
        der(0x30, &value)
    }

    fn der_oid(value: &[u8]) -> Vec<u8> {
        der(0x06, value)
    }

    fn der_bit_string(value: &[u8]) -> Vec<u8> {
        let mut bit_string = vec![0];
        bit_string.extend_from_slice(value);
        der(0x03, &bit_string)
    }

    fn fake_certificate_with_spki(spki_algorithm: &[u8], public_key: &[u8]) -> Vec<u8> {
        let tbs = der_seq(&[
            der(0x02, &[1]),
            der_seq(&[der_oid(&[0x2a, 0x86, 0x48, 0xce, 0x3d, 0x04, 0x03, 0x02])]),
            der_seq(&[]),
            der_seq(&[der(0x17, b"250101000000Z"), der(0x17, b"491231235959Z")]),
            der_seq(&[]),
            der_seq(&[
                der_seq(&[der_oid(spki_algorithm)]),
                der_bit_string(public_key),
            ]),
        ]);
        der_seq(&[
            tbs,
            der_seq(&[der_oid(&[0x2a, 0x86, 0x48, 0xce, 0x3d, 0x04, 0x03, 0x02])]),
            der_bit_string(&[0x30, 0x00]),
        ])
    }

    #[test]
    fn certificate_validator_rejects_untrusted_self_signed_chain() {
        let cert = edgerun_tls::generate_self_signed(&["example.com"]).unwrap();
        let validator = CertificateValidator::new(Some("example.com"));

        let result = validator.validate_chain(&[cert.cert_der]);

        assert!(!result.is_valid());
        assert!(result.hostname_valid);
        assert_eq!(
            result.error.as_deref(),
            Some("Self-signed certificate is not trusted")
        );
    }

    #[test]
    fn certificate_validator_accepts_configured_trusted_root() {
        let cert = edgerun_tls::generate_self_signed(&["example.com"]).unwrap();
        let validator = CertificateValidator::with_trusted_roots_der(
            Some("example.com"),
            &[cert.cert_der.clone()],
        )
        .unwrap();

        let result = validator.validate_chain(&[cert.cert_der]);

        assert!(result.is_valid(), "{result:?}");
    }

    #[cfg(feature = "std")]
    #[test]
    fn linux_ca_bundle_paths_include_arch_bundle() {
        assert!(LINUX_CA_BUNDLE_PATHS.contains(&"/etc/ca-certificates/extracted/tls-ca-bundle.pem"));
    }

    #[cfg(feature = "std")]
    #[test]
    fn parses_multiple_pem_certificates_from_linux_bundle() {
        let first = edgerun_tls::generate_self_signed(&["first.example"]).unwrap();
        let second = edgerun_tls::generate_self_signed(&["second.example"]).unwrap();
        let bundle = format!(
            "# generated test bundle\n{}\n\n{}\n",
            first.cert_pem(),
            second.cert_pem()
        );

        let roots = parse_pem_certificates(&bundle);

        assert_eq!(roots.len(), 2);
        assert_eq!(roots[0], first.cert_der);
        assert_eq!(roots[1], second.cert_der);
    }

    #[test]
    fn certificate_verify_signature_validates_tls13_context() {
        let cert = edgerun_tls::generate_self_signed(&["example.com"]).unwrap();
        let transcript = b"prior tls handshake messages";
        let cv = edgerun_tls::server::build_certificate_verify(
            transcript,
            &cert.signing_key,
            &Hasher::Sha256,
        )
        .unwrap();
        let sig_alg = read_u16_be(&cv, 4);
        let sig_len = read_u16_be(&cv, 6) as usize;
        let signature = &cv[8..8 + sig_len];
        let validator = CertificateValidator::new(Some("example.com"));

        assert!(validator.verify_certificate_signature(
            &cert.cert_der,
            sig_alg,
            signature,
            transcript,
            &Hasher::Sha256
        ));
        assert!(!validator.verify_certificate_signature(
            &cert.cert_der,
            sig_alg,
            signature,
            b"tampered transcript",
            &Hasher::Sha256
        ));
    }

    #[test]
    fn certificate_verify_accepts_ed25519_signature() {
        let mut rng = edgerun_crypto::rand_core::OsRng;
        let signing_key = edgerun_crypto::ed25519_dalek::SigningKey::generate(&mut rng);
        let verifying_key = signing_key.verifying_key();
        let cert_der = fake_certificate_with_spki(&[0x2b, 0x65, 0x70], verifying_key.as_bytes());
        let transcript = b"prior tls handshake messages";
        let signed_input = certificate_verify_signed_input(transcript, &Hasher::Sha256);
        use edgerun_crypto::ed25519_dalek::Signer;
        let signature = signing_key.sign(&signed_input);
        let validator = CertificateValidator::new(Some("example.com"));

        assert!(validator.verify_certificate_signature(
            &cert_der,
            0x0807,
            signature.to_bytes().as_slice(),
            transcript,
            &Hasher::Sha256
        ));
        assert!(!validator.verify_certificate_signature(
            &cert_der,
            0x0807,
            signature.to_bytes().as_slice(),
            b"tampered transcript",
            &Hasher::Sha256
        ));
    }

    #[test]
    fn certificate_verify_accepts_rsa_pss_signature() {
        use edgerun_crypto::rsa::pkcs1::EncodeRsaPublicKey;
        use edgerun_crypto::rsa::signature::{RandomizedSigner, SignatureEncoding};

        let mut rng = edgerun_crypto::rand_core::OsRng;
        let private_key = edgerun_crypto::rsa::RsaPrivateKey::new(&mut rng, 2048).unwrap();
        let public_key = edgerun_crypto::rsa::RsaPublicKey::from(&private_key);
        let public_key_der = public_key.to_pkcs1_der().unwrap();
        let cert_der = fake_certificate_with_spki(
            &[0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x01, 0x01],
            public_key_der.as_bytes(),
        );
        let transcript = b"prior tls handshake messages";
        let signed_input = certificate_verify_signed_input(transcript, &Hasher::Sha256);
        let signing_key =
            edgerun_crypto::rsa::pss::SigningKey::<edgerun_crypto::sha2::Sha256>::new(private_key);
        let signature = signing_key.sign_with_rng(&mut rng, &signed_input);
        let validator = CertificateValidator::new(Some("example.com"));

        assert!(validator.verify_certificate_signature(
            &cert_der,
            0x0804,
            signature.to_vec().as_slice(),
            transcript,
            &Hasher::Sha256
        ));
        assert!(!validator.verify_certificate_signature(
            &cert_der,
            0x0804,
            signature.to_vec().as_slice(),
            b"tampered transcript",
            &Hasher::Sha256
        ));
    }

    #[test]
    fn strict_handshake_rejects_unvalidated_certificate() {
        let mut hs = QuicTlsHandshaker::new("example.com");
        let cert = vec![0x30; 128];
        let err = hs
            .process_handshake_crypto(&certificate_message(&cert))
            .unwrap_err();

        assert!(err.contains("Server certificate validation failed"));
    }

    #[test]
    fn insecure_handshake_policy_makes_unverified_cert_explicit() {
        let mut hs = QuicTlsHandshaker::new("example.com");
        hs.allow_unverified_certificates(true);
        let cert = vec![0x30; 128];
        let err = hs
            .process_handshake_crypto(&certificate_message(&cert))
            .unwrap_err();

        assert!(err.contains("Server Finished not found"));
        assert!(hs.cert_validation().is_some());
    }
}
