use edgerun_bluetooth::{
    BluetoothAddressKind, BluetoothBeaconObservation, BluetoothConnectionInfo,
    BluetoothConnectionProvider, BluetoothLinkKind, BluetoothProfile, BluetoothScanResult,
    BluetoothScanner, BluetoothTransportKind,
};
use edgerun_camera_biometrics::{
    CameraBiometricError, CameraBiometricPurpose, CameraBiometricReader, CameraCapture,
    CameraCaptureQuality, CameraFrame, CameraPixelFormat, CameraStreamRole,
    PairedCameraBiometricReader, PairedCameraFrame,
};
use edgerun_capabilities::{CapabilityDescriptor, CapabilityError};
use edgerun_capability_policy::{
    PolicyContext, PolicyDecision, PolicyEngine, RevocationReason, SimplePolicyEngine,
};
use edgerun_input::{InputDevice, InputEventKind, InputEventRecord};
use edgerun_microphone::{
    AudioCapture, AudioCaptureRequest, MicrophoneDevice, MicrophoneSampleFormat,
};
use edgerun_proto::edgerun::v0::capability::{
    CapabilityGrant, CapabilityInvocation, CapabilityRequest, CapabilityResult,
    CapabilityRevocation,
};
pub use edgerun_proto::edgerun::v0::capability_runtime::{
    capability_remote_envelope, CapabilityInvocationFrame, CapabilityRemoteEnvelope,
    CapabilityResultFrame, CapabilitySessionAccept, CapabilitySessionClose, CapabilitySessionEvent,
    CapabilitySessionMode, CapabilitySessionOpen,
};
use prost::Message;
use edgerun_speaker::{
    AudioPlaybackRequest, AudioPlaybackResult, SpeakerDevice, SpeakerOutputLevel,
    SpeakerSampleFormat,
};
use edgerun_wifi::{
    WifiController, WifiInterfaceInfo, WifiInterfaceMode, WifiNetworkObservation, WifiPowerState,
    WifiScanResult, WifiScanner,
};
use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream, ToSocketAddrs};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::Path;
use std::rc::Rc;

pub mod capability_signature;
pub use capability_signature::{
    sign_invocation, sign_request, sign_grant, sign_result, sign_revocation,
    verify_invocation, verify_request, verify_grant, verify_result, verify_revocation,
};

pub struct RemoteInvocationResult {
    pub result: CapabilityResult,
    pub inline_payload: Vec<u8>,
}

pub trait RemoteCapabilityProvider {
    fn descriptor(&self) -> CapabilityDescriptor;
    fn open_session(
        &mut self,
        open: &CapabilitySessionOpen,
    ) -> Result<CapabilitySessionAccept, CapabilityError>;
    fn invoke(
        &mut self,
        session_id: &[u8],
        invocation: &CapabilityInvocation,
        inline_parameters: Option<&[u8]>,
    ) -> Result<RemoteInvocationResult, CapabilityError>;
    fn next_event(
        &mut self,
        _session_id: &[u8],
    ) -> Result<Option<CapabilitySessionEvent>, CapabilityError> {
        Ok(None)
    }
    fn close_session(&mut self, _close: &CapabilitySessionClose) -> Result<(), CapabilityError> {
        Ok(())
    }
    fn handle_request(
        &mut self,
        _request: &CapabilityRequest,
    ) -> Result<Option<CapabilityGrant>, CapabilityError> {
        Ok(None)
    }
    fn handle_grant(&mut self, _grant: &CapabilityGrant) -> Result<(), CapabilityError> {
        Ok(())
    }
    fn handle_revocation(
        &mut self,
        _revocation: &CapabilityRevocation,
    ) -> Result<(), CapabilityError> {
        Ok(())
    }
}

pub trait IntoPolicyWrappedProvider: RemoteCapabilityProvider + Sized {
    fn into_policy_wrapped(self) -> PolicyWrappedProvider<Self> {
        PolicyWrappedProvider::new(self)
    }

    fn into_policy_wrapped_with(
        self,
        policy: SimplePolicyEngine,
        context: PolicyContext,
    ) -> PolicyWrappedProvider<Self> {
        PolicyWrappedProvider::with_policy(self, policy).with_context(context)
    }
}

impl<T> IntoPolicyWrappedProvider for T where T: RemoteCapabilityProvider {}

#[derive(Clone, Debug)]
pub struct SessionGrantBinding {
    pub session_id: Vec<u8>,
    pub grant_id: Vec<u8>,
    pub granted_operations: Vec<i32>,
    pub granted_access_class: i32,
}

#[derive(Debug)]
pub struct PolicyWrappedProvider<P> {
    inner: P,
    policy: SimplePolicyEngine,
    context: PolicyContext,
    /// Sessions keyed by grant_id (used by invocations).
    sessions: HashMap<Vec<u8>, SessionGrantBinding>,
    /// Reverse index: session_id → grant_id (used by close_session).
    session_to_grant: HashMap<Vec<u8>, Vec<u8>>,
}

impl<P> PolicyWrappedProvider<P> {
    pub fn new(inner: P) -> Self {
        Self {
            inner,
            policy: SimplePolicyEngine::default(),
            context: PolicyContext {
                is_local: false,
                ..PolicyContext::default()
            },
            sessions: HashMap::new(),
            session_to_grant: HashMap::new(),
        }
    }

    pub fn with_policy(inner: P, policy: SimplePolicyEngine) -> Self {
        Self {
            inner,
            policy,
            context: PolicyContext {
                is_local: false,
                ..PolicyContext::default()
            },
            sessions: HashMap::new(),
            session_to_grant: HashMap::new(),
        }
    }

    pub fn with_context(mut self, context: PolicyContext) -> Self {
        self.context = context;
        self
    }

    pub fn inner(&self) -> &P {
        &self.inner
    }

    pub fn inner_mut(&mut self) -> &mut P {
        &mut self.inner
    }

    pub fn policy(&self) -> &SimplePolicyEngine {
        &self.policy
    }

    pub fn policy_mut(&mut self) -> &mut SimplePolicyEngine {
        &mut self.policy
    }

    pub fn context(&self) -> &PolicyContext {
        &self.context
    }

    pub fn sessions(&self) -> &HashMap<Vec<u8>, SessionGrantBinding> {
        &self.sessions
    }

    fn request_for_open(
        &self,
        descriptor: &CapabilityDescriptor,
        open: &CapabilitySessionOpen,
    ) -> edgerun_capabilities::CapabilityRequest {
        session_open_as_request(
            open,
            descriptor,
            self.context
                .requester
                .clone()
                .or_else(default_remote_requester_opt),
            self.context.requester_node.clone(),
        )
    }

    fn policy_grant_for_request(
        &mut self,
        descriptor: &CapabilityDescriptor,
        request: &CapabilityRequest,
    ) -> Result<CapabilityGrant, CapabilityError> {
        let decision = self
            .policy
            .evaluate_request(descriptor, request, &self.context)?;
        match decision {
            PolicyDecision::Allow { grant } => Ok(*grant),
            PolicyDecision::Deny { reason } | PolicyDecision::RequireInteraction { reason } => {
                Err(CapabilityError::PermissionDenied(reason))
            }
        }
    }
}

impl<P> RemoteCapabilityProvider for PolicyWrappedProvider<P>
where
    P: RemoteCapabilityProvider,
{
    fn descriptor(&self) -> CapabilityDescriptor {
        self.inner.descriptor()
    }

    fn open_session(
        &mut self,
        open: &CapabilitySessionOpen,
    ) -> Result<CapabilitySessionAccept, CapabilityError> {
        let descriptor = self.inner.descriptor();
        let request = self.request_for_open(&descriptor, open);
        let grant = match self.handle_request(&request) {
            Ok(Some(grant)) => grant,
            Ok(None) => {
                return Ok(session_reject(
                    open,
                    "policy provider did not return a grant for the session request",
                ));
            }
            Err(CapabilityError::PermissionDenied(reason)) => {
                return Ok(session_reject(open, reason));
            }
            Err(err) => return Err(err),
        };

        let mut accept = self.inner.open_session(open)?;
        if !accept.accepted {
            let _ = self
                .policy
                .revoke(&grant.grant_id, RevocationReason::Superseded);
            return Ok(accept);
        }

        let standardized_accept = session_accept_from_grant(open, &grant);
        accept.granted_operations = standardized_accept.granted_operations;
        accept.granted_access_class = standardized_accept.granted_access_class;
        accept.grant_id = standardized_accept.grant_id;
        let binding = SessionGrantBinding {
            session_id: open.session_id.clone(),
            grant_id: grant.grant_id.clone(),
            granted_operations: grant.granted_operations.clone(),
            granted_access_class: grant.access_class,
        };
        // Index by grant_id (used by invocations)
        self.sessions.insert(grant.grant_id.clone(), binding);
        // Reverse index: session_id → grant_id (used by close_session)
        self.session_to_grant.insert(open.session_id.clone(), grant.grant_id);
        Ok(accept)
    }

    fn invoke(
        &mut self,
        session_id: &[u8],
        invocation: &CapabilityInvocation,
        inline_parameters: Option<&[u8]>,
    ) -> Result<RemoteInvocationResult, CapabilityError> {
        let binding = self
            .sessions
            .get(session_id)
            .ok_or(CapabilityError::PermissionDenied(
                "capability session has no active policy grant",
            ))?;
        let mut policy_invocation = invocation.clone();
        policy_invocation.grant_id = binding.grant_id.clone();
        self.policy
            .authorize_invocation(&policy_invocation, &self.context)?;
        self.inner.invoke(session_id, invocation, inline_parameters)
    }

    fn next_event(
        &mut self,
        session_id: &[u8],
    ) -> Result<Option<CapabilitySessionEvent>, CapabilityError> {
        self.inner.next_event(session_id)
    }

    fn close_session(&mut self, close: &CapabilitySessionClose) -> Result<(), CapabilityError> {
        if let Some(grant_id) = self.session_to_grant.remove(&close.session_id) {
            if let Some(binding) = self.sessions.remove(&grant_id) {
                let _ = self
                    .policy
                    .revoke(&binding.grant_id, RevocationReason::Superseded);
            }
        }
        self.inner.close_session(close)
    }

    fn handle_request(
        &mut self,
        request: &CapabilityRequest,
    ) -> Result<Option<CapabilityGrant>, CapabilityError> {
        let descriptor = self.inner.descriptor();
        self.policy_grant_for_request(&descriptor, request)
            .map(Some)
    }

    fn handle_grant(&mut self, grant: &CapabilityGrant) -> Result<(), CapabilityError> {
        self.policy.import_grant(grant.clone())
    }

    fn handle_revocation(
        &mut self,
        revocation: &CapabilityRevocation,
    ) -> Result<(), CapabilityError> {
        self.policy.apply_revocation(revocation.clone())
    }
}

pub trait RemoteCapabilityTransport {
    fn send(&mut self, envelope: CapabilityRemoteEnvelope) -> Result<(), CapabilityError>;
    fn recv(&mut self) -> Result<Option<CapabilityRemoteEnvelope>, CapabilityError>;
}

#[derive(Debug)]
pub struct FramedRemoteTransport<S> {
    stream: S,
}

impl<S> FramedRemoteTransport<S> {
    pub fn new(stream: S) -> Self {
        Self { stream }
    }

    pub fn into_inner(self) -> S {
        self.stream
    }
}

impl FramedRemoteTransport<UnixStream> {
    pub fn connect_unix(path: impl AsRef<Path>) -> Result<Self, CapabilityError> {
        UnixStream::connect(path)
            .map(Self::new)
            .map_err(|err| CapabilityError::Provider(err.to_string()))
    }
}

impl FramedRemoteTransport<TcpStream> {
    pub fn connect_tcp(addr: impl ToSocketAddrs) -> Result<Self, CapabilityError> {
        TcpStream::connect(addr)
            .map(Self::new)
            .map_err(|err| CapabilityError::Provider(err.to_string()))
    }
}

impl<S> RemoteCapabilityTransport for FramedRemoteTransport<S>
where
    S: Read + Write,
{
    fn send(&mut self, envelope: CapabilityRemoteEnvelope) -> Result<(), CapabilityError> {
        let payload = envelope.encode_to_vec();
        let len = u32::try_from(payload.len()).map_err(|_| {
            CapabilityError::InvalidRequest("remote capability envelope exceeds max frame size")
        })?;
        self.stream
            .write_all(&len.to_be_bytes())
            .and_then(|_| self.stream.write_all(&payload))
            .map_err(|err| CapabilityError::Provider(err.to_string()))
    }

    fn recv(&mut self) -> Result<Option<CapabilityRemoteEnvelope>, CapabilityError> {
        let mut len_buf = [0u8; 4];
        let bytes_read = self
            .stream
            .read(&mut len_buf)
            .map_err(|err| CapabilityError::Provider(err.to_string()))?;
        if bytes_read == 0 {
            return Ok(None);
        }
        if bytes_read != 4 {
            self.stream
                .read_exact(&mut len_buf[bytes_read..])
                .map_err(|err| CapabilityError::Provider(err.to_string()))?;
        }
        let len = u32::from_be_bytes(len_buf) as usize;
        let mut payload = vec![0u8; len];
        self.stream
            .read_exact(&mut payload)
            .map_err(|err| CapabilityError::Provider(err.to_string()))?;
        CapabilityRemoteEnvelope::decode(payload.as_slice())
            .map(Some)
            .map_err(|err| CapabilityError::Provider(err.to_string()))
    }
}

pub fn accept_unix(
    listener: &UnixListener,
) -> Result<FramedRemoteTransport<UnixStream>, CapabilityError> {
    listener
        .accept()
        .map(|(stream, _)| FramedRemoteTransport::new(stream))
        .map_err(|err| CapabilityError::Provider(err.to_string()))
}

pub fn accept_tcp(
    listener: &TcpListener,
) -> Result<FramedRemoteTransport<TcpStream>, CapabilityError> {
    listener
        .accept()
        .map(|(stream, _)| FramedRemoteTransport::new(stream))
        .map_err(|err| CapabilityError::Provider(err.to_string()))
}

#[derive(Clone, Debug)]
pub struct MemoryRemoteTransport {
    inbox: Rc<RefCell<VecDeque<CapabilityRemoteEnvelope>>>,
    outbox: Rc<RefCell<VecDeque<CapabilityRemoteEnvelope>>>,
}

impl MemoryRemoteTransport {
    pub fn pair() -> (Self, Self) {
        let a_to_b = Rc::new(RefCell::new(VecDeque::new()));
        let b_to_a = Rc::new(RefCell::new(VecDeque::new()));
        (
            Self {
                inbox: b_to_a.clone(),
                outbox: a_to_b.clone(),
            },
            Self {
                inbox: a_to_b,
                outbox: b_to_a,
            },
        )
    }
}

impl RemoteCapabilityTransport for MemoryRemoteTransport {
    fn send(&mut self, envelope: CapabilityRemoteEnvelope) -> Result<(), CapabilityError> {
        self.outbox.borrow_mut().push_back(envelope);
        Ok(())
    }

    fn recv(&mut self) -> Result<Option<CapabilityRemoteEnvelope>, CapabilityError> {
        Ok(self.inbox.borrow_mut().pop_front())
    }
}

pub fn serve_one<P: RemoteCapabilityProvider, T: RemoteCapabilityTransport>(
    provider: &mut P,
    transport: &mut T,
) -> Result<bool, CapabilityError> {
    let Some(envelope) = transport.recv()? else {
        return Ok(false);
    };
    match envelope.message {
        Some(capability_remote_envelope::Message::SessionOpen(open)) => {
            let accept = provider.open_session(&open)?;
            transport.send(CapabilityRemoteEnvelope {
                message: Some(capability_remote_envelope::Message::SessionAccept(accept)),
            })?;
        }
        Some(capability_remote_envelope::Message::Invocation(invocation)) => {
            let message = match provider.invoke(&invocation.grant_id, &invocation, None) {
                Ok(response) => {
                    if response.inline_payload.is_empty() {
                        capability_remote_envelope::Message::Result(response.result)
                    } else {
                        capability_remote_envelope::Message::ResultFrame(CapabilityResultFrame {
                            result: Some(response.result),
                            inline_payload: response.inline_payload,
                        })
                    }
                }
                Err(err) => capability_remote_envelope::Message::Result(capability_error_result(
                    &invocation,
                    err,
                )),
            };
            transport.send(CapabilityRemoteEnvelope {
                message: Some(message),
            })?;
        }
        Some(capability_remote_envelope::Message::InvocationFrame(frame)) => {
            let Some(invocation) = frame.invocation else {
                return Err(CapabilityError::InvalidRequest(
                    "remote invocation frame must contain an invocation",
                ));
            };
            let message = match provider.invoke(
                &invocation.grant_id,
                &invocation,
                Some(&frame.inline_parameters),
            ) {
                Ok(response) => {
                    if response.inline_payload.is_empty() {
                        capability_remote_envelope::Message::Result(response.result)
                    } else {
                        capability_remote_envelope::Message::ResultFrame(CapabilityResultFrame {
                            result: Some(response.result),
                            inline_payload: response.inline_payload,
                        })
                    }
                }
                Err(err) => capability_remote_envelope::Message::Result(capability_error_result(
                    &invocation,
                    err,
                )),
            };
            transport.send(CapabilityRemoteEnvelope {
                message: Some(message),
            })?;
        }
        Some(capability_remote_envelope::Message::SessionClose(close)) => {
            provider.close_session(&close)?;
        }
        Some(capability_remote_envelope::Message::Request(request)) => {
            if let Some(grant) = provider.handle_request(&request)? {
                transport.send(CapabilityRemoteEnvelope {
                    message: Some(capability_remote_envelope::Message::Grant(grant)),
                })?;
            }
        }
        Some(capability_remote_envelope::Message::Grant(grant)) => {
            provider.handle_grant(&grant)?;
        }
        Some(capability_remote_envelope::Message::Revocation(revocation)) => {
            provider.handle_revocation(&revocation)?;
        }
        _ => {}
    }
    Ok(true)
}

pub fn pump_one_event<P: RemoteCapabilityProvider, T: RemoteCapabilityTransport>(
    provider: &mut P,
    transport: &mut T,
    session_id: &[u8],
) -> Result<bool, CapabilityError> {
    let Some(event) = provider.next_event(session_id)? else {
        return Ok(false);
    };
    transport.send(CapabilityRemoteEnvelope {
        message: Some(capability_remote_envelope::Message::SessionEvent(event)),
    })?;
    Ok(true)
}

fn capability_error_result(
    invocation: &CapabilityInvocation,
    error: CapabilityError,
) -> CapabilityResult {
    CapabilityResult {
        result_version: 1,
        invocation_id: invocation.invocation_id.clone(),
        grant_id: invocation.grant_id.clone(),
        success: false,
        result_access_class: invocation.requested_access_class,
        produced_event_kinds: Vec::new(),
        payload_object: None,
        error_reason: error.to_string(),
        produced_at: None,
        signature: None,
    }
}

fn encode_input_events(events: &[InputEventRecord]) -> Vec<u8> {
    let mut out = Vec::with_capacity(4 + events.len() * 24);
    out.extend_from_slice(&(events.len() as u32).to_le_bytes());
    for event in events {
        out.extend_from_slice(&event.timestamp_sec.to_le_bytes());
        out.extend_from_slice(&event.timestamp_usec.to_le_bytes());
        let kind = match event.kind {
            InputEventKind::Key => 1u16,
            InputEventKind::RelativeMotion => 2,
            InputEventKind::AbsoluteMotion => 3,
            InputEventKind::Switch => 4,
            InputEventKind::Misc => 5,
            InputEventKind::Synchronization => 6,
            InputEventKind::Other(v) => v | 0x8000,
        };
        out.extend_from_slice(&kind.to_le_bytes());
        out.extend_from_slice(&event.code.to_le_bytes());
        out.extend_from_slice(&event.value.to_le_bytes());
    }
    out
}

fn selector_from_descriptor(
    descriptor: &CapabilityDescriptor,
    requested_access_class: i32,
) -> edgerun_capabilities::CapabilitySelector {
    edgerun_capabilities::CapabilitySelector {
        capability_id: descriptor.capability_id.clone(),
        role: descriptor.role,
        modalities: descriptor.modalities.clone(),
        event_kinds: descriptor.event_kinds.clone(),
        operations: descriptor.operations.clone(),
        access_class: requested_access_class,
        provider_identity: descriptor.provider_identity.clone(),
        provider_node: descriptor.provider_node.clone(),
        provider_instance_id: descriptor.provider_instance_id.clone(),
    }
}

pub fn default_remote_requester() -> edgerun_proto::edgerun::v0::common::IdentityRef {
    edgerun_proto::edgerun::v0::common::IdentityRef {
        identity_id: b"remote-capability-client".to_vec(),
        identity_kind: None,
        key_hint: None,
    }
}

fn default_remote_requester_opt() -> Option<edgerun_proto::edgerun::v0::common::IdentityRef> {
    Some(default_remote_requester())
}

pub fn session_open_as_request(
    open: &CapabilitySessionOpen,
    descriptor: &CapabilityDescriptor,
    requester: Option<edgerun_proto::edgerun::v0::common::IdentityRef>,
    requester_node: Option<edgerun_proto::edgerun::v0::common::NodeRef>,
) -> edgerun_capabilities::CapabilityRequest {
    edgerun_capabilities::CapabilityRequest {
        request_version: open.version,
        request_id: open.session_id.clone(),
        requester,
        requester_node,
        selector: open.selector.clone().or_else(|| {
            Some(selector_from_descriptor(
                descriptor,
                open.requested_access_class,
            ))
        }),
        requested_operations: open.requested_operations.clone(),
        requested_constraints: open.requested_constraints.clone(),
        purpose: String::new(),
        requested_duration: None,
        correlation_id: open.correlation_id.clone(),
        signature: None,
    }
}

pub fn session_accept_from_grant(
    open: &CapabilitySessionOpen,
    grant: &edgerun_capabilities::CapabilityGrant,
) -> CapabilitySessionAccept {
    CapabilitySessionAccept {
        version: open.version,
        session_id: open.session_id.clone(),
        accepted: true,
        granted_operations: grant.granted_operations.clone(),
        granted_access_class: grant.access_class,
        error_reason: String::new(),
        grant_id: grant.grant_id.clone(),
    }
}

pub fn session_reject(
    open: &CapabilitySessionOpen,
    reason: impl Into<String>,
) -> CapabilitySessionAccept {
    CapabilitySessionAccept {
        version: open.version,
        session_id: open.session_id.clone(),
        accepted: false,
        granted_operations: Vec::new(),
        granted_access_class: edgerun_capabilities::CapabilityAccessClass::Unspecified as i32,
        error_reason: reason.into(),
        grant_id: Vec::new(),
    }
}

pub fn accept_session_open_unchecked(open: &CapabilitySessionOpen) -> CapabilitySessionAccept {
    CapabilitySessionAccept {
        version: open.version,
        session_id: open.session_id.clone(),
        accepted: true,
        granted_operations: open.requested_operations.clone(),
        granted_access_class: open.requested_access_class,
        error_reason: String::new(),
        grant_id: open.session_id.clone(),
    }
}

pub fn decode_input_events(bytes: &[u8]) -> Result<Vec<InputEventRecord>, CapabilityError> {
    if bytes.len() < 4 {
        return Err(CapabilityError::InvalidRequest(
            "remote input payload too short for event count",
        ));
    }
    let count = u32::from_le_bytes(bytes[0..4].try_into().unwrap()) as usize;
    let expected = 4 + count * 24;
    if bytes.len() != expected {
        return Err(CapabilityError::InvalidRequest(
            "remote input payload length does not match encoded event count",
        ));
    }
    let mut out = Vec::with_capacity(count);
    let mut offset = 4;
    for _ in 0..count {
        let timestamp_sec = i64::from_le_bytes(bytes[offset..offset + 8].try_into().unwrap());
        offset += 8;
        let timestamp_usec = i64::from_le_bytes(bytes[offset..offset + 8].try_into().unwrap());
        offset += 8;
        let raw_kind = u16::from_le_bytes(bytes[offset..offset + 2].try_into().unwrap());
        offset += 2;
        let code = u16::from_le_bytes(bytes[offset..offset + 2].try_into().unwrap());
        offset += 2;
        let value = i32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap());
        offset += 4;
        let kind = match raw_kind {
            1 => InputEventKind::Key,
            2 => InputEventKind::RelativeMotion,
            3 => InputEventKind::AbsoluteMotion,
            4 => InputEventKind::Switch,
            5 => InputEventKind::Misc,
            6 => InputEventKind::Synchronization,
            other if other & 0x8000 != 0 => InputEventKind::Other(other & 0x7fff),
            other => InputEventKind::Other(other),
        };
        out.push(InputEventRecord {
            timestamp_sec,
            timestamp_usec,
            kind,
            code,
            value,
        });
    }
    Ok(out)
}

fn encode_microphone_capture(capture: &AudioCapture) -> Vec<u8> {
    let mut out = Vec::with_capacity(4 + 2 + 4 + 8 + 4 + capture.bytes.len());
    out.extend_from_slice(&capture.sample_rate_hz.to_le_bytes());
    out.extend_from_slice(&capture.channels.to_le_bytes());
    let format = match capture.format {
        MicrophoneSampleFormat::PcmS16Le => 1u32,
        MicrophoneSampleFormat::PcmS24Le => 2,
        MicrophoneSampleFormat::PcmS32Le => 3,
        MicrophoneSampleFormat::Float32Le => 4,
        MicrophoneSampleFormat::Other(v) => v | 0x8000_0000,
    };
    out.extend_from_slice(&format.to_le_bytes());
    out.extend_from_slice(&capture.started_at_unix_ms.to_le_bytes());
    out.extend_from_slice(&(capture.bytes.len() as u32).to_le_bytes());
    out.extend_from_slice(&capture.bytes);
    out
}

pub fn decode_microphone_capture(bytes: &[u8]) -> Result<AudioCapture, CapabilityError> {
    if bytes.len() < 22 {
        return Err(CapabilityError::InvalidRequest(
            "remote microphone payload too short",
        ));
    }
    let sample_rate_hz = u32::from_le_bytes(bytes[0..4].try_into().unwrap());
    let channels = u16::from_le_bytes(bytes[4..6].try_into().unwrap());
    let raw_format = u32::from_le_bytes(bytes[6..10].try_into().unwrap());
    let started_at_unix_ms = i64::from_le_bytes(bytes[10..18].try_into().unwrap());
    let len = u32::from_le_bytes(bytes[18..22].try_into().unwrap()) as usize;
    if bytes.len() != 22 + len {
        return Err(CapabilityError::InvalidRequest(
            "remote microphone payload length does not match encoded byte count",
        ));
    }
    let format = match raw_format {
        1 => MicrophoneSampleFormat::PcmS16Le,
        2 => MicrophoneSampleFormat::PcmS24Le,
        3 => MicrophoneSampleFormat::PcmS32Le,
        4 => MicrophoneSampleFormat::Float32Le,
        other if other & 0x8000_0000 != 0 => MicrophoneSampleFormat::Other(other & 0x7fff_ffff),
        other => MicrophoneSampleFormat::Other(other),
    };
    Ok(AudioCapture {
        sample_rate_hz,
        channels,
        format,
        bytes: bytes[22..].to_vec(),
        started_at_unix_ms,
    })
}

pub fn encode_speaker_playback_request(request: &AudioPlaybackRequest) -> Vec<u8> {
    let mut out = Vec::with_capacity(4 + 4 + 2 + 4 + 2 + 1 + request.audio_bytes.len());
    out.extend_from_slice(&request.duration_ms.to_le_bytes());
    out.extend_from_slice(&request.sample_rate_hz.to_le_bytes());
    out.extend_from_slice(&request.channels.to_le_bytes());
    let format = match request.format {
        SpeakerSampleFormat::PcmS16Le => 1u32,
        SpeakerSampleFormat::PcmS24Le => 2,
        SpeakerSampleFormat::PcmFloat32Le => 3,
    };
    out.extend_from_slice(&format.to_le_bytes());
    out.extend_from_slice(&request.software_gain_percent.unwrap_or(0).to_le_bytes());
    out.push(request.target_output_level_percent.unwrap_or(255));
    out.extend_from_slice(&(request.audio_bytes.len() as u32).to_le_bytes());
    out.extend_from_slice(&request.audio_bytes);
    out
}

pub fn decode_speaker_playback_request(
    bytes: &[u8],
) -> Result<AudioPlaybackRequest, CapabilityError> {
    if bytes.len() < 17 {
        return Err(CapabilityError::InvalidRequest(
            "remote speaker playback request payload too short",
        ));
    }
    let duration_ms = u32::from_le_bytes(bytes[0..4].try_into().unwrap());
    let sample_rate_hz = u32::from_le_bytes(bytes[4..8].try_into().unwrap());
    let channels = u16::from_le_bytes(bytes[8..10].try_into().unwrap());
    let raw_format = u32::from_le_bytes(bytes[10..14].try_into().unwrap());
    let raw_gain = u16::from_le_bytes(bytes[14..16].try_into().unwrap());
    let raw_level = bytes[16];
    let audio_len_offset = 17;
    if bytes.len() < audio_len_offset + 4 {
        return Err(CapabilityError::InvalidRequest(
            "remote speaker playback request missing audio length",
        ));
    }
    let audio_len = u32::from_le_bytes(
        bytes[audio_len_offset..audio_len_offset + 4]
            .try_into()
            .unwrap(),
    ) as usize;
    if bytes.len() != audio_len_offset + 4 + audio_len {
        return Err(CapabilityError::InvalidRequest(
            "remote speaker playback request length does not match encoded byte count",
        ));
    }
    let format = match raw_format {
        1 => SpeakerSampleFormat::PcmS16Le,
        2 => SpeakerSampleFormat::PcmS24Le,
        3 => SpeakerSampleFormat::PcmFloat32Le,
        _ => {
            return Err(CapabilityError::InvalidRequest(
                "remote speaker playback request format is unknown",
            ))
        }
    };
    Ok(AudioPlaybackRequest {
        duration_ms,
        sample_rate_hz,
        channels,
        format,
        audio_bytes: bytes[audio_len_offset + 4..].to_vec(),
        software_gain_percent: if raw_gain == 0 { None } else { Some(raw_gain) },
        target_output_level_percent: if raw_level == 255 {
            None
        } else {
            Some(raw_level)
        },
    })
}

pub fn encode_speaker_output_level(level: &SpeakerOutputLevel) -> Vec<u8> {
    let mut out = Vec::with_capacity(1 + 8 + 8 + 1);
    out.push(level.current_percent);
    out.extend_from_slice(&level.min_raw_value.to_le_bytes());
    out.extend_from_slice(&level.max_raw_value.to_le_bytes());
    out.push(match level.muted {
        Some(true) => 1,
        Some(false) => 2,
        None => 0,
    });
    out
}

pub fn decode_speaker_output_level(bytes: &[u8]) -> Result<SpeakerOutputLevel, CapabilityError> {
    if bytes.len() != 18 {
        return Err(CapabilityError::InvalidRequest(
            "remote speaker output level payload length is invalid",
        ));
    }
    let muted = match bytes[17] {
        0 => None,
        1 => Some(true),
        2 => Some(false),
        _ => {
            return Err(CapabilityError::InvalidRequest(
                "remote speaker output level mute flag is invalid",
            ))
        }
    };
    Ok(SpeakerOutputLevel {
        current_percent: bytes[0],
        min_raw_value: i64::from_le_bytes(bytes[1..9].try_into().unwrap()),
        max_raw_value: i64::from_le_bytes(bytes[9..17].try_into().unwrap()),
        muted,
    })
}

pub fn encode_speaker_playback_result(result: &AudioPlaybackResult) -> Vec<u8> {
    let mut out = Vec::with_capacity(4 + 4 + 2 + 1);
    out.extend_from_slice(&(result.bytes_written as u32).to_le_bytes());
    out.extend_from_slice(&result.sample_rate_hz.to_le_bytes());
    out.extend_from_slice(&result.channels.to_le_bytes());
    out.push(result.finished as u8);
    out
}

pub fn decode_speaker_playback_result(
    bytes: &[u8],
) -> Result<AudioPlaybackResult, CapabilityError> {
    if bytes.len() != 11 {
        return Err(CapabilityError::InvalidRequest(
            "remote speaker playback result payload length is invalid",
        ));
    }
    Ok(AudioPlaybackResult {
        bytes_written: u32::from_le_bytes(bytes[0..4].try_into().unwrap()) as usize,
        sample_rate_hz: u32::from_le_bytes(bytes[4..8].try_into().unwrap()),
        channels: u16::from_le_bytes(bytes[8..10].try_into().unwrap()),
        finished: bytes[10] != 0,
    })
}

fn bluetooth_address_kind_to_u8(kind: BluetoothAddressKind) -> u8 {
    match kind {
        BluetoothAddressKind::Public => 1,
        BluetoothAddressKind::Random => 2,
        BluetoothAddressKind::Unknown => 0,
    }
}

fn bluetooth_address_kind_from_u8(v: u8) -> Result<BluetoothAddressKind, CapabilityError> {
    Ok(match v {
        0 => BluetoothAddressKind::Unknown,
        1 => BluetoothAddressKind::Public,
        2 => BluetoothAddressKind::Random,
        _ => {
            return Err(CapabilityError::InvalidRequest(
                "remote bluetooth address kind is invalid",
            ))
        }
    })
}

fn bluetooth_transport_kind_to_u8(kind: BluetoothTransportKind) -> u8 {
    match kind {
        BluetoothTransportKind::Classic => 1,
        BluetoothTransportKind::LowEnergy => 2,
        BluetoothTransportKind::DualMode => 3,
        BluetoothTransportKind::Unknown => 0,
    }
}

fn bluetooth_transport_kind_from_u8(v: u8) -> Result<BluetoothTransportKind, CapabilityError> {
    Ok(match v {
        0 => BluetoothTransportKind::Unknown,
        1 => BluetoothTransportKind::Classic,
        2 => BluetoothTransportKind::LowEnergy,
        3 => BluetoothTransportKind::DualMode,
        _ => {
            return Err(CapabilityError::InvalidRequest(
                "remote bluetooth transport kind is invalid",
            ))
        }
    })
}

fn bluetooth_profile_to_u8(profile: BluetoothProfile) -> u8 {
    match profile {
        BluetoothProfile::AudioSink => 1,
        BluetoothProfile::AudioSource => 2,
        BluetoothProfile::Headset => 3,
        BluetoothProfile::HandsFree => 4,
        BluetoothProfile::HearingAid => 5,
        BluetoothProfile::Microphone => 6,
        BluetoothProfile::Speaker => 7,
        BluetoothProfile::Headphones => 8,
        BluetoothProfile::CarAudio => 9,
        BluetoothProfile::Hid => 10,
        BluetoothProfile::HeartRate => 11,
        BluetoothProfile::BatteryService => 12,
        BluetoothProfile::Other => 255,
    }
}

fn bluetooth_profile_from_u8(v: u8) -> Result<BluetoothProfile, CapabilityError> {
    Ok(match v {
        1 => BluetoothProfile::AudioSink,
        2 => BluetoothProfile::AudioSource,
        3 => BluetoothProfile::Headset,
        4 => BluetoothProfile::HandsFree,
        5 => BluetoothProfile::HearingAid,
        6 => BluetoothProfile::Microphone,
        7 => BluetoothProfile::Speaker,
        8 => BluetoothProfile::Headphones,
        9 => BluetoothProfile::CarAudio,
        10 => BluetoothProfile::Hid,
        11 => BluetoothProfile::HeartRate,
        12 => BluetoothProfile::BatteryService,
        255 => BluetoothProfile::Other,
        _ => {
            return Err(CapabilityError::InvalidRequest(
                "remote bluetooth profile is invalid",
            ))
        }
    })
}

fn bluetooth_link_kind_to_u8(kind: BluetoothLinkKind) -> u8 {
    match kind {
        BluetoothLinkKind::Sco => 1,
        BluetoothLinkKind::Acl => 2,
        BluetoothLinkKind::Esco => 3,
        BluetoothLinkKind::Unknown => 0,
    }
}

fn bluetooth_link_kind_from_u8(v: u8) -> Result<BluetoothLinkKind, CapabilityError> {
    Ok(match v {
        0 => BluetoothLinkKind::Unknown,
        1 => BluetoothLinkKind::Sco,
        2 => BluetoothLinkKind::Acl,
        3 => BluetoothLinkKind::Esco,
        _ => {
            return Err(CapabilityError::InvalidRequest(
                "remote bluetooth link kind is invalid",
            ))
        }
    })
}

fn encode_string_field(value: &str, out: &mut Vec<u8>) {
    let bytes = value.as_bytes();
    out.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
    out.extend_from_slice(bytes);
}

fn decode_string_field(bytes: &[u8], cursor: &mut usize) -> Result<String, CapabilityError> {
    if bytes.len() < *cursor + 4 {
        return Err(CapabilityError::InvalidRequest(
            "remote string field length missing",
        ));
    }
    let len = u32::from_le_bytes(bytes[*cursor..*cursor + 4].try_into().unwrap()) as usize;
    *cursor += 4;
    if bytes.len() < *cursor + len {
        return Err(CapabilityError::InvalidRequest(
            "remote string field payload too short",
        ));
    }
    let s = String::from_utf8(bytes[*cursor..*cursor + len].to_vec())
        .map_err(|_| CapabilityError::InvalidRequest("remote string field is not valid utf-8"))?;
    *cursor += len;
    Ok(s)
}

fn encode_optional_string_field(value: &Option<String>, out: &mut Vec<u8>) {
    out.push(value.is_some() as u8);
    if let Some(v) = value {
        encode_string_field(v, out);
    }
}

fn decode_optional_string_field(
    bytes: &[u8],
    cursor: &mut usize,
) -> Result<Option<String>, CapabilityError> {
    if bytes.len() < *cursor + 1 {
        return Err(CapabilityError::InvalidRequest(
            "remote optional string presence byte missing",
        ));
    }
    let present = bytes[*cursor] != 0;
    *cursor += 1;
    if present {
        Ok(Some(decode_string_field(bytes, cursor)?))
    } else {
        Ok(None)
    }
}

fn encode_string_vec(values: &[String], out: &mut Vec<u8>) {
    out.extend_from_slice(&(values.len() as u32).to_le_bytes());
    for value in values {
        encode_string_field(value, out);
    }
}

fn decode_string_vec(bytes: &[u8], cursor: &mut usize) -> Result<Vec<String>, CapabilityError> {
    if bytes.len() < *cursor + 4 {
        return Err(CapabilityError::InvalidRequest(
            "remote string vector length missing",
        ));
    }
    let count = u32::from_le_bytes(bytes[*cursor..*cursor + 4].try_into().unwrap()) as usize;
    *cursor += 4;
    let mut values = Vec::with_capacity(count);
    for _ in 0..count {
        values.push(decode_string_field(bytes, cursor)?);
    }
    Ok(values)
}

fn encode_profiles_vec(values: &[BluetoothProfile], out: &mut Vec<u8>) {
    out.extend_from_slice(&(values.len() as u32).to_le_bytes());
    for value in values {
        out.push(bluetooth_profile_to_u8(*value));
    }
}

fn decode_profiles_vec(
    bytes: &[u8],
    cursor: &mut usize,
) -> Result<Vec<BluetoothProfile>, CapabilityError> {
    if bytes.len() < *cursor + 4 {
        return Err(CapabilityError::InvalidRequest(
            "remote bluetooth profile count missing",
        ));
    }
    let count = u32::from_le_bytes(bytes[*cursor..*cursor + 4].try_into().unwrap()) as usize;
    *cursor += 4;
    if bytes.len() < *cursor + count {
        return Err(CapabilityError::InvalidRequest(
            "remote bluetooth profiles payload too short",
        ));
    }
    let mut values = Vec::with_capacity(count);
    for _ in 0..count {
        values.push(bluetooth_profile_from_u8(bytes[*cursor])?);
        *cursor += 1;
    }
    Ok(values)
}

pub fn encode_bluetooth_scan_result(scan: &BluetoothScanResult) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&(scan.observations.len() as u32).to_le_bytes());
    for observation in &scan.observations {
        encode_string_field(&observation.device_id, &mut out);
        out.push(bluetooth_transport_kind_to_u8(observation.transport_kind));
        out.push(bluetooth_address_kind_to_u8(observation.address_kind));
        out.extend_from_slice(&observation.rssi_dbm.to_le_bytes());
        out.push(observation.tx_power_dbm.is_some() as u8);
        if let Some(v) = observation.tx_power_dbm {
            out.extend_from_slice(&v.to_le_bytes());
        }
        encode_optional_string_field(&observation.local_name, &mut out);
        encode_string_vec(&observation.service_uuids, &mut out);
        encode_profiles_vec(&observation.profiles, &mut out);
        out.push(observation.classic_device_class.is_some() as u8);
        if let Some(class) = observation.classic_device_class {
            out.extend_from_slice(&class.to_le_bytes());
        }
        out.extend_from_slice(&(observation.advertisement_data.len() as u32).to_le_bytes());
        out.extend_from_slice(&observation.advertisement_data);
        out.extend_from_slice(&observation.captured_at_unix_ms.to_le_bytes());
    }
    out
}

pub fn decode_bluetooth_scan_result(bytes: &[u8]) -> Result<BluetoothScanResult, CapabilityError> {
    if bytes.len() < 4 {
        return Err(CapabilityError::InvalidRequest(
            "remote bluetooth scan payload too short",
        ));
    }
    let mut cursor = 0usize;
    let count = u32::from_le_bytes(bytes[cursor..cursor + 4].try_into().unwrap()) as usize;
    cursor += 4;
    let mut observations = Vec::with_capacity(count);
    for _ in 0..count {
        let device_id = decode_string_field(bytes, &mut cursor)?;
        if bytes.len() < cursor + 4 {
            return Err(CapabilityError::InvalidRequest(
                "remote bluetooth observation header too short",
            ));
        }
        let transport_kind = bluetooth_transport_kind_from_u8(bytes[cursor])?;
        cursor += 1;
        let address_kind = bluetooth_address_kind_from_u8(bytes[cursor])?;
        cursor += 1;
        let rssi_dbm = i16::from_le_bytes(bytes[cursor..cursor + 2].try_into().unwrap());
        cursor += 2;
        if bytes.len() < cursor + 1 {
            return Err(CapabilityError::InvalidRequest(
                "remote bluetooth tx power flag missing",
            ));
        }
        let has_tx = bytes[cursor] != 0;
        cursor += 1;
        let tx_power_dbm = if has_tx {
            if bytes.len() < cursor + 2 {
                return Err(CapabilityError::InvalidRequest(
                    "remote bluetooth tx power payload too short",
                ));
            }
            let v = i16::from_le_bytes(bytes[cursor..cursor + 2].try_into().unwrap());
            cursor += 2;
            Some(v)
        } else {
            None
        };
        let local_name = decode_optional_string_field(bytes, &mut cursor)?;
        let service_uuids = decode_string_vec(bytes, &mut cursor)?;
        let profiles = decode_profiles_vec(bytes, &mut cursor)?;
        if bytes.len() < cursor + 1 {
            return Err(CapabilityError::InvalidRequest(
                "remote bluetooth classic class flag missing",
            ));
        }
        let has_class = bytes[cursor] != 0;
        cursor += 1;
        let classic_device_class = if has_class {
            if bytes.len() < cursor + 4 {
                return Err(CapabilityError::InvalidRequest(
                    "remote bluetooth classic class payload too short",
                ));
            }
            let v = u32::from_le_bytes(bytes[cursor..cursor + 4].try_into().unwrap());
            cursor += 4;
            Some(v)
        } else {
            None
        };
        if bytes.len() < cursor + 4 {
            return Err(CapabilityError::InvalidRequest(
                "remote bluetooth advertisement length missing",
            ));
        }
        let adv_len = u32::from_le_bytes(bytes[cursor..cursor + 4].try_into().unwrap()) as usize;
        cursor += 4;
        if bytes.len() < cursor + adv_len + 8 {
            return Err(CapabilityError::InvalidRequest(
                "remote bluetooth advertisement payload too short",
            ));
        }
        let advertisement_data = bytes[cursor..cursor + adv_len].to_vec();
        cursor += adv_len;
        let captured_at_unix_ms = i64::from_le_bytes(bytes[cursor..cursor + 8].try_into().unwrap());
        cursor += 8;
        observations.push(BluetoothBeaconObservation {
            device_id,
            transport_kind,
            address_kind,
            rssi_dbm,
            tx_power_dbm,
            local_name,
            service_uuids,
            profiles,
            classic_device_class,
            advertisement_data,
            captured_at_unix_ms,
        });
    }
    if cursor != bytes.len() {
        return Err(CapabilityError::InvalidRequest(
            "remote bluetooth scan payload trailing bytes are invalid",
        ));
    }
    Ok(BluetoothScanResult { observations })
}

pub fn encode_bluetooth_connections(connections: &[BluetoothConnectionInfo]) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&(connections.len() as u32).to_le_bytes());
    for connection in connections {
        encode_string_field(&connection.device_id, &mut out);
        out.push(bluetooth_transport_kind_to_u8(connection.transport_kind));
        out.push(bluetooth_address_kind_to_u8(connection.address_kind));
        out.push(bluetooth_link_kind_to_u8(connection.link_kind));
        out.push(connection.outbound as u8);
        out.extend_from_slice(&connection.state.to_le_bytes());
        encode_optional_string_field(&connection.local_name, &mut out);
        encode_string_vec(&connection.service_uuids, &mut out);
        encode_profiles_vec(&connection.profiles, &mut out);
        out.push(match connection.trusted {
            Some(true) => 1,
            Some(false) => 2,
            None => 0,
        });
        out.push(match connection.paired {
            Some(true) => 1,
            Some(false) => 2,
            None => 0,
        });
    }
    out
}

pub fn decode_bluetooth_connections(
    bytes: &[u8],
) -> Result<Vec<BluetoothConnectionInfo>, CapabilityError> {
    if bytes.len() < 4 {
        return Err(CapabilityError::InvalidRequest(
            "remote bluetooth connections payload too short",
        ));
    }
    let mut cursor = 0usize;
    let count = u32::from_le_bytes(bytes[cursor..cursor + 4].try_into().unwrap()) as usize;
    cursor += 4;
    let mut out = Vec::with_capacity(count);
    for _ in 0..count {
        let device_id = decode_string_field(bytes, &mut cursor)?;
        if bytes.len() < cursor + 6 {
            return Err(CapabilityError::InvalidRequest(
                "remote bluetooth connection header too short",
            ));
        }
        let transport_kind = bluetooth_transport_kind_from_u8(bytes[cursor])?;
        cursor += 1;
        let address_kind = bluetooth_address_kind_from_u8(bytes[cursor])?;
        cursor += 1;
        let link_kind = bluetooth_link_kind_from_u8(bytes[cursor])?;
        cursor += 1;
        let outbound = bytes[cursor] != 0;
        cursor += 1;
        let state = u16::from_le_bytes(bytes[cursor..cursor + 2].try_into().unwrap());
        cursor += 2;
        let local_name = decode_optional_string_field(bytes, &mut cursor)?;
        let service_uuids = decode_string_vec(bytes, &mut cursor)?;
        let profiles = decode_profiles_vec(bytes, &mut cursor)?;
        if bytes.len() < cursor + 2 {
            return Err(CapabilityError::InvalidRequest(
                "remote bluetooth connection trust flags missing",
            ));
        }
        let trusted = match bytes[cursor] {
            0 => None,
            1 => Some(true),
            2 => Some(false),
            _ => {
                return Err(CapabilityError::InvalidRequest(
                    "remote bluetooth trusted flag is invalid",
                ))
            }
        };
        cursor += 1;
        let paired = match bytes[cursor] {
            0 => None,
            1 => Some(true),
            2 => Some(false),
            _ => {
                return Err(CapabilityError::InvalidRequest(
                    "remote bluetooth paired flag is invalid",
                ))
            }
        };
        cursor += 1;
        out.push(BluetoothConnectionInfo {
            device_id,
            transport_kind,
            address_kind,
            link_kind,
            outbound,
            state,
            local_name,
            service_uuids,
            profiles,
            trusted,
            paired,
        });
    }
    if cursor != bytes.len() {
        return Err(CapabilityError::InvalidRequest(
            "remote bluetooth connections payload trailing bytes are invalid",
        ));
    }
    Ok(out)
}

fn wifi_power_state_to_u8(state: WifiPowerState) -> u8 {
    match state {
        WifiPowerState::Unknown => 0,
        WifiPowerState::Enabled => 1,
        WifiPowerState::Disabled => 2,
        WifiPowerState::Blocked => 3,
    }
}

fn wifi_power_state_from_u8(v: u8) -> Result<WifiPowerState, CapabilityError> {
    Ok(match v {
        0 => WifiPowerState::Unknown,
        1 => WifiPowerState::Enabled,
        2 => WifiPowerState::Disabled,
        3 => WifiPowerState::Blocked,
        _ => {
            return Err(CapabilityError::InvalidRequest(
                "remote wifi power state is invalid",
            ))
        }
    })
}

fn wifi_interface_mode_to_u8(mode: WifiInterfaceMode) -> u8 {
    match mode {
        WifiInterfaceMode::Unknown => 0,
        WifiInterfaceMode::Client => 1,
        WifiInterfaceMode::AccessPoint => 2,
        WifiInterfaceMode::AdHoc => 3,
        WifiInterfaceMode::Monitor => 4,
    }
}

fn wifi_interface_mode_from_u8(v: u8) -> Result<WifiInterfaceMode, CapabilityError> {
    Ok(match v {
        0 => WifiInterfaceMode::Unknown,
        1 => WifiInterfaceMode::Client,
        2 => WifiInterfaceMode::AccessPoint,
        3 => WifiInterfaceMode::AdHoc,
        4 => WifiInterfaceMode::Monitor,
        _ => {
            return Err(CapabilityError::InvalidRequest(
                "remote wifi interface mode is invalid",
            ))
        }
    })
}

pub fn encode_wifi_scan_result(scan: &WifiScanResult) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&(scan.observations.len() as u32).to_le_bytes());
    for observation in &scan.observations {
        encode_string_field(&observation.interface_name, &mut out);
        encode_optional_string_field(&observation.ssid, &mut out);
        encode_optional_string_field(&observation.bssid, &mut out);
        out.push(observation.signal_dbm.is_some() as u8);
        if let Some(v) = observation.signal_dbm {
            out.extend_from_slice(&v.to_le_bytes());
        }
        out.push(observation.frequency_mhz.is_some() as u8);
        if let Some(v) = observation.frequency_mhz {
            out.extend_from_slice(&v.to_le_bytes());
        }
        out.push(match observation.secure {
            None => 0,
            Some(true) => 1,
            Some(false) => 2,
        });
        out.extend_from_slice(&observation.observed_at_unix_ms.to_le_bytes());
    }
    out
}

pub fn decode_wifi_scan_result(bytes: &[u8]) -> Result<WifiScanResult, CapabilityError> {
    if bytes.len() < 4 {
        return Err(CapabilityError::InvalidRequest(
            "remote wifi scan payload too short",
        ));
    }
    let mut cursor = 0usize;
    let count = u32::from_le_bytes(bytes[cursor..cursor + 4].try_into().unwrap()) as usize;
    cursor += 4;
    let mut observations = Vec::with_capacity(count);
    for _ in 0..count {
        let interface_name = decode_string_field(bytes, &mut cursor)?;
        let ssid = decode_optional_string_field(bytes, &mut cursor)?;
        let bssid = decode_optional_string_field(bytes, &mut cursor)?;
        if bytes.len() < cursor + 1 {
            return Err(CapabilityError::InvalidRequest(
                "remote wifi signal presence byte missing",
            ));
        }
        let signal_dbm = if bytes[cursor] != 0 {
            cursor += 1;
            if bytes.len() < cursor + 2 {
                return Err(CapabilityError::InvalidRequest(
                    "remote wifi signal payload too short",
                ));
            }
            let v = i16::from_le_bytes(bytes[cursor..cursor + 2].try_into().unwrap());
            cursor += 2;
            Some(v)
        } else {
            cursor += 1;
            None
        };
        if bytes.len() < cursor + 1 {
            return Err(CapabilityError::InvalidRequest(
                "remote wifi frequency presence byte missing",
            ));
        }
        let frequency_mhz = if bytes[cursor] != 0 {
            cursor += 1;
            if bytes.len() < cursor + 4 {
                return Err(CapabilityError::InvalidRequest(
                    "remote wifi frequency payload too short",
                ));
            }
            let v = u32::from_le_bytes(bytes[cursor..cursor + 4].try_into().unwrap());
            cursor += 4;
            Some(v)
        } else {
            cursor += 1;
            None
        };
        if bytes.len() < cursor + 1 + 8 {
            return Err(CapabilityError::InvalidRequest(
                "remote wifi observation trailer too short",
            ));
        }
        let secure = match bytes[cursor] {
            0 => None,
            1 => Some(true),
            2 => Some(false),
            _ => {
                return Err(CapabilityError::InvalidRequest(
                    "remote wifi secure flag is invalid",
                ))
            }
        };
        cursor += 1;
        let observed_at_unix_ms = i64::from_le_bytes(bytes[cursor..cursor + 8].try_into().unwrap());
        cursor += 8;
        observations.push(WifiNetworkObservation {
            interface_name,
            ssid,
            bssid,
            signal_dbm,
            frequency_mhz,
            secure,
            observed_at_unix_ms,
        });
    }
    if cursor != bytes.len() {
        return Err(CapabilityError::InvalidRequest(
            "remote wifi scan payload trailing bytes are invalid",
        ));
    }
    Ok(WifiScanResult { observations })
}

pub fn encode_wifi_interface_info(info: &WifiInterfaceInfo) -> Vec<u8> {
    let mut out = Vec::new();
    encode_string_field(&info.provider, &mut out);
    encode_string_field(&info.interface_name, &mut out);
    encode_optional_string_field(&info.mac_address, &mut out);
    encode_optional_string_field(&info.phy_name, &mut out);
    encode_optional_string_field(&info.operstate, &mut out);
    out.push(wifi_power_state_to_u8(info.power_state));
    out.push(wifi_interface_mode_to_u8(info.mode));
    out
}

pub fn decode_wifi_interface_info(bytes: &[u8]) -> Result<WifiInterfaceInfo, CapabilityError> {
    let mut cursor = 0usize;
    let provider = decode_string_field(bytes, &mut cursor)?;
    let interface_name = decode_string_field(bytes, &mut cursor)?;
    let mac_address = decode_optional_string_field(bytes, &mut cursor)?;
    let phy_name = decode_optional_string_field(bytes, &mut cursor)?;
    let operstate = decode_optional_string_field(bytes, &mut cursor)?;
    if bytes.len() != cursor + 2 {
        return Err(CapabilityError::InvalidRequest(
            "remote wifi interface info payload length is invalid",
        ));
    }
    let power_state = wifi_power_state_from_u8(bytes[cursor])?;
    let mode = wifi_interface_mode_from_u8(bytes[cursor + 1])?;
    Ok(WifiInterfaceInfo {
        provider,
        interface_name,
        mac_address,
        phy_name,
        operstate,
        power_state,
        mode,
    })
}

#[derive(Debug)]
pub struct BluetoothRemoteAdapter<D> {
    pub device: D,
    descriptor: CapabilityDescriptor,
    next_sequence_no: u64,
}

impl<D> BluetoothRemoteAdapter<D> {
    pub fn new(device: D, descriptor: CapabilityDescriptor) -> Self {
        Self {
            device,
            descriptor,
            next_sequence_no: 1,
        }
    }
}

#[derive(Debug)]
pub struct BluetoothConnectionRemoteAdapter<D> {
    pub device: D,
    descriptor: CapabilityDescriptor,
}

impl<D> BluetoothConnectionRemoteAdapter<D> {
    pub fn new(device: D, descriptor: CapabilityDescriptor) -> Self {
        Self { device, descriptor }
    }
}

#[derive(Debug)]
pub struct WifiRemoteAdapter<D> {
    pub device: D,
    descriptor: CapabilityDescriptor,
    next_sequence_no: u64,
}

impl<D> WifiRemoteAdapter<D> {
    pub fn new(device: D, descriptor: CapabilityDescriptor) -> Self {
        Self {
            device,
            descriptor,
            next_sequence_no: 1,
        }
    }
}

#[derive(Debug)]
pub struct WifiControlRemoteAdapter<D> {
    pub device: D,
    descriptor: CapabilityDescriptor,
}

impl<D> WifiControlRemoteAdapter<D> {
    pub fn new(device: D, descriptor: CapabilityDescriptor) -> Self {
        Self { device, descriptor }
    }
}

fn map_camera_error(err: CameraBiometricError) -> CapabilityError {
    match err {
        CameraBiometricError::Provider(msg) => CapabilityError::Provider(msg),
        CameraBiometricError::InvalidRequest(msg) => CapabilityError::InvalidRequest(msg),
        CameraBiometricError::InvalidState(msg) => CapabilityError::Provider(msg.into()),
        CameraBiometricError::UnsupportedOperation(msg) => CapabilityError::Unsupported(msg),
    }
}

fn camera_pixel_format_to_u32(format: CameraPixelFormat) -> u32 {
    match format {
        CameraPixelFormat::Mjpeg => 1,
        CameraPixelFormat::Yuyv => 2,
        CameraPixelFormat::Nv12 => 3,
        CameraPixelFormat::Rgb24 => 4,
        CameraPixelFormat::Gray8 => 5,
        CameraPixelFormat::Other(v) => v | 0x8000_0000,
    }
}

fn camera_pixel_format_from_u32(raw: u32) -> Result<CameraPixelFormat, CapabilityError> {
    Ok(match raw {
        1 => CameraPixelFormat::Mjpeg,
        2 => CameraPixelFormat::Yuyv,
        3 => CameraPixelFormat::Nv12,
        4 => CameraPixelFormat::Rgb24,
        5 => CameraPixelFormat::Gray8,
        other if other & 0x8000_0000 != 0 => CameraPixelFormat::Other(other & 0x7fff_ffff),
        _ => {
            return Err(CapabilityError::InvalidRequest(
                "remote camera pixel format is unknown",
            ))
        }
    })
}

fn encode_camera_frame(frame: &CameraFrame) -> Vec<u8> {
    let mut out = Vec::with_capacity(20 + 4 + frame.bytes.len());
    out.extend_from_slice(&frame.width.to_le_bytes());
    out.extend_from_slice(&frame.height.to_le_bytes());
    out.extend_from_slice(&frame.stride.to_le_bytes());
    out.extend_from_slice(&camera_pixel_format_to_u32(frame.format).to_le_bytes());
    out.extend_from_slice(&(frame.bytes.len() as u32).to_le_bytes());
    out.extend_from_slice(&frame.bytes);
    out
}

fn decode_camera_frame(bytes: &[u8]) -> Result<(CameraFrame, usize), CapabilityError> {
    if bytes.len() < 20 {
        return Err(CapabilityError::InvalidRequest(
            "remote camera frame payload too short",
        ));
    }
    let width = u32::from_le_bytes(bytes[0..4].try_into().unwrap());
    let height = u32::from_le_bytes(bytes[4..8].try_into().unwrap());
    let stride = u32::from_le_bytes(bytes[8..12].try_into().unwrap());
    let format =
        camera_pixel_format_from_u32(u32::from_le_bytes(bytes[12..16].try_into().unwrap()))?;
    let len = u32::from_le_bytes(bytes[16..20].try_into().unwrap()) as usize;
    if bytes.len() < 20 + len {
        return Err(CapabilityError::InvalidRequest(
            "remote camera frame byte count exceeds payload length",
        ));
    }
    Ok((
        CameraFrame {
            width,
            height,
            stride,
            format,
            bytes: bytes[20..20 + len].to_vec(),
        },
        20 + len,
    ))
}

fn camera_capture_quality_to_u8(q: CameraCaptureQuality) -> u8 {
    match q {
        CameraCaptureQuality::Poor => 1,
        CameraCaptureQuality::Fair => 2,
        CameraCaptureQuality::Good => 3,
        CameraCaptureQuality::Excellent => 4,
    }
}

fn camera_capture_quality_from_u8(v: u8) -> Result<CameraCaptureQuality, CapabilityError> {
    Ok(match v {
        1 => CameraCaptureQuality::Poor,
        2 => CameraCaptureQuality::Fair,
        3 => CameraCaptureQuality::Good,
        4 => CameraCaptureQuality::Excellent,
        _ => {
            return Err(CapabilityError::InvalidRequest(
                "remote camera capture quality is invalid",
            ))
        }
    })
}

pub fn encode_camera_capture(capture: &CameraCapture) -> Vec<u8> {
    let frame = encode_camera_frame(&capture.frame);
    let mut out = Vec::with_capacity(frame.len() + 8 + 20);
    out.extend_from_slice(&frame);
    out.push(camera_capture_quality_to_u8(capture.quality));
    out.push(capture.face_bounds.is_some() as u8);
    if let Some(bounds) = &capture.face_bounds {
        out.extend_from_slice(&bounds.x.to_le_bytes());
        out.extend_from_slice(&bounds.y.to_le_bytes());
        out.extend_from_slice(&bounds.width.to_le_bytes());
        out.extend_from_slice(&bounds.height.to_le_bytes());
    }
    out.push(capture.state.verified as u8);
    out.push(capture.state.hardware_protected as u8);
    out.push(capture.state.user_present as u8);
    out.push(match capture.state.modality {
        Some(edgerun_biometrics::BiometricModality::Fingerprint) => 1,
        Some(edgerun_biometrics::BiometricModality::Face) => 2,
        Some(edgerun_biometrics::BiometricModality::Voice) => 3,
        Some(edgerun_biometrics::BiometricModality::Iris) => 4,
        Some(edgerun_biometrics::BiometricModality::Palm) => 5,
        Some(edgerun_biometrics::BiometricModality::Other(_)) => 255,
        None => 0,
    });
    out
}

pub fn decode_camera_capture(bytes: &[u8]) -> Result<CameraCapture, CapabilityError> {
    let (frame, offset) = decode_camera_frame(bytes)?;
    if bytes.len() < offset + 5 {
        return Err(CapabilityError::InvalidRequest(
            "remote camera capture payload too short",
        ));
    }
    let quality = camera_capture_quality_from_u8(bytes[offset])?;
    let has_bounds = bytes[offset + 1] != 0;
    let mut cursor = offset + 2;
    let face_bounds = if has_bounds {
        if bytes.len() < cursor + 16 + 4 {
            return Err(CapabilityError::InvalidRequest(
                "remote camera face bounds payload too short",
            ));
        }
        let x = u32::from_le_bytes(bytes[cursor..cursor + 4].try_into().unwrap());
        cursor += 4;
        let y = u32::from_le_bytes(bytes[cursor..cursor + 4].try_into().unwrap());
        cursor += 4;
        let width = u32::from_le_bytes(bytes[cursor..cursor + 4].try_into().unwrap());
        cursor += 4;
        let height = u32::from_le_bytes(bytes[cursor..cursor + 4].try_into().unwrap());
        cursor += 4;
        Some(edgerun_camera_biometrics::FaceBounds {
            x,
            y,
            width,
            height,
        })
    } else {
        None
    };
    if bytes.len() != cursor + 4 {
        return Err(CapabilityError::InvalidRequest(
            "remote camera capture payload length is invalid",
        ));
    }
    let modality = match bytes[cursor + 3] {
        0 => None,
        1 => Some(edgerun_biometrics::BiometricModality::Fingerprint),
        2 => Some(edgerun_biometrics::BiometricModality::Face),
        3 => Some(edgerun_biometrics::BiometricModality::Voice),
        4 => Some(edgerun_biometrics::BiometricModality::Iris),
        5 => Some(edgerun_biometrics::BiometricModality::Palm),
        _ => None,
    };
    Ok(CameraCapture {
        frame,
        quality,
        face_bounds,
        state: edgerun_biometrics::BiometricState {
            modality,
            verified: bytes[cursor] != 0,
            hardware_protected: bytes[cursor + 1] != 0,
            user_present: bytes[cursor + 2] != 0,
        },
    })
}

fn camera_stream_role_to_u8(role: CameraStreamRole) -> u8 {
    match role {
        CameraStreamRole::Unknown => 0,
        CameraStreamRole::Rgb => 1,
        CameraStreamRole::Infrared => 2,
        CameraStreamRole::Depth => 3,
        CameraStreamRole::Monochrome => 4,
    }
}

pub fn encode_paired_camera_frame(frame: &PairedCameraFrame) -> Vec<u8> {
    let mut out = Vec::new();
    for (role, maybe_frame) in [
        (CameraStreamRole::Rgb, frame.rgb.as_ref()),
        (CameraStreamRole::Infrared, frame.infrared.as_ref()),
        (CameraStreamRole::Depth, frame.depth.as_ref()),
    ] {
        out.push(maybe_frame.is_some() as u8);
        if let Some(inner) = maybe_frame {
            out.push(camera_stream_role_to_u8(role));
            out.extend_from_slice(&encode_camera_frame(inner));
        }
    }
    out
}

pub fn decode_paired_camera_frame(bytes: &[u8]) -> Result<PairedCameraFrame, CapabilityError> {
    let mut cursor = 0usize;
    let mut rgb = None;
    let mut infrared = None;
    let mut depth = None;
    for slot in 0..3 {
        if bytes.len() < cursor + 1 {
            return Err(CapabilityError::InvalidRequest(
                "remote paired camera payload too short",
            ));
        }
        let present = bytes[cursor] != 0;
        cursor += 1;
        if present {
            if bytes.len() < cursor + 1 {
                return Err(CapabilityError::InvalidRequest(
                    "remote paired camera role missing",
                ));
            }
            let role = bytes[cursor];
            cursor += 1;
            let (frame, used) = decode_camera_frame(&bytes[cursor..])?;
            cursor += used;
            match role {
                1 => rgb = Some(frame),
                2 => infrared = Some(frame),
                3 => depth = Some(frame),
                4 if slot == 1 => infrared = Some(frame),
                _ => {}
            }
        }
    }
    if cursor != bytes.len() {
        return Err(CapabilityError::InvalidRequest(
            "remote paired camera payload trailing bytes are invalid",
        ));
    }
    Ok(PairedCameraFrame {
        rgb,
        infrared,
        depth,
    })
}

#[derive(Debug)]
pub struct CameraRemoteAdapter<D> {
    pub device: D,
    pub purpose: CameraBiometricPurpose,
    pub timeout_ms: u32,
    descriptor: CapabilityDescriptor,
    next_sequence_no: u64,
}

impl<D> CameraRemoteAdapter<D> {
    pub fn new(
        device: D,
        purpose: CameraBiometricPurpose,
        timeout_ms: u32,
        descriptor: CapabilityDescriptor,
    ) -> Self {
        Self {
            device,
            purpose,
            timeout_ms,
            descriptor,
            next_sequence_no: 1,
        }
    }
}

#[derive(Debug)]
pub struct PairedCameraRemoteAdapter<D> {
    pub device: D,
    pub purpose: CameraBiometricPurpose,
    pub timeout_ms: u32,
    descriptor: CapabilityDescriptor,
    next_sequence_no: u64,
}

impl<D> PairedCameraRemoteAdapter<D> {
    pub fn new(
        device: D,
        purpose: CameraBiometricPurpose,
        timeout_ms: u32,
        descriptor: CapabilityDescriptor,
    ) -> Self {
        Self {
            device,
            purpose,
            timeout_ms,
            descriptor,
            next_sequence_no: 1,
        }
    }
}

#[derive(Debug)]
pub struct SpeakerRemoteAdapter<D> {
    pub device: D,
}

impl<D> SpeakerRemoteAdapter<D> {
    pub fn new(device: D) -> Self {
        Self { device }
    }
}

#[derive(Debug)]
pub struct InputRemoteAdapter<D> {
    pub device: D,
    pub max_events_per_poll: usize,
    next_sequence_no: u64,
}

impl<D> InputRemoteAdapter<D> {
    pub fn new(device: D, max_events_per_poll: usize) -> Self {
        Self {
            device,
            max_events_per_poll,
            next_sequence_no: 1,
        }
    }
}

#[derive(Debug)]
pub struct MicrophoneRemoteAdapter<D> {
    pub device: D,
    pub capture_request: AudioCaptureRequest,
    next_sequence_no: u64,
}

impl<D> MicrophoneRemoteAdapter<D> {
    pub fn new(device: D, capture_request: AudioCaptureRequest) -> Self {
        Self {
            device,
            capture_request,
            next_sequence_no: 1,
        }
    }
}

impl<D> RemoteCapabilityProvider for InputRemoteAdapter<D>
where
    D: InputDevice,
{
    fn descriptor(&self) -> CapabilityDescriptor {
        self.device.descriptor()
    }

    fn open_session(
        &mut self,
        open: &CapabilitySessionOpen,
    ) -> Result<CapabilitySessionAccept, CapabilityError> {
        Ok(accept_session_open_unchecked(open))
    }

    fn invoke(
        &mut self,
        _session_id: &[u8],
        invocation: &CapabilityInvocation,
        _inline_parameters: Option<&[u8]>,
    ) -> Result<RemoteInvocationResult, CapabilityError> {
        Ok(RemoteInvocationResult {
            result: CapabilityResult {
                result_version: 1,
                invocation_id: invocation.invocation_id.clone(),
                grant_id: invocation.grant_id.clone(),
                success: false,
                result_access_class: invocation.requested_access_class,
                produced_event_kinds: Vec::new(),
                payload_object: None,
                error_reason: "input remote adapter is stream-oriented; use session events".into(),
                produced_at: None,
                signature: None,
            },
            inline_payload: Vec::new(),
        })
    }

    fn next_event(
        &mut self,
        session_id: &[u8],
    ) -> Result<Option<CapabilitySessionEvent>, CapabilityError> {
        let events = self.device.read_events(self.max_events_per_poll)?;
        if events.is_empty() {
            return Ok(None);
        }
        let sequence_no = self.next_sequence_no;
        self.next_sequence_no += 1;
        Ok(Some(CapabilitySessionEvent {
            version: 1,
            session_id: session_id.to_vec(),
            sequence_no,
            event_kinds: vec![
                edgerun_capabilities::CapabilityEventKind::Touch as i32,
                edgerun_capabilities::CapabilityEventKind::Text as i32,
                edgerun_capabilities::CapabilityEventKind::State as i32,
            ],
            payload_object: None,
            inline_payload: encode_input_events(&events),
        }))
    }
}

impl<D> RemoteCapabilityProvider for MicrophoneRemoteAdapter<D>
where
    D: MicrophoneDevice,
{
    fn descriptor(&self) -> CapabilityDescriptor {
        self.device.descriptor()
    }

    fn open_session(
        &mut self,
        open: &CapabilitySessionOpen,
    ) -> Result<CapabilitySessionAccept, CapabilityError> {
        Ok(accept_session_open_unchecked(open))
    }

    fn invoke(
        &mut self,
        _session_id: &[u8],
        invocation: &CapabilityInvocation,
        _inline_parameters: Option<&[u8]>,
    ) -> Result<RemoteInvocationResult, CapabilityError> {
        Ok(RemoteInvocationResult {
            result: CapabilityResult {
                result_version: 1,
                invocation_id: invocation.invocation_id.clone(),
                grant_id: invocation.grant_id.clone(),
                success: false,
                result_access_class: invocation.requested_access_class,
                produced_event_kinds: Vec::new(),
                payload_object: None,
                error_reason: "microphone remote adapter is stream-oriented; use session events"
                    .into(),
                produced_at: None,
                signature: None,
            },
            inline_payload: Vec::new(),
        })
    }

    fn next_event(
        &mut self,
        session_id: &[u8],
    ) -> Result<Option<CapabilitySessionEvent>, CapabilityError> {
        let capture = self.device.capture_audio(&self.capture_request)?;
        let sequence_no = self.next_sequence_no;
        self.next_sequence_no += 1;
        Ok(Some(CapabilitySessionEvent {
            version: 1,
            session_id: session_id.to_vec(),
            sequence_no,
            event_kinds: vec![edgerun_capabilities::CapabilityEventKind::Auditory as i32],
            payload_object: None,
            inline_payload: encode_microphone_capture(&capture),
        }))
    }
}

impl<D> RemoteCapabilityProvider for SpeakerRemoteAdapter<D>
where
    D: SpeakerDevice,
{
    fn descriptor(&self) -> CapabilityDescriptor {
        self.device.descriptor()
    }

    fn open_session(
        &mut self,
        open: &CapabilitySessionOpen,
    ) -> Result<CapabilitySessionAccept, CapabilityError> {
        Ok(accept_session_open_unchecked(open))
    }

    fn invoke(
        &mut self,
        _session_id: &[u8],
        invocation: &CapabilityInvocation,
        inline_parameters: Option<&[u8]>,
    ) -> Result<RemoteInvocationResult, CapabilityError> {
        let parameters = inline_parameters.ok_or(CapabilityError::InvalidRequest(
            "speaker remote adapter requires inline playback parameters",
        ))?;
        let request = decode_speaker_playback_request(parameters)?;
        let output_level = if invocation.operation
            == edgerun_capabilities::CapabilityOperation::Control as i32
        {
            let target =
                request
                    .target_output_level_percent
                    .ok_or(CapabilityError::InvalidRequest(
                        "speaker control invocation requires target output level percent",
                    ))?;
            self.device.set_output_level(target)?
        } else {
            None
        };
        let playback =
            if invocation.operation == edgerun_capabilities::CapabilityOperation::Render as i32 {
                Some(self.device.play_audio(&request)?)
            } else {
                None
            };
        let inline_payload = if let Some(playback) = playback {
            encode_speaker_playback_result(&playback)
        } else if let Some(level) = output_level {
            encode_speaker_output_level(&level)
        } else {
            Vec::new()
        };
        Ok(RemoteInvocationResult {
            result: CapabilityResult {
                result_version: 1,
                invocation_id: invocation.invocation_id.clone(),
                grant_id: invocation.grant_id.clone(),
                success: true,
                result_access_class: invocation.requested_access_class,
                produced_event_kinds: vec![
                    edgerun_capabilities::CapabilityEventKind::Auditory as i32,
                ],
                payload_object: None,
                error_reason: String::new(),
                produced_at: None,
                signature: None,
            },
            inline_payload,
        })
    }
}

impl<D> RemoteCapabilityProvider for BluetoothRemoteAdapter<D>
where
    D: BluetoothScanner,
{
    fn descriptor(&self) -> CapabilityDescriptor {
        self.descriptor.clone()
    }

    fn open_session(
        &mut self,
        open: &CapabilitySessionOpen,
    ) -> Result<CapabilitySessionAccept, CapabilityError> {
        Ok(accept_session_open_unchecked(open))
    }

    fn invoke(
        &mut self,
        _session_id: &[u8],
        invocation: &CapabilityInvocation,
        _inline_parameters: Option<&[u8]>,
    ) -> Result<RemoteInvocationResult, CapabilityError> {
        Ok(RemoteInvocationResult {
            result: CapabilityResult {
                result_version: 1,
                invocation_id: invocation.invocation_id.clone(),
                grant_id: invocation.grant_id.clone(),
                success: false,
                result_access_class: invocation.requested_access_class,
                produced_event_kinds: Vec::new(),
                payload_object: None,
                error_reason: "bluetooth remote adapter is stream-oriented; use session events"
                    .into(),
                produced_at: None,
                signature: None,
            },
            inline_payload: Vec::new(),
        })
    }

    fn next_event(
        &mut self,
        session_id: &[u8],
    ) -> Result<Option<CapabilitySessionEvent>, CapabilityError> {
        let scan = self.device.scan_nearby()?;
        let sequence_no = self.next_sequence_no;
        self.next_sequence_no += 1;
        Ok(Some(CapabilitySessionEvent {
            version: 1,
            session_id: session_id.to_vec(),
            sequence_no,
            event_kinds: vec![edgerun_capabilities::CapabilityEventKind::Radio as i32],
            payload_object: None,
            inline_payload: encode_bluetooth_scan_result(&scan),
        }))
    }
}

impl<D> RemoteCapabilityProvider for BluetoothConnectionRemoteAdapter<D>
where
    D: BluetoothConnectionProvider,
{
    fn descriptor(&self) -> CapabilityDescriptor {
        self.descriptor.clone()
    }

    fn open_session(
        &mut self,
        open: &CapabilitySessionOpen,
    ) -> Result<CapabilitySessionAccept, CapabilityError> {
        Ok(accept_session_open_unchecked(open))
    }

    fn invoke(
        &mut self,
        _session_id: &[u8],
        invocation: &CapabilityInvocation,
        _inline_parameters: Option<&[u8]>,
    ) -> Result<RemoteInvocationResult, CapabilityError> {
        let connections = self.device.list_connections()?;
        Ok(RemoteInvocationResult {
            result: CapabilityResult {
                result_version: 1,
                invocation_id: invocation.invocation_id.clone(),
                grant_id: invocation.grant_id.clone(),
                success: true,
                result_access_class: invocation.requested_access_class,
                produced_event_kinds: vec![
                    edgerun_capabilities::CapabilityEventKind::Radio as i32,
                ],
                payload_object: None,
                error_reason: String::new(),
                produced_at: None,
                signature: None,
            },
            inline_payload: encode_bluetooth_connections(&connections),
        })
    }
}

impl<D> RemoteCapabilityProvider for WifiRemoteAdapter<D>
where
    D: WifiScanner,
{
    fn descriptor(&self) -> CapabilityDescriptor {
        self.descriptor.clone()
    }

    fn open_session(
        &mut self,
        open: &CapabilitySessionOpen,
    ) -> Result<CapabilitySessionAccept, CapabilityError> {
        Ok(accept_session_open_unchecked(open))
    }

    fn invoke(
        &mut self,
        _session_id: &[u8],
        invocation: &CapabilityInvocation,
        _inline_parameters: Option<&[u8]>,
    ) -> Result<RemoteInvocationResult, CapabilityError> {
        Ok(RemoteInvocationResult {
            result: CapabilityResult {
                result_version: 1,
                invocation_id: invocation.invocation_id.clone(),
                grant_id: invocation.grant_id.clone(),
                success: false,
                result_access_class: invocation.requested_access_class,
                produced_event_kinds: Vec::new(),
                payload_object: None,
                error_reason: "wifi remote adapter is stream-oriented; use session events".into(),
                produced_at: None,
                signature: None,
            },
            inline_payload: Vec::new(),
        })
    }

    fn next_event(
        &mut self,
        session_id: &[u8],
    ) -> Result<Option<CapabilitySessionEvent>, CapabilityError> {
        let scan = self.device.scan_nearby()?;
        let sequence_no = self.next_sequence_no;
        self.next_sequence_no += 1;
        Ok(Some(CapabilitySessionEvent {
            version: 1,
            session_id: session_id.to_vec(),
            sequence_no,
            event_kinds: vec![edgerun_capabilities::CapabilityEventKind::Radio as i32],
            payload_object: None,
            inline_payload: encode_wifi_scan_result(&scan),
        }))
    }
}

impl<D> RemoteCapabilityProvider for WifiControlRemoteAdapter<D>
where
    D: WifiController,
{
    fn descriptor(&self) -> CapabilityDescriptor {
        self.descriptor.clone()
    }

    fn open_session(
        &mut self,
        open: &CapabilitySessionOpen,
    ) -> Result<CapabilitySessionAccept, CapabilityError> {
        Ok(accept_session_open_unchecked(open))
    }

    fn invoke(
        &mut self,
        _session_id: &[u8],
        invocation: &CapabilityInvocation,
        inline_parameters: Option<&[u8]>,
    ) -> Result<RemoteInvocationResult, CapabilityError> {
        let info = if invocation.operation
            == edgerun_capabilities::CapabilityOperation::Control as i32
        {
            let parameters = inline_parameters.ok_or(CapabilityError::InvalidRequest(
                "wifi control invocation requires inline power state parameter",
            ))?;
            if parameters.len() != 1 {
                return Err(CapabilityError::InvalidRequest(
                    "wifi control invocation power state payload length is invalid",
                ));
            }
            let state = wifi_power_state_from_u8(parameters[0])?;
            self.device.set_power_state(state)?;
            self.device.interface_info()?
        } else {
            self.device.interface_info()?
        };
        Ok(RemoteInvocationResult {
            result: CapabilityResult {
                result_version: 1,
                invocation_id: invocation.invocation_id.clone(),
                grant_id: invocation.grant_id.clone(),
                success: true,
                result_access_class: invocation.requested_access_class,
                produced_event_kinds: vec![
                    edgerun_capabilities::CapabilityEventKind::State as i32,
                ],
                payload_object: None,
                error_reason: String::new(),
                produced_at: None,
                signature: None,
            },
            inline_payload: encode_wifi_interface_info(&info),
        })
    }
}

impl<D> RemoteCapabilityProvider for CameraRemoteAdapter<D>
where
    D: CameraBiometricReader,
{
    fn descriptor(&self) -> CapabilityDescriptor {
        self.descriptor.clone()
    }

    fn open_session(
        &mut self,
        open: &CapabilitySessionOpen,
    ) -> Result<CapabilitySessionAccept, CapabilityError> {
        Ok(accept_session_open_unchecked(open))
    }

    fn invoke(
        &mut self,
        _session_id: &[u8],
        invocation: &CapabilityInvocation,
        _inline_parameters: Option<&[u8]>,
    ) -> Result<RemoteInvocationResult, CapabilityError> {
        Ok(RemoteInvocationResult {
            result: CapabilityResult {
                result_version: 1,
                invocation_id: invocation.invocation_id.clone(),
                grant_id: invocation.grant_id.clone(),
                success: false,
                result_access_class: invocation.requested_access_class,
                produced_event_kinds: Vec::new(),
                payload_object: None,
                error_reason: "camera remote adapter is stream-oriented; use session events".into(),
                produced_at: None,
                signature: None,
            },
            inline_payload: Vec::new(),
        })
    }

    fn next_event(
        &mut self,
        session_id: &[u8],
    ) -> Result<Option<CapabilitySessionEvent>, CapabilityError> {
        let capture = self
            .device
            .capture(self.purpose, self.timeout_ms)
            .map_err(map_camera_error)?;
        let sequence_no = self.next_sequence_no;
        self.next_sequence_no += 1;
        Ok(Some(CapabilitySessionEvent {
            version: 1,
            session_id: session_id.to_vec(),
            sequence_no,
            event_kinds: vec![edgerun_capabilities::CapabilityEventKind::Visual as i32],
            payload_object: None,
            inline_payload: encode_camera_capture(&capture),
        }))
    }
}

impl<D> RemoteCapabilityProvider for PairedCameraRemoteAdapter<D>
where
    D: PairedCameraBiometricReader,
{
    fn descriptor(&self) -> CapabilityDescriptor {
        self.descriptor.clone()
    }

    fn open_session(
        &mut self,
        open: &CapabilitySessionOpen,
    ) -> Result<CapabilitySessionAccept, CapabilityError> {
        Ok(accept_session_open_unchecked(open))
    }

    fn invoke(
        &mut self,
        _session_id: &[u8],
        invocation: &CapabilityInvocation,
        _inline_parameters: Option<&[u8]>,
    ) -> Result<RemoteInvocationResult, CapabilityError> {
        Ok(RemoteInvocationResult {
            result: CapabilityResult {
                result_version: 1,
                invocation_id: invocation.invocation_id.clone(),
                grant_id: invocation.grant_id.clone(),
                success: false,
                result_access_class: invocation.requested_access_class,
                produced_event_kinds: Vec::new(),
                payload_object: None,
                error_reason: "paired camera remote adapter is stream-oriented; use session events"
                    .into(),
                produced_at: None,
                signature: None,
            },
            inline_payload: Vec::new(),
        })
    }

    fn next_event(
        &mut self,
        session_id: &[u8],
    ) -> Result<Option<CapabilitySessionEvent>, CapabilityError> {
        let capture = self
            .device
            .capture_paired(self.purpose, self.timeout_ms)
            .map_err(map_camera_error)?;
        let sequence_no = self.next_sequence_no;
        self.next_sequence_no += 1;
        Ok(Some(CapabilitySessionEvent {
            version: 1,
            session_id: session_id.to_vec(),
            sequence_no,
            event_kinds: vec![edgerun_capabilities::CapabilityEventKind::Visual as i32],
            payload_object: None,
            inline_payload: encode_paired_camera_frame(&capture),
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use edgerun_bluetooth::{default_bluetooth_descriptor, BluetoothScanResult};
    use edgerun_camera_biometrics::{
        default_face_biometric_state, CameraBiometricPurpose, CameraReaderInfo,
    };
    use edgerun_capabilities::{
        capability_descriptor, CapabilityAccessClass, CapabilityEventKind, CapabilityModality,
        CapabilityOperation, CapabilityRole,
    };
    use edgerun_input::{InputDeviceInfo, InputDeviceKind};
    use edgerun_microphone::MicrophoneInfo;
    use edgerun_proto::edgerun::v0::capability::{CapabilityInvocation, CapabilityResult};
    use edgerun_speaker::{AudioPlaybackRequest, SpeakerInfo, SpeakerOutputLevel};
    use edgerun_wifi::{
        default_wifi_descriptor, WifiInterfaceInfo, WifiInterfaceMode, WifiNetworkObservation,
        WifiPowerState, WifiScanResult,
    };

    #[derive(Default)]
    struct DummyProvider {
        opened: bool,
        event_sent: bool,
    }

    impl RemoteCapabilityProvider for DummyProvider {
        fn descriptor(&self) -> CapabilityDescriptor {
            capability_descriptor(
                "dummy",
                "provider",
                CapabilityRole::Input,
                &[CapabilityModality::Text],
                &[CapabilityEventKind::Text],
                &[CapabilityOperation::Observe],
                Vec::new(),
            )
        }

        fn open_session(
            &mut self,
            open: &CapabilitySessionOpen,
        ) -> Result<CapabilitySessionAccept, CapabilityError> {
            self.opened = true;
            let mut accept = accept_session_open_unchecked(open);
            accept.granted_access_class = CapabilityAccessClass::Raw as i32;
            Ok(accept)
        }

        fn invoke(
            &mut self,
            _session_id: &[u8],
            invocation: &CapabilityInvocation,
            _inline_parameters: Option<&[u8]>,
        ) -> Result<RemoteInvocationResult, CapabilityError> {
            Ok(RemoteInvocationResult {
                result: CapabilityResult {
                    result_version: 1,
                    invocation_id: invocation.invocation_id.clone(),
                    grant_id: invocation.grant_id.clone(),
                    success: true,
                    result_access_class: CapabilityAccessClass::Raw as i32,
                    produced_event_kinds: vec![CapabilityEventKind::Text as i32],
                    payload_object: None,
                    error_reason: String::new(),
                    produced_at: None,
                    signature: None,
                },
                inline_payload: Vec::new(),
            })
        }

        fn next_event(
            &mut self,
            session_id: &[u8],
        ) -> Result<Option<CapabilitySessionEvent>, CapabilityError> {
            if self.event_sent {
                return Ok(None);
            }
            self.event_sent = true;
            Ok(Some(CapabilitySessionEvent {
                version: 1,
                session_id: session_id.to_vec(),
                sequence_no: 1,
                event_kinds: vec![CapabilityEventKind::Text as i32],
                payload_object: None,
                inline_payload: b"hello".to_vec(),
            }))
        }
    }

    #[derive(Default)]
    struct DummyInputDevice {
        events: VecDeque<Vec<InputEventRecord>>,
    }

    impl edgerun_capabilities::CapabilityProvider for DummyInputDevice {
        fn descriptor(&self) -> CapabilityDescriptor {
            capability_descriptor(
                "dummy-input",
                "event0",
                CapabilityRole::Input,
                &[CapabilityModality::Touch, CapabilityModality::Text],
                &[
                    CapabilityEventKind::Touch,
                    CapabilityEventKind::Text,
                    CapabilityEventKind::State,
                ],
                &[CapabilityOperation::Observe],
                Vec::new(),
            )
        }
    }

    #[derive(Default)]
    struct DummyMicrophoneDevice {
        captures: VecDeque<AudioCapture>,
    }

    impl edgerun_capabilities::CapabilityProvider for DummyMicrophoneDevice {
        fn descriptor(&self) -> CapabilityDescriptor {
            capability_descriptor(
                "dummy-mic",
                "hw:0,0",
                CapabilityRole::Input,
                &[CapabilityModality::Auditory],
                &[CapabilityEventKind::Auditory],
                &[CapabilityOperation::Capture],
                Vec::new(),
            )
        }
    }

    impl MicrophoneDevice for DummyMicrophoneDevice {
        fn microphone_info(&self) -> Result<MicrophoneInfo, CapabilityError> {
            Ok(MicrophoneInfo {
                provider: "dummy-mic".into(),
                device_name: "Dummy Mic".into(),
                instance_id: "hw:0,0".into(),
                channels: 2,
                sample_rate_hz: 48_000,
                format: MicrophoneSampleFormat::PcmS16Le,
                hardware_aec: false,
                hardware_noise_suppression: false,
            })
        }

        fn capture_audio(
            &mut self,
            _request: &AudioCaptureRequest,
        ) -> Result<AudioCapture, CapabilityError> {
            self.captures
                .pop_front()
                .ok_or_else(|| CapabilityError::Provider("no dummy capture queued".into()))
        }
    }

    #[derive(Default)]
    struct DummySpeakerDevice {
        result: RefCell<Option<AudioPlaybackResult>>,
        output_level: RefCell<Option<SpeakerOutputLevel>>,
        last_request: RefCell<Option<AudioPlaybackRequest>>,
        last_set_level: RefCell<Option<u8>>,
    }

    impl edgerun_capabilities::CapabilityProvider for DummySpeakerDevice {
        fn descriptor(&self) -> CapabilityDescriptor {
            capability_descriptor(
                "dummy-speaker",
                "hw:0,0",
                CapabilityRole::Output,
                &[CapabilityModality::Auditory],
                &[CapabilityEventKind::Auditory],
                &[CapabilityOperation::Render, CapabilityOperation::Control],
                Vec::new(),
            )
        }
    }

    impl SpeakerDevice for DummySpeakerDevice {
        fn speaker_info(&self) -> Result<SpeakerInfo, CapabilityError> {
            Ok(SpeakerInfo {
                provider: "dummy-speaker".into(),
                instance_id: "hw:0,0".into(),
                display_name: "Dummy Speaker".into(),
                default_sample_rate_hz: 48_000,
                channels: 2,
                supports_playback: true,
                supports_output_level_control: true,
            })
        }

        fn output_level(&self) -> Result<Option<SpeakerOutputLevel>, CapabilityError> {
            Ok(self.output_level.borrow().clone())
        }

        fn set_output_level(
            &self,
            percent: u8,
        ) -> Result<Option<SpeakerOutputLevel>, CapabilityError> {
            *self.last_set_level.borrow_mut() = Some(percent);
            let mut level = self.output_level.borrow_mut();
            if let Some(current) = level.as_mut() {
                current.current_percent = percent;
            } else {
                *level = Some(SpeakerOutputLevel {
                    current_percent: percent,
                    min_raw_value: 0,
                    max_raw_value: 100,
                    muted: Some(false),
                });
            }
            Ok(level.clone())
        }

        fn play_audio(
            &self,
            request: &AudioPlaybackRequest,
        ) -> Result<AudioPlaybackResult, CapabilityError> {
            *self.last_request.borrow_mut() = Some(request.clone());
            self.result.borrow().clone().ok_or_else(|| {
                CapabilityError::Provider("no dummy playback result configured".into())
            })
        }
    }

    #[derive(Default)]
    struct DummyBluetoothScannerDevice {
        scans: RefCell<VecDeque<BluetoothScanResult>>,
    }

    impl edgerun_capabilities::CapabilityProvider for DummyBluetoothScannerDevice {
        fn descriptor(&self) -> CapabilityDescriptor {
            default_bluetooth_descriptor("dummy-bt", "hci0")
        }
    }

    impl BluetoothScanner for DummyBluetoothScannerDevice {
        fn scan_nearby(&self) -> Result<BluetoothScanResult, CapabilityError> {
            self.scans
                .borrow_mut()
                .pop_front()
                .ok_or_else(|| CapabilityError::Provider("no dummy bluetooth scan queued".into()))
        }
    }

    #[derive(Default)]
    struct DummyBluetoothConnectionDevice {
        connections: RefCell<Vec<BluetoothConnectionInfo>>,
    }

    impl edgerun_capabilities::CapabilityProvider for DummyBluetoothConnectionDevice {
        fn descriptor(&self) -> CapabilityDescriptor {
            default_bluetooth_descriptor("dummy-bt", "hci0")
        }
    }

    impl BluetoothConnectionProvider for DummyBluetoothConnectionDevice {
        fn list_connections(&self) -> Result<Vec<BluetoothConnectionInfo>, CapabilityError> {
            Ok(self.connections.borrow().clone())
        }
    }

    #[derive(Default)]
    struct DummyWifiScannerDevice {
        scans: RefCell<VecDeque<WifiScanResult>>,
    }

    impl edgerun_capabilities::CapabilityProvider for DummyWifiScannerDevice {
        fn descriptor(&self) -> CapabilityDescriptor {
            default_wifi_descriptor("dummy-wifi", "wlan0")
        }
    }

    impl WifiScanner for DummyWifiScannerDevice {
        fn scan_nearby(&self) -> Result<WifiScanResult, CapabilityError> {
            self.scans
                .borrow_mut()
                .pop_front()
                .ok_or_else(|| CapabilityError::Provider("no dummy wifi scan queued".into()))
        }
    }

    #[derive(Default)]
    struct DummyWifiControllerDevice {
        info: RefCell<Option<WifiInterfaceInfo>>,
        last_state: RefCell<Option<WifiPowerState>>,
    }

    impl edgerun_capabilities::CapabilityProvider for DummyWifiControllerDevice {
        fn descriptor(&self) -> CapabilityDescriptor {
            default_wifi_descriptor("dummy-wifi", "wlan0")
        }
    }

    impl WifiController for DummyWifiControllerDevice {
        fn interface_info(&self) -> Result<WifiInterfaceInfo, CapabilityError> {
            self.info.borrow().clone().ok_or_else(|| {
                CapabilityError::Provider("no dummy wifi interface info configured".into())
            })
        }

        fn power_state(&self) -> Result<WifiPowerState, CapabilityError> {
            Ok(self
                .last_state
                .borrow()
                .or_else(|| self.info.borrow().as_ref().map(|v| v.power_state))
                .unwrap_or(WifiPowerState::Unknown))
        }

        fn set_power_state(
            &self,
            state: WifiPowerState,
        ) -> Result<WifiPowerState, CapabilityError> {
            *self.last_state.borrow_mut() = Some(state);
            if let Some(info) = self.info.borrow_mut().as_mut() {
                info.power_state = state;
            }
            Ok(state)
        }
    }

    struct DummyCameraDevice {
        captures: VecDeque<CameraCapture>,
    }

    impl DummyCameraDevice {
        fn descriptor() -> CapabilityDescriptor {
            edgerun_capabilities::capability_descriptor(
                "dummy-camera",
                "video0",
                edgerun_capabilities::CapabilityRole::Input,
                &[edgerun_capabilities::CapabilityModality::Visual],
                &[edgerun_capabilities::CapabilityEventKind::Visual],
                &[edgerun_capabilities::CapabilityOperation::Observe],
                Vec::new(),
            )
        }
    }

    impl CameraBiometricReader for DummyCameraDevice {
        fn reader_info(&self) -> Result<CameraReaderInfo, CameraBiometricError> {
            Ok(CameraReaderInfo {
                provider: "dummy-camera".into(),
                reader_name: "Dummy Camera".into(),
                supports_face_detection: true,
                supports_face_matching: false,
                supports_liveness_detection: false,
                hardware_protected_match: false,
            })
        }

        fn capture(
            &mut self,
            _purpose: CameraBiometricPurpose,
            _timeout_ms: u32,
        ) -> Result<CameraCapture, CameraBiometricError> {
            self.captures.pop_front().ok_or_else(|| {
                CameraBiometricError::Provider("no dummy camera capture queued".into())
            })
        }

        fn begin_enrollment(
            &mut self,
            _request: &edgerun_camera_biometrics::CameraEnrollRequest,
        ) -> Result<edgerun_camera_biometrics::CameraEnrollmentSession, CameraBiometricError>
        {
            Err(CameraBiometricError::UnsupportedOperation("dummy"))
        }
        fn enroll_step(
            &mut self,
            _session_id: &str,
            _capture: &CameraCapture,
        ) -> Result<edgerun_camera_biometrics::CameraEnrollProgress, CameraBiometricError>
        {
            Err(CameraBiometricError::UnsupportedOperation("dummy"))
        }
        fn finish_enrollment(
            &mut self,
            _session_id: &str,
        ) -> Result<edgerun_camera_biometrics::CameraTemplateRecord, CameraBiometricError>
        {
            Err(CameraBiometricError::UnsupportedOperation("dummy"))
        }
        fn verify_capture(
            &mut self,
            _request: &edgerun_camera_biometrics::CameraVerifyRequest,
            _capture: &CameraCapture,
        ) -> Result<edgerun_camera_biometrics::CameraVerification, CameraBiometricError> {
            Err(CameraBiometricError::UnsupportedOperation("dummy"))
        }
        fn list_templates(
            &self,
        ) -> Result<Vec<edgerun_camera_biometrics::CameraTemplateRecord>, CameraBiometricError>
        {
            Ok(Vec::new())
        }
        fn delete_template(&mut self, _template_id: &str) -> Result<(), CameraBiometricError> {
            Err(CameraBiometricError::UnsupportedOperation("dummy"))
        }
    }

    struct DummyPairedCameraDevice {
        captures: VecDeque<PairedCameraFrame>,
    }

    impl CameraBiometricReader for DummyPairedCameraDevice {
        fn reader_info(&self) -> Result<CameraReaderInfo, CameraBiometricError> {
            Ok(CameraReaderInfo {
                provider: "dummy-paired-camera".into(),
                reader_name: "Dummy Paired Camera".into(),
                supports_face_detection: true,
                supports_face_matching: false,
                supports_liveness_detection: true,
                hardware_protected_match: false,
            })
        }

        fn capture(
            &mut self,
            _purpose: CameraBiometricPurpose,
            _timeout_ms: u32,
        ) -> Result<CameraCapture, CameraBiometricError> {
            Err(CameraBiometricError::UnsupportedOperation(
                "use paired capture",
            ))
        }

        fn begin_enrollment(
            &mut self,
            _request: &edgerun_camera_biometrics::CameraEnrollRequest,
        ) -> Result<edgerun_camera_biometrics::CameraEnrollmentSession, CameraBiometricError>
        {
            Err(CameraBiometricError::UnsupportedOperation("dummy"))
        }
        fn enroll_step(
            &mut self,
            _session_id: &str,
            _capture: &CameraCapture,
        ) -> Result<edgerun_camera_biometrics::CameraEnrollProgress, CameraBiometricError>
        {
            Err(CameraBiometricError::UnsupportedOperation("dummy"))
        }
        fn finish_enrollment(
            &mut self,
            _session_id: &str,
        ) -> Result<edgerun_camera_biometrics::CameraTemplateRecord, CameraBiometricError>
        {
            Err(CameraBiometricError::UnsupportedOperation("dummy"))
        }
        fn verify_capture(
            &mut self,
            _request: &edgerun_camera_biometrics::CameraVerifyRequest,
            _capture: &CameraCapture,
        ) -> Result<edgerun_camera_biometrics::CameraVerification, CameraBiometricError> {
            Err(CameraBiometricError::UnsupportedOperation("dummy"))
        }
        fn list_templates(
            &self,
        ) -> Result<Vec<edgerun_camera_biometrics::CameraTemplateRecord>, CameraBiometricError>
        {
            Ok(Vec::new())
        }
        fn delete_template(&mut self, _template_id: &str) -> Result<(), CameraBiometricError> {
            Err(CameraBiometricError::UnsupportedOperation("dummy"))
        }
    }

    impl PairedCameraBiometricReader for DummyPairedCameraDevice {
        fn supported_stream_roles(&self) -> Result<Vec<CameraStreamRole>, CameraBiometricError> {
            Ok(vec![CameraStreamRole::Rgb, CameraStreamRole::Infrared])
        }

        fn capture_paired(
            &mut self,
            _purpose: CameraBiometricPurpose,
            _timeout_ms: u32,
        ) -> Result<PairedCameraFrame, CameraBiometricError> {
            self.captures.pop_front().ok_or_else(|| {
                CameraBiometricError::Provider("no dummy paired camera capture queued".into())
            })
        }

        fn evaluate_liveness(
            &mut self,
            _challenge: &edgerun_camera_biometrics::CameraLivenessChallenge,
            _capture: &PairedCameraFrame,
        ) -> Result<edgerun_camera_biometrics::CameraLivenessResult, CameraBiometricError>
        {
            Err(CameraBiometricError::UnsupportedOperation("dummy"))
        }
    }

    impl InputDevice for DummyInputDevice {
        fn input_info(&self) -> Result<InputDeviceInfo, CapabilityError> {
            Ok(InputDeviceInfo {
                provider: "dummy-input".into(),
                instance_id: "event0".into(),
                display_name: "Dummy Input".into(),
                kind: InputDeviceKind::Keyboard,
                event_node: "/dev/input/event0".into(),
                physical_path: None,
                unique_id: None,
            })
        }

        fn read_events(
            &mut self,
            _max_events: usize,
        ) -> Result<Vec<InputEventRecord>, CapabilityError> {
            Ok(self.events.pop_front().unwrap_or_default())
        }
    }

    #[test]
    fn loopback_open_and_invoke_roundtrip() {
        let (mut client, mut server) = MemoryRemoteTransport::pair();
        let mut provider = PolicyWrappedProvider::new(DummyProvider::default());

        client
            .send(CapabilityRemoteEnvelope {
                message: Some(capability_remote_envelope::Message::SessionOpen(
                    CapabilitySessionOpen {
                        version: 1,
                        session_id: b"sess".to_vec(),
                        selector: None,
                        mode: CapabilitySessionMode::Unary as i32,
                        requested_operations: vec![CapabilityOperation::Observe as i32],
                        requested_access_class: CapabilityAccessClass::Derived as i32,
                        requested_constraints: Vec::new(),
                        correlation_id: Vec::new(),
                    },
                )),
            })
            .unwrap();
        assert!(serve_one(&mut provider, &mut server).unwrap());
        let accept = client.recv().unwrap().unwrap();
        let grant_id = match accept.message {
            Some(capability_remote_envelope::Message::SessionAccept(accept)) => {
                assert!(accept.accepted);
                assert_eq!(accept.session_id, b"sess".to_vec());
                accept.grant_id
            }
            other => panic!("unexpected message: {other:?}"),
        };

        client
            .send(CapabilityRemoteEnvelope {
                message: Some(capability_remote_envelope::Message::Invocation(
                    CapabilityInvocation {
                        invocation_version: 1,
                        invocation_id: b"inv".to_vec(),
                        grant_id,
                        invoker: None,
                        operation: CapabilityOperation::Observe as i32,
                        requested_access_class: CapabilityAccessClass::Derived as i32,
                        parameter_object: None,
                        correlation_id: Vec::new(),
                        invoked_at: None,
                        signature: None,
                    },
                )),
            })
            .unwrap();
        assert!(serve_one(&mut provider, &mut server).unwrap());
        let result = client.recv().unwrap().unwrap();
        match result.message {
            Some(capability_remote_envelope::Message::Result(result)) => {
                assert!(result.success);
                assert_eq!(result.invocation_id, b"inv".to_vec());
            }
            other => panic!("unexpected message: {other:?}"),
        }
    }

    #[test]
    fn policy_wrapper_denies_remote_raw_session() {
        let mut provider = PolicyWrappedProvider::new(DummyProvider::default());
        let accept = provider
            .open_session(&CapabilitySessionOpen {
                version: 1,
                session_id: b"sess".to_vec(),
                selector: None,
                mode: CapabilitySessionMode::Unary as i32,
                requested_operations: vec![CapabilityOperation::Observe as i32],
                requested_access_class: CapabilityAccessClass::Raw as i32,
                requested_constraints: Vec::new(),
                correlation_id: Vec::new(),
            })
            .unwrap();
        assert!(!accept.accepted);
        assert_eq!(
            accept.error_reason,
            "raw capability access is only allowed locally"
        );
    }

    #[test]
    fn session_open_stores_grant_via_unified_request_path() {
        let mut provider = PolicyWrappedProvider::new(DummyProvider::default());
        let accept = provider
            .open_session(&CapabilitySessionOpen {
                version: 1,
                session_id: b"sess".to_vec(),
                selector: None,
                mode: CapabilitySessionMode::Unary as i32,
                requested_operations: vec![CapabilityOperation::Observe as i32],
                requested_access_class: CapabilityAccessClass::Derived as i32,
                requested_constraints: Vec::new(),
                correlation_id: Vec::new(),
            })
            .unwrap();
        assert!(accept.accepted);
        // Sessions are now keyed by grant_id (which is returned in the accept)
        let binding = provider.sessions().get(&accept.grant_id).unwrap();
        assert_eq!(binding.session_id, b"sess");
        assert!(provider.policy().grant_record(&binding.grant_id).is_some());
    }

    #[test]
    fn session_open_is_normalized_into_proto_request() {
        let provider = DummyProvider::default();
        let descriptor = provider.descriptor();
        let open = CapabilitySessionOpen {
            version: 1,
            session_id: b"sess".to_vec(),
            selector: None,
            mode: CapabilitySessionMode::Unary as i32,
            requested_operations: vec![CapabilityOperation::Observe as i32],
            requested_access_class: CapabilityAccessClass::Derived as i32,
            requested_constraints: Vec::new(),
            correlation_id: b"corr".to_vec(),
        };
        let request =
            session_open_as_request(&open, &descriptor, Some(default_remote_requester()), None);
        assert_eq!(request.request_id, open.session_id);
        assert_eq!(request.requested_operations, open.requested_operations);
        assert_eq!(request.correlation_id, b"corr".to_vec());
        assert!(request.selector.is_some());
        assert!(request.requester.is_some());
    }

    #[test]
    fn adapter_can_be_wrapped_via_standard_trait() {
        let provider = DummyProvider::default().into_policy_wrapped();
        let context = provider.context();
        assert!(!context.is_local);
    }

    #[test]
    fn loopback_request_envelope_returns_grant() {
        let (mut client, mut server) = MemoryRemoteTransport::pair();
        let mut provider = PolicyWrappedProvider::new(DummyProvider::default());

        client
            .send(CapabilityRemoteEnvelope {
                message: Some(capability_remote_envelope::Message::Request(
                    CapabilityRequest {
                        request_version: 1,
                        request_id: b"req-1".to_vec(),
                        requester: Some(default_remote_requester()),
                        requester_node: None,
                        selector: Some(
                            session_open_as_request(
                                &CapabilitySessionOpen {
                                    version: 1,
                                    session_id: b"sess".to_vec(),
                                    selector: None,
                                    mode: CapabilitySessionMode::Unary as i32,
                                    requested_operations: vec![CapabilityOperation::Observe as i32],
                                    requested_access_class: CapabilityAccessClass::Derived as i32,
                                    requested_constraints: Vec::new(),
                                    correlation_id: Vec::new(),
                                },
                                &provider.descriptor(),
                                Some(default_remote_requester()),
                                None,
                            )
                            .selector
                            .unwrap(),
                        ),
                        requested_operations: vec![CapabilityOperation::Observe as i32],
                        requested_constraints: Vec::new(),
                        purpose: "test".into(),
                        requested_duration: None,
                        correlation_id: Vec::new(),
                        signature: None,
                    },
                )),
            })
            .unwrap();

        assert!(serve_one(&mut provider, &mut server).unwrap());
        let response = client.recv().unwrap().unwrap();
        match response.message {
            Some(capability_remote_envelope::Message::Grant(grant)) => {
                assert_eq!(
                    grant.granted_operations,
                    vec![CapabilityOperation::Observe as i32]
                );
                assert_eq!(grant.access_class, CapabilityAccessClass::Derived as i32);
            }
            other => panic!("unexpected message: {other:?}"),
        }
    }

    #[test]
    fn inbound_grant_and_revocation_are_applied() {
        let mut provider = PolicyWrappedProvider::new(DummyProvider::default());
        let descriptor = provider.descriptor();
        let request = session_open_as_request(
            &CapabilitySessionOpen {
                version: 1,
                session_id: b"sess".to_vec(),
                selector: None,
                mode: CapabilitySessionMode::Unary as i32,
                requested_operations: vec![CapabilityOperation::Observe as i32],
                requested_access_class: CapabilityAccessClass::Derived as i32,
                requested_constraints: Vec::new(),
                correlation_id: Vec::new(),
            },
            &descriptor,
            Some(default_remote_requester()),
            None,
        );
        let grant = match provider.handle_request(&request).unwrap() {
            Some(grant) => grant,
            None => panic!("expected grant"),
        };
        provider.handle_grant(&grant).unwrap();
        assert!(provider.policy().grant_record(&grant.grant_id).is_some());

        let revocation = CapabilityRevocation {
            revocation_version: 1,
            revocation_id: b"rev-1".to_vec(),
            grant_id: grant.grant_id.clone(),
            issuer: None,
            effective_at: None,
            reason: "test".into(),
            replacement_constraints: Vec::new(),
            signature: None,
        };
        provider.handle_revocation(&revocation).unwrap();
        assert!(provider
            .policy()
            .grant_record(&grant.grant_id)
            .unwrap()
            .is_revoked());
    }

    #[test]
    fn end_to_end_session_revocation_denies_later_invocation() {
        let (mut client, mut server) = MemoryRemoteTransport::pair();
        let mut provider = PolicyWrappedProvider::new(DummyProvider::default());

        client
            .send(CapabilityRemoteEnvelope {
                message: Some(capability_remote_envelope::Message::SessionOpen(
                    CapabilitySessionOpen {
                        version: 1,
                        session_id: b"sess".to_vec(),
                        selector: None,
                        mode: CapabilitySessionMode::Unary as i32,
                        requested_operations: vec![CapabilityOperation::Observe as i32],
                        requested_access_class: CapabilityAccessClass::Derived as i32,
                        requested_constraints: Vec::new(),
                        correlation_id: Vec::new(),
                    },
                )),
            })
            .unwrap();
        assert!(serve_one(&mut provider, &mut server).unwrap());
        let accept_env = client.recv().unwrap().unwrap();
        let grant_id = match accept_env.message {
            Some(capability_remote_envelope::Message::SessionAccept(ref a)) => a.grant_id.clone(),
            _ => panic!("expected SessionAccept"),
        };
        // Verify session is stored by grant_id
        assert!(provider.sessions().get(&grant_id).is_some());

        let revocation = CapabilityRevocation {
            revocation_version: 1,
            revocation_id: b"rev-1".to_vec(),
            grant_id: grant_id.clone(),
            issuer: None,
            effective_at: None,
            reason: "test".into(),
            replacement_constraints: Vec::new(),
            signature: None,
        };

        client
            .send(CapabilityRemoteEnvelope {
                message: Some(capability_remote_envelope::Message::Revocation(revocation)),
            })
            .unwrap();
        assert!(serve_one(&mut provider, &mut server).unwrap());

        client
            .send(CapabilityRemoteEnvelope {
                message: Some(capability_remote_envelope::Message::Invocation(
                    CapabilityInvocation {
                        invocation_version: 1,
                        invocation_id: b"inv-after-revoke".to_vec(),
                        grant_id,
                        invoker: None,
                        operation: CapabilityOperation::Observe as i32,
                        requested_access_class: CapabilityAccessClass::Derived as i32,
                        parameter_object: None,
                        correlation_id: Vec::new(),
                        invoked_at: None,
                        signature: None,
                    },
                )),
            })
            .unwrap();
        assert!(serve_one(&mut provider, &mut server).unwrap());
        let result = client.recv().unwrap().unwrap();
        match result.message {
            Some(capability_remote_envelope::Message::Result(result)) => {
                assert!(!result.success);
                assert_eq!(result.invocation_id, b"inv-after-revoke".to_vec());
                assert!(result
                    .error_reason
                    .contains("capability permission denied: capability grant has been revoked"));
            }
            other => panic!("unexpected message: {other:?}"),
        }
    }

    #[test]
    fn unix_stream_framed_transport_roundtrip() {
        let (client_stream, server_stream) = UnixStream::pair().unwrap();
        let mut client = FramedRemoteTransport::new(client_stream);
        let mut server = FramedRemoteTransport::new(server_stream);
        let mut provider = PolicyWrappedProvider::new(DummyProvider::default());

        client
            .send(CapabilityRemoteEnvelope {
                message: Some(capability_remote_envelope::Message::SessionOpen(
                    CapabilitySessionOpen {
                        version: 1,
                        session_id: b"sock-sess".to_vec(),
                        selector: None,
                        mode: CapabilitySessionMode::Unary as i32,
                        requested_operations: vec![CapabilityOperation::Observe as i32],
                        requested_access_class: CapabilityAccessClass::Derived as i32,
                        requested_constraints: Vec::new(),
                        correlation_id: Vec::new(),
                    },
                )),
            })
            .unwrap();
        assert!(serve_one(&mut provider, &mut server).unwrap());
        let accept = client.recv().unwrap().unwrap();
        match accept.message {
            Some(capability_remote_envelope::Message::SessionAccept(accept)) => {
                assert!(accept.accepted);
                assert_eq!(accept.session_id, b"sock-sess".to_vec());
            }
            other => panic!("unexpected message: {other:?}"),
        }
    }

    #[test]
    fn loopback_stream_event_roundtrip() {
        let (mut client, mut server) = MemoryRemoteTransport::pair();
        let mut provider = DummyProvider::default();
        assert!(pump_one_event(&mut provider, &mut server, b"stream").unwrap());
        let event = client.recv().unwrap().unwrap();
        match event.message {
            Some(capability_remote_envelope::Message::SessionEvent(event)) => {
                assert_eq!(event.session_id, b"stream".to_vec());
                assert_eq!(event.inline_payload, b"hello".to_vec());
            }
            other => panic!("unexpected message: {other:?}"),
        }
    }

    #[test]
    fn input_event_payload_roundtrip() {
        let events = vec![
            InputEventRecord {
                timestamp_sec: 1,
                timestamp_usec: 2,
                kind: InputEventKind::Key,
                code: 30,
                value: 1,
            },
            InputEventRecord {
                timestamp_sec: 3,
                timestamp_usec: 4,
                kind: InputEventKind::RelativeMotion,
                code: 0,
                value: -2,
            },
        ];
        let encoded = encode_input_events(&events);
        let decoded = decode_input_events(&encoded).unwrap();
        assert_eq!(decoded, events);
    }

    #[test]
    fn input_remote_adapter_streams_events() {
        let (mut client, mut server) = MemoryRemoteTransport::pair();
        let mut adapter = InputRemoteAdapter::new(
            DummyInputDevice {
                events: VecDeque::from([vec![InputEventRecord {
                    timestamp_sec: 10,
                    timestamp_usec: 20,
                    kind: InputEventKind::Key,
                    code: 30,
                    value: 1,
                }]]),
            },
            8,
        );
        assert!(pump_one_event(&mut adapter, &mut server, b"input").unwrap());
        let event = client.recv().unwrap().unwrap();
        match event.message {
            Some(capability_remote_envelope::Message::SessionEvent(event)) => {
                let decoded = decode_input_events(&event.inline_payload).unwrap();
                assert_eq!(decoded.len(), 1);
                assert_eq!(decoded[0].code, 30);
            }
            other => panic!("unexpected message: {other:?}"),
        }
    }

    #[test]
    fn microphone_capture_payload_roundtrip() {
        let capture = AudioCapture {
            sample_rate_hz: 48_000,
            channels: 2,
            format: MicrophoneSampleFormat::PcmS16Le,
            bytes: vec![1, 2, 3, 4, 5, 6],
            started_at_unix_ms: 1234,
        };
        let encoded = encode_microphone_capture(&capture);
        let decoded = decode_microphone_capture(&encoded).unwrap();
        assert_eq!(decoded, capture);
    }

    #[test]
    fn microphone_remote_adapter_streams_capture() {
        let (mut client, mut server) = MemoryRemoteTransport::pair();
        let mut adapter = MicrophoneRemoteAdapter::new(
            DummyMicrophoneDevice {
                captures: VecDeque::from([AudioCapture {
                    sample_rate_hz: 48_000,
                    channels: 2,
                    format: MicrophoneSampleFormat::PcmS16Le,
                    bytes: vec![1, 2, 3, 4],
                    started_at_unix_ms: 42,
                }]),
            },
            AudioCaptureRequest {
                duration_ms: 10,
                sample_rate_hz: 48_000,
                channels: 2,
                format: MicrophoneSampleFormat::PcmS16Le,
            },
        );
        assert!(pump_one_event(&mut adapter, &mut server, b"mic").unwrap());
        let event = client.recv().unwrap().unwrap();
        match event.message {
            Some(capability_remote_envelope::Message::SessionEvent(event)) => {
                let decoded = decode_microphone_capture(&event.inline_payload).unwrap();
                assert_eq!(decoded.bytes, vec![1, 2, 3, 4]);
                assert_eq!(decoded.started_at_unix_ms, 42);
            }
            other => panic!("unexpected message: {other:?}"),
        }
    }
    #[test]
    fn speaker_playback_request_payload_roundtrip() {
        let request = AudioPlaybackRequest {
            duration_ms: 25,
            sample_rate_hz: 48_000,
            channels: 2,
            format: SpeakerSampleFormat::PcmS16Le,
            audio_bytes: vec![1, 2, 3, 4],
            software_gain_percent: Some(35),
            target_output_level_percent: Some(12),
        };
        let encoded = encode_speaker_playback_request(&request);
        let decoded = decode_speaker_playback_request(&encoded).unwrap();
        assert_eq!(decoded, request);
    }

    #[test]
    fn speaker_playback_result_payload_roundtrip() {
        let result = AudioPlaybackResult {
            bytes_written: 1536,
            sample_rate_hz: 48_000,
            channels: 2,
            finished: true,
        };
        let encoded = encode_speaker_playback_result(&result);
        let decoded = decode_speaker_playback_result(&encoded).unwrap();
        assert_eq!(decoded, result);
    }

    #[test]
    fn speaker_remote_adapter_unary_render_roundtrip() {
        let (mut client, mut server) = MemoryRemoteTransport::pair();
        let mut adapter = SpeakerRemoteAdapter::new(DummySpeakerDevice {
            result: RefCell::new(Some(AudioPlaybackResult {
                bytes_written: 4,
                sample_rate_hz: 48_000,
                channels: 2,
                finished: true,
            })),
            output_level: RefCell::new(Some(SpeakerOutputLevel {
                current_percent: 10,
                min_raw_value: 0,
                max_raw_value: 100,
                muted: Some(false),
            })),
            last_request: RefCell::new(None),
            last_set_level: RefCell::new(None),
        });

        let request = AudioPlaybackRequest {
            duration_ms: 10,
            sample_rate_hz: 48_000,
            channels: 2,
            format: SpeakerSampleFormat::PcmS16Le,
            audio_bytes: vec![1, 2, 3, 4],
            software_gain_percent: Some(50),
            target_output_level_percent: None,
        };

        client
            .send(CapabilityRemoteEnvelope {
                message: Some(capability_remote_envelope::Message::InvocationFrame(
                    CapabilityInvocationFrame {
                        invocation: Some(CapabilityInvocation {
                            invocation_version: 1,
                            invocation_id: b"spk-inv".to_vec(),
                            grant_id: b"spk-session".to_vec(),
                            invoker: None,
                            operation: CapabilityOperation::Render as i32,
                            requested_access_class: CapabilityAccessClass::Raw as i32,
                            parameter_object: None,
                            correlation_id: Vec::new(),
                            invoked_at: None,
                            signature: None,
                        }),
                        inline_parameters: encode_speaker_playback_request(&request),
                    },
                )),
            })
            .unwrap();

        assert!(serve_one(&mut adapter, &mut server).unwrap());
        let result = client.recv().unwrap().unwrap();
        match result.message {
            Some(capability_remote_envelope::Message::ResultFrame(frame)) => {
                let proto = frame.result.unwrap();
                assert!(proto.success);
                assert_eq!(proto.invocation_id, b"spk-inv".to_vec());
                let decoded = decode_speaker_playback_result(&frame.inline_payload).unwrap();
                assert_eq!(decoded.bytes_written, 4);
            }
            other => panic!("unexpected message: {other:?}"),
        }
        assert_eq!(
            adapter.device.last_request.borrow().as_ref().unwrap(),
            &request
        );
    }

    #[test]
    fn camera_capture_payload_roundtrip() {
        let capture = CameraCapture {
            frame: CameraFrame {
                width: 640,
                height: 480,
                stride: 640,
                format: CameraPixelFormat::Gray8,
                bytes: vec![1, 2, 3, 4, 5],
            },
            quality: CameraCaptureQuality::Good,
            face_bounds: Some(edgerun_camera_biometrics::FaceBounds {
                x: 10,
                y: 20,
                width: 100,
                height: 120,
            }),
            state: default_face_biometric_state(true, false, true),
        };
        let encoded = encode_camera_capture(&capture);
        let decoded = decode_camera_capture(&encoded).unwrap();
        assert_eq!(decoded, capture);
    }

    #[test]
    fn paired_camera_frame_payload_roundtrip() {
        let frame = PairedCameraFrame {
            rgb: Some(CameraFrame {
                width: 1280,
                height: 720,
                stride: 1280,
                format: CameraPixelFormat::Mjpeg,
                bytes: vec![9, 8, 7],
            }),
            infrared: Some(CameraFrame {
                width: 340,
                height: 340,
                stride: 340,
                format: CameraPixelFormat::Gray8,
                bytes: vec![6, 5, 4, 3],
            }),
            depth: None,
        };
        let encoded = encode_paired_camera_frame(&frame);
        let decoded = decode_paired_camera_frame(&encoded).unwrap();
        assert_eq!(decoded, frame);
    }

    #[test]
    fn camera_remote_adapter_streams_capture() {
        let (mut client, mut server) = MemoryRemoteTransport::pair();
        let mut adapter = CameraRemoteAdapter::new(
            DummyCameraDevice {
                captures: VecDeque::from([CameraCapture {
                    frame: CameraFrame {
                        width: 640,
                        height: 480,
                        stride: 640,
                        format: CameraPixelFormat::Gray8,
                        bytes: vec![1, 2, 3],
                    },
                    quality: CameraCaptureQuality::Good,
                    face_bounds: None,
                    state: default_face_biometric_state(false, false, true),
                }]),
            },
            CameraBiometricPurpose::Presence,
            50,
            DummyCameraDevice::descriptor(),
        );
        assert!(pump_one_event(&mut adapter, &mut server, b"cam").unwrap());
        let event = client.recv().unwrap().unwrap();
        match event.message {
            Some(capability_remote_envelope::Message::SessionEvent(event)) => {
                let decoded = decode_camera_capture(&event.inline_payload).unwrap();
                assert_eq!(decoded.frame.width, 640);
                assert_eq!(decoded.frame.bytes, vec![1, 2, 3]);
            }
            other => panic!("unexpected message: {other:?}"),
        }
    }

    #[test]
    fn paired_camera_remote_adapter_streams_capture() {
        let (mut client, mut server) = MemoryRemoteTransport::pair();
        let mut adapter = PairedCameraRemoteAdapter::new(
            DummyPairedCameraDevice {
                captures: VecDeque::from([PairedCameraFrame {
                    rgb: Some(CameraFrame {
                        width: 1280,
                        height: 720,
                        stride: 1280,
                        format: CameraPixelFormat::Mjpeg,
                        bytes: vec![1, 2],
                    }),
                    infrared: Some(CameraFrame {
                        width: 340,
                        height: 340,
                        stride: 340,
                        format: CameraPixelFormat::Gray8,
                        bytes: vec![3, 4, 5],
                    }),
                    depth: None,
                }]),
            },
            CameraBiometricPurpose::Verification,
            75,
            DummyCameraDevice::descriptor(),
        );
        assert!(pump_one_event(&mut adapter, &mut server, b"pcam").unwrap());
        let event = client.recv().unwrap().unwrap();
        match event.message {
            Some(capability_remote_envelope::Message::SessionEvent(event)) => {
                let decoded = decode_paired_camera_frame(&event.inline_payload).unwrap();
                assert_eq!(decoded.rgb.unwrap().width, 1280);
                assert_eq!(decoded.infrared.unwrap().bytes, vec![3, 4, 5]);
            }
            other => panic!("unexpected message: {other:?}"),
        }
    }

    #[test]
    fn bluetooth_scan_payload_roundtrip() {
        let scan = BluetoothScanResult {
            observations: vec![BluetoothBeaconObservation {
                device_id: "AA:BB:CC:DD:EE:FF".into(),
                transport_kind: BluetoothTransportKind::LowEnergy,
                address_kind: BluetoothAddressKind::Random,
                rssi_dbm: -55,
                tx_power_dbm: Some(-4),
                local_name: Some("Earbuds".into()),
                service_uuids: vec!["180f".into(), "180a".into()],
                profiles: vec![
                    BluetoothProfile::AudioSink,
                    BluetoothProfile::BatteryService,
                ],
                classic_device_class: None,
                advertisement_data: vec![2, 1, 6],
                captured_at_unix_ms: 1234,
            }],
        };
        let encoded = encode_bluetooth_scan_result(&scan);
        let decoded = decode_bluetooth_scan_result(&encoded).unwrap();
        assert_eq!(decoded, scan);
    }

    #[test]
    fn bluetooth_connections_payload_roundtrip() {
        let connections = vec![BluetoothConnectionInfo {
            device_id: "11:22:33:44:55:66".into(),
            transport_kind: BluetoothTransportKind::Classic,
            address_kind: BluetoothAddressKind::Public,
            link_kind: BluetoothLinkKind::Acl,
            outbound: true,
            state: 2,
            local_name: Some("Headphones".into()),
            service_uuids: vec!["110b".into()],
            profiles: vec![BluetoothProfile::AudioSink, BluetoothProfile::Headphones],
            trusted: Some(true),
            paired: Some(true),
        }];
        let encoded = encode_bluetooth_connections(&connections);
        let decoded = decode_bluetooth_connections(&encoded).unwrap();
        assert_eq!(decoded, connections);
    }

    #[test]
    fn bluetooth_remote_adapter_streams_scan() {
        let (mut client, mut server) = MemoryRemoteTransport::pair();
        let mut adapter = BluetoothRemoteAdapter::new(
            DummyBluetoothScannerDevice {
                scans: RefCell::new(VecDeque::from([BluetoothScanResult {
                    observations: vec![BluetoothBeaconObservation {
                        device_id: "AA:BB:CC:DD:EE:FF".into(),
                        transport_kind: BluetoothTransportKind::LowEnergy,
                        address_kind: BluetoothAddressKind::Random,
                        rssi_dbm: -60,
                        tx_power_dbm: None,
                        local_name: Some("Beacon".into()),
                        service_uuids: vec!["180f".into()],
                        profiles: vec![BluetoothProfile::BatteryService],
                        classic_device_class: None,
                        advertisement_data: vec![2, 1, 6],
                        captured_at_unix_ms: 999,
                    }],
                }])),
            },
            default_bluetooth_descriptor("dummy-bt", "hci0"),
        );
        assert!(pump_one_event(&mut adapter, &mut server, b"bt").unwrap());
        let event = client.recv().unwrap().unwrap();
        match event.message {
            Some(capability_remote_envelope::Message::SessionEvent(event)) => {
                let decoded = decode_bluetooth_scan_result(&event.inline_payload).unwrap();
                assert_eq!(decoded.observations.len(), 1);
                assert_eq!(
                    decoded.observations[0].local_name.as_deref(),
                    Some("Beacon")
                );
            }
            other => panic!("unexpected message: {other:?}"),
        }
    }

    #[test]
    fn bluetooth_connection_remote_adapter_returns_connections() {
        let (mut client, mut server) = MemoryRemoteTransport::pair();
        let mut adapter = BluetoothConnectionRemoteAdapter::new(
            DummyBluetoothConnectionDevice {
                connections: RefCell::new(vec![BluetoothConnectionInfo {
                    device_id: "11:22:33:44:55:66".into(),
                    transport_kind: BluetoothTransportKind::Classic,
                    address_kind: BluetoothAddressKind::Public,
                    link_kind: BluetoothLinkKind::Acl,
                    outbound: false,
                    state: 1,
                    local_name: Some("Headset".into()),
                    service_uuids: vec!["111e".into()],
                    profiles: vec![BluetoothProfile::HandsFree],
                    trusted: Some(true),
                    paired: Some(false),
                }]),
            },
            default_bluetooth_descriptor("dummy-bt", "hci0"),
        );
        client
            .send(CapabilityRemoteEnvelope {
                message: Some(capability_remote_envelope::Message::InvocationFrame(
                    CapabilityInvocationFrame {
                        invocation: Some(CapabilityInvocation {
                            invocation_version: 1,
                            invocation_id: b"btconn".to_vec(),
                            grant_id: b"bt-session".to_vec(),
                            invoker: None,
                            operation: CapabilityOperation::Query as i32,
                            requested_access_class: CapabilityAccessClass::Raw as i32,
                            parameter_object: None,
                            correlation_id: Vec::new(),
                            invoked_at: None,
                            signature: None,
                        }),
                        inline_parameters: Vec::new(),
                    },
                )),
            })
            .unwrap();
        assert!(serve_one(&mut adapter, &mut server).unwrap());
        let result = client.recv().unwrap().unwrap();
        match result.message {
            Some(capability_remote_envelope::Message::ResultFrame(frame)) => {
                let proto = frame.result.unwrap();
                assert!(proto.success);
                let decoded = decode_bluetooth_connections(&frame.inline_payload).unwrap();
                assert_eq!(decoded.len(), 1);
                assert_eq!(decoded[0].profiles, vec![BluetoothProfile::HandsFree]);
            }
            other => panic!("unexpected message: {other:?}"),
        }
    }

    #[test]
    fn wifi_scan_payload_roundtrip() {
        let scan = WifiScanResult {
            observations: vec![WifiNetworkObservation {
                interface_name: "wlan0".into(),
                ssid: Some("office".into()),
                bssid: Some("aa:bb:cc:dd:ee:ff".into()),
                signal_dbm: Some(-48),
                frequency_mhz: Some(5180),
                secure: Some(true),
                observed_at_unix_ms: 77,
            }],
        };
        let encoded = encode_wifi_scan_result(&scan);
        let decoded = decode_wifi_scan_result(&encoded).unwrap();
        assert_eq!(decoded, scan);
    }

    #[test]
    fn wifi_interface_info_payload_roundtrip() {
        let info = WifiInterfaceInfo {
            provider: "linux-wifi".into(),
            interface_name: "wlan0".into(),
            mac_address: Some("00:11:22:33:44:55".into()),
            phy_name: Some("phy0".into()),
            operstate: Some("up".into()),
            power_state: WifiPowerState::Enabled,
            mode: WifiInterfaceMode::Client,
        };
        let encoded = encode_wifi_interface_info(&info);
        let decoded = decode_wifi_interface_info(&encoded).unwrap();
        assert_eq!(decoded, info);
    }

    #[test]
    fn wifi_remote_adapter_streams_scan() {
        let (mut client, mut server) = MemoryRemoteTransport::pair();
        let mut adapter = WifiRemoteAdapter::new(
            DummyWifiScannerDevice {
                scans: RefCell::new(VecDeque::from([WifiScanResult {
                    observations: vec![WifiNetworkObservation {
                        interface_name: "wlan0".into(),
                        ssid: Some("mesh".into()),
                        bssid: Some("de:ad:be:ef:00:01".into()),
                        signal_dbm: Some(-58),
                        frequency_mhz: Some(2412),
                        secure: Some(false),
                        observed_at_unix_ms: 111,
                    }],
                }])),
            },
            default_wifi_descriptor("dummy-wifi", "wlan0"),
        );
        assert!(pump_one_event(&mut adapter, &mut server, b"wifi").unwrap());
        let event = client.recv().unwrap().unwrap();
        match event.message {
            Some(capability_remote_envelope::Message::SessionEvent(event)) => {
                let decoded = decode_wifi_scan_result(&event.inline_payload).unwrap();
                assert_eq!(decoded.observations.len(), 1);
                assert_eq!(decoded.observations[0].ssid.as_deref(), Some("mesh"));
            }
            other => panic!("unexpected message: {other:?}"),
        }
    }

    #[test]
    fn wifi_control_remote_adapter_returns_interface_info() {
        let (mut client, mut server) = MemoryRemoteTransport::pair();
        let mut adapter = WifiControlRemoteAdapter::new(
            DummyWifiControllerDevice {
                info: RefCell::new(Some(WifiInterfaceInfo {
                    provider: "dummy-wifi".into(),
                    interface_name: "wlan0".into(),
                    mac_address: Some("00:11:22:33:44:55".into()),
                    phy_name: Some("phy0".into()),
                    operstate: Some("up".into()),
                    power_state: WifiPowerState::Enabled,
                    mode: WifiInterfaceMode::Client,
                })),
                last_state: RefCell::new(None),
            },
            default_wifi_descriptor("dummy-wifi", "wlan0"),
        );
        client
            .send(CapabilityRemoteEnvelope {
                message: Some(capability_remote_envelope::Message::InvocationFrame(
                    CapabilityInvocationFrame {
                        invocation: Some(CapabilityInvocation {
                            invocation_version: 1,
                            invocation_id: b"wifi-info".to_vec(),
                            grant_id: b"wifi-session".to_vec(),
                            invoker: None,
                            operation: CapabilityOperation::Query as i32,
                            requested_access_class: CapabilityAccessClass::Raw as i32,
                            parameter_object: None,
                            correlation_id: Vec::new(),
                            invoked_at: None,
                            signature: None,
                        }),
                        inline_parameters: Vec::new(),
                    },
                )),
            })
            .unwrap();
        assert!(serve_one(&mut adapter, &mut server).unwrap());
        let result = client.recv().unwrap().unwrap();
        match result.message {
            Some(capability_remote_envelope::Message::ResultFrame(frame)) => {
                assert!(frame.result.unwrap().success);
                let info = decode_wifi_interface_info(&frame.inline_payload).unwrap();
                assert_eq!(info.interface_name, "wlan0");
                assert_eq!(info.power_state, WifiPowerState::Enabled);
            }
            other => panic!("unexpected message: {other:?}"),
        }
    }

    #[test]
    fn wifi_control_remote_adapter_sets_power_state() {
        let (mut client, mut server) = MemoryRemoteTransport::pair();
        let mut adapter = WifiControlRemoteAdapter::new(
            DummyWifiControllerDevice {
                info: RefCell::new(Some(WifiInterfaceInfo {
                    provider: "dummy-wifi".into(),
                    interface_name: "wlan0".into(),
                    mac_address: None,
                    phy_name: None,
                    operstate: Some("down".into()),
                    power_state: WifiPowerState::Disabled,
                    mode: WifiInterfaceMode::Client,
                })),
                last_state: RefCell::new(None),
            },
            default_wifi_descriptor("dummy-wifi", "wlan0"),
        );
        client
            .send(CapabilityRemoteEnvelope {
                message: Some(capability_remote_envelope::Message::InvocationFrame(
                    CapabilityInvocationFrame {
                        invocation: Some(CapabilityInvocation {
                            invocation_version: 1,
                            invocation_id: b"wifi-set".to_vec(),
                            grant_id: b"wifi-session".to_vec(),
                            invoker: None,
                            operation: CapabilityOperation::Control as i32,
                            requested_access_class: CapabilityAccessClass::Raw as i32,
                            parameter_object: None,
                            correlation_id: Vec::new(),
                            invoked_at: None,
                            signature: None,
                        }),
                        inline_parameters: vec![wifi_power_state_to_u8(WifiPowerState::Enabled)],
                    },
                )),
            })
            .unwrap();
        assert!(serve_one(&mut adapter, &mut server).unwrap());
        let result = client.recv().unwrap().unwrap();
        match result.message {
            Some(capability_remote_envelope::Message::ResultFrame(frame)) => {
                assert!(frame.result.unwrap().success);
                let info = decode_wifi_interface_info(&frame.inline_payload).unwrap();
                assert_eq!(info.power_state, WifiPowerState::Enabled);
            }
            other => panic!("unexpected message: {other:?}"),
        }
    }

    // ------------------------------------------------------------------
    // Additional comprehensive tests
    // ------------------------------------------------------------------

    // --- Framed transport encoding/decoding edge cases ---

    #[test]
    fn framed_transport_send_empty_envelope() {
        let (client_stream, server_stream) = UnixStream::pair().unwrap();
        let mut client = FramedRemoteTransport::new(client_stream);
        let mut server = FramedRemoteTransport::new(server_stream);

        // Send an envelope with no message (valid protobuf, just empty)
        client
            .send(CapabilityRemoteEnvelope { message: None })
            .unwrap();
        let recv = server.recv().unwrap();
        assert!(recv.is_some());
        let env = recv.unwrap();
        assert!(env.message.is_none());
    }

    #[test]
    fn framed_transport_recv_eof_returns_none() {
        let (client_stream, server_stream) = UnixStream::pair().unwrap();
        let mut server = FramedRemoteTransport::new(server_stream);
        // Drop client immediately -- server should see EOF
        drop(client_stream);
        let result = server.recv();
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[test]
    fn framed_transport_recv_truncated_length_header() {
        let (mut client_stream, server_stream) = UnixStream::pair().unwrap();
        let mut server = FramedRemoteTransport::new(server_stream);

        // Write only 2 bytes of the 4-byte length header
        client_stream.write_all(&[0x00, 0x00]).unwrap();
        // Then close
        drop(client_stream);

        // The transport should try to read the remaining 2 bytes and fail
        let result = server.recv();
        assert!(result.is_err());
    }

    #[test]
    fn framed_transport_recv_malformed_protobuf() {
        let (mut client_stream, server_stream) = UnixStream::pair().unwrap();
        let mut server = FramedRemoteTransport::new(server_stream);

        // Write a length prefix for 10 bytes, then send garbage
        let garbage = vec![0xFFu8; 10];
        let len = (garbage.len() as u32).to_be_bytes();
        client_stream.write_all(&len).unwrap();
        client_stream.write_all(&garbage).unwrap();

        let result = server.recv();
        assert!(result.is_err());
    }

    #[test]
    fn framed_transport_into_inner() {
        let (stream, _) = UnixStream::pair().unwrap();
        let transport = FramedRemoteTransport::new(stream);
        let _inner = transport.into_inner();
    }

    // --- MemoryRemoteTransport edge cases ---

    #[test]
    fn memory_transport_recv_empty_returns_none() {
        let (mut client, _server) = MemoryRemoteTransport::pair();
        let result = client.recv().unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn memory_transport_send_and_recv() {
        let (mut client, mut server) = MemoryRemoteTransport::pair();
        let envelope = CapabilityRemoteEnvelope {
            message: Some(capability_remote_envelope::Message::SessionClose(
                CapabilitySessionClose {
                    version: 1,
                    session_id: b"sess".to_vec(),
                    reason: String::new(),
                },
            )),
        };
        client.send(envelope.clone()).unwrap();
        let recv = server.recv().unwrap();
        assert!(recv.is_some());
        assert_eq!(recv.unwrap().message, envelope.message);
    }

    #[test]
    fn memory_transport_multiple_messages_fifo() {
        let (mut client, mut server) = MemoryRemoteTransport::pair();
        for i in 0..5 {
            client
                .send(CapabilityRemoteEnvelope {
                    message: Some(capability_remote_envelope::Message::SessionClose(
                        CapabilitySessionClose {
                            version: 1,
                            session_id: vec![i],
                            reason: String::new(),
                        },
                    )),
                })
                .unwrap();
        }
        for i in 0..5 {
            let recv = server.recv().unwrap().unwrap();
            match recv.message {
                Some(capability_remote_envelope::Message::SessionClose(close)) => {
                    assert_eq!(close.session_id, vec![i]);
                }
                other => panic!("expected SessionClose, got {other:?}"),
            }
        }
        assert!(server.recv().unwrap().is_none());
    }

    // --- serve_one protocol handler: all message types ---

    #[test]
    fn serve_one_session_close() {
        let (mut client, mut server) = MemoryRemoteTransport::pair();
        let mut provider = DummyProvider::default();
        // Open a session first
        client
            .send(CapabilityRemoteEnvelope {
                message: Some(capability_remote_envelope::Message::SessionOpen(
                    CapabilitySessionOpen {
                        version: 1,
                        session_id: b"sess".to_vec(),
                        selector: None,
                        mode: CapabilitySessionMode::Unary as i32,
                        requested_operations: vec![CapabilityOperation::Observe as i32],
                        requested_access_class: CapabilityAccessClass::Derived as i32,
                        requested_constraints: Vec::new(),
                        correlation_id: Vec::new(),
                    },
                )),
            })
            .unwrap();
        serve_one(&mut provider, &mut server).unwrap();
        client.recv().unwrap(); // accept

        // Now close
        client
            .send(CapabilityRemoteEnvelope {
                message: Some(capability_remote_envelope::Message::SessionClose(
                    CapabilitySessionClose {
                        version: 1,
                        session_id: b"sess".to_vec(),
                        reason: String::new(),
                    },
                )),
            })
            .unwrap();
        let result = serve_one(&mut provider, &mut server);
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[test]
    fn serve_one_request_grant_roundtrip() {
        let (mut client, mut server) = MemoryRemoteTransport::pair();
        let mut provider = PolicyWrappedProvider::new(DummyProvider::default());

        let selector = edgerun_capabilities::CapabilitySelector {
            capability_id: b"test".to_vec(),
            role: CapabilityRole::Input as i32,
            modalities: vec![CapabilityModality::Text as i32],
            event_kinds: vec![CapabilityEventKind::Text as i32],
            operations: vec![CapabilityOperation::Observe as i32],
            access_class: CapabilityAccessClass::Derived as i32,
            provider_identity: None,
            provider_node: None,
            provider_instance_id: String::new(),
        };
        client
            .send(CapabilityRemoteEnvelope {
                message: Some(capability_remote_envelope::Message::Request(
                    CapabilityRequest {
                        request_version: 1,
                        request_id: b"req-1".to_vec(),
                        requester: Some(default_remote_requester()),
                        requester_node: None,
                        selector: Some(selector),
                        requested_operations: vec![CapabilityOperation::Observe as i32],
                        requested_constraints: Vec::new(),
                        purpose: "test".into(),
                        requested_duration: None,
                        correlation_id: Vec::new(),
                        signature: None,
                    },
                )),
            })
            .unwrap();

        assert!(serve_one(&mut provider, &mut server).unwrap());
        let resp = client.recv().unwrap().unwrap();
        match resp.message {
            Some(capability_remote_envelope::Message::Grant(grant)) => {
                assert_eq!(
                    grant.granted_operations,
                    vec![CapabilityOperation::Observe as i32]
                );
            }
            other => panic!("expected Grant, got {other:?}"),
        }
    }

    #[test]
    fn serve_one_invocation_frame_with_inline_params() {
        let (mut client, mut server) = MemoryRemoteTransport::pair();
        let mut provider = DummyProvider::default();

        client
            .send(CapabilityRemoteEnvelope {
                message: Some(capability_remote_envelope::Message::InvocationFrame(
                    CapabilityInvocationFrame {
                        invocation: Some(CapabilityInvocation {
                            invocation_version: 1,
                            invocation_id: b"inv-frame".to_vec(),
                            grant_id: b"grant".to_vec(),
                            invoker: None,
                            operation: CapabilityOperation::Observe as i32,
                            requested_access_class: CapabilityAccessClass::Derived as i32,
                            parameter_object: None,
                            correlation_id: Vec::new(),
                            invoked_at: None,
                            signature: None,
                        }),
                        inline_parameters: b"inline-data".to_vec(),
                    },
                )),
            })
            .unwrap();

        assert!(serve_one(&mut provider, &mut server).unwrap());
        let resp = client.recv().unwrap().unwrap();
        match resp.message {
            Some(capability_remote_envelope::Message::Result(result)) => {
                assert!(result.success);
                assert_eq!(result.invocation_id, b"inv-frame".to_vec());
            }
            Some(capability_remote_envelope::Message::ResultFrame(frame)) => {
                assert!(frame.result.as_ref().unwrap().success);
            }
            other => panic!("expected Result or ResultFrame, got {other:?}"),
        }
    }

    #[test]
    fn serve_one_invocation_frame_missing_invocation() {
        let (mut client, mut server) = MemoryRemoteTransport::pair();
        let mut provider = DummyProvider::default();

        client
            .send(CapabilityRemoteEnvelope {
                message: Some(capability_remote_envelope::Message::InvocationFrame(
                    CapabilityInvocationFrame {
                        invocation: None,
                        inline_parameters: Vec::new(),
                    },
                )),
            })
            .unwrap();

        let result = serve_one(&mut provider, &mut server);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("invocation"));
    }

    #[test]
    fn serve_one_eof_returns_false() {
        let (_, mut server) = MemoryRemoteTransport::pair();
        let mut provider = DummyProvider::default();
        let result = serve_one(&mut provider, &mut server).unwrap();
        assert!(!result);
    }

    #[test]
    fn serve_one_unknown_message_type_noop() {
        let (mut client, mut server) = MemoryRemoteTransport::pair();
        let mut provider = DummyProvider::default();

        // Send an envelope with None message (matches the `_ => {}` arm)
        client
            .send(CapabilityRemoteEnvelope { message: None })
            .unwrap();

        let result = serve_one(&mut provider, &mut server);
        assert!(result.is_ok());
        assert!(result.unwrap());
        // No response should be sent
        assert!(client.recv().unwrap().is_none());
    }

    // --- pump_one_event ---

    #[test]
    fn pump_one_event_no_event_returns_false() {
        let (mut client, mut server) = MemoryRemoteTransport::pair();
        let mut provider = DummyProvider::default();
        // First call sends an event, second returns None
        pump_one_event(&mut provider, &mut server, b"sess").unwrap();
        client.recv().unwrap(); // drain the event
        let result = pump_one_event(&mut provider, &mut server, b"sess").unwrap();
        assert!(!result);
    }

    // --- PolicyWrappedProvider ---

    #[test]
    fn policy_wrapped_with_policy_and_context() {
        let provider = DummyProvider::default().into_policy_wrapped();
        // Default policy engine allows derived access for remote sessions
        assert!(!provider.context().is_local);
    }

    #[test]
    fn policy_wrapped_inner_accessors() {
        let mut provider = PolicyWrappedProvider::new(DummyProvider::default());
        let _: &DummyProvider = provider.inner();
        let _: &mut DummyProvider = provider.inner_mut();
        let _: &edgerun_capability_policy::SimplePolicyEngine = provider.policy();
        let _: &mut edgerun_capability_policy::SimplePolicyEngine = provider.policy_mut();
        let _: &edgerun_capability_policy::PolicyContext = provider.context();
    }

    #[test]
    fn policy_wrapped_close_session_revokes_grant() {
        let mut provider = PolicyWrappedProvider::new(DummyProvider::default());

        // Open a session (which creates a grant)
        let accept = provider
            .open_session(&CapabilitySessionOpen {
                version: 1,
                session_id: b"sess".to_vec(),
                selector: None,
                mode: CapabilitySessionMode::Unary as i32,
                requested_operations: vec![CapabilityOperation::Observe as i32],
                requested_access_class: CapabilityAccessClass::Derived as i32,
                requested_constraints: Vec::new(),
                correlation_id: Vec::new(),
            })
            .unwrap();
        assert!(accept.accepted);
        let grant_id = accept.grant_id.clone();
        assert!(provider.policy().grant_record(&grant_id).is_some());

        // Close the session
        provider
            .close_session(&CapabilitySessionClose {
                version: 1,
                session_id: b"sess".to_vec(),
                reason: String::new(),
            })
            .unwrap();

        // Grant should be revoked
        let record = provider.policy().grant_record(&grant_id);
        assert!(record.is_some());
        assert!(record.unwrap().is_revoked());
    }

    #[test]
    fn policy_wrapped_invoke_without_session_fails() {
        let mut provider = PolicyWrappedProvider::new(DummyProvider::default());
        let invocation = CapabilityInvocation {
            invocation_version: 1,
            invocation_id: b"inv".to_vec(),
            grant_id: b"nonexistent".to_vec(),
            invoker: None,
            operation: CapabilityOperation::Observe as i32,
            requested_access_class: CapabilityAccessClass::Derived as i32,
            parameter_object: None,
            correlation_id: Vec::new(),
            invoked_at: None,
            signature: None,
        };
        let result = provider.invoke(b"nonexistent", &invocation, None);
        assert!(result.is_err());
        assert!(result.err().unwrap().to_string().contains("policy grant"));
    }

    #[test]
    fn policy_wrapped_invoke_with_session() {
        let mut provider = PolicyWrappedProvider::new(DummyProvider::default());

        // Open session
        let accept = provider
            .open_session(&CapabilitySessionOpen {
                version: 1,
                session_id: b"sess".to_vec(),
                selector: None,
                mode: CapabilitySessionMode::Unary as i32,
                requested_operations: vec![CapabilityOperation::Observe as i32],
                requested_access_class: CapabilityAccessClass::Derived as i32,
                requested_constraints: Vec::new(),
                correlation_id: Vec::new(),
            })
            .unwrap();
        let grant_id = accept.grant_id.clone();

        // Invoke using the grant_id as session_id (how the protocol works)
        let invocation = CapabilityInvocation {
            invocation_version: 1,
            invocation_id: b"inv".to_vec(),
            grant_id: grant_id.clone(),
            invoker: None,
            operation: CapabilityOperation::Observe as i32,
            requested_access_class: CapabilityAccessClass::Derived as i32,
            parameter_object: None,
            correlation_id: Vec::new(),
            invoked_at: None,
            signature: None,
        };
        let result = provider.invoke(&grant_id, &invocation, None);
        assert!(result.is_ok());
        assert!(result.unwrap().result.success);
    }

    // --- SessionGrantBinding ---

    #[test]
    fn session_grant_binding_clone() {
        let binding = SessionGrantBinding {
            session_id: b"sess".to_vec(),
            grant_id: b"grant".to_vec(),
            granted_operations: vec![1, 2],
            granted_access_class: 3,
        };
        let cloned = binding.clone();
        assert_eq!(binding.session_id, cloned.session_id);
        assert_eq!(binding.grant_id, cloned.grant_id);
        assert_eq!(binding.granted_operations, cloned.granted_operations);
        assert_eq!(binding.granted_access_class, cloned.granted_access_class);
    }

    // --- Input encoding/decoding error cases ---

    #[test]
    fn decode_input_events_empty_buffer() {
        let result = decode_input_events(&[]);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("too short"));
    }

    #[test]
    fn decode_input_events_truncated_buffer() {
        // Header says 1 event (24 bytes + 4 header = 28 bytes), but only provide 10
        let mut buf = Vec::new();
        buf.extend_from_slice(&1u32.to_le_bytes());
        buf.extend_from_slice(&[0u8; 6]); // truncated
        let result = decode_input_events(&buf);
        assert!(result.is_err());
    }

    #[test]
    fn decode_input_events_length_mismatch() {
        // Header says 2 events but only provide data for 1
        let events = vec![InputEventRecord {
            timestamp_sec: 1,
            timestamp_usec: 0,
            kind: InputEventKind::Key,
            code: 30,
            value: 1,
        }];
        let mut encoded = encode_input_events(&events);
        // Corrupt the count to 2
        encoded[0..4].copy_from_slice(&2u32.to_le_bytes());
        let result = decode_input_events(&encoded);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("length does not match"));
    }

    #[test]
    fn decode_input_events_all_kinds() {
        let events = vec![
            InputEventRecord {
                timestamp_sec: 0,
                timestamp_usec: 0,
                kind: InputEventKind::Key,
                code: 1,
                value: 0,
            },
            InputEventRecord {
                timestamp_sec: 0,
                timestamp_usec: 0,
                kind: InputEventKind::RelativeMotion,
                code: 2,
                value: 0,
            },
            InputEventRecord {
                timestamp_sec: 0,
                timestamp_usec: 0,
                kind: InputEventKind::AbsoluteMotion,
                code: 3,
                value: 0,
            },
            InputEventRecord {
                timestamp_sec: 0,
                timestamp_usec: 0,
                kind: InputEventKind::Switch,
                code: 4,
                value: 0,
            },
            InputEventRecord {
                timestamp_sec: 0,
                timestamp_usec: 0,
                kind: InputEventKind::Misc,
                code: 5,
                value: 0,
            },
            InputEventRecord {
                timestamp_sec: 0,
                timestamp_usec: 0,
                kind: InputEventKind::Synchronization,
                code: 6,
                value: 0,
            },
            InputEventRecord {
                timestamp_sec: 0,
                timestamp_usec: 0,
                kind: InputEventKind::Other(99),
                code: 7,
                value: 0,
            },
        ];
        let encoded = encode_input_events(&events);
        let decoded = decode_input_events(&encoded).unwrap();
        assert_eq!(decoded, events);
    }

    // --- Microphone encoding/decoding error cases ---

    #[test]
    fn decode_microphone_capture_too_short() {
        let result = decode_microphone_capture(&[0u8; 10]);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("too short"));
    }

    #[test]
    fn decode_microphone_capture_length_mismatch() {
        let capture = AudioCapture {
            sample_rate_hz: 48_000,
            channels: 1,
            format: MicrophoneSampleFormat::PcmS16Le,
            bytes: vec![1, 2, 3],
            started_at_unix_ms: 0,
        };
        let mut encoded = encode_microphone_capture(&capture);
        // Corrupt the byte count
        let bad_len = 999u32.to_le_bytes();
        encoded[18..22].copy_from_slice(&bad_len);
        let result = decode_microphone_capture(&encoded);
        assert!(result.is_err());
    }

    #[test]
    fn decode_microphone_capture_all_formats() {
        for format in [
            MicrophoneSampleFormat::PcmS16Le,
            MicrophoneSampleFormat::PcmS24Le,
            MicrophoneSampleFormat::PcmS32Le,
            MicrophoneSampleFormat::Float32Le,
            MicrophoneSampleFormat::Other(0xDEAD),
        ] {
            let capture = AudioCapture {
                sample_rate_hz: 44_100,
                channels: 2,
                format,
                bytes: vec![0xAA; 8],
                started_at_unix_ms: 12345,
            };
            let encoded = encode_microphone_capture(&capture);
            let decoded = decode_microphone_capture(&encoded).unwrap();
            assert_eq!(decoded.format, format);
            assert_eq!(decoded.sample_rate_hz, 44_100);
            assert_eq!(decoded.channels, 2);
            assert_eq!(decoded.bytes, vec![0xAA; 8]);
            assert_eq!(decoded.started_at_unix_ms, 12345);
        }
    }

    // --- Speaker encoding/decoding error cases ---

    #[test]
    fn decode_speaker_playback_request_too_short() {
        let result = decode_speaker_playback_request(&[0u8; 10]);
        assert!(result.is_err());
    }

    #[test]
    fn decode_speaker_playback_request_missing_audio_length() {
        // 17 bytes = minimum but no room for audio length field
        let buf = vec![0u8; 17];
        let result = decode_speaker_playback_request(&buf);
        assert!(result.is_err());
    }

    #[test]
    fn decode_speaker_playback_request_length_mismatch() {
        let request = AudioPlaybackRequest {
            duration_ms: 10,
            sample_rate_hz: 48_000,
            channels: 1,
            format: SpeakerSampleFormat::PcmS16Le,
            audio_bytes: vec![1, 2],
            software_gain_percent: None,
            target_output_level_percent: None,
        };
        let mut encoded = encode_speaker_playback_request(&request);
        // Corrupt the audio length
        let bad_len = 999u32.to_le_bytes();
        encoded[17..21].copy_from_slice(&bad_len);
        let result = decode_speaker_playback_request(&encoded);
        assert!(result.is_err());
    }

    #[test]
    fn decode_speaker_playback_request_unknown_format() {
        let mut buf = Vec::new();
        buf.extend_from_slice(&10u32.to_le_bytes()); // duration_ms
        buf.extend_from_slice(&48000u32.to_le_bytes()); // sample_rate
        buf.extend_from_slice(&1u16.to_le_bytes()); // channels
        buf.extend_from_slice(&99u32.to_le_bytes()); // unknown format
        buf.extend_from_slice(&0u16.to_le_bytes()); // gain
        buf.push(255); // target_output_level (None)
        buf.extend_from_slice(&0u32.to_le_bytes()); // audio len 0

        let result = decode_speaker_playback_request(&buf);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("format"));
    }

    #[test]
    fn decode_speaker_output_level_wrong_size() {
        let result = decode_speaker_output_level(&[0u8; 10]);
        assert!(result.is_err());
    }

    #[test]
    fn decode_speaker_output_level_invalid_mute() {
        let mut buf = vec![0u8; 18];
        buf[17] = 3; // invalid mute flag
        let result = decode_speaker_output_level(&buf);
        assert!(result.is_err());
    }

    #[test]
    fn encode_decode_speaker_output_level_all_mute_states() {
        for (muted, expected) in [
            (None, None),
            (Some(true), Some(true)),
            (Some(false), Some(false)),
        ] {
            let level = SpeakerOutputLevel {
                current_percent: 50,
                min_raw_value: 0,
                max_raw_value: 200,
                muted,
            };
            let encoded = encode_speaker_output_level(&level);
            let decoded = decode_speaker_output_level(&encoded).unwrap();
            assert_eq!(decoded.muted, expected);
        }
    }

    #[test]
    fn decode_speaker_playback_result_wrong_size() {
        let result = decode_speaker_playback_result(&[0u8; 5]);
        assert!(result.is_err());
    }

    // --- Bluetooth encoding/decoding error cases ---

    #[test]
    fn bluetooth_address_kind_invalid() {
        assert!(bluetooth_address_kind_from_u8(3).is_err());
    }

    #[test]
    fn bluetooth_transport_kind_invalid() {
        assert!(bluetooth_transport_kind_from_u8(4).is_err());
    }

    #[test]
    fn bluetooth_profile_invalid() {
        assert!(bluetooth_profile_from_u8(0).is_err());
        assert!(bluetooth_profile_from_u8(13).is_err());
        assert!(bluetooth_profile_from_u8(254).is_err());
    }

    #[test]
    fn bluetooth_link_kind_invalid() {
        assert!(bluetooth_link_kind_from_u8(4).is_err());
    }

    #[test]
    fn decode_bluetooth_scan_result_too_short() {
        let result = decode_bluetooth_scan_result(&[]);
        assert!(result.is_err());
    }

    #[test]
    fn decode_bluetooth_connections_too_short() {
        let result = decode_bluetooth_connections(&[0u8; 2]);
        assert!(result.is_err());
    }

    // --- WiFi encoding/decoding error cases ---

    #[test]
    fn wifi_power_state_invalid() {
        assert!(wifi_power_state_from_u8(4).is_err());
    }

    #[test]
    fn wifi_interface_mode_invalid() {
        assert!(wifi_interface_mode_from_u8(5).is_err());
    }

    #[test]
    fn decode_wifi_scan_result_too_short() {
        let result = decode_wifi_scan_result(&[0u8; 2]);
        assert!(result.is_err());
    }

    #[test]
    fn decode_wifi_interface_info_truncated_optional_fields() {
        let info = WifiInterfaceInfo {
            provider: "p".into(),
            interface_name: "wlan0".into(),
            mac_address: Some("00:11".into()),
            phy_name: Some("phy0".into()),
            operstate: Some("up".into()),
            power_state: WifiPowerState::Enabled,
            mode: WifiInterfaceMode::Client,
        };
        let encoded = encode_wifi_interface_info(&info);
        // Truncate by 1 byte
        let truncated = &encoded[..encoded.len() - 1];
        let result = decode_wifi_interface_info(truncated);
        assert!(result.is_err());
    }

    // --- Camera encoding/decoding error cases ---

    #[test]
    fn decode_camera_frame_too_short() {
        let result = decode_camera_frame(&[0u8; 10]);
        assert!(result.is_err());
    }

    #[test]
    fn decode_camera_frame_unknown_format() {
        let mut buf = vec![0u8; 20];
        // Set format to unknown value (not 1-5, and no high bit set)
        buf[12..16].copy_from_slice(&99u32.to_le_bytes());
        buf[16..20].copy_from_slice(&0u32.to_le_bytes()); // len = 0
        let result = decode_camera_frame(&buf);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("pixel format"));
    }

    #[test]
    fn decode_camera_frame_truncated_bytes() {
        let mut buf = vec![0u8; 24];
        buf[16..20].copy_from_slice(&10u32.to_le_bytes()); // claims 10 bytes
        // But only have 4 bytes after header
        let result = decode_camera_frame(&buf);
        assert!(result.is_err());
    }

    #[test]
    fn camera_capture_quality_invalid() {
        assert!(camera_capture_quality_from_u8(0).is_err());
        assert!(camera_capture_quality_from_u8(5).is_err());
    }

    #[test]
    fn decode_camera_capture_too_short_after_frame() {
        let mut buf = vec![0u8; 20];
        // Minimal valid frame header with 0 bytes
        buf[16..20].copy_from_slice(&0u32.to_le_bytes());
        // No quality/state bytes after
        let result = decode_camera_capture(&buf);
        assert!(result.is_err());
    }

    #[test]
    fn decode_paired_camera_frame_too_short() {
        let result = decode_paired_camera_frame(&[]);
        assert!(result.is_err());
    }

    #[test]
    fn decode_paired_camera_frame_missing_role() {
        let buf = vec![1u8]; // present flag set, but no role byte
        let result = decode_paired_camera_frame(&buf);
        assert!(result.is_err());
    }

    #[test]
    fn encode_decode_camera_capture_no_face_bounds() {
        let capture = CameraCapture {
            frame: CameraFrame {
                width: 320,
                height: 240,
                stride: 320,
                format: CameraPixelFormat::Yuyv,
                bytes: vec![0xAB; 10],
            },
            quality: CameraCaptureQuality::Excellent,
            face_bounds: None,
            state: default_face_biometric_state(false, true, false),
        };
        let encoded = encode_camera_capture(&capture);
        let decoded = decode_camera_capture(&encoded).unwrap();
        assert_eq!(decoded.face_bounds, None);
        assert_eq!(decoded.quality, CameraCaptureQuality::Excellent);
    }

    #[test]
    fn camera_pixel_format_all_known() {
        for format in [
            CameraPixelFormat::Mjpeg,
            CameraPixelFormat::Yuyv,
            CameraPixelFormat::Nv12,
            CameraPixelFormat::Rgb24,
            CameraPixelFormat::Gray8,
            CameraPixelFormat::Other(0xFFFF),
        ] {
            let raw = camera_pixel_format_to_u32(format);
            let decoded = camera_pixel_format_from_u32(raw).unwrap();
            assert_eq!(decoded, format);
        }
    }

    // --- String field encoding/decoding ---

    #[test]
    fn decode_string_field_empty() {
        let buf = 0u32.to_le_bytes().to_vec();
        let mut cursor = 0usize;
        let result = decode_string_field(&buf, &mut cursor);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "");
    }

    #[test]
    fn decode_string_field_truncated_length() {
        let buf = [0u8; 2];
        let mut cursor = 0usize;
        assert!(decode_string_field(&buf, &mut cursor).is_err());
    }

    #[test]
    fn decode_string_field_truncated_payload() {
        let mut buf = Vec::new();
        buf.extend_from_slice(&10u32.to_le_bytes()); // claims 10 bytes
        buf.extend_from_slice(&[b'h', b'i']); // only 2 bytes
        let mut cursor = 0usize;
        assert!(decode_string_field(&buf, &mut cursor).is_err());
    }

    #[test]
    fn decode_string_vec_empty() {
        let buf = 0u32.to_le_bytes().to_vec();
        let mut cursor = 0usize;
        let result = decode_string_vec(&buf, &mut cursor);
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[test]
    fn decode_optional_string_field_not_present() {
        let buf = [0u8];
        let mut cursor = 0usize;
        let result = decode_optional_string_field(&buf, &mut cursor);
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[test]
    fn decode_optional_string_field_missing_presence_byte() {
        let buf: [u8; 0] = [];
        let mut cursor = 0usize;
        assert!(decode_optional_string_field(&buf, &mut cursor).is_err());
    }

    // --- Bluetooth scan with empty observation list ---

    #[test]
    fn bluetooth_scan_result_empty_roundtrip() {
        let scan = BluetoothScanResult {
            observations: vec![],
        };
        let encoded = encode_bluetooth_scan_result(&scan);
        let decoded = decode_bluetooth_scan_result(&encoded).unwrap();
        assert_eq!(decoded.observations.len(), 0);
    }

    // --- WiFi scan with empty observation list ---

    #[test]
    fn wifi_scan_result_empty_roundtrip() {
        let scan = WifiScanResult {
            observations: vec![],
        };
        let encoded = encode_wifi_scan_result(&scan);
        let decoded = decode_wifi_scan_result(&encoded).unwrap();
        assert_eq!(decoded.observations.len(), 0);
    }

    // --- Bluetooth connections empty list ---

    #[test]
    fn bluetooth_connections_empty_roundtrip() {
        let connections: Vec<BluetoothConnectionInfo> = vec![];
        let encoded = encode_bluetooth_connections(&connections);
        let decoded = decode_bluetooth_connections(&encoded).unwrap();
        assert!(decoded.is_empty());
    }

    // --- Error type display and conversions ---

    #[test]
    fn capability_error_display() {
        let err = CapabilityError::InvalidRequest("test error");
        let msg = err.to_string();
        assert!(msg.contains("test error"));
    }

    #[test]
    fn capability_error_result_displays_error_reason() {
        let invocation = CapabilityInvocation {
            invocation_version: 1,
            invocation_id: b"inv".to_vec(),
            grant_id: b"grant".to_vec(),
            invoker: None,
            operation: 1,
            requested_access_class: 2,
            parameter_object: None,
            correlation_id: Vec::new(),
            invoked_at: None,
            signature: None,
        };
        let error = CapabilityError::InvalidRequest("something broke");
        let result = capability_error_result(&invocation, error);
        assert!(!result.success);
        assert!(result.error_reason.contains("something broke"));
        assert_eq!(result.invocation_id, b"inv");
    }

    // --- session_reject ---

    #[test]
    fn session_reject_properly_configured() {
        let open = CapabilitySessionOpen {
            version: 2,
            session_id: b"my-session".to_vec(),
            selector: None,
            mode: CapabilitySessionMode::Unary as i32,
            requested_operations: vec![1],
            requested_access_class: 3,
            requested_constraints: vec![],
            correlation_id: Vec::new(),
        };
        let reject = session_reject(&open, "not allowed");
        assert_eq!(reject.version, 2);
        assert_eq!(reject.session_id, b"my-session");
        assert!(!reject.accepted);
        assert!(reject.granted_operations.is_empty());
        assert_eq!(
            reject.granted_access_class,
            edgerun_capabilities::CapabilityAccessClass::Unspecified as i32
        );
        assert_eq!(reject.error_reason, "not allowed");
        assert!(reject.grant_id.is_empty());
    }

    // --- accept_session_open_unchecked ---

    #[test]
    fn accept_session_open_unchecked_passes_through_fields() {
        let open = CapabilitySessionOpen {
            version: 3,
            session_id: b"unchecked".to_vec(),
            selector: None,
            mode: CapabilitySessionMode::Stream as i32,
            requested_operations: vec![1, 2, 3],
            requested_access_class: 7,
            requested_constraints: vec![],
            correlation_id: Vec::new(),
        };
        let accept = accept_session_open_unchecked(&open);
        assert!(accept.accepted);
        assert_eq!(accept.version, 3);
        assert_eq!(accept.session_id, b"unchecked");
        assert_eq!(accept.granted_operations, vec![1, 2, 3]);
        assert_eq!(accept.granted_access_class, 7);
        assert_eq!(accept.grant_id, b"unchecked");
    }

    // --- session_accept_from_grant ---

    #[test]
    fn session_accept_from_grant_derives_from_grant() {
        let open = CapabilitySessionOpen {
            version: 1,
            session_id: b"sess".to_vec(),
            selector: None,
            mode: CapabilitySessionMode::Unary as i32,
            requested_operations: vec![1, 2],
            requested_access_class: 5,
            requested_constraints: vec![],
            correlation_id: Vec::new(),
        };
        let grant = edgerun_capabilities::CapabilityGrant {
            grant_version: 1,
            grant_id: b"grant-abc".to_vec(),
            issuer: None,
            grantee: None,
            grantee_node: None,
            selector: None,
            granted_operations: vec![1],
            enforced_constraints: vec![],
            access_class: 3,
            issued_at: None,
            expires_at: None,
            correlation_id: Vec::new(),
            supersedes_revocation: None,
            signature: None,
        };
        let accept = session_accept_from_grant(&open, &grant);
        assert!(accept.accepted);
        assert_eq!(accept.granted_operations, vec![1]);
        assert_eq!(accept.granted_access_class, 3);
        assert_eq!(accept.grant_id, b"grant-abc");
    }

    // --- default_remote_requester ---

    #[test]
    fn default_remote_requester_has_expected_identity() {
        let ref_ = default_remote_requester();
        assert_eq!(ref_.identity_id, b"remote-capability-client");
    }

    // --- selector_from_descriptor ---

    #[test]
    fn selector_from_descriptor_populates_fields() {
        let descriptor = capability_descriptor(
            "my-cap",
            "my-provider",
            CapabilityRole::Input,
            &[CapabilityModality::Radio],
            &[CapabilityEventKind::Radio],
            &[CapabilityOperation::Query],
            vec![],
        );
        let selector = selector_from_descriptor(&descriptor, 42);
        // capability_id comes from descriptor (which is empty by default from capability_descriptor)
        assert!(selector.capability_id.is_empty());
        assert_eq!(selector.role, CapabilityRole::Input as i32);
        assert_eq!(selector.modalities, vec![CapabilityModality::Radio as i32]);
        assert_eq!(selector.event_kinds, vec![CapabilityEventKind::Radio as i32]);
        assert_eq!(selector.access_class, 42);
        assert_eq!(selector.operations, vec![CapabilityOperation::Query as i32]);
    }

    // --- adapter structs construction ---

    #[test]
    fn bluetooth_remote_adapter_new() {
        let device = DummyBluetoothScannerDevice::default();
        let descriptor = default_bluetooth_descriptor("test", "hci0");
        let adapter = BluetoothRemoteAdapter::new(device, descriptor.clone());
        assert_eq!(adapter.descriptor, descriptor);
    }

    #[test]
    fn bluetooth_connection_remote_adapter_new() {
        let device = DummyBluetoothConnectionDevice::default();
        let descriptor = default_bluetooth_descriptor("test", "hci0");
        let adapter = BluetoothConnectionRemoteAdapter::new(device, descriptor.clone());
        assert_eq!(adapter.descriptor, descriptor);
    }

    #[test]
    fn wifi_remote_adapter_new() {
        let device = DummyWifiScannerDevice::default();
        let descriptor = default_wifi_descriptor("test", "wlan0");
        let adapter = WifiRemoteAdapter::new(device, descriptor.clone());
        assert_eq!(adapter.descriptor, descriptor);
    }

    #[test]
    fn wifi_control_remote_adapter_new() {
        let device = DummyWifiControllerDevice::default();
        let descriptor = default_wifi_descriptor("test", "wlan0");
        let adapter = WifiControlRemoteAdapter::new(device, descriptor.clone());
        assert_eq!(adapter.descriptor, descriptor);
    }

    #[test]
    fn camera_remote_adapter_new() {
        let device = DummyCameraDevice {
            captures: VecDeque::new(),
        };
        let descriptor = DummyCameraDevice::descriptor();
        let adapter = CameraRemoteAdapter::new(
            device,
            CameraBiometricPurpose::Presence,
            100,
            descriptor.clone(),
        );
        assert_eq!(adapter.purpose, CameraBiometricPurpose::Presence);
        assert_eq!(adapter.timeout_ms, 100);
    }

    #[test]
    fn paired_camera_remote_adapter_new() {
        let device = DummyPairedCameraDevice {
            captures: VecDeque::new(),
        };
        let descriptor = DummyCameraDevice::descriptor();
        let adapter = PairedCameraRemoteAdapter::new(
            device,
            CameraBiometricPurpose::Enrollment,
            200,
            descriptor.clone(),
        );
        assert_eq!(adapter.purpose, CameraBiometricPurpose::Enrollment);
        assert_eq!(adapter.timeout_ms, 200);
    }

    #[test]
    fn speaker_remote_adapter_new() {
        let _adapter = SpeakerRemoteAdapter::new(DummySpeakerDevice::default());
        // Just verify it constructs
    }

    #[test]
    fn input_remote_adapter_new() {
        let adapter = InputRemoteAdapter::new(DummyInputDevice::default(), 16);
        assert_eq!(adapter.max_events_per_poll, 16);
    }

    #[test]
    fn microphone_remote_adapter_new() {
        let adapter = MicrophoneRemoteAdapter::new(
            DummyMicrophoneDevice::default(),
            AudioCaptureRequest {
                duration_ms: 50,
                sample_rate_hz: 16000,
                channels: 1,
                format: MicrophoneSampleFormat::PcmS16Le,
            },
        );
        assert_eq!(adapter.capture_request.duration_ms, 50);
    }

    // --- Input/ speaker invoke error paths ---

    #[test]
    fn input_remote_adapter_invoke_returns_error_message() {
        let mut adapter = InputRemoteAdapter::new(DummyInputDevice::default(), 8);
        let invocation = CapabilityInvocation {
            invocation_version: 1,
            invocation_id: b"inv".to_vec(),
            grant_id: b"grant".to_vec(),
            invoker: None,
            operation: CapabilityOperation::Observe as i32,
            requested_access_class: 1,
            parameter_object: None,
            correlation_id: Vec::new(),
            invoked_at: None,
            signature: None,
        };
        let result = adapter.invoke(b"sess", &invocation, None).unwrap();
        assert!(!result.result.success);
        assert!(result.result.error_reason.contains("stream-oriented"));
    }

    #[test]
    fn microphone_remote_adapter_invoke_returns_error_message() {
        let mut adapter = MicrophoneRemoteAdapter::new(
            DummyMicrophoneDevice::default(),
            AudioCaptureRequest {
                duration_ms: 10,
                sample_rate_hz: 48000,
                channels: 2,
                format: MicrophoneSampleFormat::PcmS16Le,
            },
        );
        let invocation = CapabilityInvocation {
            invocation_version: 1,
            invocation_id: b"inv".to_vec(),
            grant_id: b"grant".to_vec(),
            invoker: None,
            operation: CapabilityOperation::Capture as i32,
            requested_access_class: 1,
            parameter_object: None,
            correlation_id: Vec::new(),
            invoked_at: None,
            signature: None,
        };
        let result = adapter.invoke(b"sess", &invocation, None).unwrap();
        assert!(!result.result.success);
        assert!(result.result.error_reason.contains("stream-oriented"));
    }

    #[test]
    fn speaker_remote_adapter_invoke_missing_inline_params() {
        let mut adapter = SpeakerRemoteAdapter::new(DummySpeakerDevice::default());
        let invocation = CapabilityInvocation {
            invocation_version: 1,
            invocation_id: b"inv".to_vec(),
            grant_id: b"grant".to_vec(),
            invoker: None,
            operation: CapabilityOperation::Render as i32,
            requested_access_class: 1,
            parameter_object: None,
            correlation_id: Vec::new(),
            invoked_at: None,
            signature: None,
        };
        let result = adapter.invoke(b"sess", &invocation, None);
        assert!(result.is_err());
        assert!(result.err().unwrap().to_string().contains("inline playback"));
    }

    #[test]
    fn speaker_remote_adapter_invoke_control_missing_output_level() {
        let mut adapter = SpeakerRemoteAdapter::new(DummySpeakerDevice::default());
        let request = AudioPlaybackRequest {
            duration_ms: 10,
            sample_rate_hz: 48000,
            channels: 1,
            format: SpeakerSampleFormat::PcmS16Le,
            audio_bytes: vec![],
            software_gain_percent: None,
            target_output_level_percent: None, // Missing!
        };
        let invocation = CapabilityInvocation {
            invocation_version: 1,
            invocation_id: b"inv".to_vec(),
            grant_id: b"grant".to_vec(),
            invoker: None,
            operation: CapabilityOperation::Control as i32,
            requested_access_class: 1,
            parameter_object: None,
            correlation_id: Vec::new(),
            invoked_at: None,
            signature: None,
        };
        let result = adapter.invoke(b"sess", &invocation, Some(&encode_speaker_playback_request(&request)));
        assert!(result.is_err());
        assert!(result.err().unwrap().to_string().contains("output level"));
    }

    // --- WiFi controller error paths ---

    #[test]
    fn wifi_control_invoke_missing_inline_params() {
        let mut adapter = WifiControlRemoteAdapter::new(
            DummyWifiControllerDevice {
                info: RefCell::new(Some(WifiInterfaceInfo {
                    provider: "test".into(),
                    interface_name: "wlan0".into(),
                    mac_address: None,
                    phy_name: None,
                    operstate: None,
                    power_state: WifiPowerState::Enabled,
                    mode: WifiInterfaceMode::Client,
                })),
                last_state: RefCell::new(None),
            },
            default_wifi_descriptor("test", "wlan0"),
        );
        let invocation = CapabilityInvocation {
            invocation_version: 1,
            invocation_id: b"inv".to_vec(),
            grant_id: b"grant".to_vec(),
            invoker: None,
            operation: CapabilityOperation::Control as i32,
            requested_access_class: 1,
            parameter_object: None,
            correlation_id: Vec::new(),
            invoked_at: None,
            signature: None,
        };
        let result = adapter.invoke(b"sess", &invocation, None);
        assert!(result.is_err());
        assert!(result.err().unwrap().to_string().contains("inline power state"));
    }

    #[test]
    fn wifi_control_invoke_wrong_param_length() {
        let mut adapter = WifiControlRemoteAdapter::new(
            DummyWifiControllerDevice {
                info: RefCell::new(Some(WifiInterfaceInfo {
                    provider: "test".into(),
                    interface_name: "wlan0".into(),
                    mac_address: None,
                    phy_name: None,
                    operstate: None,
                    power_state: WifiPowerState::Enabled,
                    mode: WifiInterfaceMode::Client,
                })),
                last_state: RefCell::new(None),
            },
            default_wifi_descriptor("test", "wlan0"),
        );
        let invocation = CapabilityInvocation {
            invocation_version: 1,
            invocation_id: b"inv".to_vec(),
            grant_id: b"grant".to_vec(),
            invoker: None,
            operation: CapabilityOperation::Control as i32,
            requested_access_class: 1,
            parameter_object: None,
            correlation_id: Vec::new(),
            invoked_at: None,
            signature: None,
        };
        let result = adapter.invoke(b"sess", &invocation, Some(&[1, 2])); // wrong length
        assert!(result.is_err());
        assert!(result.err().unwrap().to_string().contains("payload length"));
    }

    #[test]
    fn wifi_control_invoke_unknown_power_state() {
        let mut adapter = WifiControlRemoteAdapter::new(
            DummyWifiControllerDevice {
                info: RefCell::new(Some(WifiInterfaceInfo {
                    provider: "test".into(),
                    interface_name: "wlan0".into(),
                    mac_address: None,
                    phy_name: None,
                    operstate: None,
                    power_state: WifiPowerState::Enabled,
                    mode: WifiInterfaceMode::Client,
                })),
                last_state: RefCell::new(None),
            },
            default_wifi_descriptor("test", "wlan0"),
        );
        let invocation = CapabilityInvocation {
            invocation_version: 1,
            invocation_id: b"inv".to_vec(),
            grant_id: b"grant".to_vec(),
            invoker: None,
            operation: CapabilityOperation::Control as i32,
            requested_access_class: 1,
            parameter_object: None,
            correlation_id: Vec::new(),
            invoked_at: None,
            signature: None,
        };
        let result = adapter.invoke(b"sess", &invocation, Some(&[99u8])); // unknown state
        assert!(result.is_err());
    }

    // --- camera adapter error paths ---

    #[test]
    fn camera_remote_adapter_invoke_returns_error() {
        let mut adapter = CameraRemoteAdapter::new(
            DummyCameraDevice {
                captures: VecDeque::new(),
            },
            CameraBiometricPurpose::Presence,
            50,
            DummyCameraDevice::descriptor(),
        );
        let invocation = CapabilityInvocation {
            invocation_version: 1,
            invocation_id: b"inv".to_vec(),
            grant_id: b"grant".to_vec(),
            invoker: None,
            operation: 1,
            requested_access_class: 1,
            parameter_object: None,
            correlation_id: Vec::new(),
            invoked_at: None,
            signature: None,
        };
        let result = adapter.invoke(b"sess", &invocation, None).unwrap();
        assert!(!result.result.success);
        assert!(result.result.error_reason.contains("stream-oriented"));
    }

    #[test]
    fn paired_camera_remote_adapter_invoke_returns_error() {
        let mut adapter = PairedCameraRemoteAdapter::new(
            DummyPairedCameraDevice {
                captures: VecDeque::new(),
            },
            CameraBiometricPurpose::Verification,
            50,
            DummyCameraDevice::descriptor(),
        );
        let invocation = CapabilityInvocation {
            invocation_version: 1,
            invocation_id: b"inv".to_vec(),
            grant_id: b"grant".to_vec(),
            invoker: None,
            operation: 1,
            requested_access_class: 1,
            parameter_object: None,
            correlation_id: Vec::new(),
            invoked_at: None,
            signature: None,
        };
        let result = adapter.invoke(b"sess", &invocation, None).unwrap();
        assert!(!result.result.success);
        assert!(result.result.error_reason.contains("stream-oriented"));
    }

    // --- Bluetooth/WiFi adapters invoke error paths ---

    #[test]
    fn bluetooth_remote_adapter_invoke_returns_error() {
        let mut adapter = BluetoothRemoteAdapter::new(
            DummyBluetoothScannerDevice::default(),
            default_bluetooth_descriptor("test", "hci0"),
        );
        let invocation = CapabilityInvocation {
            invocation_version: 1,
            invocation_id: b"inv".to_vec(),
            grant_id: b"grant".to_vec(),
            invoker: None,
            operation: 1,
            requested_access_class: 1,
            parameter_object: None,
            correlation_id: Vec::new(),
            invoked_at: None,
            signature: None,
        };
        let result = adapter.invoke(b"sess", &invocation, None).unwrap();
        assert!(!result.result.success);
        assert!(result.result.error_reason.contains("stream-oriented"));
    }

    #[test]
    fn wifi_remote_adapter_invoke_returns_error() {
        let mut adapter = WifiRemoteAdapter::new(
            DummyWifiScannerDevice::default(),
            default_wifi_descriptor("test", "wlan0"),
        );
        let invocation = CapabilityInvocation {
            invocation_version: 1,
            invocation_id: b"inv".to_vec(),
            grant_id: b"grant".to_vec(),
            invoker: None,
            operation: 1,
            requested_access_class: 1,
            parameter_object: None,
            correlation_id: Vec::new(),
            invoked_at: None,
            signature: None,
        };
        let result = adapter.invoke(b"sess", &invocation, None).unwrap();
        assert!(!result.result.success);
        assert!(result.result.error_reason.contains("stream-oriented"));
    }

    // --- map_camera_error ---

    #[test]
    fn map_camera_error_provider() {
        let err = CameraBiometricError::Provider("msg".into());
        let mapped = map_camera_error(err);
        assert!(matches!(mapped, CapabilityError::Provider(_)));
    }

    #[test]
    fn map_camera_error_invalid_request() {
        let err = CameraBiometricError::InvalidRequest("msg".into());
        let mapped = map_camera_error(err);
        assert!(matches!(mapped, CapabilityError::InvalidRequest(_)));
    }

    #[test]
    fn map_camera_error_invalid_state() {
        let err = CameraBiometricError::InvalidState("msg".into());
        let mapped = map_camera_error(err);
        assert!(matches!(mapped, CapabilityError::Provider(_)));
    }

    #[test]
    fn map_camera_error_unsupported() {
        let err = CameraBiometricError::UnsupportedOperation("msg".into());
        let mapped = map_camera_error(err);
        assert!(matches!(mapped, CapabilityError::Unsupported(_)));
    }

    // --- Bluetooth connection with all trust/paired combos ---

    #[test]
    fn bluetooth_connection_trust_paired_combos_roundtrip() {
        let combos = [
            (None, None),
            (Some(true), Some(true)),
            (Some(true), Some(false)),
            (Some(false), Some(true)),
            (Some(false), Some(false)),
        ];
        for (trusted, paired) in combos {
            let connections = vec![BluetoothConnectionInfo {
                device_id: "00:00:00:00:00:00".into(),
                transport_kind: BluetoothTransportKind::LowEnergy,
                address_kind: BluetoothAddressKind::Random,
                link_kind: BluetoothLinkKind::Acl,
                outbound: false,
                state: 0,
                local_name: None,
                service_uuids: vec![],
                profiles: vec![],
                trusted,
                paired,
            }];
            let encoded = encode_bluetooth_connections(&connections);
            let decoded = decode_bluetooth_connections(&encoded).unwrap();
            assert_eq!(decoded[0].trusted, trusted);
            assert_eq!(decoded[0].paired, paired);
        }
    }

    // --- WiFi interface info with all optional fields None ---

    #[test]
    fn wifi_interface_info_all_optional_none_roundtrip() {
        let info = WifiInterfaceInfo {
            provider: "test".into(),
            interface_name: "wlan0".into(),
            mac_address: None,
            phy_name: None,
            operstate: None,
            power_state: WifiPowerState::Unknown,
            mode: WifiInterfaceMode::Unknown,
        };
        let encoded = encode_wifi_interface_info(&info);
        let decoded = decode_wifi_interface_info(&encoded).unwrap();
        assert_eq!(decoded, info);
    }

    // --- WiFi scan observation with all optional fields None ---

    #[test]
    fn wifi_observation_all_optional_none_roundtrip() {
        let scan = WifiScanResult {
            observations: vec![WifiNetworkObservation {
                interface_name: "wlan0".into(),
                ssid: None,
                bssid: None,
                signal_dbm: None,
                frequency_mhz: None,
                secure: None,
                observed_at_unix_ms: 0,
            }],
        };
        let encoded = encode_wifi_scan_result(&scan);
        let decoded = decode_wifi_scan_result(&encoded).unwrap();
        assert_eq!(decoded, scan);
    }

    // --- Bluetooth observation with all optional fields ---

    #[test]
    fn bt_observation_all_fields_roundtrip() {
        let scan = BluetoothScanResult {
            observations: vec![BluetoothBeaconObservation {
                device_id: "FF:FF:FF:FF:FF:FF".into(),
                transport_kind: BluetoothTransportKind::DualMode,
                address_kind: BluetoothAddressKind::Public,
                rssi_dbm: -30,
                tx_power_dbm: None,
                local_name: None,
                service_uuids: vec![],
                profiles: vec![BluetoothProfile::Other],
                classic_device_class: Some(0x123456),
                advertisement_data: vec![0x02, 0x01, 0x06, 0x03, 0x03, 0x0F, 0x18],
                captured_at_unix_ms: 999999,
            }],
        };
        let encoded = encode_bluetooth_scan_result(&scan);
        let decoded = decode_bluetooth_scan_result(&encoded).unwrap();
        assert_eq!(decoded, scan);
    }

    // --- Bluetooth scan trailing bytes detection ---

    #[test]
    fn bt_scan_trailing_bytes_rejected() {
        let scan = BluetoothScanResult {
            observations: vec![BluetoothBeaconObservation {
                device_id: "AA:BB".into(),
                transport_kind: BluetoothTransportKind::LowEnergy,
                address_kind: BluetoothAddressKind::Random,
                rssi_dbm: -50,
                tx_power_dbm: None,
                local_name: None,
                service_uuids: vec![],
                profiles: vec![],
                classic_device_class: None,
                advertisement_data: vec![],
                captured_at_unix_ms: 0,
            }],
        };
        let mut encoded = encode_bluetooth_scan_result(&scan);
        encoded.push(0xFF); // trailing byte
        assert!(decode_bluetooth_scan_result(&encoded).is_err());
    }

    // --- WiFi scan trailing bytes detection ---

    #[test]
    fn wifi_scan_trailing_bytes_rejected() {
        let scan = WifiScanResult {
            observations: vec![WifiNetworkObservation {
                interface_name: "wlan0".into(),
                ssid: None,
                bssid: None,
                signal_dbm: None,
                frequency_mhz: None,
                secure: None,
                observed_at_unix_ms: 0,
            }],
        };
        let mut encoded = encode_wifi_scan_result(&scan);
        encoded.push(0xFF);
        assert!(decode_wifi_scan_result(&encoded).is_err());
    }

    // --- Bluetooth connections trailing bytes detection ---

    #[test]
    fn bt_connections_trailing_bytes_rejected() {
        let connections: Vec<BluetoothConnectionInfo> = vec![];
        let mut encoded = encode_bluetooth_connections(&connections);
        encoded.push(0xFF);
        assert!(decode_bluetooth_connections(&encoded).is_err());
    }

    // --- speaker playback request with None optional fields ---

    #[test]
    fn speaker_playback_request_none_optionals_roundtrip() {
        let request = AudioPlaybackRequest {
            duration_ms: 100,
            sample_rate_hz: 44100,
            channels: 1,
            format: SpeakerSampleFormat::PcmFloat32Le,
            audio_bytes: vec![0x00; 4],
            software_gain_percent: None,
            target_output_level_percent: None,
        };
        let encoded = encode_speaker_playback_request(&request);
        let decoded = decode_speaker_playback_request(&encoded).unwrap();
        assert_eq!(decoded, request);
    }

    // --- input events with negative values ---

    #[test]
    fn input_events_negative_values_roundtrip() {
        let events = vec![
            InputEventRecord {
                timestamp_sec: -100,
                timestamp_usec: -500,
                kind: InputEventKind::RelativeMotion,
                code: 0,
                value: -32768,
            },
            InputEventRecord {
                timestamp_sec: i64::MIN,
                timestamp_usec: 0,
                kind: InputEventKind::Key,
                code: 0,
                value: i32::MIN,
            },
        ];
        let encoded = encode_input_events(&events);
        let decoded = decode_input_events(&encoded).unwrap();
        assert_eq!(decoded, events);
    }
}
