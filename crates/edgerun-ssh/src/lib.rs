#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

pub use edgerun_protocols::ssh::*;

#[cfg(feature = "std")]
pub mod host {
    use super::*;
    use edgerun_crypto::aes::Aes256;
    use edgerun_crypto::x25519::{PublicKey as X25519PublicKey, StaticSecret as X25519Secret};
    use edgerun_crypto::{fill_random, sha256};
    use edgerun_encoding::base64::standard_decode;
    use edgerun_protocols::sign::{Ed25519MessageSigner, MessageSigner};
    use std::io::{Read, Write};
    use std::net::{TcpStream, ToSocketAddrs};
    use std::path::Path;
    use std::time::Duration;

    pub fn probe_identification(
        target: &SshTarget,
        timeout: Duration,
    ) -> Result<SshIdentification, SshError> {
        let mut stream = connect(target, timeout)?;
        exchange_identification(&mut stream)
    }

    pub fn probe_server(target: &SshTarget, timeout: Duration) -> Result<SshProbe, SshError> {
        let mut stream = connect(target, timeout)?;
        let identification = exchange_identification(&mut stream)?;
        let cookie = weak_probe_cookie();
        let client_kex = default_client_kexinit(cookie);
        let client_packet = encode_binary_packet(&encode_kexinit(&client_kex));
        stream
            .write_all(&client_packet)
            .map_err(|err| SshError::Io(err.to_string()))?;
        let server_packet = read_binary_packet(&mut stream)?;
        let payload = parse_binary_packet(&server_packet)?;
        let server_kex = parse_kexinit(&payload)?;
        Ok(SshProbe {
            identification,
            server_kex,
        })
    }

    pub fn exec_with_ed25519_key_file(
        target: &SshTarget,
        key_path: &Path,
        command: &str,
        timeout: Duration,
    ) -> Result<SshExecResult, SshError> {
        let key = load_openssh_ed25519_key(key_path)?;
        exec_with_ed25519_key(target, &key, command, &[], timeout)
    }

    pub fn exec_with_ed25519_key_file_and_input(
        target: &SshTarget,
        key_path: &Path,
        command: &str,
        stdin: &[u8],
        timeout: Duration,
    ) -> Result<SshExecResult, SshError> {
        let key = load_openssh_ed25519_key(key_path)?;
        exec_with_ed25519_key(target, &key, command, stdin, timeout)
    }

    pub fn exec_with_ed25519_key(
        target: &SshTarget,
        key: &Ed25519MessageSigner,
        command: &str,
        stdin: &[u8],
        timeout: Duration,
    ) -> Result<SshExecResult, SshError> {
        let mut stream = connect(target, timeout)?;
        let identification = exchange_identification(&mut stream)?;

        let mut cookie = [0u8; 16];
        fill_random(&mut cookie).map_err(|err| SshError::Io(format!("{err:?}")))?;
        let client_kex = default_client_kexinit(cookie);
        let client_kex_payload = encode_kexinit(&client_kex);
        stream
            .write_all(&encode_binary_packet(&client_kex_payload))
            .map_err(|err| SshError::Io(err.to_string()))?;

        let server_kex_packet = read_binary_packet(&mut stream)?;
        let server_kex_payload = parse_binary_packet(&server_kex_packet)?;
        let server_kex = parse_kexinit(&server_kex_payload)?;
        let selection = choose_algorithms(&client_kex, &server_kex)?;
        if selection.kex != "curve25519-sha256" && selection.kex != "curve25519-sha256@libssh.org"
            || selection.host_key != "ssh-ed25519"
            || selection.encryption_client_to_server != "aes256-ctr"
            || selection.encryption_server_to_client != "aes256-ctr"
            || selection.mac_client_to_server != "hmac-sha2-256"
            || selection.mac_server_to_client != "hmac-sha2-256"
        {
            return Err(SshError::UnsupportedAlgorithm);
        }

        let mut secret_bytes = [0u8; 32];
        fill_random(&mut secret_bytes).map_err(|err| SshError::Io(format!("{err:?}")))?;
        let secret = X25519Secret::from(secret_bytes);
        let public: X25519PublicKey = (&secret).into();
        let q_c = public.to_bytes();
        let mut ecdh_init = Vec::new();
        ecdh_init.push(SSH_MSG_KEX_ECDH_INIT);
        write_string(&mut ecdh_init, &q_c);
        stream
            .write_all(&encode_binary_packet(&ecdh_init))
            .map_err(|err| SshError::Io(err.to_string()))?;

        let reply_packet = read_binary_packet(&mut stream)?;
        let reply_payload = parse_binary_packet(&reply_packet)?;
        let mut cursor = 0;
        if reply_payload.first().copied() != Some(SSH_MSG_KEX_ECDH_REPLY) {
            return Err(SshError::UnsupportedMessage(
                reply_payload.first().copied().unwrap_or(0),
            ));
        }
        cursor += 1;
        let host_key_blob = read_string(&reply_payload, &mut cursor)?.to_vec();
        let q_s = read_string(&reply_payload, &mut cursor)?.to_vec();
        let signature_blob = read_string(&reply_payload, &mut cursor)?.to_vec();
        if q_s.len() != 32 {
            return Err(SshError::BadPacket);
        }
        let mut q_s_bytes = [0u8; 32];
        q_s_bytes.copy_from_slice(&q_s);
        let shared = secret.diffie_hellman(&X25519PublicKey::from(q_s_bytes));
        let k = shared.to_bytes();

        let host_public_key = parse_ssh_ed25519_public_key_blob(&host_key_blob)?;
        let exchange_hash = exchange_hash(
            &identification.client,
            &identification.server,
            &client_kex_payload,
            &server_kex_payload,
            &host_key_blob,
            &q_c,
            &q_s,
            &k,
        );
        verify_ssh_ed25519_signature(&host_public_key, &signature_blob, &exchange_hash)?;
        let session_id = exchange_hash;
        let k_mpint = mpint_bytes(&k);
        let keys = TransportKeys::derive(&k_mpint, &exchange_hash, &session_id);

        stream
            .write_all(&encode_binary_packet(&[SSH_MSG_NEWKEYS]))
            .map_err(|err| SshError::Io(err.to_string()))?;
        let server_newkeys = parse_binary_packet(&read_binary_packet(&mut stream)?)?;
        if server_newkeys.first().copied() != Some(SSH_MSG_NEWKEYS) {
            return Err(SshError::UnsupportedMessage(
                server_newkeys.first().copied().unwrap_or(0),
            ));
        }

        let mut transport = EncryptedTransport::new(stream, keys);
        transport.send_plain_sequence_start(3);
        request_userauth_service(&mut transport)?;
        authenticate_publickey(&mut transport, target, key, &session_id)?;
        exec_authenticated(&mut transport, command, stdin)
    }

    fn connect(target: &SshTarget, timeout: Duration) -> Result<TcpStream, SshError> {
        let addr = (target.host.as_str(), target.port)
            .to_socket_addrs()
            .map_err(|err| SshError::Io(err.to_string()))?
            .next()
            .ok_or(SshError::BadTarget)?;
        let mut stream = TcpStream::connect_timeout(&addr, timeout)
            .map_err(|err| SshError::Io(err.to_string()))?;
        stream
            .set_read_timeout(Some(timeout))
            .map_err(|err| SshError::Io(err.to_string()))?;
        stream
            .set_write_timeout(Some(timeout))
            .map_err(|err| SshError::Io(err.to_string()))?;
        Ok(stream)
    }

    fn exchange_identification(stream: &mut TcpStream) -> Result<SshIdentification, SshError> {
        let mut line = Vec::new();
        let mut byte = [0u8; 1];
        while line.len() < 255 {
            let read = stream
                .read(&mut byte)
                .map_err(|err| SshError::Io(err.to_string()))?;
            if read == 0 {
                break;
            }
            line.push(byte[0]);
            if byte[0] == b'\n' {
                break;
            }
        }
        let server = String::from_utf8(line)
            .map_err(|_| SshError::BadIdentification)?
            .trim_end_matches(['\r', '\n'])
            .to_string();
        if !server.starts_with("SSH-") {
            return Err(SshError::BadIdentification);
        }
        let client = CLIENT_IDENTIFICATION.to_string();
        let mut client_line = client.clone();
        client_line.push_str("\r\n");
        stream
            .write_all(client_line.as_bytes())
            .map_err(|err| SshError::Io(err.to_string()))?;
        Ok(SshIdentification { server, client })
    }

    fn read_binary_packet(stream: &mut TcpStream) -> Result<Vec<u8>, SshError> {
        let mut header = [0u8; 5];
        stream
            .read_exact(&mut header)
            .map_err(|err| SshError::Io(err.to_string()))?;
        let packet_len = u32::from_be_bytes([header[0], header[1], header[2], header[3]]) as usize;
        if packet_len < 2 || packet_len > 256 * 1024 {
            return Err(SshError::BadPacket);
        }
        let mut packet = Vec::with_capacity(packet_len + 4);
        packet.extend_from_slice(&header);
        packet.resize(packet_len + 4, 0);
        stream
            .read_exact(&mut packet[5..])
            .map_err(|err| SshError::Io(err.to_string()))?;
        Ok(packet)
    }

    fn weak_probe_cookie() -> [u8; 16] {
        let mut cookie = [0u8; 16];
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        cookie[..8].copy_from_slice(&(now as u64).to_le_bytes());
        cookie[8..12].copy_from_slice(&std::process::id().to_le_bytes());
        cookie[12..].copy_from_slice(&((now >> 64) as u32).to_le_bytes());
        cookie
    }

    fn request_userauth_service(transport: &mut EncryptedTransport) -> Result<(), SshError> {
        let mut request = Vec::new();
        request.push(SSH_MSG_SERVICE_REQUEST);
        write_string(&mut request, b"ssh-userauth");
        transport.send(&request)?;
        loop {
            let payload = transport.recv()?;
            match payload.first().copied() {
                Some(SSH_MSG_SERVICE_ACCEPT) => return Ok(()),
                Some(SSH_MSG_EXT_INFO) => {}
                Some(SSH_MSG_IGNORE | SSH_MSG_DEBUG) => {}
                Some(SSH_MSG_GLOBAL_REQUEST) => respond_global_request(transport, &payload)?,
                Some(message) => return Err(SshError::UnsupportedMessage(message)),
                None => return Err(SshError::BadPacket),
            }
        }
    }

    fn authenticate_publickey(
        transport: &mut EncryptedTransport,
        target: &SshTarget,
        key: &Ed25519MessageSigner,
        session_id: &[u8; 32],
    ) -> Result<(), SshError> {
        let public_key: [u8; 32] = key
            .public_key_bytes()
            .as_slice()
            .try_into()
            .map_err(|_| SshError::BadKey)?;
        let public_blob = ssh_ed25519_public_key_blob(&public_key);
        let mut request = Vec::new();
        request.push(SSH_MSG_USERAUTH_REQUEST);
        write_string(&mut request, target.user.as_bytes());
        write_string(&mut request, b"ssh-connection");
        write_string(&mut request, b"publickey");
        write_bool(&mut request, true);
        write_string(&mut request, b"ssh-ed25519");
        write_string(&mut request, &public_blob);

        let mut signed = Vec::new();
        write_string(&mut signed, session_id);
        signed.extend_from_slice(&request);
        let signature = key
            .sign_message(&signed)
            .map_err(|_| SshError::BadSignature)?;
        let mut sig_blob = Vec::new();
        write_string(&mut sig_blob, b"ssh-ed25519");
        write_string(&mut sig_blob, &signature);
        write_string(&mut request, &sig_blob);
        transport.send(&request)?;

        loop {
            let payload = transport.recv()?;
            match payload.first().copied() {
                Some(SSH_MSG_USERAUTH_SUCCESS) => return Ok(()),
                Some(SSH_MSG_USERAUTH_FAILURE) => {
                    let mut cursor = 1;
                    let methods = read_string(&payload, &mut cursor)
                        .ok()
                        .and_then(|bytes| core::str::from_utf8(bytes).ok())
                        .unwrap_or("")
                        .to_string();
                    return Err(SshError::AuthenticationFailed(methods));
                }
                Some(SSH_MSG_EXT_INFO) => {}
                Some(SSH_MSG_IGNORE | SSH_MSG_DEBUG) => {}
                Some(SSH_MSG_GLOBAL_REQUEST) => respond_global_request(transport, &payload)?,
                Some(message) => return Err(SshError::UnsupportedMessage(message)),
                None => return Err(SshError::BadPacket),
            }
        }
    }

    fn exec_authenticated(
        transport: &mut EncryptedTransport,
        command: &str,
        stdin: &[u8],
    ) -> Result<SshExecResult, SshError> {
        let mut open = Vec::new();
        open.push(SSH_MSG_CHANNEL_OPEN);
        write_string(&mut open, b"session");
        write_u32(&mut open, 0);
        write_u32(&mut open, 2 * 1024 * 1024);
        write_u32(&mut open, 32768);
        transport.send(&open)?;

        let server_channel = loop {
            let payload = transport.recv()?;
            match payload.first().copied() {
                Some(SSH_MSG_CHANNEL_OPEN_CONFIRMATION) => {
                    let mut cursor = 1;
                    let recipient = read_u32(&payload, &mut cursor)?;
                    let sender = read_u32(&payload, &mut cursor)?;
                    if recipient != 0 {
                        return Err(SshError::BadPacket);
                    }
                    break sender;
                }
                Some(SSH_MSG_CHANNEL_OPEN_FAILURE) => return Err(SshError::BadPacket),
                Some(SSH_MSG_IGNORE | SSH_MSG_DEBUG) => {}
                Some(SSH_MSG_GLOBAL_REQUEST) => respond_global_request(transport, &payload)?,
                Some(message) => return Err(SshError::UnsupportedMessage(message)),
                None => return Err(SshError::BadPacket),
            }
        };

        let mut exec = Vec::new();
        exec.push(SSH_MSG_CHANNEL_REQUEST);
        write_u32(&mut exec, server_channel);
        write_string(&mut exec, b"exec");
        write_bool(&mut exec, true);
        write_string(&mut exec, command.as_bytes());
        transport.send(&exec)?;

        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let mut exit_status = None;
        let mut close_sent = false;
        let mut stdin_sent = false;

        loop {
            let payload = transport.recv()?;
            match payload.first().copied() {
                Some(SSH_MSG_CHANNEL_SUCCESS) => {
                    if !stdin_sent {
                        send_channel_stdin(transport, server_channel, stdin)?;
                        stdin_sent = true;
                    }
                }
                Some(SSH_MSG_CHANNEL_FAILURE) => return Err(SshError::BadPacket),
                Some(SSH_MSG_CHANNEL_DATA) => {
                    let mut cursor = 1;
                    let recipient = read_u32(&payload, &mut cursor)?;
                    if recipient != 0 {
                        return Err(SshError::BadPacket);
                    }
                    stdout.extend_from_slice(read_string(&payload, &mut cursor)?);
                }
                Some(SSH_MSG_CHANNEL_EXTENDED_DATA) => {
                    let mut cursor = 1;
                    let recipient = read_u32(&payload, &mut cursor)?;
                    let data_type = read_u32(&payload, &mut cursor)?;
                    if recipient != 0 || data_type != 1 {
                        return Err(SshError::BadPacket);
                    }
                    stderr.extend_from_slice(read_string(&payload, &mut cursor)?);
                }
                Some(SSH_MSG_CHANNEL_REQUEST) => {
                    let mut cursor = 1;
                    let recipient = read_u32(&payload, &mut cursor)?;
                    let request_type = read_string(&payload, &mut cursor)?;
                    let want_reply = read_bool(&payload, &mut cursor)?;
                    if recipient != 0 {
                        return Err(SshError::BadPacket);
                    }
                    if request_type == b"exit-status" {
                        exit_status = Some(read_u32(&payload, &mut cursor)?);
                    }
                    if want_reply {
                        let mut failure = Vec::new();
                        failure.push(SSH_MSG_CHANNEL_FAILURE);
                        write_u32(&mut failure, server_channel);
                        transport.send(&failure)?;
                    }
                }
                Some(SSH_MSG_CHANNEL_WINDOW_ADJUST) => {}
                Some(SSH_MSG_CHANNEL_EOF) => {
                    if !close_sent {
                        let mut close = Vec::new();
                        close.push(SSH_MSG_CHANNEL_CLOSE);
                        write_u32(&mut close, server_channel);
                        transport.send(&close)?;
                        close_sent = true;
                    }
                }
                Some(SSH_MSG_CHANNEL_CLOSE) => {
                    if !close_sent {
                        let mut close = Vec::new();
                        close.push(SSH_MSG_CHANNEL_CLOSE);
                        write_u32(&mut close, server_channel);
                        transport.send(&close)?;
                    }
                    return Ok(SshExecResult {
                        stdout,
                        stderr,
                        exit_status,
                    });
                }
                Some(SSH_MSG_IGNORE | SSH_MSG_DEBUG) => {}
                Some(SSH_MSG_GLOBAL_REQUEST) => respond_global_request(transport, &payload)?,
                Some(message) => return Err(SshError::UnsupportedMessage(message)),
                None => return Err(SshError::BadPacket),
            }
        }
    }

    fn send_channel_stdin(
        transport: &mut EncryptedTransport,
        server_channel: u32,
        stdin: &[u8],
    ) -> Result<(), SshError> {
        for chunk in stdin.chunks(32 * 1024) {
            let mut data = Vec::new();
            data.push(SSH_MSG_CHANNEL_DATA);
            write_u32(&mut data, server_channel);
            write_string(&mut data, chunk);
            transport.send(&data)?;
        }
        let mut eof = Vec::new();
        eof.push(SSH_MSG_CHANNEL_EOF);
        write_u32(&mut eof, server_channel);
        transport.send(&eof)
    }

    fn respond_global_request(
        transport: &mut EncryptedTransport,
        payload: &[u8],
    ) -> Result<(), SshError> {
        let mut cursor = 1;
        let _request_name = read_string(payload, &mut cursor)?;
        let want_reply = read_bool(payload, &mut cursor)?;
        if want_reply {
            transport.send(&[SSH_MSG_REQUEST_FAILURE])?;
        }
        Ok(())
    }

    fn exchange_hash(
        client_identification: &str,
        server_identification: &str,
        client_kex: &[u8],
        server_kex: &[u8],
        host_key: &[u8],
        q_c: &[u8],
        q_s: &[u8],
        k: &[u8],
    ) -> [u8; 32] {
        let mut bytes = Vec::new();
        write_string(&mut bytes, client_identification.as_bytes());
        write_string(&mut bytes, server_identification.as_bytes());
        write_string(&mut bytes, client_kex);
        write_string(&mut bytes, server_kex);
        write_string(&mut bytes, host_key);
        write_string(&mut bytes, q_c);
        write_string(&mut bytes, q_s);
        bytes.extend_from_slice(&mpint_bytes(k));
        sha256(&bytes)
    }

    fn load_openssh_ed25519_key(path: &Path) -> Result<Ed25519MessageSigner, SshError> {
        let pem = std::fs::read_to_string(path).map_err(|err| SshError::Io(err.to_string()))?;
        let mut body = String::new();
        for line in pem.lines() {
            if line.starts_with("-----") {
                continue;
            }
            body.push_str(line.trim());
        }
        let bytes = standard_decode(&body).map_err(|_| SshError::BadKey)?;
        parse_openssh_ed25519_private_key(&bytes)
    }

    struct TransportKeys {
        c2s_iv: [u8; 16],
        s2c_iv: [u8; 16],
        c2s_key: [u8; 32],
        s2c_key: [u8; 32],
        c2s_mac: [u8; 32],
        s2c_mac: [u8; 32],
    }

    impl TransportKeys {
        fn derive(k_mpint: &[u8], h: &[u8; 32], session_id: &[u8; 32]) -> Self {
            let mut c2s_iv = [0u8; 16];
            let mut s2c_iv = [0u8; 16];
            let mut c2s_key = [0u8; 32];
            let mut s2c_key = [0u8; 32];
            let mut c2s_mac = [0u8; 32];
            let mut s2c_mac = [0u8; 32];
            c2s_iv.copy_from_slice(&derive_key(k_mpint, h, session_id, b'A', 16));
            s2c_iv.copy_from_slice(&derive_key(k_mpint, h, session_id, b'B', 16));
            c2s_key.copy_from_slice(&derive_key(k_mpint, h, session_id, b'C', 32));
            s2c_key.copy_from_slice(&derive_key(k_mpint, h, session_id, b'D', 32));
            c2s_mac.copy_from_slice(&derive_key(k_mpint, h, session_id, b'E', 32));
            s2c_mac.copy_from_slice(&derive_key(k_mpint, h, session_id, b'F', 32));
            Self {
                c2s_iv,
                s2c_iv,
                c2s_key,
                s2c_key,
                c2s_mac,
                s2c_mac,
            }
        }
    }

    fn derive_key(
        k_mpint: &[u8],
        h: &[u8; 32],
        session_id: &[u8; 32],
        letter: u8,
        len: usize,
    ) -> Vec<u8> {
        let mut out = Vec::new();
        let mut first = Vec::new();
        first.extend_from_slice(k_mpint);
        first.extend_from_slice(h);
        first.push(letter);
        first.extend_from_slice(session_id);
        out.extend_from_slice(&sha256(&first));
        while out.len() < len {
            let mut next = Vec::new();
            next.extend_from_slice(k_mpint);
            next.extend_from_slice(h);
            next.extend_from_slice(&out);
            out.extend_from_slice(&sha256(&next));
        }
        out.truncate(len);
        out
    }

    struct EncryptedTransport {
        stream: TcpStream,
        c2s_cipher: AesCtr,
        s2c_cipher: AesCtr,
        c2s_mac: [u8; 32],
        s2c_mac: [u8; 32],
        send_seq: u32,
        recv_seq: u32,
        pushback: Option<Vec<u8>>,
    }

    impl EncryptedTransport {
        fn new(stream: TcpStream, keys: TransportKeys) -> Self {
            Self {
                stream,
                c2s_cipher: AesCtr::new(keys.c2s_key, keys.c2s_iv),
                s2c_cipher: AesCtr::new(keys.s2c_key, keys.s2c_iv),
                c2s_mac: keys.c2s_mac,
                s2c_mac: keys.s2c_mac,
                send_seq: 0,
                recv_seq: 0,
                pushback: None,
            }
        }

        fn send_plain_sequence_start(&mut self, next_seq: u32) {
            self.send_seq = next_seq;
            self.recv_seq = next_seq;
        }

        fn send(&mut self, payload: &[u8]) -> Result<(), SshError> {
            let mut packet = encode_binary_packet_random(payload, 16)?;
            let mac = packet_mac(self.send_seq, &self.c2s_mac, &packet);
            self.c2s_cipher.apply(&mut packet);
            self.stream
                .write_all(&packet)
                .map_err(|err| SshError::Io(err.to_string()))?;
            self.stream
                .write_all(&mac)
                .map_err(|err| SshError::Io(err.to_string()))?;
            self.send_seq = self.send_seq.wrapping_add(1);
            Ok(())
        }

        fn recv(&mut self) -> Result<Vec<u8>, SshError> {
            if let Some(payload) = self.pushback.take() {
                return Ok(payload);
            }
            let mut first_block = [0u8; 16];
            self.stream
                .read_exact(&mut first_block)
                .map_err(|err| SshError::Io(err.to_string()))?;
            let mut plaintext = first_block.to_vec();
            self.s2c_cipher.apply(&mut plaintext);
            let packet_len =
                u32::from_be_bytes([plaintext[0], plaintext[1], plaintext[2], plaintext[3]])
                    as usize;
            if packet_len < 12 || packet_len > 256 * 1024 {
                return Err(SshError::BadPacket);
            }
            let total_len = packet_len + 4;
            if total_len < 16 {
                return Err(SshError::BadPacket);
            }
            let remaining_len = total_len - 16;
            let mut encrypted_remaining = vec![0u8; remaining_len];
            self.stream
                .read_exact(&mut encrypted_remaining)
                .map_err(|err| SshError::Io(err.to_string()))?;
            self.s2c_cipher.apply(&mut encrypted_remaining);
            plaintext.extend_from_slice(&encrypted_remaining);
            let mut received_mac = [0u8; 32];
            self.stream
                .read_exact(&mut received_mac)
                .map_err(|err| SshError::Io(err.to_string()))?;
            let expected_mac = packet_mac(self.recv_seq, &self.s2c_mac, &plaintext);
            if !constant_time_eq(&received_mac, &expected_mac) {
                return Err(SshError::MacMismatch);
            }
            self.recv_seq = self.recv_seq.wrapping_add(1);
            parse_binary_packet(&plaintext)
        }
    }

    fn encode_binary_packet_random(payload: &[u8], block_size: usize) -> Result<Vec<u8>, SshError> {
        let mut padding_len = block_size - ((payload.len() + 5) % block_size);
        if padding_len < 4 {
            padding_len += block_size;
        }
        let packet_len = payload.len() + padding_len + 1;
        let mut packet = Vec::with_capacity(packet_len + 4);
        packet.extend_from_slice(&(packet_len as u32).to_be_bytes());
        packet.push(padding_len as u8);
        packet.extend_from_slice(payload);
        let start = packet.len();
        packet.resize(start + padding_len, 0);
        fill_random(&mut packet[start..]).map_err(|err| SshError::Io(format!("{err:?}")))?;
        Ok(packet)
    }

    fn packet_mac(seq: u32, key: &[u8], plaintext_packet: &[u8]) -> [u8; 32] {
        let mut input = Vec::with_capacity(4 + plaintext_packet.len());
        input.extend_from_slice(&seq.to_be_bytes());
        input.extend_from_slice(plaintext_packet);
        let mac = edgerun_crypto::hmac_sha256(key, &input);
        let mut out = [0u8; 32];
        out.copy_from_slice(&mac[..32]);
        out
    }

    fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
        if a.len() != b.len() {
            return false;
        }
        let mut diff = 0u8;
        for (left, right) in a.iter().zip(b.iter()) {
            diff |= left ^ right;
        }
        diff == 0
    }

    struct AesCtr {
        cipher: Aes256,
        counter: [u8; 16],
        block: [u8; 16],
        offset: usize,
    }

    impl AesCtr {
        fn new(key: [u8; 32], iv: [u8; 16]) -> Self {
            Self {
                cipher: Aes256::new(&key),
                counter: iv,
                block: [0u8; 16],
                offset: 16,
            }
        }

        fn apply(&mut self, bytes: &mut [u8]) {
            for byte in bytes {
                if self.offset == 16 {
                    self.block = self.cipher.encrypt_block(&self.counter);
                    increment_counter(&mut self.counter);
                    self.offset = 0;
                }
                *byte ^= self.block[self.offset];
                self.offset += 1;
            }
        }
    }

    fn increment_counter(counter: &mut [u8; 16]) {
        for byte in counter.iter_mut().rev() {
            let (next, carry) = byte.overflowing_add(1);
            *byte = next;
            if !carry {
                break;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_target() {
        assert_eq!(
            SshTarget::parse("root@example.com:2200").unwrap(),
            SshTarget {
                user: "root".to_string(),
                host: "example.com".to_string(),
                port: 2200
            }
        );
    }

    #[test]
    fn kexinit_roundtrips_through_packet() {
        let kex = default_client_kexinit([7; 16]);
        let payload = encode_kexinit(&kex);
        let packet = encode_binary_packet(&payload);
        let parsed_payload = parse_binary_packet(&packet).unwrap();
        assert_eq!(parsed_payload, payload);
        let parsed = parse_kexinit(&parsed_payload).unwrap();
        assert_eq!(parsed, kex);
    }
}
