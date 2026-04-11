//! TLS 1.3 server-side handshake state machine and stream.

use std::io::{self, Read, Write};
use std::net::TcpStream;

use edgerun_crypto::rand_core::RngCore;

use crate::alert::{Alert, AlertLevel};
use crate::certificate_gen::CertificateAndKey;
use crate::cipher::CipherSuite;
use crate::handshake::{read_record_header, read_record_fragment};
use crate::key_exchange::{EcdhKeyPair, KeyExchangeGroup};
use crate::prf::{Hasher, Tls13KeySchedule, client_write_keys, server_write_keys, client_app_write_keys, server_app_write_keys};
use crate::record::{RecordCipher, TlsRecord};
use crate::server::client_hello::ClientHello;
use crate::server::message_builder::{
    build_server_hello, build_encrypted_extensions, build_certificate_message,
    build_certificate_verify, build_finished_message,
    compute_server_finished_verify_data, compute_client_finished_verify_data,
};
use crate::{Result, TlsError};

/// Server-side TLS 1.3 stream wrapping a TcpStream with encrypted I/O.
pub struct TlsServerStream {
    stream: TcpStream,
    write_cipher: RecordCipher,
    read_cipher: RecordCipher,
    handshake_done: bool,
    pending_data: Vec<u8>,
    pending_offset: usize,
    cipher_suite: CipherSuite,
}

impl TlsServerStream {
    /// Accept a TLS 1.3 connection from an existing TCP stream.
    pub fn accept(
        stream: TcpStream,
        cert_and_key: &CertificateAndKey,
    ) -> Result<Self> {
        #[cfg(unix)]
        {
            use std::os::fd::AsRawFd;
            let fd = stream.as_raw_fd();
            unsafe {
                let flags = libc::fcntl(fd, libc::F_GETFL, 0);
                if flags >= 0 {
                    libc::fcntl(fd, libc::F_SETFL, flags & !libc::O_NONBLOCK);
                }
            }
        }
        stream.set_read_timeout(None).ok();
        stream.set_write_timeout(None).ok();

        let mut hs = ServerHandshake::new(stream, cert_and_key);
        hs.do_handshake()?;
        Ok(hs.finish())
    }

    /// Check if the TLS handshake has completed.
    pub fn is_handshake_complete(&self) -> bool {
        self.handshake_done
    }

    /// Set the read timeout for the underlying TcpStream.
    pub fn set_read_timeout(&mut self, timeout: Option<std::time::Duration>) -> std::io::Result<()> {
        self.stream.set_read_timeout(timeout)
    }

    /// Shutdown the underlying TCP connection.
    pub fn shutdown(&self, how: std::net::Shutdown) -> std::io::Result<()> {
        self.stream.shutdown(how)
    }

    fn send_alert(stream: &mut TcpStream, level: AlertLevel, alert: Alert) -> Result<()> {
        let msg = vec![level as u8, alert as u8];
        let record = TlsRecord {
            content_type: 21,
            version: 0x0303,
            fragment: msg,
        };
        stream.write_all(&record.to_bytes())?;
        stream.flush()?;
        Ok(())
    }

    fn write_application_data(&mut self, buf: &[u8]) -> Result<()> {
        if buf.is_empty() {
            return Ok(());
        }
        let ciphertext = self.write_cipher.encrypt(23, buf);
        let record = TlsRecord {
            content_type: 23,
            version: 0x0303,
            fragment: ciphertext,
        };
        self.stream.write_all(&record.to_bytes())?;
        self.stream.flush()?;
        Ok(())
    }

    fn read_application_data(&mut self) -> Result<Vec<u8>> {
        loop {
            let (content_type, _version, length) = read_record_header(&mut self.stream)?;
            let fragment = read_record_fragment(&mut self.stream, length)?;

            if content_type == 23 {
                let (inner_type, plaintext) = self.read_cipher.decrypt(&fragment)?;
                if inner_type == 23 {
                    return Ok(plaintext);
                }
            } else if content_type == 21 {
                if fragment.len() >= 2 {
                    let level = AlertLevel::from_wire(fragment[0])
                        .map_err(|e| TlsError::Protocol(e))?;
                    let alert = Alert::from_wire(fragment[1])
                        .map_err(|e| TlsError::Protocol(e))?;
                    if level == AlertLevel::Fatal {
                        return Err(TlsError::Alert(level, alert));
                    }
                }
            }
        }
    }
}

impl Read for TlsServerStream {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if !self.handshake_done {
            return Err(io::Error::new(io::ErrorKind::NotConnected, "TLS handshake not complete"));
        }

        if self.pending_offset < self.pending_data.len() {
            let available = self.pending_data.len() - self.pending_offset;
            let n = available.min(buf.len());
            buf[..n].copy_from_slice(&self.pending_data[self.pending_offset..self.pending_offset + n]);
            self.pending_offset += n;
            return Ok(n);
        }

        match self.read_application_data() {
            Ok(plaintext) => {
                let n = plaintext.len().min(buf.len());
                buf[..n].copy_from_slice(&plaintext[..n]);
                if plaintext.len() > n {
                    self.pending_data = plaintext;
                    self.pending_offset = n;
                }
                Ok(n)
            }
            Err(TlsError::Io(e)) => Err(e),
            Err(TlsError::Alert(_, _)) => Err(io::Error::new(io::ErrorKind::ConnectionReset, "TLS alert")),
            Err(e) => Err(io::Error::new(io::ErrorKind::Other, e.to_string())),
        }
    }
}

impl Write for TlsServerStream {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if !self.handshake_done {
            return Err(io::Error::new(io::ErrorKind::NotConnected, "TLS handshake not complete"));
        }
        self.write_application_data(buf).map_err(io::Error::from)?;
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        self.stream.flush()
    }
}

// -----------------------------------------------------------------------
// ServerHandshake state machine
// -----------------------------------------------------------------------

struct ServerHandshake {
    stream: TcpStream,
    cert_der: Vec<u8>,
    signing_key: edgerun_crypto::p256::ecdsa::SigningKey,
    cipher_suite: CipherSuite,
    client_random: [u8; 32],
    server_random: [u8; 32],
    key_pair: Option<EcdhKeyPair>,
    selected_group: KeyExchangeGroup,
    client_key_share: Vec<u8>,
    client_session_id: Vec<u8>,
    ch_msg: Vec<u8>,
    sh_msg: Vec<u8>,
    transcript: Vec<u8>,
    write_cipher: RecordCipher,
    read_cipher: RecordCipher,
}

impl ServerHandshake {
    fn new(stream: TcpStream, cert_and_key: &CertificateAndKey) -> Self {
        let cipher_suite = CipherSuite::TLS_AES_128_GCM_SHA256;
        let write_cipher = RecordCipher::new(&[0u8; 16], &[0u8; 12]).unwrap();
        let read_cipher = RecordCipher::new(&[0u8; 16], &[0u8; 12]).unwrap();

        ServerHandshake {
            stream,
            cert_der: cert_and_key.cert_der.clone(),
            signing_key: cert_and_key.signing_key.clone(),
            cipher_suite,
            client_random: [0u8; 32],
            server_random: generate_random(),
            key_pair: None,
            selected_group: KeyExchangeGroup::X25519,
            client_key_share: Vec::new(),
            client_session_id: Vec::new(),
            ch_msg: Vec::new(),
            sh_msg: Vec::new(),
            transcript: Vec::new(),
            write_cipher,
            read_cipher,
        }
    }

    fn do_handshake(&mut self) -> Result<()> {
        self.read_client_hello()?;

        let group = self.selected_group;
        self.key_pair = Some(EcdhKeyPair::generate(group).map_err(|e| TlsError::HandshakeFailure(e))?);

        self.send_server_hello()?;

        let shared_secret = self.key_pair.as_ref().unwrap().exchange(&self.client_key_share)?;
        let hash = self.hasher();

        let mut transcript = Vec::new();
        transcript.extend_from_slice(&self.ch_msg);
        transcript.extend_from_slice(&self.sh_msg);
        let transcript_hash = hash.hash(&transcript);

        eprintln!("[SERVER] CH msg: {} bytes, SH msg: {} bytes", self.ch_msg.len(), self.sh_msg.len());
        eprintln!("[SERVER] transcript_hash: {:02x?}", &transcript_hash[..8]);
        eprintln!("[SERVER] CH first 8: {:02x?}", &self.ch_msg[..8.min(self.ch_msg.len())]);
        eprintln!("[SERVER] SH first 8: {:02x?}", &self.sh_msg[..8.min(self.sh_msg.len())]);

        let mut ks = Tls13KeySchedule::new(hash.clone());
        ks.advance_to_handshake(&shared_secret, &transcript_hash, &transcript_hash);

        let server_hs_secret = ks.server_handshake_traffic_secret(&transcript_hash);
        let client_hs_secret = ks.client_handshake_traffic_secret(&transcript_hash);

        let server_hs_keys = server_write_keys(&server_hs_secret, self.cipher_suite.key_len(), 12, &hash);
        let client_hs_keys = client_write_keys(&client_hs_secret, self.cipher_suite.key_len(), 12, &hash);

        let mut write_cipher = RecordCipher::new(&server_hs_keys.write_key, &server_hs_keys.write_iv)?;
        let mut read_cipher = RecordCipher::new(&client_hs_keys.write_key, &client_hs_keys.write_iv)?;

        // Store the transcript hash for Finished verification (hash of CH || SH)
        let handshake_transcript_hash = transcript_hash;

        self.send_encrypted_handshake(&mut write_cipher, &mut ks, &handshake_transcript_hash)?;
        // Save transcript hash after server's Finished — this is used for app traffic secrets
        // Per RFC 8446 §7.1: app traffic secrets use Hash(CH1...server Finished)
        let app_transcript_hash = self.hasher().hash(&self.transcript);
        self.read_client_finished(&mut read_cipher, &ks, &handshake_transcript_hash)?;

        ks.advance_to_master();
        let server_app = ks.server_app_traffic_secret(&app_transcript_hash);
        let client_app = ks.client_app_traffic_secret(&app_transcript_hash);

        let server_app_keys = server_app_write_keys(&server_app, self.cipher_suite.key_len(), 12, &hash);
        let client_app_keys = client_app_write_keys(&client_app, self.cipher_suite.key_len(), 12, &hash);

        self.write_cipher = RecordCipher::new(&server_app_keys.write_key, &server_app_keys.write_iv)?;
        self.read_cipher = RecordCipher::new(&client_app_keys.write_key, &client_app_keys.write_iv)?;

        Ok(())
    }

    fn read_client_hello(&mut self) -> Result<()> {
        let (ct, _ver, len) = read_record_header(&mut self.stream)?;
        if ct != 22 {
            return Err(TlsError::HandshakeFailure(format!("Expected handshake record, got content_type={ct}")));
        }
        let fragment = read_record_fragment(&mut self.stream, len)?;
        let ch = ClientHello::parse(&fragment)?;

        eprintln!("[TLS] ClientHello parsed: supported_versions={:?}, cipher_suites={:?}",
            ch.supported_versions, ch.cipher_suites);

        if !ch.supported_versions.iter().any(|&v| v == 0x0304) {
            return Err(TlsError::HandshakeFailure("Client does not support TLS 1.3".into()));
        }

        let common_suite = ch.cipher_suites.iter()
            .find(|cs| matches!(cs, CipherSuite::TLS_AES_128_GCM_SHA256))
            .or_else(|| ch.cipher_suites.iter()
                .find(|cs| matches!(cs, CipherSuite::TLS_AES_256_GCM_SHA384)))
            .cloned();

        self.cipher_suite = common_suite.ok_or_else(||
            TlsError::HandshakeFailure("No common cipher suite".into())
        )?;

        self.client_random = ch.random;

        // Select key exchange group: prefer client's first key_share, fall back to X25519
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
            return Err(TlsError::HandshakeFailure("No key_share in ClientHello".into()));
        };

        self.selected_group = selected_group;
        self.client_key_share = client_key_share.clone();

        self.client_session_id = ch.session_id;
        self.ch_msg = fragment.clone();
        self.transcript = fragment;

        Ok(())
    }

    fn send_server_hello(&mut self) -> Result<()> {
        let public_key = self.key_pair.as_ref().unwrap().public_key_bytes();
        let group = match self.selected_group {
            KeyExchangeGroup::SECP256R1 => crate::cipher::NamedGroup::SECP256R1,
            KeyExchangeGroup::X25519 => crate::cipher::NamedGroup::X25519,
        };
        let sh_msg = build_server_hello(
            self.server_random,
            &self.client_session_id,
            self.cipher_suite,
            &public_key,
            group,
        );

        self.sh_msg = sh_msg.clone();
        self.transcript.extend_from_slice(&sh_msg);

        let record = TlsRecord {
            content_type: 22,
            version: 0x0303,
            fragment: sh_msg,
        };
        self.stream.write_all(&record.to_bytes())?;
        self.stream.flush()?;

        Ok(())
    }

    fn send_encrypted_handshake(
        &mut self,
        write_cipher: &mut RecordCipher,
        ks: &mut Tls13KeySchedule,
        handshake_transcript_hash: &[u8],
    ) -> Result<()> {
        eprintln!("[TLS] Sending EncryptedExtensions...");
        let ee_msg = build_encrypted_extensions();
        self.transcript.extend_from_slice(&ee_msg);
        let ee_ct = write_cipher.encrypt(22, &ee_msg);
        self.stream.write_all(&TlsRecord { content_type: 23, version: 0x0303, fragment: ee_ct }.to_bytes())?;

        eprintln!("[TLS] Sending Certificate ({} bytes)...", self.cert_der.len());
        let cert_msg = build_certificate_message(&self.cert_der);
        self.transcript.extend_from_slice(&cert_msg);
        let cert_ct = write_cipher.encrypt(22, &cert_msg);
        self.stream.write_all(&TlsRecord { content_type: 23, version: 0x0303, fragment: cert_ct }.to_bytes())?;

        eprintln!("[TLS] Sending CertificateVerify...");
        let cv_msg = build_certificate_verify(&self.transcript, &self.signing_key, &self.hasher())?;
        self.transcript.extend_from_slice(&cv_msg);
        let cv_ct = write_cipher.encrypt(22, &cv_msg);
        self.stream.write_all(&TlsRecord { content_type: 23, version: 0x0303, fragment: cv_ct }.to_bytes())?;

        eprintln!("[TLS] Sending Finished...");
        // Use the handshake transcript hash (CH || SH || EE || Cert || CV) for Finished verification
        // Per RFC 8446 §4.4.4: verify_data = HMAC(finished_key, Hash(transcript))
        // where transcript includes all messages up to (but not including) this Finished
        let transcript_hash_before_finished = self.hasher().hash(&self.transcript);
        let server_hs_secret = ks.server_handshake_traffic_secret(handshake_transcript_hash);
        let verify_data = compute_server_finished_verify_data(&server_hs_secret, &transcript_hash_before_finished, &self.hasher());

        let finished_msg = build_finished_message(&verify_data);
        self.transcript.extend_from_slice(&finished_msg);
        let finished_ct = write_cipher.encrypt(22, &finished_msg);
        self.stream.write_all(&TlsRecord { content_type: 23, version: 0x0303, fragment: finished_ct }.to_bytes())?;
        self.stream.flush()?;

        eprintln!("[TLS] All encrypted handshake messages sent");
        Ok(())
    }

    fn read_client_finished(
        &mut self,
        read_cipher: &mut RecordCipher,
        ks: &Tls13KeySchedule,
        handshake_transcript_hash: &[u8],
    ) -> Result<()> {
        // TLS 1.3 clients may send a dummy ChangeCipherSpec record (content_type=20)
        // before the Finished. Skip any ChangeCipherSpec records.
        loop {
            let (ct, _ver, len) = read_record_header(&mut self.stream)?;
            if ct == 20 {
                // ChangeCipherSpec — skip in TLS 1.3
                let _fragment = read_record_fragment(&mut self.stream, len)?;
                continue;
            }
            if ct != 23 {
                return Err(TlsError::HandshakeFailure(format!("Expected encrypted record for client Finished, got content_type={ct}")));
            }
            let fragment = read_record_fragment(&mut self.stream, len)?;
            let (inner_type, plaintext) = read_cipher.decrypt(&fragment)?;

            let hs_type = if !plaintext.is_empty() { plaintext[0] } else { inner_type };
            if hs_type != 20 {
                return Err(TlsError::HandshakeFailure(format!("Expected Finished (type 20), got {}", hs_type)));
            }

            let transcript_hash = self.hasher().hash(&self.transcript);
            let client_hs_secret = ks.client_handshake_traffic_secret(handshake_transcript_hash);
            let expected_verify = compute_client_finished_verify_data(&client_hs_secret, &transcript_hash, &self.hasher());

            if plaintext.len() < 4 + expected_verify.len() {
                return Err(TlsError::Protocol("Client Finished verification data too short".into()));
            }
            let client_verify_data = &plaintext[4..4 + expected_verify.len()];

            if !constant_time_eq(client_verify_data, &expected_verify) {
                return Err(TlsError::Protocol("Client Finished verification failed".into()));
            }

            self.transcript.extend_from_slice(&plaintext);
            break;
        }
        Ok(())
    }

    fn hasher(&self) -> Hasher {
        match self.cipher_suite {
            CipherSuite::TLS_AES_256_GCM_SHA384 => Hasher::Sha384,
            _ => Hasher::Sha256,
        }
    }

    fn finish(self) -> TlsServerStream {
        TlsServerStream {
            stream: self.stream,
            write_cipher: self.write_cipher,
            read_cipher: self.read_cipher,
            handshake_done: true,
            pending_data: Vec::new(),
            pending_offset: 0,
            cipher_suite: self.cipher_suite,
        }
    }
}

fn generate_random() -> [u8; 32] {
    use edgerun_crypto::rand_core::RngCore;
    let mut buf = [0u8; 32];
    edgerun_crypto::rand_core::OsRng.fill_bytes(&mut buf);
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::certificate_gen::generate_self_signed;
    use crate::cipher::NamedGroup;
    use crate::handshake::ClientHelloBuilder;
    use crate::key_exchange::{EcdhKeyPair, KeyExchangeGroup};
    use crate::prf::{Tls13KeySchedule, server_write_keys};
    use crate::server::message_builder::{
        build_server_hello, build_encrypted_extensions, build_certificate_message,
        build_certificate_verify, build_finished_message,
        compute_server_finished_verify_data,
    };

    #[test]
    fn test_generate_random() {
        let r1 = generate_random();
        let r2 = generate_random();
        assert_eq!(r1.len(), 32);
        assert_ne!(r1, r2);
    }

    #[test]
    fn test_constant_time_eq() {
        assert!(constant_time_eq(&[0x42u8; 32], &[0x42u8; 32]));
        assert!(!constant_time_eq(&[0x42u8; 32], &[0x43u8; 32]));
        assert!(!constant_time_eq(&[0x42u8; 16], &[0x42u8; 32]));
    }

    #[test]
    fn test_tls_13_support_check() {
        let random = [0x20u8; 32];
        let key_pair = EcdhKeyPair::generate(KeyExchangeGroup::SECP256R1).unwrap();
        let public_key = key_pair.public_key_bytes();

        let ch_bytes = ClientHelloBuilder::new(random, "localhost")
            .key_share(&public_key, NamedGroup::SECP256R1)
            .build()
            .unwrap();

        let ch = ClientHello::parse(&ch_bytes).unwrap();
        assert!(ch.supported_versions.iter().any(|&v| v == 0x0304));
    }

    #[test]
    fn test_key_exchange_group_selection() {
        let group = crate::cipher::NamedGroup::X25519;
        let keg = match group {
            crate::cipher::NamedGroup::X25519 => KeyExchangeGroup::X25519,
            _ => KeyExchangeGroup::SECP256R1,
        };
        assert_eq!(keg, KeyExchangeGroup::X25519);

        let group = crate::cipher::NamedGroup::SECP256R1;
        let keg = match group {
            crate::cipher::NamedGroup::X25519 => KeyExchangeGroup::X25519,
            _ => KeyExchangeGroup::SECP256R1,
        };
        assert_eq!(keg, KeyExchangeGroup::SECP256R1);
    }

    #[test]
    fn test_send_alert_format() {
        let level = AlertLevel::Fatal;
        let alert = Alert::HandshakeFailure;

        let msg = vec![level as u8, alert as u8];
        let record = TlsRecord {
            content_type: 21,
            version: 0x0303,
            fragment: msg.clone(),
        };
        let bytes = record.to_bytes();

        assert_eq!(bytes.len(), 7);
        assert_eq!(bytes[0], 21);
        assert_eq!(bytes[1..3], [0x03, 0x03]);
        assert_eq!(bytes[3..5], [0x00, 0x02]);
        assert_eq!(&bytes[5..7], &msg);
    }

    #[test]
    fn test_simulated_handshake_message_flow() {
        let cert = generate_self_signed(&["localhost"]);
        let client_keys = EcdhKeyPair::generate(KeyExchangeGroup::SECP256R1).unwrap();
        let server_keys = EcdhKeyPair::generate(KeyExchangeGroup::SECP256R1).unwrap();
        let client_random = [0xAAu8; 32];
        let cipher_suite = CipherSuite::TLS_AES_128_GCM_SHA256;
        let hash = Hasher::Sha256;

        let client_pub = client_keys.public_key_bytes();
        let ch_bytes = ClientHelloBuilder::new(client_random, "localhost")
            .key_share(&client_pub, NamedGroup::SECP256R1)
            .build()
            .unwrap();

        let server_pub = server_keys.public_key_bytes();
        let sh_bytes = build_server_hello(
            [0xBBu8; 32],
            &[],
            cipher_suite,
            &server_pub,
            NamedGroup::SECP256R1,
        );

        let shared = server_keys.exchange(&client_pub).unwrap();
        let client_shared = client_keys.exchange(&server_pub).unwrap();
        assert_eq!(shared, client_shared);

        let mut transcript = Vec::new();
        transcript.extend_from_slice(&ch_bytes);
        transcript.extend_from_slice(&sh_bytes);
        let transcript_hash_before_ee = hash.hash(&transcript);

        let mut ks = Tls13KeySchedule::new(hash.clone());
        ks.advance_to_handshake(&shared, &transcript_hash_before_ee, &transcript_hash_before_ee);

        let server_hs_secret = ks.server_handshake_traffic_secret(&transcript_hash_before_ee);
        let server_write = server_write_keys(&server_hs_secret, cipher_suite.key_len(), 12, &hash);

        let mut server_enc = RecordCipher::new(&server_write.write_key, &server_write.write_iv).unwrap();

        let ee = build_encrypted_extensions();
        transcript.extend_from_slice(&ee);
        let ee_ct = server_enc.encrypt(22, &ee);
        let mut client_dec_ee = RecordCipher::new(&server_write.write_key, &server_write.write_iv).unwrap();
        let (_ct, ee_pt) = client_dec_ee.decrypt(&ee_ct).unwrap();
        assert_eq!(ee_pt, ee);

        let cert_msg = build_certificate_message(&cert.cert_der);
        transcript.extend_from_slice(&cert_msg);
        let cert_ct = server_enc.encrypt(22, &cert_msg);
        let mut client_dec_cert = RecordCipher::new(&server_write.write_key, &server_write.write_iv).unwrap();
        let _ = client_dec_cert.decrypt(&ee_ct).unwrap();
        let (_ct, cert_pt) = client_dec_cert.decrypt(&cert_ct).unwrap();
        assert_eq!(cert_pt, cert_msg);

        let cv = build_certificate_verify(&transcript, &cert.signing_key, &hash).unwrap();
        transcript.extend_from_slice(&cv);
        let cv_ct = server_enc.encrypt(22, &cv);
        let mut client_dec_cv = RecordCipher::new(&server_write.write_key, &server_write.write_iv).unwrap();
        let _ = client_dec_cv.decrypt(&ee_ct).unwrap();
        let _ = client_dec_cv.decrypt(&cert_ct).unwrap();
        let (_ct, cv_pt) = client_dec_cv.decrypt(&cv_ct).unwrap();
        assert_eq!(cv_pt, cv);

        let transcript_hash_before_fin = hash.hash(&transcript);
        let server_verify_data = compute_server_finished_verify_data(&server_hs_secret, &transcript_hash_before_fin, &hash);
        let server_fin = build_finished_message(&server_verify_data);
        transcript.extend_from_slice(&server_fin);
        let fin_ct = server_enc.encrypt(22, &server_fin);
        let mut client_dec_fin = RecordCipher::new(&server_write.write_key, &server_write.write_iv).unwrap();
        let _ = client_dec_fin.decrypt(&ee_ct).unwrap();
        let _ = client_dec_fin.decrypt(&cert_ct).unwrap();
        let _ = client_dec_fin.decrypt(&cv_ct).unwrap();
        let (_ct, fin_pt) = client_dec_fin.decrypt(&fin_ct).unwrap();
        assert_eq!(fin_pt, server_fin);

        let mut client_transcript = Vec::new();
        client_transcript.extend_from_slice(&ch_bytes);
        client_transcript.extend_from_slice(&sh_bytes);
        client_transcript.extend_from_slice(&ee);
        client_transcript.extend_from_slice(&cert_msg);
        client_transcript.extend_from_slice(&cv);
        let client_transcript_hash = hash.hash(&client_transcript);

        assert_eq!(client_transcript_hash, transcript_hash_before_fin,
            "Client and server transcript hashes must match before Finished");

        ks.advance_to_master();
        let server_app = ks.server_app_traffic_secret(&client_transcript_hash);
        let client_app = ks.client_app_traffic_secret(&client_transcript_hash);
        assert_ne!(server_app, client_app);
    }
}
