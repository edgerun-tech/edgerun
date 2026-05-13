use rkyv::{Archive, Deserialize, Serialize};

use crate::channel::RouteAdvertisement;
use crate::codec::packet_hash;
use crate::preimage::HashBuilder;
use crate::protocol::*;
use crate::route_auth::verify_available_route_advertisement;
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
    pub role: u16,
    pub department: u16,
    pub work_type: u16,
    pub admission_relay_route_hash: Hash,
    pub worker_route_hash: Hash,
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
        .u16(value.role)
        .u16(value.department)
        .u16(value.work_type)
        .hash(&value.admission_relay_route_hash)
        .hash(&value.worker_route_hash)
        .hash(&value.policy_hash)
        .u64(value.admitted_budget)
        .u64(value.valid_until_unix_ms)
        .finish()
}

pub fn admitted_capability_route_from_parts(
    request: &WorkRequest,
    admission: &WorkAdmission,
    worker_route: &RouteAdvertisement,
    source_node_id: NodeId,
) -> Result<AdmittedCapabilityRoute, WorkProtocolError> {
    if request.abi_version != WORK_WIRE_ABI_VERSION
        || admission.abi_version != WORK_WIRE_ABI_VERSION
    {
        return Err(WorkProtocolError::InvalidShape);
    }
    if !verify_work_admission(admission) || !verify_available_route_advertisement(worker_route) {
        return Err(WorkProtocolError::InvalidSignature);
    }
    let request_hash = packet_hash(&WorkPacket::WorkRequest(request.clone()))?;
    if admission.request_hash != request_hash {
        return Err(WorkProtocolError::HashMismatch);
    }
    if admission.user != request.user {
        return Err(WorkProtocolError::InvalidShape);
    }
    if request.recipient != worker_route.node.node_id {
        return Err(WorkProtocolError::UnknownNode);
    }
    if !worker_route.roles.contains(&worker_route.node.role)
        || !worker_route.departments.contains(&request.department)
    {
        return Err(WorkProtocolError::Unsupported);
    }
    if worker_route.valid_until_unix_ms < admission.valid_until_unix_ms {
        return Err(WorkProtocolError::Expired);
    }
    if worker_route.relay_node_id == worker_route.node.node_id {
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
        target_node_id: worker_route.node.node_id,
        relay_node_id: worker_route.relay_node_id,
        role: worker_route.node.role,
        department: request.department,
        work_type: request.work_type,
        admission_relay_route_hash: admission.assigned_route_hash,
        worker_route_hash: crate::route_auth::route_hash(worker_route),
        policy_hash: admission.policy_hash,
        admitted_budget: admission.admitted_budget,
        valid_until_unix_ms: admission.valid_until_unix_ms,
    };
    admitted.route_id = admitted_capability_route_id(&admitted);
    Ok(admitted)
}

pub fn verify_admitted_capability_route(
    value: &AdmittedCapabilityRoute,
    request: &WorkRequest,
    admission: &WorkAdmission,
    worker_route: &RouteAdvertisement,
) -> Result<(), WorkProtocolError> {
    let expected = admitted_capability_route_from_parts(
        request,
        admission,
        worker_route,
        value.source_node_id,
    )?;
    if &expected != value {
        return Err(WorkProtocolError::HashMismatch);
    }
    if value.route_id != admitted_capability_route_id(value) {
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
    use crate::route_builder::{storage_route_from_relay_assignment, tcp_endpoint};
    use crate::settlement::receipt_id_for_claim;
    use crate::signing::{
        empty_signature, sign_relay_assignment, sign_work_admission, sign_work_receipt,
    };

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
        let assignment = sign_relay_assignment(
            &admission_key,
            RelayAssignment {
                abi_version: WORK_WIRE_ABI_VERSION,
                node_id: storage_node.node_id,
                relay: RelayEndpoint {
                    relay_node_id: relay_node.node_id,
                    host: "127.0.0.1".into(),
                    port: 9000,
                },
                assigned_by: admission_node.clone(),
                sequence: 1,
                valid_until_unix_ms: 100_000,
                signature: empty_signature(),
            },
        );
        let worker_route = storage_route_from_relay_assignment(
            &storage_key,
            &assignment,
            tcp_endpoint("storage", "127.0.0.1:9001"),
        )
        .expect("worker route");
        let admission = sign_work_admission(
            &admission_key,
            WorkAdmission {
                abi_version: WORK_WIRE_ABI_VERSION,
                admission_id: [4u8; 32],
                dao_id: [5u8; 32],
                user,
                admission_node,
                request_hash,
                assigned_route_hash: [6u8; 32],
                assigned_channel: ChannelEndpoint::new(
                    [11u8; 32],
                    crate::channel::CHANNEL_KIND_TCP,
                    vec![],
                    "relay".into(),
                ),
                admitted_budget: 10,
                policy_hash: [12u8; 32],
                sequence: 1,
                valid_until_unix_ms: 100_000,
                signature: empty_signature(),
            },
        );
        let admitted =
            admitted_capability_route_from_parts(&request, &admission, &worker_route, [13u8; 32])
                .expect("admitted route");
        verify_admitted_capability_route(&admitted, &request, &admission, &worker_route)
            .expect("verify route");

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
