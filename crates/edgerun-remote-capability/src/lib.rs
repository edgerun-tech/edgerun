//! Remote capability protocol: serialize hardware device access over a transport.
//!
//! ## Quick start
//!
//! ```
//! use edgerun_remote_capability::RemoteCapabilityProvider;
//! use edgerun_remote_capability::MemoryRemoteTransport;
//! ```

pub mod adapters;
pub mod capability_signature;
pub mod policy;
pub mod protocol;
pub mod transport;

// Re-export the core public API.
pub use adapters::{
    BluetoothConnectionRemoteAdapter, BluetoothRemoteAdapter, CameraRemoteAdapter,
    InputRemoteAdapter, MicrophoneRemoteAdapter, PairedCameraRemoteAdapter, SpeakerRemoteAdapter,
    WifiControlRemoteAdapter, WifiRemoteAdapter,
};
pub use capability_signature::{
    sign_grant, sign_invocation, sign_request, sign_result, sign_revocation, verify_grant,
    verify_invocation, verify_request, verify_result, verify_revocation,
};
pub use policy::{IntoPolicyWrappedProvider, PolicyWrappedProvider, SessionGrantBinding};
pub use protocol::{
    accept_session_open_unchecked, capability_error_result, capability_remote_envelope,
    default_remote_requester, pump_one_event, serve_one, session_accept_from_grant,
    session_open_as_request, session_reject, CapabilityInvocationFrame, CapabilityRemoteEnvelope,
    CapabilityResultFrame, CapabilitySessionAccept, CapabilitySessionClose, CapabilitySessionEvent,
    CapabilitySessionMode, CapabilitySessionOpen, RemoteCapabilityProvider,
    RemoteCapabilityTransport, RemoteInvocationResult,
};
pub use transport::{accept_tcp, accept_unix, FramedRemoteTransport, MemoryRemoteTransport};

#[cfg(test)]
mod tests;
