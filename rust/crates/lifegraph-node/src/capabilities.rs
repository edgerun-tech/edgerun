//! Capability discovery and server for the Lifegraph node daemon.
//!
//! Discovers hardware devices, wraps them as `RemoteCapabilityProvider`s,
//! aggregates them into a multi-provider, and serves them over a Unix socket.

use lifegraph_capabilities::{CapabilityDescriptor, CapabilityProvider};
use lifegraph_capability_policy::{PolicyContext, SimplePolicyEngine};
use lifegraph_evdev_input::EvdevInputBackend;
use lifegraph_remote_capability::{
    CameraRemoteAdapter, FramedRemoteTransport, InputRemoteAdapter,
    MicrophoneRemoteAdapter, PolicyWrappedProvider, RemoteCapabilityProvider,
    SpeakerRemoteAdapter, serve_one,
};
use lifegraph_v4l2_camera::{V4l2CameraBiometricReader, discover_camera_devices};
use lifegraph_camera_biometrics::CameraBiometricPurpose;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

// ---------------------------------------------------------------------------
// Multi-provider — aggregates multiple capability providers into one
// ---------------------------------------------------------------------------

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
    pub fn register(&mut self, descriptor: &CapabilityDescriptor, provider: Box<dyn RemoteCapabilityProvider + Send>) {
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
            provider_name: "lifegraph-multi".into(),
            provider_instance_id: id,
            signature: None,
        }
    }

    fn open_session(
        &mut self,
        open: &lifegraph_proto::lifegraph::v0::capability_runtime::CapabilitySessionOpen,
    ) -> Result<lifegraph_proto::lifegraph::v0::capability_runtime::CapabilitySessionAccept, lifegraph_capabilities::CapabilityError> {
        let mut last_err = None;
        for (key, provider) in self.providers.iter_mut() {
            match provider.open_session(open) {
                Ok(accept) => return Ok(accept),
                Err(e) => last_err = Some((key.clone(), e)),
            }
        }
        Err(last_err.map(|(_, e)| e).unwrap_or_else(|| {
            lifegraph_capabilities::CapabilityError::Unsupported("no capability providers registered")
        }))
    }

    fn invoke(
        &mut self,
        session_id: &[u8],
        invocation: &lifegraph_capabilities::CapabilityInvocation,
        inline_parameters: Option<&[u8]>,
    ) -> Result<lifegraph_remote_capability::RemoteInvocationResult, lifegraph_capabilities::CapabilityError> {
        for provider in self.providers.values_mut() {
            match provider.invoke(session_id, invocation, inline_parameters) {
                Ok(result) => return Ok(result),
                Err(lifegraph_capabilities::CapabilityError::PermissionDenied(_)) => continue,
                Err(e) => return Err(e),
            }
        }
        Err(lifegraph_capabilities::CapabilityError::PermissionDenied(
            "no provider authorized for this session",
        ))
    }

    fn close_session(
        &mut self,
        close: &lifegraph_proto::lifegraph::v0::capability_runtime::CapabilitySessionClose,
    ) -> Result<(), lifegraph_capabilities::CapabilityError> {
        for provider in self.providers.values_mut() {
            let _ = provider.close_session(close);
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Hardware discovery
// ---------------------------------------------------------------------------

/// Discovers all available hardware capabilities and registers them with the
/// multi-provider, wrapped in policy enforcement.
pub fn discover_and_register_capabilities(
    multi: &mut MultiCapabilityProvider,
    policy: SimplePolicyEngine,
) {
    let context = PolicyContext {
        is_local: true,
        ..PolicyContext::default()
    };

    // --- Input devices (evdev) ---
    match lifegraph_evdev_input::discover_evdev_devices() {
        Ok(devices) => {
            for device_info in devices {
                match EvdevInputBackend::open(device_info.clone()) {
                    Ok(backend) => {
                        let descriptor = backend.descriptor();
                        let adapter = InputRemoteAdapter::new(backend, 64);
                        let wrapped = PolicyWrappedProvider::with_policy(adapter, policy.clone())
                            .with_context(context.clone());
                        multi.register(&descriptor, Box::new(wrapped));
                    }
                    Err(e) => eprintln!("lifegraphd: warning: failed to open evdev device: {}", e),
                }
            }
        }
        Err(e) => eprintln!("lifegraphd: warning: evdev discovery failed: {}", e),
    }

    // --- Speakers (ALSA) ---
    match lifegraph_alsa_speaker::discover_speakers() {
        Ok(speakers) => {
            for backend in speakers {
                let descriptor = backend.descriptor();
                let adapter = SpeakerRemoteAdapter::new(backend);
                let wrapped = PolicyWrappedProvider::with_policy(adapter, policy.clone())
                    .with_context(context.clone());
                multi.register(&descriptor, Box::new(wrapped));
            }
        }
        Err(e) => eprintln!("lifegraphd: warning: ALSA speaker discovery failed: {}", e),
    }

    // --- Microphones (ALSA) ---
    match lifegraph_alsa_microphone::discover_alsa_pcms() {
        Ok(pcms) => {
            let capture_pcms: Vec<_> = pcms.into_iter().filter(|p| p.capture).collect();
            for pcm in capture_pcms {
                match lifegraph_alsa_microphone::AlsaMicrophoneBackend::open(pcm.clone()) {
                    Ok(backend) => {
                        let descriptor = backend.descriptor();
                        let adapter = MicrophoneRemoteAdapter::new(backend, lifegraph_microphone::AudioCaptureRequest {
                            sample_rate_hz: 48_000,
                            channels: 2,
                            duration_ms: 1000,
                            format: lifegraph_microphone::MicrophoneSampleFormat::PcmS16Le,
                        });
                        let wrapped = PolicyWrappedProvider::with_policy(adapter, policy.clone())
                            .with_context(context.clone());
                        multi.register(&descriptor, Box::new(wrapped));
                    }
                    Err(e) => eprintln!("lifegraphd: warning: failed to open ALSA mic: {}", e),
                }
            }
        }
        Err(e) => eprintln!("lifegraphd: warning: ALSA microphone discovery failed: {}", e),
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
        Err(e) => eprintln!("lifegraphd: warning: V4L2 camera discovery failed: {}", e),
    }
}

// ---------------------------------------------------------------------------
// Unix socket server
// ---------------------------------------------------------------------------

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
    println!("lifegraphd: capability server listening on {}", socket_path.display());

    loop {
        match listener.accept() {
            Ok((stream, _addr)) => {
                let mut transport = FramedRemoteTransport::new(stream);
                let mut locked = multi.lock().unwrap();
                match serve_one(&mut *locked, &mut transport) {
                    Ok(true) => {} // connection served
                    Ok(false) => {} // connection closed gracefully
                    Err(e) => eprintln!("lifegraphd: capability serve error: {}", e),
                }
            }
            Err(e) => {
                eprintln!("lifegraphd: capability server: accept error: {}", e);
                if !socket_path.exists() {
                    break; // socket was removed — time to shut down
                }
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
        }
    }

    Ok(())
}

/// Builds a multi-provider with all discovered capabilities.
/// Used by the mesh capability server.
pub fn build_multi_provider(_node_private_key: &[u8]) -> MultiCapabilityProvider {
    let mut multi = MultiCapabilityProvider::new();
    let policy = SimplePolicyEngine::default();
    discover_and_register_capabilities(&mut multi, policy);
    multi
}
