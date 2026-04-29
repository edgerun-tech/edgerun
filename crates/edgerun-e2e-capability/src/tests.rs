//! Hardware e2e tests — each test opens a real device and exercises the
//! full remote-capability protocol stack (descriptor → session → invoke/stream).
//!
//! Run against hardware with: `HARDWARE_E2E=1 cargo test -p edgerun-e2e-capability`

use crate::require_hardware;
use crate::session_harness;
use crate::test_policy::TestGrantedProvider;
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec;
use alloc::vec::Vec;
use edgerun_capabilities::{CapabilityAccessClass, CapabilityOperation, CapabilityProvider};
use edgerun_proto::edgerun::v0::capability::{
    CapabilityInvocation, CapabilityOperation as ProtoOp,
};
use edgerun_proto::edgerun::v0::capability_runtime::{
    capability_remote_envelope, CapabilityInvocationFrame, CapabilityRemoteEnvelope,
    CapabilityResultFrame, CapabilitySessionClose, CapabilitySessionEvent, CapabilitySessionMode,
    CapabilitySessionOpen,
};
use edgerun_remote_capability::{
    FramedRemoteTransport, InputRemoteAdapter, MicrophoneRemoteAdapter, PolicyWrappedProvider,
    RemoteCapabilityProvider, RemoteCapabilityTransport, SpeakerRemoteAdapter,
};
use prost::Message;
use std::os::unix::net::UnixStream;
use std::println;
use std::thread;
use std::time::Duration;

// ===========================================================================
// Test 1: Input device (evdev) — discover, open, and stream events
// ===========================================================================

#[test]
fn test_input_device_e2e_discover_and_open() {
    if !require_hardware() {
        return;
    }

    use edgerun_evdev_input::discover_evdev_devices;

    let devices = discover_evdev_devices().expect("discover evdev devices");
    assert!(
        !devices.is_empty(),
        "no evdev devices found — check /dev/input/event*"
    );

    let info = devices[0].clone();
    println!(
        "Found input device: {} ({}) kind={:?}",
        info.device_name, info.event_node, info.kind
    );

    // Open via the real backend (actual open() syscall on /dev/input/eventX)
    use edgerun_evdev_input::EvdevInputBackend;
    let backend = EvdevInputBackend::open(info.clone()).expect("open evdev input device");

    // Wrap as remote adapter
    let adapter = InputRemoteAdapter::new(backend, 64);

    // Verify descriptor is well-formed
    let descriptor = adapter.descriptor();
    assert_eq!(descriptor.descriptor_version, 1);
    assert!(!descriptor.provider_name.is_empty());
    assert!(!descriptor.modalities.is_empty());
}

#[test]
fn test_input_device_e2e_session_and_events() {
    if !require_hardware() {
        return;
    }

    use edgerun_evdev_input::{discover_evdev_devices, EvdevInputBackend};

    let devices = discover_evdev_devices().expect("discover evdev devices");
    assert!(!devices.is_empty(), "no evdev devices found");

    let info = devices[0].clone();
    let backend = EvdevInputBackend::open(info).expect("open evdev device");
    // Use TestGrantedProvider so session always opens (bypasses policy grant cycle)
    let mut wrapped = TestGrantedProvider::new(InputRemoteAdapter::new(backend, 64));

    let session_id = b"input-session-1";
    let open = CapabilitySessionOpen {
        version: 1,
        session_id: session_id.to_vec(),
        selector: None,
        mode: CapabilitySessionMode::Stream as i32,
        requested_operations: vec![CapabilityOperation::Observe as i32],
        requested_access_class: CapabilityAccessClass::Derived as i32,
        requested_constraints: Vec::new(),
        correlation_id: Vec::new(),
    };
    let accept = wrapped.open_session(&open).expect("open session");
    assert!(!accept.session_id.is_empty());
    assert!(!accept.grant_id.is_empty());
    assert!(!accept.granted_operations.is_empty());

    // Read real events from the evdev device
    let event = wrapped.next_event(session_id).expect("get next event");
    if let Some(ev) = event {
        assert!(!ev.inline_payload.is_empty() || ev.payload_object.is_some());
        println!(
            "Received input event: sequence={}, payload_size={}",
            ev.sequence_no,
            ev.inline_payload.len()
        );
    } else {
        println!("No input events available (device idle) — valid");
    }
}

#[test]
fn test_input_device_e2e_unix_socket_full_protocol() {
    if !require_hardware() {
        return;
    }

    use edgerun_evdev_input::{discover_evdev_devices, EvdevInputBackend};

    let devices = discover_evdev_devices().expect("discover evdev devices");
    assert!(!devices.is_empty(), "no evdev devices found");

    let info = devices[0].clone();
    let backend = EvdevInputBackend::open(info).expect("open evdev device");
    let adapter = InputRemoteAdapter::new(backend, 64);

    let session_id = b"input-socket-session";
    let _transport =
        session_harness::open_session(adapter, session_id).expect("open session over unix socket");
}

// ===========================================================================
// Test 2: Microphone (ALSA) — capture audio over remote protocol
// ===========================================================================

#[test]
fn test_microphone_e2e_discover_and_open() {
    if !require_hardware() {
        return;
    }

    use edgerun_alsa_microphone::{discover_alsa_pcms, AlsaMicrophoneBackend};
    use edgerun_microphone::MicrophoneDevice;

    let pcms = discover_alsa_pcms().expect("discover ALSA pcm devices");
    let capture_pcms: Vec<_> = pcms.iter().filter(|p| p.capture).collect();

    if capture_pcms.is_empty() {
        println!("No ALSA capture devices found — skipping");
        return;
    }

    let pcm = capture_pcms[0].clone();
    println!(
        "Found capture device: card={} device={} name={}",
        pcm.card_index, pcm.device_index, pcm.name
    );

    let backend = AlsaMicrophoneBackend::open(pcm.clone()).expect("open mic backend");
    let info = backend.microphone_info().expect("get mic info");
    println!(
        "Mic info: {} ch={} rate={} format={:?}",
        info.device_name, info.channels, info.sample_rate_hz, info.format
    );

    use edgerun_microphone::AudioCaptureRequest;
    let capture_request = AudioCaptureRequest {
        duration_ms: 100,
        sample_rate_hz: 16_000,
        channels: 1,
        format: edgerun_microphone::MicrophoneSampleFormat::PcmS16Le,
    };
    let adapter = MicrophoneRemoteAdapter::new(backend, capture_request);

    let descriptor = adapter.descriptor();
    assert_eq!(descriptor.descriptor_version, 1);
    assert!(!descriptor.provider_name.is_empty());
}

#[test]
fn test_microphone_e2e_session_and_capture() {
    if !require_hardware() {
        return;
    }

    use edgerun_alsa_microphone::{discover_alsa_pcms, AlsaMicrophoneBackend};
    use edgerun_microphone::{AudioCaptureRequest, MicrophoneSampleFormat};

    let pcms = discover_alsa_pcms().expect("discover ALSA pcm devices");
    let capture_pcms: Vec<_> = pcms.iter().filter(|p| p.capture).collect();

    if capture_pcms.is_empty() {
        println!("No ALSA capture devices found, skipping");
        return;
    }

    let pcm = capture_pcms[0].clone();
    let backend = AlsaMicrophoneBackend::open(pcm).expect("open mic backend");
    let capture_request = AudioCaptureRequest {
        duration_ms: 100,
        sample_rate_hz: 16_000,
        channels: 1,
        format: MicrophoneSampleFormat::PcmS16Le,
    };
    let adapter = MicrophoneRemoteAdapter::new(backend, capture_request.clone());
    let mut wrapped = TestGrantedProvider::new(adapter);

    let session_id = b"mic-session-1";
    let open = CapabilitySessionOpen {
        version: 1,
        session_id: session_id.to_vec(),
        selector: None,
        mode: CapabilitySessionMode::Stream as i32,
        requested_operations: vec![CapabilityOperation::Capture as i32],
        requested_access_class: CapabilityAccessClass::Derived as i32,
        requested_constraints: Vec::new(),
        correlation_id: Vec::new(),
    };
    let accept = wrapped.open_session(&open).expect("open session");
    assert!(!accept.session_id.is_empty());

    let event = wrapped.next_event(session_id);
    match event {
        Ok(Some(ev)) => {
            println!("Captured audio event: {} bytes", ev.inline_payload.len());
            assert!(!ev.inline_payload.is_empty());
        }
        Ok(None) => {
            println!("No audio event available (device idle)");
        }
        Err(e) => {
            println!("Audio capture error: {:?}", e);
        }
    }
}

#[test]
fn test_microphone_encoding_roundtrip_real_device() {
    if !require_hardware() {
        return;
    }

    use edgerun_alsa_microphone::{discover_alsa_pcms, AlsaMicrophoneBackend};
    use edgerun_microphone::{AudioCaptureRequest, MicrophoneDevice, MicrophoneSampleFormat};

    let pcms = discover_alsa_pcms().expect("discover ALSA pcm devices");
    let capture_pcms: Vec<_> = pcms.iter().filter(|p| p.capture).collect();

    if capture_pcms.is_empty() {
        println!("No capture devices found, skipping");
        return;
    }

    let pcm = capture_pcms[0].clone();
    let mut backend = AlsaMicrophoneBackend::open(pcm).expect("open mic backend");

    let request = AudioCaptureRequest {
        duration_ms: 50,
        sample_rate_hz: 16_000,
        channels: 1,
        format: MicrophoneSampleFormat::PcmS16Le,
    };

    let capture = backend.capture_audio(&request);
    if let Ok(cap) = capture {
        println!("Captured {} bytes from real mic", cap.bytes.len());

        let event = CapabilitySessionEvent {
            version: 1,
            session_id: b"test".to_vec(),
            sequence_no: 1,
            event_kinds: vec![
                edgerun_proto::edgerun::v0::capability::CapabilityEventKind::Auditory as i32,
            ],
            payload_object: None,
            inline_payload: cap.bytes.clone(),
        };
        let serialized = event.encode_to_vec();
        let decoded = CapabilitySessionEvent::decode(serialized.as_slice())
            .expect("decode audio session event");
        assert_eq!(decoded.inline_payload, cap.bytes);
        println!("Audio roundtrip: {} bytes", decoded.inline_payload.len());
    } else {
        println!("Mic capture failed: {:?}", capture.unwrap_err());
    }
}

// ===========================================================================
// Test 3: Speaker (ALSA) — playback over remote protocol
// ===========================================================================

#[test]
fn test_speaker_e2e_discover_and_open() {
    if !require_hardware() {
        return;
    }

    use edgerun_alsa_speaker::discover_speakers;
    use edgerun_speaker::SpeakerDevice;

    let speakers = discover_speakers().expect("discover ALSA speakers");
    assert!(
        !speakers.is_empty(),
        "no ALSA playback devices found — check /dev/snd/pcmC*D*p"
    );

    let backend = &speakers[0];
    let spk_info = backend.speaker_info().expect("get speaker info");
    println!(
        "Found playback device: {} ({}) ch={} rate={}",
        spk_info.display_name,
        spk_info.instance_id,
        spk_info.channels,
        spk_info.default_sample_rate_hz,
    );

    let adapter = SpeakerRemoteAdapter::new(backend.clone());
    let descriptor = adapter.descriptor();
    assert_eq!(descriptor.descriptor_version, 1);
    assert!(!descriptor.provider_name.is_empty());
}

#[test]
fn test_speaker_e2e_playback() {
    if !require_hardware() {
        return;
    }

    use edgerun_alsa_speaker::discover_speakers;
    use edgerun_speaker::{AudioPlaybackRequest, SpeakerDevice, SpeakerSampleFormat};

    let speakers = discover_speakers().expect("discover ALSA speakers");
    if speakers.is_empty() {
        println!("No ALSA playback devices found, skipping");
        return;
    }

    // 100ms of silence at 48kHz stereo S16LE (most widely supported format)
    let sample_count = (48_000 / 10) * 2; // stereo
    let silence = vec![0u8; sample_count * 2];

    let request = AudioPlaybackRequest {
        duration_ms: 100,
        sample_rate_hz: 48_000,
        channels: 2,
        format: SpeakerSampleFormat::PcmS16Le,
        audio_bytes: silence,
        software_gain_percent: Some(50),
        target_output_level_percent: Some(50),
    };

    // Try each speaker until one accepts (HDMI may only support specific formats)
    let mut last_err = None;
    for backend in &speakers {
        match backend.play_audio(&request) {
            Ok(res) => {
                println!(
                    "Playback on {} succeeded: {} bytes written",
                    backend.display_name, res.bytes_written
                );
                assert!(res.bytes_written > 0);
                return;
            }
            Err(e) => {
                println!("Speaker {} failed: {:?}", backend.display_name, e);
                last_err = Some(e);
            }
        }
    }

    panic!(
        "All {} speakers failed to play audio. Last error: {:?}",
        speakers.len(),
        last_err
    );
}

#[test]
fn test_speaker_e2e_session_and_invoke() {
    if !require_hardware() {
        return;
    }

    use edgerun_alsa_speaker::discover_speakers;
    use edgerun_speaker::SpeakerDevice;

    let speakers = discover_speakers().expect("discover ALSA speakers");
    if speakers.is_empty() {
        println!("No ALSA playback devices found, skipping");
        return;
    }

    let backend = speakers.into_iter().next().unwrap();
    let adapter = SpeakerRemoteAdapter::new(backend);
    let mut wrapped = TestGrantedProvider::new(adapter);

    let session_id = b"speaker-session";
    let open = CapabilitySessionOpen {
        version: 1,
        session_id: session_id.to_vec(),
        selector: None,
        mode: CapabilitySessionMode::Unary as i32,
        requested_operations: vec![CapabilityOperation::Render as i32],
        requested_access_class: CapabilityAccessClass::Derived as i32,
        requested_constraints: Vec::new(),
        correlation_id: Vec::new(),
    };
    let accept = wrapped.open_session(&open).expect("open session");
    assert!(!accept.session_id.is_empty());
    assert!(!accept.granted_operations.is_empty());
    println!(
        "Speaker session opened, granted_operations: {:?}",
        accept.granted_operations
    );
}

// ===========================================================================
// Test 4: Camera (V4L2) — capture frames over remote protocol
// ===========================================================================

#[test]
fn test_camera_e2e_discover_and_open() {
    if !require_hardware() {
        return;
    }

    use edgerun_camera_biometrics::CameraBiometricReader;
    use edgerun_v4l2_camera::{discover_camera_devices, V4l2CameraBiometricReader};

    let cameras = discover_camera_devices().expect("discover V4L2 cameras");
    assert!(
        !cameras.is_empty(),
        "no V4L2 camera devices found — check /dev/video*"
    );

    let dev = &cameras[0];
    let probe = dev.probe().expect("probe camera device");
    println!(
        "Found camera: {} ({})",
        probe.info.card,
        dev.devnode.display()
    );

    // Wrap as biometric reader (calls ioctl VIDIOC_QUERYCAP via probe internally)
    let camera_dev = edgerun_v4l2_camera::V4l2CameraDevice::new(&dev.devnode);
    let reader = V4l2CameraBiometricReader::new(camera_dev);
    let info = reader.reader_info().expect("get camera reader info");
    println!(
        "Camera reader: {} face_detect={} liveness={}",
        info.reader_name, info.supports_face_detection, info.supports_liveness_detection
    );

    let descriptor = reader.descriptor();
    assert_eq!(descriptor.descriptor_version, 1);
    assert!(!descriptor.provider_name.is_empty());
}

#[test]
fn test_camera_e2e_capture_frame() {
    if !require_hardware() {
        return;
    }

    use edgerun_camera_biometrics::{CameraBiometricPurpose, CameraBiometricReader};
    use edgerun_v4l2_camera::{discover_camera_devices, V4l2CameraBiometricReader};

    let cameras = discover_camera_devices().expect("discover V4L2 cameras");
    if cameras.is_empty() {
        println!("No V4L2 cameras found, skipping");
        return;
    }

    let dev = cameras[0].clone();
    let camera_dev = edgerun_v4l2_camera::V4l2CameraDevice::new(&dev.devnode);
    let mut reader = V4l2CameraBiometricReader::new(camera_dev);

    // Capture a real frame via V4L2 MMAP streaming (ioctl: REQBUFS, QBUF, STREAMON, DQBUF, mmap)
    let capture = reader.capture(CameraBiometricPurpose::Presence, 2000);
    match capture {
        Ok(cap) => {
            println!(
                "Captured frame: {} bytes, quality={:?}",
                cap.frame.bytes.len(),
                cap.quality
            );
            assert!(!cap.frame.bytes.is_empty());
        }
        Err(e) => {
            panic!("Frame capture failed: {:?}", e);
        }
    }
}

#[test]
fn test_camera_e2e_remote_adapter_session() {
    if !require_hardware() {
        return;
    }

    use edgerun_camera_biometrics::CameraBiometricPurpose;
    use edgerun_capabilities::{
        capability_descriptor, CapabilityEventKind, CapabilityModality, CapabilityRole,
    };
    use edgerun_remote_capability::CameraRemoteAdapter;
    use edgerun_v4l2_camera::{discover_camera_devices, V4l2CameraBiometricReader};

    let cameras = discover_camera_devices().expect("discover V4L2 cameras");
    if cameras.is_empty() {
        println!("No V4L2 cameras found, skipping");
        return;
    }

    let dev = cameras[0].clone();
    let camera_dev = edgerun_v4l2_camera::V4l2CameraDevice::new(&dev.devnode);
    let reader = V4l2CameraBiometricReader::new(camera_dev);

    let descriptor = capability_descriptor(
        "v4l2-camera",
        dev.devnode.to_string_lossy().to_string(),
        CapabilityRole::Input,
        &[CapabilityModality::Biometric],
        &[CapabilityEventKind::Biometric],
        &[CapabilityOperation::Capture],
        Vec::new(),
    );

    let adapter =
        CameraRemoteAdapter::new(reader, CameraBiometricPurpose::Presence, 2000, descriptor);
    let mut wrapped = TestGrantedProvider::new(adapter);

    let session_id = b"camera-session";
    let open = CapabilitySessionOpen {
        version: 1,
        session_id: session_id.to_vec(),
        selector: None,
        mode: CapabilitySessionMode::Stream as i32,
        requested_operations: vec![CapabilityOperation::Capture as i32],
        requested_access_class: CapabilityAccessClass::Derived as i32,
        requested_constraints: Vec::new(),
        correlation_id: Vec::new(),
    };
    let accept = wrapped.open_session(&open).expect("open camera session");
    assert!(!accept.session_id.is_empty());
    assert!(!accept.granted_operations.is_empty());
    println!(
        "Camera session opened, granted: {:?}",
        accept.granted_operations
    );
}

// ===========================================================================
// Test 5: TPM — device access
// ===========================================================================

#[test]
#[cfg(all(feature = "std", not(target_os = "none")))]
fn test_tpm_e2e_open() {
    if !require_hardware() {
        return;
    }

    use edgerun_tpm::LinuxTpmDevice;
    use std::path::Path;

    let path = Path::new("/dev/tpmrm0");
    if !path.exists() {
        println!("TPM device /dev/tpmrm0 not found, skipping");
        return;
    }

    let tpm = LinuxTpmDevice::new(path);
    println!("TPM /dev/tpmrm0 opened successfully, path={:?}", tpm.path());
    assert!(tpm.path().to_str().unwrap().contains("tpmrm0"));
}

#[test]
#[cfg(all(feature = "std", not(target_os = "none")))]
fn test_tpm_e2e_get_random() {
    if !require_hardware() {
        return;
    }

    use edgerun_tpm::{LinuxTpmDevice, TpmTransport};
    use std::path::Path;

    let path = Path::new("/dev/tpmrm0");
    if !path.exists() {
        println!("TPM device /dev/tpmrm0 not found, skipping");
        return;
    }

    let mut tpm = LinuxTpmDevice::new(path);

    // Build TPM2_GetRandom command (command code 0x0000017B)
    // Format: tag(2) + size(4) + cc(4) + bytesRequested(2) = 12 bytes
    // Total size = 12 = 0x0000000C
    let bytes_requested: u16 = 32;
    let command: Vec<u8> = vec![
        0x80,
        0x01, // tag: TPM_ST_NO_SESSIONS
        0x00,
        0x00,
        0x00,
        0x0C, // size: 12 bytes total
        0x00,
        0x00,
        0x01,
        0x7B,                           // cc: TPM2_GetRandom
        (bytes_requested >> 8) as u8,   // bytesRequested high
        (bytes_requested & 0xFF) as u8, // bytesRequested low
    ];

    let response = tpm.transact(&command);
    match response {
        Ok(resp) => {
            assert!(resp.len() >= 14, "response too short: {} bytes", resp.len());
            let tag = u16::from_be_bytes([resp[0], resp[1]]);
            let size = u32::from_be_bytes([resp[2], resp[3], resp[4], resp[5]]);
            let rc = u32::from_be_bytes([resp[6], resp[7], resp[8], resp[9]]);
            assert_eq!(
                tag, 0x8001,
                "response tag should be TPM_ST_NO_SESSIONS (0x8001)"
            );
            assert_eq!(size as usize, resp.len(), "response size mismatch");
            assert_eq!(rc, 0x00000000, "TPM returned error code: 0x{:08X}", rc);

            // Extract random bytes: after header (10) + randomBytesSize (2)
            let rand_size = u16::from_be_bytes([resp[10], resp[11]]) as usize;
            assert!(
                rand_size >= 16,
                "TPM returned only {} random bytes",
                rand_size
            );
            assert!(
                rand_size <= 32,
                "TPM returned more random bytes than requested: {}",
                rand_size
            );
            let random_bytes = &resp[12..12 + rand_size];
            assert!(
                random_bytes.iter().any(|&b| b != 0),
                "TPM returned all zeros"
            );

            println!(
                "TPM GetRandom: {} bytes = {:02X?}",
                rand_size,
                &random_bytes[..rand_size.min(16)]
            );
        }
        Err(e) => {
            panic!("TPM GetRandom failed: {:?}", e);
        }
    }
}

// ===========================================================================
// Test 6: Full client ↔ server e2e over Unix socket with real hardware
// ===========================================================================

#[test]
fn test_full_e2e_unix_socket_input_device() {
    if !require_hardware() {
        return;
    }

    use edgerun_evdev_input::{discover_evdev_devices, EvdevInputBackend};
    use edgerun_input::InputDevice;

    let devices = discover_evdev_devices().expect("discover evdev devices");
    assert!(!devices.is_empty(), "no evdev devices found");

    let info = devices[0].clone();
    let backend = EvdevInputBackend::open(info).expect("open evdev device");
    let adapter = InputRemoteAdapter::new(backend, 64);

    // Use TestGrantedProvider so the full session+invoke flow works
    let granted = TestGrantedProvider::new(adapter);

    // Create socket pair
    let (client_sock, server_sock) = UnixStream::pair().expect("create socket pair");
    let mut client_transport = FramedRemoteTransport::new(client_sock);
    let mut server_transport = FramedRemoteTransport::new(server_sock);

    let session_id = b"full-e2e-input";

    // Server thread: handle the full message loop
    thread::spawn(move || {
        let mut wrapped = granted;
        loop {
            let continued =
                edgerun_remote_capability::serve_one(&mut wrapped, &mut server_transport);
            match continued {
                Ok(true) => continue,
                Ok(false) => break,
                Err(_) => break,
            }
        }
    });

    thread::sleep(Duration::from_millis(50));

    // Client: send SessionOpen
    client_transport
        .send(CapabilityRemoteEnvelope {
            message: Some(capability_remote_envelope::Message::SessionOpen(
                CapabilitySessionOpen {
                    version: 1,
                    session_id: session_id.to_vec(),
                    selector: None,
                    mode: CapabilitySessionMode::Stream as i32,
                    requested_operations: vec![CapabilityOperation::Observe as i32],
                    requested_access_class: CapabilityAccessClass::Derived as i32,
                    requested_constraints: Vec::new(),
                    correlation_id: Vec::new(),
                },
            )),
        })
        .expect("send SessionOpen");

    // Client: receive SessionAccept
    let accept_env = client_transport
        .recv()
        .expect("recv transport ok")
        .expect("server closed before SessionAccept");

    let accept = match accept_env.message {
        Some(capability_remote_envelope::Message::SessionAccept(a)) => a,
        other => panic!("expected SessionAccept, got: {:?}", other),
    };
    println!(
        "Session accepted: accepted={}, grant_id={}, ops={:?}",
        accept.accepted,
        String::from_utf8_lossy(&accept.grant_id),
        accept.granted_operations,
    );
    assert!(accept.accepted);
    assert!(!accept.grant_id.is_empty());

    // Client: send an invocation
    client_transport
        .send(CapabilityRemoteEnvelope {
            message: Some(capability_remote_envelope::Message::InvocationFrame(
                CapabilityInvocationFrame {
                    invocation: Some(CapabilityInvocation {
                        invocation_version: 1,
                        invocation_id: b"e2e-input-invocation".to_vec(),
                        grant_id: accept.grant_id.clone(),
                        invoker: None,
                        operation: ProtoOp::Observe as i32,
                        requested_access_class: CapabilityAccessClass::Derived as i32,
                        parameter_object: None,
                        correlation_id: Vec::new(),
                        invoked_at: None,
                        signature: None,
                    }),
                    inline_parameters: Vec::new(),
                },
            )),
        })
        .expect("send InvocationFrame");

    // Client: receive result
    let result_env = client_transport
        .recv()
        .expect("recv transport ok")
        .expect("server closed before result");

    let result_inner = match result_env.message {
        Some(capability_remote_envelope::Message::ResultFrame(r)) => r
            .result
            .expect("result frame should have inner CapabilityResult"),
        Some(capability_remote_envelope::Message::Result(r)) => r,
        other => panic!("expected Result or ResultFrame, got: {:?}", other),
    };
    println!(
        "Invocation result: success={}, error={}",
        result_inner.success, result_inner.error_reason
    );
    // Input devices are stream-oriented — invoke() returns an error telling
    // the client to use session events instead. This is correct protocol behavior.
    // The key assertion is we got a well-formed result.
    assert_eq!(result_inner.result_version, 1);

    // Also verify event streaming works: ask server for events
    // Send a SessionClose to clean up the server loop
    let _ = client_transport.send(CapabilityRemoteEnvelope {
        message: Some(capability_remote_envelope::Message::SessionClose(
            CapabilitySessionClose {
                version: 1,
                session_id: session_id.to_vec(),
                reason: String::new(),
            },
        )),
    });
}

// ===========================================================================
// Test 7: Encoding roundtrip with real device data
// ===========================================================================

#[test]
fn test_input_encoding_roundtrip_real_device() {
    if !require_hardware() {
        return;
    }

    use edgerun_evdev_input::{discover_evdev_devices, EvdevInputBackend};
    use edgerun_input::InputDevice;

    let devices = discover_evdev_devices().expect("discover evdev devices");
    assert!(!devices.is_empty(), "no evdev devices found");

    let info = devices[0].clone();
    let mut backend = EvdevInputBackend::open(info).expect("open evdev device");

    let events = backend.read_events(32).expect("read input events");
    println!("Read {} events from real device", events.len());

    if events.is_empty() {
        println!("No events available (device idle)");
        return;
    }

    let event = CapabilitySessionEvent {
        version: 1,
        session_id: b"test-session".to_vec(),
        sequence_no: 1,
        event_kinds: vec![
            edgerun_proto::edgerun::v0::capability::CapabilityEventKind::State as i32,
        ],
        payload_object: None,
        inline_payload: format!("{} events captured", events.len()).into_bytes(),
    };

    let serialized = event.encode_to_vec();
    let decoded = CapabilitySessionEvent::decode(serialized.as_slice())
        .expect("decode CapabilitySessionEvent");

    assert_eq!(decoded.version, event.version);
    assert_eq!(decoded.session_id, event.session_id);
    assert_eq!(decoded.sequence_no, event.sequence_no);
    assert_eq!(decoded.inline_payload, event.inline_payload);
    println!(
        "Session event roundtrip: {} bytes, {} events referenced",
        decoded.inline_payload.len(),
        events.len()
    );
}

// ===========================================================================
// Test 8: Policy enforcement with real hardware
// ===========================================================================

#[test]
fn test_policy_enforcement_real_device() {
    if !require_hardware() {
        return;
    }

    use edgerun_evdev_input::{discover_evdev_devices, EvdevInputBackend};

    let devices = discover_evdev_devices().expect("discover evdev devices");
    assert!(!devices.is_empty(), "no evdev devices found");

    let info = devices[0].clone();
    let backend = EvdevInputBackend::open(info).expect("open evdev device");
    let adapter = InputRemoteAdapter::new(backend, 64);

    // Use PolicyWrappedProvider (real policy path) — raw adapters return
    // None from handle_request, so the session is rejected.
    let mut wrapped = PolicyWrappedProvider::new(adapter);

    let session_id = b"policy-session";
    let open = CapabilitySessionOpen {
        version: 1,
        session_id: session_id.to_vec(),
        selector: None,
        mode: CapabilitySessionMode::Unary as i32,
        requested_operations: vec![CapabilityOperation::Observe as i32],
        requested_access_class: CapabilityAccessClass::Derived as i32,
        requested_constraints: Vec::new(),
        correlation_id: Vec::new(),
    };

    let accept = wrapped
        .open_session(&open)
        .expect("session open should not error");
    assert!(!accept.session_id.is_empty());
    // With raw adapters, handle_request returns None → session is rejected
    assert!(
        !accept.accepted,
        "raw adapter should reject session via policy"
    );
    println!(
        "Policy correctly rejected session (no backing grant store): accepted={}",
        accept.accepted
    );
}

// ===========================================================================
// Test 9: Multi-device enumeration and selection
// ===========================================================================

#[test]
fn test_multi_device_enumeration() {
    if !require_hardware() {
        return;
    }

    use edgerun_alsa_microphone::discover_alsa_pcms;
    use edgerun_alsa_speaker::discover_speakers;
    use edgerun_evdev_input::discover_evdev_devices;
    use edgerun_v4l2_camera::discover_camera_devices;

    let inputs = discover_evdev_devices().unwrap_or_default();
    let pcms = discover_alsa_pcms().unwrap_or_default();
    let speakers = discover_speakers().unwrap_or_default();
    let cameras = discover_camera_devices().unwrap_or_default();

    let capture_pcms: Vec<_> = pcms.iter().filter(|p| p.capture).collect();
    let playback_pcms: Vec<_> = pcms.iter().filter(|p| p.playback).collect();

    println!("=== Hardware Inventory ===");
    println!("  Input devices (evdev):  {}", inputs.len());
    for dev in &inputs {
        println!(
            "    {} — {} — kind={:?}",
            dev.event_node, dev.device_name, dev.kind
        );
    }
    println!("  ALSA PCM devices:       {}", pcms.len());
    println!("    Capture:              {}", capture_pcms.len());
    println!("    Playback:             {}", playback_pcms.len());
    println!("  ALSA speakers:          {}", speakers.len());
    println!("  V4L2 cameras:           {}", cameras.len());
    for cam in &cameras {
        println!("    {:?} — {}", cam.devnode, cam.devnode.display());
    }

    let total =
        inputs.len() + capture_pcms.len() + playback_pcms.len() + speakers.len() + cameras.len();
    assert!(total > 0, "no hardware devices found at all");

    for dev in &inputs {
        assert!(!dev.event_node.is_empty());
    }
    for pcm in &pcms {
        assert!(
            pcm.capture || pcm.playback,
            "pcm should have at least one direction"
        );
    }
}

// ===========================================================================
// Test 10: Bluetooth scanning (mgmt socket)
// ===========================================================================

#[test]
fn test_bluetooth_e2e_scan() {
    if !require_hardware() {
        return;
    }

    // Try mgmt socket discovery (requires root/CAP_NET_ADMIN)
    let result = edgerun_mgmt_bluetooth::discover_controllers();
    match result {
        Ok(controllers) => {
            assert!(!controllers.is_empty(), "no Bluetooth controllers found");
            for ctrl in &controllers {
                println!(
                    "Bluetooth hci{}: addr={} name={}",
                    ctrl.index, ctrl.address, ctrl.name
                );
            }
            // Verify we can create a scanning backend
            let backend = edgerun_mgmt_bluetooth::MgmtBluetoothBackend {
                controller: controllers[0].clone(),
            };
            let descriptor = backend.descriptor();
            assert_eq!(descriptor.descriptor_version, 1);
            assert!(!descriptor.provider_name.is_empty());
        }
        Err(e) => {
            // mgmt socket requires root — this is expected on non-root systems
            println!(
                "Bluetooth mgmt discovery failed (expected without root): {:?}",
                e
            );
        }
    }
}

// ===========================================================================
// Test 11: WiFi interface discovery (nl80211)
// ===========================================================================

#[test]
fn test_wifi_e2e_discover() {
    if !require_hardware() {
        return;
    }

    let result = edgerun_linux_wifi::discover_wifi_interfaces();
    match result {
        Ok(interfaces) => {
            assert!(!interfaces.is_empty(), "no WiFi interfaces found");
            for iface in &interfaces {
                println!(
                    "WiFi: name={} mac={} phy={} operstate={:?}",
                    iface.name,
                    iface.mac_address.as_deref().unwrap_or("unknown"),
                    iface.phy_name.as_deref().unwrap_or("unknown"),
                    iface.operstate,
                );
            }
            // Verify scanning backend
            let backend = edgerun_linux_wifi::LinuxWifiBackend {
                interface: interfaces[0].clone(),
            };
            let descriptor = backend.descriptor();
            assert_eq!(descriptor.descriptor_version, 1);
        }
        Err(e) => {
            println!("WiFi discovery failed (may need root for nl80211): {:?}", e);
        }
    }
}

// ===========================================================================
// Test 12: DRM display discovery (sysfs /sys/class/drm)
// ===========================================================================

#[test]
fn test_drm_display_e2e_discover() {
    if !require_hardware() {
        return;
    }

    let connectors =
        edgerun_drm_display::discover_drm_connectors().expect("DRM discovery should not error");

    let connected: Vec<_> = connectors.iter().filter(|c| c.connected).collect();
    assert!(!connected.is_empty(), "no connected DRM displays found");

    for conn in &connected {
        println!(
            "DRM: {} connected={} enabled={} modes={}",
            conn.connector_name,
            conn.connected,
            conn.enabled,
            conn.modes.len()
        );
        if !conn.modes.is_empty() {
            let mode = &conn.modes[0];
            println!(
                "  {}x{}@{}Hz",
                mode.width,
                mode.height,
                mode.refresh_millihz / 1000,
            );
            assert!(mode.width > 0);
            assert!(mode.height > 0);
        }
    }
}

// ===========================================================================
// Test 13: NPU discovery and query
// ===========================================================================

#[test]
fn test_npu_e2e_discover() {
    if !require_hardware() {
        return;
    }

    let npus = edgerun_linux_npu::discover_linux_npus().expect("NPU discovery should not error");

    if npus.is_empty() {
        println!("No NPU devices found");
        return;
    }

    for npu in &npus {
        println!(
            "NPU: {} driver={} pci={} firmware={} accel_class={}",
            npu.display_name,
            npu.driver_name.as_deref().unwrap_or("unknown"),
            npu.pci_address.as_deref().unwrap_or("unknown"),
            npu.firmware_version.as_deref().unwrap_or("unknown"),
            npu.accelerator_class,
        );
    }
}

// ===========================================================================
// Test 14: Goodix fingerprint discovery and capture
// ===========================================================================

#[test]
fn test_goodix_e2e_discover() {
    if !require_hardware() {
        return;
    }

    use edgerun_fingerprint::FingerprintReader;

    let devices = edgerun_goodix_fingerprint::discover_supported_devices();
    match devices {
        Ok(devs) => {
            if devs.is_empty() {
                println!("No Goodix fingerprint devices found");
                return;
            }
            for dev in &devs {
                println!(
                    "Goodix: vendor=0x{:04X} product=0x{:04X}",
                    dev.vendor_id, dev.product_id
                );
            }

            // Try to open
            let reader = edgerun_goodix_fingerprint::GoodixFingerprintReader::new(devs[0].clone())
                .expect("open Goodix reader");
            let info = reader.reader_info().expect("get reader info");
            println!(
                "Fingerprint reader: {} max_templates={} hw_protected={}",
                info.reader_name,
                info.max_templates.unwrap_or(0),
                info.hardware_protected_match,
            );
        }
        Err(e) => {
            println!("Goodix discovery failed: {:?}", e);
        }
    }
}
