//! Consolidated test suite for remote capability protocol.
//!
//! Reduced from 60 tests → 32 meaningful tests:
//! - 8 protocol roundtrip tests (session lifecycle, invocation, revocation)
//! - 11 adapter event streaming tests
//! - 7 payload encode/decode roundtrip tests
//! - 4 error path tests
//! - 2 policy wrapper tests

use std::cell::RefCell;
use std::collections::VecDeque;
use std::os::unix::net::UnixStream;
use std::rc::Rc;

use edgerun_bluetooth::{
    BluetoothAddressKind, BluetoothBeaconObservation, BluetoothConnectionInfo,
    BluetoothConnectionProvider, BluetoothLinkKind, BluetoothProfile, BluetoothScanResult,
    BluetoothScanner, BluetoothTransportKind,
};
use edgerun_camera_biometrics::{
    CameraBiometricError, CameraBiometricPurpose, CameraCapture, CameraCaptureQuality,
    CameraFrame, CameraPixelFormat, CameraReaderInfo, CameraStreamRole,
    FaceBounds, PairedCameraBiometricReader, PairedCameraFrame,
};
use edgerun_capabilities::{
    capability_descriptor, CapabilityAccessClass, CapabilityDescriptor, CapabilityError,
    CapabilityEventKind, CapabilityModality, CapabilityOperation, CapabilityRole,
    CapabilityRequest, CapabilitySelector,
};
use edgerun_input::{InputDeviceInfo, InputDeviceKind, InputEventKind, InputEventRecord};
use edgerun_microphone::{
    AudioCapture, AudioCaptureRequest, MicrophoneDevice, MicrophoneInfo, MicrophoneSampleFormat,
};
use edgerun_proto::edgerun::v0::capability::{
    CapabilityGrant, CapabilityInvocation, CapabilityRequest as ProtoRequest,
    CapabilityRevocation, CapabilityResult,
};
use edgerun_proto::edgerun::v0::capability_runtime::{
    capability_remote_envelope, CapabilityInvocationFrame, CapabilityRemoteEnvelope, CapabilitySessionClose,
    CapabilitySessionMode, CapabilitySessionOpen,
};
use edgerun_speaker::{
    AudioPlaybackRequest, AudioPlaybackResult, SpeakerDevice, SpeakerInfo, SpeakerOutputLevel,
    SpeakerSampleFormat,
};
use edgerun_wifi::{
    default_wifi_descriptor, WifiInterfaceInfo, WifiInterfaceMode, WifiNetworkObservation,
    WifiPowerState, WifiScanResult,
};

use crate::protocol::{
    accept_session_open_unchecked, default_remote_requester, RemoteCapabilityProvider,
    RemoteCapabilityTransport, RemoteInvocationResult,
};
use crate::transport::{FramedRemoteTransport, MemoryRemoteTransport};
use crate::policy::{IntoPolicyWrappedProvider, PolicyWrappedProvider};
use crate::adapters::{
    BluetoothRemoteAdapter, BluetoothConnectionRemoteAdapter,
    CameraRemoteAdapter, InputRemoteAdapter, MicrophoneRemoteAdapter,
    PairedCameraRemoteAdapter, SpeakerRemoteAdapter,
    WifiRemoteAdapter, WifiControlRemoteAdapter,
    encode_input_events, decode_input_events,
    encode_microphone_capture, decode_microphone_capture,
    encode_speaker_playback_request, decode_speaker_playback_request,
    encode_speaker_playback_result, decode_speaker_playback_result,
    encode_bluetooth_scan_result, decode_bluetooth_scan_result,
    encode_bluetooth_connections, decode_bluetooth_connections,
    encode_wifi_scan_result, decode_wifi_scan_result,
    encode_wifi_interface_info, decode_wifi_interface_info,
    encode_camera_capture, decode_camera_capture,
    encode_paired_camera_frame, decode_paired_camera_frame,
};
use crate::serve_one;
use crate::pump_one_event;
use crate::capability_error_result;
use edgerun_capability_policy::PolicyEngine;
use edgerun_biometrics::{BiometricState, BiometricModality};

// ============================================================================
// Dummy device implementations
// ============================================================================

#[derive(Default)]
struct DummyProvider {
    opened: bool,
    event_sent: bool,
}

impl RemoteCapabilityProvider for DummyProvider {
    fn descriptor(&self) -> CapabilityDescriptor {
        capability_descriptor(
            "dummy", "provider",
            CapabilityRole::Input, &[CapabilityModality::Text],
            &[CapabilityEventKind::Text], &[CapabilityOperation::Observe],
            Vec::new(),
        )
    }

    fn open_session(
        &mut self,
        open: &CapabilitySessionOpen,
    ) -> Result<edgerun_proto::edgerun::v0::capability_runtime::CapabilitySessionAccept, CapabilityError>
    {
        self.opened = true;
        let mut accept = accept_session_open_unchecked(open);
        accept.granted_access_class = CapabilityAccessClass::Derived as i32;
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
                result_access_class: CapabilityAccessClass::Derived as i32,
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
    ) -> Result<Option<edgerun_proto::edgerun::v0::capability_runtime::CapabilitySessionEvent>, CapabilityError>
    {
        if self.event_sent {
            return Ok(None);
        }
        self.event_sent = true;
        Ok(Some(edgerun_proto::edgerun::v0::capability_runtime::CapabilitySessionEvent {
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
            "dummy-input", "event0",
            CapabilityRole::Input,
            &[CapabilityModality::Touch, CapabilityModality::Text],
            &[CapabilityEventKind::Touch, CapabilityEventKind::Text, CapabilityEventKind::State],
            &[CapabilityOperation::Observe],
            Vec::new(),
        )
    }
}

impl edgerun_input::InputDevice for DummyInputDevice {
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

    fn read_events(&mut self, _max_events: usize) -> Result<Vec<InputEventRecord>, CapabilityError> {
        Ok(self.events.pop_front().unwrap_or_default())
    }
}

#[derive(Default)]
struct DummyMicrophoneDevice {
    captures: VecDeque<AudioCapture>,
}

impl edgerun_capabilities::CapabilityProvider for DummyMicrophoneDevice {
    fn descriptor(&self) -> CapabilityDescriptor {
        capability_descriptor(
            "dummy-mic", "hw:0,0",
            CapabilityRole::Input, &[CapabilityModality::Auditory],
            &[CapabilityEventKind::Auditory], &[CapabilityOperation::Capture],
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

    fn capture_audio(&mut self, _request: &AudioCaptureRequest) -> Result<AudioCapture, CapabilityError> {
        self.captures.pop_front()
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
            "dummy-speaker", "hw:0,0",
            CapabilityRole::Output, &[CapabilityModality::Auditory],
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

    fn set_output_level(&self, percent: u8) -> Result<Option<SpeakerOutputLevel>, CapabilityError> {
        *self.last_set_level.borrow_mut() = Some(percent);
        let mut level = self.output_level.borrow_mut();
        if let Some(current) = level.as_mut() {
            current.current_percent = percent;
        } else {
            *level = Some(SpeakerOutputLevel {
                current_percent: percent, min_raw_value: 0, max_raw_value: 100, muted: Some(false),
            });
        }
        Ok(level.clone())
    }

    fn play_audio(&self, request: &AudioPlaybackRequest) -> Result<AudioPlaybackResult, CapabilityError> {
        *self.last_request.borrow_mut() = Some(request.clone());
        self.result.borrow().clone()
            .ok_or_else(|| CapabilityError::Provider("no dummy playback result queued".into()))
    }
}

#[derive(Default)]
struct DummyBluetoothScannerDevice {
    scans: RefCell<VecDeque<BluetoothScanResult>>,
}

impl edgerun_capabilities::CapabilityProvider for DummyBluetoothScannerDevice {
    fn descriptor(&self) -> CapabilityDescriptor {
        edgerun_bluetooth::default_bluetooth_descriptor("dummy-bt", "hci0")
    }
}

impl BluetoothScanner for DummyBluetoothScannerDevice {
    fn scan_nearby(&self) -> Result<BluetoothScanResult, CapabilityError> {
        self.scans.borrow_mut().pop_front()
            .ok_or_else(|| CapabilityError::Provider("no dummy bt scan queued".into()))
    }
}

#[derive(Default)]
struct DummyBluetoothConnectionDevice {
    connections: RefCell<Vec<BluetoothConnectionInfo>>,
}

impl edgerun_capabilities::CapabilityProvider for DummyBluetoothConnectionDevice {
    fn descriptor(&self) -> CapabilityDescriptor {
        edgerun_bluetooth::default_bluetooth_descriptor("dummy-bt", "hci0")
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

impl edgerun_wifi::WifiScanner for DummyWifiScannerDevice {
    fn scan_nearby(&self) -> Result<WifiScanResult, CapabilityError> {
        self.scans.borrow_mut().pop_front()
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

impl edgerun_wifi::WifiController for DummyWifiControllerDevice {
    fn interface_info(&self) -> Result<WifiInterfaceInfo, CapabilityError> {
        self.info.borrow().clone()
            .ok_or_else(|| CapabilityError::Provider("no dummy wifi info configured".into()))
    }

    fn power_state(&self) -> Result<WifiPowerState, CapabilityError> {
        Ok(self.last_state.borrow()
            .or_else(|| self.info.borrow().as_ref().map(|v| v.power_state))
            .unwrap_or(WifiPowerState::Unknown))
    }

    fn set_power_state(&self, state: WifiPowerState) -> Result<WifiPowerState, CapabilityError> {
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

impl edgerun_capabilities::CapabilityProvider for DummyCameraDevice {
    fn descriptor(&self) -> CapabilityDescriptor {
        capability_descriptor(
            "dummy-camera", "video0",
            CapabilityRole::Input, &[CapabilityModality::Visual],
            &[CapabilityEventKind::Visual], &[CapabilityOperation::Observe],
            Vec::new(),
        )
    }
}

impl edgerun_camera_biometrics::CameraBiometricReader for DummyCameraDevice {
    fn reader_info(&self) -> Result<CameraReaderInfo, CameraBiometricError> {
        Ok(CameraReaderInfo {
            provider: "dummy-camera".into(), reader_name: "Dummy Camera".into(),
            supports_face_detection: true, supports_face_matching: false,
            supports_liveness_detection: false, hardware_protected_match: false,
        })
    }

    fn capture(&mut self, _purpose: CameraBiometricPurpose, _timeout_ms: u32) -> Result<CameraCapture, CameraBiometricError> {
        self.captures.pop_front()
            .ok_or_else(|| CameraBiometricError::Provider("no dummy camera capture queued".into()))
    }

    fn begin_enrollment(&mut self, _request: &edgerun_camera_biometrics::CameraEnrollRequest) -> Result<edgerun_camera_biometrics::CameraEnrollmentSession, CameraBiometricError> {
        Err(CameraBiometricError::UnsupportedOperation("dummy"))
    }
    fn enroll_step(&mut self, _session_id: &str, _capture: &CameraCapture) -> Result<edgerun_camera_biometrics::CameraEnrollProgress, CameraBiometricError> {
        Err(CameraBiometricError::UnsupportedOperation("dummy"))
    }
    fn finish_enrollment(&mut self, _session_id: &str) -> Result<edgerun_camera_biometrics::CameraTemplateRecord, CameraBiometricError> {
        Err(CameraBiometricError::UnsupportedOperation("dummy"))
    }
    fn verify_capture(&mut self, _request: &edgerun_camera_biometrics::CameraVerifyRequest, _capture: &CameraCapture) -> Result<edgerun_camera_biometrics::CameraVerification, CameraBiometricError> {
        Err(CameraBiometricError::UnsupportedOperation("dummy"))
    }
    fn list_templates(&self) -> Result<Vec<edgerun_camera_biometrics::CameraTemplateRecord>, CameraBiometricError> {
        Ok(Vec::new())
    }
    fn delete_template(&mut self, _template_id: &str) -> Result<(), CameraBiometricError> {
        Err(CameraBiometricError::UnsupportedOperation("dummy"))
    }
}

struct DummyPairedCameraDevice {
    captures: VecDeque<PairedCameraFrame>,
}

impl PairedCameraBiometricReader for DummyPairedCameraDevice {
    fn supported_stream_roles(&self) -> Result<Vec<CameraStreamRole>, CameraBiometricError> {
        Ok(vec![CameraStreamRole::Rgb, CameraStreamRole::Infrared])
    }

    fn capture_paired(&mut self, _purpose: CameraBiometricPurpose, _timeout_ms: u32) -> Result<PairedCameraFrame, CameraBiometricError> {
        self.captures.pop_front()
            .ok_or_else(|| CameraBiometricError::Provider("no dummy paired camera capture queued".into()))
    }

    fn evaluate_liveness(&mut self, _challenge: &edgerun_camera_biometrics::CameraLivenessChallenge, _capture: &PairedCameraFrame) -> Result<edgerun_camera_biometrics::CameraLivenessResult, CameraBiometricError> {
        Err(CameraBiometricError::UnsupportedOperation("dummy"))
    }
}

impl edgerun_camera_biometrics::CameraBiometricReader for DummyPairedCameraDevice {
    fn reader_info(&self) -> Result<CameraReaderInfo, CameraBiometricError> {
        Ok(CameraReaderInfo {
            provider: "dummy-paired-camera".into(), reader_name: "Dummy Paired Camera".into(),
            supports_face_detection: true, supports_face_matching: false,
            supports_liveness_detection: true, hardware_protected_match: false,
        })
    }

    fn capture(&mut self, _purpose: CameraBiometricPurpose, _timeout_ms: u32) -> Result<CameraCapture, CameraBiometricError> {
        Err(CameraBiometricError::UnsupportedOperation("use paired capture"))
    }

    fn begin_enrollment(&mut self, _request: &edgerun_camera_biometrics::CameraEnrollRequest) -> Result<edgerun_camera_biometrics::CameraEnrollmentSession, CameraBiometricError> {
        Err(CameraBiometricError::UnsupportedOperation("dummy"))
    }
    fn enroll_step(&mut self, _session_id: &str, _capture: &CameraCapture) -> Result<edgerun_camera_biometrics::CameraEnrollProgress, CameraBiometricError> {
        Err(CameraBiometricError::UnsupportedOperation("dummy"))
    }
    fn finish_enrollment(&mut self, _session_id: &str) -> Result<edgerun_camera_biometrics::CameraTemplateRecord, CameraBiometricError> {
        Err(CameraBiometricError::UnsupportedOperation("dummy"))
    }
    fn verify_capture(&mut self, _request: &edgerun_camera_biometrics::CameraVerifyRequest, _capture: &CameraCapture) -> Result<edgerun_camera_biometrics::CameraVerification, CameraBiometricError> {
        Err(CameraBiometricError::UnsupportedOperation("dummy"))
    }
    fn list_templates(&self) -> Result<Vec<edgerun_camera_biometrics::CameraTemplateRecord>, CameraBiometricError> {
        Ok(Vec::new())
    }
    fn delete_template(&mut self, _template_id: &str) -> Result<(), CameraBiometricError> {
        Err(CameraBiometricError::UnsupportedOperation("dummy"))
    }
}

// ============================================================================
// Protocol roundtrip tests
// ============================================================================

#[test]
fn loopback_open_and_invoke_roundtrip() {
    let (mut client, mut server) = MemoryRemoteTransport::pair();
    let mut provider = PolicyWrappedProvider::new(DummyProvider::default());

    client.send(CapabilityRemoteEnvelope {
        message: Some(capability_remote_envelope::Message::SessionOpen(CapabilitySessionOpen {
            version: 1, session_id: b"sess".to_vec(), selector: None,
            mode: CapabilitySessionMode::Unary as i32,
            requested_operations: vec![CapabilityOperation::Observe as i32],
            requested_access_class: CapabilityAccessClass::Derived as i32,
            requested_constraints: Vec::new(), correlation_id: Vec::new(),
        })),
    }).unwrap();
    assert!(serve_one(&mut provider, &mut server).unwrap());
    let accept = client.recv().unwrap().unwrap();
    let grant_id = match accept.message {
        Some(capability_remote_envelope::Message::SessionAccept(a)) => {
            assert!(a.accepted);
            assert_eq!(a.session_id, b"sess".to_vec());
            a.grant_id
        }
        other => panic!("unexpected: {other:?}"),
    };

    client.send(CapabilityRemoteEnvelope {
        message: Some(capability_remote_envelope::Message::Invocation(CapabilityInvocation {
            invocation_version: 1, invocation_id: b"inv".to_vec(),
            grant_id, invoker: None,
            operation: CapabilityOperation::Observe as i32,
            requested_access_class: CapabilityAccessClass::Derived as i32,
            parameter_object: None, correlation_id: Vec::new(),
            invoked_at: None, signature: None,
        })),
    }).unwrap();
    assert!(serve_one(&mut provider, &mut server).unwrap());
    let result = client.recv().unwrap().unwrap();
    match result.message {
        Some(capability_remote_envelope::Message::Result(r)) => {
            assert!(r.success);
            assert_eq!(r.invocation_id, b"inv".to_vec());
        }
        other => panic!("unexpected: {other:?}"),
    }
}

#[test]
fn policy_wrapper_denies_remote_raw_session() {
    let mut provider = PolicyWrappedProvider::new(DummyProvider::default());
    let accept = provider.open_session(&CapabilitySessionOpen {
        version: 1, session_id: b"sess".to_vec(), selector: None,
        mode: CapabilitySessionMode::Unary as i32,
        requested_operations: vec![CapabilityOperation::Observe as i32],
        requested_access_class: CapabilityAccessClass::Raw as i32,
        requested_constraints: Vec::new(), correlation_id: Vec::new(),
    }).unwrap();
    assert!(!accept.accepted);
    assert_eq!(accept.error_reason, "raw capability access is only allowed locally");
}

#[test]
fn session_open_stores_grant_via_unified_request_path() {
    let mut provider = PolicyWrappedProvider::new(DummyProvider::default());
    let accept = provider.open_session(&CapabilitySessionOpen {
        version: 1, session_id: b"sess".to_vec(), selector: None,
        mode: CapabilitySessionMode::Unary as i32,
        requested_operations: vec![CapabilityOperation::Observe as i32],
        requested_access_class: CapabilityAccessClass::Derived as i32,
        requested_constraints: Vec::new(), correlation_id: Vec::new(),
    }).unwrap();
    assert!(accept.accepted);
    let binding = provider.sessions().get(&accept.grant_id).unwrap();
    assert_eq!(binding.session_id, b"sess");
    assert!(provider.policy().grant_record(&binding.grant_id).is_some());
}

#[test]
fn end_to_end_session_revocation_denies_later_invocation() {
    let (mut client, mut server) = MemoryRemoteTransport::pair();
    let mut provider = PolicyWrappedProvider::new(DummyProvider::default());

    client.send(CapabilityRemoteEnvelope {
        message: Some(capability_remote_envelope::Message::SessionOpen(CapabilitySessionOpen {
            version: 1, session_id: b"sess".to_vec(), selector: None,
            mode: CapabilitySessionMode::Unary as i32,
            requested_operations: vec![CapabilityOperation::Observe as i32],
            requested_access_class: CapabilityAccessClass::Derived as i32,
            requested_constraints: Vec::new(), correlation_id: Vec::new(),
        })),
    }).unwrap();
    assert!(serve_one(&mut provider, &mut server).unwrap());
    let accept_env = client.recv().unwrap().unwrap();
    let grant_id = match accept_env.message {
        Some(capability_remote_envelope::Message::SessionOpen(_)) => unreachable!(),
        Some(capability_remote_envelope::Message::SessionAccept(ref a)) => a.grant_id.clone(),
        _ => panic!("expected SessionAccept"),
    };

    let revocation = CapabilityRevocation {
        revocation_version: 1, revocation_id: b"rev-1".to_vec(),
        grant_id: grant_id.clone(), issuer: None, effective_at: None,
        reason: "test".into(), replacement_constraints: Vec::new(), signature: None,
    };
    client.send(CapabilityRemoteEnvelope {
        message: Some(capability_remote_envelope::Message::Revocation(revocation)),
    }).unwrap();
    assert!(serve_one(&mut provider, &mut server).unwrap());

    client.send(CapabilityRemoteEnvelope {
        message: Some(capability_remote_envelope::Message::Invocation(CapabilityInvocation {
            invocation_version: 1, invocation_id: b"inv-after-revoke".to_vec(),
            grant_id, invoker: None,
            operation: CapabilityOperation::Observe as i32,
            requested_access_class: CapabilityAccessClass::Derived as i32,
            parameter_object: None, correlation_id: Vec::new(),
            invoked_at: None, signature: None,
        })),
    }).unwrap();
    assert!(serve_one(&mut provider, &mut server).unwrap());
    let result = client.recv().unwrap().unwrap();
    match result.message {
        Some(capability_remote_envelope::Message::Result(r)) => {
            assert!(!r.success);
            assert!(r.error_reason.contains("revoked"));
        }
        other => panic!("unexpected: {other:?}"),
    }
}

#[test]
fn inbound_grant_and_revocation_are_applied() {
    let mut provider = PolicyWrappedProvider::new(DummyProvider::default());
    let descriptor = provider.descriptor();
    let selector = CapabilitySelector {
        capability_id: descriptor.capability_id.clone(),
        role: descriptor.role,
        modalities: descriptor.modalities.clone(),
        event_kinds: descriptor.event_kinds.clone(),
        operations: descriptor.operations.clone(),
        access_class: CapabilityAccessClass::Derived as i32,
        provider_identity: descriptor.provider_identity.clone(),
        provider_node: descriptor.provider_node.clone(),
        provider_instance_id: descriptor.provider_instance_id.clone(),
    };
    let request = ProtoRequest {
        request_version: 1, request_id: b"sess".to_vec(),
        requester: Some(default_remote_requester()), requester_node: None,
        selector: Some(selector),
        requested_operations: vec![CapabilityOperation::Observe as i32],
        requested_constraints: Vec::new(), purpose: "test".into(),
        requested_duration: None, correlation_id: Vec::new(), signature: None,
    };
    let grant = match provider.handle_request(&request).unwrap() {
        Some(grant) => grant,
        None => panic!("expected grant"),
    };
    provider.handle_grant(&grant).unwrap();
    assert!(provider.policy().grant_record(&grant.grant_id).is_some());

    let revocation = CapabilityRevocation {
        revocation_version: 1, revocation_id: b"rev-1".to_vec(),
        grant_id: grant.grant_id.clone(), issuer: None, effective_at: None,
        reason: "test".into(), replacement_constraints: Vec::new(), signature: None,
    };
    provider.handle_revocation(&revocation).unwrap();
    assert!(provider.policy().grant_record(&grant.grant_id).unwrap().is_revoked());
}

#[test]
fn loopback_stream_event_roundtrip() {
    let (mut client, mut server) = MemoryRemoteTransport::pair();
    let mut provider = DummyProvider::default();
    assert!(pump_one_event(&mut provider, &mut server, b"stream").unwrap());
    let event = client.recv().unwrap().unwrap();
    match event.message {
        Some(capability_remote_envelope::Message::SessionEvent(e)) => {
            assert_eq!(e.session_id, b"stream".to_vec());
            assert_eq!(e.inline_payload, b"hello".to_vec());
        }
        other => panic!("unexpected: {other:?}"),
    }
}

#[test]
fn serve_one_eof_returns_false() {
    let (_, mut server) = MemoryRemoteTransport::pair();
    let mut provider = DummyProvider::default();
    let result = serve_one(&mut provider, &mut server).unwrap();
    assert!(!result);
}

#[test]
fn serve_one_invocation_frame_missing_invocation() {
    let (mut client, mut server) = MemoryRemoteTransport::pair();
    let mut provider = DummyProvider::default();

    client.send(CapabilityRemoteEnvelope {
        message: Some(capability_remote_envelope::Message::InvocationFrame(CapabilityInvocationFrame {
            invocation: None, inline_parameters: Vec::new(),
        })),
    }).unwrap();

    let result = serve_one(&mut provider, &mut server);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("invocation"));
}

// ============================================================================
// Adapter event streaming tests
// ============================================================================

#[test]
fn input_remote_adapter_streams_events() {
    let mut device = DummyInputDevice::default();
    device.events.push_back(vec![InputEventRecord {
        timestamp_sec: 1, timestamp_usec: 0, kind: InputEventKind::Key, code: 30, value: 1,
    }]);
    let mut adapter = InputRemoteAdapter::new(device, 64);
    let open = CapabilitySessionOpen {
        version: 1, session_id: b"s".to_vec(), selector: None,
        mode: CapabilitySessionMode::Unary as i32,
        requested_operations: vec![CapabilityOperation::Observe as i32],
        requested_access_class: CapabilityAccessClass::Derived as i32,
        requested_constraints: Vec::new(), correlation_id: Vec::new(),
    };
    assert!(adapter.open_session(&open).unwrap().accepted);

    let event = adapter.next_event(b"s").unwrap().unwrap();
    assert_eq!(event.inline_payload.len(), 4 + 24); // count + 1 event
    assert_eq!(adapter.next_event(b"s").unwrap(), None);
}

#[test]
fn microphone_remote_adapter_streams_capture() {
    let mut device = DummyMicrophoneDevice::default();
    device.captures.push_back(AudioCapture {
        sample_rate_hz: 48_000, channels: 2, format: MicrophoneSampleFormat::PcmS16Le,
        started_at_unix_ms: 0, bytes: vec![0u8; 64],
    });
    let mut adapter = MicrophoneRemoteAdapter::new(device, AudioCaptureRequest {
        sample_rate_hz: 48_000, channels: 2, format: MicrophoneSampleFormat::PcmS16Le,
        duration_ms: 100,
    });
    let open = CapabilitySessionOpen {
        version: 1, session_id: b"s".to_vec(), selector: None,
        mode: CapabilitySessionMode::Unary as i32,
        requested_operations: vec![CapabilityOperation::Capture as i32],
        requested_access_class: CapabilityAccessClass::Derived as i32,
        requested_constraints: Vec::new(), correlation_id: Vec::new(),
    };
    assert!(adapter.open_session(&open).unwrap().accepted);
    let event = adapter.next_event(b"s").unwrap().unwrap();
    assert!(!event.inline_payload.is_empty());
}

#[test]
fn speaker_remote_adapter_unary_render_roundtrip() {
    let mut device = DummySpeakerDevice::default();
    *device.result.borrow_mut() = Some(AudioPlaybackResult {
        bytes_written: 64, sample_rate_hz: 48_000, channels: 2, finished: true,
    });
    let mut adapter = SpeakerRemoteAdapter::new(device);
    let open = CapabilitySessionOpen {
        version: 1, session_id: b"s".to_vec(), selector: None,
        mode: CapabilitySessionMode::Unary as i32,
        requested_operations: vec![CapabilityOperation::Render as i32],
        requested_access_class: CapabilityAccessClass::Derived as i32,
        requested_constraints: Vec::new(), correlation_id: Vec::new(),
    };
    assert!(adapter.open_session(&open).unwrap().accepted);

    let request = AudioPlaybackRequest {
        duration_ms: 100, sample_rate_hz: 48_000, channels: 2,
        format: SpeakerSampleFormat::PcmS16Le, audio_bytes: vec![0u8; 64],
        software_gain_percent: None, target_output_level_percent: None,
    };
    let params = encode_speaker_playback_request(&request);
    let invocation = CapabilityInvocation {
        invocation_version: 1, invocation_id: b"inv".to_vec(),
        grant_id: b"g".to_vec(), invoker: None,
        operation: CapabilityOperation::Render as i32,
        requested_access_class: CapabilityAccessClass::Derived as i32,
        parameter_object: None, correlation_id: Vec::new(),
        invoked_at: None, signature: None,
    };
    let result = adapter.invoke(b"g", &invocation, Some(&params)).unwrap();
    assert!(result.result.success);
    assert!(!result.inline_payload.is_empty());
}

#[test]
fn bluetooth_remote_adapter_streams_scan() {
    let device = DummyBluetoothScannerDevice::default();
    device.scans.borrow_mut().push_back(BluetoothScanResult {
        observations: vec![BluetoothBeaconObservation {
            device_id: "dev1".into(), transport_kind: BluetoothTransportKind::LowEnergy,
            address_kind: BluetoothAddressKind::Public, rssi_dbm: -50,
            tx_power_dbm: None, local_name: Some("Test".into()),
            service_uuids: Vec::new(), profiles: Vec::new(),
            classic_device_class: None, advertisement_data: Vec::new(),
            captured_at_unix_ms: 0,
        }],
    });
    let mut adapter = BluetoothRemoteAdapter::new(
        device,
        edgerun_bluetooth::default_bluetooth_descriptor("dummy-bt", "hci0"),
    );
    let open = CapabilitySessionOpen {
        version: 1, session_id: b"s".to_vec(), selector: None,
        mode: CapabilitySessionMode::Unary as i32,
        requested_operations: vec![CapabilityOperation::Observe as i32],
        requested_access_class: CapabilityAccessClass::Derived as i32,
        requested_constraints: Vec::new(), correlation_id: Vec::new(),
    };
    assert!(adapter.open_session(&open).unwrap().accepted);
    let event = adapter.next_event(b"s").unwrap().unwrap();
    assert!(!event.inline_payload.is_empty());
}

#[test]
fn bluetooth_connection_remote_adapter_returns_connections() {
    let device = DummyBluetoothConnectionDevice::default();
    device.connections.borrow_mut().push(BluetoothConnectionInfo {
        device_id: "dev1".into(), transport_kind: BluetoothTransportKind::LowEnergy,
        address_kind: BluetoothAddressKind::Public, link_kind: BluetoothLinkKind::Acl,
        outbound: true, state: 1, local_name: Some("Test".into()),
        service_uuids: Vec::new(), profiles: Vec::new(),
        trusted: Some(true), paired: Some(true),
    });
    let mut adapter = BluetoothConnectionRemoteAdapter::new(
        device,
        edgerun_bluetooth::default_bluetooth_descriptor("dummy-bt", "hci0"),
    );
    let open = CapabilitySessionOpen {
        version: 1, session_id: b"s".to_vec(), selector: None,
        mode: CapabilitySessionMode::Unary as i32,
        requested_operations: vec![CapabilityOperation::Observe as i32],
        requested_access_class: CapabilityAccessClass::Derived as i32,
        requested_constraints: Vec::new(), correlation_id: Vec::new(),
    };
    assert!(adapter.open_session(&open).unwrap().accepted);
    let invocation = CapabilityInvocation {
        invocation_version: 1, invocation_id: b"inv".to_vec(),
        grant_id: b"g".to_vec(), invoker: None,
        operation: CapabilityOperation::Observe as i32,
        requested_access_class: CapabilityAccessClass::Derived as i32,
        parameter_object: None, correlation_id: Vec::new(),
        invoked_at: None, signature: None,
    };
    let result = adapter.invoke(b"g", &invocation, None).unwrap();
    assert!(result.result.success);
    assert!(!result.inline_payload.is_empty());
}

#[test]
fn wifi_remote_adapter_streams_scan() {
    let device = DummyWifiScannerDevice::default();
    device.scans.borrow_mut().push_back(WifiScanResult {
        observations: vec![WifiNetworkObservation {
            interface_name: "wlan0".into(), ssid: Some("TestNet".into()),
            bssid: Some("aa:bb:cc".into()), signal_dbm: Some(-60),
            frequency_mhz: Some(2437), secure: Some(true), observed_at_unix_ms: 0,
        }],
    });
    let mut adapter = WifiRemoteAdapter::new(
        device,
        default_wifi_descriptor("dummy-wifi", "wlan0"),
    );
    let open = CapabilitySessionOpen {
        version: 1, session_id: b"s".to_vec(), selector: None,
        mode: CapabilitySessionMode::Unary as i32,
        requested_operations: vec![CapabilityOperation::Observe as i32],
        requested_access_class: CapabilityAccessClass::Derived as i32,
        requested_constraints: Vec::new(), correlation_id: Vec::new(),
    };
    assert!(adapter.open_session(&open).unwrap().accepted);
    let event = adapter.next_event(b"s").unwrap().unwrap();
    assert!(!event.inline_payload.is_empty());
}

#[test]
fn wifi_control_remote_adapter_returns_interface_info() {
    let device = DummyWifiControllerDevice::default();
    device.info.borrow_mut().replace(WifiInterfaceInfo {
        provider: "dummy".into(), interface_name: "wlan0".into(),
        mac_address: Some("aa:bb".into()), phy_name: Some("phy0".into()),
        operstate: Some("up".into()), power_state: WifiPowerState::Enabled,
        mode: WifiInterfaceMode::Client,
    });
    let mut adapter = WifiControlRemoteAdapter::new(
        device,
        default_wifi_descriptor("dummy-wifi", "wlan0"),
    );
    let open = CapabilitySessionOpen {
        version: 1, session_id: b"s".to_vec(), selector: None,
        mode: CapabilitySessionMode::Unary as i32,
        requested_operations: vec![CapabilityOperation::Control as i32],
        requested_access_class: CapabilityAccessClass::Derived as i32,
        requested_constraints: Vec::new(), correlation_id: Vec::new(),
    };
    assert!(adapter.open_session(&open).unwrap().accepted);
    let invocation = CapabilityInvocation {
        invocation_version: 1, invocation_id: b"inv".to_vec(),
        grant_id: b"g".to_vec(), invoker: None,
        operation: CapabilityOperation::Control as i32,
        requested_access_class: CapabilityAccessClass::Derived as i32,
        parameter_object: None, correlation_id: Vec::new(),
        invoked_at: None, signature: None,
    };
    let result = adapter.invoke(b"g", &invocation, Some(&[1u8])).unwrap();
    assert!(result.result.success);
    assert!(!result.inline_payload.is_empty());
}

#[test]
fn camera_remote_adapter_streams_capture() {
    let mut device = DummyCameraDevice { captures: VecDeque::new() };
    device.captures.push_back(CameraCapture {
        frame: CameraFrame {
            width: 640, height: 480, stride: 640,
            format: CameraPixelFormat::Mjpeg, bytes: vec![0u8; 100],
        },
        quality: CameraCaptureQuality::Good,
        face_bounds: None,
        state: BiometricState {
            modality: None, verified: false, hardware_protected: false, user_present: false,
        },
    });
    let mut adapter = CameraRemoteAdapter::new(
        device, CameraBiometricPurpose::Presence, 1000,
        capability_descriptor(
            "dummy-camera", "video0",
            CapabilityRole::Input, &[CapabilityModality::Visual],
            &[CapabilityEventKind::Visual], &[CapabilityOperation::Observe],
            Vec::new(),
        ),
    );
    let open = CapabilitySessionOpen {
        version: 1, session_id: b"s".to_vec(), selector: None,
        mode: CapabilitySessionMode::Unary as i32,
        requested_operations: vec![CapabilityOperation::Observe as i32],
        requested_access_class: CapabilityAccessClass::Derived as i32,
        requested_constraints: Vec::new(), correlation_id: Vec::new(),
    };
    assert!(adapter.open_session(&open).unwrap().accepted);
    let event = adapter.next_event(b"s").unwrap().unwrap();
    assert!(!event.inline_payload.is_empty());
}

#[test]
fn paired_camera_remote_adapter_streams_capture() {
    let mut device = DummyPairedCameraDevice { captures: VecDeque::new() };
    device.captures.push_back(PairedCameraFrame {
        rgb: Some(CameraFrame {
            width: 640, height: 480, stride: 640,
            format: CameraPixelFormat::Mjpeg, bytes: vec![0u8; 100],
        }),
        infrared: None,
        depth: None,
    });
    let mut adapter = PairedCameraRemoteAdapter::new(
        device, CameraBiometricPurpose::Presence, 1000,
        capability_descriptor(
            "dummy-paired", "video0",
            CapabilityRole::Input, &[CapabilityModality::Visual],
            &[CapabilityEventKind::Visual], &[CapabilityOperation::Observe],
            Vec::new(),
        ),
    );
    let open = CapabilitySessionOpen {
        version: 1, session_id: b"s".to_vec(), selector: None,
        mode: CapabilitySessionMode::Unary as i32,
        requested_operations: vec![CapabilityOperation::Observe as i32],
        requested_access_class: CapabilityAccessClass::Derived as i32,
        requested_constraints: Vec::new(), correlation_id: Vec::new(),
    };
    assert!(adapter.open_session(&open).unwrap().accepted);
    let event = adapter.next_event(b"s").unwrap().unwrap();
    assert!(!event.inline_payload.is_empty());
}

// ============================================================================
// Payload encode/decode roundtrip tests
// ============================================================================

#[test]
fn input_event_payload_roundtrip() {
    let events = vec![
        InputEventRecord { timestamp_sec: 1, timestamp_usec: 0, kind: InputEventKind::Key, code: 30, value: 1 },
        InputEventRecord { timestamp_sec: 1, timestamp_usec: 500, kind: InputEventKind::RelativeMotion, code: 0, value: -5 },
    ];
    let encoded = encode_input_events(&events);
    let decoded = decode_input_events(&encoded).unwrap();
    assert_eq!(decoded.len(), 2);
    assert_eq!(decoded[0].kind, InputEventKind::Key);
    assert_eq!(decoded[1].value, -5);
}

#[test]
fn microphone_capture_payload_roundtrip() {
    let capture = AudioCapture {
        sample_rate_hz: 48_000, channels: 2, format: MicrophoneSampleFormat::PcmS16Le,
        started_at_unix_ms: 1_000_000, bytes: vec![0xAA; 128],
    };
    let encoded = encode_microphone_capture(&capture);
    let decoded = decode_microphone_capture(&encoded).unwrap();
    assert_eq!(decoded.sample_rate_hz, 48_000);
    assert_eq!(decoded.channels, 2);
    assert_eq!(decoded.format, MicrophoneSampleFormat::PcmS16Le);
    assert_eq!(decoded.bytes.len(), 128);
}

#[test]
fn speaker_playback_request_payload_roundtrip() {
    let request = AudioPlaybackRequest {
        duration_ms: 200, sample_rate_hz: 44_100, channels: 1,
        format: SpeakerSampleFormat::PcmFloat32Le,
        audio_bytes: vec![0xBB; 256],
        software_gain_percent: Some(50), target_output_level_percent: Some(75),
    };
    let encoded = encode_speaker_playback_request(&request);
    let decoded = decode_speaker_playback_request(&encoded).unwrap();
    assert_eq!(decoded.duration_ms, 200);
    assert_eq!(decoded.sample_rate_hz, 44_100);
    assert_eq!(decoded.channels, 1);
    assert_eq!(decoded.format, SpeakerSampleFormat::PcmFloat32Le);
    assert_eq!(decoded.audio_bytes.len(), 256);
    assert_eq!(decoded.software_gain_percent, Some(50));
    assert_eq!(decoded.target_output_level_percent, Some(75));
}

#[test]
fn speaker_playback_result_payload_roundtrip() {
    let result = AudioPlaybackResult {
        bytes_written: 1024, sample_rate_hz: 48_000, channels: 2, finished: true,
    };
    let encoded = encode_speaker_playback_result(&result);
    let decoded = decode_speaker_playback_result(&encoded).unwrap();
    assert_eq!(decoded.bytes_written, 1024);
    assert!(decoded.finished);
}

#[test]
fn bluetooth_scan_payload_roundtrip() {
    let scan = BluetoothScanResult {
        observations: vec![BluetoothBeaconObservation {
            device_id: "dev1".into(), transport_kind: BluetoothTransportKind::LowEnergy,
            address_kind: BluetoothAddressKind::Random, rssi_dbm: -70,
            tx_power_dbm: Some(-30), local_name: Some("My Device".into()),
            service_uuids: vec!["180f".into()], profiles: vec![BluetoothProfile::HeartRate],
            classic_device_class: None, advertisement_data: vec![0x02, 0x01, 0x06],
            captured_at_unix_ms: 1_700_000_000,
        }],
    };
    let encoded = encode_bluetooth_scan_result(&scan);
    let decoded = decode_bluetooth_scan_result(&encoded).unwrap();
    assert_eq!(decoded.observations.len(), 1);
    assert_eq!(decoded.observations[0].device_id, "dev1");
    assert_eq!(decoded.observations[0].rssi_dbm, -70);
}

#[test]
fn bluetooth_connections_payload_roundtrip() {
    let connections = vec![BluetoothConnectionInfo {
        device_id: "dev1".into(), transport_kind: BluetoothTransportKind::LowEnergy,
        address_kind: BluetoothAddressKind::Public, link_kind: BluetoothLinkKind::Acl,
        outbound: true, state: 1, local_name: Some("Connected".into()),
        service_uuids: Vec::new(), profiles: vec![BluetoothProfile::AudioSink],
        trusted: Some(true), paired: Some(false),
    }];
    let encoded = encode_bluetooth_connections(&connections);
    let decoded = decode_bluetooth_connections(&encoded).unwrap();
    assert_eq!(decoded.len(), 1);
    assert_eq!(decoded[0].device_id, "dev1");
    assert_eq!(decoded[0].trusted, Some(true));
}

#[test]
fn wifi_scan_payload_roundtrip() {
    let scan = WifiScanResult {
        observations: vec![WifiNetworkObservation {
            interface_name: "wlan0".into(), ssid: Some("TestNet".into()),
            bssid: Some("aa:bb:cc:dd".into()), signal_dbm: Some(-55),
            frequency_mhz: Some(5180), secure: Some(true), observed_at_unix_ms: 0,
        }],
    };
    let encoded = encode_wifi_scan_result(&scan);
    let decoded = decode_wifi_scan_result(&encoded).unwrap();
    assert_eq!(decoded.observations.len(), 1);
    assert_eq!(decoded.observations[0].ssid, Some("TestNet".into()));
}

#[test]
fn wifi_interface_info_payload_roundtrip() {
    let info = WifiInterfaceInfo {
        provider: "nl80211".into(), interface_name: "wlan0".into(),
        mac_address: Some("aa:bb".into()), phy_name: Some("phy0".into()),
        operstate: Some("up".into()), power_state: WifiPowerState::Enabled,
        mode: WifiInterfaceMode::Client,
    };
    let encoded = encode_wifi_interface_info(&info);
    let decoded = decode_wifi_interface_info(&encoded).unwrap();
    assert_eq!(decoded.interface_name, "wlan0");
    assert_eq!(decoded.power_state, WifiPowerState::Enabled);
}

#[test]
fn camera_capture_payload_roundtrip() {
    let capture = CameraCapture {
        frame: CameraFrame {
            width: 1920, height: 1080, stride: 1920,
            format: CameraPixelFormat::Nv12, bytes: vec![0xCC; 500],
        },
        quality: CameraCaptureQuality::Excellent,
        face_bounds: Some(FaceBounds { x: 100, y: 200, width: 300, height: 400 }),
        state: BiometricState {
            modality: Some(BiometricModality::Face),
            verified: true, hardware_protected: true, user_present: true,
        },
    };
    let encoded = encode_camera_capture(&capture);
    let decoded = decode_camera_capture(&encoded).unwrap();
    assert_eq!(decoded.frame.width, 1920);
    assert_eq!(decoded.frame.format, CameraPixelFormat::Nv12);
    assert_eq!(decoded.quality, CameraCaptureQuality::Excellent);
    assert!(decoded.face_bounds.is_some());
    assert!(decoded.state.verified);
}

#[test]
fn paired_camera_frame_payload_roundtrip() {
    let frame = PairedCameraFrame {
        rgb: Some(CameraFrame {
            width: 640, height: 480, stride: 640,
            format: CameraPixelFormat::Rgb24, bytes: vec![0xDD; 100],
        }),
        infrared: Some(CameraFrame {
            width: 640, height: 480, stride: 640,
            format: CameraPixelFormat::Gray8, bytes: vec![0xEE; 100],
        }),
        depth: None,
    };
    let encoded = encode_paired_camera_frame(&frame);
    let decoded = decode_paired_camera_frame(&encoded).unwrap();
    assert!(decoded.rgb.is_some());
    assert!(decoded.infrared.is_some());
    assert!(decoded.depth.is_none());
    assert_eq!(decoded.rgb.unwrap().format, CameraPixelFormat::Rgb24);
}

// ============================================================================
// Error path tests
// ============================================================================

#[test]
fn input_remote_adapter_invoke_returns_stream_error() {
    let mut adapter = InputRemoteAdapter::new(DummyInputDevice::default(), 64);
    let invocation = CapabilityInvocation {
        invocation_version: 1, invocation_id: b"inv".to_vec(),
        grant_id: b"g".to_vec(), invoker: None,
        operation: CapabilityOperation::Observe as i32,
        requested_access_class: CapabilityAccessClass::Derived as i32,
        parameter_object: None, correlation_id: Vec::new(),
        invoked_at: None, signature: None,
    };
    let result = adapter.invoke(b"g", &invocation, None).unwrap();
    assert!(!result.result.success);
    assert!(result.result.error_reason.contains("stream-oriented"));
}

#[test]
fn speaker_remote_adapter_invoke_missing_inline_params() {
    let mut adapter = SpeakerRemoteAdapter::new(DummySpeakerDevice::default());
    let open = CapabilitySessionOpen {
        version: 1, session_id: b"s".to_vec(), selector: None,
        mode: CapabilitySessionMode::Unary as i32,
        requested_operations: vec![CapabilityOperation::Render as i32],
        requested_access_class: CapabilityAccessClass::Derived as i32,
        requested_constraints: Vec::new(), correlation_id: Vec::new(),
    };
    adapter.open_session(&open).unwrap();
    let invocation = CapabilityInvocation {
        invocation_version: 1, invocation_id: b"inv".to_vec(),
        grant_id: b"g".to_vec(), invoker: None,
        operation: CapabilityOperation::Render as i32,
        requested_access_class: CapabilityAccessClass::Derived as i32,
        parameter_object: None, correlation_id: Vec::new(),
        invoked_at: None, signature: None,
    };
    let result = adapter.invoke(b"g", &invocation, None);
    assert!(result.is_err());
}

#[test]
fn decode_input_events_length_mismatch() {
    let events = vec![InputEventRecord {
        timestamp_sec: 1, timestamp_usec: 0, kind: InputEventKind::Key, code: 30, value: 1,
    }];
    let mut encoded = encode_input_events(&events);
    encoded[0..4].copy_from_slice(&2u32.to_le_bytes()); // claim 2 events
    let result = decode_input_events(&encoded);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("length does not match"));
}

#[test]
fn capability_error_result_displays_error_reason() {
    let invocation = CapabilityInvocation {
        invocation_version: 1, invocation_id: b"inv".to_vec(),
        grant_id: b"g".to_vec(), invoker: None,
        operation: CapabilityOperation::Observe as i32,
        requested_access_class: CapabilityAccessClass::Derived as i32,
        parameter_object: None, correlation_id: Vec::new(),
        invoked_at: None, signature: None,
    };
    let result = capability_error_result(&invocation, CapabilityError::PermissionDenied("nope".into()));
    assert!(!result.success);
    assert!(result.error_reason.contains("nope"));
}

// ============================================================================
// Policy wrapper tests
// ============================================================================

#[test]
fn adapter_can_be_wrapped_via_standard_trait() {
    let provider = DummyProvider::default().into_policy_wrapped();
    let context = provider.context();
    assert!(!context.is_local);
}

#[test]
fn policy_wrapped_close_session_revokes_grant() {
    let mut provider = PolicyWrappedProvider::new(DummyProvider::default());
    let accept = provider.open_session(&CapabilitySessionOpen {
        version: 1, session_id: b"sess".to_vec(), selector: None,
        mode: CapabilitySessionMode::Unary as i32,
        requested_operations: vec![CapabilityOperation::Observe as i32],
        requested_access_class: CapabilityAccessClass::Derived as i32,
        requested_constraints: Vec::new(), correlation_id: Vec::new(),
    }).unwrap();
    assert!(accept.accepted);
    let grant_id = accept.grant_id.clone();
    assert!(provider.policy().grant_record(&grant_id).is_some());

    provider.close_session(&CapabilitySessionClose {
        version: 1, session_id: b"sess".to_vec(), reason: String::new(),
    }).unwrap();

    let record = provider.policy().grant_record(&grant_id);
    assert!(record.is_some());
    assert!(record.unwrap().is_revoked());
}
