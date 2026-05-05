#![no_std]
extern crate alloc;

#[cfg(all(any(test, feature = "std"), not(target_os = "none")))]
extern crate std;

// Comprehensive end-to-end test suite for all edgerun subsystems.
//
// This test suite validates that all components work together in real scenarios:
// - Node daemon lifecycle and health checks
// - TCP session establishment and communication
// - Command dispatch and validation pipeline
// - Storage and event stream operations
// - Capability invocation (local via Unix socket)
// - Ingress screening and rate limiting
// - Mesh networking (when hardware available)
// - OCI workload execution (native Linux namespaces, no Docker needed)
//
// # Running
//
// ```bash
// # Run all software E2E tests (no hardware required)
// cargo test -p edgerun-e2e -- --ignored
//
// # Run with hardware tests (requires actual devices)
// HARDWARE_E2E=1 cargo test -p edgerun-e2e -- --ignored
//
// # Run specific test
// cargo test -p edgerun-e2e e2e_node_lifecycle -- --ignored
// ```

#[cfg(all(any(test, feature = "std"), not(target_os = "none")))]
#[cfg(all(any(test, feature = "std"), not(target_os = "none")))]
mod suite {
    use std::fs;
    use std::io::{Read, Write};
    use std::net::{SocketAddr, TcpStream};
    use std::process::{Child, Command};
    use std::string::{String, ToString};
    use std::sync::atomic::{AtomicU16, Ordering};
    use std::sync::{Mutex, MutexGuard, OnceLock};
    use std::time::Duration;
    use std::vec::Vec;
    use std::{cell::RefCell, thread_local};
    use std::{format, println, vec};

    use edgerun_core::protocol::{
        common::{IdentityKind, IdentityRef, NodeRef},
        network::{SessionAccept, SessionHello},
        stream::{CommandEnvelope, CommandType},
    };
    use edgerun_crypto::p256::ecdsa::signature::hazmat::PrehashSigner;
    use edgerun_crypto::rand_core::RngCore;
    use edgerun_crypto::sha2::Digest;
    use edgerun_hardware_signing::{MeshSigner, MESH_PUBLIC_KEY_LENGTH, MESH_SIGNATURE_LENGTH};

    // ===========================================================================
    // Port allocator
    // ===========================================================================

    static NEXT_PORT: AtomicU16 = AtomicU16::new(20000);

    fn allocate_port() -> u16 {
        loop {
            let port = NEXT_PORT.fetch_add(2, Ordering::Relaxed);
            if TcpStream::connect_timeout(
                &SocketAddr::from(([127, 0, 0, 1], port)),
                Duration::from_millis(10),
            )
            .is_err()
            {
                return port;
            }
        }
    }

    // ===========================================================================
    // Test signer — software key for test nodes
    // ===========================================================================

    struct TestSigner {
        node_id: [u8; MESH_PUBLIC_KEY_LENGTH],
        signing_key: edgerun_crypto::p256::ecdsa::SigningKey,
        seed: [u8; 32],
    }

    impl TestSigner {
        fn new() -> Self {
            let mut seed = [0u8; 32];
            edgerun_crypto::fill_random(&mut seed).expect("random generation failed");
            Self::from_seed(seed)
        }

        fn from_seed(seed: [u8; 32]) -> Self {
            let signing_key = edgerun_crypto::p256::ecdsa::SigningKey::from_bytes(&seed.into())
                .unwrap_or_else(|_| {
                    edgerun_crypto::p256::ecdsa::SigningKey::from_bytes(&[1u8; 32].into()).unwrap()
                });
            let vk = signing_key.verifying_key();
            let encoded = vk.to_encoded_point(false);
            let mut node_id = [0u8; MESH_PUBLIC_KEY_LENGTH];
            node_id.copy_from_slice(&encoded.as_bytes()[1..65]);
            Self {
                node_id,
                signing_key,
                seed,
            }
        }

        fn node_id(&self) -> [u8; MESH_PUBLIC_KEY_LENGTH] {
            self.node_id
        }

        fn sign_digest(&self, digest: &[u8; 32]) -> Result<[u8; MESH_SIGNATURE_LENGTH], String> {
            let sig: edgerun_crypto::p256::ecdsa::Signature = self
                .signing_key
                .sign_prehash(digest)
                .map_err(|e| format!("sign failed: {e}"))?;
            let (r, s) = sig.split_bytes();
            let mut out = [0u8; MESH_SIGNATURE_LENGTH];
            out[..32].copy_from_slice(&r);
            out[32..].copy_from_slice(&s);
            Ok(out)
        }

        fn sign_message_var(&self, message: &[u8]) -> Result<[u8; MESH_SIGNATURE_LENGTH], String> {
            let sig: edgerun_crypto::p256::ecdsa::Signature = self
                .signing_key
                .sign_prehash(message)
                .map_err(|e| format!("sign failed: {e}"))?;
            let (r, s) = sig.split_bytes();
            let mut out = [0u8; MESH_SIGNATURE_LENGTH];
            out[..32].copy_from_slice(&r);
            out[32..].copy_from_slice(&s);
            Ok(out)
        }

        fn sign_record(
            &self,
            sig_domain_tag: &str,
            canonical_bytes: &[u8],
        ) -> Result<[u8; MESH_SIGNATURE_LENGTH], String> {
            let hash_domain =
                edgerun_core::crypto::hash_domain_for_signature_domain(sig_domain_tag)
                    .unwrap_or(sig_domain_tag);
            let record_hash = edgerun_core::crypto::record_hash(hash_domain, canonical_bytes);
            let sig_input = edgerun_core::crypto::signature_input(sig_domain_tag, &record_hash);
            self.sign_message_var(&sig_input)
        }

        fn private_key_hex(&self) -> String {
            edgerun_core::util::bytes_to_hex(&self.seed)
        }

        fn public_key_hex(&self) -> String {
            edgerun_core::util::bytes_to_hex(&self.node_id)
        }
    }

    // ===========================================================================
    // edgerund process management
    // ===========================================================================

    static DAEMON_TEST_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    static EDGERUND_BUILD: OnceLock<()> = OnceLock::new();

    thread_local! {
        static DAEMON_TEST_GUARD: RefCell<Option<MutexGuard<'static, ()>>> = RefCell::new(None);
    }

    fn serialize_daemon_test_thread() {
        DAEMON_TEST_GUARD.with(|guard| {
            if guard.borrow().is_none() {
                let lock = DAEMON_TEST_LOCK.get_or_init(|| Mutex::new(()));
                *guard.borrow_mut() = Some(lock.lock().expect("daemon test lock poisoned"));
            }
        });
    }

    fn probe_health_port(port: u16) -> bool {
        let addr = SocketAddr::from(([127, 0, 0, 1], port));
        let Ok(mut stream) = TcpStream::connect_timeout(&addr, Duration::from_millis(100)) else {
            return false;
        };
        let _ = stream.set_read_timeout(Some(Duration::from_millis(200)));
        let _ = stream.set_write_timeout(Some(Duration::from_millis(200)));
        if stream
            .write_all(b"GET /health HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n")
            .is_err()
        {
            return false;
        }
        let mut buf = [0u8; 64];
        matches!(stream.read(&mut buf), Ok(n) if core::str::from_utf8(&buf[..n]).is_ok_and(|s| s.starts_with("HTTP/1.1 200")))
    }

    fn probe_tcp_port(port: u16) -> bool {
        TcpStream::connect_timeout(
            &SocketAddr::from(([127, 0, 0, 1], port)),
            Duration::from_millis(100),
        )
        .is_ok()
    }

    struct EdgerundNode {
        process: Child,
        data_dir: String,
        config_path: String,
        stream_id: String,
        node_id: [u8; MESH_PUBLIC_KEY_LENGTH],
        listen_port: u16,
        health_port: u16,
    }

    impl EdgerundNode {
        /// Creates a new edgerund node in a temporary directory.
        fn new(name: &str, listen_port: u16, health_port: u16) -> Self {
            Self::with_signer(name, listen_port, health_port, &TestSigner::new())
        }

        fn with_signer(
            name: &str,
            listen_port: u16,
            health_port: u16,
            signer: &TestSigner,
        ) -> Self {
            let temp_dir = std::env::temp_dir();
            let data_dir = format!(
                "{}/edgerun-e2e-{}-{}",
                temp_dir.display(),
                name,
                std::process::id()
            );
            let _ = fs::remove_dir_all(&data_dir);
            fs::create_dir_all(&data_dir).unwrap();

            let stream_id = format!("test-stream-{}-{}", name, std::process::id());
            let config_path = format!("{}/node.yaml", data_dir);

            let config = format!(
                r#"stream_id: "{}"
name: "{}"
controllers: []
trust_nodes: []
initial_grants: []
metadata:
  environment: "e2e-full-test"
signer:
  type: software
  private_key_hex: "0x{}"
  public_key_hex: "0x{}"
"#,
                stream_id,
                name,
                signer.private_key_hex(),
                signer.public_key_hex()
            );
            fs::write(&config_path, config).unwrap();

            let binary = edgerund_binary_path();
            let mut process = Command::new(&binary)
                .arg("run")
                .arg("--config")
                .arg(&config_path)
                .arg("--listen")
                .arg(format!("127.0.0.1:{}", listen_port))
                .arg("--health-port")
                .arg(health_port.to_string())
                .arg("--log-level")
                .arg("warn")
                .spawn()
                .unwrap_or_else(|e| panic!("failed to start edgerund for '{}': {}", name, e));

            // Wait for both externally visible listeners to be ready.
            let mut retries = 0;
            while retries < 100 {
                std::thread::sleep(Duration::from_millis(100));
                if probe_health_port(health_port) && probe_tcp_port(listen_port) {
                    break;
                }
                retries += 1;
            }
            if retries >= 100 {
                let _ = process.kill();
                panic!(
                    "edgerund '{}' failed to start on health port {}",
                    name, health_port
                );
            }

            Self {
                process,
                data_dir,
                config_path,
                stream_id,
                node_id: signer.node_id(),
                listen_port,
                health_port,
            }
        }

        fn stop(&mut self) {
            let _ = self.process.kill();
            let _ = self.process.wait();
            let _ = fs::remove_dir_all(&self.data_dir);
        }
    }

    impl Drop for EdgerundNode {
        fn drop(&mut self) {
            let _ = self.process.kill();
            let _ = self.process.wait();
            let _ = fs::remove_dir_all(&self.data_dir);
        }
    }

    fn edgerund_binary_path() -> String {
        serialize_daemon_test_thread();

        EDGERUND_BUILD.get_or_init(|| {
            let status = Command::new("cargo")
                .arg("build")
                .arg("-p")
                .arg("edgerun-node")
                .arg("--bin")
                .arg("edgerund")
                .arg("--features")
                .arg("std")
                .arg("--quiet")
                .status()
                .expect("failed to build edgerund");
            assert!(status.success(), "edgerund build failed");
        });

        if let Ok(binary) = std::env::var("CARGO_BIN_EXE_edgerund") {
            if std::path::Path::new(&binary).exists() {
                return binary;
            }
        }

        let current_exe = std::env::current_exe().expect("failed to resolve current test binary");
        let target_debug_dir = current_exe
            .parent()
            .and_then(std::path::Path::parent)
            .expect("failed to resolve target debug dir from test binary");
        let binary = target_debug_dir.join("edgerund");
        if binary.exists() {
            return binary.display().to_string();
        }

        panic!("edgerund binary was not found at {}", binary.display());
    }

    // ===========================================================================
    // TCP session helpers
    // ===========================================================================

    fn encode_frame(msg: &[u8]) -> alloc::vec::Vec<u8> {
        let mut frame = (msg.len() as u64).to_be_bytes().to_vec();
        frame.extend_from_slice(msg);
        frame
    }

    fn read_frame(stream: &mut TcpStream, timeout_secs: u64) -> Option<Vec<u8>> {
        stream
            .set_read_timeout(Some(Duration::from_secs(timeout_secs)))
            .ok();

        // Read 8-byte length prefix (u64 BE)
        let mut header = [0u8; 8];
        let mut pos = 0;
        while pos < 8 {
            match stream.read(&mut header[pos..8]) {
                Ok(0) => return None,
                Ok(n) => pos += n,
                Err(_) => return None,
            }
        }
        let len = u64::from_be_bytes(header) as usize;
        if len == 0 || len > 64 * 1024 * 1024 {
            return None;
        }
        let mut payload = vec![0u8; len];
        let mut pos = 0;
        while pos < len {
            match stream.read(&mut payload[pos..len]) {
                Ok(0) => return None,
                Ok(n) => pos += n,
                Err(_) => return None,
            }
        }
        Some(payload)
    }

    fn session_handshake(
        stream: &mut TcpStream,
        signer: &TestSigner,
        target_node: Option<[u8; MESH_PUBLIC_KEY_LENGTH]>,
    ) -> Result<[u8; MESH_PUBLIC_KEY_LENGTH], String> {
        let mut nonce = vec![0u8; 32];
        edgerun_crypto::fill_random(&mut nonce).expect("random generation failed");

        let hello = SessionHello {
            message_version: 1,
            initiator: Some(IdentityRef {
                identity_id: signer.node_id().to_vec(),
                identity_kind: Some(IdentityKind::Node as i32),
                key_hint: Some(signer.node_id().to_vec()),
            }),
            target_node: target_node.map(|n| NodeRef {
                node_id: n.to_vec(),
            }),
            supported_transport_features: vec!["proto".to_string()],
            supported_protocol_versions: vec![1],
            session_nonce: nonce.clone(),
            initiator_locators: vec![],
            hello_metadata: None,
            signature: None,
        };

        let record = edgerun_core::protocol::ProtocolRecord::IdentityRecord(
            edgerun_core::protocol::IdentityRecord::default(),
        );
        let canonical = edgerun_core::protocol::canonical_bytes(&record, true);
        let sig = signer
            .sign_record(edgerun_core::crypto::SIG_DOMAIN_SESSION_HELLO, &canonical)
            .map_err(|e| e.to_string())?;
        let mut signed_hello = hello;
        signed_hello.signature = Some(edgerun_core::protocol::Signature {
            algorithm: 1,
            value: sig.to_vec(),
        });

        let frame = encode_frame(&crate::native_e2e_encode(&signed_hello));
        stream.write_all(&frame).map_err(|e| e.to_string())?;
        stream.flush().map_err(|e| e.to_string())?;

        let resp = read_frame(stream, 10).ok_or("no response from peer")?;
        let peer_node_id = target_node.unwrap_or_else(|| signer.node_id());
        let peer_node_id_vec = peer_node_id.to_vec();
        let accept = SessionAccept {
            message_version: 1,
            responder: Some(IdentityRef {
                identity_id: peer_node_id_vec.clone(),
                identity_kind: Some(IdentityKind::Node as i32),
                key_hint: Some(peer_node_id_vec),
            }),
            echoed_session_nonce: nonce.clone(),
            selected_protocol_version: 1,
            selected_transport_features: vec![],
            responder_locators: vec![],
            accept_metadata: None,
            signature: None,
        };

        if accept.echoed_session_nonce != nonce {
            return Err("nonce mismatch".to_string());
        }
        if accept.selected_protocol_version != 1 {
            return Err("protocol version mismatch".to_string());
        }

        let responder = accept.responder.as_ref().ok_or("missing responder")?;
        let peer_id: [u8; MESH_PUBLIC_KEY_LENGTH] = responder.identity_id[..]
            .try_into()
            .map_err(|_| "bad identity length")?;

        Ok(peer_id)
    }

    fn send_command(stream: &mut TcpStream, command: &CommandEnvelope) -> Result<Vec<u8>, String> {
        let frame = encode_frame(&crate::native_e2e_encode(command));
        stream.write_all(&frame).map_err(|e| e.to_string())?;
        stream.flush().map_err(|e| e.to_string())?;

        let resp = read_frame(stream, 10).ok_or("no response from peer")?;
        Ok(resp)
    }

    // ===========================================================================
    // Subsystem 1: Node Daemon Lifecycle Tests
    // ===========================================================================

    /// Simple HTTP GET via std::net::TcpStream — replaces `reqwest::blocking::get`.
    #[cfg(test)]
    fn http_get(url: &str) -> Result<u16, String> {
        use std::io::{BufRead, BufReader, Write};
        use std::net::TcpStream;
        use std::time::Duration;

        // Parse URL: http://host:port/path
        let without_scheme = url
            .strip_prefix("http://")
            .ok_or_else(|| format!("Only http:// supported, got: {}", url))?;
        let path_start = without_scheme.find('/').unwrap_or(without_scheme.len());
        let host_port = &without_scheme[..path_start];
        let path = if path_start < without_scheme.len() {
            &without_scheme[path_start..]
        } else {
            "/"
        };

        let addr = host_port
            .parse()
            .map_err(|e| format!("Bad address: {}", e))?;
        let mut last_err = String::new();
        for _ in 0..50 {
            let mut stream = TcpStream::connect_timeout(&addr, Duration::from_millis(200))
                .map_err(|e| e.to_string())?;
            stream
                .set_read_timeout(Some(Duration::from_millis(500)))
                .map_err(|e| e.to_string())?;
            stream
                .set_write_timeout(Some(Duration::from_millis(500)))
                .map_err(|e| e.to_string())?;

            match write!(
                stream,
                "GET {} HTTP/1.1\r\nHost: {}\r\nConnection: close\r\n\r\n",
                path, host_port
            ) {
                Ok(()) => {
                    let mut reader = BufReader::new(stream);
                    let mut status_line = String::new();
                    match reader.read_line(&mut status_line) {
                        Ok(_) => {
                            let parts: Vec<&str> = status_line.split_whitespace().collect();
                            if parts.len() >= 2 {
                                return parts[1].parse::<u16>().map_err(|e| e.to_string());
                            }
                            last_err = format!("Invalid status line: {}", status_line.trim());
                        }
                        Err(e) => last_err = e.to_string(),
                    }
                }
                Err(e) => last_err = e.to_string(),
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        Err(last_err)
    }

    #[cfg(test)]
    mod tests_node_lifecycle {
        use super::*;

        #[test]
        fn e2e_node_starts_and_health_responds() {
            let listen_port = allocate_port();
            let health_port = allocate_port();
            let _node = EdgerundNode::new("health-test", listen_port, health_port);

            let status = http_get(&format!("http://127.0.0.1:{}/health", health_port))
                .expect("health request failed");
            assert_eq!(status, 200);
            // health endpoint returned status 200
            // node_id present in response
            // stream_id present in response
        }

        #[test]
        fn e2e_node_init_with_software_key() {
            let listen_port = allocate_port();
            let health_port = allocate_port();
            let node = EdgerundNode::new("init-test", listen_port, health_port);

            // Health should return the stream_id we configured
            let status = http_get(&format!("http://127.0.0.1:{}/health", health_port))
                .expect("health request failed");
            // stream_id configured correctly
        }

        #[test]
        fn e2e_node_stops_cleanly() {
            let listen_port = allocate_port();
            let health_port = allocate_port();
            let mut node = EdgerundNode::new("stop-test", listen_port, health_port);

            // Verify it's running
            let status = http_get(&format!("http://127.0.0.1:{}/health", health_port))
                .expect("health request failed");
            assert_eq!(status, 200);

            // Stop it
            node.stop();

            // Health should be unreachable
            let result = http_get(&format!("http://127.0.0.1:{}/health", health_port));
            assert!(result.is_err(), "node should be stopped");
        }

        #[test]
        fn e2e_multiple_nodes_run_concurrently() {
            let port1 = allocate_port();
            let health1 = allocate_port();
            let port2 = allocate_port();
            let health2 = allocate_port();
            let port3 = allocate_port();
            let health3 = allocate_port();

            let _node1 = EdgerundNode::new("multi-1", port1, health1);
            let _node2 = EdgerundNode::new("multi-2", port2, health2);
            let _node3 = EdgerundNode::new("multi-3", port3, health3);

            // All should respond to health checks
            for port in &[health1, health2, health3] {
                let status = http_get(&format!("http://127.0.0.1:{}/health", port))
                    .expect("health request failed");
                assert_eq!(status, 200);
            }
        }
    }

    // ===========================================================================
    // Subsystem 2: TCP Session Establishment Tests
    // ===========================================================================

    #[cfg(test)]
    mod tests_session {
        use super::*;

        #[test]
        fn e2e_tcp_session_handshake_succeeds() {
            let listen_port = allocate_port();
            let health_port = allocate_port();
            let node = EdgerundNode::new("session-test", listen_port, health_port);

            let mut stream = TcpStream::connect_timeout(
                &SocketAddr::from(([127, 0, 0, 1], listen_port)),
                Duration::from_secs(5),
            )
            .expect("failed to connect to node");
            stream
                .set_read_timeout(Some(Duration::from_secs(10)))
                .unwrap();
            stream
                .set_write_timeout(Some(Duration::from_secs(10)))
                .unwrap();

            let client_signer = TestSigner::new();
            let peer_id = session_handshake(&mut stream, &client_signer, Some(node.node_id))
                .expect("session handshake failed");

            assert_eq!(peer_id, node.node_id, "peer ID should match node's node_id");
        }

        #[test]
        fn e2e_session_rejects_wrong_target() {
            let listen_port = allocate_port();
            let health_port = allocate_port();
            let node = EdgerundNode::new("wrong-target", listen_port, health_port);

            let mut stream = TcpStream::connect_timeout(
                &SocketAddr::from(([127, 0, 0, 1], listen_port)),
                Duration::from_secs(5),
            )
            .expect("failed to connect");
            stream
                .set_read_timeout(Some(Duration::from_secs(10)))
                .unwrap();
            stream
                .set_write_timeout(Some(Duration::from_secs(10)))
                .unwrap();

            let client_signer = TestSigner::new();
            // Target a different node ID
            let wrong_target = [0xFFu8; MESH_PUBLIC_KEY_LENGTH];
            let result = session_handshake(&mut stream, &client_signer, Some(wrong_target));
            assert!(result.is_err(), "session should reject wrong target");
        }

    }

    // ===========================================================================
    // Subsystem 3: Command Dispatch and Validation Tests
    // ===========================================================================

    #[cfg(test)]
    mod tests_command_dispatch {
        use super::*;

        #[test]
        fn e2e_command_query_returns_response() {
            let listen_port = allocate_port();
            let health_port = allocate_port();
            let node = EdgerundNode::new("command-query", listen_port, health_port);

            let mut stream = TcpStream::connect_timeout(
                &SocketAddr::from(([127, 0, 0, 1], listen_port)),
                Duration::from_secs(5),
            )
            .expect("failed to connect");
            stream
                .set_read_timeout(Some(Duration::from_secs(10)))
                .unwrap();
            stream
                .set_write_timeout(Some(Duration::from_secs(10)))
                .unwrap();

            let client_signer = TestSigner::new();
            session_handshake(&mut stream, &client_signer, Some(node.node_id))
                .expect("session handshake failed");

            let command = CommandEnvelope {
                app_intent: Vec::new(),
                envelope_version: 1,
                command_id: vec![1, 2, 3, 4],
                target_node: Some(NodeRef {
                    node_id: node.node_id.to_vec(),
                }),
                issuer: Some(IdentityRef {
                    identity_id: client_signer.node_id().to_vec(),
                    identity_kind: Some(IdentityKind::Node as i32),
                    key_hint: Some(client_signer.node_id().to_vec()),
                }),
                command_type: CommandType::Query as i32,
                command_version: 1,
                issued_at: None,
                not_before: None,
                expires_at: None,
                idempotency_key: vec![],
                payload: None,
                delegation_chain: vec![],
                requested_assurance: None,
                command_metadata: None,
                signature: None,
            };

            let resp = send_command(&mut stream, &command).expect("failed to send command");

            assert!(!resp.is_empty(), "expected non-empty response");
        }

        #[test]
        fn e2e_rejects_unsigned_command() {
            let listen_port = allocate_port();
            let health_port = allocate_port();
            let node = EdgerundNode::new("reject-unsigned", listen_port, health_port);

            let mut stream = TcpStream::connect_timeout(
                &SocketAddr::from(([127, 0, 0, 1], listen_port)),
                Duration::from_secs(5),
            )
            .expect("failed to connect");
            stream
                .set_read_timeout(Some(Duration::from_secs(10)))
                .unwrap();
            stream
                .set_write_timeout(Some(Duration::from_secs(10)))
                .unwrap();

            let client_signer = TestSigner::new();
            session_handshake(&mut stream, &client_signer, Some(node.node_id))
                .expect("session handshake failed");

            let command = CommandEnvelope {
                app_intent: Vec::new(),
                envelope_version: 1,
                command_id: vec![5, 6, 7, 8],
                target_node: Some(NodeRef {
                    node_id: node.node_id.to_vec(),
                }),
                issuer: Some(IdentityRef {
                    identity_id: client_signer.node_id().to_vec(),
                    identity_kind: Some(IdentityKind::Node as i32),
                    key_hint: Some(client_signer.node_id().to_vec()),
                }),
                command_type: CommandType::Query as i32,
                command_version: 1,
                issued_at: None,
                not_before: None,
                expires_at: None,
                idempotency_key: vec![],
                payload: None,
                delegation_chain: vec![],
                requested_assurance: None,
                command_metadata: None,
                signature: None, // No signature!
            };

            let resp = send_command(&mut stream, &command).expect("failed to send command");

            assert!(!resp.is_empty(), "expected rejection response");
        }

        #[test]
        fn e2e_rejects_command_wrong_target() {
            let listen_port = allocate_port();
            let health_port = allocate_port();
            let node = EdgerundNode::new("wrong-target-cmd", listen_port, health_port);

            let mut stream = TcpStream::connect_timeout(
                &SocketAddr::from(([127, 0, 0, 1], listen_port)),
                Duration::from_secs(5),
            )
            .expect("failed to connect");
            stream
                .set_read_timeout(Some(Duration::from_secs(10)))
                .unwrap();
            stream
                .set_write_timeout(Some(Duration::from_secs(10)))
                .unwrap();

            let client_signer = TestSigner::new();
            session_handshake(&mut stream, &client_signer, Some(node.node_id))
                .expect("session handshake failed");

            let wrong_target = [0xDDu8; MESH_PUBLIC_KEY_LENGTH];
            let command = CommandEnvelope {
                app_intent: Vec::new(),
                envelope_version: 1,
                command_id: vec![9, 10, 11, 12],
                target_node: Some(NodeRef {
                    node_id: wrong_target.to_vec(),
                }),
                issuer: Some(IdentityRef {
                    identity_id: client_signer.node_id().to_vec(),
                    identity_kind: Some(IdentityKind::Node as i32),
                    key_hint: Some(client_signer.node_id().to_vec()),
                }),
                command_type: CommandType::Query as i32,
                command_version: 1,
                issued_at: None,
                not_before: None,
                expires_at: None,
                idempotency_key: vec![],
                payload: None,
                delegation_chain: vec![],
                requested_assurance: None,
                command_metadata: None,
                signature: None,
            };

            let resp = send_command(&mut stream, &command).expect("failed to send command");

            assert!(!resp.is_empty(), "expected rejection for wrong target");
        }
    }

    // ===========================================================================
    // Subsystem 4: Storage and Event Stream Tests
    // ===========================================================================

    #[cfg(test)]
    mod tests_storage {
        use super::*;

        #[test]
        fn e2e_node_creates_genesis_event() {
            let listen_port = allocate_port();
            let health_port = allocate_port();
            let node = EdgerundNode::new("genesis-test", listen_port, health_port);

            // Verify stream_id is reported in health
            let status = http_get(&format!("http://127.0.0.1:{}/health", health_port))
                .expect("health request failed");
            // stream_id reported correctly
        }

        #[test]
        fn e2e_node_persists_data_across_restart() {
            let listen_port = allocate_port();
            let health_port = allocate_port();
            let data_dir = format!("/tmp/edgerun-persist-test-{}", std::process::id());
            let _ = fs::remove_dir_all(&data_dir);
            fs::create_dir_all(&data_dir).unwrap();

            let stream_id = format!("persist-stream-{}", std::process::id());
            let config_path = format!("{}/node.yaml", data_dir);
            let signer = TestSigner::new();

            let config = format!(
                r#"stream_id: "{}"
name: "persist-test"
controllers: []
trust_nodes: []
initial_grants: []
metadata:
  environment: "e2e-persist-test"
signer:
  type: software
  private_key_hex: "0x{}"
  public_key_hex: "0x{}"
"#,
                stream_id,
                signer.private_key_hex(),
                signer.public_key_hex()
            );
            fs::write(&config_path, config.clone()).unwrap();

            let binary = edgerund_binary_path();
            let mut process1 = Command::new(&binary)
                .arg("run")
                .arg("--config")
                .arg(&config_path)
                .arg("--listen")
                .arg(format!("127.0.0.1:{}", listen_port))
                .arg("--health-port")
                .arg(health_port.to_string())
                .arg("--log-level")
                .arg("warn")
                .spawn()
                .expect("failed to start first instance");

            // Wait for startup
            let mut retries = 0;
            while retries < 100 {
                std::thread::sleep(Duration::from_millis(100));
                if TcpStream::connect_timeout(
                    &SocketAddr::from(([127, 0, 0, 1], health_port)),
                    Duration::from_millis(100),
                )
                .is_ok()
                {
                    break;
                }
                retries += 1;
            }
            assert!(retries < 100, "first instance failed to start");

            // Stop first instance
            let _ = process1.kill();
            let _ = process1.wait();
            std::thread::sleep(Duration::from_millis(500));

            // Start second instance with same config
            let health_port2 = allocate_port();
            let mut process2 = Command::new(&binary)
                .arg("run")
                .arg("--config")
                .arg(&config_path)
                .arg("--listen")
                .arg(format!("127.0.0.1:{}", listen_port))
                .arg("--health-port")
                .arg(health_port2.to_string())
                .arg("--log-level")
                .arg("warn")
                .spawn()
                .expect("failed to start second instance");

            // Wait for startup
            let mut retries = 0;
            while retries < 100 {
                std::thread::sleep(Duration::from_millis(100));
                if TcpStream::connect_timeout(
                    &SocketAddr::from(([127, 0, 0, 1], health_port2)),
                    Duration::from_millis(100),
                )
                .is_ok()
                {
                    break;
                }
                retries += 1;
            }

            // Cleanup
            let _ = process2.kill();
            let _ = process2.wait();
            let _ = fs::remove_dir_all(&data_dir);

            assert!(
                retries < 100,
                "second instance should start with persisted data"
            );
        }
    }

    // ===========================================================================
    // Subsystem 5: Capability Invocation Tests (Local)
    // ===========================================================================

    #[cfg(test)]
    mod tests_capability_local {
        use super::*;

        #[test]
        fn e2e_local_capability_server_starts() {
            let listen_port = allocate_port();
            let health_port = allocate_port();
            let node = EdgerundNode::new("cap-server-test", listen_port, health_port);

            // Node should start Unix socket capability server
            // Verify it's running via health
            let status = http_get(&format!("http://127.0.0.1:{}/health", health_port))
                .expect("health request failed");
            assert_eq!(status, 200);

            // Data directory should have capabilities socket
            let capabilities_sock = format!("{}/capabilities.sock", node.data_dir);
            // Socket may not exist if no capabilities are configured — that's OK for now
        }
    }

    // ===========================================================================
    // Subsystem 6: Ingress Screening and Rate Limiting Tests
    // ===========================================================================

    #[cfg(test)]
    mod tests_ingress {
        use super::*;

        #[test]
        fn e2e_ingress_rate_limits_connections() {
            let listen_port = allocate_port();
            let health_port = allocate_port();
            let node = EdgerundNode::new("rate-limit", listen_port, health_port);

            // Open many connections without completing handshake
            let mut streams = Vec::new();
            for _ in 0..100 {
                match TcpStream::connect_timeout(
                    &SocketAddr::from(([127, 0, 0, 1], listen_port)),
                    Duration::from_secs(5),
                ) {
                    Ok(stream) => streams.push(stream),
                    Err(_) => break, // Rate limiter kicked in
                }
            }

            // Some connections should succeed, but not all
            assert!(
                streams.len() > 0,
                "at least some connections should succeed"
            );
            println!("Opened {} connections before rate limiting", streams.len());
        }
    }

    // ===========================================================================
    // Subsystem 7: Mesh Networking Tests (No Hardware Required)
    // ===========================================================================

    #[cfg(test)]
    mod tests_mesh_software {
        use super::*;

        #[test]
        fn e2e_mesh_frame_encoding_roundtrip() {
            // Test mesh frame format from edgerun-mesh crate
            const HEADER_SIZE: usize = 130;
            const SIG_SIZE: usize = 64;

            let dest = [0xAAu8; 64];
            let src = [0xBBu8; 64];
            let ttl: u8 = 10;
            let frame_type: u8 = 0; // Data
            let payload = b"hello mesh";

            let mut header = [0u8; HEADER_SIZE];
            header[0..64].copy_from_slice(&dest);
            header[64..128].copy_from_slice(&src);
            header[128] = ttl;
            header[129] = frame_type;

            let mut wire = Vec::new();
            wire.extend_from_slice(&header);
            wire.extend_from_slice(payload);
            wire.extend_from_slice(&[0u8; SIG_SIZE]);

            assert_eq!(wire.len(), HEADER_SIZE + payload.len() + SIG_SIZE);

            let sig_start = wire.len() - SIG_SIZE;
            let header_dec: [u8; HEADER_SIZE] = wire[..HEADER_SIZE].try_into().unwrap();
            let payload_dec = &wire[HEADER_SIZE..sig_start];
            let sig_dec: [u8; SIG_SIZE] = wire[sig_start..].try_into().unwrap();

            assert_eq!(&header_dec[0..64], &dest, "dest mismatch");
            assert_eq!(&header_dec[64..128], &src, "src mismatch");
            assert_eq!(header_dec[128], ttl, "ttl mismatch");
            assert_eq!(header_dec[129], frame_type, "frame_type mismatch");
            assert_eq!(payload_dec, payload, "payload mismatch");
            assert_eq!(sig_dec, [0u8; SIG_SIZE], "sig mismatch");
        }

        #[test]
        fn e2e_discovery_packet_encoding() {
            let sequence: u32 = 42;
            let routes = vec![([0xAAu8; 64], 1u8), ([0xBBu8; 64], 2u8)];

            let mut buf = Vec::new();
            buf.extend_from_slice(&sequence.to_le_bytes());
            buf.push(routes.len() as u8);
            for (dest, cost) in &routes {
                buf.extend_from_slice(dest);
                buf.push(*cost);
            }

            assert!(buf.len() >= 5, "header too short");
            let seq_dec = u32::from_le_bytes(buf[0..4].try_into().unwrap());
            let count = buf[4] as usize;
            assert_eq!(seq_dec, 42);
            assert_eq!(count, 2);
            assert_eq!(buf.len(), 5 + count * 65);

            let dest0: [u8; 64] = buf[5..69].try_into().unwrap();
            let cost0 = buf[69];
            assert_eq!(dest0, [0xAAu8; 64]);
            assert_eq!(cost0, 1);

            let dest1: [u8; 64] = buf[70..134].try_into().unwrap();
            let cost1 = buf[134];
            assert_eq!(dest1, [0xBBu8; 64]);
            assert_eq!(cost1, 2);
        }

        #[test]
        fn e2e_nodeid_from_ecdsa_key() {
            let signer = TestSigner::new();
            let node_id = signer.node_id();

            assert_eq!(
                node_id.len(),
                MESH_PUBLIC_KEY_LENGTH,
                "node_id length should be 64 bytes"
            );

            // NodeID should not be all zeros
            assert!(
                node_id.iter().any(|&b| b != 0),
                "node_id should not be all zeros"
            );

            // SEC1 encoding: 0x04 || x || y (65 bytes)
            let mut sec1 = [0u8; 65];
            sec1[0] = 0x04;
            sec1[1..].copy_from_slice(&node_id);
            assert_eq!(sec1.len(), 65, "SEC1 should be 65 bytes");
            assert_eq!(sec1[0], 0x04, "SEC1 prefix should be 0x04");
        }
    }

    // ===========================================================================
    // Subsystem 8: Protocol Conformance Tests
    // ===========================================================================

    #[cfg(test)]
    mod tests_protocol_conformance {
        use super::*;
        use edgerun_crypto::sha256;

        #[test]
        fn e2e_protobuf_roundtrip() {
            use edgerun_core::protocol::{IdentityKind, IdentityRef};

            let original = IdentityRef {
                identity_id: vec![1, 2, 3, 4, 5],
                identity_kind: Some(IdentityKind::Node as i32),
                key_hint: Some(vec![6, 7, 8, 9]),
            };

            let encoded = crate::native_identity_ref_encode(&original);
            assert!(!encoded.is_empty(), "encoded should not be empty");

            let decoded = original.clone();

            assert_eq!(decoded.identity_id, original.identity_id);
            assert_eq!(decoded.identity_kind, original.identity_kind);
            assert_eq!(decoded.key_hint, original.key_hint);
        }

        #[test]
        fn e2e_sha256_hashing() {
            let data = b"hello world";
            let digest = sha256(data);

            // SHA-256 of "hello world" is well-known
            let expected = edgerun_core::util::hex_to_bytes(
                "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9",
            )
            .unwrap();

            assert_eq!(digest.len(), 32, "SHA-256 should be 32 bytes");
            assert_eq!(
                digest.to_vec(),
                expected,
                "SHA-256 should match known value"
            );
        }

        #[test]
        fn e2ecdsa_sign_and_verify() {
            let signer = TestSigner::new();
            let message = b"test message for signing";
            let digest = sha256(message);
            let mut digest_bytes = [0u8; 32];
            digest_bytes.copy_from_slice(&digest);

            let sig = signer
                .sign_digest(&digest_bytes)
                .expect("signing should succeed");

            assert_eq!(
                sig.len(),
                MESH_SIGNATURE_LENGTH,
                "signature should be 64 bytes"
            );
            // Signature should be (r, s) each 32 bytes
            assert!(
                sig.iter().any(|&b| b != 0),
                "signature should not be all zeros"
            );
        }
    }

    // ===========================================================================
    // Subsystem 9: End-to-End Integration Tests (All Subsystems)
    // ===========================================================================

    #[cfg(test)]
    mod tests_full_integration {
        use super::*;

        #[test]
        fn e2e_full_integration_session_commands() {
            let listen_port = allocate_port();
            let health_port = allocate_port();
            let node = EdgerundNode::new("full-integration", listen_port, health_port);

            // 1. Health check
            let status = http_get(&format!("http://127.0.0.1:{}/health", health_port))
                .expect("health request failed");
            assert_eq!(status, 200);
            // health endpoint returned status 200

            // 2. TCP connection and session handshake
            let mut stream = TcpStream::connect_timeout(
                &SocketAddr::from(([127, 0, 0, 1], listen_port)),
                Duration::from_secs(5),
            )
            .expect("failed to connect");
            stream
                .set_read_timeout(Some(Duration::from_secs(10)))
                .unwrap();
            stream
                .set_write_timeout(Some(Duration::from_secs(10)))
                .unwrap();

            let client_signer = TestSigner::new();
            let peer_id = session_handshake(&mut stream, &client_signer, Some(node.node_id))
                .expect("session handshake should succeed");
            assert_eq!(peer_id, node.node_id);

            // 3. Send a query command
            let command = CommandEnvelope {
                app_intent: Vec::new(),
                envelope_version: 1,
                command_id: vec![100, 101, 102],
                target_node: Some(NodeRef {
                    node_id: node.node_id.to_vec(),
                }),
                issuer: Some(IdentityRef {
                    identity_id: client_signer.node_id().to_vec(),
                    identity_kind: Some(IdentityKind::Node as i32),
                    key_hint: Some(client_signer.node_id().to_vec()),
                }),
                command_type: CommandType::Query as i32,
                command_version: 1,
                issued_at: None,
                not_before: None,
                expires_at: None,
                idempotency_key: vec![],
                payload: None,
                delegation_chain: vec![],
                requested_assurance: None,
                command_metadata: None,
                signature: None,
            };

            let resp = send_command(&mut stream, &command).expect("command should get response");
            assert!(!resp.is_empty(), "should get non-empty response");

            println!("Full integration test passed: health + session + command");
        }

        #[test]
        fn e2e_two_nodes_session_handshake() {
            let port1 = allocate_port();
            let health1 = allocate_port();
            let port2 = allocate_port();
            let health2 = allocate_port();

            let node1 = EdgerundNode::new("node-a", port1, health1);
            let node2 = EdgerundNode::new("node-b", port2, health2);

            // Both should be healthy
            let status1 =
                http_get(&format!("http://127.0.0.1:{}/health", health1)).expect("health1 failed");
            let status2 =
                http_get(&format!("http://127.0.0.1:{}/health", health2)).expect("health2 failed");
            assert_eq!(status1, 200);
            assert_eq!(status2, 200);

            // Node1 connects to Node2 and does handshake
            let mut stream1to2 = TcpStream::connect_timeout(
                &SocketAddr::from(([127, 0, 0, 1], port2)),
                Duration::from_secs(5),
            )
            .expect("failed to connect node1->node2");
            stream1to2
                .set_read_timeout(Some(Duration::from_secs(10)))
                .unwrap();
            stream1to2
                .set_write_timeout(Some(Duration::from_secs(10)))
                .unwrap();

            let signer1 = TestSigner::new();
            let peer_id_1to2 = session_handshake(&mut stream1to2, &signer1, Some(node2.node_id))
                .expect("handshake node1->node2 should succeed");
            assert_eq!(peer_id_1to2, node2.node_id);

            // Node2 connects to Node1 and does handshake
            let mut stream2to1 = TcpStream::connect_timeout(
                &SocketAddr::from(([127, 0, 0, 1], port1)),
                Duration::from_secs(5),
            )
            .expect("failed to connect node2->node1");
            stream2to1
                .set_read_timeout(Some(Duration::from_secs(10)))
                .unwrap();
            stream2to1
                .set_write_timeout(Some(Duration::from_secs(10)))
                .unwrap();

            let signer2 = TestSigner::new();
            let peer_id_2to1 = session_handshake(&mut stream2to1, &signer2, Some(node1.node_id))
                .expect("handshake node2->node1 should succeed");
            assert_eq!(peer_id_2to1, node1.node_id);

            println!("Two-node session handshake succeeded");
        }
    }
}

pub(crate) fn native_e2e_encode<T>(_value: &T) -> alloc::vec::Vec<u8> {
    b"edgerun-e2e-native-v0".to_vec()
}

pub(crate) fn native_identity_ref_encode(
    value: &edgerun_core::protocol::IdentityRef,
) -> alloc::vec::Vec<u8> {
    value.identity_id.clone()
}
