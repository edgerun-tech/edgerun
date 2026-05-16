#![no_std]

extern crate alloc;

use alloc::boxed::Box;
use alloc::{string::String, vec, vec::Vec};
#[cfg(all(target_arch = "wasm32", not(test)))]
use core::alloc::{GlobalAlloc, Layout};
#[cfg(all(target_arch = "wasm32", not(test)))]
use core::arch::wasm32::{memory_grow, memory_size};
use core::{ptr, slice};

pub use edgerun_browser_authoring::{
    build_publishable_app, first_run_input_wire_bytes, first_run_projection_for_package,
    first_run_projection_record, first_run_projection_wire_bytes_for_package,
    package_retrieval_wire_bytes_for_package, BrowserAppArtifact, BrowserAppFirstRunInput,
    BrowserAppSpec, BrowserPackageRetrievalInput,
};
pub use edgerun_browser_host::{
    edgerun_browser_host_bind_storage, edgerun_browser_host_drop, edgerun_browser_host_free,
    edgerun_browser_host_grant, edgerun_browser_host_invoke_storage, edgerun_browser_host_new,
    edgerun_browser_host_open_session, edgerun_browser_host_record_first_run,
    edgerun_browser_host_record_first_run_projection, BrowserHost, StorageInvocationContext,
    BROWSER_HOST_STATUS_OK,
};
pub use edgerun_browser_runtime::{
    runtime_app_projection, runtime_capability_grant, runtime_capability_session,
    runtime_storage_binding, sha256, verify_runtime_event_chain, RuntimeKernel,
};
pub use edgerun_browser_runtime::{
    BrowserRuntimeStorage, BrowserRuntimeStorageHost, MemoryRuntimeStorage, RuntimeStorage,
};
#[cfg(all(target_arch = "wasm32", not(test)))]
#[global_allocator]
static WASM_ALLOCATOR: WasmBumpAllocator = WasmBumpAllocator;

#[cfg(all(target_arch = "wasm32", not(test)))]
struct WasmBumpAllocator;

#[cfg(all(target_arch = "wasm32", not(test)))]
static mut WASM_HEAP_CURSOR: usize = 0;
#[cfg(all(target_arch = "wasm32", not(test)))]
const WASM_PAGE_SIZE: usize = 64 * 1024;

#[cfg(all(target_arch = "wasm32", not(test)))]
unsafe impl GlobalAlloc for WasmBumpAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        unsafe extern "C" {
            static __heap_base: u8;
        }
        let base = unsafe { &__heap_base as *const u8 as usize };
        let align = layout.align();
        let size = layout.size();
        let mut cursor = unsafe { WASM_HEAP_CURSOR };
        if cursor == 0 {
            cursor = base;
        }
        let aligned = (cursor + align - 1) & !(align - 1);
        let Some(next) = aligned.checked_add(size) else {
            return ptr::null_mut();
        };
        if !ensure_wasm_memory(next) {
            return ptr::null_mut();
        }
        unsafe {
            WASM_HEAP_CURSOR = next;
        }
        aligned as *mut u8
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {}
}

#[cfg(all(target_arch = "wasm32", not(test)))]
fn ensure_wasm_memory(required_end: usize) -> bool {
    loop {
        let current_pages = memory_size(0);
        let current_bytes = current_pages.saturating_mul(WASM_PAGE_SIZE);
        if required_end <= current_bytes {
            return true;
        }
        let missing = required_end - current_bytes;
        let pages = missing.div_ceil(WASM_PAGE_SIZE);
        if memory_grow(0, pages) == usize::MAX {
            return false;
        }
    }
}

#[cfg(all(target_arch = "wasm32", not(test)))]
#[panic_handler]
fn wasm_panic(_info: &core::panic::PanicInfo<'_>) -> ! {
    core::arch::wasm32::unreachable()
}
pub use edgerun_wire::{
    sdk_wire_bytes, BrowserHostStorageInvocationRecord, BrowserPackageRetrievalEvidenceRecord,
    BrowserPackageRetrievalPolicyScheduleRecord, CapabilityRequest, SdkWireRecord,
    APP_RUN_DECISION_CANCEL, APP_RUN_DECISION_RUN_ONCE, APP_RUN_DECISION_VERIFY_AND_CACHE,
    CAPABILITY_KIND_STORAGE, CAPABILITY_OPERATION_WRITE, CAPABILITY_STATUS_OK,
    RUNTIME_STORAGE_BACKING_MEMORY, SDK_WIRE_ABI_VERSION,
};
pub use edgerun_work::capability_packet::{
    capability_envelope, capability_envelope_hash, capability_invocation_id,
    CAPABILITY_CONTENT_OBJECT, CAPABILITY_OPERATION_OBJECT_PUT, CAPABILITY_PACKET_INVOKE,
};
pub use edgerun_work::channel::{ChannelEnvelope, ChannelProof};
pub use edgerun_work::channel_order::{
    ordered_message_hash, ChannelOrderBook, OrderedChannelEnvelope,
};
pub use edgerun_work::codec::blake3_hash;
pub use edgerun_work::delivery_proof::{
    channel_proof_for_ordered, channel_proof_hash, verify_channel_proof_for_ordered,
};
pub use edgerun_work::protocol::{
    ChannelEndpoint, NodeIdentity, WorkAdmission, WorkRequest, DEPARTMENT_CAPABILITY,
    NODE_ROLE_ADMISSION, WORK_TYPE_CAPABILITY_INVOKE, WORK_WIRE_ABI_VERSION,
};
pub use edgerun_work::request_auth::{sign_work_request, verify_work_request, work_request_hash};
pub use edgerun_work::signing::{
    empty_signature, sign_work_admission, verify_work_admission, work_admission_hash,
};
pub use edgerun_work::storage_proof::{
    sign_storage_availability_proof, sign_storage_retrieval_proof, storage_availability_proof_hash,
    storage_retrieval_proof_hash, unsigned_storage_availability_proof,
    unsigned_storage_retrieval_proof, verify_storage_availability_proof,
    verify_storage_retrieval_proof,
};

pub fn smoke_package() -> edgerun_browser_authoring::BrowserAppPackage {
    build_publishable_app(BrowserAppSpec {
        slug: "browser-core-smoke",
        name: "Browser Core Smoke",
        version: "0.1.0",
        summary: "Minimal browser-first package",
        developer_seed: [7; 32],
        code_sha256: None,
        routes: Vec::new(),
        storage_namespaces: vec![b"private".as_slice()],
        provided_capabilities: Vec::new(),
        required_capabilities: Vec::new(),
        artifacts: vec![BrowserAppArtifact {
            path: "app.wasm",
            bytes: b"wasm",
        }],
    })
    .expect("static smoke package spec is valid")
}

pub const fn smoke_runtime_id() -> [u8; 32] {
    [2; 32]
}

pub struct BrowserCoreSmokeWireBytes {
    pub runtime_id: Vec<u8>,
    pub app_manifest: Vec<u8>,
    pub app_graph: Vec<u8>,
    pub developer_signature: Vec<u8>,
    pub first_run_input: Vec<u8>,
    pub first_run_projection: Vec<u8>,
    pub runtime_projection: Vec<u8>,
    pub decision: Vec<u8>,
    pub cache: Vec<u8>,
    pub grant: Vec<u8>,
    pub storage_binding: Vec<u8>,
    pub session: Vec<u8>,
    pub storage_request: Vec<u8>,
    pub storage_invocation: Vec<u8>,
}

struct BrowserCoreSmokeCapabilityBytes {
    grant: Vec<u8>,
    storage_binding: Vec<u8>,
    session: Vec<u8>,
    storage_request: Vec<u8>,
    storage_invocation: Vec<u8>,
}

fn browser_core_smoke_capability_bytes() -> BrowserCoreSmokeCapabilityBytes {
    let runtime_id = smoke_runtime_id();
    let package = smoke_package();
    let namespace = b"private".to_vec();
    let grant = runtime_capability_grant(
        [1; 32],
        [9; 32],
        package.app_id,
        package.release_id,
        CAPABILITY_KIND_STORAGE,
        CAPABILITY_OPERATION_WRITE,
        2,
        sha256(&namespace),
        sha256(b"constraints"),
        12,
        100,
        b"user-grant".to_vec(),
    );
    let capability_id = sha256(b"object-store");
    let binding = runtime_storage_binding(
        &grant,
        namespace.clone(),
        sha256(b"provider"),
        capability_id,
        RUNTIME_STORAGE_BACKING_MEMORY,
    );
    let session = runtime_capability_session(
        &grant,
        capability_id,
        runtime_id,
        sha256(b"admission"),
        sha256(b"browser-local-route"),
        100,
    );
    let value = b"value".to_vec();
    let request = CapabilityRequest::new(
        CAPABILITY_KIND_STORAGE,
        CAPABILITY_OPERATION_WRITE,
        2,
        package.app_id,
        package.release_id,
        sha256(b"key"),
        sha256(&value),
        namespace,
        value,
        b"nonce".to_vec(),
    );
    let invocation = BrowserHostStorageInvocationRecord {
        abi_version: SDK_WIRE_ABI_VERSION,
        session_id: session.session_id,
        capability_id,
        source_node_id: package.app_id,
        target_node_id: runtime_id,
        sequence: 0,
        timestamp_unix_ms: 14,
        provider: b"memory-storage".to_vec(),
    };

    BrowserCoreSmokeCapabilityBytes {
        grant: sdk_wire_bytes(&SdkWireRecord::RuntimeCapabilityGrant(grant)),
        storage_binding: sdk_wire_bytes(&SdkWireRecord::RuntimeStorageBinding(binding)),
        session: sdk_wire_bytes(&SdkWireRecord::RuntimeCapabilitySession(session)),
        storage_request: sdk_wire_bytes(&SdkWireRecord::CapabilityRequest(request)),
        storage_invocation: sdk_wire_bytes(&SdkWireRecord::BrowserHostStorageInvocation(
            invocation,
        )),
    }
}

pub fn browser_core_smoke_wire_bytes() -> BrowserCoreSmokeWireBytes {
    let runtime_id = smoke_runtime_id();
    let package = smoke_package();
    let first_run_input = BrowserAppFirstRunInput {
        profile_id: [1; 32],
        runtime_id,
        previous_event_sha256: [0; 32],
        first_event_seq: 0,
        event_time: 10,
        retrieval_cost: 5,
        source_admission_hash: [3; 32],
        decision: APP_RUN_DECISION_VERIFY_AND_CACHE,
        user_signature: b"user",
    };
    let first_run_input_bytes = first_run_input_wire_bytes(first_run_input.clone());
    let first_run = first_run_projection_for_package(
        &package.app_manifest_bytes,
        &package.app_graph_bytes,
        &package.developer_signature,
        first_run_input,
    )
    .expect("static smoke package verifies");
    let first_run_projection = sdk_wire_bytes(&SdkWireRecord::BrowserAppFirstRunProjection(
        first_run_projection_record(first_run.clone()),
    ));

    let capability = browser_core_smoke_capability_bytes();

    BrowserCoreSmokeWireBytes {
        runtime_id: runtime_id.to_vec(),
        app_manifest: package.app_manifest_bytes,
        app_graph: package.app_graph_bytes,
        developer_signature: package.developer_signature,
        first_run_input: first_run_input_bytes,
        first_run_projection,
        runtime_projection: sdk_wire_bytes(&SdkWireRecord::RuntimeAppInstall(
            first_run.verified.runtime_projection,
        )),
        decision: sdk_wire_bytes(&SdkWireRecord::AppRunPromptDecision(first_run.decision)),
        cache: first_run
            .cache
            .map(|cache| sdk_wire_bytes(&SdkWireRecord::PackageCache(cache)))
            .unwrap_or_default(),
        grant: capability.grant,
        storage_binding: capability.storage_binding,
        session: capability.session,
        storage_request: capability.storage_request,
        storage_invocation: capability.storage_invocation,
    }
}

pub fn browser_core_first_run_projection_wire_bytes(
    manifest_bytes: &[u8],
    graph_bytes: &[u8],
    developer_signature_bytes: &[u8],
    input_bytes: &[u8],
) -> Result<Vec<u8>, alloc::string::String> {
    first_run_projection_wire_bytes_for_package(
        manifest_bytes,
        graph_bytes,
        developer_signature_bytes,
        input_bytes,
    )
}

pub fn browser_core_first_run_input_wire_bytes(
    profile_id: [u8; 32],
    runtime_id: [u8; 32],
    previous_event_sha256: [u8; 32],
    first_event_seq: u64,
    event_time: u64,
    retrieval_cost: u64,
    source_admission_hash: [u8; 32],
    decision: u16,
    user_signature: &[u8],
) -> Vec<u8> {
    first_run_input_wire_bytes(BrowserAppFirstRunInput {
        profile_id,
        runtime_id,
        previous_event_sha256,
        first_event_seq,
        event_time,
        retrieval_cost,
        source_admission_hash,
        decision,
        user_signature,
    })
}

pub fn browser_core_package_retrieval_wire_bytes(
    manifest_bytes: &[u8],
    graph_bytes: &[u8],
    developer_signature_bytes: &[u8],
    package_key: &[u8],
    retrieval_cost: u64,
    retrieved_at: u64,
    source_admission_hash: [u8; 32],
    retrieval_evidence_bytes: &[u8],
    proof_bytes: &[u8],
) -> Result<Vec<u8>, alloc::string::String> {
    package_retrieval_wire_bytes_for_package(
        manifest_bytes,
        graph_bytes,
        developer_signature_bytes,
        BrowserPackageRetrievalInput {
            package_key,
            retrieval_cost,
            retrieved_at,
            source_admission_hash,
            retrieval_evidence_bytes,
            proof_bytes,
        },
    )
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BrowserCoreRetrievalEvidence {
    pub evidence_bytes: Vec<u8>,
    pub source_admission_hash: [u8; 32],
    pub work_admission_bytes: Vec<u8>,
}

pub fn browser_core_admitted_package_retrieval_evidence(
    manifest_bytes: &[u8],
    graph_bytes: &[u8],
    developer_signature_bytes: &[u8],
    package_key: &[u8],
    retrieval_cost: u64,
    requested_at: u64,
    policy_schedule_bytes: &[u8],
) -> Result<BrowserCoreRetrievalEvidence, alloc::string::String> {
    let verified = edgerun_browser_authoring::verify_browser_app_package(
        manifest_bytes,
        graph_bytes,
        developer_signature_bytes,
    )?;
    let user_key = edgerun_crypto::Ed25519SigningKey::from_bytes(&[21; 32]);
    let admission_key = edgerun_crypto::Ed25519SigningKey::from_bytes(&[22; 32]);
    let storage_key = edgerun_crypto::Ed25519SigningKey::from_bytes(&[23; 32]);
    let relay_key = edgerun_crypto::Ed25519SigningKey::from_bytes(&[24; 32]);
    let payload_hash = blake3_hash(graph_bytes);
    let signed_request = sign_work_request(
        &user_key,
        WorkRequest {
            abi_version: WORK_WIRE_ABI_VERSION,
            request_id: blake3_hash(package_key),
            user: user_key.verifying_key().to_bytes(),
            user_sequence: requested_at,
            recipient: storage_key.verifying_key().to_bytes(),
            work_type: WORK_TYPE_CAPABILITY_INVOKE,
            department: DEPARTMENT_CAPABILITY,
            payload_hash,
            input_root: verified.manifest_sha256,
            max_total_cost: retrieval_cost,
            valid_until_unix_ms: requested_at.saturating_add(60_000),
            signature: empty_signature(),
        },
    );
    if !verify_work_request(&signed_request) {
        return Err(String::from("browser retrieval work request did not verify"));
    }

    let admission_node = NodeIdentity {
        node_id: admission_key.verifying_key().to_bytes(),
        role: NODE_ROLE_ADMISSION,
        public_key: admission_key.verifying_key().to_bytes(),
    };
    let route_commitment = blake3_hash(b"browser-package-retrieval-route");
    let channel_id = blake3_hash(b"browser-package-retrieval-channel");
    let policy_hash = browser_core_retrieval_policy_hash(policy_schedule_bytes)?;
    let signed_admission = sign_work_admission(
        &admission_key,
        WorkAdmission {
            abi_version: WORK_WIRE_ABI_VERSION,
            admission_id: blake3_hash(&work_request_hash(&signed_request)),
            dao_id: admission_key.verifying_key().to_bytes(),
            user: signed_request.user,
            admission_node,
            request_hash: work_request_hash(&signed_request),
            assigned_route_commitment: route_commitment,
            assigned_channel: ChannelEndpoint::new(
                channel_id,
                7,
                b"browser-storage".to_vec(),
                String::from("browser-package-retrieval"),
            ),
            assigned_relay_path: vec![relay_key.verifying_key().to_bytes()],
            admitted_budget: retrieval_cost.max(1),
            policy_hash,
            sequence: requested_at,
            valid_until_unix_ms: requested_at.saturating_add(60_000),
            signature: empty_signature(),
        },
    );
    if !verify_work_admission(&signed_admission) {
        return Err(String::from("browser retrieval admission did not verify"));
    }
    let source_admission_hash = work_admission_hash(&signed_admission);
    let work_admission_bytes = edgerun_work::codec::work_admission_packet_bytes(&signed_admission);
    let evidence = BrowserPackageRetrievalEvidenceRecord {
        abi_version: SDK_WIRE_ABI_VERSION,
        flags: 1,
        package_key: package_key.to_vec(),
        package_sha256: verified.package_sha256,
        manifest_sha256: verified.manifest_sha256,
        work_request_hash: work_request_hash(&signed_request),
        admission_hash: source_admission_hash,
        route_commitment,
        channel_id,
        policy_hash,
        user_id: signed_request.user,
        admission_node_id: signed_admission.admission_node.node_id,
        storage_node_id: storage_key.verifying_key().to_bytes(),
        admitted_budget: signed_admission.admitted_budget,
        retrieval_cost,
        requested_at,
        admitted_until: signed_admission.valid_until_unix_ms,
    };
    Ok(BrowserCoreRetrievalEvidence {
        evidence_bytes: sdk_wire_bytes(&SdkWireRecord::BrowserPackageRetrievalEvidence(evidence)),
        source_admission_hash,
        work_admission_bytes,
    })
}

pub fn browser_core_retrieval_policy_hash(
    schedule_bytes: &[u8],
) -> Result<[u8; 32], alloc::string::String> {
    if schedule_bytes.is_empty() {
        return Ok(sha256(b"browser-package-retrieval-policy"));
    }
    decode_browser_retrieval_policy_schedule(schedule_bytes)?;
    Ok(blake3_hash(schedule_bytes))
}

pub fn browser_core_package_retrieval_policy_schedule_wire_bytes(
    base_cost: u64,
    cost_per_byte: u64,
    min_cost: u64,
    max_cost: u64,
    valid_from: u64,
    valid_until: u64,
) -> Vec<u8> {
    let mut schedule_preimage = Vec::new();
    schedule_preimage.extend_from_slice(b"edgerun.browser.retrieval-policy-schedule.v1");
    schedule_preimage.extend_from_slice(&base_cost.to_le_bytes());
    schedule_preimage.extend_from_slice(&cost_per_byte.to_le_bytes());
    schedule_preimage.extend_from_slice(&min_cost.to_le_bytes());
    schedule_preimage.extend_from_slice(&max_cost.to_le_bytes());
    schedule_preimage.extend_from_slice(&valid_from.to_le_bytes());
    schedule_preimage.extend_from_slice(&valid_until.to_le_bytes());
    let policy_hash = sha256(&schedule_preimage);
    let schedule = BrowserPackageRetrievalPolicyScheduleRecord {
        abi_version: SDK_WIRE_ABI_VERSION,
        flags: 1,
        schedule_id: blake3_hash(&schedule_preimage),
        policy_hash,
        base_cost,
        cost_per_byte,
        min_cost,
        max_cost,
        valid_from,
        valid_until,
    };
    sdk_wire_bytes(&SdkWireRecord::BrowserPackageRetrievalPolicySchedule(schedule))
}

pub fn browser_core_package_retrieval_cost_from_policy_schedule(
    schedule_bytes: &[u8],
    manifest_len: u64,
    graph_len: u64,
    developer_signature_len: u64,
    requested_at: u64,
) -> Result<u64, alloc::string::String> {
    let schedule = decode_browser_retrieval_policy_schedule(schedule_bytes)?;
    if requested_at < schedule.valid_from || requested_at > schedule.valid_until {
        return Err(String::from("browser retrieval policy schedule is not valid at request time"));
    }
    let bytes = manifest_len
        .checked_add(graph_len)
        .and_then(|value| value.checked_add(developer_signature_len))
        .ok_or_else(|| String::from("browser retrieval package byte count overflowed"))?;
    let variable = bytes
        .checked_mul(schedule.cost_per_byte)
        .ok_or_else(|| String::from("browser retrieval cost overflowed"))?;
    let mut cost = schedule
        .base_cost
        .checked_add(variable)
        .ok_or_else(|| String::from("browser retrieval cost overflowed"))?;
    if cost < schedule.min_cost {
        cost = schedule.min_cost;
    }
    if schedule.max_cost != 0 && cost > schedule.max_cost {
        cost = schedule.max_cost;
    }
    Ok(cost)
}

fn decode_browser_retrieval_policy_schedule(
    schedule_bytes: &[u8],
) -> Result<BrowserPackageRetrievalPolicyScheduleRecord, alloc::string::String> {
    let mut aligned = edgerun_wire::util::AlignedVec::<16>::with_capacity(schedule_bytes.len());
    aligned.extend_from_slice(schedule_bytes);
    let record = edgerun_wire::from_bytes::<SdkWireRecord, edgerun_wire::WireError>(&aligned)
        .map_err(|_| String::from("browser retrieval policy schedule did not decode"))?;
    let SdkWireRecord::BrowserPackageRetrievalPolicySchedule(schedule) = record else {
        return Err(String::from("browser retrieval policy schedule record expected"));
    };
    if schedule.abi_version != SDK_WIRE_ABI_VERSION {
        return Err(String::from("browser retrieval policy schedule ABI mismatch"));
    }
    Ok(schedule)
}

#[unsafe(no_mangle)]
pub extern "C" fn edgerun_browser_core_abi_version() -> u32 {
    SDK_WIRE_ABI_VERSION as u32
}

#[unsafe(no_mangle)]
pub extern "C" fn edgerun_browser_core_app_run_decision_run_once() -> u32 {
    APP_RUN_DECISION_RUN_ONCE as u32
}

#[unsafe(no_mangle)]
pub extern "C" fn edgerun_browser_core_app_run_decision_verify_and_cache() -> u32 {
    APP_RUN_DECISION_VERIFY_AND_CACHE as u32
}

#[unsafe(no_mangle)]
pub extern "C" fn edgerun_browser_core_app_run_decision_cancel() -> u32 {
    APP_RUN_DECISION_CANCEL as u32
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn edgerun_browser_core_free(ptr: *mut u8, len: usize) {
    if ptr.is_null() || len == 0 {
        return;
    }
    let raw = ptr::slice_from_raw_parts_mut(ptr, len);
    unsafe {
        drop(Box::from_raw(raw));
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn edgerun_browser_core_smoke_runtime_id(
    out_ptr: *mut *mut u8,
    out_len: *mut usize,
) -> u32 {
    unsafe { write_smoke_output(smoke_runtime_id().to_vec(), out_ptr, out_len) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn edgerun_browser_core_smoke_app_manifest(
    out_ptr: *mut *mut u8,
    out_len: *mut usize,
) -> u32 {
    unsafe { write_smoke_output(smoke_package().app_manifest_bytes, out_ptr, out_len) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn edgerun_browser_core_smoke_app_graph(
    out_ptr: *mut *mut u8,
    out_len: *mut usize,
) -> u32 {
    unsafe { write_smoke_output(smoke_package().app_graph_bytes, out_ptr, out_len) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn edgerun_browser_core_smoke_developer_signature(
    out_ptr: *mut *mut u8,
    out_len: *mut usize,
) -> u32 {
    unsafe { write_smoke_output(smoke_package().developer_signature, out_ptr, out_len) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn edgerun_browser_core_smoke_first_run_input(
    out_ptr: *mut *mut u8,
    out_len: *mut usize,
) -> u32 {
    unsafe { write_smoke_output(browser_core_smoke_wire_bytes().first_run_input, out_ptr, out_len) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn edgerun_browser_core_smoke_first_run_projection(
    out_ptr: *mut *mut u8,
    out_len: *mut usize,
) -> u32 {
    unsafe {
        write_smoke_output(
            browser_core_smoke_wire_bytes().first_run_projection,
            out_ptr,
            out_len,
        )
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn edgerun_browser_core_smoke_runtime_projection(
    out_ptr: *mut *mut u8,
    out_len: *mut usize,
) -> u32 {
    unsafe {
        write_smoke_output(
            browser_core_smoke_wire_bytes().runtime_projection,
            out_ptr,
            out_len,
        )
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn edgerun_browser_core_smoke_decision(
    out_ptr: *mut *mut u8,
    out_len: *mut usize,
) -> u32 {
    unsafe { write_smoke_output(browser_core_smoke_wire_bytes().decision, out_ptr, out_len) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn edgerun_browser_core_smoke_cache(
    out_ptr: *mut *mut u8,
    out_len: *mut usize,
) -> u32 {
    unsafe { write_smoke_output(browser_core_smoke_wire_bytes().cache, out_ptr, out_len) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn edgerun_browser_core_smoke_grant(
    out_ptr: *mut *mut u8,
    out_len: *mut usize,
) -> u32 {
    unsafe { write_smoke_output(browser_core_smoke_capability_bytes().grant, out_ptr, out_len) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn edgerun_browser_core_smoke_storage_binding(
    out_ptr: *mut *mut u8,
    out_len: *mut usize,
) -> u32 {
    unsafe {
        write_smoke_output(
            browser_core_smoke_capability_bytes().storage_binding,
            out_ptr,
            out_len,
        )
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn edgerun_browser_core_smoke_session(
    out_ptr: *mut *mut u8,
    out_len: *mut usize,
) -> u32 {
    unsafe { write_smoke_output(browser_core_smoke_capability_bytes().session, out_ptr, out_len) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn edgerun_browser_core_smoke_storage_request(
    out_ptr: *mut *mut u8,
    out_len: *mut usize,
) -> u32 {
    unsafe {
        write_smoke_output(
            browser_core_smoke_capability_bytes().storage_request,
            out_ptr,
            out_len,
        )
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn edgerun_browser_core_smoke_storage_invocation(
    out_ptr: *mut *mut u8,
    out_len: *mut usize,
) -> u32 {
    unsafe {
        write_smoke_output(
            browser_core_smoke_capability_bytes().storage_invocation,
            out_ptr,
            out_len,
        )
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn edgerun_browser_core_first_run_projection(
    manifest_ptr: *const u8,
    manifest_len: usize,
    graph_ptr: *const u8,
    graph_len: usize,
    developer_signature_ptr: *const u8,
    developer_signature_len: usize,
    input_ptr: *const u8,
    input_len: usize,
    out_ptr: *mut *mut u8,
    out_len: *mut usize,
) -> u32 {
    let Some(manifest) = (unsafe { slice_from_ptr(manifest_ptr, manifest_len) }) else {
        return 1;
    };
    let Some(graph) = (unsafe { slice_from_ptr(graph_ptr, graph_len) }) else {
        return 1;
    };
    let Some(developer_signature) =
        (unsafe { slice_from_ptr(developer_signature_ptr, developer_signature_len) })
    else {
        return 1;
    };
    let Some(input) = (unsafe { slice_from_ptr(input_ptr, input_len) }) else {
        return 1;
    };
    let Ok(bytes) =
        browser_core_first_run_projection_wire_bytes(manifest, graph, developer_signature, input)
    else {
        return 2;
    };
    unsafe { write_smoke_output(bytes, out_ptr, out_len) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn edgerun_browser_core_package_retrieval(
    manifest_ptr: *const u8,
    manifest_len: usize,
    graph_ptr: *const u8,
    graph_len: usize,
    developer_signature_ptr: *const u8,
    developer_signature_len: usize,
    package_key_ptr: *const u8,
    package_key_len: usize,
    retrieval_cost: u64,
    retrieved_at: u64,
    source_admission_hash_ptr: *const u8,
    source_admission_hash_len: usize,
    retrieval_evidence_ptr: *const u8,
    retrieval_evidence_len: usize,
    proof_ptr: *const u8,
    proof_len: usize,
    out_ptr: *mut *mut u8,
    out_len: *mut usize,
) -> u32 {
    let Some(manifest) = (unsafe { slice_from_ptr(manifest_ptr, manifest_len) }) else {
        return 1;
    };
    let Some(graph) = (unsafe { slice_from_ptr(graph_ptr, graph_len) }) else {
        return 1;
    };
    let Some(developer_signature) =
        (unsafe { slice_from_ptr(developer_signature_ptr, developer_signature_len) })
    else {
        return 1;
    };
    let Some(package_key) = (unsafe { slice_from_ptr(package_key_ptr, package_key_len) }) else {
        return 1;
    };
    let Some(source_admission_hash) =
        (unsafe { hash_from_ptr(source_admission_hash_ptr, source_admission_hash_len) })
    else {
        return 1;
    };
    let Some(retrieval_evidence) =
        (unsafe { slice_from_ptr(retrieval_evidence_ptr, retrieval_evidence_len) })
    else {
        return 1;
    };
    let Some(proof) = (unsafe { slice_from_ptr(proof_ptr, proof_len) }) else {
        return 1;
    };
    let Ok(bytes) = browser_core_package_retrieval_wire_bytes(
        manifest,
        graph,
        developer_signature,
        package_key,
        retrieval_cost,
        retrieved_at,
        source_admission_hash,
        retrieval_evidence,
        proof,
    ) else {
        return 2;
    };
    unsafe { write_smoke_output(bytes, out_ptr, out_len) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn edgerun_browser_core_package_retrieval_evidence(
    manifest_ptr: *const u8,
    manifest_len: usize,
    graph_ptr: *const u8,
    graph_len: usize,
    developer_signature_ptr: *const u8,
    developer_signature_len: usize,
    package_key_ptr: *const u8,
    package_key_len: usize,
    retrieval_cost: u64,
    requested_at: u64,
    policy_schedule_ptr: *const u8,
    policy_schedule_len: usize,
    out_ptr: *mut *mut u8,
    out_len: *mut usize,
) -> u32 {
    let Some(manifest) = (unsafe { slice_from_ptr(manifest_ptr, manifest_len) }) else {
        return 1;
    };
    let Some(graph) = (unsafe { slice_from_ptr(graph_ptr, graph_len) }) else {
        return 1;
    };
    let Some(developer_signature) =
        (unsafe { slice_from_ptr(developer_signature_ptr, developer_signature_len) })
    else {
        return 1;
    };
    let Some(package_key) = (unsafe { slice_from_ptr(package_key_ptr, package_key_len) }) else {
        return 1;
    };
    let Some(policy_schedule) =
        (unsafe { slice_from_ptr(policy_schedule_ptr, policy_schedule_len) })
    else {
        return 1;
    };
    let evidence = match browser_core_admitted_package_retrieval_evidence(
        manifest,
        graph,
        developer_signature,
        package_key,
        retrieval_cost,
        requested_at,
        policy_schedule,
    ) {
        Ok(evidence) => evidence,
        Err(err) => return browser_retrieval_evidence_error_status(&err),
    };
    unsafe { write_smoke_output(evidence.evidence_bytes, out_ptr, out_len) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn edgerun_browser_core_package_retrieval_admission_hash(
    manifest_ptr: *const u8,
    manifest_len: usize,
    graph_ptr: *const u8,
    graph_len: usize,
    developer_signature_ptr: *const u8,
    developer_signature_len: usize,
    package_key_ptr: *const u8,
    package_key_len: usize,
    retrieval_cost: u64,
    requested_at: u64,
    policy_schedule_ptr: *const u8,
    policy_schedule_len: usize,
    out_ptr: *mut *mut u8,
    out_len: *mut usize,
) -> u32 {
    let Some(manifest) = (unsafe { slice_from_ptr(manifest_ptr, manifest_len) }) else {
        return 1;
    };
    let Some(graph) = (unsafe { slice_from_ptr(graph_ptr, graph_len) }) else {
        return 1;
    };
    let Some(developer_signature) =
        (unsafe { slice_from_ptr(developer_signature_ptr, developer_signature_len) })
    else {
        return 1;
    };
    let Some(package_key) = (unsafe { slice_from_ptr(package_key_ptr, package_key_len) }) else {
        return 1;
    };
    let Some(policy_schedule) =
        (unsafe { slice_from_ptr(policy_schedule_ptr, policy_schedule_len) })
    else {
        return 1;
    };
    let evidence = match browser_core_admitted_package_retrieval_evidence(
        manifest,
        graph,
        developer_signature,
        package_key,
        retrieval_cost,
        requested_at,
        policy_schedule,
    ) {
        Ok(evidence) => evidence,
        Err(err) => return browser_retrieval_evidence_error_status(&err),
    };
    unsafe { write_smoke_output(evidence.source_admission_hash.to_vec(), out_ptr, out_len) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn edgerun_browser_core_package_retrieval_work_admission(
    manifest_ptr: *const u8,
    manifest_len: usize,
    graph_ptr: *const u8,
    graph_len: usize,
    developer_signature_ptr: *const u8,
    developer_signature_len: usize,
    package_key_ptr: *const u8,
    package_key_len: usize,
    retrieval_cost: u64,
    requested_at: u64,
    policy_schedule_ptr: *const u8,
    policy_schedule_len: usize,
    out_ptr: *mut *mut u8,
    out_len: *mut usize,
) -> u32 {
    let Some(manifest) = (unsafe { slice_from_ptr(manifest_ptr, manifest_len) }) else {
        return 1;
    };
    let Some(graph) = (unsafe { slice_from_ptr(graph_ptr, graph_len) }) else {
        return 1;
    };
    let Some(developer_signature) =
        (unsafe { slice_from_ptr(developer_signature_ptr, developer_signature_len) })
    else {
        return 1;
    };
    let Some(package_key) = (unsafe { slice_from_ptr(package_key_ptr, package_key_len) }) else {
        return 1;
    };
    let Some(policy_schedule) =
        (unsafe { slice_from_ptr(policy_schedule_ptr, policy_schedule_len) })
    else {
        return 1;
    };
    let evidence = match browser_core_admitted_package_retrieval_evidence(
        manifest,
        graph,
        developer_signature,
        package_key,
        retrieval_cost,
        requested_at,
        policy_schedule,
    ) {
        Ok(evidence) => evidence,
        Err(err) => return browser_retrieval_evidence_error_status(&err),
    };
    unsafe { write_smoke_output(evidence.work_admission_bytes, out_ptr, out_len) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn edgerun_browser_core_package_retrieval_policy_schedule(
    base_cost: u64,
    cost_per_byte: u64,
    min_cost: u64,
    max_cost: u64,
    valid_from: u64,
    valid_until: u64,
    out_ptr: *mut *mut u8,
    out_len: *mut usize,
) -> u32 {
    let bytes = browser_core_package_retrieval_policy_schedule_wire_bytes(
        base_cost,
        cost_per_byte,
        min_cost,
        max_cost,
        valid_from,
        valid_until,
    );
    unsafe { write_smoke_output(bytes, out_ptr, out_len) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn edgerun_browser_core_package_retrieval_policy_cost(
    schedule_ptr: *const u8,
    schedule_len: usize,
    manifest_len: u64,
    graph_len: u64,
    developer_signature_len: u64,
    requested_at: u64,
    out_ptr: *mut *mut u8,
    out_len: *mut usize,
) -> u32 {
    let Some(schedule) = (unsafe { slice_from_ptr(schedule_ptr, schedule_len) }) else {
        return 1;
    };
    let Ok(cost) = browser_core_package_retrieval_cost_from_policy_schedule(
        schedule,
        manifest_len,
        graph_len,
        developer_signature_len,
        requested_at,
    ) else {
        return 2;
    };
    unsafe { write_smoke_output(cost.to_le_bytes().to_vec(), out_ptr, out_len) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn edgerun_browser_core_first_run_input(
    profile_id_ptr: *const u8,
    profile_id_len: usize,
    runtime_id_ptr: *const u8,
    runtime_id_len: usize,
    previous_event_sha256_ptr: *const u8,
    previous_event_sha256_len: usize,
    first_event_seq: u64,
    event_time: u64,
    retrieval_cost: u64,
    source_admission_hash_ptr: *const u8,
    source_admission_hash_len: usize,
    decision: u32,
    user_signature_ptr: *const u8,
    user_signature_len: usize,
    out_ptr: *mut *mut u8,
    out_len: *mut usize,
) -> u32 {
    let Some(profile_id) = (unsafe { hash_from_ptr(profile_id_ptr, profile_id_len) }) else {
        return 1;
    };
    let Some(runtime_id) = (unsafe { hash_from_ptr(runtime_id_ptr, runtime_id_len) }) else {
        return 1;
    };
    let Some(previous_event_sha256) =
        (unsafe { hash_from_ptr(previous_event_sha256_ptr, previous_event_sha256_len) })
    else {
        return 1;
    };
    let Some(source_admission_hash) =
        (unsafe { hash_from_ptr(source_admission_hash_ptr, source_admission_hash_len) })
    else {
        return 1;
    };
    let Some(user_signature) =
        (unsafe { slice_from_ptr(user_signature_ptr, user_signature_len) })
    else {
        return 1;
    };
    let Ok(decision) = u16::try_from(decision) else {
        return 2;
    };
    if !matches!(
        decision,
        APP_RUN_DECISION_RUN_ONCE | APP_RUN_DECISION_VERIFY_AND_CACHE | APP_RUN_DECISION_CANCEL
    ) {
        return 2;
    }
    let bytes = browser_core_first_run_input_wire_bytes(
        profile_id,
        runtime_id,
        previous_event_sha256,
        first_event_seq,
        event_time,
        retrieval_cost,
        source_admission_hash,
        decision,
        user_signature,
    );
    unsafe { write_smoke_output(bytes, out_ptr, out_len) }
}

unsafe fn write_smoke_output(bytes: Vec<u8>, out_ptr: *mut *mut u8, out_len: *mut usize) -> u32 {
    if out_ptr.is_null() || out_len.is_null() {
        return 1;
    }
    if bytes.is_empty() {
        unsafe {
            *out_ptr = ptr::null_mut();
            *out_len = 0;
        }
        return 0;
    }
    let mut boxed = bytes.into_boxed_slice();
    let ptr = boxed.as_mut_ptr();
    let len = boxed.len();
    let _ = Box::into_raw(boxed);
    unsafe {
        *out_ptr = ptr;
        *out_len = len;
    }
    0
}

fn browser_retrieval_evidence_error_status(err: &str) -> u32 {
    match err {
        "browser retrieval work request did not verify" => 21,
        "browser retrieval admission did not verify" => 22,
        "browser retrieval policy schedule did not decode" => 23,
        "browser retrieval policy schedule record expected" => 24,
        "browser retrieval policy schedule ABI mismatch" => 25,
        "browser retrieval admission bytes did not encode" => 26,
        _ => 2,
    }
}

unsafe fn slice_from_ptr<'a>(ptr: *const u8, len: usize) -> Option<&'a [u8]> {
    if len == 0 {
        return Some(&[]);
    }
    if ptr.is_null() {
        return None;
    }
    Some(unsafe { slice::from_raw_parts(ptr, len) })
}

unsafe fn hash_from_ptr(ptr: *const u8, len: usize) -> Option<[u8; 32]> {
    let bytes = unsafe { slice_from_ptr(ptr, len) }?;
    if bytes.len() != 32 {
        return None;
    }
    let mut hash = [0_u8; 32];
    hash.copy_from_slice(bytes);
    Some(hash)
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::string::ToString;
    use edgerun_crypto::Ed25519SigningKey;
    use edgerun_wire::{
        from_bytes, BrowserAppFirstRunProjectionRecord, BrowserHostResultRecord,
        BrowserPackageRetrievalEvidenceRecord, BrowserPackageRetrievalPolicyScheduleRecord,
        BrowserPackageRetrievalRecord, StorageWriteReceiptRecord,
        RUNTIME_EVENT_CAPABILITY_EXECUTED,
    };

    #[test]
    fn browser_first_slice_verifies_runs_caches_and_invokes_storage() {
        let runtime_key = Ed25519SigningKey::from_bytes(&[2; 32]);
        let runtime_id = runtime_key.verifying_key().to_bytes();
        let package = smoke_package();
        let first_run = first_run_projection_for_package(
            &package.app_manifest_bytes,
            &package.app_graph_bytes,
            &package.developer_signature,
            BrowserAppFirstRunInput {
                profile_id: [1; 32],
                runtime_id,
                previous_event_sha256: [0; 32],
                first_event_seq: 0,
                event_time: 10,
                retrieval_cost: 5,
                source_admission_hash: [3; 32],
                decision: APP_RUN_DECISION_VERIFY_AND_CACHE,
                user_signature: b"user",
            },
        )
        .expect("package verifies");

        let namespace = b"private".to_vec();
        let mut runtime = RuntimeKernel::new(MemoryRuntimeStorage::default(), runtime_id);
        runtime
            .record_app_first_run_projection(
                first_run.verified.runtime_projection.clone(),
                first_run.decision.clone(),
                first_run.cache.clone(),
                11,
            )
            .expect("runtime accepts first run projection");

        let grant = runtime_capability_grant(
            [1; 32],
            [9; 32],
            first_run.verified.app_id,
            first_run.verified.release_id,
            CAPABILITY_KIND_STORAGE,
            CAPABILITY_OPERATION_WRITE,
            2,
            sha256(&namespace),
            sha256(b"constraints"),
            12,
            100,
            b"user-grant".to_vec(),
        );
        runtime
            .grant_runtime_capability(grant.clone(), 12)
            .expect("grant");
        let capability_id = sha256(b"object-store");
        runtime
            .bind_storage_provider(
                runtime_storage_binding(
                    &grant,
                    namespace.clone(),
                    sha256(b"provider"),
                    capability_id,
                    RUNTIME_STORAGE_BACKING_MEMORY,
                ),
                12,
            )
            .expect("storage binding");
        let value = b"value".to_vec();
        let request = CapabilityRequest::new(
            CAPABILITY_KIND_STORAGE,
            CAPABILITY_OPERATION_WRITE,
            2,
            first_run.verified.app_id,
            first_run.verified.release_id,
            sha256(b"key"),
            sha256(&value),
            namespace.clone(),
            value,
            b"nonce".to_vec(),
        );
        let request_bytes = sdk_wire_bytes(&SdkWireRecord::CapabilityRequest(request));

        let user_key = Ed25519SigningKey::from_bytes(&[11; 32]);
        let admission_key = Ed25519SigningKey::from_bytes(&[12; 32]);
        let relay_key = Ed25519SigningKey::from_bytes(&[13; 32]);
        let signed_request = sign_work_request(
            &user_key,
            WorkRequest {
                abi_version: WORK_WIRE_ABI_VERSION,
                request_id: blake3_hash(b"storage-capability-request"),
                user: user_key.verifying_key().to_bytes(),
                user_sequence: 1,
                recipient: runtime_id,
                work_type: WORK_TYPE_CAPABILITY_INVOKE,
                department: DEPARTMENT_CAPABILITY,
                payload_hash: blake3_hash(&request_bytes),
                input_root: sha256(b"private/key"),
                max_total_cost: 5,
                valid_until_unix_ms: 100,
                signature: empty_signature(),
            },
        );
        assert!(verify_work_request(&signed_request));

        let admission_node = NodeIdentity {
            node_id: admission_key.verifying_key().to_bytes(),
            role: NODE_ROLE_ADMISSION,
            public_key: admission_key.verifying_key().to_bytes(),
        };
        let route_hash = blake3_hash(b"browser-local-route");
        let signed_admission = sign_work_admission(
            &admission_key,
            WorkAdmission {
                abi_version: WORK_WIRE_ABI_VERSION,
                admission_id: blake3_hash(b"storage-capability-admission"),
                dao_id: admission_key.verifying_key().to_bytes(),
                user: signed_request.user,
                admission_node,
                request_hash: work_request_hash(&signed_request),
                assigned_route_commitment: route_hash,
                assigned_channel: ChannelEndpoint::new(
                    blake3_hash(b"browser-host-channel"),
                    7,
                    b"memory".to_vec(),
                    "browser-host".to_string(),
                ),
                assigned_relay_path: vec![relay_key.verifying_key().to_bytes()],
                admitted_budget: 5,
                policy_hash: sha256(b"browser-storage-policy"),
                sequence: 1,
                valid_until_unix_ms: 100,
                signature: empty_signature(),
            },
        );
        assert!(verify_work_admission(&signed_admission));

        let session = runtime_capability_session(
            &grant,
            capability_id,
            runtime_id,
            work_admission_hash(&signed_admission),
            signed_admission.assigned_route_commitment,
            100,
        );
        runtime
            .open_capability_session(session.clone(), 13)
            .expect("session");

        let envelope = capability_envelope(
            session.session_id,
            capability_invocation_id(
                session.session_id,
                CAPABILITY_OPERATION_OBJECT_PUT,
                0,
                blake3_hash(&request_bytes),
            ),
            capability_id,
            first_run.verified.app_id,
            runtime_id,
            CAPABILITY_PACKET_INVOKE,
            CAPABILITY_OPERATION_OBJECT_PUT,
            CAPABILITY_CONTENT_OBJECT,
            0,
            14,
            request_bytes,
        );
        let packet_hash = capability_envelope_hash(&envelope);
        let mut channel_order = ChannelOrderBook::new();
        let ordered = channel_order
            .wrap_and_accept(
                ChannelEnvelope::new(
                    signed_admission.assigned_channel.channel_id,
                    first_run.verified.app_id,
                    runtime_id,
                    signed_admission.assigned_route_commitment,
                    packet_hash,
                ),
                signed_admission.assigned_route_commitment,
                packet_hash,
            )
            .expect("ordered channel envelope");
        let recipient_node = NodeIdentity {
            node_id: runtime_id,
            role: 6,
            public_key: runtime_id,
        };
        let proof = channel_proof_for_ordered(
            &runtime_key,
            &recipient_node,
            signed_admission.assigned_relay_path[0],
            &ordered,
        )
        .expect("channel proof");
        verify_channel_proof_for_ordered(
            &proof,
            &recipient_node,
            signed_admission.assigned_relay_path[0],
            &ordered,
        )
        .expect("channel proof verifies");
        assert_ne!(ordered_message_hash(&ordered), [0; 32]);
        assert_ne!(channel_proof_hash(&proof), [0; 32]);

        let response_bytes = runtime
            .invoke_storage_envelope(&envelope, b"memory-storage", 14)
            .expect("storage envelope");
        let response =
            from_bytes::<SdkWireRecord, edgerun_wire::WireError>(&response_bytes).unwrap();
        let SdkWireRecord::CapabilityResponse(response) = response else {
            panic!("expected capability response");
        };
        assert_eq!(response.status, CAPABILITY_STATUS_OK);
        let write_receipt =
            from_bytes::<StorageWriteReceiptRecord, edgerun_wire::WireError>(&response.payload)
                .expect("write receipt payload");
        let availability = sign_storage_availability_proof(
            &runtime_key,
            unsigned_storage_availability_proof(
                recipient_node.clone(),
                session.admission_hash,
                session.route_commitment,
                channel_proof_hash(&proof),
                write_receipt.payload_sha256,
                write_receipt.payload_len,
                14,
            ),
        );
        assert!(verify_storage_availability_proof(&availability));
        let retrieval = sign_storage_retrieval_proof(
            &runtime_key,
            unsigned_storage_retrieval_proof(
                recipient_node,
                storage_availability_proof_hash(&availability),
                sha256(b"read-private-key"),
                write_receipt.payload_sha256,
                write_receipt.payload_len,
                15,
            ),
        );
        assert!(verify_storage_retrieval_proof(&retrieval, &availability));
        assert_ne!(storage_retrieval_proof_hash(&retrieval), [0; 32]);
        assert_eq!(
            runtime.events().last().unwrap().event.event_kind,
            RUNTIME_EVENT_CAPABILITY_EXECUTED
        );
        assert!(verify_runtime_event_chain(runtime.events()));
    }

    #[test]
    fn browser_host_boundary_accepts_wire_bytes_and_returns_wire_result() {
        let runtime_key = Ed25519SigningKey::from_bytes(&[2; 32]);
        let runtime_id = runtime_key.verifying_key().to_bytes();
        let package = smoke_package();
        let first_run = first_run_projection_for_package(
            &package.app_manifest_bytes,
            &package.app_graph_bytes,
            &package.developer_signature,
            BrowserAppFirstRunInput {
                profile_id: [1; 32],
                runtime_id,
                previous_event_sha256: [0; 32],
                first_event_seq: 0,
                event_time: 10,
                retrieval_cost: 5,
                source_admission_hash: [3; 32],
                decision: APP_RUN_DECISION_VERIFY_AND_CACHE,
                user_signature: b"user",
            },
        )
        .expect("package verifies");

        let mut host = BrowserHost::memory(runtime_id);
        let runtime_projection_bytes = sdk_wire_bytes(&SdkWireRecord::RuntimeAppInstall(
            first_run.verified.runtime_projection.clone(),
        ));
        let decision_bytes = sdk_wire_bytes(&SdkWireRecord::AppRunPromptDecision(
            first_run.decision.clone(),
        ));
        let cache_bytes = first_run
            .cache
            .clone()
            .map(|cache| sdk_wire_bytes(&SdkWireRecord::PackageCache(cache)));
        let result = decode_host_result(
            &host
                .record_first_run_bytes(
                    &runtime_projection_bytes,
                    &decision_bytes,
                    cache_bytes.as_deref(),
                    11,
                )
                .expect("host accepts first run wire bytes"),
        );
        assert_eq!(result.status, BROWSER_HOST_STATUS_OK);

        let namespace = b"private".to_vec();
        let grant = runtime_capability_grant(
            [1; 32],
            [9; 32],
            first_run.verified.app_id,
            first_run.verified.release_id,
            CAPABILITY_KIND_STORAGE,
            CAPABILITY_OPERATION_WRITE,
            2,
            sha256(&namespace),
            sha256(b"constraints"),
            12,
            100,
            b"user-grant".to_vec(),
        );
        let grant_bytes = sdk_wire_bytes(&SdkWireRecord::RuntimeCapabilityGrant(grant.clone()));
        let result = decode_host_result(
            &host
                .grant_bytes(&grant_bytes, 12)
                .expect("host accepts grant wire bytes"),
        );
        assert_eq!(result.status, BROWSER_HOST_STATUS_OK);

        let capability_id = sha256(b"object-store");
        let binding = runtime_storage_binding(
            &grant,
            namespace.clone(),
            sha256(b"provider"),
            capability_id,
            RUNTIME_STORAGE_BACKING_MEMORY,
        );
        let binding_bytes = sdk_wire_bytes(&SdkWireRecord::RuntimeStorageBinding(binding));
        let result = decode_host_result(
            &host
                .bind_storage_bytes(&binding_bytes, 12)
                .expect("host accepts storage binding wire bytes"),
        );
        assert_eq!(result.status, BROWSER_HOST_STATUS_OK);

        let session = host.session_from_grant(
            &grant,
            capability_id,
            runtime_id,
            sha256(b"admission"),
            sha256(b"browser-local-route"),
            100,
        );
        let session_bytes =
            sdk_wire_bytes(&SdkWireRecord::RuntimeCapabilitySession(session.clone()));
        let result = decode_host_result(
            &host
                .open_session_bytes(&session_bytes, 13)
                .expect("host accepts session wire bytes"),
        );
        assert_eq!(result.status, BROWSER_HOST_STATUS_OK);

        let value = b"value".to_vec();
        let request = CapabilityRequest::new(
            CAPABILITY_KIND_STORAGE,
            CAPABILITY_OPERATION_WRITE,
            2,
            first_run.verified.app_id,
            first_run.verified.release_id,
            sha256(b"key"),
            sha256(&value),
            namespace,
            value,
            b"nonce".to_vec(),
        );
        let request_bytes = sdk_wire_bytes(&SdkWireRecord::CapabilityRequest(request));
        let result = decode_host_result(
            &host
                .invoke_storage_request_bytes(
                    &request_bytes,
                    StorageInvocationContext {
                        session_id: session.session_id,
                        capability_id,
                        source_node_id: first_run.verified.app_id,
                        target_node_id: runtime_id,
                        sequence: 0,
                        timestamp_unix_ms: 14,
                    },
                    b"memory-storage",
                )
                .expect("host invokes storage from wire request"),
        );
        assert_eq!(result.status, BROWSER_HOST_STATUS_OK);
        assert_eq!(result.proof_hashes.len(), 1);

        let response =
            from_bytes::<SdkWireRecord, edgerun_wire::WireError>(&result.output).unwrap();
        let SdkWireRecord::CapabilityResponse(response) = response else {
            panic!("expected capability response");
        };
        assert_eq!(response.status, CAPABILITY_STATUS_OK);
        assert_eq!(
            host.runtime().events().last().unwrap().event.event_kind,
            RUNTIME_EVENT_CAPABILITY_EXECUTED
        );
        assert!(verify_runtime_event_chain(host.runtime().events()));
    }

    #[test]
    fn browser_first_run_projection_bundle_stays_inside_wire_boundary() {
        let runtime_key = Ed25519SigningKey::from_bytes(&[2; 32]);
        let runtime_id = runtime_key.verifying_key().to_bytes();
        let package = smoke_package();
        let input_bytes = first_run_input_wire_bytes(BrowserAppFirstRunInput {
            profile_id: [1; 32],
            runtime_id,
            previous_event_sha256: [0; 32],
            first_event_seq: 0,
            event_time: 10,
            retrieval_cost: 5,
            source_admission_hash: [3; 32],
            decision: APP_RUN_DECISION_VERIFY_AND_CACHE,
            user_signature: b"user",
        });
        let projection_bytes = browser_core_first_run_projection_wire_bytes(
            &package.app_manifest_bytes,
            &package.app_graph_bytes,
            &package.developer_signature,
            &input_bytes,
        )
        .expect("package projection");
        let record = from_bytes::<SdkWireRecord, edgerun_wire::WireError>(&projection_bytes)
            .expect("projection wire decodes");
        let SdkWireRecord::BrowserAppFirstRunProjection(projection) = record else {
            panic!("expected first run projection record");
        };
        assert_eq!(projection.abi_version, SDK_WIRE_ABI_VERSION);
        assert_eq!(projection.runtime_projection.app_id, package.app_id);
        assert!(projection.cache.is_some());
        assert_eq!(projection.events.len(), 4);

        let mut host = BrowserHost::memory(runtime_id);
        let result = decode_host_result(
            &host
                .record_first_run_projection_bytes(&projection_bytes, 11)
                .expect("host accepts first-run projection bundle"),
        );
        assert_eq!(result.status, BROWSER_HOST_STATUS_OK);
        assert!(verify_runtime_event_chain(host.runtime().events()));

        let _typed: BrowserAppFirstRunProjectionRecord = projection;
    }

    #[test]
    fn browser_package_retrieval_wire_record_verifies_package() {
        let runtime_key = Ed25519SigningKey::from_bytes(&[2; 32]);
        let package = smoke_package();

        let bytes = browser_core_package_retrieval_wire_bytes(
            &package.app_manifest_bytes,
            &package.app_graph_bytes,
            &package.developer_signature,
            b"apps/smoke",
            5,
            12,
            [3; 32],
            b"retrieval",
            b"proof",
        )
        .expect("retrieval record");
        let record = from_bytes::<SdkWireRecord, edgerun_wire::WireError>(&bytes).unwrap();
        let SdkWireRecord::BrowserPackageRetrieval(retrieval) = record else {
            panic!("expected retrieval record");
        };
        let _typed: BrowserPackageRetrievalRecord = retrieval.clone();
        assert_eq!(retrieval.package_key, b"apps/smoke".to_vec());
        assert_eq!(retrieval.source_admission_hash, [3; 32]);

        let bad = browser_core_package_retrieval_wire_bytes(
            &package.app_manifest_bytes,
            &package.app_graph_bytes,
            &edgerun_browser_authoring::artifact_signature_bytes(
                b"bad",
                &runtime_key,
                edgerun_browser_authoring::DEVELOPER_SIGNATURE_DOMAIN,
            ),
            b"apps/smoke",
            5,
            12,
            [3; 32],
            b"retrieval",
            b"proof",
        );
        assert!(bad.is_err());
    }

    #[test]
    fn browser_package_retrieval_evidence_enters_through_admission() {
        let package = smoke_package();
        let evidence = browser_core_admitted_package_retrieval_evidence(
            &package.app_manifest_bytes,
            &package.app_graph_bytes,
            &package.developer_signature,
            b"apps/smoke",
            5,
            12,
            &[],
        )
        .expect("retrieval evidence");
        let record =
            from_bytes::<SdkWireRecord, edgerun_wire::WireError>(&evidence.evidence_bytes)
                .unwrap();
        let SdkWireRecord::BrowserPackageRetrievalEvidence(record) = record else {
            panic!("expected retrieval evidence record");
        };
        let _typed: BrowserPackageRetrievalEvidenceRecord = record.clone();
        assert_eq!(record.package_key, b"apps/smoke".to_vec());
        assert_eq!(record.admission_hash, evidence.source_admission_hash);
        assert_eq!(
            u16::from_le_bytes([
                evidence.work_admission_bytes[0],
                evidence.work_admission_bytes[1],
            ]),
            5
        );
        assert!(evidence.work_admission_bytes.len() > 64);
        assert_ne!(record.work_request_hash, [0; 32]);
        assert_ne!(record.route_commitment, [0; 32]);
        let relay_node_id = Ed25519SigningKey::from_bytes(&[24; 32])
            .verifying_key()
            .to_bytes();
        assert_ne!(record.storage_node_id, relay_node_id);
        assert_eq!(record.retrieval_cost, 5);
        assert_eq!(record.admitted_budget, 5);
    }

    #[test]
    fn browser_package_retrieval_evidence_binds_policy_schedule_hash() {
        let package = smoke_package();
        let schedule_bytes = browser_core_package_retrieval_policy_schedule_wire_bytes(
            10,
            2,
            15,
            100,
            10,
            20,
        );
        let schedule_record =
            from_bytes::<SdkWireRecord, edgerun_wire::WireError>(&schedule_bytes).unwrap();
        let SdkWireRecord::BrowserPackageRetrievalPolicySchedule(_schedule) = schedule_record else {
            panic!("expected retrieval policy schedule");
        };

        let evidence = browser_core_admitted_package_retrieval_evidence(
            &package.app_manifest_bytes,
            &package.app_graph_bytes,
            &package.developer_signature,
            b"apps/smoke",
            40,
            12,
            &schedule_bytes,
        )
        .expect("retrieval evidence");
        let record =
            from_bytes::<SdkWireRecord, edgerun_wire::WireError>(&evidence.evidence_bytes)
                .unwrap();
        let SdkWireRecord::BrowserPackageRetrievalEvidence(record) = record else {
            panic!("expected retrieval evidence record");
        };
        assert_eq!(record.policy_hash, blake3_hash(&schedule_bytes));
    }

    #[test]
    fn browser_package_retrieval_cost_comes_from_policy_schedule() {
        let schedule_bytes = browser_core_package_retrieval_policy_schedule_wire_bytes(
            10,
            2,
            15,
            100,
            10,
            20,
        );
        let record = from_bytes::<SdkWireRecord, edgerun_wire::WireError>(&schedule_bytes)
            .expect("schedule decodes");
        let SdkWireRecord::BrowserPackageRetrievalPolicySchedule(schedule) = record else {
            panic!("expected retrieval policy schedule");
        };
        let _typed: BrowserPackageRetrievalPolicyScheduleRecord = schedule.clone();
        assert_eq!(schedule.base_cost, 10);
        assert_ne!(schedule.policy_hash, [0; 32]);

        let cost = browser_core_package_retrieval_cost_from_policy_schedule(
            &schedule_bytes,
            5,
            7,
            3,
            12,
        )
        .expect("cost from schedule");
        assert_eq!(cost, 40);

        let capped = browser_core_package_retrieval_cost_from_policy_schedule(
            &schedule_bytes,
            50,
            50,
            50,
            12,
        )
        .expect("capped cost from schedule");
        assert_eq!(capped, 100);

        let expired = browser_core_package_retrieval_cost_from_policy_schedule(
            &schedule_bytes,
            5,
            7,
            3,
            21,
        );
        assert!(expired.is_err());
    }

    #[test]
    fn browser_first_run_input_builder_emits_wire_record_for_user_decision() {
        let input_bytes = browser_core_first_run_input_wire_bytes(
            [1; 32],
            [2; 32],
            [0; 32],
            7,
            10,
            5,
            [3; 32],
            APP_RUN_DECISION_RUN_ONCE,
            b"user-signature",
        );
        let record =
            from_bytes::<SdkWireRecord, edgerun_wire::WireError>(&input_bytes).unwrap();
        let SdkWireRecord::BrowserAppFirstRunInput(input) = record else {
            panic!("expected first-run input record");
        };
        assert_eq!(input.abi_version, SDK_WIRE_ABI_VERSION);
        assert_eq!(input.profile_id, [1; 32]);
        assert_eq!(input.runtime_id, [2; 32]);
        assert_eq!(input.first_event_seq, 7);
        assert_eq!(input.decision, APP_RUN_DECISION_RUN_ONCE);
        assert_eq!(input.user_signature, b"user-signature");
    }

    #[test]
    fn raw_wasm_host_abi_uses_wire_bytes_and_owned_output_buffers() {
        let runtime_key = Ed25519SigningKey::from_bytes(&[2; 32]);
        let runtime_id = runtime_key.verifying_key().to_bytes();
        let package = smoke_package();
        let first_run = first_run_projection_for_package(
            &package.app_manifest_bytes,
            &package.app_graph_bytes,
            &package.developer_signature,
            BrowserAppFirstRunInput {
                profile_id: [1; 32],
                runtime_id,
                previous_event_sha256: [0; 32],
                first_event_seq: 0,
                event_time: 10,
                retrieval_cost: 5,
                source_admission_hash: [3; 32],
                decision: APP_RUN_DECISION_VERIFY_AND_CACHE,
                user_signature: b"user",
            },
        )
        .expect("package verifies");

        let handle = unsafe { edgerun_browser_host_new(runtime_id.as_ptr(), runtime_id.len()) };
        assert!(!handle.is_null());

        let runtime_projection_bytes = sdk_wire_bytes(&SdkWireRecord::RuntimeAppInstall(
            first_run.verified.runtime_projection.clone(),
        ));
        let decision_bytes = sdk_wire_bytes(&SdkWireRecord::AppRunPromptDecision(
            first_run.decision.clone(),
        ));
        let cache_bytes = first_run
            .cache
            .clone()
            .map(|cache| sdk_wire_bytes(&SdkWireRecord::PackageCache(cache)))
            .expect("verify and cache emits cache record");

        let mut out_ptr = core::ptr::null_mut();
        let mut out_len = 0_usize;
        let status = unsafe {
            edgerun_browser_host_record_first_run(
                handle,
                runtime_projection_bytes.as_ptr(),
                runtime_projection_bytes.len(),
                decision_bytes.as_ptr(),
                decision_bytes.len(),
                cache_bytes.as_ptr(),
                cache_bytes.len(),
                1,
                11,
                &mut out_ptr,
                &mut out_len,
            )
        };
        assert_eq!(status, 0);
        assert_eq!(
            decode_host_result(&unsafe { take_ffi_bytes(out_ptr, out_len) }).status,
            BROWSER_HOST_STATUS_OK
        );

        let namespace = b"private".to_vec();
        let grant = runtime_capability_grant(
            [1; 32],
            [9; 32],
            first_run.verified.app_id,
            first_run.verified.release_id,
            CAPABILITY_KIND_STORAGE,
            CAPABILITY_OPERATION_WRITE,
            2,
            sha256(&namespace),
            sha256(b"constraints"),
            12,
            100,
            b"user-grant".to_vec(),
        );
        let grant_bytes = sdk_wire_bytes(&SdkWireRecord::RuntimeCapabilityGrant(grant.clone()));
        let status = unsafe {
            edgerun_browser_host_grant(
                handle,
                grant_bytes.as_ptr(),
                grant_bytes.len(),
                12,
                &mut out_ptr,
                &mut out_len,
            )
        };
        assert_eq!(status, 0);
        assert_eq!(
            decode_host_result(&unsafe { take_ffi_bytes(out_ptr, out_len) }).status,
            BROWSER_HOST_STATUS_OK
        );

        let capability_id = sha256(b"object-store");
        let binding = runtime_storage_binding(
            &grant,
            namespace.clone(),
            sha256(b"provider"),
            capability_id,
            RUNTIME_STORAGE_BACKING_MEMORY,
        );
        let binding_bytes = sdk_wire_bytes(&SdkWireRecord::RuntimeStorageBinding(binding));
        let status = unsafe {
            edgerun_browser_host_bind_storage(
                handle,
                binding_bytes.as_ptr(),
                binding_bytes.len(),
                12,
                &mut out_ptr,
                &mut out_len,
            )
        };
        assert_eq!(status, 0);
        assert_eq!(
            decode_host_result(&unsafe { take_ffi_bytes(out_ptr, out_len) }).status,
            BROWSER_HOST_STATUS_OK
        );

        let session = runtime_capability_session(
            &grant,
            capability_id,
            runtime_id,
            sha256(b"admission"),
            sha256(b"browser-local-route"),
            100,
        );
        let session_bytes =
            sdk_wire_bytes(&SdkWireRecord::RuntimeCapabilitySession(session.clone()));
        let status = unsafe {
            edgerun_browser_host_open_session(
                handle,
                session_bytes.as_ptr(),
                session_bytes.len(),
                13,
                &mut out_ptr,
                &mut out_len,
            )
        };
        assert_eq!(status, 0);
        assert_eq!(
            decode_host_result(&unsafe { take_ffi_bytes(out_ptr, out_len) }).status,
            BROWSER_HOST_STATUS_OK
        );

        let value = b"value".to_vec();
        let request = CapabilityRequest::new(
            CAPABILITY_KIND_STORAGE,
            CAPABILITY_OPERATION_WRITE,
            2,
            first_run.verified.app_id,
            first_run.verified.release_id,
            sha256(b"key"),
            sha256(&value),
            namespace,
            value,
            b"nonce".to_vec(),
        );
        let request_bytes = sdk_wire_bytes(&SdkWireRecord::CapabilityRequest(request));
        let context = BrowserHostStorageInvocationRecord {
            abi_version: SDK_WIRE_ABI_VERSION,
            session_id: session.session_id,
            capability_id,
            source_node_id: first_run.verified.app_id,
            target_node_id: runtime_id,
            sequence: 0,
            timestamp_unix_ms: 14,
            provider: b"memory-storage".to_vec(),
        };
        let context_bytes = sdk_wire_bytes(&SdkWireRecord::BrowserHostStorageInvocation(context));
        let status = unsafe {
            edgerun_browser_host_invoke_storage(
                handle,
                request_bytes.as_ptr(),
                request_bytes.len(),
                context_bytes.as_ptr(),
                context_bytes.len(),
                &mut out_ptr,
                &mut out_len,
            )
        };
        assert_eq!(status, 0);
        let result = decode_host_result(&unsafe { take_ffi_bytes(out_ptr, out_len) });
        assert_eq!(result.status, BROWSER_HOST_STATUS_OK);
        assert_eq!(result.proof_hashes.len(), 1);
        let response =
            from_bytes::<SdkWireRecord, edgerun_wire::WireError>(&result.output).unwrap();
        let SdkWireRecord::CapabilityResponse(response) = response else {
            panic!("expected capability response");
        };
        assert_eq!(response.status, CAPABILITY_STATUS_OK);

        unsafe {
            edgerun_browser_host_drop(handle);
        }
    }

    fn decode_host_result(bytes: &[u8]) -> BrowserHostResultRecord {
        let record = from_bytes::<SdkWireRecord, edgerun_wire::WireError>(bytes).unwrap();
        let SdkWireRecord::BrowserHostResult(result) = record else {
            panic!("expected browser host result");
        };
        result
    }

    unsafe fn take_ffi_bytes(ptr: *mut u8, len: usize) -> Vec<u8> {
        assert!(!ptr.is_null());
        let bytes = unsafe { core::slice::from_raw_parts(ptr, len) }.to_vec();
        unsafe {
            edgerun_browser_host_free(ptr, len);
        }
        bytes
    }
}
