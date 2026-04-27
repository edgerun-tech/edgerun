//! Async TLS 1.3 streams — client and server — wrapping any
//! `edgerun_rt::AsyncRead + edgerun_rt::AsyncWrite + Unpin` transport.
//!
//! Mirrors the sync `TlsStream` / `TlsServerStream` API but uses
//! async `poll_read` / `poll_write` instead of `std::io::Read` / `Write`.
//!
//! # Example (client)
//! ```ignore
//! use edgerun_tls::async_tls::AsyncTlsStream;
//! use std::sync::Arc;
//!
//! // AsyncTlsStream works with any async read/write stream
//! async fn tls_client_example() {
//!     // let tcp_stream = ... // some async TCP connection
//!     // let arc_stream = Arc::new(tcp_stream);
//!     // let mut tls = AsyncTlsStream::client(arc_stream, "example.com").await.unwrap();
//!     // tls.write_all(b"GET / HTTP/1.1\r\n").await.unwrap();
//! }
//! ```

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
    build_certificate_message, build_certificate_verify, build_encrypted_extensions,
    build_finished_message, build_server_hello, compute_client_finished_verify_data,
    compute_server_finished_verify_data,
};
use crate::session_cache::SessionCache;
use crate::{Result, TlsError};
use edgerun_crypto::CipherSuite;

use edgerun_rt::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

// ---------------------------------------------------------------------------
// AsyncTlsStream — client-side async TLS stream
// ---------------------------------------------------------------------------

/// Async TLS 1.3 client stream wrapping any `AsyncRead + AsyncWrite` transport.
pub struct AsyncTlsStream<S> {
    /// The underlying transport stream.
    stream: S,
    /// The expected server hostname for SNI and certificate verification.
    server_name: String,
    /// The negotiated cipher suite.
    cipher_suite: CipherSuite,
    /// Record cipher for writing (client → server).
    write_cipher: RecordCipher,
    /// Record cipher for reading (server → client).
    read_cipher: RecordCipher,
    /// Whether the TLS handshake has completed.
    handshake_done: bool,
    /// Buffered application data that was read but not yet consumed.
    pending_data: Vec<u8>,
    /// Current read position within `pending_data`.
    pending_offset: usize,
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
        let client_random = generate_random();
        let key_pair =
            EcdhKeyPair::generate(KeyExchangeGroup::X25519).map_err(TlsError::HandshakeFailure)?;
        let cipher_suite = CipherSuite::TLS_AES_128_GCM_SHA256;

        // 1. Send ClientHello
        let public_key = key_pair.public_key_bytes();
        let group = match key_pair.group() {
            KeyExchangeGroup::SECP256R1 => NamedGroup::SECP256R1,
            KeyExchangeGroup::X25519 => NamedGroup::X25519,
        };
        let mut ch_builder = ClientHelloBuilder::new(client_random, server_name)
            .key_share(&key_pair.public_key_bytes(), group);

        // Try session resumption if cache provided
        if let Some(cache) = session_cache {
            if let Some(ticket) = cache.get(server_name) {
                ch_builder = ch_builder.psk_identity(&ticket.ticket, 0, ticket.obfuscated_age());
            }
        }

        if !alpn_protocols.is_empty() {
            ch_builder = ch_builder.alpn_protocols(alpn_protocols);
        }
        let ch = ch_builder.build()?;

        let _ch_hash = Hasher::Sha256.hash(&ch);
        let mut transcript = ch.clone();

        let record = crate::record::TlsRecord {
            content_type: 22,
            version: 0x0303,
            fragment: ch,
        };
        stream.write_all(&record.to_bytes()).await?;
        stream.flush().await?;

        // 2. Read ServerHello (plaintext)
        let mut hdr = [0u8; 5];
        stream.read_exact(&mut hdr).await?;
        let ct = hdr[0];
        if ct != 22 {
            return Err(TlsError::HandshakeFailure(format!(
                "Expected handshake record, got content_type={ct}",
            )));
        }
        let len = u16::from_be_bytes([hdr[3], hdr[4]]) as usize;
        let mut fragment = vec![0u8; len];
        stream.read_exact(&mut fragment).await?;
        let sh = ServerHello::parse(&fragment)?;

        if sh.supported_version != Some(0x0304) {
            return Err(TlsError::HandshakeFailure(format!(
                "Server did not negotiate TLS 1.3 (got supported_version={:?})",
                sh.supported_version,
            )));
        }

        let _server_random = sh.random;
        let negotiated_suite = sh.cipher_suite;
        let _sh_hash = Hasher::Sha256.hash(&fragment);
        transcript.extend_from_slice(&fragment);

        // 3. Derive handshake keys — offload ECDH to blocking pool
        let key_pair = std::sync::Arc::new(key_pair);
        let server_key_share = sh.server_key_share.clone();
        let shared_secret =
            edgerun_rt::spawn_blocking(move || key_pair.exchange(&server_key_share))
                .await
                .map_err(|_| TlsError::HandshakeFailure("blocking pool shutdown".into()))
                .and_then(|r| r.map_err(TlsError::HandshakeFailure))?;
        let hash = Hasher::Sha256;
        let transcript_hash = hash.hash(&transcript);

        let mut ks = Tls13KeySchedule::new(hash.clone());
        ks.advance_to_handshake(&shared_secret, &transcript_hash, &transcript_hash);
        let client_hs_secret = ks.client_handshake_traffic_secret(&transcript_hash);
        let server_hs_secret = ks.server_handshake_traffic_secret(&transcript_hash);

        let client_hs_keys =
            client_write_keys(&client_hs_secret, cipher_suite.key_len(), 12, &hash);
        let server_hs_keys =
            server_write_keys(&server_hs_secret, cipher_suite.key_len(), 12, &hash);

        let mut _write_cipher =
            RecordCipher::new(&client_hs_keys.write_key, &client_hs_keys.write_iv)?;
        let mut _read_cipher =
            RecordCipher::new(&server_hs_keys.write_key, &server_hs_keys.write_iv)?;

        // 4. Read encrypted handshake messages
        async_read_encrypted_handshake_messages(
            &mut stream,
            &mut _read_cipher,
            &mut ks,
            &mut transcript,
            &hash,
            &transcript_hash,
            server_name,
        )
        .await?;

        let app_transcript_hash = hash.hash(&transcript);

        // 5. Send client Finished
        async_send_client_finished(
            &mut stream,
            &mut _write_cipher,
            &ks,
            &transcript,
            &hash,
            &transcript_hash,
        )
        .await?;

        // 6. Derive application keys
        ks.advance_to_master();
        let client_app = ks.client_app_traffic_secret(&app_transcript_hash);
        let server_app = ks.server_app_traffic_secret(&app_transcript_hash);

        let client_app_keys = client_app_write_keys(&client_app, cipher_suite.key_len(), 12, &hash);
        let server_app_keys = server_app_write_keys(&server_app, cipher_suite.key_len(), 12, &hash);

        let write_cipher =
            RecordCipher::new(&client_app_keys.write_key, &client_app_keys.write_iv)?;
        let read_cipher = RecordCipher::new(&server_app_keys.write_key, &server_app_keys.write_iv)?;

        Ok(AsyncTlsStream {
            stream,
            server_name: server_name.to_string(),
            cipher_suite: negotiated_suite,
            write_cipher,
            read_cipher,
            handshake_done: true,
            pending_data: Vec::new(),
            pending_offset: 0,
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
        let ciphertext = self.write_cipher.encrypt(23, buf);
        let record = crate::record::TlsRecord {
            content_type: 23,
            version: 0x0303,
            fragment: ciphertext,
        };
        let bytes = record.to_bytes();

        // Write all bytes, then flush — inline poll loop
        let mut pos = 0;
        while pos < bytes.len() {
            match Pin::new(&mut self.stream).poll_write(cx, &bytes[pos..]) {
                Poll::Ready(Ok(n)) => {
                    if n == 0 {
                        return Poll::Ready(Err(std::io::Error::new(
                            std::io::ErrorKind::WriteZero,
                            "failed to write whole buffer",
                        )));
                    }
                    pos += n;
                }
                Poll::Ready(Err(e)) => return Poll::Ready(Err(e)),
                Poll::Pending => return Poll::Pending,
            }
        }
        // Flush
        match Pin::new(&mut self.stream).poll_flush(cx) {
            Poll::Ready(Ok(())) => {}
            Poll::Ready(Err(e)) => return Poll::Ready(Err(e)),
            Poll::Pending => return Poll::Pending,
        }
        Poll::Ready(Ok(buf.len()))
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
            let _version = u16::from_be_bytes([hdr[1], hdr[2]]);
            let length = u16::from_be_bytes([hdr[3], hdr[4]]) as usize;

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
                        // Post-handshake message (e.g., NewSessionTicket) — ignore
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
    /// ```ignore
    /// let mut tls_stream = AsyncTlsServerStream::new(tcp_stream);
    /// // ... read plaintext: EHLO, STARTTLS ...
    /// // ... send "220 Ready to start TLS" ...
    /// tls_stream.handshake(&cert).await?;
    /// // ... continue reading encrypted SMTP commands ...
    /// ```
    pub async fn handshake(&mut self, cert_and_key: &CertificateAndKey) -> Result<()> {
        server_handshake_impl(&mut self.stream, cert_and_key)
            .await
            .map(|(write_cipher, read_cipher, cipher_suite, alpn)| {
                self.write_cipher = write_cipher;
                self.read_cipher = read_cipher;
                self.handshake_done = true;
                self.cipher_suite = cipher_suite;
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
        let ciphertext = self.write_cipher.encrypt(23, buf);
        let record = crate::record::TlsRecord {
            content_type: 23,
            version: 0x0303,
            fragment: ciphertext,
        };
        let bytes = record.to_bytes();

        // Write all bytes, then flush — inline poll loop
        let mut pos = 0;
        while pos < bytes.len() {
            match Pin::new(&mut self.stream).poll_write(cx, &bytes[pos..]) {
                Poll::Ready(Ok(n)) => {
                    if n == 0 {
                        return Poll::Ready(Err(std::io::Error::new(
                            std::io::ErrorKind::WriteZero,
                            "failed to write whole buffer",
                        )));
                    }
                    pos += n;
                }
                Poll::Ready(Err(e)) => return Poll::Ready(Err(e)),
                Poll::Pending => return Poll::Pending,
            }
        }
        // Flush
        match Pin::new(&mut self.stream).poll_flush(cx) {
            Poll::Ready(Ok(())) => {}
            Poll::Ready(Err(e)) => return Poll::Ready(Err(e)),
            Poll::Pending => return Poll::Pending,
        }
        Poll::Ready(Ok(buf.len()))
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
            let _version = u16::from_be_bytes([hdr[1], hdr[2]]);
            let length = u16::from_be_bytes([hdr[3], hdr[4]]) as usize;

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
) -> Result<()> {
    loop {
        let mut hdr = [0u8; 5];
        stream.read_exact(&mut hdr).await?;
        let len = u16::from_be_bytes([hdr[3], hdr[4]]) as usize;
        let mut fragment = vec![0u8; len];
        stream.read_exact(&mut fragment).await?;
        let (inner_type, plaintext) = read_cipher.decrypt(&fragment)?;
        let hs_type = if !plaintext.is_empty() {
            plaintext[0]
        } else {
            inner_type
        };

        match hs_type {
            8 => {
                // EncryptedExtensions
                transcript.extend_from_slice(&plaintext);
            }
            11 => {
                // Certificate
                transcript.extend_from_slice(&plaintext);
                if plaintext.len() >= 8 {
                    let cert_list_len =
                        u32::from_be_bytes([0, plaintext[5], plaintext[6], plaintext[7]]) as usize;
                    let cert_list_start = 8;
                    if cert_list_start + cert_list_len <= plaintext.len() {
                        let mut cert_pos = cert_list_start;
                        let cert_list_end = cert_list_start + cert_list_len;
                        let mut certs = Vec::new();
                        while cert_pos + 5 < cert_list_end {
                            let cert_data_len = u32::from_be_bytes([
                                0,
                                plaintext[cert_pos],
                                plaintext[cert_pos + 1],
                                plaintext[cert_pos + 2],
                            ]) as usize;
                            cert_pos += 3;
                            if cert_pos + cert_data_len + 2 > cert_list_end {
                                break;
                            }
                            let cert_der = &plaintext[cert_pos..cert_pos + cert_data_len];
                            cert_pos += cert_data_len;
                            let ext_len =
                                u16::from_be_bytes([plaintext[cert_pos], plaintext[cert_pos + 1]])
                                    as usize;
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
                        if !leaf.is_valid_now() {
                            return Err(TlsError::Certificate(
                                "Server certificate is expired".into(),
                            ));
                        }
                        if !leaf.matches_hostname(server_name) {
                            if certs.len() == 1 {
                                // Self-signed — accept for testing
                            } else {
                                return Err(TlsError::Certificate(format!(
                                    "Certificate does not match hostname {}",
                                    server_name,
                                )));
                            }
                        }
                        if certs.len() >= 2 {
                            let issuer = &certs[1];
                            if let Err(e) = leaf.verify_signature(issuer) {
                                return Err(TlsError::Certificate(format!(
                                    "Certificate signature verification failed: {}",
                                    e
                                )));
                            }
                        }
                    }
                }
            }
            15 => {
                // CertificateVerify
                transcript.extend_from_slice(&plaintext);
            }
            20 => {
                // Finished — verify
                let full_transcript_hash = hash.hash(transcript);
                let server_hs_secret =
                    ks.server_handshake_traffic_secret(handshake_transcript_hash);
                let finished_key =
                    hash.expand_label(&server_hs_secret, "finished", &[], hash.len());
                if plaintext.len() < 4 + hash.len() {
                    return Err(TlsError::Protocol("Finished message too short".into()));
                }
                let verify_data = &plaintext[4..];
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
                transcript.extend_from_slice(&plaintext);
                return Ok(());
            }
            _ => {}
        }
    }
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
    cert_der: &[u8],
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
    let cert_msg = build_certificate_message(cert_der);
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
        let len = u16::from_be_bytes([hdr[3], hdr[4]]) as usize;
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
    let len = u16::from_be_bytes([hdr[3], hdr[4]]) as usize;
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

    // Negotiate ALPN: prefer "h2" for HTTP/2, fallback to "http/1.1"
    let alpn_protocol = if !ch.alpn_protocols.is_empty() {
        if ch.alpn_protocols.iter().any(|p| p == b"h2") {
            Some(b"h2".to_vec())
        } else if ch.alpn_protocols.iter().any(|p| p == b"http/1.1") {
            Some(b"http/1.1".to_vec())
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

    // 3. Derive handshake keys — offload ECDH to blocking pool
    let key_pair = std::sync::Arc::new(key_pair);
    let client_key_share = client_key_share.clone();
    let shared_secret = edgerun_rt::spawn_blocking(move || key_pair.exchange(&client_key_share))
        .await
        .map_err(|_| TlsError::HandshakeFailure("blocking pool shutdown".into()))
        .and_then(|r| r.map_err(TlsError::HandshakeFailure))?;
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
        &cert_and_key.cert_der,
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
    edgerun_crypto::getrandom(&mut buf).expect("getrandom failed");
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

// ---------------------------------------------------------------------------
// AsyncRead / AsyncWrite impls for AsyncTlsStream
// ---------------------------------------------------------------------------

impl<S: AsyncRead + AsyncWrite + Unpin> AsyncRead for AsyncTlsStream<S> {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<std::io::Result<usize>> {
        self.get_mut().poll_read(cx, buf)
    }
}

impl<S: AsyncRead + AsyncWrite + Unpin> AsyncWrite for AsyncTlsStream<S> {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        self.get_mut().poll_write(cx, buf)
    }
    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        self.get_mut().poll_flush(cx)
    }
    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        self.get_mut().poll_shutdown(cx)
    }
}

impl<S: AsyncRead + AsyncWrite + Unpin> AsyncRead for AsyncTlsServerStream<S> {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<std::io::Result<usize>> {
        self.get_mut().poll_read(cx, buf)
    }
}

impl<S: AsyncRead + AsyncWrite + Unpin> AsyncWrite for AsyncTlsServerStream<S> {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        self.get_mut().poll_write(cx, buf)
    }
    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        self.get_mut().poll_flush(cx)
    }
    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        self.get_mut().poll_shutdown(cx)
    }
}
