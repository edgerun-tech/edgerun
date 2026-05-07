#![cfg_attr(not(feature = "std"), no_std)]

//! Edgerun wire protocol boundary.
//!
//! The only supported internal wire protocol is rkyv. Legacy structural wire
//! APIs have intentionally been removed so old call sites fail at compile time
//! instead of continuing on a second wire format.

extern crate alloc;

use alloc::vec::Vec;

pub const WIRE_PROTOCOL: &str = "rkyv";

pub use rkyv::rancor::Error as WireError;
pub use rkyv::*;

pub const SDK_WIRE_ABI_VERSION: u16 = 2;

pub const CAPABILITY_KIND_SIGNING: u16 = 1;
pub const CAPABILITY_KIND_SEALING: u16 = 2;
pub const CAPABILITY_KIND_PAYMENT: u16 = 3;
pub const CAPABILITY_KIND_STORAGE: u16 = 4;
pub const CAPABILITY_KIND_NETWORK: u16 = 5;

pub const CAPABILITY_OPERATION_SIGN: u16 = 1;
pub const CAPABILITY_OPERATION_VERIFY: u16 = 2;
pub const CAPABILITY_OPERATION_SEAL: u16 = 3;
pub const CAPABILITY_OPERATION_UNSEAL: u16 = 4;
pub const CAPABILITY_OPERATION_AUTHORIZE: u16 = 5;
pub const CAPABILITY_OPERATION_READ: u16 = 6;
pub const CAPABILITY_OPERATION_WRITE: u16 = 7;
pub const CAPABILITY_OPERATION_SEND: u16 = 8;
pub const CAPABILITY_OPERATION_RECEIVE: u16 = 9;

pub const CAPABILITY_STATUS_OK: u16 = 0;
pub const CAPABILITY_STATUS_POLICY_DENIED: u16 = 1;
pub const CAPABILITY_STATUS_INVALID_REQUEST: u16 = 2;
pub const CAPABILITY_STATUS_PROVIDER_FAILED: u16 = 3;

pub const RUNTIME_EVENT_PROFILE_OPENED: u16 = 1;
pub const RUNTIME_EVENT_PROFILE_ROLLBACK_REJECTED: u16 = 2;
pub const RUNTIME_EVENT_CAPABILITY_ALLOWED: u16 = 3;
pub const RUNTIME_EVENT_CAPABILITY_DENIED: u16 = 4;
pub const RUNTIME_EVENT_CAPABILITY_EXECUTED: u16 = 5;
pub const RUNTIME_EVENT_APP_INSTALLED: u16 = 6;
pub const RUNTIME_EVENT_ROUTE_GRANTED: u16 = 7;
pub const RUNTIME_EVENT_HTTP_DISPATCHED: u16 = 8;
pub const RUNTIME_EVENT_APP_MESSAGE_DISPATCHED: u16 = 9;
pub const RUNTIME_EVENT_IDENTITY_ROUTE_GRANTED: u16 = 10;
pub const RUNTIME_EVENT_APP_MESSAGE_FORWARDED: u16 = 11;

pub const HTTP_METHOD_GET: u16 = 1;
pub const HTTP_METHOD_POST: u16 = 2;
pub const HTTP_METHOD_PUT: u16 = 3;
pub const HTTP_METHOD_DELETE: u16 = 4;
pub const HTTP_METHOD_PATCH: u16 = 5;

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

impl CapabilityResponse {
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
    pub body_sha256: [u8; 32],
    pub sealed_body: Vec<u8>,
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
    pub grants: Vec<UserCapabilityGrant>,
    pub signature: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[rkyv(crate = rkyv)]
pub struct UserCapabilityGrant {
    pub capability_kind: u16,
    pub operation: u16,
    pub min_assurance: u16,
    pub flags: u16,
    pub valid_from: u64,
    pub valid_until: u64,
    pub app_id: [u8; 32],
    pub release_id: [u8; 32],
    pub scope_sha256: [u8; 32],
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
pub struct AppGraphRecord {
    pub abi_version: u16,
    pub flags: u32,
    pub app_id: [u8; 32],
    pub developer_public_key: [u8; 32],
    pub app_manifest_sha256: [u8; 32],
    pub app_slug: Vec<u8>,
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
pub enum SdkWireRecord {
    CapabilityRequest(CapabilityRequest),
    CapabilityResponse(CapabilityResponse),
    UserProfile(UserProfile),
    UserProfileBody(UserProfileBody),
    UserCapabilityGrant(UserCapabilityGrant),
    RuntimeEvent(RuntimeEvent),
    RuntimeAppInstall(RuntimeAppInstall),
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
    AppGraph(AppGraphRecord),
    AppArtifact(AppArtifactRecord),
    Entitlement(EntitlementRecord),
    Product(ProductRecord),
    Settlement(SettlementRecord),
    Payment(PaymentRecord),
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
            grants: vec![UserCapabilityGrant {
                capability_kind: CAPABILITY_KIND_SEALING,
                operation: CAPABILITY_OPERATION_UNSEAL,
                min_assurance: 2,
                flags: 0,
                valid_from: 10,
                valid_until: 20,
                app_id: [3; 32],
                release_id: [4; 32],
                scope_sha256: [5; 32],
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
        let install = RuntimeAppInstall {
            abi_version: SDK_WIRE_ABI_VERSION,
            flags: 1,
            app_id: [1; 32],
            release_id: [2; 32],
            code_sha256: [3; 32],
            developer_id: [4; 32],
            manifest_sha256: [5; 32],
            declared_routes: vec![route.clone()],
            storage_namespaces: vec![b"private".to_vec()],
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
        assert!(!sdk_wire_bytes(&SdkWireRecord::RuntimeAppInstall(install)).is_empty());
        assert!(!sdk_wire_bytes(&SdkWireRecord::RuntimeHttpRoute(route)).is_empty());
        assert!(!sdk_wire_bytes(&SdkWireRecord::RuntimeHttpRequest(request)).is_empty());
        assert!(!sdk_wire_bytes(&SdkWireRecord::RuntimeHttpDispatch(dispatch)).is_empty());
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
