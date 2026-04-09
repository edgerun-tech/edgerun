//! Session establishment protocol for edgerun TCP connections.
//!
//! Implements the signed SessionHello/SessionAccept handshake per §9 of the spec.
//! After session establishment, peers exchange RouteAdvertisements.

use edgerun_hardware_signing::NodeID;
use edgerun_proto::edgerun::v0::{
    common::{IdentityKind, IdentityRef, NodeRef},
    network::{SessionAccept, SessionHello},
};
use edgerun_core::crypto::sha256;
use edgerun_core::protocol::ProtocolRecord;
use prost::Message;
use std::time::{SystemTime, UNIX_EPOCH};

/// Supported protocol versions.
pub const PROTOCOL_VERSION: u32 = 1;

/// Supported transport features.
pub const TRANSPORT_FEATURES: &[&str] = &["proto"];

/// Maximum session nonce size (32 bytes of randomness).
pub const NONCE_SIZE: usize = 32;

/// Session state after successful handshake.
#[derive(Clone, Debug)]
pub struct SessionState {
    /// Peer's node identity (public key bytes).
    pub peer_node_id: NodeID,
    /// Peer's identity reference.
    pub peer_identity: IdentityRef,
    /// Protocol version negotiated.
    pub protocol_version: u32,
    /// Selected transport features.
    pub transport_features: Vec<String>,
    /// Whether this session is initiated (we sent hello first) or accepted.
    pub is_initiator: bool,
}

/// Generate a random nonce for session handshake.
pub fn generate_nonce() -> Vec<u8> {
    let mut nonce = vec![0u8; NONCE_SIZE];
    edgerun_core::crypto::fill_random(&mut nonce);
    nonce
}

/// Build a signed SessionHello message.
pub fn build_session_hello(
    local_node_id: &NodeID,
    target_node_id: Option<&NodeID>,
    nonce: &[u8],
    signer: &dyn edgerun_hardware_signing::MeshSigner,
) -> Result<SessionHello, String> {
    let hello = SessionHello {
        message_version: 1,
        initiator: Some(IdentityRef {
            identity_id: local_node_id.0.to_vec(),
            identity_kind: Some(IdentityKind::Node as i32),
            key_hint: Some(local_node_id.0.to_vec()),
        }),
        target_node: target_node_id.map(|n| NodeRef {
            node_id: n.0.to_vec(),
        }),
        supported_transport_features: TRANSPORT_FEATURES.iter().map(|s| s.to_string()).collect(),
        supported_protocol_versions: vec![PROTOCOL_VERSION],
        session_nonce: nonce.to_vec(),
        initiator_locators: vec![],
        hello_metadata: None,
        signature: None,
    };

    sign_session_hello(hello, signer)
}

/// Sign a SessionHello with the local node's key.
fn sign_session_hello(
    mut hello: SessionHello,
    signer: &dyn edgerun_hardware_signing::MeshSigner,
) -> Result<SessionHello, String> {
    let record = ProtocolRecord::SessionHello(hello.clone());
    let canonical = edgerun_core::protocol::canonical_bytes(&record, true);
    let digest = sha256(&canonical);
    let mut digest_bytes = [0u8; 32];
    digest_bytes.copy_from_slice(&digest);
    match signer.sign_digest(&digest_bytes) {
        Ok(sig) => {
            hello.signature = Some(edgerun_core::protocol::Signature {
                algorithm: 1,
                value: sig.to_vec(),
            });
            Ok(hello)
        }
        Err(e) => Err(format!("failed to sign SessionHello: {}", e)),
    }
}

/// Build a signed SessionAccept message.
pub fn build_session_accept(
    local_node_id: &NodeID,
    echoed_nonce: &[u8],
    protocol_version: u32,
    transport_features: Vec<String>,
    signer: &dyn edgerun_hardware_signing::MeshSigner,
) -> Result<SessionAccept, String> {
    let accept = SessionAccept {
        message_version: 1,
        responder: Some(IdentityRef {
            identity_id: local_node_id.0.to_vec(),
            identity_kind: Some(IdentityKind::Node as i32),
            key_hint: Some(local_node_id.0.to_vec()),
        }),
        echoed_session_nonce: echoed_nonce.to_vec(),
        selected_protocol_version: protocol_version,
        selected_transport_features: transport_features,
        responder_locators: vec![],
        accept_metadata: None,
        signature: None,
    };

    sign_session_accept(accept, signer)
}

/// Sign a SessionAccept with the local node's key.
fn sign_session_accept(
    mut accept: SessionAccept,
    signer: &dyn edgerun_hardware_signing::MeshSigner,
) -> Result<SessionAccept, String> {
    let record = ProtocolRecord::SessionAccept(accept.clone());
    let canonical = edgerun_core::protocol::canonical_bytes(&record, true);
    let digest = sha256(&canonical);
    let mut digest_bytes = [0u8; 32];
    digest_bytes.copy_from_slice(&digest);
    match signer.sign_digest(&digest_bytes) {
        Ok(sig) => {
            accept.signature = Some(edgerun_core::protocol::Signature {
                algorithm: 1,
                value: sig.to_vec(),
            });
            Ok(accept)
        }
        Err(e) => Err(format!("failed to sign SessionAccept: {}", e)),
    }
}

/// Verify a SessionHello's signature and extract the peer's node ID.
pub fn verify_session_hello(hello: &SessionHello) -> Result<NodeID, &'static str> {
    let Some(ref sig) = hello.signature else {
        return Err("missing_signature");
    };
    if sig.value.len() != 64 {
        return Err("bad_signature_length");
    }
    let Some(ref initiator) = hello.initiator else {
        return Err("missing_initiator");
    };
    if initiator.identity_id.len() != 64 {
        return Err("bad_identity_id_length");
    }

    // Verify the signature using canonical bytes
    let mut hello_for_verify = hello.clone();
    let record = ProtocolRecord::SessionHello(hello_for_verify.clone());
    let canonical = edgerun_core::protocol::canonical_bytes(&record, true);
    let digest = sha256(&canonical);

    // Build verifying key from identity_id
    let mut vk_sec1 = [0u8; 65];
    vk_sec1[0] = 0x04; // Uncompressed point marker
    vk_sec1[1..].copy_from_slice(&initiator.identity_id);

    let vk = p256::ecdsa::VerifyingKey::from_sec1_bytes(&vk_sec1)
        .map_err(|_| "bad_public_key")?;

    // Construct signature from raw bytes (r || s)
    let sig_bytes = &sig.value;
    let r = p256::FieldBytes::from_slice(&sig_bytes[..32]);
    let s = p256::FieldBytes::from_slice(&sig_bytes[32..]);
    let ecdsa_sig = p256::ecdsa::Signature::from_scalars(*r, *s)
        .map_err(|_| "invalid_signature")?;

    use p256::ecdsa::signature::hazmat::PrehashVerifier;
    vk.verify_prehash(digest.as_slice(), &ecdsa_sig)
        .map_err(|_| "invalid_signature")?;

    let mut node_id_bytes = [0u8; 64];
    node_id_bytes.copy_from_slice(&initiator.identity_id);
    Ok(NodeID(node_id_bytes))
}

/// Verify a SessionAccept's signature, nonce echo, and protocol version.
pub fn verify_session_accept(
    accept: &SessionAccept,
    expected_nonce: &[u8],
) -> Result<NodeID, &'static str> {
    let Some(ref sig) = accept.signature else {
        return Err("missing_signature");
    };
    if sig.value.len() != 64 {
        return Err("bad_signature_length");
    }
    let Some(ref responder) = accept.responder else {
        return Err("missing_responder");
    };
    if responder.identity_id.len() != 64 {
        return Err("bad_identity_id_length");
    }

    // Verify nonce echo
    if accept.echoed_session_nonce != expected_nonce {
        return Err("nonce_mismatch");
    }

    // Verify protocol version is supported
    if accept.selected_protocol_version != PROTOCOL_VERSION {
        return Err("unsupported_protocol_version");
    }

    // Verify the signature using canonical bytes
    let record = ProtocolRecord::SessionAccept(accept.clone());
    let canonical = edgerun_core::protocol::canonical_bytes(&record, true);
    let digest = sha256(&canonical);

    // Build verifying key from identity_id
    let mut vk_sec1 = [0u8; 65];
    vk_sec1[0] = 0x04; // Uncompressed point marker
    vk_sec1[1..].copy_from_slice(&responder.identity_id);

    let vk = p256::ecdsa::VerifyingKey::from_sec1_bytes(&vk_sec1)
        .map_err(|_| "bad_public_key")?;

    // Construct signature from raw bytes (r || s)
    let sig_bytes = &sig.value;
    let r = p256::FieldBytes::from_slice(&sig_bytes[..32]);
    let s = p256::FieldBytes::from_slice(&sig_bytes[32..]);
    let ecdsa_sig = p256::ecdsa::Signature::from_scalars(*r, *s)
        .map_err(|_| "invalid_signature")?;

    use p256::ecdsa::signature::hazmat::PrehashVerifier;
    vk.verify_prehash(digest.as_slice(), &ecdsa_sig)
        .map_err(|_| "invalid_signature")?;

    let mut node_id_bytes = [0u8; 64];
    node_id_bytes.copy_from_slice(&responder.identity_id);
    Ok(NodeID(node_id_bytes))
}

/// Select the best protocol version from the intersection of supported versions.
pub fn select_protocol_version(supported: &[u32]) -> Option<u32> {
    // In v0, we only support version 1
    if supported.contains(&PROTOCOL_VERSION) {
        Some(PROTOCOL_VERSION)
    } else {
        None
    }
}

/// Select transport features from the intersection of supported features.
pub fn select_transport_features(supported: &[String]) -> Vec<String> {
    TRANSPORT_FEATURES
        .iter()
        .map(|s| s.to_string())
        .filter(|f| supported.contains(f))
        .collect()
}

/// Encode a SessionHello to bytes.
pub fn encode_hello(hello: &SessionHello) -> Vec<u8> {
    Message::encode_to_vec(hello)
}

/// Encode a SessionAccept to bytes.
pub fn encode_accept(accept: &SessionAccept) -> Vec<u8> {
    Message::encode_to_vec(accept)
}

/// Decode a SessionHello from bytes.
pub fn decode_hello(bytes: &[u8]) -> Result<SessionHello, prost::DecodeError> {
    SessionHello::decode(bytes)
}

/// Decode a SessionAccept from bytes.
pub fn decode_accept(bytes: &[u8]) -> Result<SessionAccept, prost::DecodeError> {
    SessionAccept::decode(bytes)
}

/// Get current timestamp as protobuf Timestamp.
pub fn now_timestamp() -> prost_types::Timestamp {
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
    prost_types::Timestamp {
        seconds: now.as_secs() as i64,
        nanos: now.subsec_nanos() as i32,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use p256::ecdsa::SigningKey;
    use p256::ecdsa::signature::hazmat::PrehashSigner;
    use edgerun_hardware_signing::MeshSigner;

    struct TestSigner {
        node_id: NodeID,
        signing_key: SigningKey,
    }

    impl TestSigner {
        fn new() -> Self {
            // Generate a deterministic key from a counter for testing
            static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
            let count = COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            let mut seed = [0u8; 32];
            seed[..8].copy_from_slice(&count.to_be_bytes());
            seed[8..16].copy_from_slice(&count.to_be_bytes());
            seed[16..24].copy_from_slice(&count.to_be_bytes());
            seed[24..].copy_from_slice(&count.to_be_bytes());

            let signing_key = SigningKey::from_bytes(&seed.into())
                .unwrap_or_else(|_| {
                    // If seed is invalid, use a fallback
                    SigningKey::from_bytes(&[1u8; 32].into()).unwrap()
                });
            let verifying_key = signing_key.verifying_key();
            let encoded = verifying_key.to_encoded_point(false);
            let mut node_id_bytes = [0u8; 64];
            node_id_bytes.copy_from_slice(&encoded.as_bytes()[1..65]);
            Self {
                node_id: NodeID(node_id_bytes),
                signing_key,
            }
        }
    }

    impl edgerun_hardware_signing::MeshSigner for TestSigner {
        fn node_id(&self) -> NodeID {
            self.node_id
        }

        fn sign_digest(&self, digest: &[u8; 32]) -> Result<[u8; edgerun_hardware_signing::MESH_SIGNATURE_LENGTH], edgerun_hardware_signing::HardwareSigningError> {
            let sig: p256::ecdsa::Signature = self.signing_key.sign_prehash(digest)
                .map_err(|_| edgerun_hardware_signing::HardwareSigningError::Provider("signing failed".into()))?;
            let (r, s) = sig.split_bytes();
            let mut out = [0u8; edgerun_hardware_signing::MESH_SIGNATURE_LENGTH];
            out[..32].copy_from_slice(&r);
            out[32..].copy_from_slice(&s);
            Ok(out)
        }
    }

    #[test]
    fn test_session_hello_roundtrip() {
        let signer = TestSigner::new();
        let nonce = generate_nonce();
        let hello = build_session_hello(&signer.node_id(), None, &nonce, &signer).unwrap();

        let bytes = encode_hello(&hello);
        let decoded = decode_hello(&bytes).unwrap();

        assert_eq!(decoded.session_nonce, nonce);
        assert!(decoded.signature.is_some());
    }

    #[test]
    fn test_session_hello_verification() {
        let signer = TestSigner::new();
        let nonce = generate_nonce();
        let hello = build_session_hello(&signer.node_id(), None, &nonce, &signer).unwrap();

        let result = verify_session_hello(&hello);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), signer.node_id());
    }

    #[test]
    fn test_session_accept_roundtrip() {
        let signer = TestSigner::new();
        let nonce = generate_nonce();
        let accept = build_session_accept(&signer.node_id(), &nonce, PROTOCOL_VERSION,
            TRANSPORT_FEATURES.iter().map(|s| s.to_string()).collect(), &signer).unwrap();

        let bytes = encode_accept(&accept);
        let decoded = decode_accept(&bytes).unwrap();

        assert_eq!(decoded.echoed_session_nonce, nonce);
        assert_eq!(decoded.selected_protocol_version, PROTOCOL_VERSION);
        assert!(decoded.signature.is_some());
    }

    #[test]
    fn test_session_accept_verification() {
        let signer = TestSigner::new();
        let nonce = generate_nonce();
        let accept = build_session_accept(&signer.node_id(), &nonce, PROTOCOL_VERSION,
            TRANSPORT_FEATURES.iter().map(|s| s.to_string()).collect(), &signer).unwrap();

        let result = verify_session_accept(&accept, &nonce);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), signer.node_id());
    }

    #[test]
    fn test_session_accept_wrong_nonce() {
        let signer = TestSigner::new();
        let nonce = generate_nonce();
        let wrong_nonce = generate_nonce();
        let accept = build_session_accept(&signer.node_id(), &nonce, PROTOCOL_VERSION,
            TRANSPORT_FEATURES.iter().map(|s| s.to_string()).collect(), &signer).unwrap();

        let result = verify_session_accept(&accept, &wrong_nonce);
        assert!(result.is_err());
    }

    #[test]
    fn test_protocol_version_selection() {
        assert_eq!(select_protocol_version(&[1]), Some(1));
        assert_eq!(select_protocol_version(&[2, 3]), None);
        assert_eq!(select_protocol_version(&[1, 2]), Some(1));
    }

    #[test]
    fn test_transport_feature_selection() {
        let features = vec!["proto".to_string(), "unknown".to_string()];
        let selected = select_transport_features(&features);
        assert_eq!(selected, vec!["proto".to_string()]);
    }
}
