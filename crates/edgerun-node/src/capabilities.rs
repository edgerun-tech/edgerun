//! Capability discovery and server for the edgerun node daemon.
//!
//! Discovers hardware devices, wraps them as `RemoteCapabilityProvider`s,
//! aggregates them into a multi-provider, and serves them over:
//! - Unix domain socket (local clients)
//! - Mesh network (remote nodes via edgerun-mesh-capability)
//!
//! Capability grants are managed through the event stream. Grants are recorded
//! as `EventType::CapabilityGranted` events and revoked as
//! `EventType::CapabilityRevoked` events. On startup, the event log is
//! replayed to rebuild the grant state.

use edgerun_core::protocol::ObjectKind;

use edgerun_capabilities::{CapabilityDescriptor, CapabilityProvider};
use edgerun_capability_policy::{
    PolicyContext, PolicyEngine, RevocationReason, SimplePolicyEngine,
};
use edgerun_core::protocol::EventType;
use edgerun_hardware_signing::MeshSigner;
use edgerun_remote_capability::{
    serve_one, FramedRemoteTransport, PolicyWrappedProvider, RemoteCapabilityProvider,
};
use edgerun_storage::NodeStore;
use prost::Message;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

const KIND_PAYLOAD: i32 = ObjectKind::Payload as i32; // 1

#[cfg(feature = "all-hardware")]
use edgerun_camera_biometrics::CameraBiometricPurpose;
#[cfg(feature = "hardware")]
use edgerun_evdev_input::EvdevInputBackend;
#[cfg(feature = "all-hardware")]
use edgerun_microphone::{AudioCaptureRequest, MicrophoneSampleFormat};
#[cfg(feature = "all-hardware")]
use edgerun_remote_capability::adapters::{
    BluetoothConnectionRemoteAdapter, BluetoothRemoteAdapter, CameraRemoteAdapter,
    InputRemoteAdapter, MicrophoneRemoteAdapter, SpeakerRemoteAdapter, WifiControlRemoteAdapter,
    WifiRemoteAdapter,
};
#[cfg(feature = "all-hardware")]
use edgerun_v4l2_camera::{discover_camera_devices, V4l2CameraBiometricReader};

// ===========================================================================
// Grant projection from event stream
// ===========================================================================

/// Replays capability grant and revocation events from the event stream
/// to rebuild a `SimplePolicyEngine` with all active grants.
///
/// This walks events from seq 1 through `head_seq`, processing:
/// - `EventType::CapabilityGranted` → imports the grant object into the engine
/// - `EventType::CapabilityRevoked` → revokes the grant in the engine
pub fn project_capability_grants(
    store: &edgerun_storage::NodeStore,
    stream_id: &[u8],
    head_seq: u64,
) -> SimplePolicyEngine {
    use edgerun_capability_policy::RevocationReason;

    let mut engine = SimplePolicyEngine::default();

    for seq in 1..=head_seq {
        let event = match store.get_event(stream_id, seq) {
            Ok(Some(e)) => e,
            _ => continue,
        };

        let event_type = EventType::try_from(event.event_type).unwrap_or(EventType::Unspecified);

        match event_type {
            EventType::CapabilityGranted => {
                // Resolve the grant object from the store
                if let Some(object_ref) = &event.payload_object {
                    if let Ok(Some(obj)) = store.get_object(object_ref) {
                        if let Ok(grant) =
                            edgerun_proto::edgerun::v0::capability::CapabilityGrant::decode(
                                &obj.content[..],
                            )
                        {
                            if let Err(e) = engine.import_grant(grant) {
                                edgerun_log::warn!(
                                    "failed to import capability grant for event seq {}: {}",
                                    event.seq,
                                    e
                                );
                            }
                        }
                    }
                }
            }
            EventType::CapabilityRevoked => {
                // Resolve the revocation object from the store
                if let Some(object_ref) = &event.payload_object {
                    if let Ok(Some(obj)) = store.get_object(object_ref) {
                        if let Ok(revocation) =
                            edgerun_proto::edgerun::v0::capability::CapabilityRevocation::decode(
                                &obj.content[..],
                            )
                        {
                            if let Err(e) =
                                engine.revoke(&revocation.grant_id, RevocationReason::Superseded)
                            {
                                edgerun_log::warn!(
                                    "failed to revoke capability for event seq {}: {}",
                                    event.seq,
                                    e
                                );
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }

    engine
}

/// Records a capability granted event into the event stream.
///
/// The grant is stored as an object in the store, then an event is appended
/// referencing it via `payload_object`.
pub fn record_capability_grant_event(
    store: &mut edgerun_storage::NodeStore,
    stream_id: &[u8],
    signer: &dyn edgerun_hardware_signing::MeshSigner,
    grant: &edgerun_proto::edgerun::v0::capability::CapabilityGrant,
) -> Result<u64, Box<dyn std::error::Error>> {
    // Store the grant as an object
    let grant_bytes = grant.encode_to_vec();
    let object_ref =
        store.put_object(&grant_bytes, KIND_PAYLOAD, &[signer.node_id().0.to_vec()])?;

    // Append signed event (using the internal function from command_dispatch)
    let seq = crate::command_dispatch::append_signed_event(
        store,
        stream_id,
        signer,
        EventType::CapabilityGranted,
        1,
        Some(object_ref),
        vec![],
        vec![],
        vec![],
    )
    .ok_or("failed to append capability grant event")?;

    Ok(seq)
}

/// Records a capability revoked event into the event stream.
pub fn record_capability_revocation_event(
    store: &mut edgerun_storage::NodeStore,
    stream_id: &[u8],
    signer: &dyn edgerun_hardware_signing::MeshSigner,
    revocation: &edgerun_proto::edgerun::v0::capability::CapabilityRevocation,
) -> Result<u64, Box<dyn std::error::Error>> {
    // Store the revocation as an object
    let rev_bytes = revocation.encode_to_vec();
    let object_ref = store.put_object(&rev_bytes, KIND_PAYLOAD, &[signer.node_id().0.to_vec()])?;

    let seq = crate::command_dispatch::append_signed_event(
        store,
        stream_id,
        signer,
        EventType::CapabilityRevoked,
        1,
        Some(object_ref),
        vec![],
        vec![],
        vec![],
    )
    .ok_or("failed to append capability revocation event")?;

    Ok(seq)
}

// ===========================================================================
// Multi-provider — aggregates multiple capability providers into one
// ===========================================================================

/// A capability provider that wraps multiple underlying providers.
///
/// Clients request a specific capability by matching the provider's descriptor.
/// The multi-provider routes session opens and invocations to the correct
/// underlying provider.
pub struct MultiCapabilityProvider {
    /// Registered providers, keyed by their capability ID.
    providers: HashMap<String, Box<dyn RemoteCapabilityProvider + Send>>,
}

impl MultiCapabilityProvider {
    pub fn new() -> Self {
        Self {
            providers: HashMap::new(),
        }
    }

    /// Registers a capability provider.
    pub fn register(
        &mut self,
        descriptor: &CapabilityDescriptor,
        provider: Box<dyn RemoteCapabilityProvider + Send>,
    ) {
        let key = format!(
            "{}:{}:{}",
            descriptor.provider_name,
            descriptor.provider_instance_id,
            descriptor.modalities.first().unwrap_or(&0)
        );
        self.providers.insert(key, provider);
    }

    /// Returns the number of registered providers.
    pub fn len(&self) -> usize {
        self.providers.len()
    }

    /// Returns true if no providers are registered.
    pub fn is_empty(&self) -> bool {
        self.providers.is_empty()
    }
}

impl RemoteCapabilityProvider for MultiCapabilityProvider {
    fn descriptor(&self) -> CapabilityDescriptor {
        let id = format!("{}-providers", self.providers.len());
        CapabilityDescriptor {
            descriptor_version: 1,
            capability_id: id.as_bytes().to_vec(),
            provider_identity: None,
            provider_node: None,
            role: 0,
            modalities: Vec::new(),
            event_kinds: Vec::new(),
            operations: Vec::new(),
            default_constraints: Vec::new(),
            provider_name: "edgerun-multi".into(),
            provider_instance_id: id,
            signature: None,
        }
    }

    fn open_session(
        &mut self,
        open: &edgerun_proto::edgerun::v0::capability_runtime::CapabilitySessionOpen,
    ) -> Result<
        edgerun_proto::edgerun::v0::capability_runtime::CapabilitySessionAccept,
        edgerun_capabilities::CapabilityError,
    > {
        let mut last_err = None;
        for (key, provider) in self.providers.iter_mut() {
            match provider.open_session(open) {
                Ok(accept) => return Ok(accept),
                Err(e) => last_err = Some((key.clone(), e)),
            }
        }
        Err(last_err.map(|(_, e)| e).unwrap_or_else(|| {
            edgerun_capabilities::CapabilityError::Unsupported("no capability providers registered")
        }))
    }

    fn invoke(
        &mut self,
        session_id: &[u8],
        invocation: &edgerun_capabilities::CapabilityInvocation,
        inline_parameters: Option<&[u8]>,
    ) -> Result<
        edgerun_remote_capability::RemoteInvocationResult,
        edgerun_capabilities::CapabilityError,
    > {
        for provider in self.providers.values_mut() {
            match provider.invoke(session_id, invocation, inline_parameters) {
                Ok(result) => return Ok(result),
                Err(edgerun_capabilities::CapabilityError::PermissionDenied(_)) => continue,
                Err(e) => return Err(e),
            }
        }
        Err(edgerun_capabilities::CapabilityError::PermissionDenied(
            "no provider authorized for this session",
        ))
    }

    fn next_event(
        &mut self,
        session_id: &[u8],
    ) -> Result<
        Option<edgerun_proto::edgerun::v0::capability_runtime::CapabilitySessionEvent>,
        edgerun_capabilities::CapabilityError,
    > {
        for provider in self.providers.values_mut() {
            match provider.next_event(session_id) {
                Ok(Some(event)) => return Ok(Some(event)),
                Ok(None) => continue,
                Err(edgerun_capabilities::CapabilityError::PermissionDenied(_)) => continue,
                Err(e) => return Err(e),
            }
        }
        Ok(None)
    }

    fn close_session(
        &mut self,
        close: &edgerun_proto::edgerun::v0::capability_runtime::CapabilitySessionClose,
    ) -> Result<(), edgerun_capabilities::CapabilityError> {
        for provider in self.providers.values_mut() {
            if let Err(e) = provider.close_session(close) {
                edgerun_log::debug!("provider close_session error: {}", e);
            }
        }
        Ok(())
    }

    fn handle_request(
        &mut self,
        request: &edgerun_capabilities::CapabilityRequest,
    ) -> Result<Option<edgerun_capabilities::CapabilityGrant>, edgerun_capabilities::CapabilityError>
    {
        // Try to find a provider that can handle this request
        for provider in self.providers.values_mut() {
            match provider.handle_request(request) {
                Ok(Some(grant)) => return Ok(Some(grant)),
                Ok(None) => continue,
                Err(_) => continue,
            }
        }
        Ok(None)
    }

    fn handle_grant(
        &mut self,
        grant: &edgerun_capabilities::CapabilityGrant,
    ) -> Result<(), edgerun_capabilities::CapabilityError> {
        // Forward grant to all providers (each policy engine tracks it)
        for provider in self.providers.values_mut() {
            if let Err(e) = provider.handle_grant(grant) {
                edgerun_log::debug!("provider handle_grant error: {}", e);
            }
        }
        Ok(())
    }

    fn handle_revocation(
        &mut self,
        revocation: &edgerun_capabilities::CapabilityRevocation,
    ) -> Result<(), edgerun_capabilities::CapabilityError> {
        // Forward revocation to all providers
        for provider in self.providers.values_mut() {
            if let Err(e) = provider.handle_revocation(revocation) {
                edgerun_log::debug!("provider handle_revocation error: {}", e);
            }
        }
        Ok(())
    }
}

// ===========================================================================
// Hardware discovery
// ===========================================================================

/// Discovers all available hardware capabilities and registers them with the
/// multi-provider, wrapped in policy enforcement.
///
/// The `policy` engine should already have grants replayed from the event stream
/// via `project_capability_grants()` before calling this function.
pub fn discover_and_register_capabilities(
    multi: &mut MultiCapabilityProvider,
    policy: SimplePolicyEngine,
) {
    let context = PolicyContext {
        is_local: true,
        ..PolicyContext::default()
    };

    #[cfg(feature = "hardware")]
    {
        // --- Input devices (evdev) ---
        match edgerun_evdev_input::discover_evdev_devices() {
            Ok(devices) => {
                for device_info in devices {
                    match EvdevInputBackend::open(device_info.clone()) {
                        Ok(backend) => {
                            let descriptor = backend.descriptor();
                            let adapter = InputRemoteAdapter::new(backend, 64);
                            let wrapped =
                                PolicyWrappedProvider::with_policy(adapter, policy.clone())
                                    .with_context(context.clone());
                            multi.register(&descriptor, Box::new(wrapped));
                        }
                        Err(e) => {
                            edgerun_log::warn!(
                                "edgerund: warning: failed to open evdev device: {}",
                                e
                            )
                        }
                    }
                }
            }
            Err(e) => edgerun_log::warn!("edgerund: warning: evdev discovery failed: {}", e),
        }

        // --- Speakers (ALSA) ---
        match edgerun_alsa_speaker::discover_speakers() {
            Ok(speakers) => {
                for backend in speakers {
                    let descriptor = backend.descriptor();
                    let adapter = SpeakerRemoteAdapter::new(backend);
                    let wrapped = PolicyWrappedProvider::with_policy(adapter, policy.clone())
                        .with_context(context.clone());
                    multi.register(&descriptor, Box::new(wrapped));
                }
            }
            Err(e) => edgerun_log::warn!("edgerund: warning: ALSA speaker discovery failed: {}", e),
        }

        // --- Microphones (ALSA) ---
        match edgerun_alsa_microphone::discover_alsa_pcms() {
            Ok(pcms) => {
                let capture_pcms: Vec<_> = pcms.into_iter().filter(|p| p.capture).collect();
                for pcm in capture_pcms {
                    match edgerun_alsa_microphone::AlsaMicrophoneBackend::open(pcm.clone()) {
                        Ok(backend) => {
                            let descriptor = backend.descriptor();
                            let adapter = MicrophoneRemoteAdapter::new(
                                backend,
                                AudioCaptureRequest {
                                    sample_rate_hz: 48_000,
                                    channels: 2,
                                    duration_ms: 1000,
                                    format: MicrophoneSampleFormat::PcmS16Le,
                                },
                            );
                            let wrapped =
                                PolicyWrappedProvider::with_policy(adapter, policy.clone())
                                    .with_context(context.clone());
                            multi.register(&descriptor, Box::new(wrapped));
                        }
                        Err(e) => {
                            edgerun_log::warn!("edgerund: warning: failed to open ALSA mic: {}", e)
                        }
                    }
                }
            }
            Err(e) => {
                edgerun_log::warn!("edgerund: warning: ALSA microphone discovery failed: {}", e)
            }
        }

        // --- Cameras (V4L2) ---
        match discover_camera_devices() {
            Ok(devices) => {
                for device in devices {
                    let reader = V4l2CameraBiometricReader::new(device);
                    let descriptor = reader.descriptor();
                    let adapter = CameraRemoteAdapter::new(
                        reader,
                        CameraBiometricPurpose::Enrollment,
                        5000,
                        descriptor.clone(),
                    );
                    let wrapped = PolicyWrappedProvider::with_policy(adapter, policy.clone())
                        .with_context(context.clone());
                    multi.register(&descriptor, Box::new(wrapped));
                }
            }
            Err(e) => edgerun_log::warn!("edgerund: warning: V4L2 camera discovery failed: {}", e),
        }
    }

    #[cfg(feature = "all-hardware")]
    {
        // --- WiFi scanning and control (discover once, use for both) ---
        match edgerun_linux_wifi::discover_wifi_interfaces() {
            Ok(interfaces) => {
                // WiFi scanning adapters
                for (i, iface) in interfaces.iter().enumerate() {
                    let backend = edgerun_linux_wifi::LinuxWifiBackend {
                        interface: iface.clone(),
                    };
                    let instance_id = format!("wifi-{}", i);
                    let descriptor =
                        edgerun_wifi::default_wifi_descriptor("edgerun-linux-wifi", &instance_id);
                    let adapter = WifiRemoteAdapter::new(backend, descriptor.clone());
                    let wrapped = PolicyWrappedProvider::with_policy(adapter, policy.clone())
                        .with_context(context.clone());
                    multi.register(&descriptor, Box::new(wrapped));
                }
                // WiFi control adapters (same interfaces, different adapter type)
                for (i, iface) in interfaces.iter().enumerate() {
                    let backend = edgerun_linux_wifi::LinuxWifiBackend {
                        interface: iface.clone(),
                    };
                    let instance_id = format!("wifi-ctrl-{}", i);
                    let descriptor =
                        edgerun_wifi::default_wifi_descriptor("edgerun-linux-wifi", &instance_id);
                    let adapter = WifiControlRemoteAdapter::new(backend, descriptor.clone());
                    let wrapped = PolicyWrappedProvider::with_policy(adapter, policy.clone())
                        .with_context(context.clone());
                    multi.register(&descriptor, Box::new(wrapped));
                }
            }
            Err(e) => {
                edgerun_log::warn!("edgerund: warning: WiFi interface discovery failed: {}", e)
            }
        }

        // --- Bluetooth scanning and connections (discover once, use for both) ---
        match edgerun_mgmt_bluetooth::discover_controllers() {
            Ok(controllers) => {
                // Bluetooth scanning adapters
                for ctrl in &controllers {
                    let backend = edgerun_mgmt_bluetooth::MgmtBluetoothBackend {
                        controller: ctrl.clone(),
                    };
                    let instance_id = format!("bt-{}", ctrl.index);
                    let descriptor = edgerun_bluetooth::default_bluetooth_descriptor(
                        "edgerun-mgmt-bluetooth",
                        &instance_id,
                    );
                    let adapter = BluetoothRemoteAdapter::new(backend, descriptor.clone());
                    let wrapped = PolicyWrappedProvider::with_policy(adapter, policy.clone())
                        .with_context(context.clone());
                    multi.register(&descriptor, Box::new(wrapped));
                }
                // Bluetooth connection adapters (same controllers, different adapter type)
                for ctrl in &controllers {
                    let backend = edgerun_mgmt_bluetooth::MgmtBluetoothBackend {
                        controller: ctrl.clone(),
                    };
                    let instance_id = format!("bt-conn-{}", ctrl.index);
                    let descriptor = edgerun_bluetooth::default_bluetooth_descriptor(
                        "edgerun-mgmt-bluetooth",
                        &instance_id,
                    );
                    let adapter =
                        BluetoothConnectionRemoteAdapter::new(backend, descriptor.clone());
                    let wrapped = PolicyWrappedProvider::with_policy(adapter, policy.clone())
                        .with_context(context.clone());
                    multi.register(&descriptor, Box::new(wrapped));
                }
            }
            Err(e) => edgerun_log::warn!(
                "edgerund: warning: Bluetooth controller discovery failed: {}",
                e
            ),
        }
    }

    let _ = (multi, context);
}

// ===========================================================================
// Unix socket server
// ===========================================================================

/// Thread-safe capability server using `Arc<std::sync::Mutex<MultiCapabilityProvider>>`.
/// Listens on a Unix domain socket and serves capabilities to connecting clients.
pub fn serve_capabilities_unix(
    multi: Arc<std::sync::Mutex<MultiCapabilityProvider>>,
    socket_path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    if socket_path.exists() {
        let _ = std::fs::remove_file(socket_path);
    }
    if let Some(parent) = socket_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    let listener = std::os::unix::net::UnixListener::bind(socket_path)?;
    edgerun_log::warn!(
        "edgerund: capability server listening on {}",
        socket_path.display()
    );

    loop {
        match listener.accept() {
            Ok((stream, _addr)) => {
                let mut transport = FramedRemoteTransport::new(stream);
                let mut locked = multi
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
                match serve_one(&mut *locked, &mut transport) {
                    Ok(true) => {}  // connection served
                    Ok(false) => {} // connection closed gracefully
                    Err(e) => edgerun_log::warn!("edgerund: capability serve error: {}", e),
                }
            }
            Err(e) => {
                edgerun_log::warn!("edgerund: capability server: accept error: {}", e);
                if !socket_path.exists() {
                    break; // socket was removed — time to shut down
                }
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
        }
    }

    Ok(())
}

// ===========================================================================
// Mesh capability server
// ===========================================================================

/// Builds a mesh-capable capability server from the local multi-provider.
///
/// Returns `(MeshCapabilityServer, OutboundQueue, MeshEnvelopeDispatcher)`.
/// The daemon should:
/// 1. Register the server's inboxes with the mesh link
/// 2. Drain the `OutboundQueue` and send frames via the mesh
/// 3. Deliver inbound capability frames to the dispatcher
#[cfg(feature = "all-hardware")]
pub fn build_mesh_capability_server(
    multi: MultiCapabilityProvider,
) -> (
    edgerun_mesh_capability::MeshCapabilityServer<MultiCapabilityProvider>,
    edgerun_mesh_capability::OutboundQueue,
    edgerun_mesh_capability::MeshEnvelopeDispatcher,
) {
    use edgerun_mesh_capability::collections::VecDeque;
    use edgerun_mesh_capability::sync::{Arc, Mutex};
    use edgerun_mesh_capability::{MeshCapabilityServer, MeshEnvelopeDispatcher, OutboundQueue};

    let outbound: OutboundQueue = Arc::new(Mutex::new(VecDeque::new()));
    let dispatcher = MeshEnvelopeDispatcher::new();
    let server = MeshCapabilityServer::new(multi);
    (server, outbound, dispatcher)
}

/// Processes one inbound capability envelope for the mesh server.
/// Returns true if an envelope was processed.
#[cfg(feature = "all-hardware")]
pub fn mesh_capability_server_tick<P: RemoteCapabilityProvider>(
    server: &mut edgerun_mesh_capability::MeshCapabilityServer<P>,
) -> Result<bool, edgerun_capabilities::CapabilityError> {
    let mut dummy_link = edgerun_mesh_link::MeshLink::new();
    server.serve_one(&mut dummy_link)
}

// ===========================================================================
// Builds
// ===========================================================================

/// Builds a multi-provider with all discovered capabilities.
/// The `policy` engine should have grants replayed from the event stream first.
pub fn build_multi_provider(policy: SimplePolicyEngine) -> MultiCapabilityProvider {
    let mut multi = MultiCapabilityProvider::new();
    discover_and_register_capabilities(&mut multi, policy);
    multi
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use edgerun_capabilities::{CapabilityDescriptor, CapabilityError};
    use edgerun_proto::edgerun::v0::capability_runtime::{
        CapabilitySessionClose, CapabilitySessionOpen,
    };
    use edgerun_remote_capability::RemoteCapabilityProvider;

    // -----------------------------------------------------------------------
    // MultiCapabilityProvider tests
    // -----------------------------------------------------------------------

    #[test]
    fn multi_provider_new_is_empty() {
        let multi = MultiCapabilityProvider::new();
        assert_eq!(multi.len(), 0);
    }

    #[test]
    fn multi_provider_descriptor_with_no_providers() {
        let multi = MultiCapabilityProvider::new();
        let desc = multi.descriptor();
        assert_eq!(desc.provider_name, "edgerun-multi");
        assert!(desc.capability_id.ends_with(b"-providers"));
    }

    #[test]
    fn multi_provider_descriptor_with_providers() {
        let mut multi = MultiCapabilityProvider::new();
        // Register a dummy provider to test descriptor changes
        let desc = CapabilityDescriptor {
            descriptor_version: 1,
            capability_id: b"test-cap".to_vec(),
            provider_identity: None,
            provider_node: None,
            role: 0,
            modalities: vec![1],
            event_kinds: vec![],
            operations: vec![],
            default_constraints: vec![],
            provider_name: "test-provider".into(),
            provider_instance_id: "inst-1".into(),
            signature: None,
        };
        let dummy = DummyProvider::new();
        multi.register(&desc, Box::new(dummy));
        assert_eq!(multi.len(), 1);

        let multi_desc = multi.descriptor();
        assert!(multi_desc
            .capability_id
            .ends_with(b"1-providers".as_slice()));
    }

    #[test]
    fn multi_provider_register_uses_correct_key() {
        let mut multi = MultiCapabilityProvider::new();
        let desc = CapabilityDescriptor {
            descriptor_version: 1,
            capability_id: b"cap-1".to_vec(),
            provider_identity: None,
            provider_node: None,
            role: 0,
            modalities: vec![2],
            event_kinds: vec![],
            operations: vec![],
            default_constraints: vec![],
            provider_name: "my-provider".into(),
            provider_instance_id: "instance-42".into(),
            signature: None,
        };
        let provider = DummyProvider::new();
        multi.register(&desc, Box::new(provider));
        assert_eq!(multi.len(), 1);
    }

    #[test]
    fn multi_provider_register_overwrites_same_key() {
        let mut multi = MultiCapabilityProvider::new();
        let desc = CapabilityDescriptor {
            descriptor_version: 1,
            capability_id: b"cap".to_vec(),
            provider_identity: None,
            provider_node: None,
            role: 0,
            modalities: vec![1],
            event_kinds: vec![],
            operations: vec![],
            default_constraints: vec![],
            provider_name: "prov".into(),
            provider_instance_id: "inst".into(),
            signature: None,
        };
        multi.register(&desc, Box::new(DummyProvider::new()));
        multi.register(&desc, Box::new(DummyProvider::new()));
        // Same key -> overwrites, so length stays 1
        assert_eq!(multi.len(), 1);
    }

    #[test]
    fn multi_provider_register_multiple_different_keys() {
        let mut multi = MultiCapabilityProvider::new();

        let desc1 = CapabilityDescriptor {
            descriptor_version: 1,
            capability_id: b"cap1".to_vec(),
            provider_identity: None,
            provider_node: None,
            role: 0,
            modalities: vec![1],
            event_kinds: vec![],
            operations: vec![],
            default_constraints: vec![],
            provider_name: "p1".into(),
            provider_instance_id: "i1".into(),
            signature: None,
        };
        let desc2 = CapabilityDescriptor {
            descriptor_version: 1,
            capability_id: b"cap2".to_vec(),
            provider_identity: None,
            provider_node: None,
            role: 0,
            modalities: vec![2],
            event_kinds: vec![],
            operations: vec![],
            default_constraints: vec![],
            provider_name: "p2".into(),
            provider_instance_id: "i2".into(),
            signature: None,
        };

        multi.register(&desc1, Box::new(DummyProvider::new()));
        multi.register(&desc2, Box::new(DummyProvider::new()));
        assert_eq!(multi.len(), 2);
    }

    #[test]
    fn multi_provider_open_session_first_provider_accepts() {
        let mut multi = MultiCapabilityProvider::new();
        let desc = CapabilityDescriptor {
            descriptor_version: 1,
            capability_id: b"session-test".to_vec(),
            provider_identity: None,
            provider_node: None,
            role: 0,
            modalities: vec![1],
            event_kinds: vec![],
            operations: vec![],
            default_constraints: vec![],
            provider_name: "accepting".into(),
            provider_instance_id: "acc-1".into(),
            signature: None,
        };
        let mut accepting = DummyProvider::new();
        accepting.accept_sessions = true;
        multi.register(&desc, Box::new(accepting));

        let open = CapabilitySessionOpen {
            version: 1,
            session_id: vec![1, 2, 3],
            selector: None,
            mode: 0,
            requested_operations: vec![],
            requested_access_class: 0,
            requested_constraints: vec![],
            correlation_id: vec![],
        };
        let result = multi.open_session(&open);
        assert!(result.is_ok());
    }

    #[test]
    fn multi_provider_open_session_all_reject_returns_error() {
        let mut multi = MultiCapabilityProvider::new();
        let desc = CapabilityDescriptor {
            descriptor_version: 1,
            capability_id: b"reject-test".to_vec(),
            provider_identity: None,
            provider_node: None,
            role: 0,
            modalities: vec![1],
            event_kinds: vec![],
            operations: vec![],
            default_constraints: vec![],
            provider_name: "rejecting".into(),
            provider_instance_id: "rej-1".into(),
            signature: None,
        };
        multi.register(&desc, Box::new(DummyProvider::new()));

        let open = CapabilitySessionOpen {
            version: 1,
            session_id: vec![1],
            selector: None,
            mode: 0,
            requested_operations: vec![],
            requested_access_class: 0,
            requested_constraints: vec![],
            correlation_id: vec![],
        };
        let result = multi.open_session(&open);
        assert!(result.is_err());
    }

    #[test]
    fn multi_provider_open_session_empty_returns_unsupported() {
        let mut multi = MultiCapabilityProvider::new();
        let open = CapabilitySessionOpen {
            version: 1,
            session_id: vec![],
            selector: None,
            mode: 0,
            requested_operations: vec![],
            requested_access_class: 0,
            requested_constraints: vec![],
            correlation_id: vec![],
        };
        let result = multi.open_session(&open);
        assert!(result.is_err());
        match result.unwrap_err() {
            CapabilityError::Unsupported(msg) => {
                assert!(msg.contains("no capability providers registered"));
            }
            _ => panic!("expected Unsupported error"),
        }
    }

    #[test]
    fn multi_provider_invoke_first_provider_succeeds() {
        let mut multi = MultiCapabilityProvider::new();
        let desc = CapabilityDescriptor {
            descriptor_version: 1,
            capability_id: b"invoke-test".to_vec(),
            provider_identity: None,
            provider_node: None,
            role: 0,
            modalities: vec![1],
            event_kinds: vec![],
            operations: vec![],
            default_constraints: vec![],
            provider_name: "invoker".into(),
            provider_instance_id: "inv-1".into(),
            signature: None,
        };
        let mut invoker = DummyProvider::new();
        invoker.allow_invokes = true;
        multi.register(&desc, Box::new(invoker));

        let invocation = edgerun_capabilities::CapabilityInvocation {
            invocation_version: 1,
            invocation_id: vec![1],
            grant_id: vec![],
            invoker: None,
            operation: 0,
            requested_access_class: 0,
            parameter_object: None,
            correlation_id: vec![],
            invoked_at: None,
            signature: None,
        };
        let result = multi.invoke(b"session-1", &invocation, None);
        assert!(result.is_ok());
    }

    #[test]
    fn multi_provider_invoke_permission_denied_continues() {
        let mut multi = MultiCapabilityProvider::new();
        let desc = CapabilityDescriptor {
            descriptor_version: 1,
            capability_id: b"perm-test".to_vec(),
            provider_identity: None,
            provider_node: None,
            role: 0,
            modalities: vec![1],
            event_kinds: vec![],
            operations: vec![],
            default_constraints: vec![],
            provider_name: "denier".into(),
            provider_instance_id: "den-1".into(),
            signature: None,
        };
        multi.register(&desc, Box::new(DummyProvider::new()));

        let invocation = edgerun_capabilities::CapabilityInvocation {
            invocation_version: 1,
            invocation_id: vec![1],
            grant_id: vec![],
            invoker: None,
            operation: 0,
            requested_access_class: 0,
            parameter_object: None,
            correlation_id: vec![],
            invoked_at: None,
            signature: None,
        };
        let result = multi.invoke(b"session-1", &invocation, None);
        // DummyProvider returns PermissionDenied by default
        assert!(result.is_err());
    }

    #[test]
    fn multi_provider_close_session_calls_all() {
        let mut multi = MultiCapabilityProvider::new();
        let desc = CapabilityDescriptor {
            descriptor_version: 1,
            capability_id: b"close-test".to_vec(),
            provider_identity: None,
            provider_node: None,
            role: 0,
            modalities: vec![1],
            event_kinds: vec![],
            operations: vec![],
            default_constraints: vec![],
            provider_name: "closer".into(),
            provider_instance_id: "close-1".into(),
            signature: None,
        };
        multi.register(&desc, Box::new(DummyProvider::new()));

        let close = CapabilitySessionClose {
            version: 1,
            session_id: vec![1, 2, 3],
            reason: String::new(),
        };
        let result = multi.close_session(&close);
        assert!(result.is_ok());
    }

    #[test]
    fn multi_provider_close_empty_sessions() {
        let mut multi = MultiCapabilityProvider::new();
        let close = CapabilitySessionClose {
            version: 1,
            session_id: vec![],
            reason: String::new(),
        };
        let result = multi.close_session(&close);
        assert!(result.is_ok());
    }

    // -----------------------------------------------------------------------
    // Build multi-provider
    // -----------------------------------------------------------------------

    #[test]
    fn build_multi_provider_returns_multi() {
        let policy = edgerun_capability_policy::SimplePolicyEngine::default();
        let multi = build_multi_provider(policy);
        // Without hardware, no capabilities are discovered, but the multi is valid
        let _ = multi;
    }

    // -----------------------------------------------------------------------
    // Dummy provider for testing
    // -----------------------------------------------------------------------

    struct DummyProvider {
        accept_sessions: bool,
        allow_invokes: bool,
    }

    impl DummyProvider {
        fn new() -> Self {
            Self {
                accept_sessions: false,
                allow_invokes: false,
            }
        }
    }

    impl RemoteCapabilityProvider for DummyProvider {
        fn descriptor(&self) -> CapabilityDescriptor {
            CapabilityDescriptor {
                descriptor_version: 1,
                capability_id: b"dummy".to_vec(),
                provider_identity: None,
                provider_node: None,
                role: 0,
                modalities: vec![0],
                event_kinds: vec![],
                operations: vec![],
                default_constraints: vec![],
                provider_name: "dummy".into(),
                provider_instance_id: "dummy-1".into(),
                signature: None,
            }
        }

        fn open_session(
            &mut self,
            _open: &CapabilitySessionOpen,
        ) -> Result<
            edgerun_proto::edgerun::v0::capability_runtime::CapabilitySessionAccept,
            CapabilityError,
        > {
            if self.accept_sessions {
                Ok(
                    edgerun_proto::edgerun::v0::capability_runtime::CapabilitySessionAccept {
                        version: 1,
                        session_id: vec![1, 2, 3],
                        accepted: true,
                        granted_operations: vec![],
                        granted_access_class: 0,
                        error_reason: String::new(),
                        grant_id: vec![],
                    },
                )
            } else {
                Err(CapabilityError::Unsupported("not accepting"))
            }
        }

        fn invoke(
            &mut self,
            _session_id: &[u8],
            _invocation: &edgerun_capabilities::CapabilityInvocation,
            _inline_parameters: Option<&[u8]>,
        ) -> Result<edgerun_remote_capability::RemoteInvocationResult, CapabilityError> {
            if self.allow_invokes {
                Ok(edgerun_remote_capability::RemoteInvocationResult {
                    result: edgerun_proto::edgerun::v0::capability::CapabilityResult {
                        result_version: 1,
                        invocation_id: vec![],
                        grant_id: vec![],
                        success: true,
                        result_access_class: 0,
                        produced_event_kinds: vec![],
                        payload_object: None,
                        error_reason: String::new(),
                        produced_at: None,
                        signature: None,
                    },
                    inline_payload: vec![],
                })
            } else {
                Err(CapabilityError::PermissionDenied("no access"))
            }
        }

        fn close_session(
            &mut self,
            _close: &CapabilitySessionClose,
        ) -> Result<(), CapabilityError> {
            Ok(())
        }
    }
}
