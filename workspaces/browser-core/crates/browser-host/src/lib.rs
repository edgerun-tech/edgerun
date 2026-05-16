#![no_std]

extern crate alloc;

use alloc::boxed::Box;
use alloc::vec::Vec;
use core::{ptr, slice};

use edgerun_browser_runtime::{
    runtime_capability_session, sha256, MemoryRuntimeStorage, RuntimeError, RuntimeKernel,
};
use edgerun_wire::{
    from_bytes, sdk_wire_bytes, AppRunPromptDecisionRecord, BrowserAppFirstRunProjectionRecord,
    BrowserHostResultRecord, PackageCacheRecord, RuntimeAppInstall, RuntimeCapabilityGrant,
    RuntimeCapabilitySession, RuntimeStorageBinding, SdkWireRecord, WireError,
    SDK_WIRE_ABI_VERSION,
};
use edgerun_work::capability_packet::{
    capability_envelope, capability_envelope_hash, capability_invocation_id,
    CAPABILITY_CONTENT_OBJECT, CAPABILITY_OPERATION_OBJECT_GET, CAPABILITY_OPERATION_OBJECT_PUT,
    CAPABILITY_PACKET_INVOKE,
};
use edgerun_work::codec::blake3_hash;
use edgerun_work::protocol::Hash;

pub const BROWSER_HOST_STATUS_OK: u16 = 0;
pub const BROWSER_HOST_STATUS_INVALID_INPUT: u16 = 1;
pub const BROWSER_HOST_STATUS_RUNTIME_ERROR: u16 = 2;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BrowserHostError {
    InvalidWireRecord,
    UnexpectedRecord,
    Runtime(RuntimeError),
}

impl From<RuntimeError> for BrowserHostError {
    fn from(value: RuntimeError) -> Self {
        Self::Runtime(value)
    }
}

pub struct BrowserHost {
    runtime: RuntimeKernel<MemoryRuntimeStorage>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StorageInvocationContext {
    pub session_id: Hash,
    pub capability_id: Hash,
    pub source_node_id: Hash,
    pub target_node_id: Hash,
    pub sequence: u64,
    pub timestamp_unix_ms: u64,
}

impl BrowserHost {
    pub fn memory(runtime_id: Hash) -> Self {
        Self {
            runtime: RuntimeKernel::new(MemoryRuntimeStorage::default(), runtime_id),
        }
    }

    pub fn runtime(&self) -> &RuntimeKernel<MemoryRuntimeStorage> {
        &self.runtime
    }

    pub fn runtime_mut(&mut self) -> &mut RuntimeKernel<MemoryRuntimeStorage> {
        &mut self.runtime
    }

    pub fn record_first_run_bytes(
        &mut self,
        runtime_projection_bytes: &[u8],
        decision_bytes: &[u8],
        cache_bytes: Option<&[u8]>,
        time: u64,
    ) -> Result<Vec<u8>, BrowserHostError> {
        let runtime_projection = decode_runtime_app_install(runtime_projection_bytes)?;
        let decision = decode_app_run_prompt_decision(decision_bytes)?;
        let cache = match cache_bytes {
            Some(bytes) => Some(decode_package_cache(bytes)?),
            None => None,
        };
        self.runtime
            .record_app_first_run_projection(runtime_projection, decision, cache, time)?;
        self.last_event_result(Vec::new())
    }

    pub fn record_first_run_projection_bytes(
        &mut self,
        projection_bytes: &[u8],
        time: u64,
    ) -> Result<Vec<u8>, BrowserHostError> {
        let projection = decode_browser_first_run_projection(projection_bytes)?;
        self.runtime.record_app_first_run_projection(
            projection.runtime_projection,
            projection.decision,
            projection.cache,
            time,
        )?;
        self.last_event_result(Vec::new())
    }

    pub fn grant_bytes(
        &mut self,
        grant_bytes: &[u8],
        time: u64,
    ) -> Result<Vec<u8>, BrowserHostError> {
        let grant = decode_runtime_capability_grant(grant_bytes)?;
        self.runtime.grant_runtime_capability(grant, time)?;
        self.last_event_result(Vec::new())
    }

    pub fn bind_storage_bytes(
        &mut self,
        binding_bytes: &[u8],
        time: u64,
    ) -> Result<Vec<u8>, BrowserHostError> {
        let binding = decode_runtime_storage_binding(binding_bytes)?;
        self.runtime.bind_storage_provider(binding, time)?;
        self.last_event_result(Vec::new())
    }

    pub fn open_session_bytes(
        &mut self,
        session_bytes: &[u8],
        time: u64,
    ) -> Result<Vec<u8>, BrowserHostError> {
        let session = decode_runtime_capability_session(session_bytes)?;
        self.runtime.open_capability_session(session, time)?;
        self.last_event_result(Vec::new())
    }

    pub fn invoke_storage_request_bytes(
        &mut self,
        request_bytes: &[u8],
        context: StorageInvocationContext,
        provider: &[u8],
    ) -> Result<Vec<u8>, BrowserHostError> {
        let record = decode_sdk_record(request_bytes)?;
        let SdkWireRecord::CapabilityRequest(request) = record else {
            return Err(BrowserHostError::UnexpectedRecord);
        };
        let operation = match request.operation {
            edgerun_wire::CAPABILITY_OPERATION_READ => CAPABILITY_OPERATION_OBJECT_GET,
            edgerun_wire::CAPABILITY_OPERATION_WRITE => CAPABILITY_OPERATION_OBJECT_PUT,
            _ => return Err(BrowserHostError::UnexpectedRecord),
        };
        let payload = sdk_wire_bytes(&SdkWireRecord::CapabilityRequest(request));
        let envelope = capability_envelope(
            context.session_id,
            capability_invocation_id(
                context.session_id,
                operation,
                context.sequence,
                blake3_hash(&payload),
            ),
            context.capability_id,
            context.source_node_id,
            context.target_node_id,
            CAPABILITY_PACKET_INVOKE,
            operation,
            CAPABILITY_CONTENT_OBJECT,
            context.sequence,
            context.timestamp_unix_ms,
            payload,
        );
        let envelope_hash = capability_envelope_hash(&envelope);
        let output =
            self.runtime
                .invoke_storage_envelope(&envelope, provider, context.timestamp_unix_ms)?;
        self.last_event_result_with_output(output, alloc::vec![envelope_hash])
    }

    pub fn invoke_storage_request_wire_bytes(
        &mut self,
        request_bytes: &[u8],
        context_bytes: &[u8],
    ) -> Result<Vec<u8>, BrowserHostError> {
        let record = decode_sdk_record(context_bytes)?;
        let SdkWireRecord::BrowserHostStorageInvocation(context) = record else {
            return Err(BrowserHostError::UnexpectedRecord);
        };
        if context.abi_version != SDK_WIRE_ABI_VERSION {
            return Err(BrowserHostError::UnexpectedRecord);
        }
        self.invoke_storage_request_bytes(
            request_bytes,
            StorageInvocationContext {
                session_id: context.session_id,
                capability_id: context.capability_id,
                source_node_id: context.source_node_id,
                target_node_id: context.target_node_id,
                sequence: context.sequence,
                timestamp_unix_ms: context.timestamp_unix_ms,
            },
            &context.provider,
        )
    }

    pub fn session_from_grant(
        &self,
        grant: &RuntimeCapabilityGrant,
        capability_id: Hash,
        provider_node_id: Hash,
        admission_hash: Hash,
        route_commitment: Hash,
        valid_until: u64,
    ) -> RuntimeCapabilitySession {
        runtime_capability_session(
            grant,
            capability_id,
            provider_node_id,
            admission_hash,
            route_commitment,
            valid_until,
        )
    }

    fn last_event_result(&self, proof_hashes: Vec<Hash>) -> Result<Vec<u8>, BrowserHostError> {
        let output = self
            .runtime
            .events()
            .last()
            .map(|entry| sdk_wire_bytes(&SdkWireRecord::RuntimeEvent(entry.event.clone())))
            .unwrap_or_default();
        Ok(host_result(BROWSER_HOST_STATUS_OK, output, proof_hashes))
    }

    fn last_event_result_with_output(
        &self,
        output: Vec<u8>,
        proof_hashes: Vec<Hash>,
    ) -> Result<Vec<u8>, BrowserHostError> {
        Ok(host_result(BROWSER_HOST_STATUS_OK, output, proof_hashes))
    }
}

pub fn host_result(status: u16, output: Vec<u8>, proof_hashes: Vec<Hash>) -> Vec<u8> {
    sdk_wire_bytes(&SdkWireRecord::BrowserHostResult(BrowserHostResultRecord {
        abi_version: SDK_WIRE_ABI_VERSION,
        status,
        output,
        proof_hashes,
    }))
}

pub fn host_error_result(error: BrowserHostError) -> Vec<u8> {
    let status = match error {
        BrowserHostError::InvalidWireRecord | BrowserHostError::UnexpectedRecord => {
            BROWSER_HOST_STATUS_INVALID_INPUT
        }
        BrowserHostError::Runtime(_) => BROWSER_HOST_STATUS_RUNTIME_ERROR,
    };
    host_result(
        status,
        Vec::new(),
        alloc::vec![sha256(b"browser-host-error")],
    )
}

fn decode_sdk_record(bytes: &[u8]) -> Result<SdkWireRecord, BrowserHostError> {
    from_bytes::<SdkWireRecord, WireError>(bytes).map_err(|_| BrowserHostError::InvalidWireRecord)
}

fn decode_runtime_app_install(bytes: &[u8]) -> Result<RuntimeAppInstall, BrowserHostError> {
    match decode_sdk_record(bytes)? {
        SdkWireRecord::RuntimeAppInstall(record) => Ok(record),
        _ => Err(BrowserHostError::UnexpectedRecord),
    }
}

fn decode_app_run_prompt_decision(
    bytes: &[u8],
) -> Result<AppRunPromptDecisionRecord, BrowserHostError> {
    match decode_sdk_record(bytes)? {
        SdkWireRecord::AppRunPromptDecision(record) => Ok(record),
        _ => Err(BrowserHostError::UnexpectedRecord),
    }
}

fn decode_package_cache(bytes: &[u8]) -> Result<PackageCacheRecord, BrowserHostError> {
    match decode_sdk_record(bytes)? {
        SdkWireRecord::PackageCache(record) => Ok(record),
        _ => Err(BrowserHostError::UnexpectedRecord),
    }
}

fn decode_browser_first_run_projection(
    bytes: &[u8],
) -> Result<BrowserAppFirstRunProjectionRecord, BrowserHostError> {
    match decode_sdk_record(bytes)? {
        SdkWireRecord::BrowserAppFirstRunProjection(record) => {
            if record.abi_version == SDK_WIRE_ABI_VERSION {
                Ok(record)
            } else {
                Err(BrowserHostError::UnexpectedRecord)
            }
        }
        _ => Err(BrowserHostError::UnexpectedRecord),
    }
}

fn decode_runtime_capability_grant(
    bytes: &[u8],
) -> Result<RuntimeCapabilityGrant, BrowserHostError> {
    match decode_sdk_record(bytes)? {
        SdkWireRecord::RuntimeCapabilityGrant(record) => Ok(record),
        _ => Err(BrowserHostError::UnexpectedRecord),
    }
}

fn decode_runtime_storage_binding(bytes: &[u8]) -> Result<RuntimeStorageBinding, BrowserHostError> {
    match decode_sdk_record(bytes)? {
        SdkWireRecord::RuntimeStorageBinding(record) => Ok(record),
        _ => Err(BrowserHostError::UnexpectedRecord),
    }
}

fn decode_runtime_capability_session(
    bytes: &[u8],
) -> Result<RuntimeCapabilitySession, BrowserHostError> {
    match decode_sdk_record(bytes)? {
        SdkWireRecord::RuntimeCapabilitySession(record) => Ok(record),
        _ => Err(BrowserHostError::UnexpectedRecord),
    }
}

#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BrowserHostFfiStatus {
    Ok = 0,
    NullPointer = 1,
    InvalidLength = 2,
}

#[unsafe(no_mangle)]
pub extern "C" fn edgerun_browser_host_abi_version() -> u32 {
    SDK_WIRE_ABI_VERSION as u32
}

#[unsafe(no_mangle)]
pub extern "C" fn edgerun_browser_host_status_ok() -> u32 {
    BROWSER_HOST_STATUS_OK as u32
}

#[unsafe(no_mangle)]
pub extern "C" fn edgerun_browser_host_alloc(len: usize) -> *mut u8 {
    if len == 0 {
        return ptr::null_mut();
    }
    let mut bytes = alloc::vec![0_u8; len].into_boxed_slice();
    let ptr = bytes.as_mut_ptr();
    let _ = Box::into_raw(bytes);
    ptr
}

/// # Safety
///
/// `ptr` must have been returned by `edgerun_browser_host_alloc` or by one of
/// the host functions that writes an output pointer and length. `len` must be
/// the exact length returned for that buffer.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn edgerun_browser_host_free(ptr: *mut u8, len: usize) {
    if ptr.is_null() || len == 0 {
        return;
    }
    let raw = ptr::slice_from_raw_parts_mut(ptr, len);
    unsafe {
        drop(Box::from_raw(raw));
    }
}

/// # Safety
///
/// `runtime_id_ptr` must point to exactly 32 bytes for the duration of this
/// call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn edgerun_browser_host_new(
    runtime_id_ptr: *const u8,
    runtime_id_len: usize,
) -> *mut BrowserHost {
    let Some(runtime_id) = (unsafe { hash_from_ptr(runtime_id_ptr, runtime_id_len) }) else {
        return ptr::null_mut();
    };
    Box::into_raw(Box::new(BrowserHost::memory(runtime_id)))
}

/// # Safety
///
/// `handle` must have been returned by `edgerun_browser_host_new` and not freed
/// before.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn edgerun_browser_host_drop(handle: *mut BrowserHost) {
    if handle.is_null() {
        return;
    }
    unsafe {
        drop(Box::from_raw(handle));
    }
}

/// # Safety
///
/// All pointer/length pairs must be valid for the duration of this call.
/// `out_ptr` and `out_len` must be valid writable pointers.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn edgerun_browser_host_record_first_run(
    handle: *mut BrowserHost,
    runtime_projection_ptr: *const u8,
    runtime_projection_len: usize,
    decision_ptr: *const u8,
    decision_len: usize,
    cache_ptr: *const u8,
    cache_len: usize,
    has_cache: u32,
    time: u64,
    out_ptr: *mut *mut u8,
    out_len: *mut usize,
) -> u32 {
    let Some(host) = (unsafe { host_from_ptr(handle) }) else {
        return BrowserHostFfiStatus::NullPointer as u32;
    };
    let Some(runtime_projection) =
        (unsafe { slice_from_ptr(runtime_projection_ptr, runtime_projection_len) })
    else {
        return BrowserHostFfiStatus::NullPointer as u32;
    };
    let Some(decision) = (unsafe { slice_from_ptr(decision_ptr, decision_len) }) else {
        return BrowserHostFfiStatus::NullPointer as u32;
    };
    let cache = if has_cache == 0 {
        None
    } else {
        let Some(cache) = (unsafe { slice_from_ptr(cache_ptr, cache_len) }) else {
            return BrowserHostFfiStatus::NullPointer as u32;
        };
        Some(cache)
    };
    let output = host
        .record_first_run_bytes(runtime_projection, decision, cache, time)
        .unwrap_or_else(host_error_result);
    unsafe { write_output(output, out_ptr, out_len) }
}

/// # Safety
///
/// All pointer/length pairs must be valid for the duration of this call.
/// `out_ptr` and `out_len` must be valid writable pointers.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn edgerun_browser_host_record_first_run_projection(
    handle: *mut BrowserHost,
    projection_ptr: *const u8,
    projection_len: usize,
    time: u64,
    out_ptr: *mut *mut u8,
    out_len: *mut usize,
) -> u32 {
    let Some(host) = (unsafe { host_from_ptr(handle) }) else {
        return BrowserHostFfiStatus::NullPointer as u32;
    };
    let Some(projection) = (unsafe { slice_from_ptr(projection_ptr, projection_len) }) else {
        return BrowserHostFfiStatus::NullPointer as u32;
    };
    let output = host
        .record_first_run_projection_bytes(projection, time)
        .unwrap_or_else(host_error_result);
    unsafe { write_output(output, out_ptr, out_len) }
}

/// # Safety
///
/// All pointer/length pairs must be valid for the duration of this call.
/// `out_ptr` and `out_len` must be valid writable pointers.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn edgerun_browser_host_grant(
    handle: *mut BrowserHost,
    grant_ptr: *const u8,
    grant_len: usize,
    time: u64,
    out_ptr: *mut *mut u8,
    out_len: *mut usize,
) -> u32 {
    let Some(host) = (unsafe { host_from_ptr(handle) }) else {
        return BrowserHostFfiStatus::NullPointer as u32;
    };
    let Some(grant) = (unsafe { slice_from_ptr(grant_ptr, grant_len) }) else {
        return BrowserHostFfiStatus::NullPointer as u32;
    };
    let output = host
        .grant_bytes(grant, time)
        .unwrap_or_else(host_error_result);
    unsafe { write_output(output, out_ptr, out_len) }
}

/// # Safety
///
/// All pointer/length pairs must be valid for the duration of this call.
/// `out_ptr` and `out_len` must be valid writable pointers.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn edgerun_browser_host_bind_storage(
    handle: *mut BrowserHost,
    binding_ptr: *const u8,
    binding_len: usize,
    time: u64,
    out_ptr: *mut *mut u8,
    out_len: *mut usize,
) -> u32 {
    let Some(host) = (unsafe { host_from_ptr(handle) }) else {
        return BrowserHostFfiStatus::NullPointer as u32;
    };
    let Some(binding) = (unsafe { slice_from_ptr(binding_ptr, binding_len) }) else {
        return BrowserHostFfiStatus::NullPointer as u32;
    };
    let output = host
        .bind_storage_bytes(binding, time)
        .unwrap_or_else(host_error_result);
    unsafe { write_output(output, out_ptr, out_len) }
}

/// # Safety
///
/// All pointer/length pairs must be valid for the duration of this call.
/// `out_ptr` and `out_len` must be valid writable pointers.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn edgerun_browser_host_open_session(
    handle: *mut BrowserHost,
    session_ptr: *const u8,
    session_len: usize,
    time: u64,
    out_ptr: *mut *mut u8,
    out_len: *mut usize,
) -> u32 {
    let Some(host) = (unsafe { host_from_ptr(handle) }) else {
        return BrowserHostFfiStatus::NullPointer as u32;
    };
    let Some(session) = (unsafe { slice_from_ptr(session_ptr, session_len) }) else {
        return BrowserHostFfiStatus::NullPointer as u32;
    };
    let output = host
        .open_session_bytes(session, time)
        .unwrap_or_else(host_error_result);
    unsafe { write_output(output, out_ptr, out_len) }
}

/// # Safety
///
/// All pointer/length pairs must be valid for the duration of this call.
/// `out_ptr` and `out_len` must be valid writable pointers.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn edgerun_browser_host_invoke_storage(
    handle: *mut BrowserHost,
    request_ptr: *const u8,
    request_len: usize,
    context_ptr: *const u8,
    context_len: usize,
    out_ptr: *mut *mut u8,
    out_len: *mut usize,
) -> u32 {
    let Some(host) = (unsafe { host_from_ptr(handle) }) else {
        return BrowserHostFfiStatus::NullPointer as u32;
    };
    let Some(request) = (unsafe { slice_from_ptr(request_ptr, request_len) }) else {
        return BrowserHostFfiStatus::NullPointer as u32;
    };
    let Some(context) = (unsafe { slice_from_ptr(context_ptr, context_len) }) else {
        return BrowserHostFfiStatus::NullPointer as u32;
    };
    let output = host
        .invoke_storage_request_wire_bytes(request, context)
        .unwrap_or_else(host_error_result);
    unsafe { write_output(output, out_ptr, out_len) }
}

unsafe fn host_from_ptr<'a>(handle: *mut BrowserHost) -> Option<&'a mut BrowserHost> {
    if handle.is_null() {
        None
    } else {
        Some(unsafe { &mut *handle })
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

unsafe fn hash_from_ptr(ptr: *const u8, len: usize) -> Option<Hash> {
    let bytes = unsafe { slice_from_ptr(ptr, len) }?;
    if bytes.len() != 32 {
        return None;
    }
    let mut hash = [0_u8; 32];
    hash.copy_from_slice(bytes);
    Some(hash)
}

unsafe fn write_output(bytes: Vec<u8>, out_ptr: *mut *mut u8, out_len: *mut usize) -> u32 {
    if out_ptr.is_null() || out_len.is_null() {
        return BrowserHostFfiStatus::NullPointer as u32;
    }
    if bytes.is_empty() {
        unsafe {
            *out_ptr = ptr::null_mut();
            *out_len = 0;
        }
        return BrowserHostFfiStatus::Ok as u32;
    }
    let mut boxed = bytes.into_boxed_slice();
    let ptr = boxed.as_mut_ptr();
    let len = boxed.len();
    let _ = Box::into_raw(boxed);
    unsafe {
        *out_ptr = ptr;
        *out_len = len;
    }
    BrowserHostFfiStatus::Ok as u32
}
