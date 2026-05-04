//! Capability message signing and verification.
//!
//! Every critical capability message carries an ECDSA P-256 signature over its
//! serialized protobuf bytes (with the signature field cleared). The signer's
//! public key is their NodeID (64-byte uncompressed P-256 point).
//!
//! ## Signing
//!
//! 1. Construct the capability message (Invocation, Request, Grant, etc.)
//! 2. Set `signature = None`
//! 3. Serialize to protobuf bytes
//! 4. SHA-256 hash the bytes
//! 5. Sign the 32-byte digest via the hardware signer
//! 6. Set `signature = Some(Signature { algorithm: ECDSA_P256, value: sig })`
//!
//! ## Verification
//!
//! 1. Extract `signature` from the message (fail if missing)
//! 2. Clone message, set `signature = None`
//! 3. Serialize to protobuf bytes
//! 4. SHA-256 hash the bytes
//! 5. Verify ECDSA P-256 signature using sender's NodeID as public key

use crate::prelude::v1::*;
use edgerun_core::protocol::capability::{
    CapabilityGrant, CapabilityInvocation, CapabilityRequest, CapabilityResult,
    CapabilityRevocation,
};
use edgerun_core::protocol::{signature, Signature};
use edgerun_crypto::p256::ecdsa::VerifyingKey;
use edgerun_hardware_signing::{MeshSigner, NodeID};

/// The ECDSA P-256 algorithm identifier used in protobuf Signature messages.
pub const SIGNATURE_ALGORITHM_ECDSA_P256_SHA256: i32 =
    signature::Algorithm::SignatureAlgorithmEcdsaP256Sha256 as i32;
const CAPABILITY_MESSAGE_SIG_DOMAIN: &str = "edgerun:v0:sig:capability-message";

// ---------------------------------------------------------------------------
// Generic sign / verify
// ---------------------------------------------------------------------------

/// Signs a protobuf message in place. The message's signature field is set to
/// the ECDSA P-256 signature over the serialized message bytes (with signature
/// cleared).
pub fn sign_message<M: Message>(
    signer: &dyn MeshSigner,
    msg: &mut M,
    clear_sig: impl FnOnce(&mut M),
    set_sig: impl FnOnce(&mut M, Signature),
) -> Result<(), String> {
    // Clear signature and serialize
    clear_sig(msg);
    let mut buf = Vec::new();
    msg.encode(&mut buf)
        .map_err(|e| format!("proto encode: {e}"))?;

    // Sign with domain separation for capability protocol messages
    let sig_bytes = signer
        .sign_record(CAPABILITY_MESSAGE_SIG_DOMAIN, &buf)
        .map_err(|e| format!("sign: {e}"))?;

    // Set signature
    set_sig(
        msg,
        Signature {
            algorithm: SIGNATURE_ALGORITHM_ECDSA_P256_SHA256,
            value: sig_bytes.to_vec(),
        },
    );
    Ok(())
}

/// Verifies a protobuf message signature.
///
/// Extracts the signature, re-serializes the message with signature cleared,
/// and verifies the ECDSA P-256 signature against the sender's NodeID.
pub fn verify_message<M: Message + Clone>(
    msg: &M,
    sender: NodeID,
    get_sig: impl Fn(&M) -> Option<Signature>,
    clear_sig: impl FnOnce(&M) -> M,
) -> Result<(), String> {
    let sig = get_sig(msg).ok_or("message has no signature")?;
    if sig.algorithm != SIGNATURE_ALGORITHM_ECDSA_P256_SHA256 {
        return Err(format!(
            "unsupported signature algorithm: {}",
            sig.algorithm
        ));
    }
    if sig.value.len() != 64 {
        return Err(format!("invalid signature length: {}", sig.value.len()));
    }

    // Clear signature and re-serialize
    let msg_clone = clear_sig(msg);
    let mut buf = Vec::new();
    msg_clone
        .encode(&mut buf)
        .map_err(|e| format!("proto encode: {e}"))?;

    // Build verifying key from sender's NodeID (x||y without 0x04 prefix)
    let mut sec1_bytes = [0u8; 65];
    sec1_bytes[0] = 0x04;
    sec1_bytes[1..].copy_from_slice(&sender.0);
    let pubkey = VerifyingKey::from_sec1_bytes(&sec1_bytes)
        .map_err(|e| format!("invalid public key in NodeID: {e}"))?;

    if edgerun_core::crypto::verify_canonical_record(
        &pubkey,
        CAPABILITY_MESSAGE_SIG_DOMAIN,
        &buf,
        &sig.value,
    ) || edgerun_core::crypto::verify_canonical_record_hw(
        &pubkey,
        CAPABILITY_MESSAGE_SIG_DOMAIN,
        &buf,
        &sig.value,
    ) {
        Ok(())
    } else {
        Err("signature verification failed: signature error".into())
    }
}

// ---------------------------------------------------------------------------
// Per-message-type helpers
// ---------------------------------------------------------------------------

macro_rules! impl_sign_verify {
    ($msg:ty, $clear:ident, $set:ident, $sign:ident, $verify:ident) => {
        fn $clear(msg: &mut $msg) {
            msg.signature = None;
        }
        fn $set(msg: &mut $msg, sig: Signature) {
            msg.signature = Some(sig);
        }
        pub fn $sign(signer: &dyn MeshSigner, msg: &mut $msg) -> Result<(), String> {
            sign_message_bytes(signer, msg, $clear, $set)
        }
        pub fn $verify(msg: &$msg, sender: NodeID) -> Result<(), String> {
            let get_sig = |m: &$msg| m.signature.clone();
            verify_message_bytes(msg, sender, get_sig, |m| {
                let mut c = m.clone();
                c.signature = None;
                c
            })
        }
    };
}

impl_sign_verify!(
    CapabilityInvocation,
    clear_invocation,
    set_invocation_sig,
    sign_invocation,
    verify_invocation
);

impl_sign_verify!(
    CapabilityRequest,
    clear_request,
    set_request_sig,
    sign_request,
    verify_request
);

impl_sign_verify!(
    CapabilityGrant,
    clear_grant,
    set_grant_sig,
    sign_grant,
    verify_grant
);

impl_sign_verify!(
    CapabilityResult,
    clear_result,
    set_result_sig,
    sign_result,
    verify_result
);

impl_sign_verify!(
    CapabilityRevocation,
    clear_revocation,
    set_revocation_sig,
    sign_revocation,
    verify_revocation
);

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use edgerun_crypto::p256::ecdsa::signature::hazmat::PrehashSigner;
    use edgerun_hardware_signing::MeshSigner;

    struct TestSigner {
        node_id: NodeID,
        key: edgerun_crypto::p256::ecdsa::SigningKey,
    }

    impl TestSigner {
        fn new() -> Self {
            let key = edgerun_crypto::random_p256_signing_key();
            let vk = key.verifying_key();
            let encoded = vk.to_encoded_point(false);
            let mut node_bytes = [0u8; 64];
            // Skip the 0x04 prefix, take x||y
            node_bytes.copy_from_slice(&encoded.as_bytes()[1..65]);
            Self {
                node_id: NodeID(node_bytes),
                key,
            }
        }
    }

    impl MeshSigner for TestSigner {
        fn node_id(&self) -> NodeID {
            self.node_id
        }

        fn sign_digest(
            &self,
            digest: &[u8; 32],
        ) -> Result<[u8; 64], edgerun_hardware_signing::HardwareSigningError> {
            let sig: edgerun_crypto::p256::ecdsa::Signature =
                self.key.sign_prehash(digest).map_err(|e| {
                    edgerun_hardware_signing::HardwareSigningError::Provider(e.to_string())
                })?;
            let mut bytes = [0u8; 64];
            bytes.copy_from_slice(&sig.to_bytes());
            Ok(bytes)
        }
    }

    #[test]
    fn raw_ecdsa_sign_verify_roundtrip() {
        use edgerun_crypto::p256::ecdsa::signature::{Signer, Verifier as SigVerifier};

        let key = edgerun_crypto::random_p256_signing_key();
        let vk = key.verifying_key();

        let msg = b"test message";
        let sig: edgerun_crypto::p256::ecdsa::Signature = key.sign(msg);

        // Verify using standard trait
        use edgerun_crypto::p256::ecdsa::signature::Verifier;
        vk.verify(msg, &sig).unwrap();

        // Now test via our helper functions
        let encoded = vk.to_encoded_point(false);
        let mut node_bytes = [0u8; 64];
        node_bytes.copy_from_slice(&encoded.as_bytes()[1..65]);
        let node_id = NodeID(node_bytes);

        // Build sec1 bytes for verification
        let mut sec1_bytes = [0u8; 65];
        sec1_bytes[0] = 0x04;
        sec1_bytes[1..].copy_from_slice(&node_id.0);
        let vk2 = VerifyingKey::from_sec1_bytes(&sec1_bytes).unwrap();

        // Verify the same signature
        vk2.verify(msg, &sig).unwrap();

        // And test the to_bytes/from_slice roundtrip
        let sig_bytes = sig.to_bytes();
        let sig2 = edgerun_crypto::p256::ecdsa::Signature::from_slice(&sig_bytes).unwrap();
        vk.verify(msg, &sig2).unwrap();
    }

    #[test]
    fn sign_and_verify_invocation() {
        // Create a signing key and corresponding NodeID
        let key = edgerun_crypto::random_p256_signing_key();
        let vk = key.verifying_key();
        let encoded = vk.to_encoded_point(false);
        let mut node_bytes = [0u8; 64];
        node_bytes.copy_from_slice(&encoded.as_bytes()[1..65]);
        let node_id = NodeID(node_bytes);

        // Create a signer that uses this key
        struct KeySigner {
            node_id: NodeID,
            key: edgerun_crypto::p256::ecdsa::SigningKey,
        }
        impl MeshSigner for KeySigner {
            fn node_id(&self) -> NodeID {
                self.node_id
            }
            fn sign_digest(
                &self,
                digest: &[u8; 32],
            ) -> Result<[u8; 64], edgerun_hardware_signing::HardwareSigningError> {
                let sig: edgerun_crypto::p256::ecdsa::Signature =
                    self.key.sign_prehash(digest).map_err(|e| {
                        edgerun_hardware_signing::HardwareSigningError::Provider(e.to_string())
                    })?;
                let mut bytes = [0u8; 64];
                bytes.copy_from_slice(&sig.to_bytes());
                Ok(bytes)
            }
        }
        let signer = KeySigner { node_id, key };

        let mut invocation = CapabilityInvocation {
            invocation_version: 1,
            invocation_id: b"inv-1".to_vec(),
            grant_id: b"grant-1".to_vec(),
            invoker: None,
            operation: 1,
            requested_access_class: 2,
            parameter_object: None,
            correlation_id: Vec::new(),
            invoked_at: None,
            signature: None,
        };

        sign_invocation(&signer, &mut invocation).unwrap();
        assert!(invocation.signature.is_some());

        verify_invocation(&invocation, signer.node_id()).unwrap();
    }

    #[test]
    fn verify_fails_on_tampered_message() {
        let signer = TestSigner::new();
        let mut invocation = CapabilityInvocation {
            invocation_version: 1,
            invocation_id: b"inv-1".to_vec(),
            grant_id: b"grant-1".to_vec(),
            invoker: None,
            operation: 1,
            requested_access_class: 2,
            parameter_object: None,
            correlation_id: Vec::new(),
            invoked_at: None,
            signature: None,
        };

        sign_invocation(&signer, &mut invocation).unwrap();

        // Tamper with the message
        invocation.operation = 99;

        let err = verify_invocation(&invocation, signer.node_id()).unwrap_err();
        assert!(err.contains("verification failed"));
    }

    #[test]
    fn verify_fails_on_wrong_signer() {
        let signer_a = TestSigner::new();
        let signer_b = TestSigner::new();

        let mut invocation = CapabilityInvocation {
            invocation_version: 1,
            invocation_id: b"inv-1".to_vec(),
            grant_id: b"grant-1".to_vec(),
            invoker: None,
            operation: 1,
            requested_access_class: 2,
            parameter_object: None,
            correlation_id: Vec::new(),
            invoked_at: None,
            signature: None,
        };

        sign_invocation(&signer_a, &mut invocation).unwrap();

        // Verify with wrong public key
        let err = verify_invocation(&invocation, signer_b.node_id()).unwrap_err();
        assert!(err.contains("verification failed"));
    }

    #[test]
    fn sign_and_verify_request() {
        let signer = TestSigner::new();
        let mut request = CapabilityRequest {
            request_version: 1,
            request_id: b"req-1".to_vec(),
            requester: None,
            requester_node: None,
            selector: None,
            requested_operations: vec![1],
            requested_constraints: vec![],
            purpose: "test".into(),
            requested_duration: None,
            correlation_id: Vec::new(),
            signature: None,
        };

        sign_request(&signer, &mut request).unwrap();
        assert!(request.signature.is_some());
        verify_request(&request, signer.node_id()).unwrap();
    }

    #[test]
    fn sign_and_verify_grant() {
        let signer = TestSigner::new();
        let mut grant = CapabilityGrant {
            grant_version: 1,
            grant_id: b"grant-1".to_vec(),
            issuer: None,
            grantee: None,
            grantee_node: None,
            selector: None,
            granted_operations: vec![1],
            enforced_constraints: vec![],
            access_class: 2,
            issued_at: None,
            expires_at: None,
            correlation_id: Vec::new(),
            supersedes_revocation: None,
            signature: None,
        };

        sign_grant(&signer, &mut grant).unwrap();
        assert!(grant.signature.is_some());
        verify_grant(&grant, signer.node_id()).unwrap();
    }

    #[test]
    fn sign_and_verify_result() {
        let signer = TestSigner::new();
        let mut result = CapabilityResult {
            result_version: 1,
            invocation_id: b"inv-1".to_vec(),
            grant_id: b"grant-1".to_vec(),
            success: true,
            result_access_class: 2,
            produced_event_kinds: vec![],
            payload_object: None,
            error_reason: String::new(),
            produced_at: None,
            signature: None,
        };

        sign_result(&signer, &mut result).unwrap();
        assert!(result.signature.is_some());
        verify_result(&result, signer.node_id()).unwrap();
    }

    #[test]
    fn sign_and_verify_revocation() {
        let signer = TestSigner::new();
        let mut revocation = CapabilityRevocation {
            revocation_version: 1,
            revocation_id: b"rev-1".to_vec(),
            grant_id: b"grant-1".to_vec(),
            issuer: None,
            effective_at: None,
            reason: "test".into(),
            replacement_constraints: vec![],
            signature: None,
        };

        sign_revocation(&signer, &mut revocation).unwrap();
        assert!(revocation.signature.is_some());
        verify_revocation(&revocation, signer.node_id()).unwrap();
    }

    #[test]
    fn verify_rejects_missing_signature() {
        let invocation = CapabilityInvocation {
            invocation_version: 1,
            invocation_id: b"inv-1".to_vec(),
            grant_id: b"grant-1".to_vec(),
            invoker: None,
            operation: 1,
            requested_access_class: 2,
            parameter_object: None,
            correlation_id: Vec::new(),
            invoked_at: None,
            signature: None,
        };

        let err = verify_invocation(&invocation, NodeID([0u8; 64])).unwrap_err();
        assert!(err.contains("no signature"));
    }

    #[test]
    fn verify_rejects_wrong_algorithm() {
        let invocation = CapabilityInvocation {
            invocation_version: 1,
            invocation_id: b"inv-1".to_vec(),
            grant_id: b"grant-1".to_vec(),
            invoker: None,
            operation: 1,
            requested_access_class: 2,
            parameter_object: None,
            correlation_id: Vec::new(),
            invoked_at: None,
            signature: Some(Signature {
                algorithm: 0, // UNSPECIFIED -- wrong
                value: vec![0u8; 64],
            }),
        };

        let err = verify_invocation(&invocation, NodeID([0u8; 64])).unwrap_err();
        assert!(err.contains("unsupported signature algorithm"));
    }
}

pub fn sign_bytes(
    message_bytes: &[u8],
    signer: &dyn MeshSigner,
) -> Result<Signature, CapabilitySignatureError> {
    let sig = signer
        .sign(message_bytes)
        .map_err(|_| CapabilitySignatureError::SigningFailed)?;
    Ok(Signature {
        algorithm: 1,
        value: sig,
    })
}

pub fn verify_bytes(
    message_bytes: &[u8],
    signature: &Signature,
    verifier: &dyn MeshVerifier,
) -> Result<(), CapabilitySignatureError> {
    verifier
        .verify(message_bytes, &signature.value)
        .map_err(|_| CapabilitySignatureError::VerificationFailed)
}
