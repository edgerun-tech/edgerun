use alloc::vec::Vec;

use crate::codec::{blake3_hash, wire_bytes, wire_from_bytes};
use crate::generated_wire::ArchivedNotaryPayload;
use crate::identity::node_identity_from_key;
use crate::preimage::{HashBuilder, PreimageBuilder};
use crate::protocol::*;
use crate::roles::{network_message_for_role, RoleContext, RoleInput, RoleOutput, WorkRole};
use crate::signing::{empty_signature, sign_ed25519, verify_signature};
use edgerun_crypto::sealing::{seal_aes256_gcm, unseal_aes256_gcm, SealedBytes};
use edgerun_crypto::Ed25519SigningKey;

const NOTARY_AAD_HASH_DOMAIN: &[u8] = b"edgerun:v1:work:notary:aad";
const NOTARY_SEALED_HASH_DOMAIN: &[u8] = b"edgerun:v1:work:notary:sealed";
const NOTARY_DELIVERY_REPORT_DOMAIN: &[u8] = b"edgerun:v1:work:notary:delivery-report";

pub type NotarySealKey = [u8; 32];

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NotaryError {
    InvalidPayload,
    InvalidShape,
    SealFailed,
    UnsealFailed,
    InvalidReport,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NotarySealRequest {
    pub abi_version: u16,
    pub aad: Vec<u8>,
    pub plaintext_hash: Hash,
    pub plaintext: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NotarySealResponse {
    pub abi_version: u16,
    pub notary: NodeIdentity,
    pub requester: NodeId,
    pub aad_hash: Hash,
    pub plaintext_hash: Hash,
    pub sealed_hash: Hash,
    pub sealed_envelope: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NotaryUnsealRequest {
    pub abi_version: u16,
    pub aad: Vec<u8>,
    pub sealed_hash: Hash,
    pub sealed_envelope: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NotaryDeliveryReport {
    pub abi_version: u16,
    pub notary: NodeIdentity,
    pub requester: NodeId,
    pub request_message_id: Hash,
    pub request_payload_hash: Hash,
    pub via_relay: NodeId,
    pub sequence: u64,
    pub aad_hash: Hash,
    pub sealed_hash: Hash,
    pub plaintext_hash: Hash,
    pub opened_unix_ms: u64,
    pub signature: WorkSignature,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NotaryUnsealResponse {
    pub abi_version: u16,
    pub plaintext_hash: Hash,
    pub plaintext: Vec<u8>,
    pub delivery_report: NotaryDeliveryReport,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NotaryPayload {
    SealRequest(NotarySealRequest),
    SealResponse(NotarySealResponse),
    UnsealRequest(NotaryUnsealRequest),
    UnsealResponse(NotaryUnsealResponse),
}

pub fn notary_aad_hash(aad: &[u8]) -> Hash {
    HashBuilder::domain(NOTARY_AAD_HASH_DOMAIN)
        .bytes(aad)
        .finish()
}

pub fn notary_sealed_hash(sealed_envelope: &[u8]) -> Hash {
    HashBuilder::domain(NOTARY_SEALED_HASH_DOMAIN)
        .bytes(sealed_envelope)
        .finish()
}

pub fn notary_payload_bytes(payload: &NotaryPayload) -> Result<Vec<u8>, WorkProtocolError> {
    wire_bytes(payload)
}

pub fn notary_payload_from_bytes(bytes: &[u8]) -> Result<NotaryPayload, WorkProtocolError> {
    wire_from_bytes::<NotaryPayload, ArchivedNotaryPayload>(bytes)
}

pub fn notary_seal_request(aad: Vec<u8>, plaintext: Vec<u8>) -> NotarySealRequest {
    NotarySealRequest {
        abi_version: WORK_WIRE_ABI_VERSION,
        aad,
        plaintext_hash: blake3_hash(&plaintext),
        plaintext,
    }
}

pub fn notary_unseal_request(aad: Vec<u8>, sealed_envelope: Vec<u8>) -> NotaryUnsealRequest {
    NotaryUnsealRequest {
        abi_version: WORK_WIRE_ABI_VERSION,
        aad,
        sealed_hash: notary_sealed_hash(&sealed_envelope),
        sealed_envelope,
    }
}

pub fn notary_delivery_report_preimage(value: &NotaryDeliveryReport) -> Vec<u8> {
    PreimageBuilder::domain(NOTARY_DELIVERY_REPORT_DOMAIN)
        .node(&value.notary)
        .node_id(&value.requester)
        .hash(&value.request_message_id)
        .hash(&value.request_payload_hash)
        .node_id(&value.via_relay)
        .u64(value.sequence)
        .hash(&value.aad_hash)
        .hash(&value.sealed_hash)
        .hash(&value.plaintext_hash)
        .u64(value.opened_unix_ms)
        .finish()
}

pub fn sign_notary_delivery_report(
    key: &Ed25519SigningKey,
    mut value: NotaryDeliveryReport,
) -> NotaryDeliveryReport {
    value.signature = sign_ed25519(key, &notary_delivery_report_preimage(&value));
    value
}

pub fn verify_notary_delivery_report(value: &NotaryDeliveryReport) -> bool {
    value.abi_version == WORK_WIRE_ABI_VERSION
        && value.notary.role == NODE_ROLE_NOTARY
        && verify_signature(
            &value.notary,
            &value.signature,
            &notary_delivery_report_preimage(value),
        )
}

#[derive(Clone, Debug)]
pub struct NotaryRole {
    signing_key: Ed25519SigningKey,
    seal_key: NotarySealKey,
    sealed_count: usize,
    opened_count: usize,
}

impl NotaryRole {
    pub fn new(signing_key: Ed25519SigningKey, seal_key: NotarySealKey) -> Self {
        Self {
            signing_key,
            seal_key,
            sealed_count: 0,
            opened_count: 0,
        }
    }

    pub fn identity(&self) -> NodeIdentity {
        node_identity_from_key(&self.signing_key, NODE_ROLE_NOTARY)
    }

    pub fn sealed_count(&self) -> usize {
        self.sealed_count
    }

    pub fn opened_count(&self) -> usize {
        self.opened_count
    }

    fn handle_seal(&mut self, message: &NetworkMessage, request: NotarySealRequest) -> RoleOutput {
        if request.abi_version != WORK_WIRE_ABI_VERSION
            || request.plaintext_hash != blake3_hash(&request.plaintext)
        {
            return RoleOutput::rejected(b"invalid notary seal request".to_vec());
        }
        let Ok(sealed) = seal_aes256_gcm(&self.seal_key, &request.aad, &request.plaintext) else {
            return RoleOutput::rejected(b"notary seal failed".to_vec());
        };
        let sealed_envelope = sealed.to_bytes();
        let response = NotarySealResponse {
            abi_version: WORK_WIRE_ABI_VERSION,
            notary: self.identity(),
            requester: message.from,
            aad_hash: notary_aad_hash(&request.aad),
            plaintext_hash: request.plaintext_hash,
            sealed_hash: notary_sealed_hash(&sealed_envelope),
            sealed_envelope,
        };
        let Ok(bytes) = notary_payload_bytes(&NotaryPayload::SealResponse(response)) else {
            return RoleOutput::rejected(b"notary seal response encode failed".to_vec());
        };
        self.sealed_count += 1;
        RoleOutput::accepted_bytes(bytes)
    }

    fn handle_unseal(
        &mut self,
        context: &RoleContext,
        message: &NetworkMessage,
        request: NotaryUnsealRequest,
    ) -> RoleOutput {
        if request.abi_version != WORK_WIRE_ABI_VERSION
            || request.sealed_hash != notary_sealed_hash(&request.sealed_envelope)
        {
            return RoleOutput::rejected(b"invalid notary unseal request".to_vec());
        }
        let Ok(sealed) = SealedBytes::from_bytes(&request.sealed_envelope) else {
            return RoleOutput::rejected(b"invalid sealed envelope".to_vec());
        };
        let Ok(plaintext) = unseal_aes256_gcm(&self.seal_key, &request.aad, &sealed) else {
            return RoleOutput::rejected(b"notary unseal failed".to_vec());
        };
        let plaintext_hash = blake3_hash(&plaintext);
        let report = sign_notary_delivery_report(
            &self.signing_key,
            NotaryDeliveryReport {
                abi_version: WORK_WIRE_ABI_VERSION,
                notary: self.identity(),
                requester: message.from,
                request_message_id: message.message_id,
                request_payload_hash: message.payload_hash,
                via_relay: message.via_relay,
                sequence: message.sequence,
                aad_hash: notary_aad_hash(&request.aad),
                sealed_hash: request.sealed_hash,
                plaintext_hash,
                opened_unix_ms: context.now_unix_ms,
                signature: empty_signature(),
            },
        );
        let response = NotaryUnsealResponse {
            abi_version: WORK_WIRE_ABI_VERSION,
            plaintext_hash,
            plaintext,
            delivery_report: report,
        };
        let Ok(bytes) = notary_payload_bytes(&NotaryPayload::UnsealResponse(response)) else {
            return RoleOutput::rejected(b"notary unseal response encode failed".to_vec());
        };
        self.opened_count += 1;
        RoleOutput::accepted_bytes(bytes)
    }
}

impl WorkRole for NotaryRole {
    fn role_id(&self) -> u16 {
        NODE_ROLE_NOTARY
    }

    fn accepts_department(&self, department: u16) -> bool {
        department == DEPARTMENT_NOTARY
    }

    fn accepts_work_type(&self, work_type: u16) -> bool {
        matches!(work_type, WORK_TYPE_NOTARY_SEAL | WORK_TYPE_NOTARY_UNSEAL)
    }

    fn handle(&mut self, context: &RoleContext, input: RoleInput) -> RoleOutput {
        let Some(message) = network_message_for_role(self, context, input) else {
            return RoleOutput::ignored();
        };
        let Ok(payload) = notary_payload_from_bytes(&message.payload) else {
            return RoleOutput::rejected(b"invalid notary payload".to_vec());
        };
        match (message.work_type, payload) {
            (WORK_TYPE_NOTARY_SEAL, NotaryPayload::SealRequest(request)) => {
                self.handle_seal(&message, request)
            }
            (WORK_TYPE_NOTARY_UNSEAL, NotaryPayload::UnsealRequest(request)) => {
                self.handle_unseal(context, &message, request)
            }
            _ => RoleOutput::rejected(b"notary payload does not match work type".to_vec()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::roles::{ROLE_STATUS_ACCEPTED, ROLE_STATUS_REJECTED};
    use crate::signing::sign_network_message_payload;

    fn message(
        client_key: &Ed25519SigningKey,
        notary: &NodeIdentity,
        work_type: u16,
        payload: NotaryPayload,
        sequence: u64,
    ) -> WorkPacket {
        let bytes = notary_payload_bytes(&payload).expect("payload bytes");
        WorkPacket::NetworkMessage(sign_network_message_payload(
            client_key,
            blake3_hash(&[sequence as u8]),
            [0u8; 32],
            *client_key.verifying_key().as_bytes(),
            notary.node_id,
            [9u8; 32],
            DEPARTMENT_NOTARY,
            work_type,
            sequence,
            bytes,
        ))
    }

    #[test]
    fn notary_seals_then_unseals_and_reports_plaintext_access() {
        let client_key = Ed25519SigningKey::from_bytes(&[11u8; 32]);
        let notary_key = Ed25519SigningKey::from_bytes(&[12u8; 32]);
        let seal_key = [13u8; 32];
        let mut role = NotaryRole::new(notary_key, seal_key);
        let identity = role.identity();
        let context = RoleContext {
            now_unix_ms: 1_000,
            local_node: identity.clone(),
            policy_hash: [7u8; 32],
        };

        let seal_packet = message(
            &client_key,
            &identity,
            WORK_TYPE_NOTARY_SEAL,
            NotaryPayload::SealRequest(notary_seal_request(
                b"vfs aad".to_vec(),
                b"file plaintext".to_vec(),
            )),
            1,
        );
        let seal_response = role.handle(
            &context,
            RoleInput {
                packet: seal_packet,
                previous_hash: [0u8; 32],
                channel_hash: [0u8; 32],
            },
        );
        assert_eq!(seal_response.status, ROLE_STATUS_ACCEPTED);
        let NotaryPayload::SealResponse(sealed) =
            notary_payload_from_bytes(&seal_response.bytes).expect("seal response")
        else {
            panic!("expected seal response");
        };
        assert_ne!(sealed.sealed_envelope, b"file plaintext");

        let unseal_payload = NotaryPayload::UnsealRequest(notary_unseal_request(
            b"vfs aad".to_vec(),
            sealed.sealed_envelope,
        ));
        let unseal_packet = message(
            &client_key,
            &identity,
            WORK_TYPE_NOTARY_UNSEAL,
            unseal_payload,
            2,
        );
        let unseal_response = role.handle(
            &context,
            RoleInput {
                packet: unseal_packet,
                previous_hash: [0u8; 32],
                channel_hash: [0u8; 32],
            },
        );
        assert_eq!(unseal_response.status, ROLE_STATUS_ACCEPTED);
        let NotaryPayload::UnsealResponse(opened) =
            notary_payload_from_bytes(&unseal_response.bytes).expect("unseal response")
        else {
            panic!("expected unseal response");
        };

        assert_eq!(opened.plaintext, b"file plaintext");
        assert_eq!(opened.plaintext_hash, blake3_hash(b"file plaintext"));
        assert!(verify_notary_delivery_report(&opened.delivery_report));
        assert_eq!(
            opened.delivery_report.requester,
            *client_key.verifying_key().as_bytes()
        );
        assert_eq!(opened.delivery_report.via_relay, [9u8; 32]);
        assert_eq!(role.sealed_count(), 1);
        assert_eq!(role.opened_count(), 1);
    }

    #[test]
    fn notary_rejects_wrong_aad_on_unseal() {
        let client_key = Ed25519SigningKey::from_bytes(&[21u8; 32]);
        let notary_key = Ed25519SigningKey::from_bytes(&[22u8; 32]);
        let mut role = NotaryRole::new(notary_key, [23u8; 32]);
        let identity = role.identity();
        let context = RoleContext {
            now_unix_ms: 2_000,
            local_node: identity.clone(),
            policy_hash: [0u8; 32],
        };

        let seal_packet = message(
            &client_key,
            &identity,
            WORK_TYPE_NOTARY_SEAL,
            NotaryPayload::SealRequest(notary_seal_request(b"right".to_vec(), b"secret".to_vec())),
            1,
        );
        let seal_response = role.handle(
            &context,
            RoleInput {
                packet: seal_packet,
                previous_hash: [0u8; 32],
                channel_hash: [0u8; 32],
            },
        );
        let NotaryPayload::SealResponse(sealed) =
            notary_payload_from_bytes(&seal_response.bytes).expect("seal response")
        else {
            panic!("expected seal response");
        };

        let unseal_packet = message(
            &client_key,
            &identity,
            WORK_TYPE_NOTARY_UNSEAL,
            NotaryPayload::UnsealRequest(notary_unseal_request(
                b"wrong".to_vec(),
                sealed.sealed_envelope,
            )),
            2,
        );
        let rejected = role.handle(
            &context,
            RoleInput {
                packet: unseal_packet,
                previous_hash: [0u8; 32],
                channel_hash: [0u8; 32],
            },
        );

        assert_eq!(rejected.status, ROLE_STATUS_REJECTED);
        assert_eq!(role.opened_count(), 0);
    }
}
