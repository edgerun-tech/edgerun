#![cfg_attr(not(feature = "std"), no_std)]

//! Edgerun wire protocol boundary.
//!
//! The only supported internal wire protocol is rkyv. Legacy structural wire
//! APIs have intentionally been removed so old call sites fail at compile time
//! instead of continuing on a second wire format.

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

pub const WIRE_PROTOCOL: &str = "rkyv";

pub use rkyv::rancor::Error as WireError;
pub use rkyv::*;

pub const SDK_WIRE_ABI_VERSION: u16 = 2;

pub const SIGNATURE_ALGORITHM_ED25519: u16 = 1;
pub const SIGNATURE_ALGORITHM_ECDSA_P256_SHA256: u16 = 2;

pub const RELAY_WIRE_ABI_VERSION: u16 = 1;
pub const RELAY_DELIVERY_STATUS_ACCEPTED: u16 = 1;
pub const RELAY_DELIVERY_STATUS_REJECTED: u16 = 2;
pub const RELAY_REPORT_STATUS_ACCEPTED: u16 = 1;
pub const RELAY_REPORT_STATUS_REJECTED: u16 = 2;

pub const CAPABILITY_KIND_SIGNING: u16 = 1;
pub const CAPABILITY_KIND_SEALING: u16 = 2;
pub const CAPABILITY_KIND_STORAGE: u16 = 4;
pub const CAPABILITY_KIND_NETWORK: u16 = 5;

pub const CAPABILITY_OPERATION_SIGN: u16 = 1;
pub const CAPABILITY_OPERATION_VERIFY: u16 = 2;
pub const CAPABILITY_OPERATION_SEAL: u16 = 3;
pub const CAPABILITY_OPERATION_UNSEAL: u16 = 4;
pub const CAPABILITY_OPERATION_READ: u16 = 6;
pub const CAPABILITY_OPERATION_WRITE: u16 = 7;
pub const CAPABILITY_OPERATION_RECEIVE: u16 = 9;

pub const CAPABILITY_STATUS_OK: u16 = 0;
pub const CAPABILITY_STATUS_POLICY_DENIED: u16 = 1;
pub const CAPABILITY_STATUS_INVALID_REQUEST: u16 = 2;
pub const CAPABILITY_STATUS_PROVIDER_FAILED: u16 = 3;

pub const USER_PROFILE_KDF_NONE: u16 = 0;
pub const USER_PROFILE_KDF_PBKDF2_HMAC_SHA256: u16 = 1;
pub const USER_PROFILE_OWNER_KEY_ED25519: u16 = 1;

pub const RUNTIME_EVENT_PROFILE_OPENED: u16 = 1;
pub const RUNTIME_EVENT_CAPABILITY_DENIED: u16 = 4;
pub const RUNTIME_EVENT_CAPABILITY_EXECUTED: u16 = 5;
pub const RUNTIME_EVENT_APP_INSTALLED: u16 = 6;
pub const RUNTIME_EVENT_ROUTE_GRANTED: u16 = 7;
pub const RUNTIME_EVENT_HTTP_DISPATCHED: u16 = 8;
pub const RUNTIME_EVENT_APP_MESSAGE_DISPATCHED: u16 = 9;
pub const RUNTIME_EVENT_IDENTITY_ROUTE_GRANTED: u16 = 10;
pub const RUNTIME_EVENT_APP_MESSAGE_FORWARDED: u16 = 11;
pub const RUNTIME_EVENT_CAPABILITY_GRANTED: u16 = 12;
pub const RUNTIME_EVENT_STORAGE_BOUND: u16 = 13;
pub const RUNTIME_EVENT_NETWORK_BOUND: u16 = 14;
pub const RUNTIME_EVENT_CAPABILITY_SESSION_OPENED: u16 = 15;
pub const RUNTIME_EVENT_NODE_INSTANCE_STARTED: u16 = 16;
pub const RUNTIME_EVENT_ADMISSION_POLICY_SELECTED: u16 = 17;
pub const RUNTIME_EVENT_PACKAGE_RETRIEVED: u16 = 18;
pub const RUNTIME_EVENT_PACKAGE_VERIFIED: u16 = 19;
pub const RUNTIME_EVENT_APP_RUN_PROMPT_DECIDED: u16 = 20;
pub const RUNTIME_EVENT_PACKAGE_CACHE_UPDATED: u16 = 21;

pub const NODE_ROLE_BROWSER: u16 = 100;
pub const NODE_ROLE_ADMISSION: u16 = 101;
pub const NODE_ROLE_RELAY: u16 = 102;
pub const NODE_ROLE_STORAGE: u16 = 103;
pub const NODE_ROLE_COMPUTE: u16 = 104;
pub const NODE_ROLE_CAPABILITY: u16 = 105;
pub const NODE_ROLE_PUBLISHER: u16 = 106;
pub const NODE_ROLE_WALLET: u16 = 107;

pub const RUNTIME_TARGET_BROWSER_WASM: u16 = 1;
pub const RUNTIME_TARGET_NATIVE_STD: u16 = 2;
pub const RUNTIME_TARGET_WASM_WORKER: u16 = 3;
pub const RUNTIME_TARGET_FIRMWARE: u16 = 4;

pub const NODE_INSTANCE_STATUS_PENDING: u16 = 1;
pub const NODE_INSTANCE_STATUS_RUNNING: u16 = 2;
pub const NODE_INSTANCE_STATUS_STOPPED: u16 = 3;
pub const NODE_INSTANCE_STATUS_REVOKED: u16 = 4;

pub const ADMISSION_POLICY_SOURCE_DAO: u16 = 1;
pub const ADMISSION_POLICY_SOURCE_USER: u16 = 2;
pub const ADMISSION_POLICY_SOURCE_INHERITED: u16 = 3;

pub const APP_RUN_DECISION_RUN_ONCE: u16 = 1;
pub const APP_RUN_DECISION_VERIFY_AND_CACHE: u16 = 2;
pub const APP_RUN_DECISION_CANCEL: u16 = 3;

pub const PACKAGE_CACHE_STATE_NONE: u16 = 0;
pub const PACKAGE_CACHE_STATE_VERIFIED: u16 = 1;
pub const PACKAGE_CACHE_STATE_REMOVED: u16 = 2;
pub const PACKAGE_CACHE_STATE_INVALID: u16 = 3;

pub const RUNTIME_STORAGE_BACKING_MEMORY: u16 = 1;
pub const RUNTIME_STORAGE_BACKING_NATIVE: u16 = 2;
pub const RUNTIME_STORAGE_BACKING_BROWSER: u16 = 3;
pub const RUNTIME_STORAGE_BACKING_REMOTE: u16 = 4;

pub const RUNTIME_NETWORK_BINDING_FETCH: u16 = 1;
pub const RUNTIME_NETWORK_BINDING_SOCKET: u16 = 2;
pub const RUNTIME_NETWORK_BINDING_NODE_MESSAGE: u16 = 3;

pub const RUNTIME_SESSION_STATUS_OPEN: u16 = 1;
pub const RUNTIME_SESSION_STATUS_CLOSED: u16 = 2;
pub const RUNTIME_SESSION_STATUS_REVOKED: u16 = 3;

pub const HTTP_METHOD_GET: u16 = 1;

pub const ROUTE_SCHEME_HTTP: u16 = 1;
pub const ROUTE_SCHEME_HTTPS: u16 = 2;

pub const APP_MESSAGE_STATUS_ACCEPTED: u16 = 0;
pub const APP_MESSAGE_STATUS_DENIED: u16 = 1;
pub const APP_MESSAGE_STATUS_FORWARDED: u16 = 2;

pub const RUNTIME_PROTOCOL_HTTP: u16 = 1;
pub const RUNTIME_PROTOCOL_HTTPS: u16 = 2;
pub const RUNTIME_PROTOCOL_DNS_UDP: u16 = 3;
pub const RUNTIME_PROTOCOL_DNS_TCP: u16 = 4;
pub const RUNTIME_PROTOCOL_SMTP: u16 = 5;
pub const RUNTIME_PROTOCOL_SUBMISSION: u16 = 6;
pub const RUNTIME_PROTOCOL_IMAP: u16 = 7;
pub const RUNTIME_PROTOCOL_IMAPS: u16 = 8;
pub const RUNTIME_PROTOCOL_LMTP: u16 = 9;
pub const RUNTIME_PROTOCOL_TFTP: u16 = 10;
pub const RUNTIME_PROTOCOL_PROXY: u16 = 11;
pub const RUNTIME_PROTOCOL_ACME: u16 = 12;

pub const APP_STORE_SUBMISSION_STATUS_SUBMITTED: u16 = 1;
pub const APP_STORE_SUBMISSION_STATUS_PUBLISHED: u16 = 4;

pub const APP_STORE_REVIEW_DECISION_ACCEPT: u16 = 1;

#[derive(Clone, Debug, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[rkyv(crate = rkyv)]
pub struct CapabilityRequest {
    pub abi_version: u16,
    pub flags: u32,
    pub capability_kind: u16,
    pub operation: u16,
    pub assurance: u16,
    pub app_id: [u8; 32],
    pub release_id: [u8; 32],
    pub subject_sha256: [u8; 32],
    pub payload_sha256: [u8; 32],
    pub context: Vec<u8>,
    pub payload: Vec<u8>,
    pub nonce: Vec<u8>,
}

impl CapabilityRequest {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        capability_kind: u16,
        operation: u16,
        assurance: u16,
        app_id: [u8; 32],
        release_id: [u8; 32],
        subject_sha256: [u8; 32],
        payload_sha256: [u8; 32],
        context: Vec<u8>,
        payload: Vec<u8>,
        nonce: Vec<u8>,
    ) -> Self {
        Self {
            abi_version: SDK_WIRE_ABI_VERSION,
            flags: 1,
            capability_kind,
            operation,
            assurance,
            app_id,
            release_id,
            subject_sha256,
            payload_sha256,
            context,
            payload,
            nonce,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[rkyv(crate = rkyv)]
pub struct CapabilityResponse {
    pub abi_version: u16,
    pub flags: u32,
    pub capability_kind: u16,
    pub operation: u16,
    pub status: u16,
    pub assurance: u16,
    pub request_sha256: [u8; 32],
    pub provider: Vec<u8>,
    pub responder: Vec<u8>,
    pub payload: Vec<u8>,
    pub proof: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[rkyv(crate = rkyv)]
pub struct CapabilityResponseProofRecord {
    pub domain: Vec<u8>,
    pub request_sha256: [u8; 32],
    pub operation_or_status: u16,
    pub provider: Vec<u8>,
    pub payload_sha256: [u8; 32],
}

#[derive(Clone, Debug, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[rkyv(crate = rkyv)]
pub struct StorageWriteReceiptRecord {
    pub payload_sha256: [u8; 32],
    pub payload_len: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[rkyv(crate = rkyv)]
pub struct SigningResponsePayloadRecord {
    pub public_key: Vec<u8>,
    pub signature: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[rkyv(crate = rkyv)]
pub struct SigningAlgorithmRecord {
    pub algorithm: u16,
}

#[derive(Clone, Debug, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[rkyv(crate = rkyv)]
pub struct SigningCapabilityInputRecord {
    pub domain: Vec<u8>,
    pub app_id: [u8; 32],
    pub release_id: [u8; 32],
    pub subject_sha256: [u8; 32],
    pub payload_sha256: [u8; 32],
    pub payload: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[rkyv(crate = rkyv)]
pub struct UserProfileIdSeedRecord {
    pub domain: Vec<u8>,
    pub owner_id: [u8; 32],
    pub epoch: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Archive, Serialize, Deserialize)]
#[rkyv(crate = rkyv)]
pub struct RelayIdentity {
    pub algorithm: u16,
    pub public_key: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[rkyv(crate = rkyv)]
pub struct RelaySignature {
    pub algorithm: u16,
    pub public_key: Vec<u8>,
    pub signature: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[rkyv(crate = rkyv)]
pub struct RelayRegister {
    pub abi_version: u16,
    pub flags: u32,
    pub node: RelayIdentity,
    pub sequence: u64,
    pub log_head: [u8; 32],
    pub signature: RelaySignature,
}

#[derive(Clone, Debug, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[rkyv(crate = rkyv)]
pub struct RelaySubmit {
    pub abi_version: u16,
    pub flags: u32,
    pub message_id: [u8; 32],
    pub from: RelayIdentity,
    pub to: RelayIdentity,
    pub sequence: u64,
    pub payload_sha256: [u8; 32],
    pub payload: Vec<u8>,
    pub signature: RelaySignature,
}

#[derive(Clone, Debug, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[rkyv(crate = rkyv)]
pub struct RelayDeliveryRequest {
    pub abi_version: u16,
    pub flags: u32,
    pub relay_id: Vec<u8>,
    pub submit: RelaySubmit,
    pub received_unix_ms: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[rkyv(crate = rkyv)]
pub struct RelayDeliveryReceipt {
    pub abi_version: u16,
    pub flags: u32,
    pub message_id: [u8; 32],
    pub recipient: RelayIdentity,
    pub status: u16,
    pub recipient_sequence: u64,
    pub recipient_log_head: [u8; 32],
    pub request_sha256: [u8; 32],
    pub signature: RelaySignature,
}

#[derive(Clone, Debug, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[rkyv(crate = rkyv)]
pub struct RelayDeliveryReport {
    pub abi_version: u16,
    pub flags: u32,
    pub relay_id: Vec<u8>,
    pub submit: RelaySubmit,
    pub recipient_receipt: RelayDeliveryReceipt,
    pub reported_unix_ms: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[rkyv(crate = rkyv)]
pub struct RelayDeliveryReportReceipt {
    pub abi_version: u16,
    pub flags: u32,
    pub message_id: [u8; 32],
    pub sender: RelayIdentity,
    pub status: u16,
    pub sender_sequence: u64,
    pub sender_log_head: [u8; 32],
    pub report_sha256: [u8; 32],
    pub signature: RelaySignature,
}

#[derive(Clone, Debug, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[rkyv(crate = rkyv)]
pub struct RelayAck {
    pub ok: bool,
    pub code: u16,
    pub text: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[rkyv(crate = rkyv)]
pub enum RelayMessage {
    Register(RelayRegister),
    Submit(RelaySubmit),
    DeliveryRequest(RelayDeliveryRequest),
    DeliveryReceipt(RelayDeliveryReceipt),
    DeliveryReport(RelayDeliveryReport),
    DeliveryReportReceipt(RelayDeliveryReportReceipt),
    Ack(RelayAck),
}

pub fn relay_message_bytes(message: &RelayMessage) -> Result<Vec<u8>, WireError> {
    to_bytes::<WireError>(message).map(|bytes| bytes.to_vec())
}

pub fn relay_message_from_bytes(bytes: &[u8]) -> Result<RelayMessage, WireError> {
    if bytes
        .as_ptr()
        .align_offset(core::mem::align_of::<ArchivedRelayMessage>())
        != 0
    {
        let mut aligned = util::AlignedVec::<16>::with_capacity(bytes.len());
        aligned.extend_from_slice(bytes);
        return relay_message_from_aligned_bytes(aligned.as_slice());
    }

    relay_message_from_aligned_bytes(bytes)
}

fn relay_message_from_aligned_bytes(bytes: &[u8]) -> Result<RelayMessage, WireError> {
    let archived = access::<ArchivedRelayMessage, WireError>(bytes)?;
    deserialize::<RelayMessage, WireError>(archived)
}

impl CapabilityResponse {
    #[allow(clippy::too_many_arguments)]
    pub fn ok(
        request_sha256: [u8; 32],
        capability_kind: u16,
        operation: u16,
        assurance: u16,
        provider: Vec<u8>,
        responder: Vec<u8>,
        payload: Vec<u8>,
        proof: Vec<u8>,
    ) -> Self {
        Self {
            abi_version: SDK_WIRE_ABI_VERSION,
            flags: 1,
            capability_kind,
            operation,
            status: CAPABILITY_STATUS_OK,
            assurance,
            request_sha256,
            provider,
            responder,
            payload,
            proof,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn denied(
        request_sha256: [u8; 32],
        capability_kind: u16,
        operation: u16,
        assurance: u16,
        provider: Vec<u8>,
        responder: Vec<u8>,
        reason: Vec<u8>,
        proof: Vec<u8>,
    ) -> Self {
        Self {
            abi_version: SDK_WIRE_ABI_VERSION,
            flags: 1,
            capability_kind,
            operation,
            status: CAPABILITY_STATUS_POLICY_DENIED,
            assurance,
            request_sha256,
            provider,
            responder,
            payload: reason,
            proof,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[rkyv(crate = rkyv)]
pub struct UserProfile {
    pub abi_version: u16,
    pub flags: u32,
    pub monotonic_version: u64,
    pub profile_id: [u8; 32],
    pub owner_id: [u8; 32],
    pub password_kdf: UserProfilePasswordKdf,
    pub body_sha256: [u8; 32],
    pub sealed_body: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[rkyv(crate = rkyv)]
pub struct UserProfilePasswordKdf {
    pub kdf: u16,
    pub flags: u16,
    pub rounds: u32,
    pub salt: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[rkyv(crate = rkyv)]
pub struct UserProfileBody {
    pub abi_version: u16,
    pub flags: u32,
    pub epoch: u64,
    pub monotonic_version: u64,
    pub profile_id: [u8; 32],
    pub owner_id: [u8; 32],
    pub owner_key_algorithm: u16,
    pub owner_private_key: Vec<u8>,
    pub grants: Vec<RuntimeCapabilityGrant>,
    pub signature: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[rkyv(crate = rkyv)]
pub struct RuntimeCapabilityGrant {
    pub abi_version: u16,
    pub flags: u32,
    pub grant_id: [u8; 32],
    pub profile_id: [u8; 32],
    pub user_id: [u8; 32],
    pub app_id: [u8; 32],
    pub release_id: [u8; 32],
    pub capability_kind: u16,
    pub operation: u16,
    pub min_assurance: u16,
    pub scope_sha256: [u8; 32],
    pub constraints_sha256: [u8; 32],
    pub valid_from: u64,
    pub valid_until: u64,
    pub user_signature: Vec<u8>,
}

#[allow(clippy::too_many_arguments)]
pub fn runtime_capability_grant_id(
    profile_id: [u8; 32],
    user_id: [u8; 32],
    app_id: [u8; 32],
    release_id: [u8; 32],
    capability_kind: u16,
    operation: u16,
    scope_sha256: [u8; 32],
    constraints_sha256: [u8; 32],
    valid_from: u64,
    valid_until: u64,
) -> [u8; 32] {
    let mut bytes = b"edgerun-runtime.capability-grant.v1".to_vec();
    bytes.push(0);
    bytes.extend_from_slice(&profile_id);
    bytes.extend_from_slice(&user_id);
    bytes.extend_from_slice(&app_id);
    bytes.extend_from_slice(&release_id);
    bytes.extend_from_slice(&capability_kind.to_le_bytes());
    bytes.extend_from_slice(&operation.to_le_bytes());
    bytes.extend_from_slice(&scope_sha256);
    bytes.extend_from_slice(&constraints_sha256);
    bytes.extend_from_slice(&valid_from.to_le_bytes());
    bytes.extend_from_slice(&valid_until.to_le_bytes());
    edgerun_crypto::sha256(&bytes)
}

pub fn runtime_storage_binding_id(
    grant_id: [u8; 32],
    namespace: &[u8],
    provider_id: [u8; 32],
    capability_id: [u8; 32],
) -> [u8; 32] {
    let mut bytes = b"edgerun-runtime.storage-binding.v1".to_vec();
    bytes.push(0);
    bytes.extend_from_slice(&grant_id);
    bytes.extend_from_slice(&(namespace.len() as u64).to_le_bytes());
    bytes.extend_from_slice(namespace);
    bytes.extend_from_slice(&provider_id);
    bytes.extend_from_slice(&capability_id);
    edgerun_crypto::sha256(&bytes)
}

#[allow(clippy::too_many_arguments)]
pub fn runtime_network_binding_id(
    grant_id: [u8; 32],
    provider_id: [u8; 32],
    capability_id: [u8; 32],
    binding_kind: u16,
    protocol: u16,
    port: u16,
    origin: &[u8],
    methods_sha256: [u8; 32],
) -> [u8; 32] {
    let mut bytes = b"edgerun-runtime.network-binding.v1".to_vec();
    bytes.push(0);
    bytes.extend_from_slice(&grant_id);
    bytes.extend_from_slice(&provider_id);
    bytes.extend_from_slice(&capability_id);
    bytes.extend_from_slice(&binding_kind.to_le_bytes());
    bytes.extend_from_slice(&protocol.to_le_bytes());
    bytes.extend_from_slice(&port.to_le_bytes());
    bytes.extend_from_slice(&(origin.len() as u64).to_le_bytes());
    bytes.extend_from_slice(origin);
    bytes.extend_from_slice(&methods_sha256);
    edgerun_crypto::sha256(&bytes)
}

pub fn runtime_capability_session_id(
    grant_id: [u8; 32],
    app_id: [u8; 32],
    release_id: [u8; 32],
    capability_id: [u8; 32],
    provider_node_id: [u8; 32],
    admission_hash: [u8; 32],
    route_commitment: [u8; 32],
) -> [u8; 32] {
    let mut bytes = b"edgerun-runtime.capability-session.v1".to_vec();
    bytes.push(0);
    bytes.extend_from_slice(&grant_id);
    bytes.extend_from_slice(&app_id);
    bytes.extend_from_slice(&release_id);
    bytes.extend_from_slice(&capability_id);
    bytes.extend_from_slice(&provider_node_id);
    bytes.extend_from_slice(&admission_hash);
    bytes.extend_from_slice(&route_commitment);
    edgerun_crypto::sha256(&bytes)
}

pub fn node_instance_id(
    owner_id: [u8; 32],
    node_id: [u8; 32],
    role: u16,
    runtime_target: u16,
    route_scope_sha256: [u8; 32],
) -> [u8; 32] {
    let mut bytes = b"edgerun-runtime.node-instance.v1".to_vec();
    bytes.push(0);
    bytes.extend_from_slice(&owner_id);
    bytes.extend_from_slice(&node_id);
    bytes.extend_from_slice(&role.to_le_bytes());
    bytes.extend_from_slice(&runtime_target.to_le_bytes());
    bytes.extend_from_slice(&route_scope_sha256);
    edgerun_crypto::sha256(&bytes)
}

pub fn admission_policy_id(
    owner_id: [u8; 32],
    admission_node_id: [u8; 32],
    policy_hash: [u8; 32],
    source: u16,
    valid_from: u64,
    valid_until: u64,
) -> [u8; 32] {
    let mut bytes = b"edgerun-runtime.admission-policy.v1".to_vec();
    bytes.push(0);
    bytes.extend_from_slice(&owner_id);
    bytes.extend_from_slice(&admission_node_id);
    bytes.extend_from_slice(&policy_hash);
    bytes.extend_from_slice(&source.to_le_bytes());
    bytes.extend_from_slice(&valid_from.to_le_bytes());
    bytes.extend_from_slice(&valid_until.to_le_bytes());
    edgerun_crypto::sha256(&bytes)
}

pub fn package_cache_id(
    profile_id: [u8; 32],
    app_id: [u8; 32],
    release_id: [u8; 32],
    package_sha256: [u8; 32],
    manifest_sha256: [u8; 32],
) -> [u8; 32] {
    let mut bytes = b"edgerun-runtime.package-cache.v1".to_vec();
    bytes.push(0);
    bytes.extend_from_slice(&profile_id);
    bytes.extend_from_slice(&app_id);
    bytes.extend_from_slice(&release_id);
    bytes.extend_from_slice(&package_sha256);
    bytes.extend_from_slice(&manifest_sha256);
    edgerun_crypto::sha256(&bytes)
}

pub fn app_run_prompt_decision_id(
    profile_id: [u8; 32],
    app_id: [u8; 32],
    release_id: [u8; 32],
    package_sha256: [u8; 32],
    manifest_sha256: [u8; 32],
    decision: u16,
    decided_at: u64,
) -> [u8; 32] {
    let mut bytes = b"edgerun-runtime.app-run-prompt-decision.v1".to_vec();
    bytes.push(0);
    bytes.extend_from_slice(&profile_id);
    bytes.extend_from_slice(&app_id);
    bytes.extend_from_slice(&release_id);
    bytes.extend_from_slice(&package_sha256);
    bytes.extend_from_slice(&manifest_sha256);
    bytes.extend_from_slice(&decision.to_le_bytes());
    bytes.extend_from_slice(&decided_at.to_le_bytes());
    edgerun_crypto::sha256(&bytes)
}

#[derive(Clone, Debug, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[rkyv(crate = rkyv)]
pub struct NodeInstanceRecord {
    pub abi_version: u16,
    pub flags: u32,
    pub instance_id: [u8; 32],
    pub owner_id: [u8; 32],
    pub node_id: [u8; 32],
    pub role: u16,
    pub runtime_target: u16,
    pub policy_hash: [u8; 32],
    pub admitted_budget: u64,
    pub route_scope_sha256: [u8; 32],
    pub valid_from: u64,
    pub valid_until: u64,
    pub status: u16,
}

#[derive(Clone, Debug, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[rkyv(crate = rkyv)]
pub struct AdmissionPolicyRecord {
    pub abi_version: u16,
    pub flags: u32,
    pub policy_id: [u8; 32],
    pub owner_id: [u8; 32],
    pub admission_node_id: [u8; 32],
    pub source: u16,
    pub policy_hash: [u8; 32],
    pub inherited_policy_hashes: Vec<[u8; 32]>,
    pub max_budget: u64,
    pub valid_from: u64,
    pub valid_until: u64,
    pub signature: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[rkyv(crate = rkyv)]
pub struct PackageCacheRecord {
    pub abi_version: u16,
    pub flags: u32,
    pub cache_id: [u8; 32],
    pub profile_id: [u8; 32],
    pub app_id: [u8; 32],
    pub release_id: [u8; 32],
    pub package_sha256: [u8; 32],
    pub manifest_sha256: [u8; 32],
    pub code_sha256: [u8; 32],
    pub cached_bytes: u64,
    pub state: u16,
    pub verified_at: u64,
    pub source_admission_hash: [u8; 32],
}

#[derive(Clone, Debug, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[rkyv(crate = rkyv)]
pub struct AppRunPromptDecisionRecord {
    pub abi_version: u16,
    pub flags: u32,
    pub decision_id: [u8; 32],
    pub profile_id: [u8; 32],
    pub app_id: [u8; 32],
    pub release_id: [u8; 32],
    pub package_sha256: [u8; 32],
    pub manifest_sha256: [u8; 32],
    pub retrieval_cost: u64,
    pub decision: u16,
    pub decided_at: u64,
    pub user_signature: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[rkyv(crate = rkyv)]
pub struct RuntimeStorageBinding {
    pub abi_version: u16,
    pub flags: u32,
    pub binding_id: [u8; 32],
    pub grant_id: [u8; 32],
    pub profile_id: [u8; 32],
    pub app_id: [u8; 32],
    pub release_id: [u8; 32],
    pub namespace: Vec<u8>,
    pub provider_id: [u8; 32],
    pub capability_id: [u8; 32],
    pub backing_kind: u16,
    pub scope_sha256: [u8; 32],
}

#[derive(Clone, Debug, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[rkyv(crate = rkyv)]
pub struct RuntimeNetworkBinding {
    pub abi_version: u16,
    pub flags: u32,
    pub binding_id: [u8; 32],
    pub grant_id: [u8; 32],
    pub profile_id: [u8; 32],
    pub app_id: [u8; 32],
    pub release_id: [u8; 32],
    pub provider_id: [u8; 32],
    pub capability_id: [u8; 32],
    pub binding_kind: u16,
    pub protocol: u16,
    pub port: u16,
    pub origin: Vec<u8>,
    pub methods_sha256: [u8; 32],
    pub scope_sha256: [u8; 32],
}

#[derive(Clone, Debug, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[rkyv(crate = rkyv)]
pub struct RuntimeCapabilitySession {
    pub abi_version: u16,
    pub flags: u32,
    pub session_id: [u8; 32],
    pub grant_id: [u8; 32],
    pub app_id: [u8; 32],
    pub release_id: [u8; 32],
    pub capability_id: [u8; 32],
    pub provider_node_id: [u8; 32],
    pub admission_hash: [u8; 32],
    pub route_commitment: [u8; 32],
    pub valid_until: u64,
    pub status: u16,
}

#[derive(Clone, Debug, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[rkyv(crate = rkyv)]
pub struct RuntimeEvent {
    pub abi_version: u16,
    pub flags: u32,
    pub seq: u64,
    pub time: u64,
    pub event_kind: u16,
    pub status: u16,
    pub runtime_id: [u8; 32],
    pub previous_event_sha256: [u8; 32],
    pub payload_sha256: [u8; 32],
    pub payload: Vec<u8>,
    pub signature: Vec<u8>,
}

impl RuntimeEvent {
    #[allow(clippy::too_many_arguments)]
    pub fn unsigned_payload(
        seq: u64,
        time: u64,
        event_kind: u16,
        status: u16,
        runtime_id: [u8; 32],
        previous_event_sha256: [u8; 32],
        payload_sha256: [u8; 32],
        payload: Vec<u8>,
    ) -> Self {
        Self {
            abi_version: SDK_WIRE_ABI_VERSION,
            flags: 1,
            seq,
            time,
            event_kind,
            status,
            runtime_id,
            previous_event_sha256,
            payload_sha256,
            payload,
            signature: Vec::new(),
        }
    }
}

/// Stable ABI record for an app runtime projection.
///
/// The type name is retained for wire compatibility. It describes verified app
/// identity, release, declared routes, storage namespaces, and capability
/// declarations; it is not installation authority by itself.
#[allow(clippy::large_enum_variant)]
#[derive(Clone, Debug, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[rkyv(crate = rkyv)]
pub struct RuntimeAppInstall {
    pub abi_version: u16,
    pub flags: u32,
    pub app_id: [u8; 32],
    pub release_id: [u8; 32],
    pub code_sha256: [u8; 32],
    pub developer_id: [u8; 32],
    pub manifest_sha256: [u8; 32],
    pub declared_routes: Vec<RuntimeHttpRoute>,
    pub storage_namespaces: Vec<Vec<u8>>,
    pub provided_capabilities: Vec<RuntimeCapabilityDeclaration>,
    pub required_capabilities: Vec<RuntimeCapabilityDeclaration>,
}

#[derive(Clone, Debug, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[rkyv(crate = rkyv)]
pub struct RuntimeCapabilityDeclaration {
    pub abi_version: u16,
    pub flags: u32,
    pub capability_kind: u16,
    pub operation: u16,
    pub min_assurance: u16,
    pub scope_sha256: [u8; 32],
    pub label: Vec<u8>,
    pub context: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[rkyv(crate = rkyv)]
pub struct RuntimeHttpRoute {
    pub abi_version: u16,
    pub flags: u32,
    pub app_id: [u8; 32],
    pub release_id: [u8; 32],
    pub scheme: u16,
    pub host: Vec<u8>,
    pub path_prefix: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[rkyv(crate = rkyv)]
pub struct RuntimeHttpRequest {
    pub abi_version: u16,
    pub flags: u32,
    pub method: u16,
    pub scheme: u16,
    pub host: Vec<u8>,
    pub path: Vec<u8>,
    pub header_sha256: [u8; 32],
    pub body_sha256: [u8; 32],
    pub body: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[rkyv(crate = rkyv)]
pub struct RuntimeHttpDispatch {
    pub abi_version: u16,
    pub flags: u32,
    pub status: u16,
    pub app_id: [u8; 32],
    pub release_id: [u8; 32],
    pub route_host: Vec<u8>,
    pub route_path_prefix: Vec<u8>,
    pub request_sha256: [u8; 32],
}

#[derive(Clone, Debug, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[rkyv(crate = rkyv)]
pub struct RuntimeAppMessage {
    pub abi_version: u16,
    pub flags: u32,
    pub from_app_id: [u8; 32],
    pub to_app_id: [u8; 32],
    pub message_kind: u16,
    pub payload_sha256: [u8; 32],
    pub payload: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[rkyv(crate = rkyv)]
pub struct RuntimeIdentityRoute {
    pub abi_version: u16,
    pub flags: u32,
    pub identity_id: [u8; 32],
    pub runtime_id: [u8; 32],
    pub node_id: [u8; 64],
    pub valid_from: u64,
    pub valid_until: u64,
    pub route_hint: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[rkyv(crate = rkyv)]
pub struct RuntimeRoutedAppMessage {
    pub abi_version: u16,
    pub flags: u32,
    pub from_runtime_id: [u8; 32],
    pub to_runtime_id: [u8; 32],
    pub to_node_id: [u8; 64],
    pub message_sha256: [u8; 32],
    pub message: RuntimeAppMessage,
}

#[derive(Clone, Debug, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[rkyv(crate = rkyv)]
pub struct RuntimeProtocolBinding {
    pub abi_version: u16,
    pub flags: u32,
    pub protocol: u16,
    pub port: u16,
    pub bind_ipv4: [u8; 4],
    pub host: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[rkyv(crate = rkyv)]
pub struct RuntimeMailbox {
    pub abi_version: u16,
    pub flags: u32,
    pub address: Vec<u8>,
    pub target_app_id: [u8; 32],
}

#[derive(Clone, Debug, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[rkyv(crate = rkyv)]
pub struct RuntimeDomainConfig {
    pub abi_version: u16,
    pub flags: u32,
    pub domain: Vec<u8>,
    pub authoritative_dns: bool,
    pub mail_enabled: bool,
    pub acme_enabled: bool,
    pub mailboxes: Vec<RuntimeMailbox>,
    pub routes: Vec<RuntimeHttpRoute>,
}

#[derive(Clone, Debug, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[rkyv(crate = rkyv)]
pub struct RuntimeDeploymentConfig {
    pub abi_version: u16,
    pub flags: u32,
    pub runtime_id: [u8; 32],
    pub public_ipv4: [u8; 4],
    pub hostname: Vec<u8>,
    pub origin: Vec<u8>,
    pub acme_contact: Vec<u8>,
    pub protocol_bindings: Vec<RuntimeProtocolBinding>,
    pub domains: Vec<RuntimeDomainConfig>,
    /// Verified app runtime projections included in this deployment.
    pub apps: Vec<RuntimeAppInstall>,
}

#[derive(Clone, Debug, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[rkyv(crate = rkyv)]
pub struct TrustPolicy {
    pub abi_version: u16,
    pub flags: u32,
    pub entries: Vec<TrustEntry>,
}

#[derive(Clone, Debug, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[rkyv(crate = rkyv)]
pub struct TrustEntry {
    pub role: u16,
    pub flags: u16,
    pub valid_from: u64,
    pub valid_until: u64,
    pub public_key: [u8; 32],
    pub app_id: [u8; 32],
    pub developer_id: [u8; 32],
}

#[derive(Clone, Debug, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[rkyv(crate = rkyv)]
pub struct Revocation {
    pub abi_version: u16,
    pub flags: u32,
    pub kind: u16,
    pub issued_at: u64,
    pub target: [u8; 32],
    pub issuer: [u8; 32],
    pub reason: Vec<u8>,
    pub signature: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[rkyv(crate = rkyv)]
pub struct UnitManifestRecord {
    pub abi_version: u16,
    pub flags: u32,
    pub import_count: u16,
    pub export_count: u16,
    pub standard_id: i32,
    pub wasm_sha256: [u8; 32],
    pub unit_id: Vec<u8>,
    pub standard: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[rkyv(crate = rkyv)]
pub struct UnitApi {
    pub abi_version: u16,
    pub functions: Vec<UnitApiFunction>,
}

#[derive(Clone, Debug, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[rkyv(crate = rkyv)]
pub struct UnitApiFunction {
    pub name: Vec<u8>,
    pub params: Vec<u8>,
    pub results: Vec<u8>,
    pub cost_base: u32,
    pub cost_per_byte: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[rkyv(crate = rkyv)]
pub struct CompositionRecord {
    pub abi_version: u16,
    pub flags: u32,
    pub id: Vec<u8>,
    pub output_unit: Vec<u8>,
    pub components: Vec<CompositionComponentRecord>,
    pub steps: Vec<CompositionStepRecord>,
}

#[derive(Clone, Debug, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[rkyv(crate = rkyv)]
pub struct CompositionComponentRecord {
    pub unit_id: Vec<u8>,
    pub wasm_sha256: [u8; 32],
}

#[derive(Clone, Debug, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[rkyv(crate = rkyv)]
pub struct CompositionStepRecord {
    pub opcode: u8,
    pub component_index: u8,
    pub function_index: u8,
    pub arg0: u32,
    pub arg1: u32,
    pub arg2: u32,
    pub arg3: u32,
    pub arg4: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[rkyv(crate = rkyv)]
pub struct SegmentRecord {
    pub abi_version: u16,
    pub flags: u32,
    pub input_count: u16,
    pub output_count: u16,
    pub component_start: u16,
    pub component_count: u16,
    pub step_start: u16,
    pub step_count: u16,
    pub composition_sha256: [u8; 32],
    pub id: Vec<u8>,
    pub composition_id: Vec<u8>,
    pub node_role: Vec<u8>,
    pub capability: Vec<u8>,
    pub input_kinds: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[rkyv(crate = rkyv)]
pub struct ChainRecord {
    pub abi_version: u16,
    pub flags: u32,
    pub id: Vec<u8>,
    pub segments: Vec<Vec<u8>>,
    pub links: Vec<ChainLinkRecord>,
}

#[derive(Clone, Debug, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[rkyv(crate = rkyv)]
pub struct ChainLinkRecord {
    pub from_segment: u16,
    pub from_output: u16,
    pub to_segment: u16,
    pub to_input: u16,
}

#[derive(Clone, Debug, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[rkyv(crate = rkyv)]
pub struct ExecutionReportRecord {
    pub abi_version: u16,
    pub flags: u32,
    pub input_count: u16,
    pub step_count: u16,
    pub cost: u64,
    pub output_len: u32,
    pub composition_sha256: [u8; 32],
    pub output_sha256: [u8; 32],
    pub composition_id: Vec<u8>,
    pub output_unit_id: Vec<u8>,
    pub input_lengths: Vec<u32>,
    pub components: Vec<CompositionComponentRecord>,
}

#[derive(Clone, Debug, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[rkyv(crate = rkyv)]
pub struct SegmentReportRecord {
    pub abi_version: u16,
    pub flags: u32,
    pub status: u16,
    pub input_count: u16,
    pub output_count: u16,
    pub cost: u64,
    pub output_len: u32,
    pub failed_step: u16,
    pub segment_sha256: [u8; 32],
    pub composition_sha256: [u8; 32],
    pub output_sha256: [u8; 32],
    pub segment_id: Vec<u8>,
    pub composition_id: Vec<u8>,
    pub node_role: Vec<u8>,
    pub input_lengths: Vec<u32>,
    pub input_sha256: Vec<[u8; 32]>,
}

#[derive(Clone, Debug, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[rkyv(crate = rkyv)]
pub struct ArtifactSignature {
    pub abi_version: u16,
    pub flags: u32,
    pub algorithm: u16,
    pub artifact_sha256: [u8; 32],
    pub public_key: Vec<u8>,
    pub signature: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[rkyv(crate = rkyv)]
pub struct SignerPolicy {
    pub abi_version: u16,
    pub flags: u32,
    pub entries: Vec<SignerPolicyEntry>,
}

#[derive(Clone, Debug, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[rkyv(crate = rkyv)]
pub struct SignerPolicyEntry {
    pub algorithm: u16,
    pub segment_sha256: [u8; 32],
    pub public_key: Vec<u8>,
    pub segment_id: Vec<u8>,
    pub node_role: Vec<u8>,
    pub capability: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[rkyv(crate = rkyv)]
pub struct AppManifestRecord {
    pub abi_version: u16,
    pub flags: u32,
    pub app_id: [u8; 32],
    pub developer_id: [u8; 32],
    pub app_slug: Vec<u8>,
    pub name: Vec<u8>,
    pub version: Vec<u8>,
    pub summary: Vec<u8>,
    pub code_sha256: [u8; 32],
    pub routes: Vec<AppHttpRouteRecord>,
    pub storage_namespaces: Vec<Vec<u8>>,
    pub provided_capabilities: Vec<RuntimeCapabilityDeclaration>,
    pub required_capabilities: Vec<RuntimeCapabilityDeclaration>,
}

#[derive(Clone, Debug, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[rkyv(crate = rkyv)]
pub struct AppHttpRouteRecord {
    pub scheme: u16,
    pub host: Vec<u8>,
    pub path_prefix: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[rkyv(crate = rkyv)]
pub struct AppGraphRecord {
    pub abi_version: u16,
    pub flags: u32,
    pub app_id: [u8; 32],
    pub developer_public_key: [u8; 32],
    pub app_manifest_sha256: [u8; 32],
    pub app_slug: Vec<u8>,
    /// Verified runtime projection for this app graph. Field name is retained
    /// for wire compatibility.
    pub runtime_install: RuntimeAppInstall,
    pub artifacts: Vec<AppArtifactRecord>,
}

#[derive(Clone, Debug, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[rkyv(crate = rkyv)]
pub struct AppArtifactRecord {
    pub kind: u16,
    pub path: Vec<u8>,
    pub sha256: [u8; 32],
}

#[derive(Clone, Debug, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[rkyv(crate = rkyv)]
pub struct EntitlementRecord {
    pub abi_version: u16,
    pub flags: u32,
    pub kind: u16,
    pub store_fee_bps: u16,
    pub developer_share_bps: u16,
    pub valid_from: u64,
    pub valid_until: u64,
    pub app_id: [u8; 32],
    pub developer_id: [u8; 32],
    pub store_id: [u8; 32],
    pub release_id: [u8; 32],
    pub terms_sha256: [u8; 32],
    pub product_sha256: [u8; 32],
    pub subject_id: Vec<u8>,
    pub product_id: Vec<u8>,
    pub purchase_id: Vec<u8>,
    pub signature: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[rkyv(crate = rkyv)]
pub struct ProductRecord {
    pub abi_version: u16,
    pub flags: u32,
    pub kind: u16,
    pub store_fee_bps: u16,
    pub developer_share_bps: u16,
    pub price_minor: u64,
    pub validity_seconds: u64,
    pub app_id: [u8; 32],
    pub developer_id: [u8; 32],
    pub store_id: [u8; 32],
    pub release_id: [u8; 32],
    pub terms_sha256: [u8; 32],
    pub product_id: Vec<u8>,
    pub currency: Vec<u8>,
    pub developer_signature: Vec<u8>,
    pub store_signature: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[rkyv(crate = rkyv)]
pub struct AppStoreCatalogRecord {
    pub abi_version: u16,
    pub flags: u32,
    pub store_id: [u8; 32],
    pub generated_at: u64,
    pub sequence: u64,
    pub previous_catalog_sha256: [u8; 32],
    pub entries: Vec<AppStoreCatalogEntry>,
    pub signature: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[rkyv(crate = rkyv)]
pub struct AppStoreCatalogEntry {
    pub app_id: [u8; 32],
    pub release_id: [u8; 32],
    pub developer_id: [u8; 32],
    pub app_graph_sha256: [u8; 32],
    pub manifest_sha256: [u8; 32],
    pub package_sha256: [u8; 32],
    pub package_bytes: u64,
    pub status: u16,
    pub name: Vec<u8>,
    pub version: Vec<u8>,
    pub summary: Vec<u8>,
    pub app_slug: Vec<u8>,
    pub package_ref: Vec<u8>,
    pub manifest_ref: Vec<u8>,
    pub launch_ref: Vec<u8>,
    pub required_capabilities: Vec<Vec<u8>>,
    pub optional_capabilities: Vec<Vec<u8>>,
    pub asset_refs: Vec<AppStoreAssetRef>,
}

#[derive(Clone, Debug, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[rkyv(crate = rkyv)]
pub struct AppStoreAssetRef {
    pub path: Vec<u8>,
    pub sha256: [u8; 32],
    pub bytes: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[rkyv(crate = rkyv)]
pub struct AppStoreSubmissionRecord {
    pub abi_version: u16,
    pub flags: u32,
    pub submitted_at: u64,
    pub app_id: [u8; 32],
    pub release_id: [u8; 32],
    pub developer_id: [u8; 32],
    pub app_graph_sha256: [u8; 32],
    pub manifest_sha256: [u8; 32],
    pub package_sha256: [u8; 32],
    pub package_bytes: u64,
    pub app_slug: Vec<u8>,
    pub package_ref: Vec<u8>,
    pub manifest_ref: Vec<u8>,
    pub notes: Vec<u8>,
    pub app_graph: AppGraphRecord,
    pub developer_signature: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[rkyv(crate = rkyv)]
pub struct AppStoreReviewRecord {
    pub abi_version: u16,
    pub flags: u32,
    pub reviewed_at: u64,
    pub decision: u16,
    pub app_id: [u8; 32],
    pub release_id: [u8; 32],
    pub submission_sha256: [u8; 32],
    pub reviewer_id: [u8; 32],
    pub reason: Vec<u8>,
    pub signature: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[rkyv(crate = rkyv)]
pub struct SettlementRecord {
    pub abi_version: u16,
    pub flags: u32,
    pub period_start: u64,
    pub period_end: u64,
    pub gross_minor: u64,
    pub processor_fee_minor: u64,
    pub store_fee_minor: u64,
    pub developer_net_minor: u64,
    pub developer_id: [u8; 32],
    pub store_id: [u8; 32],
    pub currency: Vec<u8>,
    pub entitlement_hashes: Vec<[u8; 32]>,
    pub signature: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[rkyv(crate = rkyv)]
pub struct PaymentRecord {
    pub abi_version: u16,
    pub flags: u32,
    pub purpose: u16,
    pub amount_minor: u64,
    pub created_at: u64,
    pub settled_at: u64,
    pub app_id: [u8; 32],
    pub release_id: [u8; 32],
    pub payer_id: [u8; 32],
    pub payee_id: [u8; 32],
    pub context_sha256: [u8; 32],
    pub rail_ref_sha256: [u8; 32],
    pub currency: Vec<u8>,
    pub rail_kind: Vec<u8>,
    pub payer_signature: Vec<u8>,
    pub store_id: Vec<u8>,
    pub store_signature: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[rkyv(crate = rkyv)]
pub struct DerivedDbMetaRecord {
    pub key: Vec<u8>,
    pub value: Vec<u8>,
    pub updated_at: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[rkyv(crate = rkyv)]
pub struct DerivedDbKeyRecord {
    pub key: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[rkyv(crate = rkyv)]
pub struct DerivedDbObjectIndexRecord {
    pub object_id: Vec<u8>,
    pub stream_id: Vec<u8>,
    pub seq: u64,
    pub representation_id: Vec<u8>,
    pub content_type: Option<String>,
    pub updated_at: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[rkyv(crate = rkyv)]
pub struct DerivedDbAdminAuditRecord {
    pub id: u64,
    pub event_time: i64,
    pub subject: String,
    pub action: String,
    pub outcome: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[rkyv(crate = rkyv)]
pub struct RemoteInputEvents {
    pub events: Vec<RemoteInputEventRecord>,
}

#[derive(Clone, Debug, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[rkyv(crate = rkyv)]
pub struct RemoteInputEventRecord {
    pub timestamp_sec: i64,
    pub timestamp_usec: i64,
    pub kind: u16,
    pub code: u16,
    pub value: i32,
}

#[derive(Clone, Debug, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[rkyv(crate = rkyv)]
pub struct RemoteAudioCapture {
    pub sample_rate_hz: u32,
    pub channels: u16,
    pub format: u32,
    pub started_at_unix_ms: i64,
    pub bytes: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[rkyv(crate = rkyv)]
pub struct RemoteAudioPlaybackRequest {
    pub duration_ms: u32,
    pub sample_rate_hz: u32,
    pub channels: u16,
    pub format: u32,
    pub audio_bytes: Vec<u8>,
    pub software_gain_percent: Option<u16>,
    pub target_output_level_percent: Option<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[rkyv(crate = rkyv)]
pub struct RemoteSpeakerOutputLevel {
    pub current_percent: u8,
    pub min_raw_value: i64,
    pub max_raw_value: i64,
    pub muted: Option<bool>,
}

#[derive(Clone, Debug, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[rkyv(crate = rkyv)]
pub struct RemoteAudioPlaybackResult {
    pub bytes_written: u64,
    pub sample_rate_hz: u32,
    pub channels: u16,
    pub finished: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[rkyv(crate = rkyv)]
pub struct RemoteDisplayMode {
    pub width: u32,
    pub height: u32,
    pub refresh_millihz: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[rkyv(crate = rkyv)]
pub struct RemoteDisplayInfo {
    pub provider: String,
    pub display_name: String,
    pub instance_id: String,
    pub built_in: bool,
    pub primary: bool,
    pub current_mode: RemoteDisplayMode,
    pub modes: Vec<RemoteDisplayMode>,
    pub hdr_capable: bool,
    pub touch_capable: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[rkyv(crate = rkyv)]
pub struct RemoteDisplayUpdateRequest {
    pub content_kind: u32,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub refresh_millihz: Option<u32>,
}

#[derive(Clone, Debug, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[rkyv(crate = rkyv)]
pub struct RemoteCameraFrame {
    pub width: u32,
    pub height: u32,
    pub stride: u32,
    pub format: u32,
    pub bytes: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[rkyv(crate = rkyv)]
pub struct RemoteFaceBounds {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[rkyv(crate = rkyv)]
pub struct RemoteBiometricState {
    pub modality: u8,
    pub verified: bool,
    pub hardware_protected: bool,
    pub user_present: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[rkyv(crate = rkyv)]
pub struct RemoteCameraCapture {
    pub frame: RemoteCameraFrame,
    pub quality: u8,
    pub face_bounds: Option<RemoteFaceBounds>,
    pub state: RemoteBiometricState,
}

#[derive(Clone, Debug, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[rkyv(crate = rkyv)]
pub struct RemotePairedCameraFrame {
    pub rgb: Option<RemoteCameraFrame>,
    pub infrared: Option<RemoteCameraFrame>,
    pub depth: Option<RemoteCameraFrame>,
}

#[derive(Clone, Debug, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[rkyv(crate = rkyv)]
pub struct RemoteWifiScanResult {
    pub observations: Vec<RemoteWifiNetworkObservation>,
}

#[derive(Clone, Debug, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[rkyv(crate = rkyv)]
pub struct RemoteWifiNetworkObservation {
    pub interface_name: String,
    pub ssid: Option<String>,
    pub bssid: Option<String>,
    pub signal_dbm: Option<i16>,
    pub frequency_mhz: Option<u32>,
    pub secure: Option<bool>,
    pub observed_at_unix_ms: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[rkyv(crate = rkyv)]
pub struct RemoteWifiInterfaceInfo {
    pub provider: String,
    pub interface_name: String,
    pub mac_address: Option<String>,
    pub phy_name: Option<String>,
    pub operstate: Option<String>,
    pub power_state: u8,
    pub mode: u8,
}

#[derive(Clone, Debug, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[rkyv(crate = rkyv)]
pub struct RemoteBluetoothScanResult {
    pub observations: Vec<RemoteBluetoothBeaconObservation>,
}

#[derive(Clone, Debug, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[rkyv(crate = rkyv)]
pub struct RemoteBluetoothBeaconObservation {
    pub device_id: String,
    pub transport_kind: u8,
    pub address_kind: u8,
    pub rssi_dbm: i16,
    pub tx_power_dbm: Option<i16>,
    pub local_name: Option<String>,
    pub service_uuids: Vec<String>,
    pub profiles: Vec<u8>,
    pub classic_device_class: Option<u32>,
    pub advertisement_data: Vec<u8>,
    pub captured_at_unix_ms: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[rkyv(crate = rkyv)]
pub struct RemoteBluetoothConnections {
    pub connections: Vec<RemoteBluetoothConnectionInfo>,
}

#[derive(Clone, Debug, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[rkyv(crate = rkyv)]
pub struct RemoteBluetoothConnectionInfo {
    pub device_id: String,
    pub transport_kind: u8,
    pub address_kind: u8,
    pub link_kind: u8,
    pub outbound: bool,
    pub state: u16,
    pub local_name: Option<String>,
    pub service_uuids: Vec<String>,
    pub profiles: Vec<u8>,
    pub trusted: Option<bool>,
    pub paired: Option<bool>,
}

pub type RemoteBlockRequestId = u64;

#[derive(Clone, Debug, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[rkyv(crate = rkyv)]
pub struct RemoteBlockDeviceInfo {
    pub block_size: u32,
    pub block_count: u64,
    pub readonly: bool,
    pub supports_flush: bool,
    pub supports_discard: bool,
    pub supports_write_zeroes: bool,
    pub model: String,
    pub serial: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[rkyv(crate = rkyv)]
pub enum RemoteBlockError {
    OutOfRange,
    ReadOnly,
    Misaligned,
    Unsupported,
    BackendFailure(String),
    ProtocolError(String),
    NotReady,
    Timeout,
}

#[derive(Clone, Debug, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[rkyv(crate = rkyv)]
pub enum RemoteBlockRequest {
    Handshake {
        protocol_version: u16,
    },
    GetInfo,
    Read {
        request_id: RemoteBlockRequestId,
        lba: u64,
        blocks: u32,
    },
    Write {
        request_id: RemoteBlockRequestId,
        lba: u64,
        blocks: u32,
        data: Vec<u8>,
    },
    Flush {
        request_id: RemoteBlockRequestId,
    },
    Discard {
        request_id: RemoteBlockRequestId,
        lba: u64,
        blocks: u32,
    },
    WriteZeroes {
        request_id: RemoteBlockRequestId,
        lba: u64,
        blocks: u32,
    },
    Ping,
}

#[derive(Clone, Debug, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[rkyv(crate = rkyv)]
pub enum RemoteBlockResponse {
    HandshakeAck {
        protocol_version: u16,
    },
    Info(RemoteBlockDeviceInfo),
    ReadResult {
        request_id: RemoteBlockRequestId,
        data: Vec<u8>,
    },
    WriteAck {
        request_id: RemoteBlockRequestId,
    },
    FlushAck {
        request_id: RemoteBlockRequestId,
    },
    DiscardAck {
        request_id: RemoteBlockRequestId,
    },
    WriteZeroesAck {
        request_id: RemoteBlockRequestId,
    },
    Pong,
    Error {
        request_id: Option<RemoteBlockRequestId>,
        error: RemoteBlockError,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[rkyv(crate = rkyv)]
pub struct EdgeFsFileMeta {
    pub mode: u32,
    pub uid: u32,
    pub gid: u32,
    pub mtime: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[rkyv(crate = rkyv)]
pub struct EdgeFsDeviceId {
    pub major: u32,
    pub minor: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[rkyv(crate = rkyv)]
pub enum EdgeFsRecordPayload {
    Put {
        path: String,
        kind: u8,
        meta: EdgeFsFileMeta,
        data: Vec<u8>,
        link_target: Option<String>,
        device: Option<EdgeFsDeviceId>,
    },
    Delete {
        path: String,
    },
    DeleteChildren {
        path: String,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[rkyv(crate = rkyv)]
pub enum SdkWireRecord {
    CapabilityRequest(CapabilityRequest),
    CapabilityResponse(CapabilityResponse),
    UserProfile(UserProfile),
    UserProfileBody(UserProfileBody),
    RuntimeCapabilityGrant(RuntimeCapabilityGrant),
    RuntimeStorageBinding(RuntimeStorageBinding),
    RuntimeNetworkBinding(RuntimeNetworkBinding),
    RuntimeCapabilitySession(RuntimeCapabilitySession),
    RuntimeEvent(RuntimeEvent),
    NodeInstance(NodeInstanceRecord),
    AdmissionPolicy(AdmissionPolicyRecord),
    PackageCache(PackageCacheRecord),
    AppRunPromptDecision(AppRunPromptDecisionRecord),
    RuntimeAppInstall(RuntimeAppInstall),
    RuntimeCapabilityDeclaration(RuntimeCapabilityDeclaration),
    RuntimeHttpRoute(RuntimeHttpRoute),
    RuntimeHttpRequest(RuntimeHttpRequest),
    RuntimeHttpDispatch(RuntimeHttpDispatch),
    RuntimeAppMessage(RuntimeAppMessage),
    RuntimeIdentityRoute(RuntimeIdentityRoute),
    RuntimeRoutedAppMessage(RuntimeRoutedAppMessage),
    RuntimeProtocolBinding(RuntimeProtocolBinding),
    RuntimeMailbox(RuntimeMailbox),
    RuntimeDomainConfig(RuntimeDomainConfig),
    RuntimeDeploymentConfig(RuntimeDeploymentConfig),
    TrustPolicy(TrustPolicy),
    TrustEntry(TrustEntry),
    Revocation(Revocation),
    UnitManifest(UnitManifestRecord),
    UnitApi(UnitApi),
    UnitApiFunction(UnitApiFunction),
    Composition(CompositionRecord),
    CompositionComponent(CompositionComponentRecord),
    CompositionStep(CompositionStepRecord),
    Segment(SegmentRecord),
    Chain(ChainRecord),
    ChainLink(ChainLinkRecord),
    ExecutionReport(ExecutionReportRecord),
    SegmentReport(SegmentReportRecord),
    ArtifactSignature(ArtifactSignature),
    SignerPolicy(SignerPolicy),
    SignerPolicyEntry(SignerPolicyEntry),
    AppManifest(AppManifestRecord),
    AppGraph(AppGraphRecord),
    AppArtifact(AppArtifactRecord),
    Entitlement(EntitlementRecord),
    Product(ProductRecord),
    AppStoreCatalog(AppStoreCatalogRecord),
    AppStoreCatalogEntry(AppStoreCatalogEntry),
    AppStoreSubmission(AppStoreSubmissionRecord),
    AppStoreReview(AppStoreReviewRecord),
    Settlement(SettlementRecord),
    Payment(PaymentRecord),
    UserProfilePasswordKdf(UserProfilePasswordKdf),
}

pub fn sdk_wire_bytes(record: &SdkWireRecord) -> Vec<u8> {
    to_bytes::<WireError>(record)
        .expect("SDK wire records must serialize through the rkyv wire boundary")
        .into_vec()
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    #[test]
    fn capability_request_archives_through_rkyv() {
        let request = CapabilityRequest::new(
            CAPABILITY_KIND_STORAGE,
            CAPABILITY_OPERATION_READ,
            2,
            [1; 32],
            [2; 32],
            [3; 32],
            [4; 32],
            b"user/state".to_vec(),
            Vec::new(),
            b"nonce".to_vec(),
        );
        let bytes = sdk_wire_bytes(&SdkWireRecord::CapabilityRequest(request));
        assert!(!bytes.is_empty());
    }

    #[test]
    fn user_profile_body_archives_grants() {
        let body = UserProfileBody {
            abi_version: SDK_WIRE_ABI_VERSION,
            flags: 1,
            epoch: 7,
            monotonic_version: 2,
            profile_id: [1; 32],
            owner_id: [2; 32],
            owner_key_algorithm: USER_PROFILE_OWNER_KEY_ED25519,
            owner_private_key: vec![6; 32],
            grants: vec![RuntimeCapabilityGrant {
                abi_version: SDK_WIRE_ABI_VERSION,
                flags: 1,
                grant_id: [10; 32],
                profile_id: [1; 32],
                user_id: [2; 32],
                app_id: [3; 32],
                release_id: [4; 32],
                capability_kind: CAPABILITY_KIND_SEALING,
                operation: CAPABILITY_OPERATION_UNSEAL,
                min_assurance: 2,
                scope_sha256: [5; 32],
                constraints_sha256: [11; 32],
                valid_from: 10,
                valid_until: 20,
                user_signature: vec![12; 64],
            }],
            signature: vec![9; 64],
        };
        let bytes = sdk_wire_bytes(&SdkWireRecord::UserProfileBody(body));
        assert!(!bytes.is_empty());
    }

    #[test]
    fn runtime_event_archives_signed_log_entry_shape() {
        let event = RuntimeEvent::unsigned_payload(
            3,
            42,
            RUNTIME_EVENT_CAPABILITY_DENIED,
            CAPABILITY_STATUS_POLICY_DENIED,
            [1; 32],
            [2; 32],
            [3; 32],
            b"policy_denied".to_vec(),
        );
        let bytes = sdk_wire_bytes(&SdkWireRecord::RuntimeEvent(event));
        assert!(!bytes.is_empty());
    }

    #[test]
    fn runtime_choice_binding_and_session_records_archive() {
        let grant = RuntimeCapabilityGrant {
            abi_version: SDK_WIRE_ABI_VERSION,
            flags: 1,
            grant_id: [1; 32],
            profile_id: [2; 32],
            user_id: [3; 32],
            app_id: [4; 32],
            release_id: [5; 32],
            capability_kind: CAPABILITY_KIND_STORAGE,
            operation: CAPABILITY_OPERATION_WRITE,
            min_assurance: 2,
            scope_sha256: [6; 32],
            constraints_sha256: [7; 32],
            valid_from: 10,
            valid_until: 20,
            user_signature: vec![8; 64],
        };
        let storage = RuntimeStorageBinding {
            abi_version: SDK_WIRE_ABI_VERSION,
            flags: 1,
            binding_id: [9; 32],
            grant_id: grant.grant_id,
            profile_id: grant.profile_id,
            app_id: grant.app_id,
            release_id: grant.release_id,
            namespace: b"chat".to_vec(),
            provider_id: [10; 32],
            capability_id: [11; 32],
            backing_kind: RUNTIME_STORAGE_BACKING_BROWSER,
            scope_sha256: grant.scope_sha256,
        };
        let network = RuntimeNetworkBinding {
            abi_version: SDK_WIRE_ABI_VERSION,
            flags: 1,
            binding_id: [12; 32],
            grant_id: grant.grant_id,
            profile_id: grant.profile_id,
            app_id: grant.app_id,
            release_id: grant.release_id,
            provider_id: [13; 32],
            capability_id: [14; 32],
            binding_kind: RUNTIME_NETWORK_BINDING_FETCH,
            protocol: RUNTIME_PROTOCOL_HTTPS,
            port: 443,
            origin: b"https://api.example.com".to_vec(),
            methods_sha256: [15; 32],
            scope_sha256: grant.scope_sha256,
        };
        let session = RuntimeCapabilitySession {
            abi_version: SDK_WIRE_ABI_VERSION,
            flags: 1,
            session_id: [16; 32],
            grant_id: grant.grant_id,
            app_id: grant.app_id,
            release_id: grant.release_id,
            capability_id: [17; 32],
            provider_node_id: [18; 32],
            admission_hash: [19; 32],
            route_commitment: [20; 32],
            valid_until: 20,
            status: RUNTIME_SESSION_STATUS_OPEN,
        };
        assert!(!sdk_wire_bytes(&SdkWireRecord::RuntimeCapabilityGrant(grant)).is_empty());
        assert!(!sdk_wire_bytes(&SdkWireRecord::RuntimeStorageBinding(storage)).is_empty());
        assert!(!sdk_wire_bytes(&SdkWireRecord::RuntimeNetworkBinding(network)).is_empty());
        assert!(!sdk_wire_bytes(&SdkWireRecord::RuntimeCapabilitySession(session)).is_empty());
    }

    #[test]
    fn node_admission_cache_and_run_prompt_records_archive() {
        let route_scope = [4; 32];
        let instance = NodeInstanceRecord {
            abi_version: SDK_WIRE_ABI_VERSION,
            flags: 1,
            instance_id: node_instance_id(
                [1; 32],
                [2; 32],
                NODE_ROLE_BROWSER,
                RUNTIME_TARGET_BROWSER_WASM,
                route_scope,
            ),
            owner_id: [1; 32],
            node_id: [2; 32],
            role: NODE_ROLE_BROWSER,
            runtime_target: RUNTIME_TARGET_BROWSER_WASM,
            policy_hash: [3; 32],
            admitted_budget: 100,
            route_scope_sha256: route_scope,
            valid_from: 10,
            valid_until: 20,
            status: NODE_INSTANCE_STATUS_RUNNING,
        };
        let admission = AdmissionPolicyRecord {
            abi_version: SDK_WIRE_ABI_VERSION,
            flags: 1,
            policy_id: admission_policy_id(
                [1; 32],
                [5; 32],
                [6; 32],
                ADMISSION_POLICY_SOURCE_USER,
                10,
                20,
            ),
            owner_id: [1; 32],
            admission_node_id: [5; 32],
            source: ADMISSION_POLICY_SOURCE_USER,
            policy_hash: [6; 32],
            inherited_policy_hashes: vec![[7; 32]],
            max_budget: 1000,
            valid_from: 10,
            valid_until: 20,
            signature: vec![8; 64],
        };
        let cache = PackageCacheRecord {
            abi_version: SDK_WIRE_ABI_VERSION,
            flags: 1,
            cache_id: package_cache_id([1; 32], [2; 32], [3; 32], [4; 32], [5; 32]),
            profile_id: [1; 32],
            app_id: [2; 32],
            release_id: [3; 32],
            package_sha256: [4; 32],
            manifest_sha256: [5; 32],
            code_sha256: [6; 32],
            cached_bytes: 4096,
            state: PACKAGE_CACHE_STATE_VERIFIED,
            verified_at: 42,
            source_admission_hash: [7; 32],
        };
        let decision = AppRunPromptDecisionRecord {
            abi_version: SDK_WIRE_ABI_VERSION,
            flags: 1,
            decision_id: app_run_prompt_decision_id(
                [1; 32],
                [2; 32],
                [3; 32],
                [4; 32],
                [5; 32],
                APP_RUN_DECISION_VERIFY_AND_CACHE,
                42,
            ),
            profile_id: [1; 32],
            app_id: [2; 32],
            release_id: [3; 32],
            package_sha256: [4; 32],
            manifest_sha256: [5; 32],
            retrieval_cost: 33,
            decision: APP_RUN_DECISION_VERIFY_AND_CACHE,
            decided_at: 42,
            user_signature: vec![9; 64],
        };
        assert!(!sdk_wire_bytes(&SdkWireRecord::NodeInstance(instance)).is_empty());
        assert!(!sdk_wire_bytes(&SdkWireRecord::AdmissionPolicy(admission)).is_empty());
        assert!(!sdk_wire_bytes(&SdkWireRecord::PackageCache(cache)).is_empty());
        assert!(!sdk_wire_bytes(&SdkWireRecord::AppRunPromptDecision(decision)).is_empty());
    }

    #[test]
    fn runtime_app_and_http_records_archive_through_rkyv() {
        let route = RuntimeHttpRoute {
            abi_version: SDK_WIRE_ABI_VERSION,
            flags: 1,
            app_id: [1; 32],
            release_id: [2; 32],
            scheme: ROUTE_SCHEME_HTTPS,
            host: b"example.com".to_vec(),
            path_prefix: b"/app/".to_vec(),
        };
        let runtime_projection = RuntimeAppInstall {
            abi_version: SDK_WIRE_ABI_VERSION,
            flags: 1,
            app_id: [1; 32],
            release_id: [2; 32],
            code_sha256: [3; 32],
            developer_id: [4; 32],
            manifest_sha256: [5; 32],
            declared_routes: vec![route.clone()],
            storage_namespaces: vec![b"private".to_vec()],
            provided_capabilities: vec![RuntimeCapabilityDeclaration {
                abi_version: SDK_WIRE_ABI_VERSION,
                flags: 1,
                capability_kind: CAPABILITY_KIND_STORAGE,
                operation: CAPABILITY_OPERATION_WRITE,
                min_assurance: 1,
                scope_sha256: [9; 32],
                label: b"mailbox.delivery".to_vec(),
                context: b"runtime-owned-provider".to_vec(),
            }],
            required_capabilities: vec![RuntimeCapabilityDeclaration {
                abi_version: SDK_WIRE_ABI_VERSION,
                flags: 1,
                capability_kind: CAPABILITY_KIND_NETWORK,
                operation: CAPABILITY_OPERATION_RECEIVE,
                min_assurance: 1,
                scope_sha256: [10; 32],
                label: b"smtp.ingress".to_vec(),
                context: b"node-routed".to_vec(),
            }],
        };
        let request = RuntimeHttpRequest {
            abi_version: SDK_WIRE_ABI_VERSION,
            flags: 1,
            method: HTTP_METHOD_GET,
            scheme: ROUTE_SCHEME_HTTPS,
            host: b"example.com".to_vec(),
            path: b"/app/index.html".to_vec(),
            header_sha256: [6; 32],
            body_sha256: [7; 32],
            body: Vec::new(),
        };
        let dispatch = RuntimeHttpDispatch {
            abi_version: SDK_WIRE_ABI_VERSION,
            flags: 1,
            status: CAPABILITY_STATUS_OK,
            app_id: [1; 32],
            release_id: [2; 32],
            route_host: route.host.clone(),
            route_path_prefix: route.path_prefix.clone(),
            request_sha256: [8; 32],
        };
        let app_graph = AppGraphRecord {
            abi_version: SDK_WIRE_ABI_VERSION,
            flags: 1,
            app_id: runtime_projection.app_id,
            developer_public_key: runtime_projection.developer_id,
            app_manifest_sha256: runtime_projection.manifest_sha256,
            app_slug: b"example-app".to_vec(),
            runtime_install: runtime_projection.clone(),
            artifacts: vec![AppArtifactRecord {
                kind: 1,
                path: b"app.edapp".to_vec(),
                sha256: runtime_projection.manifest_sha256,
            }],
        };
        assert!(!sdk_wire_bytes(&SdkWireRecord::RuntimeAppInstall(runtime_projection)).is_empty());
        assert!(!sdk_wire_bytes(&SdkWireRecord::AppGraph(app_graph)).is_empty());
        assert!(
            !sdk_wire_bytes(&SdkWireRecord::RuntimeCapabilityDeclaration(
                RuntimeCapabilityDeclaration {
                    abi_version: SDK_WIRE_ABI_VERSION,
                    flags: 1,
                    capability_kind: CAPABILITY_KIND_STORAGE,
                    operation: CAPABILITY_OPERATION_READ,
                    min_assurance: 1,
                    scope_sha256: [11; 32],
                    label: b"mailbox.read".to_vec(),
                    context: b"node-authorized".to_vec(),
                }
            ))
            .is_empty()
        );
        assert!(!sdk_wire_bytes(&SdkWireRecord::RuntimeHttpRoute(route)).is_empty());
        assert!(!sdk_wire_bytes(&SdkWireRecord::RuntimeHttpRequest(request)).is_empty());
        assert!(!sdk_wire_bytes(&SdkWireRecord::RuntimeHttpDispatch(dispatch)).is_empty());
    }

    #[test]
    fn app_store_records_archive_through_rkyv() {
        let runtime_projection = RuntimeAppInstall {
            abi_version: SDK_WIRE_ABI_VERSION,
            flags: 1,
            app_id: [1; 32],
            release_id: [2; 32],
            code_sha256: [3; 32],
            developer_id: [4; 32],
            manifest_sha256: [5; 32],
            declared_routes: Vec::new(),
            storage_namespaces: Vec::new(),
            provided_capabilities: Vec::new(),
            required_capabilities: Vec::new(),
        };
        let app_graph = AppGraphRecord {
            abi_version: SDK_WIRE_ABI_VERSION,
            flags: 1,
            app_id: runtime_projection.app_id,
            developer_public_key: runtime_projection.developer_id,
            app_manifest_sha256: runtime_projection.manifest_sha256,
            app_slug: b"example-app".to_vec(),
            runtime_install: runtime_projection.clone(),
            artifacts: vec![AppArtifactRecord {
                kind: 1,
                path: b"app.edapp".to_vec(),
                sha256: runtime_projection.manifest_sha256,
            }],
        };
        let app_manifest = AppManifestRecord {
            abi_version: SDK_WIRE_ABI_VERSION,
            flags: 1,
            app_id: runtime_projection.app_id,
            developer_id: runtime_projection.developer_id,
            app_slug: b"example-app".to_vec(),
            name: b"Example App".to_vec(),
            version: b"0.1.0".to_vec(),
            summary: b"Example rkyv app manifest".to_vec(),
            code_sha256: runtime_projection.code_sha256,
            routes: vec![AppHttpRouteRecord {
                scheme: ROUTE_SCHEME_HTTPS,
                host: b"example.com".to_vec(),
                path_prefix: b"/app".to_vec(),
            }],
            storage_namespaces: vec![b"example-app/state".to_vec()],
            provided_capabilities: Vec::new(),
            required_capabilities: Vec::new(),
        };
        let asset = AppStoreAssetRef {
            path: b"index.html".to_vec(),
            sha256: [6; 32],
            bytes: 128,
        };
        let entry = AppStoreCatalogEntry {
            app_id: runtime_projection.app_id,
            release_id: runtime_projection.release_id,
            developer_id: runtime_projection.developer_id,
            app_graph_sha256: [7; 32],
            manifest_sha256: runtime_projection.manifest_sha256,
            package_sha256: [8; 32],
            package_bytes: 4096,
            status: APP_STORE_SUBMISSION_STATUS_PUBLISHED,
            name: b"Example".to_vec(),
            version: b"0.1.0".to_vec(),
            summary: b"Example app".to_vec(),
            app_slug: b"example-app".to_vec(),
            package_ref: b"edgerun://store/apps/example/app.eapp".to_vec(),
            manifest_ref: b"edgerun://store/apps/example/app.edapp".to_vec(),
            launch_ref: b"edgerun://store/apps/example/index.html".to_vec(),
            required_capabilities: vec![b"browser.wasm.execute".to_vec()],
            optional_capabilities: Vec::new(),
            asset_refs: vec![asset],
        };
        let catalog = AppStoreCatalogRecord {
            abi_version: SDK_WIRE_ABI_VERSION,
            flags: 1,
            store_id: [9; 32],
            generated_at: 10,
            sequence: 1,
            previous_catalog_sha256: [0; 32],
            entries: vec![entry.clone()],
            signature: vec![11; 64],
        };
        let submission = AppStoreSubmissionRecord {
            abi_version: SDK_WIRE_ABI_VERSION,
            flags: 1,
            submitted_at: 12,
            app_id: runtime_projection.app_id,
            release_id: runtime_projection.release_id,
            developer_id: runtime_projection.developer_id,
            app_graph_sha256: [7; 32],
            manifest_sha256: runtime_projection.manifest_sha256,
            package_sha256: [8; 32],
            package_bytes: 4096,
            app_slug: b"example-app".to_vec(),
            package_ref: b"edgerun://store/submissions/example/app.eapp".to_vec(),
            manifest_ref: b"edgerun://store/submissions/example/app.edapp".to_vec(),
            notes: Vec::new(),
            app_graph,
            developer_signature: vec![13; 64],
        };
        let review = AppStoreReviewRecord {
            abi_version: SDK_WIRE_ABI_VERSION,
            flags: 1,
            reviewed_at: 14,
            decision: APP_STORE_REVIEW_DECISION_ACCEPT,
            app_id: runtime_projection.app_id,
            release_id: runtime_projection.release_id,
            submission_sha256: [15; 32],
            reviewer_id: [16; 32],
            reason: b"accepted".to_vec(),
            signature: vec![17; 64],
        };
        assert!(!sdk_wire_bytes(&SdkWireRecord::AppStoreCatalog(catalog)).is_empty());
        assert!(!sdk_wire_bytes(&SdkWireRecord::AppManifest(app_manifest)).is_empty());
        assert!(!sdk_wire_bytes(&SdkWireRecord::AppStoreCatalogEntry(entry)).is_empty());
        assert!(!sdk_wire_bytes(&SdkWireRecord::AppStoreSubmission(submission)).is_empty());
        assert!(!sdk_wire_bytes(&SdkWireRecord::AppStoreReview(review)).is_empty());
    }

    #[test]
    fn runtime_identity_route_and_routed_message_archive_through_rkyv() {
        let message = RuntimeAppMessage {
            abi_version: SDK_WIRE_ABI_VERSION,
            flags: 1,
            from_app_id: [1; 32],
            to_app_id: [2; 32],
            message_kind: 7,
            payload_sha256: [3; 32],
            payload: b"hello".to_vec(),
        };
        let route = RuntimeIdentityRoute {
            abi_version: SDK_WIRE_ABI_VERSION,
            flags: 1,
            identity_id: [2; 32],
            runtime_id: [4; 32],
            node_id: [5; 64],
            valid_from: 10,
            valid_until: 20,
            route_hint: b"mesh".to_vec(),
        };
        let routed = RuntimeRoutedAppMessage {
            abi_version: SDK_WIRE_ABI_VERSION,
            flags: 1,
            from_runtime_id: [6; 32],
            to_runtime_id: route.runtime_id,
            to_node_id: route.node_id,
            message_sha256: [7; 32],
            message,
        };
        assert!(!sdk_wire_bytes(&SdkWireRecord::RuntimeIdentityRoute(route)).is_empty());
        assert!(!sdk_wire_bytes(&SdkWireRecord::RuntimeRoutedAppMessage(routed)).is_empty());
    }

    #[test]
    fn runtime_deployment_config_archives_protocol_surfaces() {
        let route = RuntimeHttpRoute {
            abi_version: SDK_WIRE_ABI_VERSION,
            flags: 1,
            app_id: [1; 32],
            release_id: [2; 32],
            scheme: ROUTE_SCHEME_HTTPS,
            host: b"example.com".to_vec(),
            path_prefix: b"/".to_vec(),
        };
        let mailbox = RuntimeMailbox {
            abi_version: SDK_WIRE_ABI_VERSION,
            flags: 1,
            address: b"admin@example.com".to_vec(),
            target_app_id: [1; 32],
        };
        let domain = RuntimeDomainConfig {
            abi_version: SDK_WIRE_ABI_VERSION,
            flags: 1,
            domain: b"example.com".to_vec(),
            authoritative_dns: true,
            mail_enabled: true,
            acme_enabled: true,
            mailboxes: vec![mailbox],
            routes: vec![route],
        };
        let config = RuntimeDeploymentConfig {
            abi_version: SDK_WIRE_ABI_VERSION,
            flags: 1,
            runtime_id: [9; 32],
            public_ipv4: [203, 0, 113, 10],
            hostname: b"runtime.example.com".to_vec(),
            origin: b"example.com".to_vec(),
            acme_contact: b"admin@example.com".to_vec(),
            protocol_bindings: vec![
                RuntimeProtocolBinding {
                    abi_version: SDK_WIRE_ABI_VERSION,
                    flags: 1,
                    protocol: RUNTIME_PROTOCOL_DNS_UDP,
                    port: 53,
                    bind_ipv4: [0, 0, 0, 0],
                    host: Vec::new(),
                },
                RuntimeProtocolBinding {
                    abi_version: SDK_WIRE_ABI_VERSION,
                    flags: 1,
                    protocol: RUNTIME_PROTOCOL_HTTPS,
                    port: 443,
                    bind_ipv4: [0, 0, 0, 0],
                    host: Vec::new(),
                },
            ],
            domains: vec![domain],
            apps: Vec::new(),
        };
        let bytes = sdk_wire_bytes(&SdkWireRecord::RuntimeDeploymentConfig(config));
        assert!(!bytes.is_empty());
    }
}
