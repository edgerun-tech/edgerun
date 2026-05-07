//! Async TLS 1.3 streams — client and server — wrapping any
//! `edgerun_tls::AsyncRead + edgerun_tls::AsyncWrite + Unpin` transport.
//!
//! Mirrors the sync `TlsStream` / `TlsServerStream` API but uses
//! async `poll_read` / `poll_write` instead of `std::io::Read` / `Write`.
//!
//! # Example (client)
//! ```rust
//! use edgerun_tls::{AsyncRead, AsyncTlsStream, AsyncWrite};
//!
//! // AsyncTlsStream works with any async read/write stream
//! async fn tls_client_example<S>(stream: S) -> edgerun_tls::Result<AsyncTlsStream<S>>
//! where
//!     S: AsyncRead + AsyncWrite + Unpin,
//! {
//!     AsyncTlsStream::client(stream, "example.com", &[], None).await
//! }
//! ```

use crate::std;
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec;
use alloc::vec::Vec;
use std::pin::Pin;
use std::task::{Context, Poll};

use crate::alert::{Alert, AlertLevel};
use crate::certificate::Certificate;
use crate::certificate_gen::CertificateAndKey;
use crate::cipher::NamedGroup;
use crate::handshake::{ClientHelloBuilder, ServerHello};
use crate::key_exchange::{EcdhKeyPair, KeyExchangeGroup};
use crate::prf::{
    client_app_write_keys, client_write_keys, hmac_sha256, hmac_sha384, server_app_write_keys,
    server_write_keys, Hasher, Tls13KeySchedule,
};
use crate::record::RecordCipher;
use crate::server::client_hello::ClientHello;
use crate::server::message_builder::{
    build_certificate_chain_message, build_certificate_verify, build_encrypted_extensions,
    build_finished_message, build_server_hello, compute_client_finished_verify_data,
    compute_server_finished_verify_data,
};
use crate::session_cache::SessionCache;
use crate::{Result, TlsError};
use edgerun_crypto::CipherSuite;
use edgerun_encoding::byteorder::{read_u16_be, read_u24_be};

use crate::compat::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

const TLS_MAX_PLAINTEXT_FRAGMENT: usize = 16 * 1024;

// ---------------------------------------------------------------------------
// AsyncTlsStream — client-side async TLS stream
// ---------------------------------------------------------------------------

enum ClientRecordCipher {
    Tls13(RecordCipher),
    Tls12(Tls12RecordCipher),
}

impl ClientRecordCipher {
    fn encrypt(&mut self, content_type: u8, plaintext: &[u8]) -> Vec<u8> {
        match self {
            Self::Tls13(cipher) => cipher.encrypt(content_type, plaintext),
            Self::Tls12(cipher) => cipher.encrypt(content_type, plaintext),
        }
    }

    fn decrypt(&mut self, ciphertext: &[u8]) -> Result<(u8, Vec<u8>)> {
        self.decrypt_record(23, ciphertext)
    }

    fn decrypt_record(&mut self, content_type: u8, ciphertext: &[u8]) -> Result<(u8, Vec<u8>)> {
        match self {
            Self::Tls13(cipher) => cipher.decrypt(ciphertext).map_err(TlsError::Cipher),
            Self::Tls12(cipher) => cipher.decrypt_record(content_type, ciphertext),
        }
    }
}

struct Tls12RecordCipher {
    cipher: edgerun_crypto::Aes256GcmCipher,
    fixed_iv: Vec<u8>,
    seq: u64,
}

impl Tls12RecordCipher {
    fn new(key: &[u8], fixed_iv: &[u8]) -> Result<Self> {
        let cipher = edgerun_crypto::Aes256GcmCipher::new(key)
            .map_err(|_| TlsError::Cipher("invalid TLS 1.2 AES-GCM key".into()))?;
        Ok(Self {
            cipher,
            fixed_iv: fixed_iv.to_vec(),
            seq: 0,
        })
    }

    fn encrypt(&mut self, content_type: u8, plaintext: &[u8]) -> Vec<u8> {
        let explicit = self.seq.to_be_bytes();
        let nonce = tls12_gcm_nonce(&self.fixed_iv, &explicit);
        let aad = tls12_gcm_aad(self.seq, content_type, plaintext.len());
        let mut buffer = plaintext.to_vec();
        let tag = self
            .cipher
            .encrypt_in_place_detached(
                edgerun_crypto::aes_gcm::Nonce::from_slice(&nonce),
                &aad,
                &mut buffer,
            )
            .expect("TLS 1.2 AEAD encryption failed");
        self.seq = self.seq.wrapping_add(1);

        let encrypted_len = 8 + buffer.len() + tag.len();
        let mut out = Vec::with_capacity(encrypted_len);
        out.extend_from_slice(&explicit);
        out.extend_from_slice(&buffer);
        out.extend_from_slice(&tag);
        out
    }

    fn decrypt(&mut self, fragment: &[u8]) -> Result<(u8, Vec<u8>)> {
        self.decrypt_record(23, fragment)
    }

    fn decrypt_record(&mut self, content_type: u8, fragment: &[u8]) -> Result<(u8, Vec<u8>)> {
        if fragment.len() < 8 + 16 {
            return Err(TlsError::Cipher("TLS 1.2 AEAD record too short".into()));
        }
        let (explicit, encrypted) = fragment.split_at(8);
        let cipher_len = encrypted.len() - 16;
        let (ciphertext, tag) = encrypted.split_at(cipher_len);
        let nonce = tls12_gcm_nonce(&self.fixed_iv, explicit);
        let aad = tls12_gcm_aad(self.seq, content_type, cipher_len);
        let mut buffer = ciphertext.to_vec();
        self.cipher
            .decrypt_in_place_detached(
                edgerun_crypto::aes_gcm::Nonce::from_slice(&nonce),
                &aad,
                &mut buffer,
                edgerun_crypto::aes_gcm::aead::generic_array::GenericArray::from_slice(tag),
            )
            .map_err(|_| TlsError::Cipher("TLS 1.2 AEAD decryption failed".into()))?;
        self.seq = self.seq.wrapping_add(1);
        Ok((content_type, buffer))
    }
}

fn tls12_gcm_nonce(fixed_iv: &[u8], explicit: &[u8]) -> [u8; 12] {
    let mut nonce = [0u8; 12];
    nonce[..4].copy_from_slice(&fixed_iv[..4]);
    nonce[4..].copy_from_slice(&explicit[..8]);
    nonce
}

fn tls12_gcm_aad(seq: u64, content_type: u8, plaintext_len: usize) -> [u8; 13] {
    let mut aad = [0u8; 13];
    aad[..8].copy_from_slice(&seq.to_be_bytes());
    aad[8] = content_type;
    aad[9..11].copy_from_slice(&0x0303u16.to_be_bytes());
    aad[11..13].copy_from_slice(&(plaintext_len as u16).to_be_bytes());
    aad
}

/// Async TLS 1.3 client stream wrapping any `AsyncRead + AsyncWrite` transport.
pub struct AsyncTlsStream<S> {
    /// The underlying transport stream.
    stream: S,
    /// The expected server hostname for SNI and certificate verification.
    server_name: String,
    /// The negotiated cipher suite.
    cipher_suite: CipherSuite,
    /// Record cipher for writing (client → server).
    write_cipher: ClientRecordCipher,
    /// Record cipher for reading (server → client).
    read_cipher: ClientRecordCipher,
    /// Whether the TLS handshake has completed.
    handshake_done: bool,
    /// Buffered application data that was read but not yet consumed.
    pending_data: Vec<u8>,
    /// Current read position within `pending_data`.
    pending_offset: usize,
    /// Partially read TLS record header.
    read_header: [u8; 5],
    /// Current read position within `read_header`.
    read_header_pos: usize,
    /// Partially read TLS record fragment.
    read_fragment: Vec<u8>,
    /// Current read position within `read_fragment`.
    read_fragment_pos: usize,
    /// Partially written encrypted TLS record.
    write_record: Vec<u8>,
    /// Current write position within `write_record`.
    write_record_pos: usize,
    /// Plaintext byte count represented by `write_record`.
    write_plaintext_len: usize,
    /// Whether the underlying stream still needs flushing after `write_record`.
    write_needs_flush: bool,
    /// The ALPN protocol negotiated during the TLS handshake.
    alpn_protocol: Option<Vec<u8>>,
}

impl<S> AsyncTlsStream<S> {
    /// Check if the TLS handshake has completed.
    pub fn is_handshake_complete(&self) -> bool {
        self.handshake_done
    }

    /// Consume the TLS stream and return the underlying transport stream.
    pub fn into_inner(self) -> S {
        self.stream
    }

    /// Returns the negotiated ALPN protocol, or `None` if none was negotiated.
    pub fn alpn_protocol(&self) -> Option<&[u8]> {
        self.alpn_protocol.as_deref()
    }
}

impl<S: AsyncRead + AsyncWrite + Unpin> AsyncTlsStream<S> {
    /// Perform an async TLS 1.3 client handshake with ALPN protocol negotiation.
    ///
    /// `alpn_protocols` is a list of protocols to advertise (e.g., `&[b"h2", b"http/1.1"]`).
    /// For plain HTTP/1.1 over TLS with no ALPN, pass `&[]`.
    ///
    /// `session_cache` enables TLS 1.3 session resumption. Pass `None` for
    /// full handshake every time.
    pub async fn client(
        mut stream: S,
        server_name: &str,
        alpn_protocols: &[&[u8]],
        session_cache: Option<&SessionCache>,
    ) -> Result<Self> {
        Self::client_at_unix_secs(
            stream,
            server_name,
            alpn_protocols,
            session_cache,
            current_unix_secs(),
        )
        .await
    }

    /// Perform an async TLS 1.3 client handshake using caller-provided Unix
    /// time for certificate validation.
    pub async fn client_at_unix_secs(
        mut stream: S,
        server_name: &str,
        alpn_protocols: &[&[u8]],
        session_cache: Option<&SessionCache>,
        unix_secs: u64,
    ) -> Result<Self> {
        let mut stream = stream;
        let mut hrr_group = KeyExchangeGroup::X25519;
        let mut cookie: Vec<u8> = Vec::new();
        loop {
            let client_random = generate_random();
            let key_pair = EcdhKeyPair::generate(if cookie.is_empty() {
                KeyExchangeGroup::X25519
            } else {
                hrr_group
            })
            .map_err(TlsError::HandshakeFailure)?;

            match Self::client_inner_with_cookie(
                stream,
                server_name,
                alpn_protocols,
                session_cache,
                client_random,
                key_pair,
                &cookie,
                unix_secs,
            )
            .await
            {
                Ok(tls) => return Ok(tls),
                Err((TlsError::HelloRetryRequest(selected_group, new_cookie), s)) => {
                    stream = s;
                    hrr_group = selected_group;
                    cookie = new_cookie;
                }
                Err((e, _s)) => return Err(e),
            }
        }
    }

    async fn client_inner_with_cookie(
        stream: S,
        server_name: &str,
        alpn_protocols: &[&[u8]],
        session_cache: Option<&SessionCache>,
        client_random: [u8; 32],
        key_pair: EcdhKeyPair,
        hrr_cookie: &[u8],
        unix_secs: u64,
    ) -> core::result::Result<Self, (TlsError, S)> {
        let group = match key_pair.group() {
            KeyExchangeGroup::SECP256R1 => NamedGroup::SECP256R1,
            KeyExchangeGroup::X25519 => NamedGroup::X25519,
        };
        let mut ch_builder = ClientHelloBuilder::new(client_random, server_name)
            .key_share(&key_pair.public_key_bytes(), group);

        if !hrr_cookie.is_empty() {
            ch_builder = ch_builder.cookie(hrr_cookie);
        }

        if let Some(cache) = session_cache {
            if let Some(ticket) = cache.get(server_name) {
                ch_builder = ch_builder.psk_identity(&ticket.ticket, 0, ticket.obfuscated_age());
            }
        }

        if !alpn_protocols.is_empty() {
            ch_builder = ch_builder.alpn_protocols(alpn_protocols);
        }
        let ch = match ch_builder.build() {
            Ok(c) => c,
            Err(e) => return Err((e.into(), stream)),
        };

        let mut transcript = ch.clone();

        let record = crate::record::TlsRecord {
            content_type: 22,
            version: 0x0301,
            fragment: ch,
        };
        let mut stream = stream;
        if let Err(e) = stream.write_all(&record.to_bytes()).await {
            return Err((TlsError::Io(e), stream));
        }
        if let Err(e) = stream.flush().await {
            return Err((TlsError::Io(e), stream));
        }

        let mut hdr = [0u8; 5];
        if let Err(e) = stream.read_exact(&mut hdr).await {
            return Err((TlsError::Io(e), stream));
        }
        let ct = hdr[0];
        if ct != 22 {
            if ct == 21 {
                let len = read_u16_be(&hdr, 3) as usize;
                let mut fragment = vec![0u8; len];
                if let Err(e) = stream.read_exact(&mut fragment).await {
                    return Err((TlsError::Io(e), stream));
                }
                if fragment.len() >= 2 {
                    match AlertLevel::from_wire(fragment[0]) {
                        Ok(level) => match Alert::from_wire(fragment[1]) {
                            Ok(alert) => return Err((TlsError::Alert(level, alert), stream)),
                            Err(e) => return Err((TlsError::Protocol(e), stream)),
                        },
                        Err(e) => return Err((TlsError::Protocol(e), stream)),
                    }
                }
            }
            return Err((
                TlsError::HandshakeFailure(format!(
                    "Expected handshake record, got content_type={ct}",
                )),
                stream,
            ));
        }
        let len = read_u16_be(&hdr, 3) as usize;
        let mut fragment = vec![0u8; len];
        if let Err(e) = stream.read_exact(&mut fragment).await {
            return Err((TlsError::Io(e), stream));
        }

        const HRR_MAGIC: [u8; 32] = [
            0xCF, 0x21, 0xAD, 0x74, 0xE5, 0x9A, 0x61, 0x11, 0xBE, 0x1D, 0x8C, 0x02, 0x1E, 0x65,
            0xB8, 0x91, 0xC2, 0xA2, 0x11, 0x16, 0x7A, 0xBB, 0x8C, 0x5E, 0x07, 0x9E, 0x44, 0xE2,
            0xF6, 0x8E, 0x02, 0x81,
        ];
        if fragment.len() >= 4 + 32 {
            let random_start = 4 + 2;
            if random_start + 32 <= fragment.len()
                && fragment[random_start..random_start + 32] == HRR_MAGIC
            {
                let selected_group = match parse_hrr_selected_group(&fragment) {
                    Ok(g) => g,
                    Err(e) => return Err((e.into(), stream)),
                };
                let cookie = parse_hrr_cookie(&fragment).unwrap_or_default();
                return Err((TlsError::HelloRetryRequest(selected_group, cookie), stream));
            }
        }

        let sh = match ServerHello::parse(&fragment) {
            Ok(s) => s,
            Err(e) => return Err((e.into(), stream)),
        };

        if sh.supported_version != Some(0x0304) {
            return Err((
                TlsError::HandshakeFailure(format!(
                    "Server did not negotiate TLS 1.3 (got supported_version={:?})",
                    sh.supported_version,
                )),
                stream,
            ));
        }

        let negotiated_suite = sh.cipher_suite;
        let hash = match negotiated_suite {
            CipherSuite::TLS_AES_128_GCM_SHA256 => Hasher::Sha256,
            CipherSuite::TLS_AES_256_GCM_SHA384 => Hasher::Sha384,
        };
        transcript.extend_from_slice(&fragment);

        let server_key_share = sh.server_key_share.clone();
        let shared_secret = match key_pair.exchange(&server_key_share) {
            Ok(s) => s,
            Err(e) => return Err((TlsError::HandshakeFailure(e.to_string()), stream)),
        };
        let transcript_hash = hash.hash(&transcript);

        let mut ks = Tls13KeySchedule::new(hash.clone());
        ks.advance_to_handshake(&shared_secret, &transcript_hash, &transcript_hash);
        let client_hs_secret = ks.client_handshake_traffic_secret(&transcript_hash);
        let server_hs_secret = ks.server_handshake_traffic_secret(&transcript_hash);

        let client_hs_keys =
            client_write_keys(&client_hs_secret, negotiated_suite.key_len(), 12, &hash);
        let server_hs_keys =
            server_write_keys(&server_hs_secret, negotiated_suite.key_len(), 12, &hash);

        let mut write_cipher =
            match RecordCipher::new(&client_hs_keys.write_key, &client_hs_keys.write_iv) {
                Ok(c) => c,
                Err(e) => return Err((e.into(), stream)),
            };
        let mut read_cipher =
            match RecordCipher::new(&server_hs_keys.write_key, &server_hs_keys.write_iv) {
                Ok(c) => c,
                Err(e) => return Err((e.into(), stream)),
            };

        let alpn_protocol = match async_read_encrypted_handshake_messages(
            &mut stream,
            &mut read_cipher,
            &mut ks,
            &mut transcript,
            &hash,
            &transcript_hash,
            server_name,
            unix_secs,
        )
        .await
        {
            Ok(a) => a,
            Err(e) => return Err((e, stream)),
        };

        let app_transcript_hash = hash.hash(&transcript);

        if let Err(e) = async_send_client_finished(
            &mut stream,
            &mut write_cipher,
            &ks,
            &transcript,
            &hash,
            &transcript_hash,
        )
        .await
        {
            return Err((e, stream));
        }

        ks.advance_to_master();
        let client_app = ks.client_app_traffic_secret(&app_transcript_hash);
        let server_app = ks.server_app_traffic_secret(&app_transcript_hash);

        let client_app_keys =
            client_app_write_keys(&client_app, negotiated_suite.key_len(), 12, &hash);
        let server_app_keys =
            server_app_write_keys(&server_app, negotiated_suite.key_len(), 12, &hash);

        let write_cipher =
            match RecordCipher::new(&client_app_keys.write_key, &client_app_keys.write_iv) {
                Ok(c) => ClientRecordCipher::Tls13(c),
                Err(e) => return Err((e.into(), stream)),
            };
        let read_cipher =
            match RecordCipher::new(&server_app_keys.write_key, &server_app_keys.write_iv) {
                Ok(c) => ClientRecordCipher::Tls13(c),
                Err(e) => return Err((e.into(), stream)),
            };

        Ok(AsyncTlsStream {
            stream,
            server_name: server_name.to_string(),
            cipher_suite: negotiated_suite,
            write_cipher,
            read_cipher,
            handshake_done: true,
            pending_data: Vec::new(),
            pending_offset: 0,
            read_header: [0; 5],
            read_header_pos: 0,
            read_fragment: Vec::new(),
            read_fragment_pos: 0,
            write_record: Vec::new(),
            write_record_pos: 0,
            write_plaintext_len: 0,
            write_needs_flush: false,
            alpn_protocol,
        })
    }

    /// Perform an async TLS 1.2 client handshake for servers that do not
    /// negotiate TLS 1.3. This intentionally supports only ECDHE + AES-GCM
    /// suites so the fallback stays small and modern.
    pub async fn client_tls12(mut stream: S, server_name: &str) -> Result<Self> {
        let client_random = generate_random();
        let key_pair = EcdhKeyPair::generate(KeyExchangeGroup::SECP256R1)
            .map_err(TlsError::HandshakeFailure)?;
        let ch = build_tls12_client_hello(client_random, server_name);
        let mut transcript = ch.clone();
        write_plain_record_with_version(&mut stream, 22, 0x0301, &ch).await?;

        let mut server_random = [0u8; 32];
        let mut cipher_suite = 0u16;
        let mut extended_master_secret = false;
        let mut certificate_requested = false;
        let mut server_key_exchange = Vec::new();

        loop {
            let (content_type, record) = read_plain_record(&mut stream).await?;
            if content_type == 21 {
                return Err(parse_alert_record(&record));
            }
            if content_type != 22 {
                return Err(TlsError::Protocol(format!(
                    "TLS 1.2 expected handshake record, got {content_type}",
                )));
            }
            let mut pos = 0usize;
            while pos + 4 <= record.len() {
                let msg_type = record[pos];
                let len = read_u24_be(&record, pos + 1) as usize;
                let end = pos + 4 + len;
                if end > record.len() {
                    return Err(TlsError::Protocol("TLS 1.2 handshake truncated".into()));
                }
                let msg = &record[pos..end];
                match msg_type {
                    2 => {
                        parse_tls12_server_hello(
                            msg,
                            &mut server_random,
                            &mut cipher_suite,
                            &mut extended_master_secret,
                        )?;
                        transcript.extend_from_slice(msg);
                    }
                    11 => {
                        transcript.extend_from_slice(msg);
                    }
                    12 => {
                        server_key_exchange = msg.to_vec();
                        transcript.extend_from_slice(msg);
                    }
                    13 => {
                        certificate_requested = true;
                        transcript.extend_from_slice(msg);
                    }
                    14 => {
                        transcript.extend_from_slice(msg);
                        break;
                    }
                    _ => transcript.extend_from_slice(msg),
                }
                if msg_type == 14 {
                    break;
                }
                pos = end;
            }
            if !server_key_exchange.is_empty() && record_handshake_has_type(&record, 14) {
                break;
            }
        }

        let server_public = parse_tls12_server_key_exchange(&server_key_exchange)?;
        let shared_secret = key_pair
            .exchange(&server_public)
            .map_err(TlsError::HandshakeFailure)?;

        if certificate_requested {
            let certificate = handshake_message(11, &[0, 0, 0]);
            transcript.extend_from_slice(&certificate);
            write_plain_record(&mut stream, 22, &certificate).await?;
        }
        let client_key_exchange = build_tls12_client_key_exchange(&key_pair.public_key_bytes());
        transcript.extend_from_slice(&client_key_exchange);
        write_plain_record(&mut stream, 22, &client_key_exchange).await?;

        let master_seed = if extended_master_secret {
            tls12_handshake_hash(cipher_suite, &transcript)?
        } else {
            [client_random.as_slice(), server_random.as_slice()].concat()
        };
        let master_label = if extended_master_secret {
            b"extended master secret".as_slice()
        } else {
            b"master secret".as_slice()
        };
        let master_secret =
            tls12_prf(cipher_suite, &shared_secret, master_label, &master_seed, 48)?;
        let key_block = tls12_prf(
            cipher_suite,
            &master_secret,
            b"key expansion",
            &[server_random.as_slice(), client_random.as_slice()].concat(),
            tls12_key_block_len(cipher_suite)?,
        )?;
        let keys = split_tls12_key_block(cipher_suite, &key_block)?;

        stream.write_all(&[20, 0x03, 0x03, 0, 1, 1]).await?;
        stream.flush().await?;

        let verify_data = tls12_finished_verify_data(
            cipher_suite,
            &master_secret,
            b"client finished",
            &transcript,
        )?;
        let client_finished = build_tls12_finished(&verify_data);
        transcript.extend_from_slice(&client_finished);
        let mut write_cipher = Tls12RecordCipher::new(&keys.client_key, &keys.client_iv)?;
        let mut read_cipher = Tls12RecordCipher::new(&keys.server_key, &keys.server_iv)?;
        let encrypted_finished = write_cipher.encrypt(22, &client_finished);
        write_plain_record(&mut stream, 22, &encrypted_finished).await?;
        stream.flush().await?;

        loop {
            let (content_type, fragment) = read_plain_record(&mut stream).await?;
            if content_type == 20 && fragment == [1] {
                break;
            }
            if content_type == 21 {
                return Err(parse_alert_record(&fragment));
            }
            if content_type == 22 {
                // Some TLS 1.2 servers send post-handshake messages such as
                // NewSessionTicket before ChangeCipherSpec. They are still
                // part of the handshake transcript used for server Finished.
                transcript.extend_from_slice(&fragment);
                continue;
            }
            return Err(TlsError::Protocol(
                "TLS 1.2 expected ChangeCipherSpec".into(),
            ));
        }
        let (content_type, encrypted) = read_plain_record(&mut stream).await?;
        if content_type != 22 {
            return Err(TlsError::Protocol(
                "TLS 1.2 expected encrypted Finished".into(),
            ));
        }
        let (inner_type, server_finished) = read_cipher.decrypt_record(22, &encrypted)?;
        if inner_type != 23 && inner_type != 22 {
            return Err(TlsError::Protocol("TLS 1.2 invalid Finished record".into()));
        }
        if server_finished.len() < 16 || server_finished[0] != 20 {
            return Err(TlsError::Protocol(
                "TLS 1.2 invalid Finished message".into(),
            ));
        }
        let expected = tls12_finished_verify_data(
            cipher_suite,
            &master_secret,
            b"server finished",
            &transcript,
        )?;
        if !constant_time_eq(&server_finished[4..], &expected) {
            return Err(TlsError::HandshakeFailure(
                "TLS 1.2 server Finished verification failed".into(),
            ));
        }

        Ok(AsyncTlsStream {
            stream,
            server_name: server_name.to_string(),
            cipher_suite: CipherSuite::TLS_AES_128_GCM_SHA256,
            write_cipher: ClientRecordCipher::Tls12(write_cipher),
            read_cipher: ClientRecordCipher::Tls12(read_cipher),
            handshake_done: true,
            pending_data: Vec::new(),
            pending_offset: 0,
            read_header: [0; 5],
            read_header_pos: 0,
            read_fragment: Vec::new(),
            read_fragment_pos: 0,
            write_record: Vec::new(),
            write_record_pos: 0,
            write_plaintext_len: 0,
            write_needs_flush: false,
            alpn_protocol: None,
        })
    }

    /// Async read — returns decrypted application data.
    pub fn poll_read(
        &mut self,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<std::io::Result<usize>> {
        if !self.handshake_done {
            return Poll::Ready(Err(std::io::Error::new(
                std::io::ErrorKind::NotConnected,
                "TLS handshake not complete",
            )));
        }

        // Return pending data first
        if self.pending_offset < self.pending_data.len() {
            let available = self.pending_data.len() - self.pending_offset;
            let n = available.min(buf.len());
            buf[..n]
                .copy_from_slice(&self.pending_data[self.pending_offset..self.pending_offset + n]);
            self.pending_offset += n;
            return Poll::Ready(Ok(n));
        }

        // Read a new record
        match self.poll_read_application_data(cx) {
            Poll::Ready(Ok(plaintext)) => {
                let n = plaintext.len().min(buf.len());
                buf[..n].copy_from_slice(&plaintext[..n]);
                if plaintext.len() > n {
                    self.pending_data = plaintext;
                    self.pending_offset = n;
                } else {
                    self.pending_data.clear();
                    self.pending_offset = 0;
                }
                Poll::Ready(Ok(n))
            }
            Poll::Ready(Err(TlsError::Io(e))) => Poll::Ready(Err(e)),
            Poll::Ready(Err(TlsError::Alert(AlertLevel::Warning, Alert::CloseNotify))) => {
                Poll::Ready(Ok(0))
            }
            Poll::Ready(Err(TlsError::Alert(_, _))) => Poll::Ready(Err(std::io::Error::new(
                std::io::ErrorKind::ConnectionReset,
                "TLS alert",
            ))),
            Poll::Ready(Err(e)) => Poll::Ready(Err(std::io::Error::other(e.to_string()))),
            Poll::Pending => Poll::Pending,
        }
    }

    /// Async write — encrypts and sends application data.
    pub fn poll_write(&mut self, cx: &mut Context<'_>, buf: &[u8]) -> Poll<std::io::Result<usize>> {
        if !self.handshake_done {
            return Poll::Ready(Err(std::io::Error::new(
                std::io::ErrorKind::NotConnected,
                "TLS handshake not complete",
            )));
        }
        if buf.is_empty() {
            return Poll::Ready(Ok(0));
        }
        if self.write_record.is_empty() {
            let plaintext_len = buf.len().min(TLS_MAX_PLAINTEXT_FRAGMENT);
            let ciphertext = self.write_cipher.encrypt(23, &buf[..plaintext_len]);
            let record = crate::record::TlsRecord {
                content_type: 23,
                version: 0x0303,
                fragment: ciphertext,
            };
            self.write_record = record.to_bytes();
            self.write_record_pos = 0;
            self.write_plaintext_len = plaintext_len;
            self.write_needs_flush = true;
        }

        while self.write_record_pos < self.write_record.len() {
            match Pin::new(&mut self.stream)
                .poll_write(cx, &self.write_record[self.write_record_pos..])
            {
                Poll::Ready(Ok(n)) => {
                    if n == 0 {
                        return Poll::Ready(Err(std::io::Error::new(
                            std::io::ErrorKind::WriteZero,
                            "failed to write whole buffer",
                        )));
                    }
                    self.write_record_pos += n;
                }
                Poll::Ready(Err(e)) => return Poll::Ready(Err(e)),
                Poll::Pending => return Poll::Pending,
            }
        }

        if self.write_needs_flush {
            match Pin::new(&mut self.stream).poll_flush(cx) {
                Poll::Ready(Ok(())) => {
                    self.write_needs_flush = false;
                }
                Poll::Ready(Err(e)) => return Poll::Ready(Err(e)),
                Poll::Pending => return Poll::Pending,
            }
        }

        let written = self.write_plaintext_len;
        self.write_record.clear();
        self.write_record_pos = 0;
        self.write_plaintext_len = 0;
        Poll::Ready(Ok(written))
    }

    /// Flush the underlying transport.
    pub fn poll_flush(&mut self, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.stream).poll_flush(cx)
    }

    /// Shut down the underlying transport.
    pub fn poll_shutdown(&mut self, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.stream).poll_shutdown(cx)
    }

    // --- Internal ---

    fn poll_read_application_data(&mut self, cx: &mut Context<'_>) -> Poll<Result<Vec<u8>>> {
        loop {
            while self.read_header_pos < self.read_header.len() {
                match Pin::new(&mut self.stream)
                    .poll_read(cx, &mut self.read_header[self.read_header_pos..])
                {
                    Poll::Ready(Ok(n)) => {
                        if n == 0 {
                            return Poll::Ready(Err(TlsError::Io(std::io::Error::new(
                                std::io::ErrorKind::UnexpectedEof,
                                "failed to read record header",
                            ))));
                        }
                        self.read_header_pos += n;
                    }
                    Poll::Ready(Err(e)) => return Poll::Ready(Err(TlsError::Io(e))),
                    Poll::Pending => return Poll::Pending,
                }
            }
            let content_type = self.read_header[0];
            let _version = read_u16_be(&self.read_header, 1);
            let length = read_u16_be(&self.read_header, 3) as usize;

            if self.read_fragment.len() != length {
                self.read_fragment.resize(length, 0);
                self.read_fragment_pos = 0;
            }

            while self.read_fragment_pos < self.read_fragment.len() {
                match Pin::new(&mut self.stream)
                    .poll_read(cx, &mut self.read_fragment[self.read_fragment_pos..])
                {
                    Poll::Ready(Ok(n)) => {
                        if n == 0 {
                            return Poll::Ready(Err(TlsError::Io(std::io::Error::new(
                                std::io::ErrorKind::UnexpectedEof,
                                "failed to read record fragment",
                            ))));
                        }
                        self.read_fragment_pos += n;
                    }
                    Poll::Ready(Err(e)) => return Poll::Ready(Err(TlsError::Io(e))),
                    Poll::Pending => return Poll::Pending,
                }
            }
            let fragment = core::mem::take(&mut self.read_fragment);
            self.read_header = [0; 5];
            self.read_header_pos = 0;
            self.read_fragment_pos = 0;

            if content_type == 23 || content_type == 21 {
                // application_data or encrypted alert
                match self.read_cipher.decrypt_record(content_type, &fragment) {
                    Ok((inner_type, plaintext)) => {
                        if inner_type == 23 {
                            return Poll::Ready(Ok(plaintext));
                        }
                        if inner_type == 21 && plaintext.len() >= 2 {
                            let level =
                                AlertLevel::from_wire(plaintext[0]).map_err(TlsError::Protocol)?;
                            let alert =
                                Alert::from_wire(plaintext[1]).map_err(TlsError::Protocol)?;
                            return Poll::Ready(Err(TlsError::Alert(level, alert)));
                        }
                        // Post-handshake message (e.g., NewSessionTicket) — ignore
                    }
                    Err(e) => return Poll::Ready(Err(e)),
                }
            } else if content_type == 21 {
                // alert
                if fragment.len() >= 2 {
                    let level = AlertLevel::from_wire(fragment[0]).map_err(TlsError::Protocol)?;
                    let alert = Alert::from_wire(fragment[1]).map_err(TlsError::Protocol)?;
                    if level == AlertLevel::Fatal {
                        return Poll::Ready(Err(TlsError::Alert(level, alert)));
                    }
                }
            }
            // content_type 22 (handshake post-handshake) or unknown — skip
        }
    }
}

// ---------------------------------------------------------------------------
// AsyncTlsServerStream — server-side async TLS stream
// ---------------------------------------------------------------------------

/// Async TLS 1.3 server stream wrapping any `AsyncRead + AsyncWrite` transport.
pub struct AsyncTlsServerStream<S> {
    stream: S,
    write_cipher: RecordCipher,
    read_cipher: RecordCipher,
    handshake_done: bool,
    pending_data: Vec<u8>,
    pending_offset: usize,
    cipher_suite: CipherSuite,
    /// Partially written encrypted TLS record.
    write_record: Vec<u8>,
    /// Current write position within `write_record`.
    write_record_pos: usize,
    /// Plaintext byte count represented by `write_record`.
    write_plaintext_len: usize,
    /// Whether the underlying stream still needs flushing after `write_record`.
    write_needs_flush: bool,
    /// The ALPN protocol negotiated during the TLS handshake
    /// (e.g. b"h2" or b"http/1.1").
    alpn_protocol: Option<Vec<u8>>,
}

impl<S> AsyncTlsServerStream<S> {
    /// Check if the TLS handshake has completed.
    pub fn is_handshake_complete(&self) -> bool {
        self.handshake_done
    }

    /// Create a TLS server stream wrapper without performing the handshake.
    /// Use [`Self::handshake()`] to perform the TLS handshake afterwards.
    ///
    /// This is needed for STARTTLS where plaintext bytes are read before
    /// upgrading to TLS on the same connection.
    pub fn new(stream: S) -> Self {
        // Dummy ciphers for pre-handshake state — replaced during handshake.
        // The dummy keys are arbitrary; no real encryption happens before handshake.
        let dummy_key = [0u8; 16];
        let dummy_iv = [0u8; 12];
        let write_cipher = RecordCipher::new(&dummy_key, &dummy_iv)
            .expect("dummy write cipher should always succeed");
        let read_cipher = RecordCipher::new(&dummy_key, &dummy_iv)
            .expect("dummy read cipher should always succeed");
        Self {
            stream,
            write_cipher,
            read_cipher,
            handshake_done: false,
            pending_data: Vec::new(),
            pending_offset: 0,
            cipher_suite: CipherSuite::TLS_AES_128_GCM_SHA256,
            write_record: Vec::new(),
            write_record_pos: 0,
            write_plaintext_len: 0,
            write_needs_flush: false,
            alpn_protocol: None,
        }
    }

    /// Returns the negotiated ALPN protocol (e.g. `b"h2"` or `b"http/1.1"`),
    /// or `None` if no ALPN was negotiated.
    pub fn alpn_protocol(&self) -> Option<&[u8]> {
        self.alpn_protocol.as_deref()
    }

    /// Consume the TLS server stream and return the underlying transport stream.
    pub fn into_inner(self) -> S {
        self.stream
    }
}

impl<S: AsyncRead + AsyncWrite + Unpin> AsyncTlsServerStream<S> {
    /// Accept an async TLS 1.3 handshake from a connected stream.
    pub async fn accept(mut stream: S, cert_and_key: &CertificateAndKey) -> Result<Self> {
        let (write_cipher, read_cipher, cipher_suite, alpn_protocol) =
            server_handshake_impl(&mut stream, cert_and_key).await?;

        Ok(AsyncTlsServerStream {
            stream,
            write_cipher,
            read_cipher,
            handshake_done: true,
            pending_data: Vec::new(),
            pending_offset: 0,
            cipher_suite,
            write_record: Vec::new(),
            write_record_pos: 0,
            write_plaintext_len: 0,
            write_needs_flush: false,
            alpn_protocol,
        })
    }

    /// Perform the TLS 1.3 server handshake on the already-wrapped stream.
    ///
    /// This enables STARTTLS-style upgrades: read plaintext bytes from a
    /// TCP connection, negotiate the upgrade, then call this method to
    /// complete the TLS handshake on the same fd.
    ///
    /// # Example
    /// ```rust
    /// use edgerun_tls::{AsyncRead, AsyncTlsServerStream, AsyncWrite};
    /// use edgerun_tls::certificate_gen::CertificateAndKey;
    ///
    /// async fn starttls<S>(tcp_stream: S, cert: CertificateAndKey) -> edgerun_tls::Result<()>
    /// where
    ///     S: AsyncRead + AsyncWrite + Unpin,
    /// {
    ///     let mut tls_stream = AsyncTlsServerStream::new(tcp_stream);
    ///     // ... read plaintext: EHLO, STARTTLS ...
    ///     // ... send "220 Ready to start TLS" ...
    ///     tls_stream.handshake(&cert).await?;
    ///     // ... continue reading encrypted SMTP commands ...
    ///     Ok(())
    /// }
    /// ```
    pub async fn handshake(&mut self, cert_and_key: &CertificateAndKey) -> Result<()> {
        server_handshake_impl(&mut self.stream, cert_and_key)
            .await
            .map(|(write_cipher, read_cipher, cipher_suite, alpn)| {
                self.write_cipher = write_cipher;
                self.read_cipher = read_cipher;
                self.handshake_done = true;
                self.cipher_suite = cipher_suite;
                self.write_record.clear();
                self.write_record_pos = 0;
                self.write_plaintext_len = 0;
                self.write_needs_flush = false;
                self.alpn_protocol = alpn;
            })
    }

    /// Read decrypted application data.
    pub fn poll_read(
        &mut self,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<std::io::Result<usize>> {
        if !self.handshake_done {
            return Poll::Ready(Err(std::io::Error::new(
                std::io::ErrorKind::NotConnected,
                "TLS handshake not complete",
            )));
        }

        if self.pending_offset < self.pending_data.len() {
            let available = self.pending_data.len() - self.pending_offset;
            let n = available.min(buf.len());
            buf[..n]
                .copy_from_slice(&self.pending_data[self.pending_offset..self.pending_offset + n]);
            self.pending_offset += n;
            return Poll::Ready(Ok(n));
        }

        match self.poll_read_application_data(cx) {
            Poll::Ready(Ok(plaintext)) => {
                let n = plaintext.len().min(buf.len());
                buf[..n].copy_from_slice(&plaintext[..n]);
                if plaintext.len() > n {
                    self.pending_data = plaintext;
                    self.pending_offset = n;
                }
                Poll::Ready(Ok(n))
            }
            Poll::Ready(Err(TlsError::Io(e))) => Poll::Ready(Err(e)),
            Poll::Ready(Err(TlsError::Alert(_, _))) => Poll::Ready(Err(std::io::Error::new(
                std::io::ErrorKind::ConnectionReset,
                "TLS alert",
            ))),
            Poll::Ready(Err(e)) => Poll::Ready(Err(std::io::Error::other(e.to_string()))),
            Poll::Pending => Poll::Pending,
        }
    }

    /// Write encrypted application data.
    pub fn poll_write(&mut self, cx: &mut Context<'_>, buf: &[u8]) -> Poll<std::io::Result<usize>> {
        if !self.handshake_done {
            return Poll::Ready(Err(std::io::Error::new(
                std::io::ErrorKind::NotConnected,
                "TLS handshake not complete",
            )));
        }
        if buf.is_empty() {
            return Poll::Ready(Ok(0));
        }
        if self.write_record.is_empty() {
            let plaintext_len = buf.len().min(TLS_MAX_PLAINTEXT_FRAGMENT);
            let ciphertext = self.write_cipher.encrypt(23, &buf[..plaintext_len]);
            let record = crate::record::TlsRecord {
                content_type: 23,
                version: 0x0303,
                fragment: ciphertext,
            };
            self.write_record = record.to_bytes();
            self.write_record_pos = 0;
            self.write_plaintext_len = plaintext_len;
            self.write_needs_flush = true;
        }

        while self.write_record_pos < self.write_record.len() {
            match Pin::new(&mut self.stream)
                .poll_write(cx, &self.write_record[self.write_record_pos..])
            {
                Poll::Ready(Ok(n)) => {
                    if n == 0 {
                        return Poll::Ready(Err(std::io::Error::new(
                            std::io::ErrorKind::WriteZero,
                            "failed to write whole buffer",
                        )));
                    }
                    self.write_record_pos += n;
                }
                Poll::Ready(Err(e)) => return Poll::Ready(Err(e)),
                Poll::Pending => return Poll::Pending,
            }
        }

        if self.write_needs_flush {
            match Pin::new(&mut self.stream).poll_flush(cx) {
                Poll::Ready(Ok(())) => {
                    self.write_needs_flush = false;
                }
                Poll::Ready(Err(e)) => return Poll::Ready(Err(e)),
                Poll::Pending => return Poll::Pending,
            }
        }

        let written = self.write_plaintext_len;
        self.write_record.clear();
        self.write_record_pos = 0;
        self.write_plaintext_len = 0;
        Poll::Ready(Ok(written))
    }

    /// Flush the underlying transport.
    pub fn poll_flush(&mut self, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.stream).poll_flush(cx)
    }

    /// Shut down the underlying transport.
    pub fn poll_shutdown(&mut self, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.stream).poll_shutdown(cx)
    }

    fn poll_read_application_data(&mut self, cx: &mut Context<'_>) -> Poll<Result<Vec<u8>>> {
        loop {
            // Read record header (5 bytes)
            let mut hdr = [0u8; 5];
            let mut pos = 0;
            loop {
                match Pin::new(&mut self.stream).poll_read(cx, &mut hdr[pos..5]) {
                    Poll::Ready(Ok(n)) => {
                        if n == 0 {
                            return Poll::Ready(Err(TlsError::Io(std::io::Error::new(
                                std::io::ErrorKind::UnexpectedEof,
                                "failed to read record header",
                            ))));
                        }
                        pos += n;
                        if pos == 5 {
                            break;
                        }
                    }
                    Poll::Ready(Err(e)) => return Poll::Ready(Err(TlsError::Io(e))),
                    Poll::Pending => return Poll::Pending,
                }
            }
            let content_type = hdr[0];
            let _version = read_u16_be(&hdr, 1);
            let length = read_u16_be(&hdr, 3) as usize;

            // Read record fragment
            let mut fragment = vec![0u8; length];
            let mut pos = 0;
            loop {
                match Pin::new(&mut self.stream).poll_read(cx, &mut fragment[pos..length]) {
                    Poll::Ready(Ok(n)) => {
                        if n == 0 {
                            return Poll::Ready(Err(TlsError::Io(std::io::Error::new(
                                std::io::ErrorKind::UnexpectedEof,
                                "failed to read record fragment",
                            ))));
                        }
                        pos += n;
                        if pos == length {
                            break;
                        }
                    }
                    Poll::Ready(Err(e)) => return Poll::Ready(Err(TlsError::Io(e))),
                    Poll::Pending => return Poll::Pending,
                }
            }

            if content_type == 23 {
                // application_data
                match self.read_cipher.decrypt(&fragment) {
                    Ok((inner_type, plaintext)) => {
                        if inner_type == 23 {
                            return Poll::Ready(Ok(plaintext));
                        }
                        // Post-handshake message — ignore
                    }
                    Err(e) => return Poll::Ready(Err(TlsError::Cipher(e))),
                }
            } else if content_type == 21 {
                // alert
                if fragment.len() >= 2 {
                    let level = AlertLevel::from_wire(fragment[0]).map_err(TlsError::Protocol)?;
                    let alert = Alert::from_wire(fragment[1]).map_err(TlsError::Protocol)?;
                    if level == AlertLevel::Fatal {
                        return Poll::Ready(Err(TlsError::Alert(level, alert)));
                    }
                }
            }
            // content_type 22 (handshake) or unknown — skip
        }
    }
}

// ---------------------------------------------------------------------------
// Client handshake helpers (async)
// ---------------------------------------------------------------------------

async fn async_read_encrypted_handshake_messages<S: AsyncRead + AsyncWrite + Unpin>(
    stream: &mut S,
    read_cipher: &mut RecordCipher,
    ks: &mut Tls13KeySchedule,
    transcript: &mut Vec<u8>,
    hash: &Hasher,
    handshake_transcript_hash: &[u8],
    server_name: &str,
    unix_secs: u64,
) -> Result<Option<Vec<u8>>> {
    let mut handshake_buf = Vec::new();
    let mut alpn_protocol = None;

    loop {
        let mut hdr = [0u8; 5];
        stream.read_exact(&mut hdr).await?;
        let content_type = hdr[0];
        let len = read_u16_be(&hdr, 3) as usize;
        let mut fragment = vec![0u8; len];
        stream.read_exact(&mut fragment).await?;

        if content_type == 20 {
            // TLS 1.3 compatibility ChangeCipherSpec; ignore it.
            continue;
        }
        if content_type == 21 && fragment.len() >= 2 {
            let level = AlertLevel::from_wire(fragment[0]).map_err(TlsError::Protocol)?;
            let alert = Alert::from_wire(fragment[1]).map_err(TlsError::Protocol)?;
            return Err(TlsError::Alert(level, alert));
        }

        let (inner_type, plaintext) = read_cipher.decrypt(&fragment)?;
        if inner_type == 21 && plaintext.len() >= 2 {
            let level = AlertLevel::from_wire(plaintext[0]).map_err(TlsError::Protocol)?;
            let alert = Alert::from_wire(plaintext[1]).map_err(TlsError::Protocol)?;
            return Err(TlsError::Alert(level, alert));
        }
        if inner_type != 22 {
            continue;
        }

        handshake_buf.extend_from_slice(&plaintext);
        while handshake_buf.len() >= 4 {
            let msg_len = read_u24_be(&handshake_buf, 1) as usize + 4;
            if handshake_buf.len() < msg_len {
                break;
            }
            let msg: Vec<u8> = handshake_buf.drain(..msg_len).collect();
            match msg[0] {
                8 => {
                    // EncryptedExtensions
                    alpn_protocol = parse_encrypted_extensions_alpn(&msg);
                    transcript.extend_from_slice(&msg);
                }
                11 => {
                    // Certificate
                    transcript.extend_from_slice(&msg);
                    if msg.len() >= 8 {
                        let cert_list_len = read_u24_be(&msg, 5) as usize;
                        let cert_list_start = 8;
                        if cert_list_start + cert_list_len <= msg.len() {
                            let mut cert_pos = cert_list_start;
                            let cert_list_end = cert_list_start + cert_list_len;
                            let mut certs = Vec::new();
                            while cert_pos + 5 < cert_list_end {
                                let cert_data_len = read_u24_be(&msg, cert_pos) as usize;
                                cert_pos += 3;
                                if cert_pos + cert_data_len + 2 > cert_list_end {
                                    break;
                                }
                                let cert_der = &msg[cert_pos..cert_pos + cert_data_len];
                                cert_pos += cert_data_len;
                                let ext_len = read_u16_be(&msg, cert_pos) as usize;
                                cert_pos += 2 + ext_len;
                                let cert = Certificate::from_der(cert_der)?;
                                certs.push(cert);
                            }
                            if certs.is_empty() {
                                return Err(TlsError::Certificate(
                                    "No certificates from server".into(),
                                ));
                            }
                            let leaf = &certs[0];
                            if !leaf.is_valid_at_unix_secs(unix_secs) {
                                return Err(TlsError::Certificate(
                                    "Server certificate is expired".into(),
                                ));
                            }
                            if !leaf.matches_hostname(server_name) {
                                return Err(TlsError::Certificate(format!(
                                    "Certificate does not match hostname {}",
                                    server_name,
                                )));
                            }
                            // Full chain validation needs a root store and broader signature
                            // algorithm support. For now, validate time and hostname only.
                        }
                    }
                }
                15 => {
                    // CertificateVerify
                    transcript.extend_from_slice(&msg);
                }
                20 => {
                    // Finished — verify
                    let full_transcript_hash = hash.hash(transcript);
                    let server_hs_secret =
                        ks.server_handshake_traffic_secret(handshake_transcript_hash);
                    let finished_key =
                        hash.expand_label(&server_hs_secret, "finished", &[], hash.len());
                    if msg.len() < 4 + hash.len() {
                        return Err(TlsError::Protocol("Finished message too short".into()));
                    }
                    let verify_data = &msg[4..];
                    let expected_verify_data = match hash {
                        Hasher::Sha256 => hmac_sha256(&finished_key, &full_transcript_hash),
                        Hasher::Sha384 => hmac_sha384(&finished_key, &full_transcript_hash),
                    };
                    if verify_data.len() < expected_verify_data.len()
                        || !constant_time_eq(
                            &verify_data[..expected_verify_data.len()],
                            &expected_verify_data,
                        )
                    {
                        return Err(TlsError::Protocol(
                            "Finished message verification failed".into(),
                        ));
                    }
                    transcript.extend_from_slice(&msg);
                    return Ok(alpn_protocol);
                }
                _ => {}
            }
        }
    }
}

fn parse_encrypted_extensions_alpn(msg: &[u8]) -> Option<Vec<u8>> {
    if msg.len() < 6 || msg[0] != 8 {
        return None;
    }
    let body_len = read_u24_be(msg, 1) as usize;
    if msg.len() < 4 + body_len || body_len < 2 {
        return None;
    }
    let body = &msg[4..4 + body_len];
    let ext_len = u16::from_be_bytes([body[0], body[1]]) as usize;
    if body.len() < 2 + ext_len {
        return None;
    }

    let mut pos = 2usize;
    let end = 2 + ext_len;
    while pos + 4 <= end {
        let ext_type = u16::from_be_bytes([body[pos], body[pos + 1]]);
        let ext_data_len = u16::from_be_bytes([body[pos + 2], body[pos + 3]]) as usize;
        pos += 4;
        if pos + ext_data_len > end {
            return None;
        }
        if ext_type == 16 {
            return parse_alpn_extension_data(&body[pos..pos + ext_data_len]);
        }
        pos += ext_data_len;
    }
    None
}

fn parse_alpn_extension_data(data: &[u8]) -> Option<Vec<u8>> {
    if data.len() < 3 {
        return None;
    }
    let list_len = u16::from_be_bytes([data[0], data[1]]) as usize;
    if list_len == 0 || data.len() < 2 + list_len {
        return None;
    }
    let protocol_len = data[2] as usize;
    if protocol_len == 0 || 3 + protocol_len > 2 + list_len {
        return None;
    }
    Some(data[3..3 + protocol_len].to_vec())
}

async fn async_send_client_finished<S: AsyncRead + AsyncWrite + Unpin>(
    stream: &mut S,
    write_cipher: &mut RecordCipher,
    ks: &Tls13KeySchedule,
    transcript: &[u8],
    hash: &Hasher,
    handshake_transcript_hash: &[u8],
) -> Result<()> {
    let full_transcript_hash = hash.hash(transcript);
    let client_hs_secret = ks.client_handshake_traffic_secret(handshake_transcript_hash);

    let finished_key = hash.expand_label(&client_hs_secret, "finished", &[], hash.len());
    // RFC 8446 §4.4.3: verify_data uses hash of all handshake messages including Finished
    let verify_data = match hash {
        Hasher::Sha256 => hmac_sha256(&finished_key, &full_transcript_hash),
        Hasher::Sha384 => hmac_sha384(&finished_key, &full_transcript_hash),
    };
    let finished_msg = build_finished_message(&verify_data);
    let finished_ct = write_cipher.encrypt(22, &finished_msg);
    let record = crate::record::TlsRecord {
        content_type: 23,
        version: 0x0303,
        fragment: finished_ct,
    };
    stream.write_all(&record.to_bytes()).await?;
    stream.flush().await?;
    Ok(())
}

struct Tls12Keys {
    client_key: Vec<u8>,
    server_key: Vec<u8>,
    client_iv: Vec<u8>,
    server_iv: Vec<u8>,
}

fn build_tls12_client_hello(client_random: [u8; 32], server_name: &str) -> Vec<u8> {
    let mut body = Vec::new();
    body.extend_from_slice(&0x0303u16.to_be_bytes());
    body.extend_from_slice(&client_random);
    body.push(0);
    for bytes in [&2u16.to_be_bytes(), &0xC02Fu16.to_be_bytes()] {
        body.extend_from_slice(bytes);
    }
    body.push(1);
    body.push(0);

    let mut extensions = Vec::new();
    extensions.extend_from_slice(&0xff01u16.to_be_bytes());
    extensions.extend_from_slice(&1u16.to_be_bytes());
    extensions.push(0);

    let mut sni = Vec::new();
    sni.extend_from_slice(&((1 + 2 + server_name.len()) as u16).to_be_bytes());
    sni.push(0);
    sni.extend_from_slice(&(server_name.len() as u16).to_be_bytes());
    sni.extend_from_slice(server_name.as_bytes());
    extensions.extend_from_slice(&0u16.to_be_bytes());
    extensions.extend_from_slice(&(sni.len() as u16).to_be_bytes());
    extensions.extend_from_slice(&sni);

    extensions.extend_from_slice(&10u16.to_be_bytes());
    extensions.extend_from_slice(&4u16.to_be_bytes());
    extensions.extend_from_slice(&[0, 2, 0, 0x17]);
    extensions.extend_from_slice(&11u16.to_be_bytes());
    extensions.extend_from_slice(&2u16.to_be_bytes());
    extensions.extend_from_slice(&[1, 0]);
    extensions.extend_from_slice(&35u16.to_be_bytes());
    extensions.extend_from_slice(&0u16.to_be_bytes());
    extensions.extend_from_slice(&22u16.to_be_bytes());
    extensions.extend_from_slice(&0u16.to_be_bytes());
    extensions.extend_from_slice(&23u16.to_be_bytes());
    extensions.extend_from_slice(&0u16.to_be_bytes());
    extensions.extend_from_slice(&13u16.to_be_bytes());
    extensions.extend_from_slice(&16u16.to_be_bytes());
    extensions.extend_from_slice(&[0, 14, 8, 4, 4, 1, 4, 3, 5, 1, 5, 3, 6, 1, 6, 3]);

    body.extend_from_slice(&(extensions.len() as u16).to_be_bytes());
    body.extend_from_slice(&extensions);
    handshake_message(1, &body)
}

fn build_tls12_client_key_exchange(public_key: &[u8]) -> Vec<u8> {
    let mut body = Vec::with_capacity(1 + public_key.len());
    body.push(public_key.len() as u8);
    body.extend_from_slice(public_key);
    handshake_message(16, &body)
}

fn build_tls12_finished(verify_data: &[u8]) -> Vec<u8> {
    handshake_message(20, verify_data)
}

fn handshake_message(kind: u8, body: &[u8]) -> Vec<u8> {
    let mut msg = Vec::with_capacity(4 + body.len());
    msg.push(kind);
    msg.extend_from_slice(&(body.len() as u32).to_be_bytes()[1..]);
    msg.extend_from_slice(body);
    msg
}

async fn write_plain_record<S: AsyncRead + AsyncWrite + Unpin>(
    stream: &mut S,
    content_type: u8,
    fragment: &[u8],
) -> Result<()> {
    write_plain_record_with_version(stream, content_type, 0x0303, fragment).await
}

async fn write_plain_record_with_version<S: AsyncRead + AsyncWrite + Unpin>(
    stream: &mut S,
    content_type: u8,
    version: u16,
    fragment: &[u8],
) -> Result<()> {
    let mut record = Vec::with_capacity(5 + fragment.len());
    record.push(content_type);
    record.extend_from_slice(&version.to_be_bytes());
    record.extend_from_slice(&(fragment.len() as u16).to_be_bytes());
    record.extend_from_slice(fragment);
    stream.write_all(&record).await?;
    stream.flush().await?;
    Ok(())
}

async fn read_plain_record<S: AsyncRead + AsyncWrite + Unpin>(
    stream: &mut S,
) -> Result<(u8, Vec<u8>)> {
    let mut hdr = [0u8; 5];
    stream.read_exact(&mut hdr).await?;
    let len = read_u16_be(&hdr, 3) as usize;
    let mut fragment = vec![0u8; len];
    stream.read_exact(&mut fragment).await?;
    Ok((hdr[0], fragment))
}

fn parse_alert_record(fragment: &[u8]) -> TlsError {
    if fragment.len() >= 2 {
        let level = AlertLevel::from_wire(fragment[0]).unwrap_or(AlertLevel::Fatal);
        let alert = Alert::from_wire(fragment[1]).unwrap_or(Alert::HandshakeFailure);
        TlsError::Alert(level, alert)
    } else {
        TlsError::Protocol("TLS alert record truncated".into())
    }
}

fn record_handshake_has_type(record: &[u8], wanted: u8) -> bool {
    let mut pos = 0usize;
    while pos + 4 <= record.len() {
        let len = read_u24_be(record, pos + 1) as usize;
        if record[pos] == wanted {
            return true;
        }
        pos = pos.saturating_add(4 + len);
    }
    false
}

fn parse_tls12_server_hello(
    msg: &[u8],
    random: &mut [u8; 32],
    cipher_suite: &mut u16,
    extended_master_secret: &mut bool,
) -> Result<()> {
    if msg.len() < 42 || msg[0] != 2 {
        return Err(TlsError::Protocol("TLS 1.2 ServerHello truncated".into()));
    }
    let body = &msg[4..];
    random.copy_from_slice(&body[2..34]);
    let suite_pos = 35 + body[34] as usize;
    if suite_pos + 3 > body.len() {
        return Err(TlsError::Protocol("TLS 1.2 ServerHello invalid".into()));
    }
    *cipher_suite = read_u16_be(body, suite_pos);
    if !matches!(*cipher_suite, 0xC02B | 0xC02C | 0xC02F | 0xC030) {
        return Err(TlsError::Protocol(format!(
            "TLS 1.2 unsupported cipher suite 0x{cipher_suite:04x}",
        )));
    }
    *extended_master_secret = false;
    let ext_len_pos = suite_pos + 3;
    if ext_len_pos + 2 <= body.len() {
        let ext_len = read_u16_be(body, ext_len_pos) as usize;
        let mut pos = ext_len_pos + 2;
        let end = pos.saturating_add(ext_len).min(body.len());
        while pos + 4 <= end {
            let ext_type = read_u16_be(body, pos);
            let len = read_u16_be(body, pos + 2) as usize;
            pos += 4;
            if pos + len > end {
                break;
            }
            if ext_type == 23 {
                *extended_master_secret = true;
            }
            pos += len;
        }
    }
    Ok(())
}

fn parse_tls12_server_key_exchange(msg: &[u8]) -> Result<Vec<u8>> {
    if msg.len() < 12 || msg[0] != 12 {
        return Err(TlsError::Protocol(
            "TLS 1.2 ServerKeyExchange missing".into(),
        ));
    }
    let body = &msg[4..];
    if body[0] != 3 {
        return Err(TlsError::Protocol(
            "TLS 1.2 only named curves are supported".into(),
        ));
    }
    let group = read_u16_be(body, 1);
    let key_len = body[3] as usize;
    if body.len() < 4 + key_len {
        return Err(TlsError::Protocol("TLS 1.2 ECDHE key truncated".into()));
    }
    if !matches!(group, 0x0017 | 0x001D) {
        return Err(TlsError::Protocol(format!(
            "TLS 1.2 unsupported ECDHE group 0x{group:04x}",
        )));
    }
    Ok(body[4..4 + key_len].to_vec())
}

fn split_tls12_key_block(cipher_suite: u16, key_block: &[u8]) -> Result<Tls12Keys> {
    let key_len = match cipher_suite {
        0xC02B | 0xC02F => 16,
        0xC02C | 0xC030 => 32,
        _ => {
            return Err(TlsError::Protocol(
                "TLS 1.2 unsupported cipher suite".into(),
            ));
        }
    };
    let mut pos = 0usize;
    let client_key = key_block[pos..pos + key_len].to_vec();
    pos += key_len;
    let server_key = key_block[pos..pos + key_len].to_vec();
    pos += key_len;
    let client_iv = key_block[pos..pos + 4].to_vec();
    pos += 4;
    let server_iv = key_block[pos..pos + 4].to_vec();
    Ok(Tls12Keys {
        client_key,
        server_key,
        client_iv,
        server_iv,
    })
}

fn tls12_key_block_len(cipher_suite: u16) -> Result<usize> {
    match cipher_suite {
        0xC02B | 0xC02F => Ok(40),
        0xC02C | 0xC030 => Ok(72),
        _ => Err(TlsError::Protocol(
            "TLS 1.2 unsupported cipher suite".into(),
        )),
    }
}

fn tls12_prf(
    cipher_suite: u16,
    secret: &[u8],
    label: &[u8],
    seed: &[u8],
    len: usize,
) -> Result<Vec<u8>> {
    let mut label_seed = Vec::with_capacity(label.len() + seed.len());
    label_seed.extend_from_slice(label);
    label_seed.extend_from_slice(seed);
    Ok(match cipher_suite {
        0xC02B | 0xC02F => p_hash(secret, &label_seed, len, hmac_sha256),
        0xC02C | 0xC030 => p_hash(secret, &label_seed, len, hmac_sha384),
        _ => return Err(TlsError::Protocol("TLS 1.2 unsupported PRF suite".into())),
    })
}

fn p_hash(secret: &[u8], seed: &[u8], len: usize, hmac: fn(&[u8], &[u8]) -> Vec<u8>) -> Vec<u8> {
    let mut out = Vec::with_capacity(len);
    let mut a = hmac(secret, seed);
    while out.len() < len {
        let mut input = Vec::with_capacity(a.len() + seed.len());
        input.extend_from_slice(&a);
        input.extend_from_slice(seed);
        out.extend_from_slice(&hmac(secret, &input));
        a = hmac(secret, &a);
    }
    out.truncate(len);
    out
}

fn tls12_finished_verify_data(
    cipher_suite: u16,
    master_secret: &[u8],
    label: &[u8],
    transcript: &[u8],
) -> Result<Vec<u8>> {
    let hash = match cipher_suite {
        0xC02B | 0xC02F => Hasher::Sha256.hash(transcript),
        0xC02C | 0xC030 => Hasher::Sha384.hash(transcript),
        _ => {
            return Err(TlsError::Protocol(
                "TLS 1.2 unsupported Finished suite".into(),
            ));
        }
    };
    tls12_prf(cipher_suite, master_secret, label, &hash, 12)
}

fn tls12_handshake_hash(cipher_suite: u16, transcript: &[u8]) -> Result<Vec<u8>> {
    Ok(match cipher_suite {
        0xC02B | 0xC02F => Hasher::Sha256.hash(transcript),
        0xC02C | 0xC030 => Hasher::Sha384.hash(transcript),
        _ => {
            return Err(TlsError::Protocol(
                "TLS 1.2 unsupported handshake hash suite".into(),
            ));
        }
    })
}

// ---------------------------------------------------------------------------
// Server handshake helpers (async)
// ---------------------------------------------------------------------------

async fn async_server_send_encrypted_handshake<S: AsyncRead + AsyncWrite + Unpin>(
    stream: &mut S,
    write_cipher: &mut RecordCipher,
    ks: &mut Tls13KeySchedule,
    transcript: &mut Vec<u8>,
    hash: &Hasher,
    handshake_transcript_hash: &[u8],
    cert_chain_der: &[Vec<u8>],
    signing_key: &edgerun_crypto::p256::ecdsa::SigningKey,
    alpn_protocol: Option<&[u8]>,
) -> Result<()> {
    // EncryptedExtensions
    let ee_msg = build_encrypted_extensions(alpn_protocol);
    transcript.extend_from_slice(&ee_msg);
    let ee_ct = write_cipher.encrypt(22, &ee_msg);
    stream
        .write_all(
            &crate::record::TlsRecord {
                content_type: 23,
                version: 0x0303,
                fragment: ee_ct,
            }
            .to_bytes(),
        )
        .await?;

    // Certificate
    let cert_refs: Vec<&[u8]> = cert_chain_der.iter().map(Vec::as_slice).collect();
    let cert_msg = build_certificate_chain_message(&cert_refs);
    transcript.extend_from_slice(&cert_msg);
    let cert_ct = write_cipher.encrypt(22, &cert_msg);
    stream
        .write_all(
            &crate::record::TlsRecord {
                content_type: 23,
                version: 0x0303,
                fragment: cert_ct,
            }
            .to_bytes(),
        )
        .await?;

    // CertificateVerify
    let cv_msg = build_certificate_verify(transcript, signing_key, hash)?;
    transcript.extend_from_slice(&cv_msg);
    let cv_ct = write_cipher.encrypt(22, &cv_msg);
    stream
        .write_all(
            &crate::record::TlsRecord {
                content_type: 23,
                version: 0x0303,
                fragment: cv_ct,
            }
            .to_bytes(),
        )
        .await?;

    // Finished
    let transcript_hash_before_finished = hash.hash(transcript);
    let server_hs_secret = ks.server_handshake_traffic_secret(handshake_transcript_hash);
    let verify_data = compute_server_finished_verify_data(
        &server_hs_secret,
        &transcript_hash_before_finished,
        hash,
    );
    let finished_msg = build_finished_message(&verify_data);
    transcript.extend_from_slice(&finished_msg);
    let finished_ct = write_cipher.encrypt(22, &finished_msg);
    stream
        .write_all(
            &crate::record::TlsRecord {
                content_type: 23,
                version: 0x0303,
                fragment: finished_ct,
            }
            .to_bytes(),
        )
        .await?;
    stream.flush().await?;

    Ok(())
}

async fn async_server_read_client_finished<S: AsyncRead + AsyncWrite + Unpin>(
    stream: &mut S,
    read_cipher: &mut RecordCipher,
    ks: &Tls13KeySchedule,
    transcript: &mut Vec<u8>,
    hash: &Hasher,
    handshake_transcript_hash: &[u8],
) -> Result<()> {
    loop {
        let mut hdr = [0u8; 5];
        stream.read_exact(&mut hdr).await?;
        let ct = hdr[0];
        let len = read_u16_be(&hdr, 3) as usize;
        if ct == 20 {
            // ChangeCipherSpec — skip
            let _fragment = {
                let mut buf = vec![0u8; len];
                stream.read_exact(&mut buf).await?;
                buf
            };
            continue;
        }
        if ct != 23 {
            return Err(TlsError::HandshakeFailure(format!(
                "Expected encrypted record for client Finished, got content_type={ct}",
            )));
        }
        let mut fragment = vec![0u8; len];
        stream.read_exact(&mut fragment).await?;
        let (inner_type, plaintext) = read_cipher.decrypt(&fragment)?;
        let hs_type = if !plaintext.is_empty() {
            plaintext[0]
        } else {
            inner_type
        };
        if hs_type != 20 {
            return Err(TlsError::HandshakeFailure(format!(
                "Expected Finished (type 20), got {}",
                hs_type
            )));
        }
        let transcript_hash = hash.hash(transcript);
        // RFC 8446 §4.4.3: verify_data is computed from hash of all handshake messages
        // including the Finished being verified.
        let client_hs_secret = ks.client_handshake_traffic_secret(handshake_transcript_hash);
        let expected_verify =
            compute_client_finished_verify_data(&client_hs_secret, &transcript_hash, hash);
        if plaintext.len() < 4 + expected_verify.len() {
            return Err(TlsError::Protocol(
                "Client Finished verification data too short".into(),
            ));
        }
        let client_verify_data = &plaintext[4..4 + expected_verify.len()];
        if !constant_time_eq(client_verify_data, &expected_verify) {
            return Err(TlsError::Protocol(
                "Client Finished verification failed".into(),
            ));
        }
        transcript.extend_from_slice(&plaintext);
        break;
    }
    Ok(())
}

/// Server-side TLS handshake on an existing `&mut S`.
///
/// This is the extracted handshake logic from `accept()`, reused by
/// `AsyncTlsServerStream::handshake()` for STARTTLS upgrades.
async fn server_handshake_impl<S: AsyncRead + AsyncWrite + Unpin>(
    stream: &mut S,
    cert_and_key: &CertificateAndKey,
) -> Result<(RecordCipher, RecordCipher, CipherSuite, Option<Vec<u8>>)> {
    // 1. Read ClientHello
    let mut hdr = [0u8; 5];
    stream.read_exact(&mut hdr).await?;
    let ct = hdr[0];
    if ct != 22 {
        return Err(TlsError::HandshakeFailure(format!(
            "Expected handshake record, got content_type={ct}",
        )));
    }
    let len = read_u16_be(&hdr, 3) as usize;
    let mut fragment = vec![0u8; len];
    stream.read_exact(&mut fragment).await?;
    let ch = ClientHello::parse(&fragment)?;

    if !ch.supported_versions.contains(&0x0304) {
        return Err(TlsError::HandshakeFailure(
            "Client does not support TLS 1.3".into(),
        ));
    }

    let negotiated_suite = ch
        .cipher_suites
        .iter()
        .find(|cs| matches!(cs, CipherSuite::TLS_AES_128_GCM_SHA256))
        .or_else(|| {
            ch.cipher_suites
                .iter()
                .find(|cs| matches!(cs, CipherSuite::TLS_AES_256_GCM_SHA384))
        })
        .cloned()
        .ok_or_else(|| TlsError::HandshakeFailure("No common cipher suite".into()))?;

    // Prefer HTTP/1.1 until the HTTP/2 server path is robust enough for browsers.
    let alpn_protocol = if !ch.alpn_protocols.is_empty() {
        if ch.alpn_protocols.iter().any(|p| p == b"http/1.1") {
            Some(b"http/1.1".to_vec())
        } else if ch.alpn_protocols.iter().any(|p| p == b"h2") {
            Some(b"h2".to_vec())
        } else {
            None
        }
    } else {
        None
    };

    let server_random = generate_random();

    let (selected_group, client_key_share) = if let Some((group, key)) = ch.all_key_shares.first() {
        let keg = match group {
            crate::cipher::NamedGroup::X25519 => KeyExchangeGroup::X25519,
            _ => KeyExchangeGroup::SECP256R1,
        };
        (keg, key.clone())
    } else if let Some(ks) = ch.client_key_share {
        let keg = match ch.client_key_share_group {
            Some(crate::cipher::NamedGroup::X25519) => KeyExchangeGroup::X25519,
            _ => KeyExchangeGroup::SECP256R1,
        };
        (keg, ks)
    } else {
        return Err(TlsError::HandshakeFailure(
            "No key_share in ClientHello".into(),
        ));
    };

    let client_session_id = ch.session_id;
    let mut transcript = fragment;

    // 2. Send ServerHello
    let key_pair = EcdhKeyPair::generate(selected_group).map_err(TlsError::HandshakeFailure)?;
    let public_key = key_pair.public_key_bytes();
    let named_group = match selected_group {
        KeyExchangeGroup::SECP256R1 => NamedGroup::SECP256R1,
        KeyExchangeGroup::X25519 => NamedGroup::X25519,
    };
    let sh_msg = build_server_hello(
        server_random,
        &client_session_id,
        negotiated_suite,
        &public_key,
        named_group,
    );
    transcript.extend_from_slice(&sh_msg);

    let record = crate::record::TlsRecord {
        content_type: 22,
        version: 0x0303,
        fragment: sh_msg,
    };
    stream.write_all(&record.to_bytes()).await?;
    stream.flush().await?;

    // 3. Derive handshake keys.
    let client_key_share = client_key_share.clone();
    let shared_secret = key_pair
        .exchange(&client_key_share)
        .map_err(TlsError::HandshakeFailure)?;
    let hash = Hasher::Sha256;
    let transcript_hash = hash.hash(&transcript);

    let mut ks = Tls13KeySchedule::new(hash.clone());
    ks.advance_to_handshake(&shared_secret, &transcript_hash, &transcript_hash);

    let server_hs_secret = ks.server_handshake_traffic_secret(&transcript_hash);
    let client_hs_secret = ks.client_handshake_traffic_secret(&transcript_hash);

    let server_hs_keys =
        server_write_keys(&server_hs_secret, negotiated_suite.key_len(), 12, &hash);
    let client_hs_keys =
        client_write_keys(&client_hs_secret, negotiated_suite.key_len(), 12, &hash);

    let mut write_cipher = RecordCipher::new(&server_hs_keys.write_key, &server_hs_keys.write_iv)?;
    let mut read_cipher = RecordCipher::new(&client_hs_keys.write_key, &client_hs_keys.write_iv)?;

    let handshake_transcript_hash = transcript_hash;

    // 4. Send encrypted handshake messages
    async_server_send_encrypted_handshake(
        stream,
        &mut write_cipher,
        &mut ks,
        &mut transcript,
        &hash,
        &handshake_transcript_hash,
        &cert_and_key.cert_chain_der,
        &cert_and_key.signing_key,
        alpn_protocol.as_deref(),
    )
    .await?;

    let app_transcript_hash = hash.hash(&transcript);

    // 5. Read client Finished
    async_server_read_client_finished(
        stream,
        &mut read_cipher,
        &ks,
        &mut transcript,
        &hash,
        &handshake_transcript_hash,
    )
    .await?;

    // 6. Derive application keys
    ks.advance_to_master();
    let server_app = ks.server_app_traffic_secret(&app_transcript_hash);
    let client_app = ks.client_app_traffic_secret(&app_transcript_hash);

    let server_app_keys = server_app_write_keys(&server_app, negotiated_suite.key_len(), 12, &hash);
    let client_app_keys = client_app_write_keys(&client_app, negotiated_suite.key_len(), 12, &hash);

    let write_cipher = RecordCipher::new(&server_app_keys.write_key, &server_app_keys.write_iv)?;
    let read_cipher = RecordCipher::new(&client_app_keys.write_key, &client_app_keys.write_iv)?;

    Ok((write_cipher, read_cipher, negotiated_suite, alpn_protocol))
}

// ---------------------------------------------------------------------------
// Utilities
// ---------------------------------------------------------------------------

pub(crate) fn generate_random() -> [u8; 32] {
    let mut buf = [0u8; 32];
    edgerun_crypto::fill_random(&mut buf).expect("random generation failed");
    buf
}

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

fn current_unix_secs() -> u64 {
    1_704_067_200
}

#[cfg(test)]
mod tests {
    use super::parse_encrypted_extensions_alpn;

    #[test]
    fn parses_alpn_from_encrypted_extensions() {
        let msg = [
            8, 0, 0, 11, // EncryptedExtensions, 11-byte body
            0, 9, // extensions length
            0, 16, // ALPN extension
            0, 5, // extension data length
            0, 3, // protocol name list length
            2, b'h', b'2',
        ];

        assert_eq!(parse_encrypted_extensions_alpn(&msg), Some(b"h2".to_vec()));
    }

    #[test]
    fn ignores_encrypted_extensions_without_alpn() {
        let msg = [
            8, 0, 0, 4, // EncryptedExtensions, 4-byte body
            0, 2, // extensions length
            0, 0, // empty SNI extension
        ];

        assert_eq!(parse_encrypted_extensions_alpn(&msg), None);
    }
}

/// Parse the selected group from a HelloRetryRequest key_share extension.
/// HRR key_share extension format: group_id (2 bytes) only.
fn parse_hrr_selected_group(data: &[u8]) -> Result<crate::key_exchange::KeyExchangeGroup> {
    use crate::key_exchange::KeyExchangeGroup;

    // Skip handshake header: type (1) + length (3) + legacy_version (2) + random (32) +
    // legacy_session_id_echo (1 byte length + variable) + cipher_suite (2) + legacy_compression (1)
    let mut pos = 1 + 3 + 2 + 32;
    if pos >= data.len() {
        return Err(TlsError::Protocol("HRR: session_id length missing".into()));
    }
    let sid_len = data[pos] as usize;
    pos += 1 + sid_len;
    if pos + 2 + 1 + 2 > data.len() {
        return Err(TlsError::Protocol("HRR: too short for extensions".into()));
    }
    // cipher_suite (2) + legacy_compression (1) + extensions_length (2)
    pos += 2 + 1;
    let ext_len = read_u16_be(data, pos) as usize;
    pos += 2;

    let ext_end = pos + ext_len;
    while pos < ext_end {
        if pos + 4 > ext_end {
            break;
        }
        let ext_type = read_u16_be(data, pos);
        let ext_data_len = read_u16_be(data, pos + 2) as usize;
        pos += 4;

        // key_share extension (type 51) in HRR contains only selected_group (2 bytes)
        if ext_type == 51 && ext_data_len >= 2 {
            let group_id = read_u16_be(data, pos);
            return KeyExchangeGroup::from_wire(group_id)
                .ok_or_else(|| TlsError::Protocol(format!("HRR: unknown group {group_id:#06x}")));
        }

        pos += ext_data_len;
    }

    Err(TlsError::Protocol(
        "HRR: no key_share extension found".into(),
    ))
}

/// Parse the cookie from a HelloRetryRequest (optional).
fn parse_hrr_cookie(data: &[u8]) -> Option<Vec<u8>> {
    // Skip handshake header: type (1) + length (3) + legacy_version (2) + random (32) +
    // legacy_session_id_echo (1 byte length + variable) + cipher_suite (2) + legacy_compression (1)
    let mut pos = 1 + 3 + 2 + 32;
    if pos >= data.len() {
        return None;
    }
    let sid_len = data[pos] as usize;
    pos += 1 + sid_len;
    if pos + 2 + 1 + 2 > data.len() {
        return None;
    }
    pos += 2 + 1;
    let ext_len = read_u16_be(data, pos) as usize;
    pos += 2;

    let ext_end = pos + ext_len;
    while pos < ext_end {
        if pos + 4 > ext_end {
            break;
        }
        let ext_type = read_u16_be(data, pos);
        let ext_data_len = read_u16_be(data, pos + 2) as usize;
        pos += 4;

        // cookie extension (type 44)
        if ext_type == 44 && ext_data_len > 0 {
            if pos + ext_data_len <= data.len() {
                return Some(data[pos..pos + ext_data_len].to_vec());
            }
        }

        pos += ext_data_len;
    }

    None
}
