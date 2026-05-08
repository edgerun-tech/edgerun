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

#[derive(Clone, Debug, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[rkyv(crate = rkyv)]
pub struct CapabilityGrantIdSeed {
    pub request_id: Vec<u8>,
    pub provider_name: Vec<u8>,
    pub provider_instance_id: Vec<u8>,
    pub nonce: u64,
    pub unix_secs: u64,
    pub unix_nanos: u32,
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
    UserCapabilityGrant(UserCapabilityGrant),
    RuntimeEvent(RuntimeEvent),
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
            app_id: install.app_id,
            developer_public_key: install.developer_id,
            app_manifest_sha256: install.manifest_sha256,
            app_slug: b"example-app".to_vec(),
            runtime_install: install.clone(),
            artifacts: vec![AppArtifactRecord {
                kind: 1,
                path: b"app.edapp".to_vec(),
                sha256: install.manifest_sha256,
            }],
        };
        assert!(!sdk_wire_bytes(&SdkWireRecord::RuntimeAppInstall(install)).is_empty());
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
        let install = RuntimeAppInstall {
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
            app_id: install.app_id,
            developer_public_key: install.developer_id,
            app_manifest_sha256: install.manifest_sha256,
            app_slug: b"example-app".to_vec(),
            runtime_install: install.clone(),
            artifacts: vec![AppArtifactRecord {
                kind: 1,
                path: b"app.edapp".to_vec(),
                sha256: install.manifest_sha256,
            }],
        };
        let app_manifest = AppManifestRecord {
            abi_version: SDK_WIRE_ABI_VERSION,
            flags: 1,
            app_id: install.app_id,
            developer_id: install.developer_id,
            app_slug: b"example-app".to_vec(),
            name: b"Example App".to_vec(),
            version: b"0.1.0".to_vec(),
            summary: b"Example rkyv app manifest".to_vec(),
            code_sha256: install.code_sha256,
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
            app_id: install.app_id,
            release_id: install.release_id,
            developer_id: install.developer_id,
            app_graph_sha256: [7; 32],
            manifest_sha256: install.manifest_sha256,
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
            app_id: install.app_id,
            release_id: install.release_id,
            developer_id: install.developer_id,
            app_graph_sha256: [7; 32],
            manifest_sha256: install.manifest_sha256,
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
            app_id: install.app_id,
            release_id: install.release_id,
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
