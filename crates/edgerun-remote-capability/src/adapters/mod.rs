//! Remote capability adapters for hardware devices.

mod input;
mod microphone;
mod speaker;
mod bluetooth;
mod wifi;
mod camera;

pub use input::InputRemoteAdapter;
pub use microphone::MicrophoneRemoteAdapter;
pub use speaker::SpeakerRemoteAdapter;
pub use bluetooth::{BluetoothRemoteAdapter, BluetoothConnectionRemoteAdapter};
pub use wifi::{WifiRemoteAdapter, WifiControlRemoteAdapter};
pub use camera::{CameraRemoteAdapter, PairedCameraRemoteAdapter};

// Re-export encode/decode functions that external crates may use.
pub use input::{encode_input_events, decode_input_events};
pub use microphone::{encode_microphone_capture, decode_microphone_capture};
pub use speaker::{
    encode_speaker_playback_request, decode_speaker_playback_request,
    encode_speaker_output_level, decode_speaker_output_level,
    encode_speaker_playback_result, decode_speaker_playback_result,
};
pub use bluetooth::{
    encode_bluetooth_scan_result, decode_bluetooth_scan_result,
    encode_bluetooth_connections, decode_bluetooth_connections,
};
pub use wifi::{
    encode_wifi_scan_result, decode_wifi_scan_result,
    encode_wifi_interface_info, decode_wifi_interface_info,
};
pub use camera::{
    encode_camera_capture, decode_camera_capture,
    encode_paired_camera_frame, decode_paired_camera_frame,
};
