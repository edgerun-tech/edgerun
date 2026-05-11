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
    pub route_hash: Hash,
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
        .hash(&value.route_hash)
        .hash(&value.policy_hash)
        .u64(value.admitted_budget)
        .u64(value.valid_until_unix_ms)
        .finish()
}

pub fn admitted_capability_route_from_parts(
    request: &WorkRequest,
    admission: &WorkAdmission,
    route: &RouteAdvertisement,
    source_node_id: NodeId,
) -> Result<AdmittedCapabilityRoute, WorkProtocolError> {
    if request.abi_version != WORK_WIRE_ABI_VERSION || admission.abi_version != WORK_WIRE_ABI_VERSION {
        return Err(WorkProtocolError::InvalidShape);
    }
    if !verify_work_admission(admission) || !verify_available_route_advertisement(route) {
        return Err(WorkProtocolError::InvalidSignature);
    }
    let request_hash = packet_hash(&WorkPacket::WorkRequest(request.clone()))?;
    if admission.request_hash != request_hash {
        return Err(WorkProtocolError::HashMismatch);
    }
    if admission.user != request.user {
        return Err(WorkProtocolError::InvalidShape);
    }
    if request.recipient != route.node.node_id {
        return Err(WorkProtocolError::UnknownNode);
    }
    if !route.roles.contains(&route.node.role) || !route.departments.contains(&request.department) {
        return Err(WorkProtocolError::Unsupported);
    }
    if route.valid_until_unix_ms < admission.valid_until_unix_ms {
        return Err(WorkProtocolError::Expired);
    }
    if route.relay_node_id == route.node.node_id {
        return Err(WorkProtocolError::WrongRelay);
    }
    if admission.assigned_route_hash != crate::memory_channel::route_hash(route) {
        return Err(WorkProtocolError::HashMismatch);
    }

    let admission_hash = packet_hash(&WorkPacket::WorkAdmission(admission.clone()))?;
    let mut admitted = AdmittedCapabilityRoute {
        abi_version: WORK_WIRE_ABI_VERSION,
        route_id: [0u8; 32],
        request_hash,
        admission_hash,
        user: request.user,
        source_node_id,
        target_node_id: route.node.node_id,
        relay_node_id: route.relay_node_id,
        role: route.node.role,
        department: request.department,
        work_type: request.work_type,
        route_hash: admission.assigned_route_hash,
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
    route: &RouteAdvertisement,
) -> Result<(), WorkProtocolError> {
    let expected = admitted_capability_route_from_parts(request, admission, route, value.source_node_id)?;
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
