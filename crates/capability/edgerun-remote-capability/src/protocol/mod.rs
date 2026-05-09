//! Core protocol traits and session lifecycle helpers.

use crate::prelude::v1::*;

mod serve;
mod session;

pub use serve::{capability_error_result, pump_one_event, serve_one};
pub use session::{
    accept_session_open_unchecked, default_remote_requester, default_remote_requester_opt,
    session_accept_from_grant, session_open_as_request, session_reject,
};

use edgerun_capabilities::{CapabilityDescriptor, CapabilityError};
use edgerun_protocols::core_protocol::protocol::capability::{
    CapabilityGrant, CapabilityInvocation, CapabilityRequest, CapabilityRevocation,
};
pub use edgerun_protocols::core_protocol::protocol::capability_runtime::{
    CapabilityInvocationFrame, CapabilityRemoteEnvelope, CapabilityResultFrame,
    CapabilitySessionAccept, CapabilitySessionClose, CapabilitySessionEvent, CapabilitySessionMode,
    CapabilitySessionOpen, capability_remote_envelope,
};

/// Result of a remote capability invocation.
pub struct RemoteInvocationResult {
    pub result: edgerun_protocols::core_protocol::protocol::capability::CapabilityResult,
    pub inline_payload: Vec<u8>,
}

/// A provider of a remote capability (camera, microphone, bluetooth, etc.).
pub trait RemoteCapabilityProvider {
    fn descriptor(&self) -> CapabilityDescriptor;

    fn open_session(
        &mut self,
        open: &CapabilitySessionOpen,
    ) -> Result<CapabilitySessionAccept, CapabilityError> {
        Ok(accept_session_open_unchecked(open))
    }

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

/// Wire transport for remote capability envelopes.
pub trait RemoteCapabilityTransport {
    fn send(&mut self, envelope: CapabilityRemoteEnvelope) -> Result<(), CapabilityError>;
    fn recv(&mut self) -> Result<Option<CapabilityRemoteEnvelope>, CapabilityError>;
}
