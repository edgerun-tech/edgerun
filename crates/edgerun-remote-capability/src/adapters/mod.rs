//! Remote capability adapters for hardware devices.

mod bluetooth;
mod camera;
mod common;
mod input;
mod microphone;
mod speaker;
mod wifi;

pub use bluetooth::{BluetoothConnectionRemoteAdapter, BluetoothRemoteAdapter};
pub use camera::{CameraRemoteAdapter, PairedCameraRemoteAdapter};
pub use input::InputRemoteAdapter;
pub use microphone::MicrophoneRemoteAdapter;
pub use speaker::SpeakerRemoteAdapter;
pub use wifi::{WifiControlRemoteAdapter, WifiRemoteAdapter};

// Re-export encode/decode functions that external crates may use.
pub use bluetooth::{
    decode_bluetooth_connections, decode_bluetooth_scan_result, encode_bluetooth_connections,
    encode_bluetooth_scan_result,
};
pub use camera::{
    decode_camera_capture, decode_paired_camera_frame, encode_camera_capture,
    encode_paired_camera_frame,
};
pub use input::{decode_input_events, encode_input_events};
pub use microphone::{decode_microphone_capture, encode_microphone_capture};
pub use speaker::{
    decode_speaker_output_level, decode_speaker_playback_request, decode_speaker_playback_result,
    encode_speaker_output_level, encode_speaker_playback_request, encode_speaker_playback_result,
};
pub use wifi::{
    decode_wifi_interface_info, decode_wifi_scan_result, encode_wifi_interface_info,
    encode_wifi_scan_result,
};
