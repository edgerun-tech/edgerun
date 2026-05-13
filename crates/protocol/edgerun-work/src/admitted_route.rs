use alloc::vec::Vec;

use rkyv::{Archive, Deserialize, Serialize};

use crate::codec::packet_hash;
use crate::preimage::HashBuilder;
use crate::protocol::*;
use crate::request_auth::verify_work_request;
use crate::signing::{verify_work_admission, verify_work_receipt};

const ADMITTED_CAPABILITY_ROUTE_DOMAIN: &[u8] = b"edgerun:v1:work:admitted-capability-route";

#[derive(Clone, Debug, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[rkyv(crate = rkyv)]
pub struct AdmittedCapabilityRoute {
    pub abi_version: u16,
    pub route_id: Hash,
    pub request_hash: Hash,
    pub admission_hash: Hash,
    pub user: PublicKey,
    pub source_node_id: NodeId,
    pub target_node_id: NodeId,
    pub relay_node_id: NodeId,
    pub relay_path: Vec<NodeId>,
    pub role: u16,
    pub department: u16,
    pub work_type: u16,
    pub admission_route_commitment: Hash,
    pub target_route_commitment: Hash,
    pub policy_hash: Hash,
    pub admitted_budget: u64,
    pub valid_until_unix_ms: u64,
}

pub fn admitted_capability_route_id(value: &AdmittedCapabilityRoute) -> Hash {
    HashBuilder::domain(ADMITTED_CAPABILITY_ROUTE_DOMAIN)
        .hash(&value.request_hash)
        .hash(&value.admission_hash)
        .hash(&value.user)
        .node_id(&value.source_node_id)
        .node_id(&value.target_node_id)
        .node_id(&value.relay_node_id)
        .hash_list(&value.relay_path)
        .u16(value.role)
        .u16(value.department)
        .u16(value.work_type)
        .hash(&value.admission_route_commitment)
        .hash(&value.target_route_commitment)
        .hash(&value.policy_hash)
        .u64(value.admitted_budget)
        .u64(value.valid_until_unix_ms)
        .finish()
}

#[allow(clippy::too_many_arguments)]
pub fn admitted_capability_route_from_admission(
    request: &WorkRequest,
    admission: &WorkAdmission,
    source_node_id: NodeId,
    relay_node_id: NodeId,
    target_role: u16,
) -> Result<AdmittedCapabilityRoute, WorkProtocolError> {
    if request.abi_version != WORK_WIRE_ABI_VERSION
        || admission.abi_version != WORK_WIRE_ABI_VERSION
    {
        return Err(WorkProtocolError::InvalidShape);
    }
    if !verify_work_request(request) || !verify_work_admission(admission) {
        return Err(WorkProtocolError::InvalidSignature);
    }
    let request_hash = packet_hash(&WorkPacket::WorkRequest(request.clone()))?;
    if admission.request_hash != request_hash || admission.user != request.user {
        return Err(WorkProtocolError::HashMismatch);
    }
    if admission.valid_until_unix_ms > request.valid_until_unix_ms {
        return Err(WorkProtocolError::Expired);
    }
    if admission.assigned_relay_path.is_empty()
        || admission.assigned_relay_path[0] != relay_node_id
        || admission.assigned_relay_path.contains(&request.recipient)
    {
        return Err(WorkProtocolError::WrongRelay);
    }

    let admission_hash = packet_hash(&WorkPacket::WorkAdmission(admission.clone()))?;
    let mut admitted = AdmittedCapabilityRoute {
        abi_version: WORK_WIRE_ABI_VERSION,
        route_id: [0u8; 32],
        request_hash,
        admission_hash,
        user: request.user,
        source_node_id,
        target_node_id: request.recipient,
        relay_node_id,
        relay_path: admission.assigned_relay_path.clone(),
        role: target_role,
        department: request.department,
        work_type: request.work_type,
        admission_route_commitment: admission.assigned_route_commitment,
        target_route_commitment: admission.assigned_route_commitment,
        policy_hash: admission.policy_hash,
        admitted_budget: admission.admitted_budget,
        valid_until_unix_ms: admission.valid_until_unix_ms,
    };
    admitted.route_id = admitted_capability_route_id(&admitted);
    Ok(admitted)
}

pub fn verify_admission_defined_route(
    value: &AdmittedCapabilityRoute,
    request: &WorkRequest,
    admission: &WorkAdmission,
) -> Result<(), WorkProtocolError> {
    let expected = admitted_capability_route_from_admission(
        request,
        admission,
        value.source_node_id,
        value.relay_node_id,
        value.role,
    )?;
    if &expected != value || value.route_id != admitted_capability_route_id(value) {
        return Err(WorkProtocolError::HashMismatch);
    }
    Ok(())
}

pub fn verify_message_against_admitted_route(
    message: &NetworkMessage,
    route: &AdmittedCapabilityRoute,
) -> Result<(), WorkProtocolError> {
    if message.abi_version != WORK_WIRE_ABI_VERSION {
        return Err(WorkProtocolError::InvalidShape);
    }
    if message.from != route.source_node_id
        || message.to != route.target_node_id
        || message.via_relay != route.relay_node_id
        || route.relay_path.first() != Some(&route.relay_node_id)
        || message.department != route.department
        || message.work_type != route.work_type
    {
        return Err(WorkProtocolError::WrongRelay);
    }
    Ok(())
}

pub fn verify_receipt_against_admitted_route(
    receipt: &WorkReceipt,
    route: &AdmittedCapabilityRoute,
) -> Result<(), WorkProtocolError> {
    if !verify_work_receipt(receipt) {
        return Err(WorkProtocolError::InvalidSignature);
    }
    if receipt.request_hash != route.request_hash
        || receipt.admission_hash != route.admission_hash
        || receipt.worker.node_id != route.target_node_id
        || receipt.worker.role != route.role
        || receipt.relay_node_id != route.relay_node_id
    {
        return Err(WorkProtocolError::HashMismatch);
    }
    if receipt.total_claim > route.admitted_budget {
        return Err(WorkProtocolError::Unsupported);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;
    use edgerun_crypto::Ed25519SigningKey;

    use crate::channel::ChannelEndpoint;
    use crate::identity::node_identity_from_key;
    use crate::request_auth::sign_work_request;
    use crate::settlement::receipt_id_for_claim;
    use crate::signing::{empty_signature, sign_work_admission, sign_work_receipt};

    fn signed_storage_request_and_admission(
        user_key: &Ed25519SigningKey,
        admission_key: &Ed25519SigningKey,
        admission_node: NodeIdentity,
        storage_node_id: NodeId,
        relay_path: Vec<NodeId>,
    ) -> (WorkRequest, WorkAdmission) {
        let user = *user_key.verifying_key().as_bytes();
        let request = sign_work_request(
            user_key,
            WorkRequest {
                abi_version: WORK_WIRE_ABI_VERSION,
                request_id: [1u8; 32],
                user,
                user_sequence: 1,
                recipient: storage_node_id,
                work_type: WORK_TYPE_OBJECT_STORE,
                department: DEPARTMENT_STORAGE,
                payload_hash: [2u8; 32],
                input_root: [3u8; 32],
                max_total_cost: 10,
                valid_until_unix_ms: 100_000,
                signature: empty_signature(),
            },
        );
        let request_hash =
            packet_hash(&WorkPacket::WorkRequest(request.clone())).expect("request hash");
        let admission = sign_work_admission(
            admission_key,
            WorkAdmission {
                abi_version: WORK_WIRE_ABI_VERSION,
                admission_id: [4u8; 32],
                dao_id: [5u8; 32],
                user,
                admission_node,
                request_hash,
                assigned_route_commitment: [6u8; 32],
                assigned_channel: ChannelEndpoint::new(
                    [11u8; 32],
                    crate::channel::CHANNEL_KIND_TCP,
                    vec![],
                    "relay".into(),
                ),
                assigned_relay_path: relay_path,
                admitted_budget: 10,
                policy_hash: [12u8; 32],
                sequence: 1,
                valid_until_unix_ms: 100_000,
                signature: empty_signature(),
            },
        );
        (request, admission)
    }

    #[test]
    fn admission_defined_route_does_not_require_node_binding() {
        let user_key = Ed25519SigningKey::from_bytes(&[17u8; 32]);
        let admission_key = Ed25519SigningKey::from_bytes(&[18u8; 32]);
        let relay_key = Ed25519SigningKey::from_bytes(&[19u8; 32]);
        let storage_key = Ed25519SigningKey::from_bytes(&[20u8; 32]);
        let admission_node = node_identity_from_key(&admission_key, NODE_ROLE_ADMISSION);
        let relay_node = node_identity_from_key(&relay_key, NODE_ROLE_RELAY);
        let storage_node = node_identity_from_key(&storage_key, NODE_ROLE_STORAGE);
        let (request, admission) = signed_storage_request_and_admission(
            &user_key,
            &admission_key,
            admission_node,
            storage_node.node_id,
            vec![relay_node.node_id],
        );

        let admitted = admitted_capability_route_from_admission(
            &request,
            &admission,
            [21u8; 32],
            relay_node.node_id,
            NODE_ROLE_STORAGE,
        )
        .expect("admission route");
        verify_admission_defined_route(&admitted, &request, &admission)
            .expect("admission route verifies");

        assert_eq!(admitted.target_node_id, storage_node.node_id);
        assert_eq!(admitted.relay_node_id, relay_node.node_id);
        assert_eq!(
            admitted.target_route_commitment,
            admission.assigned_route_commitment
        );
    }

    #[test]
    fn admission_defined_route_can_bind_multiple_relays() {
        let user_key = Ed25519SigningKey::from_bytes(&[22u8; 32]);
        let admission_key = Ed25519SigningKey::from_bytes(&[23u8; 32]);
        let relay_a_key = Ed25519SigningKey::from_bytes(&[24u8; 32]);
        let relay_b_key = Ed25519SigningKey::from_bytes(&[25u8; 32]);
        let storage_key = Ed25519SigningKey::from_bytes(&[26u8; 32]);
        let admission_node = node_identity_from_key(&admission_key, NODE_ROLE_ADMISSION);
        let relay_a = node_identity_from_key(&relay_a_key, NODE_ROLE_RELAY);
        let relay_b = node_identity_from_key(&relay_b_key, NODE_ROLE_RELAY);
        let storage_node = node_identity_from_key(&storage_key, NODE_ROLE_STORAGE);
        let relay_path = vec![relay_a.node_id, relay_b.node_id];
        let (request, admission) = signed_storage_request_and_admission(
            &user_key,
            &admission_key,
            admission_node,
            storage_node.node_id,
            relay_path.clone(),
        );

        let admitted = admitted_capability_route_from_admission(
            &request,
            &admission,
            [27u8; 32],
            relay_a.node_id,
            NODE_ROLE_STORAGE,
        )
        .expect("multi relay route");

        assert_eq!(admitted.relay_node_id, relay_a.node_id);
        assert_eq!(admitted.relay_path, relay_path);
        verify_admission_defined_route(&admitted, &request, &admission)
            .expect("multi relay route verifies");

        let message = NetworkMessage {
            abi_version: WORK_WIRE_ABI_VERSION,
            message_id: [28u8; 32],
            prev_hash: [0u8; 32],
            from: admitted.source_node_id,
            to: admitted.target_node_id,
            via_relay: relay_a.node_id,
            department: admitted.department,
            work_type: admitted.work_type,
            sequence: 1,
            payload_hash: [2u8; 32],
            payload: vec![],
            signature: empty_signature(),
        };
        verify_message_against_admitted_route(&message, &admitted).expect("first relay accepted");

        let mut tampered = admitted.clone();
        tampered.relay_path = vec![relay_b.node_id, relay_a.node_id];
        assert!(verify_admission_defined_route(&tampered, &request, &admission).is_err());
    }

    #[test]
    fn admitted_route_binds_request_admission_message_and_receipt() {
        let user_key = Ed25519SigningKey::from_bytes(&[7u8; 32]);
        let admission_key = Ed25519SigningKey::from_bytes(&[8u8; 32]);
        let relay_key = Ed25519SigningKey::from_bytes(&[9u8; 32]);
        let storage_key = Ed25519SigningKey::from_bytes(&[10u8; 32]);
        let user = *user_key.verifying_key().as_bytes();
        let admission_node = node_identity_from_key(&admission_key, NODE_ROLE_ADMISSION);
        let relay_node = node_identity_from_key(&relay_key, NODE_ROLE_RELAY);
        let storage_node = node_identity_from_key(&storage_key, NODE_ROLE_STORAGE);

        let request = sign_work_request(
            &user_key,
            WorkRequest {
                abi_version: WORK_WIRE_ABI_VERSION,
                request_id: [1u8; 32],
                user,
                user_sequence: 1,
                recipient: storage_node.node_id,
                work_type: WORK_TYPE_OBJECT_STORE,
                department: DEPARTMENT_STORAGE,
                payload_hash: [2u8; 32],
                input_root: [3u8; 32],
                max_total_cost: 10,
                valid_until_unix_ms: 100_000,
                signature: empty_signature(),
            },
        );
        let request_hash =
            packet_hash(&WorkPacket::WorkRequest(request.clone())).expect("request hash");
        let admission = sign_work_admission(
            &admission_key,
            WorkAdmission {
                abi_version: WORK_WIRE_ABI_VERSION,
                admission_id: [4u8; 32],
                dao_id: [5u8; 32],
                user,
                admission_node,
                request_hash,
                assigned_route_commitment: [6u8; 32],
                assigned_channel: ChannelEndpoint::new(
                    [11u8; 32],
                    crate::channel::CHANNEL_KIND_TCP,
                    vec![],
                    "relay".into(),
                ),
                assigned_relay_path: vec![relay_node.node_id],
                admitted_budget: 10,
                policy_hash: [12u8; 32],
                sequence: 1,
                valid_until_unix_ms: 100_000,
                signature: empty_signature(),
            },
        );
        let admitted = admitted_capability_route_from_admission(
            &request,
            &admission,
            [13u8; 32],
            relay_node.node_id,
            NODE_ROLE_STORAGE,
        )
        .expect("admitted route");
        verify_admission_defined_route(&admitted, &request, &admission).expect("verify route");

        let message = NetworkMessage {
            abi_version: WORK_WIRE_ABI_VERSION,
            message_id: [14u8; 32],
            prev_hash: [0u8; 32],
            from: admitted.source_node_id,
            to: admitted.target_node_id,
            via_relay: admitted.relay_node_id,
            department: admitted.department,
            work_type: admitted.work_type,
            sequence: 1,
            payload_hash: [2u8; 32],
            payload: vec![],
            signature: empty_signature(),
        };
        verify_message_against_admitted_route(&message, &admitted).expect("message route");

        let input_hash = [15u8; 32];
        let output_hash = [16u8; 32];
        let admission_hash =
            packet_hash(&WorkPacket::WorkAdmission(admission.clone())).expect("admission hash");
        let receipt = sign_work_receipt(
            &storage_key,
            WorkReceipt {
                abi_version: WORK_WIRE_ABI_VERSION,
                receipt_id: receipt_id_for_claim(
                    admitted.request_hash,
                    admission_hash,
                    storage_node.node_id,
                    input_hash,
                    output_hash,
                    1,
                ),
                request_hash: admitted.request_hash,
                admission_hash,
                worker: storage_node,
                relay_node_id: admitted.relay_node_id,
                input_hash,
                output_hash,
                units_used: 1,
                total_claim: 5,
                sequence: 1,
                signature: empty_signature(),
            },
        );
        verify_receipt_against_admitted_route(&receipt, &admitted).expect("receipt route");
    }
}
