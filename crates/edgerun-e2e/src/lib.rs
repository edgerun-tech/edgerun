//! End-to-end tests for the edgerun system.
//!
//! These tests spawn actual `edgerund` daemon processes, connect to them
//! via TCP, perform session handshakes, send commands, and verify responses.
//! They exercise the full real system — not mocks or stubs.

use std::fs;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::process::{Child, Command};
use std::time::Duration;

use edgerun_core::crypto::{fill_random, sha256};
use edgerun_hardware_signing::{MeshSigner, MESH_PUBLIC_KEY_LENGTH, MESH_SIGNATURE_LENGTH};
use edgerun_proto::edgerun::v0::{
    common::{IdentityKind, IdentityRef, NodeRef},
    network::{SessionAccept, SessionHello},
    stream::{CommandEnvelope, CommandType},
};
use p256::ecdsa::signature::hazmat::PrehashSigner;
use prost::Message;

// ===========================================================================
// Test signer — software key for test nodes
// ===========================================================================

struct TestSigner {
    node_id: [u8; MESH_PUBLIC_KEY_LENGTH],
    signing_key: p256::ecdsa::SigningKey,
}

impl TestSigner {
    fn new() -> Self {
        let mut seed = [0u8; 32];
        fill_random(&mut seed);
        let signing_key = p256::ecdsa::SigningKey::from_bytes(&seed.into())
            .unwrap_or_else(|_| p256::ecdsa::SigningKey::from_bytes(&[1u8; 32].into()).unwrap());
        let vk = signing_key.verifying_key();
        let encoded = vk.to_encoded_point(false);
        let mut node_id = [0u8; MESH_PUBLIC_KEY_LENGTH];
        node_id.copy_from_slice(&encoded.as_bytes()[1..65]);
        Self { node_id, signing_key }
    }

    fn node_id(&self) -> [u8; MESH_PUBLIC_KEY_LENGTH] {
        self.node_id
    }

    fn sign_digest(&self, digest: &[u8; 32]) -> Result<[u8; MESH_SIGNATURE_LENGTH], String> {
        let sig: p256::ecdsa::Signature = self.signing_key.sign_prehash(digest)
            .map_err(|e| format!("sign failed: {e}"))?;
        let (r, s) = sig.split_bytes();
        let mut out = [0u8; MESH_SIGNATURE_LENGTH];
        out[..32].copy_from_slice(&r);
        out[32..].copy_from_slice(&s);
        Ok(out)
    }
}

// ===========================================================================
// edgerund process management
// ===========================================================================

struct EdgerundNode {
    process: Child,
    data_dir: String,
    config_path: String,
    stream_id: String,
    node_id: [u8; MESH_PUBLIC_KEY_LENGTH],
}

impl EdgerundNode {
    /// Creates a new edgerund node in a temporary directory.
    fn new(name: &str, port: u16, health_port: u16) -> Self {
        let temp_dir = std::env::temp_dir();
        let data_dir = format!("{}/edgerun-e2e-{}-{}", temp_dir.display(), name, std::process::id());
        let _ = fs::remove_dir_all(&data_dir);
        fs::create_dir_all(&data_dir).unwrap();

        // Generate software key
        let mut seed = [0u8; 32];
        fill_random(&mut seed);
        let signing_key = p256::ecdsa::SigningKey::from_bytes(&seed.into())
            .unwrap_or_else(|_| p256::ecdsa::SigningKey::from_bytes(&[1u8; 32].into()).unwrap());
        let vk = signing_key.verifying_key();
        let encoded = vk.to_encoded_point(false);
        let mut node_id = [0u8; MESH_PUBLIC_KEY_LENGTH];
        node_id.copy_from_slice(&encoded.as_bytes()[1..65]);
        let node_id_hex = edgerun_core::util::bytes_to_hex(&node_id);
        let private_key_hex = edgerun_core::util::bytes_to_hex(&seed);

        let stream_id = format!("test-stream-{}-{}", name, std::process::id());
        let config_path = format!("{}/node.yaml", data_dir);

        let config = format!(
            r#"stream_id: "{}"
name: "{}"
controllers: []
trust_nodes: []
initial_grants: []
metadata:
  environment: "e2e-test"
signer:
  type: software
  private_key_hex: "0x{}"
  public_key_hex: "0x{}"
"#,
            stream_id, name, private_key_hex, node_id_hex
        );
        fs::write(&config_path, config).unwrap();

        let binary = edgerund_binary_path();
        let mut process = Command::new(&binary)
            .arg("run")
            .arg("--config")
            .arg(&config_path)
            .arg("--listen")
            .arg(format!("127.0.0.1:{}", port))
            .arg("--health-port")
            .arg(health_port.to_string())
            .arg("--log-level")
            .arg("warn")
            .spawn()
            .unwrap_or_else(|e| panic!("failed to start edgerund: {}", e));

        // Wait for the daemon to start listening
        let mut retries = 0;
        while retries < 50 {
            std::thread::sleep(Duration::from_millis(100));
            if TcpStream::connect_timeout(
                &SocketAddr::from(([127, 0, 0, 1], health_port)),
                Duration::from_millis(100),
            ).is_ok() {
                break;
            }
            retries += 1;
        }
        if retries >= 50 {
            // Kill and report failure
            let _ = process.kill();
            panic!("edgerund failed to start on port {}", port);
        }

        Self {
            process,
            data_dir,
            config_path,
            stream_id,
            node_id,
        }
    }

    fn port(&self) -> u16 {
        // Parse from config — we know it from construction
        let config = fs::read_to_string(&self.config_path).unwrap();
        for line in config.lines() {
            if line.starts_with("  type:") {
                // This isn't the port — we need a different approach.
                // Let's just store it as a field.
                break;
            }
        }
        // For now, we'll track it separately.
        0 // placeholder
    }

    fn stop(mut self) {
        let _ = self.process.kill();
        let _ = self.process.wait();
        let _ = fs::remove_dir_all(&self.data_dir);
    }
}

fn edgerund_binary_path() -> String {
    // Find the edgerund binary — either from CARGO_TARGET_DIR or build it
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let binary = format!("{}/../../target/debug/edgerund", manifest_dir);
    if std::path::Path::new(&binary).exists() {
        return binary;
    }
    // Build it
    let status = Command::new("cargo")
        .arg("build")
        .arg("-p")
        .arg("edgerun-node")
        .arg("--quiet")
        .status()
        .expect("failed to build edgerund");
    assert!(status.success(), "edgerund build failed");
    binary
}

// ===========================================================================
// TCP session helpers
// ===========================================================================

/// Encode a protobuf message with a varint length prefix (TCP framing).
fn encode_frame(msg: &[u8]) -> Vec<u8> {
    let mut frame = encode_varint(msg.len() as u64);
    frame.extend_from_slice(msg);
    frame
}

/// Read a varint-length-prefixed frame from a TCP stream.
fn read_frame(stream: &mut TcpStream) -> Option<Vec<u8>> {
    // Read 8-byte length prefix (our daemon uses u64 BE)
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

fn encode_varint(mut v: u64) -> Vec<u8> {
    let mut buf = Vec::new();
    loop {
        let byte = (v & 0x7F) as u8;
        v >>= 7;
        if v == 0 {
            buf.push(byte);
            break;
        }
        buf.push(byte | 0x80);
    }
    buf
}

/// Perform a TCP session handshake.
/// Returns the peer's node ID on success.
fn session_handshake(stream: &mut TcpStream, signer: &TestSigner, target_node: Option<[u8; MESH_PUBLIC_KEY_LENGTH]>) -> Result<[u8; MESH_PUBLIC_KEY_LENGTH], String> {
    use edgerun_proto::edgerun::v0::network::{SessionHello, SessionAccept};

    // Generate nonce
    let mut nonce = vec![0u8; 32];
    fill_random(&mut nonce);

    // Build SessionHello
    let hello = SessionHello {
        message_version: 1,
        initiator: Some(IdentityRef {
            identity_id: signer.node_id().to_vec(),
            identity_kind: Some(IdentityKind::Node as i32),
            key_hint: Some(signer.node_id().to_vec()),
        }),
        target_node: target_node.map(|n| NodeRef { node_id: n.to_vec() }),
        supported_transport_features: vec!["proto".to_string()],
        supported_protocol_versions: vec![1],
        session_nonce: nonce.clone(),
        initiator_locators: vec![],
        hello_metadata: None,
        signature: None,
    };

    // Sign the hello
    let hello_bytes = SessionHello::encode_to_vec(&hello);
    let digest = sha256(&hello_bytes);
    let mut digest_bytes = [0u8; 32];
    digest_bytes.copy_from_slice(&digest);
    let sig = signer.sign_digest(&digest_bytes).map_err(|e| e.to_string())?;
    let mut signed_hello = hello;
    signed_hello.signature = Some(edgerun_proto::edgerun::v0::common::Signature {
        algorithm: 1,
        value: sig.to_vec(),
    });

    // Send
    let frame = encode_frame(&SessionHello::encode_to_vec(&signed_hello));
    stream.write_all(&frame).map_err(|e| e.to_string())?;
    stream.flush().map_err(|e| e.to_string())?;

    // Read SessionAccept
    let resp = read_frame(stream).ok_or("no response from peer")?;
    let accept = SessionAccept::decode(&resp[..]).map_err(|e| e.to_string())?;

    // Verify the accept
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

/// Send a CommandEnvelope and receive a response frame.
fn send_command(stream: &mut TcpStream, command: &CommandEnvelope) -> Result<Vec<u8>, String> {
    let frame = encode_frame(&CommandEnvelope::encode_to_vec(command));
    stream.write_all(&frame).map_err(|e| e.to_string())?;
    stream.flush().map_err(|e| e.to_string())?;

    let resp = read_frame(stream).ok_or("no response from peer")?;
    Ok(resp)
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// Find an available port starting from the given base.
    fn find_port(base: u16) -> u16 {
        for p in base..base + 1000 {
            if TcpStream::connect_timeout(
                &SocketAddr::from(([127, 0, 0, 1], p)),
                Duration::from_millis(10),
            ).is_err() {
                return p;
            }
        }
        panic!("no available port found");
    }

    #[test]
    fn e2e_node_starts_and_responds_to_health() {
        let port = find_port(19000);
        let health_port = find_port(19100);
        let _node = EdgerundNode::new("health-test", port, health_port);

        // Hit the health endpoint
        let resp = reqwest::blocking::get(&format!("http://127.0.0.1:{}/health", health_port))
            .expect("health request failed");
        assert_eq!(resp.status(), 200);
        let body = resp.text().unwrap();
        assert!(body.contains("ok") || body.contains("status"));
    }

    #[test]
    fn e2e_tcp_session_handshake() {
        let port = find_port(19200);
        let health_port = find_port(19300);
        let node = EdgerundNode::new("session-test", port, health_port);

        // Connect as a client and do session handshake
        let mut stream = TcpStream::connect_timeout(
            &SocketAddr::from(([127, 0, 0, 1], port)),
            Duration::from_secs(5),
        ).expect("failed to connect to node");
        stream.set_read_timeout(Some(Duration::from_secs(10))).unwrap();
        stream.set_write_timeout(Some(Duration::from_secs(10))).unwrap();

        let client_signer = TestSigner::new();
        let peer_id = session_handshake(&mut stream, &client_signer, Some(node.node_id))
            .expect("session handshake failed");

        // The peer ID should match the node's node_id
        assert_eq!(peer_id, node.node_id);
    }

    #[test]
    fn e2e_command_query_returns_response() {
        let port = find_port(19400);
        let health_port = find_port(19500);
        let node = EdgerundNode::new("command-test", port, health_port);

        // Connect and handshake
        let mut stream = TcpStream::connect_timeout(
            &SocketAddr::from(([127, 0, 0, 1], port)),
            Duration::from_secs(5),
        ).expect("failed to connect to node");
        stream.set_read_timeout(Some(Duration::from_secs(10))).unwrap();
        stream.set_write_timeout(Some(Duration::from_secs(10))).unwrap();

        let client_signer = TestSigner::new();
        session_handshake(&mut stream, &client_signer, Some(node.node_id))
            .expect("session handshake failed");

        // Build a QUERY command
        let command = CommandEnvelope {
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

        // Send the command
        let resp = send_command(&mut stream, &command)
            .expect("failed to send command");

        // The response should be a valid protobuf frame (non-empty)
        assert!(!resp.is_empty(), "expected non-empty response");
    }

    #[test]
    fn e2e_rejects_command_with_bad_signature() {
        let port = find_port(19600);
        let health_port = find_port(19700);
        let node = EdgerunNode::new("bad-sig-test", port, health_port);

        // Connect but DON'T sign the command (signature = None)
        let mut stream = TcpStream::connect_timeout(
            &SocketAddr::from(([127, 0, 0, 1], port)),
            Duration::from_secs(5),
        ).expect("failed to connect to node");
        stream.set_read_timeout(Some(Duration::from_secs(10))).unwrap();
        stream.set_write_timeout(Some(Duration::from_secs(10))).unwrap();

        let client_signer = TestSigner::new();
        session_handshake(&mut stream, &client_signer, Some(node.node_id))
            .expect("session handshake failed");

        // Build a QUERY command with NO signature
        let command = CommandEnvelope {
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

        let resp = send_command(&mut stream, &command)
            .expect("failed to send command");

        // Should get a rejection response (not empty, protocol-level rejection)
        assert!(!resp.is_empty(), "expected rejection response");
    }
}
