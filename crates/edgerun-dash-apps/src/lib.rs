use edgerun_core::crypto::{
    record_hash, HASH_DOMAIN_CAPABILITY_DESCRIPTOR, HASH_DOMAIN_QUERY_REQUEST,
};
use edgerun_core::protocol::{
    canonical_bytes, CapabilityDescriptor, ConstraintSet, IdentityRef, ProtocolRecord,
    QueryRequest, ScopeDescriptor,
};
use edgerun_encoding::hex::bytes_to_hex;
use edgerun_proto::edgerun::v0::access::{ProofClass, QueryClass};
use edgerun_proto::edgerun::v0::common::IdentityKind;
use edgerun_proto::edgerun::v0::trust::{
    CapabilityKind, DelegationPolicy, ExportPolicy, ScopeKind,
};
use std::format;

#[link(wasm_import_module = "edgerun_host")]
extern "C" {
    fn log(ptr: *const u8, len: usize);
    fn capability_request(ptr: *const u8, len: usize) -> i32;
    fn query(ptr: *const u8, len: usize) -> i32;
}

const ABI_VERSION: u32 = 1;
const APP_IDENTITY: &[u8] = b"browser-node:dash:edgerun-dash-apps";

#[no_mangle]
pub extern "C" fn edgerun_app_start() {
    emit_log("edgerun dash wasm app started with v0 protocol records");

    let capability = app_capability();
    let capability_bytes = canonical_bytes(
        &ProtocolRecord::CapabilityDescriptor(capability.clone()),
        true,
    );
    let capability_hash = record_hash(
        HASH_DOMAIN_CAPABILITY_DESCRIPTOR,
        capability_bytes.as_slice(),
    );
    let capability_message = format!(
        "{{\"kind\":\"capability_request\",\"protocol\":\"edgerun.v0\",\"record_family\":\"CapabilityDescriptor\",\"encoding\":\"protobuf-canonical-hex\",\"hash\":\"{}\",\"bytes\":\"{}\"}}",
        bytes_to_hex(&capability_hash),
        bytes_to_hex(&capability_bytes)
    );
    emit_capability_request(&capability_message);

    let query_request = app_query_request();
    let query_bytes = canonical_bytes(&ProtocolRecord::QueryRequest(query_request), true);
    let query_hash = record_hash(HASH_DOMAIN_QUERY_REQUEST, query_bytes.as_slice());
    let query_message = format!(
        "{{\"kind\":\"query\",\"protocol\":\"edgerun.v0\",\"record_family\":\"QueryRequest\",\"encoding\":\"protobuf-canonical-hex\",\"hash\":\"{}\",\"bytes\":\"{}\"}}",
        bytes_to_hex(&query_hash),
        bytes_to_hex(&query_bytes)
    );
    emit_query(&query_message);
}

#[no_mangle]
pub extern "C" fn edgerun_app_abi_version() -> u32 {
    ABI_VERSION
}

fn app_identity() -> IdentityRef {
    IdentityRef {
        identity_id: APP_IDENTITY.to_vec(),
        identity_kind: Some(IdentityKind::Agent as i32),
        key_hint: None,
    }
}

fn app_capability() -> CapabilityDescriptor {
    CapabilityDescriptor {
        capability_version: 1,
        capability_kind: CapabilityKind::Query as i32,
        actions: vec!["query".into(), "read".into()],
        scope: Some(ScopeDescriptor {
            scope_version: 1,
            scope_kind: ScopeKind::Domain as i32,
            target_nodes: vec![],
            target_streams: vec![],
            target_object_kinds: vec![],
            target_view_types: vec!["dashboard-surface".into()],
            target_domains: vec![
                "blog://edgerun.tech/*".into(),
                "git://edgerun_core/*".into(),
            ],
            time_bounds: None,
            scope_metadata: None,
        }),
        constraints: Some(ConstraintSet {
            constraint_version: 1,
            not_before: None,
            expires_at: None,
            max_uses: None,
            rate_limit: None,
            requires_local_session: Some(true),
            requires_user_presence: Some(false),
            requires_transport_classes: vec![],
            requires_location_classes: vec!["browser-node".into()],
            export_policy: ExportPolicy::QueryOnly as i32,
            execution_class_limits: vec![],
            storage_class_limits: vec![],
            constraint_metadata: None,
        }),
        delegation_policy: DelegationPolicy::NonDelegable as i32,
        minimum_assurance: None,
        capability_metadata: None,
    }
}

fn app_query_request() -> QueryRequest {
    QueryRequest {
        request_version: 1,
        query_id: b"dash-bootstrap-query".to_vec(),
        requester: Some(app_identity()),
        target_scope: app_capability().scope,
        query_class: QueryClass::View as i32,
        time_window: None,
        checkpoint_base: None,
        result_limit: Some(32),
        cost_limit: None,
        required_proof_classes: vec![ProofClass::Signature as i32],
        query_payload_object: None,
        signature: None,
    }
}

fn emit_log(message: &str) {
    unsafe { log(message.as_ptr(), message.len()) };
}

fn emit_capability_request(message: &str) {
    unsafe {
        let _ = capability_request(message.as_ptr(), message.len());
    }
}

fn emit_query(message: &str) {
    unsafe {
        let _ = query(message.as_ptr(), message.len());
    }
}
