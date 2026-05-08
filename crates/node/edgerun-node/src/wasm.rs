//! WASM ABI for the normal edgerun-node crate.
//!
//! The browser build is not a separate node implementation. It is this node
//! compiled for `wasm32` with browser-provided transport and storage adapters.

use alloc::collections::BTreeMap;
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::sync::atomic::{AtomicPtr, AtomicUsize, Ordering};
use std::sync::{Mutex, OnceLock};

use edgerun_crypto::ed25519_dalek::{Signature, Verifier, VerifyingKey};
use edgerun_crypto::sha::sha256;
use edgerun_protocols::wire::{
    from_bytes, sdk_wire_bytes, AppGraphRecord, AppStoreCatalogRecord, SdkWireRecord, WireError,
    SDK_WIRE_ABI_VERSION,
};

use crate::runtime::{RuntimeKernel, RuntimeMessageDelivery};
use crate::storage::MemoryRuntimeStorage;

struct BrowserNodeState {
    runtime: RuntimeKernel<MemoryRuntimeStorage>,
    installed: BTreeMap<[u8; 32], InstalledApp>,
    node_id: [u8; 32],
    stream_id: [u8; 32],
    started_at_ms: u64,
    peers: BTreeMap<String, ReachablePeer>,
    inbound_frames: Vec<Vec<u8>>,
    outbound_frames: Vec<Vec<u8>>,
}

#[derive(Clone)]
struct InstalledApp {
    app_id: [u8; 32],
    release_id: [u8; 32],
    manifest_sha256: [u8; 32],
    developer_id: [u8; 32],
}

#[derive(Clone)]
struct ReachablePeer {
    node_id: String,
    locator: String,
    transport: String,
    registered_at_ms: u64,
}

impl BrowserNodeState {
    fn new() -> Self {
        let node_id = [7; 32];
        Self {
            runtime: RuntimeKernel::new(MemoryRuntimeStorage::default(), node_id),
            installed: BTreeMap::new(),
            node_id,
            stream_id: [8; 32],
            started_at_ms: 0,
            peers: BTreeMap::new(),
            inbound_frames: Vec::new(),
            outbound_frames: Vec::new(),
        }
    }
}

static NODE: OnceLock<Mutex<BrowserNodeState>> = OnceLock::new();
static LAST_RESULT: OnceLock<Mutex<Vec<u8>>> = OnceLock::new();
static OUTBOUND_PTR: AtomicPtr<u8> = AtomicPtr::new(core::ptr::null_mut());
static OUTBOUND_LEN: AtomicUsize = AtomicUsize::new(0);

#[no_mangle]
pub extern "C" fn edgerun_node_alloc(len: usize) -> *mut u8 {
    let mut bytes = Vec::with_capacity(len);
    let ptr = bytes.as_mut_ptr();
    core::mem::forget(bytes);
    ptr
}

#[no_mangle]
pub unsafe extern "C" fn edgerun_node_dealloc(ptr: *mut u8, len: usize) {
    if !ptr.is_null() {
        drop(Vec::from_raw_parts(ptr, 0, len));
    }
}

#[no_mangle]
pub unsafe extern "C" fn edgerun_node_install_eapp(
    ptr: *const u8,
    len: usize,
    time_ms: u64,
) -> u64 {
    let bytes = core::slice::from_raw_parts(ptr, len);
    let owned = bytes.to_vec();
    let graph = match from_bytes::<SdkWireRecord, WireError>(&owned) {
        Ok(SdkWireRecord::AppGraph(graph)) => graph,
        Ok(_) => return write_result(400, br#"{"ok":false,"error":"not_app_graph"}"#.to_vec()),
        Err(_) => return write_result(400, br#"{"ok":false,"error":"invalid_rkyv"}"#.to_vec()),
    };

    if !app_graph_is_structurally_bound(&graph) {
        return write_result(400, br#"{"ok":false,"error":"unbound_app_graph"}"#.to_vec());
    }

    let install = graph.runtime_install.clone();
    let app_id = install.app_id;
    let release_id = install.release_id;
    let manifest_sha256 = install.manifest_sha256;
    let developer_id = install.developer_id;
    let node = NODE.get_or_init(|| Mutex::new(BrowserNodeState::new()));
    let mut node = node.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    if let Err(error) = node.runtime.install_app_graph(graph, time_ms) {
        return write_result(
            400,
            format!(
                r#"{{"ok":false,"error":"{}"}}"#,
                escape_json(&error.to_string())
            )
            .into_bytes(),
        );
    }
    node.installed.insert(
        app_id,
        InstalledApp {
            app_id,
            release_id,
            manifest_sha256,
            developer_id,
        },
    );

    write_result(
        200,
        format!(
            r#"{{"ok":true,"appId":"{}","releaseId":"{}","manifestSha256":"{}","developerId":"{}"}}"#,
            hex32(&app_id),
            hex32(&release_id),
            hex32(&manifest_sha256),
            hex32(&developer_id),
        )
        .into_bytes(),
    )
}

#[no_mangle]
pub unsafe extern "C" fn edgerun_node_decode_app_store_catalog(
    ptr: *const u8,
    len: usize,
) -> u64 {
    let bytes = core::slice::from_raw_parts(ptr, len);
    let owned = bytes.to_vec();
    let catalog = match from_bytes::<SdkWireRecord, WireError>(&owned) {
        Ok(SdkWireRecord::AppStoreCatalog(catalog)) => catalog,
        Ok(_) => return write_result(400, br#"{"ok":false,"error":"not_app_store_catalog"}"#.to_vec()),
        Err(_) => return write_result(400, br#"{"ok":false,"error":"invalid_rkyv"}"#.to_vec()),
    };
    if catalog.abi_version != SDK_WIRE_ABI_VERSION || catalog.flags & 1 != 1 {
        return write_result(400, br#"{"ok":false,"error":"invalid_catalog_abi"}"#.to_vec());
    }
    if !app_store_catalog_signature_is_valid(&catalog) {
        return write_result(
            400,
            br#"{"ok":false,"error":"invalid_catalog_signature"}"#.to_vec(),
        );
    }
    write_result(200, app_store_catalog_json(&catalog).into_bytes())
}

#[no_mangle]
pub unsafe extern "C" fn edgerun_node_decode_app_manifest(
    ptr: *const u8,
    len: usize,
) -> u64 {
    let bytes = core::slice::from_raw_parts(ptr, len);
    let owned = bytes.to_vec();
    let manifest = match from_bytes::<SdkWireRecord, WireError>(&owned) {
        Ok(SdkWireRecord::AppManifest(manifest)) => manifest,
        Ok(_) => return write_result(400, br#"{"ok":false,"error":"not_app_manifest"}"#.to_vec()),
        Err(_) => return write_result(400, br#"{"ok":false,"error":"invalid_rkyv"}"#.to_vec()),
    };
    if manifest.abi_version != SDK_WIRE_ABI_VERSION || manifest.flags & 1 != 1 {
        return write_result(400, br#"{"ok":false,"error":"invalid_manifest_abi"}"#.to_vec());
    }
    write_result(
        200,
        format!(
            r#"{{"ok":true,"appId":"{}","developerId":"{}","slug":"{}","name":"{}","version":"{}","summary":"{}","codeSha256":"{}"}}"#,
            hex32(&manifest.app_id),
            hex32(&manifest.developer_id),
            json_bytes(&manifest.app_slug),
            json_bytes(&manifest.name),
            json_bytes(&manifest.version),
            json_bytes(&manifest.summary),
            hex32(&manifest.code_sha256),
        )
        .into_bytes(),
    )
}

#[no_mangle]
pub unsafe extern "C" fn edgerun_node_protocol_request(
    ptr: *const u8,
    len: usize,
    time_ms: u64,
) -> u64 {
    let bytes = core::slice::from_raw_parts(ptr, len);
    let raw = match core::str::from_utf8(bytes) {
        Ok(raw) => raw,
        Err(_) => return write_result(400, br#"{"error":"invalid_request_utf8"}"#.to_vec()),
    };
    let (method, path, body) = match parse_protocol_request(raw) {
        Some(parts) => parts,
        None => return write_result(400, br#"{"error":"invalid_request"}"#.to_vec()),
    };
    let node = NODE.get_or_init(|| Mutex::new(BrowserNodeState::new()));
    let mut node = node.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    if node.started_at_ms == 0 {
        node.started_at_ms = time_ms;
    }
    let (status, body) = route_wasm_protocol(&mut node, method, path, body, time_ms);
    write_result(status, body.into_bytes())
}

#[no_mangle]
pub unsafe extern "C" fn edgerun_node_add_locator(ptr: *const u8, len: usize, time_ms: u64) -> u64 {
    let bytes = core::slice::from_raw_parts(ptr, len);
    let raw = match core::str::from_utf8(bytes) {
        Ok(raw) => raw,
        Err(_) => {
            return write_result(
                400,
                br#"{"ok":false,"error":"invalid_locator_utf8"}"#.to_vec(),
            )
        }
    };
    let mut lines = raw.lines();
    let node_id = lines.next().unwrap_or_default().trim();
    let transport = lines.next().unwrap_or_default().trim();
    let locator = lines.next().unwrap_or_default().trim();
    if node_id.is_empty() || transport.is_empty() || locator.is_empty() {
        return write_result(400, br#"{"ok":false,"error":"invalid_locator"}"#.to_vec());
    }

    let node = NODE.get_or_init(|| Mutex::new(BrowserNodeState::new()));
    node.lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .peers
        .insert(
            node_id.to_string(),
            ReachablePeer {
                node_id: node_id.to_string(),
                locator: locator.to_string(),
                transport: transport.to_string(),
                registered_at_ms: time_ms,
            },
        );
    write_result(200, br#"{"ok":true}"#.to_vec())
}

#[no_mangle]
pub unsafe extern "C" fn edgerun_node_ingest_transport_bytes(
    ptr: *const u8,
    len: usize,
    time_ms: u64,
) -> u64 {
    let bytes = core::slice::from_raw_parts(ptr, len).to_vec();
    let node = NODE.get_or_init(|| Mutex::new(BrowserNodeState::new()));
    let mut node = node.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    match ingest_edgerun_protocol_record(&mut node, bytes, time_ms) {
        Ok(status) => write_result(200, status.into_bytes()),
        Err(error) => write_result(
            400,
            format!(r#"{{"ok":false,"error":"{}"}}"#, escape_json(&error)).into_bytes(),
        ),
    }
}

#[no_mangle]
pub extern "C" fn edgerun_node_next_transport_frame() -> u64 {
    clear_outbound_frame();
    let node = NODE.get_or_init(|| Mutex::new(BrowserNodeState::new()));
    let Some(frame) = node
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .outbound_frames
        .pop()
    else {
        return 0;
    };
    let mut frame = frame;
    let len = frame.len();
    let ptr = frame.as_mut_ptr();
    core::mem::forget(frame);
    OUTBOUND_PTR.store(ptr, Ordering::Release);
    OUTBOUND_LEN.store(len, Ordering::Release);
    ((200u64) << 32) | len as u64
}

#[no_mangle]
pub extern "C" fn edgerun_node_outbound_frame_ptr() -> *const u8 {
    OUTBOUND_PTR.load(Ordering::Acquire)
}

#[no_mangle]
pub extern "C" fn edgerun_node_outbound_frame_len() -> usize {
    OUTBOUND_LEN.load(Ordering::Acquire)
}

#[no_mangle]
pub extern "C" fn edgerun_node_last_result_ptr() -> *const u8 {
    LAST_RESULT
        .get_or_init(|| Mutex::new(Vec::new()))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .as_ptr()
}

#[no_mangle]
pub extern "C" fn edgerun_node_last_result_len() -> usize {
    LAST_RESULT
        .get_or_init(|| Mutex::new(Vec::new()))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .len()
}

#[no_mangle]
pub extern "C" fn edgerun_node_installed_count() -> usize {
    NODE.get_or_init(|| Mutex::new(BrowserNodeState::new()))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .installed
        .len()
}

fn parse_protocol_request(raw: &str) -> Option<(&str, &str, &str)> {
    let (head, body) = raw.split_once("\n\n").unwrap_or((raw, ""));
    let mut lines = head.lines();
    let method = lines.next()?.trim();
    let path = lines.next()?.trim();
    if method.is_empty() || path.is_empty() {
        return None;
    }
    Some((method, path, body))
}

fn route_wasm_protocol(
    node: &mut BrowserNodeState,
    method: &str,
    path: &str,
    body: &str,
    time_ms: u64,
) -> (u32, String) {
    let clean_path = path.split('?').next().unwrap_or(path);
    if let Some((approval_id, decision)) = parse_approval_decision_path(clean_path) {
        return (
            200,
            format!(
                r#"{{"approvalId":"{}","decision":"{}"}}"#,
                escape_json(approval_id),
                decision
            ),
        );
    }

    match (method, clean_path) {
        ("GET", "/health") | ("GET", "/") => (200, health_json(node, time_ms)),
        ("GET", "/protocol/node/status") => (200, node_status_json(node)),
        ("GET", "/protocol/nodes") => (200, nodes_json(node)),
        ("GET", "/protocol/apps") => (200, apps_json(node)),
        ("GET", "/protocol/capabilities") => (200, capabilities_json(node)),
        ("GET", "/protocol/approvals") => (200, r#"{"approvals":[]}"#.to_string()),
        ("POST", "/protocol/tools/invoke") => invoke_protocol_tool(body),
        (_, path) if path.starts_with("/protocol/app/") && path.ends_with("/grants") => {
            (200, "[]".to_string())
        }
        _ => (404, r#"{"error":"not_found"}"#.to_string()),
    }
}

fn ingest_edgerun_protocol_record(
    node: &mut BrowserNodeState,
    bytes: Vec<u8>,
    time_ms: u64,
) -> Result<String, String> {
    let record = from_bytes::<SdkWireRecord, WireError>(&bytes)
        .map_err(|_| "invalid_rkyv_protocol_record".to_string())?;
    match record {
        SdkWireRecord::RuntimeAppMessage(message) => {
            let delivery = node
                .runtime
                .route_app_message(message, time_ms)
                .map_err(|error| error.to_string())?;
            match delivery {
                RuntimeMessageDelivery::Local(message) => {
                    node.inbound_frames
                        .push(sdk_wire_bytes(&SdkWireRecord::RuntimeAppMessage(message)));
                    Ok(r#"{"ok":true,"delivery":"local"}"#.to_string())
                }
                RuntimeMessageDelivery::Remote(routed) => {
                    node.outbound_frames.push(sdk_wire_bytes(
                        &SdkWireRecord::RuntimeRoutedAppMessage(routed),
                    ));
                    Ok(r#"{"ok":true,"delivery":"remote"}"#.to_string())
                }
            }
        }
        SdkWireRecord::RuntimeRoutedAppMessage(routed) => {
            if routed.to_runtime_id != node.node_id {
                node.outbound_frames
                    .push(sdk_wire_bytes(&SdkWireRecord::RuntimeRoutedAppMessage(
                        routed,
                    )));
                return Ok(r#"{"ok":true,"delivery":"forwarded"}"#.to_string());
            }
            let message = node
                .runtime
                .dispatch_app_message(routed.message, time_ms)
                .map_err(|error| error.to_string())?;
            node.inbound_frames
                .push(sdk_wire_bytes(&SdkWireRecord::RuntimeAppMessage(message)));
            Ok(r#"{"ok":true,"delivery":"local"}"#.to_string())
        }
        _ => Err("unsupported_transport_protocol_record".to_string()),
    }
}

fn health_json(node: &BrowserNodeState, time_ms: u64) -> String {
    let uptime_ms = time_ms.saturating_sub(node.started_at_ms);
    format!(
        r#"{{"status":"ok","uptime_ms":{},"node_id":"{}","stream_id":"{}","runtime":"wasm"}}"#,
        uptime_ms,
        hex32(&node.node_id),
        hex32(&node.stream_id),
    )
}

fn node_status_json(node: &BrowserNodeState) -> String {
    format!(
        r#"{{"nodeId":"{}","identity":"{}","health":"healthy","streamHead":{{"streamId":"{}","lastSeq":{},"lastEventId":""}},"runtimeVersion":"{}","protocolVersions":["rkyv","wasm"],"syncStatus":"synced","lastRefresh":"wasm","controlledNodeCount":{},"reachableNodeCount":{}}}"#,
        hex32(&node.node_id),
        hex32(&node.node_id),
        hex32(&node.stream_id),
        node.installed.len(),
        env!("CARGO_PKG_VERSION"),
        1 + node.peers.len(),
        node.peers.len(),
    )
}

fn nodes_json(node: &BrowserNodeState) -> String {
    let mut nodes = Vec::with_capacity(1 + node.peers.len());
    nodes.push(format!(
        r#"{{"nodeId":"{}","identity":"{}","target":"wasm","runtime":"wasm","relationship":"self","controlled":true,"reachable":true,"health":"healthy","installedAppCount":{}}}"#,
        hex32(&node.node_id),
        hex32(&node.node_id),
        node.installed.len(),
    ));
    for peer in node.peers.values() {
        nodes.push(format!(
            r#"{{"nodeId":"{}","identity":"{}","target":"{}","runtime":"native","relationship":"reachable","controlled":false,"reachable":true,"health":"reachable","transport":"{}","registeredAtMs":{}}}"#,
            escape_json(&peer.node_id),
            escape_json(&peer.node_id),
            escape_json(&peer.locator),
            escape_json(&peer.transport),
            peer.registered_at_ms,
        ));
    }
    format!(r#"{{"nodes":[{}]}}"#, nodes.join(","))
}

fn apps_json(node: &BrowserNodeState) -> String {
    let mut apps = Vec::with_capacity(node.installed.len());
    for app in node.installed.values() {
        apps.push(format!(
            r#"{{"version":1,"name":"{}","entry":"app.edapp","wasm_object":{{"object_id":{},"object_kind":9}},"metadata":{{"object_id":{},"object_kind":4}},"routes":{{}},"assets":{{}}}}"#,
            hex32(&app.app_id),
            byte_array_json(&app.release_id),
            byte_array_json(&app.manifest_sha256),
        ));
    }
    format!(r#"{{"apps":[{}]}}"#, apps.join(","))
}

fn capabilities_json(node: &BrowserNodeState) -> String {
    format!(
        r#"[{{"descriptor_version":1,"capability_id":{},"provider_node":{{"node_id":{}}},"provider_name":"Edgerun node","provider_instance_id":"edgerun-node"}},{{"descriptor_version":1,"capability_id":{},"provider_node":{{"node_id":{}}},"provider_name":"Xray viewport","provider_instance_id":"edgerun-node-xray"}}]"#,
        byte_array_json(&stable_capability_id(b"node_connection")),
        byte_array_json(&node.node_id),
        byte_array_json(&stable_capability_id(b"xray_viewport_control")),
        byte_array_json(&node.node_id),
    )
}

fn invoke_protocol_tool(body: &str) -> (u32, String) {
    let tool_id = extract_json_string(body, "toolId").unwrap_or_default();
    if tool_id.is_empty() {
        return (
            400,
            r#"{"status":"failed","error":"missing toolId"}"#.to_string(),
        );
    }

    match tool_id.as_str() {
        "xray.viewport.focus_node" | "xray.viewport.show_related" | "xray.viewport.set_camera" => (
            200,
            format!(
                r#"{{"status":"executed","result":{{"queued":true,"toolId":"{}","runtime":"edgerun-node"}}}}"#,
                escape_json(&tool_id)
            ),
        ),
        _ => (
            200,
            format!(
                r#"{{"status":"blocked","error":"unknown tool {}"}}"#,
                escape_json(&tool_id)
            ),
        ),
    }
}

fn parse_approval_decision_path(path: &str) -> Option<(&str, &str)> {
    let rest = path.strip_prefix("/protocol/approvals/")?;
    if let Some(id) = rest.strip_suffix("/approve") {
        return Some((id, "approved"));
    }
    if let Some(id) = rest.strip_suffix("/reject") {
        return Some((id, "rejected"));
    }
    None
}

fn write_result(status: u32, bytes: Vec<u8>) -> u64 {
    let len = bytes.len() as u64;
    *LAST_RESULT
        .get_or_init(|| Mutex::new(Vec::new()))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner()) = bytes;
    ((status as u64) << 32) | len
}

fn clear_outbound_frame() {
    let ptr = OUTBOUND_PTR.swap(core::ptr::null_mut(), Ordering::AcqRel);
    let len = OUTBOUND_LEN.swap(0, Ordering::AcqRel);
    if !ptr.is_null() {
        unsafe {
            drop(Vec::from_raw_parts(ptr, len, len));
        }
    }
}

fn app_graph_is_structurally_bound(graph: &AppGraphRecord) -> bool {
    graph.abi_version == SDK_WIRE_ABI_VERSION
        && graph.flags & 1 == 1
        && graph.runtime_install.abi_version == SDK_WIRE_ABI_VERSION
        && graph.runtime_install.flags & 1 == 1
        && graph.runtime_install.app_id == graph.app_id
        && graph.runtime_install.developer_id == graph.developer_public_key
        && graph.runtime_install.manifest_sha256 == graph.app_manifest_sha256
        && graph.artifacts.iter().any(|artifact| {
            artifact.path.as_slice() == b"app.edapp" && artifact.sha256 == graph.app_manifest_sha256
        })
}

fn app_store_catalog_json(catalog: &AppStoreCatalogRecord) -> String {
    let entries = catalog
        .entries
        .iter()
        .map(|entry| {
            let required = entry
                .required_capabilities
                .iter()
                .map(|capability| format!(r#""{}""#, json_bytes(capability)))
                .collect::<Vec<_>>()
                .join(",");
            let optional = entry
                .optional_capabilities
                .iter()
                .map(|capability| format!(r#""{}""#, json_bytes(capability)))
                .collect::<Vec<_>>()
                .join(",");
            let assets = entry
                .asset_refs
                .iter()
                .map(|asset| {
                    format!(
                        r#"{{"path":"{}","sha256":"{}","bytes":{}}}"#,
                        json_bytes(&asset.path),
                        hex32(&asset.sha256),
                        asset.bytes,
                    )
                })
                .collect::<Vec<_>>()
                .join(",");
            format!(
                r#"{{"appId":"{}","runtimeAppId":"{}","releaseId":"{}","developerId":"{}","slug":"{}","name":"{}","version":"{}","description":"{}","runtime":"browser-iframe","packageUrl":"{}","manifestUrl":"{}","launchUrl":"{}","eappSha256":"{}","manifestSha256":"{}","packageBytes":{},"developer":"{}","requiredCapabilityIds":[{}],"optionalCapabilityIds":[{}],"assets":[{}],"status":{}}}"#,
                json_bytes(&entry.app_slug),
                hex32(&entry.app_id),
                hex32(&entry.release_id),
                hex32(&entry.developer_id),
                json_bytes(&entry.app_slug),
                json_bytes(&entry.name),
                json_bytes(&entry.version),
                json_bytes(&entry.summary),
                json_bytes(&entry.package_ref),
                json_bytes(&entry.manifest_ref),
                json_bytes(&entry.launch_ref),
                hex32(&entry.package_sha256),
                hex32(&entry.manifest_sha256),
                entry.package_bytes,
                hex32(&entry.developer_id),
                required,
                optional,
                assets,
                entry.status,
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    format!(
        r#"{{"ok":true,"format":"edgerun-app-store-catalog-rkyv-v1","generatedAt":{},"sequence":{},"storeId":"{}","signatureVerified":true,"apps":[{}]}}"#,
        catalog.generated_at,
        catalog.sequence,
        hex32(&catalog.store_id),
        entries,
    )
}

const APP_STORE_CATALOG_DOMAIN: &[u8] = b"edgerun-sdk.ecat.v1.store-catalog";

fn app_store_catalog_signature_is_valid(catalog: &AppStoreCatalogRecord) -> bool {
    let Ok(signature) = Signature::try_from(catalog.signature.as_slice()) else {
        return false;
    };
    let Ok(verifying_key) = VerifyingKey::from_bytes(&catalog.store_id) else {
        return false;
    };
    let mut unsigned = catalog.clone();
    unsigned.signature.clear();
    let unsigned_bytes = sdk_wire_bytes(&SdkWireRecord::AppStoreCatalog(unsigned));
    let payload = signature_payload_for_domain(APP_STORE_CATALOG_DOMAIN, &sha256(&unsigned_bytes));
    verifying_key.verify(&payload, &signature).is_ok()
}

fn signature_payload_for_domain(domain: &[u8], artifact_hash: &[u8; 32]) -> Vec<u8> {
    let mut payload = Vec::with_capacity(domain.len() + artifact_hash.len());
    payload.extend_from_slice(domain);
    payload.extend_from_slice(artifact_hash);
    payload
}

fn hex32(bytes: &[u8; 32]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(64);
    for byte in bytes {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}

fn json_bytes(bytes: &[u8]) -> String {
    escape_json(&String::from_utf8_lossy(bytes))
}

fn byte_array_json(bytes: &[u8; 32]) -> String {
    let mut out = String::with_capacity(97);
    out.push('[');
    for (index, byte) in bytes.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push_str(&byte.to_string());
    }
    out.push(']');
    out
}

fn stable_capability_id(name: &[u8]) -> [u8; 32] {
    let mut out = [0u8; 32];
    for (index, byte) in name.iter().enumerate() {
        let slot = index % 32;
        out[slot] = out[slot].wrapping_mul(31).wrapping_add(*byte);
    }
    out
}

fn extract_json_string(raw: &str, key: &str) -> Option<String> {
    let needle = format!("\"{}\"", key);
    let start = raw.find(&needle)?;
    let after_key = &raw[start + needle.len()..];
    let colon = after_key.find(':')?;
    let after_colon = after_key[colon + 1..].trim_start();
    let after_quote = after_colon.strip_prefix('"')?;
    let end = after_quote.find('"')?;
    Some(after_quote[..end].to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use edgerun_crypto::{Ed25519SigningKey as SigningKey, Signer};

    #[test]
    fn app_store_catalog_signature_must_verify() {
        let key = SigningKey::from_bytes(&[9; 32]);
        let mut catalog = AppStoreCatalogRecord {
            abi_version: SDK_WIRE_ABI_VERSION,
            flags: 1,
            store_id: *key.verifying_key().as_bytes(),
            generated_at: 0,
            sequence: 1,
            previous_catalog_sha256: [0; 32],
            entries: Vec::new(),
            signature: Vec::new(),
        };
        let unsigned_bytes = sdk_wire_bytes(&SdkWireRecord::AppStoreCatalog(catalog.clone()));
        let signature = key.sign(&signature_payload_for_domain(
            APP_STORE_CATALOG_DOMAIN,
            &sha256(&unsigned_bytes),
        ));
        catalog.signature = signature.to_bytes().to_vec();

        assert!(app_store_catalog_signature_is_valid(&catalog));

        catalog.sequence = 2;
        assert!(!app_store_catalog_signature_is_valid(&catalog));
    }
}

fn escape_json(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
}
